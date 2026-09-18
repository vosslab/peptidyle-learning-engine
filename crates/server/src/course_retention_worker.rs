//! Closed orchestration for stored Course-retention actions.
//!
//! The executor Store supplies database-owned due actions and transitions; the
//! notifier Store owns recipient receipts, leases, and idempotency. This module
//! only orders those two capabilities. It has no policy calculation, Account
//! lookup, listener, object, session, renderer, or provider configuration.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use learning_data_access::{
    CourseRetentionDueActionKind, CourseRetentionNotificationStore, CourseRetentionStore,
    StoreError,
};
use question_model::Timestamp;

use crate::course_retention_notification_delivery::{
    CourseRetentionNotificationDelivery, claim_and_deliver_one_course_retention_notification,
};

const RETENTION_SWEEP_INTERVAL: Duration = Duration::from_secs(60);
const RETENTION_NOTIFICATION_LEASE_SECONDS: u16 = 60;

/// Runs stored due actions until termination with no listener or extra process
/// authority. A worker pass starts immediately, then repeats on a fixed
/// operational interval; PostgreSQL remains the authority for all dates.
pub async fn run_until_shutdown<R, N, D>(retention: R, notifications: N, delivery: D) -> Result<()>
where
    R: CourseRetentionStore,
    N: CourseRetentionNotificationStore,
    D: CourseRetentionNotificationDelivery,
{
    let mut interval = tokio::time::interval(RETENTION_SWEEP_INTERVAL);
    loop {
        tokio::select! {
            result = shutdown_signal() => {
                result?;
                tracing::info!(event = "course_retention_worker_shutdown_requested");
                return Ok(());
            }
            _ = interval.tick() => {
                if let Err(error) = run_iteration(&retention, &notifications, &delivery).await {
                    tracing::warn!(event = "course_retention_iteration_failed", error = %error);
                }
            }
        }
    }
}

/// Performs one stored-action pass for a single server-observed instant.
///
/// Notices are claimed in the notifier's stored due-time order before any
/// inactivity/archive/delete transition. A recorded pre-acceptance delivery failure has
/// a database-owned retry time, so this pass continues with the next receipt.
/// Only a typed notifier-boundary error whose receipt state is unknown stops
/// the drain; neither outcome blocks the later transition stage.
pub async fn run_iteration<R, N, D>(retention: &R, notifications: &N, delivery: &D) -> Result<()>
where
    R: CourseRetentionStore,
    N: CourseRetentionNotificationStore,
    D: CourseRetentionNotificationDelivery,
{
    let evaluated_at = now();
    let actions = retention
        .list_due_course_retention_actions(evaluated_at)
        .await
        .map_err(retention_unavailable)?;

    loop {
        let notification = claim_and_deliver_one_course_retention_notification(
            notifications,
            delivery,
            evaluated_at,
            RETENTION_NOTIFICATION_LEASE_SECONDS,
        )
        .await;
        match notification {
            Err(error) => {
                // A notifier/database failure cannot delay a stored lifecycle
                // transition. The receipt remains the sole retry authority.
                tracing::warn!(event = "course_retention_notification_attempt_failed", error = %error);
                break;
            }
            Ok(false) => break,
            Ok(true) => {}
        }
    }

    for action in actions {
        let result = match action.action {
            CourseRetentionDueActionKind::MarkInactive => {
                retention
                    .mark_course_instance_inactive(action.course.clone(), evaluated_at)
                    .await
            }
            CourseRetentionDueActionKind::Archive => {
                retention
                    .archive_course_student_records(action.course.clone(), evaluated_at)
                    .await
            }
            CourseRetentionDueActionKind::Delete => {
                retention
                    .delete_course_student_records(action.course.clone(), evaluated_at)
                    .await
            }
            CourseRetentionDueActionKind::WarnInactive
            | CourseRetentionDueActionKind::NotifyArchive => continue,
        };
        if let Err(error) = result {
            tracing::warn!(
                event = "course_retention_transition_failed",
                course_id = %action.course,
                action = ?action.action,
                error = %error,
            );
        }
    }
    Ok(())
}

fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

fn retention_unavailable(error: StoreError) -> anyhow::Error {
    anyhow!("Course-retention actions unavailable: {error}")
}

async fn shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result?,
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;
    Ok(())
}
