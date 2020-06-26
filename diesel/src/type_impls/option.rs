use std::io::Write;

use crate::backend::{self, Backend};
use crate::deserialize::{self, FromSql, FromSqlRow, StaticallySizedRow};
use crate::expression::bound::Bound;
use crate::expression::*;
use crate::query_builder::QueryId;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{
    HasSqlType, IntoNotNullable, IsNullable, NotNull, Nullable, SqlType, Typed,
};

impl<T, DB> HasSqlType<Nullable<T>> for DB
where
    DB: Backend + HasSqlType<T>,
    T: SqlType,
{
    fn metadata(lookup: &DB::MetadataLookup) -> DB::TypeMetadata {
        <DB as HasSqlType<T>>::metadata(lookup)
    }

    #[cfg(feature = "mysql")]
    fn mysql_row_metadata(out: &mut Vec<DB::TypeMetadata>, lookup: &DB::MetadataLookup) {
        <DB as HasSqlType<T>>::mysql_row_metadata(out, lookup)
    }
}

impl<T> QueryId for Nullable<T>
where
    T: QueryId + SqlType<IsNull = NotNull>,
{
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, ST, DB> FromSql<Nullable<ST>, DB> for Option<T>
where
    T: FromSql<ST, DB>,
    DB: Backend,
    ST: SqlType<IsNull = NotNull>,
{
    fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
        T::from_sql(bytes).map(Some)
    }

    fn from_nullable_sql(bytes: Option<backend::RawValue<DB>>) -> deserialize::Result<Self> {
        match bytes {
            Some(bytes) => T::from_sql(bytes).map(Some),
            None => Ok(None),
        }
    }
}

impl<T, ST, DB> FromSqlRow<Typed<ST>, DB> for Option<T>
where
    DB: Backend,
    ST: SqlType<IsNull = IsNullable> + IntoNotNullable,
    T: FromSqlRow<Typed<ST::NotNullable>, DB>,
{
    fn build_from_row<'a, R: crate::row::Row<'a, DB>>(row: &mut R) -> deserialize::Result<Self>
    where
        R::Item: crate::row::Field<'a, DB>,
    {
        let mut iter = row.clone();
        if T::is_null(row) {
            Ok(None)
        } else {
            T::build_from_row(&mut iter).map(Some)
        }
    }

    fn is_null<'a, R: crate::row::Row<'a, DB>>(row: &mut R) -> bool
    where
        R::Item: crate::row::Field<'a, DB>,
    {
        T::is_null(row);
        false
    }
}

impl<T, ST, DB> StaticallySizedRow<Typed<Nullable<ST>>, DB> for Option<T>
where
    T: StaticallySizedRow<Typed<ST>, DB>,
    ST: SqlType,
    DB: Backend,
    Self: FromSqlRow<Typed<Nullable<ST>>, DB>,
{
    const FIELD_COUNT: usize = T::FIELD_COUNT;
}

impl<T, ST, DB> ToSql<Nullable<ST>, DB> for Option<T>
where
    T: ToSql<ST, DB>,
    DB: Backend,
    ST: SqlType<IsNull = NotNull>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        if let Some(ref value) = *self {
            value.to_sql(out)
        } else {
            Ok(IsNull::Yes)
        }
    }
}

impl<T, ST> AsExpression<Nullable<ST>> for Option<T>
where
    ST: SqlType<IsNull = NotNull>,
{
    type Expression = Bound<Nullable<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'a, T, ST> AsExpression<Nullable<ST>> for &'a Option<T>
where
    ST: SqlType<IsNull = NotNull>,
{
    type Expression = Bound<Nullable<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

#[cfg(all(test, feature = "postgres"))]
use crate::pg::Pg;
#[cfg(all(test, feature = "postgres"))]
use crate::sql_types;

#[test]
#[cfg(feature = "postgres")]
fn option_to_sql() {
    type Type = sql_types::Nullable<sql_types::VarChar>;
    let mut bytes = Output::test();

    let is_null = ToSql::<Type, Pg>::to_sql(&None::<String>, &mut bytes).unwrap();
    assert_eq!(IsNull::Yes, is_null);
    assert!(bytes.is_empty());

    let is_null = ToSql::<Type, Pg>::to_sql(&Some(""), &mut bytes).unwrap();
    assert_eq!(IsNull::No, is_null);
    assert!(bytes.is_empty());

    let is_null = ToSql::<Type, Pg>::to_sql(&Some("Sean"), &mut bytes).unwrap();
    let expectd_bytes = b"Sean".to_vec();
    assert_eq!(IsNull::No, is_null);
    assert_eq!(bytes, expectd_bytes);
}
