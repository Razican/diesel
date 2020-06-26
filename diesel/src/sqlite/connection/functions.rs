extern crate libsqlite3_sys as ffi;

use super::raw::RawConnection;
use super::serialized_value::SerializedValue;
use super::{Sqlite, SqliteAggregateFunction, SqliteValue};
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::Field;
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::{HasSqlType, Typed};
use std::marker::PhantomData;

pub fn register<ArgsSqlType, RetSqlType, Args, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    deterministic: bool,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut(&RawConnection, Args) -> Ret + Send + 'static,
    Args: FromSqlRow<Typed<ArgsSqlType>, Sqlite> + StaticallySizedRow<Typed<ArgsSqlType>, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let fields_needed = Args::FIELD_COUNT;
    if fields_needed > 127 {
        return Err(Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new("SQLite functions cannot take more than 127 parameters".to_string()),
        ));
    }

    conn.register_sql_function(fn_name, fields_needed, deterministic, move |conn, args| {
        let args = build_sql_function_args::<ArgsSqlType, Args>(args)?;

        let result = f(conn, args);

        process_sql_function_result::<RetSqlType, Ret>(result)
    })?;
    Ok(())
}

pub fn register_aggregate<ArgsSqlType, RetSqlType, Args, Ret, A>(
    conn: &RawConnection,
    fn_name: &str,
) -> QueryResult<()>
where
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send,
    Args: FromSqlRow<Typed<ArgsSqlType>, Sqlite> + StaticallySizedRow<Typed<ArgsSqlType>, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let fields_needed = Args::FIELD_COUNT;
    if fields_needed > 127 {
        return Err(Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new("SQLite functions cannot take more than 127 parameters".to_string()),
        ));
    }

    conn.register_aggregate_function::<ArgsSqlType, RetSqlType, Args, Ret, A>(
        fn_name,
        fields_needed,
    )?;

    Ok(())
}

pub(crate) fn build_sql_function_args<ArgsSqlType, Args>(
    args: &[*mut ffi::sqlite3_value],
) -> Result<Args, Error>
where
    Args: FromSqlRow<Typed<ArgsSqlType>, Sqlite>,
{
    let mut row = FunctionRow::new(args);
    Args::build_from_row(&mut row).map_err(Error::DeserializationError)
}

pub(crate) fn process_sql_function_result<RetSqlType, Ret>(
    result: Ret,
) -> QueryResult<SerializedValue>
where
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let mut buf = Output::new(Vec::new(), &());
    let is_null = result.to_sql(&mut buf).map_err(Error::SerializationError)?;

    let bytes = if let IsNull::Yes = is_null {
        None
    } else {
        Some(buf.into_inner())
    };

    Ok(SerializedValue {
        ty: Sqlite::metadata(&()),
        data: bytes,
    })
}

#[derive(Clone)]
struct FunctionRow<'a> {
    column_count: usize,
    args: &'a [*mut ffi::sqlite3_value],
}

impl<'a> FunctionRow<'a> {
    fn new(args: &'a [*mut ffi::sqlite3_value]) -> Self {
        Self {
            column_count: args.len(),
            args,
        }
    }
}

impl<'a> ExactSizeIterator for FunctionRow<'a> {}

impl<'a> Iterator for FunctionRow<'a> {
    type Item = FunctionArgument<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.args.split_first().map(|(&first, rest)| {
            self.args = rest;
            FunctionArgument {
                arg: first,
                p: PhantomData,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.column_count, Some(self.column_count))
    }
}

struct FunctionArgument<'a> {
    arg: *mut ffi::sqlite3_value,
    p: PhantomData<&'a ()>,
}

impl<'a> Field<'a, Sqlite> for FunctionArgument<'a> {
    fn column_name(&self) -> Option<&str> {
        None
    }

    fn is_null(&self) -> bool {
        dbg!(self.value().is_none())
    }

    fn value(&self) -> Option<crate::backend::RawValue<'a, Sqlite>> {
        unsafe { SqliteValue::new(self.arg) }
    }
}
