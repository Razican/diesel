//! Contains the `Row` trait

use crate::backend::{self, Backend};

/// Represents a single database row.
/// Apps should not need to concern themselves with this trait.
///
/// This trait is only used as an argument to [`FromSqlRow`].
///
/// [`FromSqlRow`]: ../deserialize/trait.FromSqlRow.html
pub trait Row<'a, DB: Backend>: ExactSizeIterator + Clone
where
    Self::Item: Field<'a, DB>,
{
}

impl<'a, T, DB: Backend> Row<'a, DB> for T
where
    T: Clone + ExactSizeIterator,
    T::Item: Field<'a, DB>,
{
}

pub trait Field<'a, DB: Backend> {
    fn column_name(&self) -> Option<&str>;
    fn value(&self) -> Option<backend::RawValue<'a, DB>>;

    fn is_null(&self) -> bool {
        self.value().is_none()
    }
}
