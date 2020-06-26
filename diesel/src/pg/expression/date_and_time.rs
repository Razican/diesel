use crate::expression::{Expression, ValidGrouping};
use crate::pg::Pg;
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::{Date, NotNull, Nullable, SqlType, Timestamp, Timestamptz, Typed, VarChar};

/// Marker trait for types which are valid in `AT TIME ZONE` expressions
pub trait DateTimeLike {}
impl DateTimeLike for Date {}
impl DateTimeLike for Timestamp {}
impl DateTimeLike for Timestamptz {}
impl<T> DateTimeLike for Nullable<T> where T: SqlType<IsNull = NotNull> + DateTimeLike {}
impl<T> DateTimeLike for Typed<T> where T: DateTimeLike + SqlType {}
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
pub struct AtTimeZone<Ts, Tz> {
    timestamp: Ts,
    timezone: Tz,
}

impl<Ts, Tz> AtTimeZone<Ts, Tz> {
    pub fn new(timestamp: Ts, timezone: Tz) -> Self {
        AtTimeZone {
            timestamp: timestamp,
            timezone: timezone,
        }
    }
}

impl<Ts, Tz> Expression for AtTimeZone<Ts, Tz>
where
    Ts: Expression,
    Ts::SqlType: DateTimeLike,
    Tz: Expression<SqlType = Typed<VarChar>>,
{
    type SqlType = Typed<Timestamp>;
}

impl<Ts, Tz> QueryFragment<Pg> for AtTimeZone<Ts, Tz>
where
    Ts: QueryFragment<Pg>,
    Tz: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        self.timestamp.walk_ast(out.reborrow())?;
        out.push_sql(" AT TIME ZONE ");
        self.timezone.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl_selectable_expression!(AtTimeZone<Ts, Tz>);
