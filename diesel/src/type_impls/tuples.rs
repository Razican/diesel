use crate::associations::BelongsTo;
use crate::backend::Backend;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::expression::{
    AppearsOnTable, AsExpression, AsExpressionList, Expression, SelectableExpression, ValidGrouping,
};
use crate::insertable::{CanInsertInSingleQuery, InsertValues, Insertable};
use crate::query_builder::*;
use crate::query_source::*;
use crate::result::QueryResult;
use crate::row::*;
use crate::sql_types::{HasSqlType, IntoNullable, MixedNullable, Nullable, SqlType, Typed};
use crate::util::TupleAppend;

macro_rules! tuple_impls {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<$($T),+, __DB> HasSqlType<($(Typed<$T>,)+)> for __DB where
                $(__DB: HasSqlType<$T>),+,
                __DB: Backend,
            {
                fn metadata(_: &__DB::MetadataLookup) -> __DB::TypeMetadata {
                    unreachable!("Tuples should never implement `ToSql` directly");
                }

                #[cfg(feature = "mysql")]
                fn mysql_row_metadata(out: &mut Vec<__DB::TypeMetadata>, lookup: &__DB::MetadataLookup) {
                    $(<__DB as HasSqlType<$T>>::mysql_row_metadata(out, lookup);)+
                }
            }

            impl_from_sql_row!($Tuple, ($($T,)+), ($($ST,)+));

            impl<$($T),+, $($ST),+, __DB > StaticallySizedRow<Typed<($($ST,)*)>, __DB> for ($($T,)+) where
                __DB: Backend,
                Self: FromSqlRow<Typed<($($ST,)+)>, __DB>,
                $($T: StaticallySizedRow<$ST, __DB>,)+
            {
                const FIELD_COUNT: usize = $($T::FIELD_COUNT +)+ 0;
            }

            impl<$($T: Expression),+> Expression for ($($T,)+) {
                type SqlType = Typed<($(<$T as Expression>::SqlType,)+)>;
            }

            impl<$($T: SqlType,)*> IntoNullable for ($(Typed<$T>,)*)
                where Self: SqlType,
            {
                type Nullable = Nullable<($(Typed<$T>,)*)>;
            }

            impl<$($T: QueryFragment<__DB>),+, __DB: Backend> QueryFragment<__DB> for ($($T,)+) {
                #[allow(unused_assignments)]
                fn walk_ast(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
                    let mut needs_comma = false;
                    $(
                        if !self.$idx.is_noop()? {
                            if needs_comma {
                                out.push_sql(", ");
                            }
                            self.$idx.walk_ast(out.reborrow())?;
                            needs_comma = true;
                        }
                    )+
                    Ok(())
                }
            }

            impl<$($T,)+ Tab> ColumnList for ($($T,)+)
            where
                $($T: ColumnList<Table = Tab>,)+
            {
                type Table = Tab;

                fn walk_ast<__DB: Backend>(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
                    $(
                        if $idx != 0 {
                            out.push_sql(", ");
                        }
                        self.$idx.walk_ast(out.reborrow())?;
                    )+
                    Ok(())
                }
            }

            impl<$($T: QueryId),+> QueryId for ($($T,)+) {
                type QueryId = ($($T::QueryId,)+);

                const HAS_STATIC_QUERY_ID: bool = $($T::HAS_STATIC_QUERY_ID &&)+ true;
            }

            const _: () = {
                #[derive(ValidGrouping)]
                #[diesel(foreign_derive)]
                struct TupleWrapper<$($T,)*>(($($T,)*));
            };

            impl<$($T,)+ Tab> UndecoratedInsertRecord<Tab> for ($($T,)+)
            where
                $($T: UndecoratedInsertRecord<Tab>,)+
            {
            }

            impl<$($T,)+ __DB> CanInsertInSingleQuery<__DB> for ($($T,)+)
            where
                __DB: Backend,
                $($T: CanInsertInSingleQuery<__DB>,)+
            {
                fn rows_to_insert(&self) -> Option<usize> {
                    $(debug_assert_eq!(self.$idx.rows_to_insert(), Some(1));)+
                    Some(1)
                }
            }

            impl<$($T,)+ $($ST,)+ Tab> Insertable<Tab> for ($($T,)+)
            where
                $($T: Insertable<Tab, Values = ValuesClause<$ST, Tab>>,)+
            {
                type Values = ValuesClause<($($ST,)+), Tab>;

                fn values(self) -> Self::Values {
                    ValuesClause::new(($(self.$idx.values().values,)+))
                }
            }

            impl<'a, $($T,)+ Tab> Insertable<Tab> for &'a ($($T,)+)
            where
                ($(&'a $T,)+): Insertable<Tab>,
            {
                type Values = <($(&'a $T,)+) as Insertable<Tab>>::Values;

                fn values(self) -> Self::Values {
                    ($(&self.$idx,)+).values()
                }
            }

            #[allow(unused_assignments)]
            impl<$($T,)+ Tab, __DB> InsertValues<Tab, __DB> for ($($T,)+)
            where
                Tab: Table,
                __DB: Backend,
                $($T: InsertValues<Tab, __DB>,)+
            {
                fn column_names(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
                    let mut needs_comma = false;
                    $(
                        let noop_element = self.$idx.is_noop()?;
                        if !noop_element {
                            if needs_comma {
                                out.push_sql(", ");
                            }
                            self.$idx.column_names(out.reborrow())?;
                            needs_comma = true;
                        }
                    )+
                    Ok(())
                }
            }

            impl<$($T,)+ QS> SelectableExpression<QS> for ($($T,)+) where
                $($T: SelectableExpression<QS>,)+
                ($($T,)+): AppearsOnTable<QS>,
            {
            }

            impl<$($T,)+ QS> AppearsOnTable<QS> for ($($T,)+) where
                $($T: AppearsOnTable<QS>,)+
                ($($T,)+): Expression,
            {
            }

            impl<Target, $($T,)+> AsChangeset for ($($T,)+) where
                $($T: AsChangeset<Target=Target>,)+
                Target: QuerySource,
            {
                type Target = Target;
                type Changeset = ($($T::Changeset,)+);

                fn as_changeset(self) -> Self::Changeset {
                    ($(self.$idx.as_changeset(),)+)
                }
            }

            impl<$($T,)+ Parent> BelongsTo<Parent> for ($($T,)+) where
                A: BelongsTo<Parent>,
            {
                type ForeignKey = A::ForeignKey;
                type ForeignKeyColumn = A::ForeignKeyColumn;

                fn foreign_key(&self) -> Option<&Self::ForeignKey> {
                    self.0.foreign_key()
                }

                fn foreign_key_column() -> Self::ForeignKeyColumn {
                    A::foreign_key_column()
                }
            }

            impl<$($T,)+ Next> TupleAppend<Next> for ($($T,)+) {
                type Output = ($($T,)+ Next);

                #[allow(non_snake_case)]
                fn tuple_append(self, next: Next) -> Self::Output {
                    let ($($T,)+) = self;
                    ($($T,)+ next)
                }
            }

            impl<$($T,)+ ST> AsExpressionList<ST> for ($($T,)+) where
                $($T: AsExpression<ST>,)+
                ST: SqlType,
            {
                type Expression = ($($T::Expression,)+);

                fn as_expression_list(self) -> Self::Expression {
                    ($(self.$idx.as_expression(),)+)
                }
            }

            impl_sql_type!($($T,)*);
        )+
    }
}

macro_rules! impl_from_sql_row {
    ($Tuple: expr, ($T1: ident, $($T: ident,)*), ($ST1: ident, $($ST: ident,)*)) => {
        impl<$T1, $ST1, $($T,)* $($ST,)* __DB> FromSqlRow<Typed<($($ST,)* $ST1,)>, __DB> for ($($T,)* $T1,) where
            __DB: Backend,
            $T1: FromSqlRow<$ST1, __DB>,
            $(
                $T: FromSqlRow<$ST, __DB> + StaticallySizedRow<$ST, __DB>,
            )*

        {

            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn build_from_row<'a, RowT: Row<'a, __DB>>(row: &mut RowT)
                -> crate::deserialize::Result<Self>
            where
                RowT::Item: Field<'a, __DB>,
            {
                Ok(($(
                    $T::build_from_row(row)?,
                )* $T1::build_from_row(row)?,))
            }

            #[allow(non_snake_case)]
            fn is_null<'a, RowT: Row<'a, __DB>>(row: &mut RowT) -> bool
            where
                RowT::Item: Field<'a, __DB>,
            {
                $(
                    let $ST = $T::is_null(row);
                )*

                let $ST1 = $T1::is_null(row);

                $($ST ||)* $ST1
            }
        }
    }
}

macro_rules! impl_sql_type {
    ($T1: ident, $($T: ident,)+) => {
        impl<$T1, $($T,)+> SqlType for (Typed<$T1>, $(Typed<$T>,)*)
        where $T1: SqlType,
              ($(Typed<$T>,)*): SqlType,
              $T1::IsNull: MixedNullable<<($(Typed<$T>,)*) as SqlType>::IsNull>,
        {
            type IsNull = <$T1::IsNull as MixedNullable<<($(Typed<$T>,)*) as SqlType>::IsNull>>::Out;
        }
    };
    ($T1: ident,) => {
        impl<$T1> SqlType for (Typed<$T1>,)
        where $T1: SqlType,
        {
            type IsNull = $T1::IsNull;
        }
    }
}

__diesel_for_each_tuple!(tuple_impls);
