use proc_macro2;
use syn;

use field::Field;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;
    let field_ty = model
        .fields()
        .iter()
        .map(Field::ty_for_deserialize)
        .collect::<Result<Vec<_>, _>>()?;
    let field_ty = &field_ty;
    let build_expr = model.fields().iter().enumerate().map(|(i, f)| {
        let i = syn::Index::from(i);
        f.name.assign(parse_quote!(row.#i.into()))
    });

    let (_, ty_generics, _) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));
    generics.params.push(parse_quote!(__ST));
    {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!((#(#field_ty,)*): FromSqlRow<Typed<__ST>, __DB>));
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let field_count = field_ty.len();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{FromSqlRow, Result, StaticallySizedRow};
        use diesel::row::{Row, Field};
        use diesel::sql_types::Typed;

        impl #impl_generics FromSqlRow<Typed<__ST>, __DB> for #struct_name #ty_generics
            #where_clause
        {
            fn build_from_row<'__a, __T: Row<'__a, __DB>>(row: &mut __T) -> Result<Self>
            where
                __T::Item: Field<'__a, __DB>,
            {
                let row =
                    <(#(#field_ty,)*) as FromSqlRow<Typed<__ST>, __DB>>::build_from_row(row)?;
                Result::Ok(Self {
                    #(#build_expr,)*
                })
            }

            fn is_null<'__a, __T: Row<'__a, __DB>>(row: &mut __T) -> bool
            where
                __T::Item: Field<'__a, __DB>,
            {
                <(#(#field_ty,)*) as FromSqlRow<Typed<__ST>, __DB>>::is_null(row)
            }
        }

        impl #impl_generics StaticallySizedRow<Typed<__ST>, __DB> for #struct_name #ty_generics
            #where_clause
        {
            const FIELD_COUNT:usize = #field_count;
        }
    }))
}
