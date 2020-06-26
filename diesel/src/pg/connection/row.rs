use super::result::PgResult;
use crate::pg::{Pg, PgValue};
use crate::row::*;

#[derive(Clone)]
pub struct PgRow<'a> {
    db_result: &'a PgResult,
    row_idx: usize,
    col_idx: usize,
    col_count: usize,
}

impl<'a> PgRow<'a> {
    pub fn new(db_result: &'a PgResult, row_idx: usize) -> Self {
        PgRow {
            row_idx,
            col_idx: 0,
            col_count: db_result.column_count(),
            db_result,
        }
    }
}

impl<'a> ExactSizeIterator for PgRow<'a> {}

impl<'a> Iterator for PgRow<'a> {
    type Item = PgField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_idx = self.col_idx;
        if current_idx < self.col_count {
            self.col_idx += 1;
            Some(PgField {
                db_result: self.db_result,
                row_idx: self.row_idx,
                col_idx: current_idx,
            })
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n < self.col_count {
            self.col_idx = n + 1;
            Some(PgField {
                db_result: self.db_result,
                row_idx: self.row_idx,
                col_idx: n,
            })
        } else {
            self.col_idx = self.col_count;
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.col_count, Some(self.col_count))
    }
}

pub struct PgField<'a> {
    db_result: &'a PgResult,
    row_idx: usize,
    col_idx: usize,
}

impl<'a> Field<'a, Pg> for PgField<'a> {
    fn column_name(&self) -> Option<&str> {
        self.db_result.column_name(self.col_idx)
    }

    fn value(&self) -> Option<crate::backend::RawValue<'a, Pg>> {
        let raw = self.db_result.get(self.row_idx, self.col_idx)?;

        Some(PgValue::new(raw, self.db_result.column_type(self.col_idx)))
    }
}

// pub struct PgNamedRow<'a> {
//     cursor: &'a NamedCursor,
//     idx: usize,
// }

// impl<'a> PgNamedRow<'a> {
//     pub fn new(cursor: &'a NamedCursor, idx: usize) -> Self {
//         PgNamedRow { cursor, idx }
//     }
// }

// impl<'a> NamedRow<Pg> for PgNamedRow<'a> {
//     fn get_raw_value(&self, index: usize) -> Option<PgValue<'_>> {
//         let raw = self.cursor.get_value(self.idx, index)?;
//         Some(PgValue::new(raw, self.cursor.db_result.column_type(index)))
//     }

//     fn index_of(&self, column_name: &str) -> Option<usize> {
//         self.cursor.index_of_column(column_name)
//     }

//     fn field_names(&self) -> Vec<&str> {
//         (0..self.cursor.db_result.column_count())
//             .filter_map(|i| self.cursor.db_result.column_name(i))
//             .collect()
//     }
// }
