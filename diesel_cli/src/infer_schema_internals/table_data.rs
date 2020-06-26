use diesel::backend::Backend;
use diesel::{
    deserialize::{FromSqlRow, StaticallySizedRow},
    sql_types::Typed,
};
use std::fmt;
use std::str::FromStr;

use super::data_structures::ColumnDefinition;
use super::inference;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableName {
    pub sql_name: String,
    pub rust_name: String,
    pub schema: Option<String>,
}

impl TableName {
    pub fn from_name<T: Into<String>>(name: T) -> Self {
        let name = name.into();

        TableName {
            rust_name: inference::rust_name_for_sql_name(&name),
            sql_name: name,
            schema: None,
        }
    }

    pub fn new<T, U>(name: T, schema: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        let name = name.into();

        TableName {
            rust_name: inference::rust_name_for_sql_name(&name),
            sql_name: name,
            schema: Some(schema.into()),
        }
    }

    #[cfg(feature = "uses_information_schema")]
    pub fn strip_schema_if_matches(&mut self, schema: &str) {
        if self.schema.as_ref().map(|s| &**s) == Some(schema) {
            self.schema = None;
        }
    }

    pub fn full_sql_name(&self) -> String {
        match self.schema {
            Some(ref schema_name) => format!("{}.{}", schema_name, self.sql_name),
            None => self.sql_name.to_string(),
        }
    }
}

impl<ST, DB> FromSqlRow<Typed<ST>, DB> for TableName
where
    DB: Backend,
    (String, String): FromSqlRow<Typed<ST>, DB>,
{
    fn build_from_row<'a, T: diesel::row::Row<'a, DB>>(
        row: &mut T,
    ) -> diesel::deserialize::Result<Self>
    where
        T::Item: diesel::row::Field<'a, DB>,
    {
        let (name, schema) = <(String, String) as FromSqlRow<Typed<ST>, DB>>::build_from_row(row)?;

        Ok(TableName::new(name, schema))
    }
    fn is_null<'a, T: diesel::row::Row<'a, DB>>(row: &mut T) -> bool
    where
        T::Item: diesel::row::Field<'a, DB>,
    {
        <(String, String) as FromSqlRow<Typed<ST>, DB>>::is_null(row)
    }
}

impl<ST, DB> StaticallySizedRow<Typed<ST>, DB> for TableName
where
    DB: Backend,
    Self: FromSqlRow<Typed<ST>, DB>,
{
    const FIELD_COUNT: usize = 2;
}

impl fmt::Display for TableName {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.schema {
            Some(ref schema_name) => write!(out, "{}.{}", schema_name, self.rust_name),
            None => write!(out, "{}", self.rust_name),
        }
    }
}

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub enum Never {}

impl FromStr for TableName {
    type Err = Never;

    fn from_str(table_name: &str) -> Result<Self, Self::Err> {
        let mut parts = table_name.split('.');
        match (parts.next(), parts.next()) {
            (Some(schema), Some(name)) => Ok(TableName::new(name, schema)),
            _ => Ok(TableName::from_name(table_name)),
        }
    }
}

#[derive(Debug)]
pub struct TableData {
    pub name: TableName,
    pub primary_key: Vec<String>,
    pub column_data: Vec<ColumnDefinition>,
    pub docs: String,
}

mod serde_impls {
    extern crate serde;

    use self::serde::de::Visitor;
    use self::serde::{de, Deserialize, Deserializer};
    use super::TableName;
    use std::fmt;

    impl<'de> Deserialize<'de> for TableName {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct TableNameVisitor;

            impl<'de> Visitor<'de> for TableNameVisitor {
                type Value = TableName;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("A valid table name")
                }

                fn visit_str<E>(self, value: &str) -> Result<TableName, E>
                where
                    E: de::Error,
                {
                    value.parse().map_err(|_| unreachable!())
                }
            }

            deserializer.deserialize_string(TableNameVisitor)
        }
    }
}
