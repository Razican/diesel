//! Types and traits related to deserializing values from the database

use std::error::Error;
use std::result;

use crate::backend::{self, Backend};
use crate::row::{Field, Row};
use crate::sql_types::{HasSqlType, TypeMetadata, Typed, Untyped};

/// A specialized result type representing the result of deserializing
/// a value from the database.
pub type Result<T> = result::Result<T, Box<dyn Error + Send + Sync>>;

#[doc(inline)]
pub use diesel_derives::Queryable;

#[doc(inline)]
pub use diesel_derives::QueryableByName;

/// Deserialize a single field of a given SQL type.
///
/// When possible, implementations of this trait should prefer to use an
/// existing implementation, rather than reading from `bytes`. (For example, if
/// you are implementing this for an enum which is represented as an integer in
/// the database, prefer `i32::from_sql(bytes)` over reading from `bytes`
/// directly)
///
/// Types which implement this trait should also have `#[derive(FromSqlRow)]`
///
/// ### Backend specific details
///
/// - For PostgreSQL, the bytes will be sent using the binary protocol, not text.
/// - For SQLite, the actual type of `DB::RawValue` is private API. All
///   implementations of this trait must be written in terms of an existing
///   primitive.
/// - For MySQL, the value of `bytes` will depend on the return value of
///   `type_metadata` for the given SQL type. See [`MysqlType`] for details.
/// - For third party backends, consult that backend's documentation.
///
/// [`MysqlType`]: ../mysql/enum.MysqlType.html
///
/// ### Examples
///
/// Most implementations of this trait will be defined in terms of an existing
/// implementation.
///
/// ```rust
/// # use diesel::backend::{self, Backend};
/// # use diesel::sql_types::*;
/// # use diesel::deserialize::{self, FromSql};
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy)]
/// pub enum MyEnum {
///     A = 1,
///     B = 2,
/// }
///
/// impl<DB> FromSql<Integer, DB> for MyEnum
/// where
///     DB: Backend,
///     i32: FromSql<Integer, DB>,
/// {
///     fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
///         match i32::from_sql(bytes)? {
///             1 => Ok(MyEnum::A),
///             2 => Ok(MyEnum::B),
///             x => Err(format!("Unrecognized variant {}", x).into()),
///         }
///     }
/// }
/// ```
pub trait FromSql<A, DB: Backend>: Sized {
    /// See the trait documentation.
    fn from_sql(bytes: backend::RawValue<DB>) -> Result<Self>;

    fn from_nullable_sql(bytes: Option<backend::RawValue<DB>>) -> Result<Self> {
        match bytes {
            Some(bytes) => Self::from_sql(bytes),
            None => return Err(Box::new(crate::result::UnexpectedNullError)),
        }
    }
}

pub trait IsCompatibleType<DB> {
    type Compatible;

    #[doc(hidden)]
    #[cfg(feature = "mysql")]
    fn mysql_row_metadata(_lookup: &DB::MetadataLookup) -> Option<Vec<DB::TypeMetadata>>
    where
        DB: Backend + TypeMetadata,
    {
        None
    }
}

// Any typed row with the same sql type is compatible
impl<ST, DB> IsCompatibleType<DB> for Typed<ST>
where
    DB: Backend + HasSqlType<ST>,
{
    type Compatible = Typed<ST>;

    #[cfg(feature = "mysql")]
    fn mysql_row_metadata(lookup: &DB::MetadataLookup) -> Option<Vec<DB::TypeMetadata>>
    where
        DB: Backend + TypeMetadata,
    {
        let mut out = Vec::new();
        <DB as HasSqlType<ST>>::mysql_row_metadata(&mut out, lookup);
        Some(out)
    }
}

// Any untyped row is compatible
impl<DB> IsCompatibleType<DB> for Untyped
where
    DB: Backend,
{
    type Compatible = Untyped;
}

// // Any unorderd typed row is compatible with any untyped row
// impl<ST, DB> IsCompatibleType<UntypedRow, DB> for UnorderedTypedRow<ST> where DB: Backend {}

