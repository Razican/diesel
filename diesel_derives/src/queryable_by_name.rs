#![warn(warnings)]
use proc_macro2::{self, Ident, Span, TokenStream};
use syn;

use field::*;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;
    let fields = model.fields().iter().map(get_ident).collect::<Vec<_>>();
    let field_names = model.fields().iter().map(|f| &f.name).collect::<Vec<_>>();

    let match_block = model
        .fields()
        .iter()
        .filter_map(|f| {
            if f.has_flag("embed") {
                None
            } else {
                Some(match_block(f, &model))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let column_names = model
        .fields()
        .iter()
        .map(|f| f.column_name())
        .collect::<Vec<_>>();

    let initial_field_expr = model
        .fields()
        .iter()
        .map(|f| {
            if f.has_flag("embed") {
                // quote!(std::option::Option::Some(FromSqlRow::build(
                //     &mut row.clone()
                // )?))
                quote!(std::option::Option::None)
            } else {
                quote!(std::option::Option::None)
            }
        })
        .collect::<Vec<_>>();

    let (_, ty_generics, ..) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for field in model.fields() {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        let field_ty = field.ty_for_deserialize()?;
        if field.has_flag("embed") {
            // where_clause
            //     .predicates
            //     .push(parse_quote!(#field_ty: FromSqlRow<UntypedRow,__DB>));
        } else {
            let st = sql_type(field, &model);
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: diesel::deserialize::FromSql<#st, __DB>));
        }
    }

    let field_number = fields.len();
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, FromSqlRow};
        use diesel::row::{Row, Field};
        use diesel::sql_types::Untyped;

        impl #impl_generics FromSqlRow<Untyped, __DB>
            for #struct_name #ty_generics
        #where_clause
        {
            fn build_from_row<'__a, __T: Row<'__a, __DB>>(row: &mut __T) -> deserialize::Result<Self>
            where
                __T::Item: Field<'__a, __DB>,
            {
                #(
                    let mut #fields = #initial_field_expr;
                )*
                let mut field_counter = #field_number;

                for field in row {
                    if field_counter == 0 {
                        break;
                    }
                    match field.column_name() {
                        #(
                            #match_block
                        )*
                        // skip everything else
                        _ => {},
                    }
                }

                deserialize::Result::Ok(Self {
                    #(
                        #field_names: #fields
                        .ok_or_else(|| -> std::boxed::Box<dyn std::error::Error + std::marker::Send + std::marker::Sync> {
                            String::from(
                                concat!("Column `", stringify!(#column_names), "` was not present in query")
                            ).into()})?,
                    )*
                })
            }
            fn is_null<'__a, __T: Row<'__a, __DB>>(row: &mut __T) -> bool
            where
                __T::Item: Field<'__a, __DB>,
            {
                todo!()
            }
        }
    }))
}

fn get_ident(field: &Field) -> Ident {
    match &field.name {
        FieldName::Named(n) => n.clone(),
        FieldName::Unnamed(i) => Ident::new(&format!("field_{}", i.index), Span::call_site()),
    }
}

fn match_block(field: &Field, model: &Model) -> Result<TokenStream, Diagnostic> {
    let column_name = field.column_name();
    let sql_type = sql_type(field, model);
    let field_name = get_ident(field);
    let deserialize_ty = field.ty_for_deserialize()?;

    Ok(quote!(
        std::option::Option::Some(stringify!(#column_name)) => {
            field_counter -= 1;
            let val = field.value();
            #field_name = std::option::Option::Some(
                <#deserialize_ty as diesel::deserialize::FromSql<#sql_type, __DB>>::from_nullable_sql(val)?.into()
            );
    }))
}

fn sql_type(field: &Field, model: &Model) -> syn::Type {
    let table_name = model.table_name();
    let column_name = field.column_name();

    match field.sql_type {
        Some(ref st) => st.clone(),
        None => {
            if model.has_table_name_attribute() {
                parse_quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>)
            } else {
                let field_name = match field.name {
                    FieldName::Named(ref x) => x.clone(),
                    _ => Ident::new("field", Span::call_site()),
                };
                field
                    .span
                    .error(format!("Cannot determine the SQL type of {}", field_name))
                    .help(
                        "Your struct must either be annotated with `#[table_name = \"foo\"]` \
                         or have all of its fields annotated with `#[sql_type = \"Integer\"]`",
                    )
                    .emit();
                parse_quote!(())
            }
        }
    }
}
