use proc_macro2::*;
use syn;

use meta::*;
use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let flags =
        MetaItem::with_name(&item.attrs, "diesel").unwrap_or_else(|| MetaItem::empty("diesel"));
    let struct_ty = ty_for_foreign_derive(&item, &flags)?;

    item.generics.params.push(parse_quote!(__ST));
    item.generics.params.push(parse_quote!(__DB));
    {
        let where_clause = item
            .generics
            .where_clause
            .get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!(__DB: diesel::backend::Backend));
        where_clause
            .predicates
            .push(parse_quote!(Self: FromSql<__ST, __DB>));
    }
    let (impl_generics, _, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, FromSql, FromSqlRow, Queryable, StaticallySizedRow};
        use diesel::sql_types::Typed;

        impl #impl_generics FromSqlRow<Typed<__ST>, __DB> for #struct_ty
        #where_clause
        {

            fn build_from_row<'a, R: diesel::row::Row<'a, __DB>>(row: &mut R)
                                                                 -> deserialize::Result<Self>
            where
                R::Item: diesel::row::Field<'a, __DB>,
            {
                use diesel::row::Field;

                FromSql::<__ST, __DB>::from_nullable_sql(
                    row.next()
                        .ok_or_else(|| String::from("Unexpected end of row"))?
                        .value()
                )
            }

            fn is_null<'a, R: diesel::row::Row<'a, __DB>>(row: &mut R) -> bool
            where
                R::Item: diesel::row::Field<'a, __DB>,
            {
                row.next().map(|v| diesel::row::Field::is_null(&v)).unwrap_or(false)
            }
        }

        // impl #impl_generics Queryable<TypedRow<__ST>, __DB> for #struct_ty
        // #where_clause
        // {
        //     type Row = Self;

        //     fn build(row: Self::Row) -> Self {
        //         row
        //     }
        // }

        impl #impl_generics StaticallySizedRow<Typed<__ST>, __DB> for #struct_ty
        #where_clause
        {}
    }))
}
