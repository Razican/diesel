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

/// Represents a single field in a database row.
///
/// This trait allows retrieving information on the name of the colum and on the value of the
/// field.
pub trait Field<'a, DB: Backend> {
    /// Retrieves the column name of the field, if any.
    fn column_name(&self) -> Option<&str>;

    /// Retrieves the raw value of the field.
    ///
    /// This raw value is backend-dependant.
    fn value(&self) -> Option<backend::RawValue<'a, DB>>;

    /// Checks whether this field is null or not.
    fn is_null(&self) -> bool {
        self.value().is_none()
    }
}