// impl<ST1, ST2, DB, I> IsCompatibleType<TypedRow<ST1>, DB, I> for UnorderedTypedRow<ST2> where
//     UnorderedTypedRow<ST2>: IsCompatibleType<UnorderedTypedRow<ST1>, DB, I>
// {
//}

// macro_rules! expand_tuple_impls {
//     (
//         @make_hlist_tuple: $T1: ident,
//     ) => {
//         ($T1, ())
//     };
//     (
//         @make_hlist_tuple: $T1: ident, $($T: ident,)*
//     ) => {
//         ($T1, expand_tuple_impls!(@make_hlist_tuple: $($T,)*))
//     };
//     (
//         @get_bounds:
//         ts = [],
//         tts = [],
//         last = [
//             ty = [$($last_ty:tt)*],
//             bound = [$($bound:tt)*],
//         ],
//         aggregate = [
//             $($aggrate:tt)*
//         ],
//         others = [
//             ts = [$($T: ident,)*],
//             sts = [$($ST: ident,)* ],
//             tts = [$($TT: ident,)* ],
//         ],
//     ) => {
//         expand_tuple_impls!(
//             @impl:
//             ts = [$($T,)*],
//             sts = [$($ST,)*],
//             tts = [$($TT,)*],
//             bounds = [
//                 $($aggrate)*
//                 $($last_ty)*: $($bound)*,
//             ],
//         );
//     };
//     (
//         @get_bounds:
//         ts = [$T1: ident, $($T: ident,)*],
//         tts = [$TT1: ident, $($TT: ident,)*],
//         last = [
//             ty = [$($last_ty:tt)*],
//             bound = [$($bound:tt)*],
//         ],
//         aggregate = [
//             $($aggregate:tt)*
//         ],
//         others = [$($others:tt)*],
//     ) => {
//         expand_tuple_impls!(
//             @get_bounds:
//             ts = [$($T,)*],
//             tts = [$($TT,)*],
//             last = [
//                 ty = [<$($last_ty)* as $($bound)*>::Remainder],
//                 bound = [Plucker<$T1, $TT1>],
//             ],
//             aggregate = [
//                 $($aggregate)*
//                 $($last_ty)*: $($bound)*,
//             ],
//             others = [$($others)*],
//         );

//     };
//     (
//         @get_bounds:
//         ts = [$T1:ident, $($T: ident,)*],
//         sts = [$($ST: ident,)*],
//         tts = [$TT1: ident, $($TT: ident,)*],
//     ) => {
//         expand_tuple_impls!(
//             @get_bounds:
//             ts = [$($T,)*],
//             tts = [$($TT,)*],
//             last = [
//                 ty = [expand_tuple_impls!(@make_hlist_tuple: $($ST,)*)],
//                 bound = [Plucker<$T1, $TT1>],
//             ],
//             aggregate = [],
//             others = [
//                 ts = [$T1, $($T,)*],
//                 sts = [$($ST,)*],
//                 tts = [$TT1, $($TT,)*],
//             ],
//         );

//     };
//     (
//         @impl:
//         ts = [$($T: ident,)*],
//         sts = [$($ST:ident,)*],
//         tts = [$($TT:ident,)*],
//         bounds = [$($bounds:tt)*],
//     ) => {
//         impl<$($T,)* $($ST,)* $($TT,)* __DB> IsCompatibleType<UnorderedTypedRow<($($ST,)*)>, __DB, ($($TT,)*)> for UnorderedTypedRow<($($T,)*)>
//             where $($bounds)*
//         {

//         }
//     };
//     (@decouple2: ts = $T:tt, sts = [$({$($ST:ident,)*},)*], tts = $TT:tt,) => {
//         $(
//             expand_tuple_impls!(
//                 @get_bounds:
//                 ts = $T,
//                 sts = [$($ST,)*],
//                 tts = $TT,
//             );
//         )*
//     };

//     (@decouple: ts = [$({$($T:ident,)*},)*], sts = $ST:tt, tts = [$({$($TT: ident,)*},)*],) => {
//         $(
//             expand_tuple_impls!(
//                 @decouple2:
//                 ts = [$($T,)*],
//                 sts = $ST,
//                 tts = [$($TT,)*],
//             );
//         )*
//     };
//     (pairs = [$({ ts = [$($T: ident,)*], sts = [$($ST: ident,)*], tts = [$($TT: ident,)*]},)*]) => {
//         expand_tuple_impls!(
//             @decouple
//             ts = [$({$($T,)*},)*].
//             sts = [$({$($ST,)*},)*],
//             tts = [$({$($TT,)*},)*],
//         );
//     }
// }

