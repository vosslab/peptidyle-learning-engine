//! PostgreSQL-clock-derived term and deadline values for the live oracle.

use super::*;

pub(super) struct OracleTimingWindow {
    start_date: String,
    end_date: String,
    closes_at: ActivityTimestamp,
}

impl OracleTimingWindow {
    /// Uses one database snapshot to derive a valid term and an elapsed close.
    pub(super) async fn from_database(pool: &PgPool) -> Self {
        let (start_date, end_date, closes_at_millis, database_now_millis):
            (String, String, i64, i64) = sqlx::query_as(
            "SELECT \
                to_char((CURRENT_TIMESTAMP AT TIME ZONE 'America/Chicago' - interval '7 days')::date, 'YYYY-MM-DD'), \
                to_char((CURRENT_TIMESTAMP AT TIME ZONE 'America/Chicago' + interval '7 days')::date, 'YYYY-MM-DD'), \
                floor(extract(epoch FROM (CURRENT_TIMESTAMP - interval '1 minute')) * 1000)::bigint, \
                floor(extract(epoch FROM CURRENT_TIMESTAMP) * 1000)::bigint",
        )
        .fetch_one(pool)
        .await
        .expect("derive fixture term and elapsed close from PostgreSQL");
        assert!(
            closes_at_millis < database_now_millis,
            "database-clock close is elapsed before the public AutoSubmit transition"
        );
        Self {
            start_date,
            end_date,
            closes_at: ActivityTimestamp::from_unix_millis(closes_at_millis),
        }
    }

    pub(super) fn start_date(&self) -> &str {
        &self.start_date
    }

    pub(super) fn end_date(&self) -> &str {
        &self.end_date
    }

    pub(super) fn closes_at(&self) -> ActivityTimestamp {
        self.closes_at
    }
}
