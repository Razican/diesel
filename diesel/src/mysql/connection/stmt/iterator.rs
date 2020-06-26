use super::{metadata::MysqlFieldMetadata, BindData, Binds, Statement, StatementMetadata};
use crate::mysql::{Mysql, MysqlType};
use crate::result::QueryResult;
use crate::row::*;

pub struct StatementIterator<'a> {
    stmt: &'a mut Statement,
    output_binds: Binds,
    metadata: StatementMetadata,
}

#[allow(clippy::should_implement_trait)] // don't neet `Iterator` here
impl<'a> StatementIterator<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(stmt: &'a mut Statement, types: Option<Vec<MysqlType>>) -> QueryResult<Self> {
        let metadata = stmt.metadata()?;
        let mut output_binds = if let Some(types) = types {
            Binds::from_output_types(types)
        } else {
            Binds::from_result_metadata(&metadata)
        };

        unsafe {
            stmt.execute_statement(&mut output_binds)?;
        }

        Ok(StatementIterator {
            stmt,
            output_binds,
            metadata,
        })
    }

    pub fn map<F, T>(mut self, mut f: F) -> QueryResult<Vec<T>>
    where
        F: FnMut(MysqlRow) -> QueryResult<T>,
    {
        let mut results = Vec::new();
        while let Some(row) = self.next() {
            results.push(f(row?)?);
        }
        Ok(results)
    }

    fn next(&mut self) -> Option<QueryResult<MysqlRow>> {
        match self.stmt.populate_row_buffers(&mut self.output_binds) {
            Ok(Some(())) => Some(Ok(MysqlRow {
                col_idx: 0,
                binds: &mut self.output_binds,
                metadata: &self.metadata,
            })),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[derive(Clone)]
pub struct MysqlRow<'a> {
    col_idx: usize,
    binds: &'a Binds,
    metadata: &'a StatementMetadata,
}

impl<'a> ExactSizeIterator for MysqlRow<'a> {}

impl<'a> Iterator for MysqlRow<'a> {
    type Item = MysqlField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_idx = self.col_idx;
        if next_idx < self.binds.len() {
            self.col_idx += 1;
            Some(MysqlField {
                bind: &self.binds[next_idx],
                metadata: &self.metadata.fields()[next_idx],
            })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.binds.len(), Some(self.binds.len()))
    }
}

pub struct MysqlField<'a> {
    bind: &'a BindData,
    metadata: &'a MysqlFieldMetadata<'a>,
}

impl<'a> Field<'a, Mysql> for MysqlField<'a> {
    fn column_name(&self) -> Option<&str> {
        self.metadata.field_name()
    }

    fn is_null(&self) -> bool {
        self.bind.is_null()
    }

    fn value(&self) -> Option<crate::backend::RawValue<'a, Mysql>> {
        self.bind.value()
    }
}

// pub struct NamedStatementIterator<'a> {
//     stmt: &'a mut Statement,
//     output_binds: Binds,
//     metadata: StatementMetadata,
// }

// #[allow(clippy::should_implement_trait)] // don't need `Iterator` here
// impl<'a> NamedStatementIterator<'a> {
//     #[allow(clippy::new_ret_no_self)]
//     pub fn new(stmt: &'a mut Statement) -> QueryResult<Self> {
//         let metadata = stmt.metadata()?;
//         let mut output_binds = Binds::from_result_metadata(&metadata);

//         stmt.execute_statement(&mut output_binds)?;

//         Ok(NamedStatementIterator {
//             stmt,
//             output_binds,
//             metadata,
//         })
//     }

//     pub fn map<F, T>(mut self, mut f: F) -> QueryResult<Vec<T>>
//     where
//         F: FnMut(NamedMysqlRow) -> QueryResult<T>,
//     {
//         let mut results = Vec::new();
//         while let Some(row) = self.next() {
//             results.push(f(row?)?);
//         }
//         Ok(results)
//     }

//     fn next(&mut self) -> Option<QueryResult<NamedMysqlRow>> {
//         match self.stmt.populate_row_buffers(&mut self.output_binds) {
//             Ok(Some(())) => Some(Ok(NamedMysqlRow {
//                 binds: &self.output_binds,
//                 column_indices: self.metadata.column_indices(),
//                 metadata: &self.metadata,
//             })),
//             Ok(None) => None,
//             Err(e) => Some(Err(e)),
//         }
//     }
// }

// pub struct NamedMysqlRow<'a> {
//     binds: &'a Binds,
//     column_indices: &'a HashMap<&'a str, usize>,
//     metadata: &'a StatementMetadata,
// }

// impl<'a> NamedRow<Mysql> for NamedMysqlRow<'a> {
//     fn index_of(&self, column_name: &str) -> Option<usize> {
//         self.column_indices.get(column_name).cloned()
//     }

//     fn get_raw_value(&self, idx: usize) -> Option<MysqlValue<'_>> {
//         self.binds.field_data(idx)
//     }

//     fn field_names(&self) -> Vec<&str> {
//         self.metadata
//             .fields()
//             .iter()
//             .filter_map(|f| f.field_name())
//             .collect()
//     }
// }