// macro_rules! tuple_impls {
//     ($(
//         $Tuple:tt {
//             $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
//         }
//     )+) => {
//         expand_tuple_impls!(
//             @decouple:
//             ts = [$({$($T,)*},)*],
//             sts = [$({$($ST,)*},)*],
//             tts = [$({$($TT,)*},)*],
//         );
// //        expand_tuple_impls!(pairs = [$({ts = [$($T,)*], sts = [$($ST,)*], tts = [$($TT,)*]},)*]);

//     }
// }

// //trace_macros!(true);
// __diesel_for_each_tuple!(tuple_impls);
// //trace_macros!(false);

/// Deserialize one or more fields.
///
/// All types which implement `FromSql` should also implement this trait. This
/// trait differs from `FromSql` in that it is also implemented by tuples.
/// Implementations of this trait are usually derived.
///
/// In the future, we hope to be able to provide a blanket impl of this trait
/// for all types which implement `FromSql`. However, as of Diesel 1.0, such an
/// impl would conflict with our impl for tuples.
///
/// This trait can be [derived](derive.FromSqlRow.html)
pub trait FromSqlRow<ST, DB: Backend>: Sized {
    /// See the trait documentation.
    fn build_from_row<'a, T: Row<'a, DB>>(row: &mut T) -> Result<Self>
    where
        T::Item: Field<'a, DB>;

    fn is_null<'a, T: Row<'a, DB>>(row: &mut T) -> bool
    where
        T::Item: Field<'a, DB>;
}

#[doc(inline)]
pub use diesel_derives::FromSqlRow;

/// A marker trait indicating that the corresponding type is a statically sized row
///
/// This trait is implemented for all types provided by diesel, that
/// implement `FromSqlRow`.
///
/// For dynamically sized types, like `diesel_dynamic_schema::DynamicRow`
/// this traits should not be implemented.
///
/// This trait can be [derived](derive.FromSqlRow.html)
pub trait StaticallySizedRow<ST, DB: Backend>: FromSqlRow<ST, DB> {
    /// The number of fields that this type will consume. Must be equal to
    /// the number of times you would call `row.take()` in `build_from_row`
    const FIELD_COUNT: usize = 1;
}

// Reasons we can't write this:
//
// impl<T, ST, DB> FromSqlRow<ST, DB> for T
// where
//     DB: Backend + HasSqlType<ST>,
//     T: FromSql<ST, DB>,
// {
//     fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self> {
//         Self::from_sql(row.take())
//     }
// }
//
// (this is mostly here so @sgrif has a better reference every time they think
// they've somehow had a breakthrough on solving this problem):
//
// - It conflicts with our impl for tuples, because `DB` is a bare type
//   parameter, it could in theory be a local type for some other impl.
//   - This is fixed by replacing our impl with 3 impls, where `DB` is changed
//     concrete backends. This would mean that any third party crates adding new
//     backends would need to add the tuple impls, which sucks but is fine.
// - It conflicts with our impl for `Option`
//   - So we could in theory fix this by both splitting the generic impl into
//     backend specific impls, and removing the `FromSql` impls. In theory there
//     is no reason that it needs to implement `FromSql`, since everything
//     requires `FromSqlRow`, but it really feels like it should.
//   - Specialization might also fix this one. The impl isn't quite a strict
//     subset (the `FromSql` impl has `T: FromSql`, and the `FromSqlRow` impl
//     has `T: FromSqlRow`), but if `FromSql` implies `FromSqlRow`,
//     specialization might consider that a subset?
// - I don't know that we really need it. `#[derive(FromSqlRow)]` is probably
//   good enough. That won't improve our own codebase, since 99% of our
//   `FromSqlRow` impls are for types from another crate, but it's almost
//   certainly good enough for user types.
//   - Still, it really feels like `FromSql` *should* be able to imply both
//   `FromSqlRow` and `Queryable`
