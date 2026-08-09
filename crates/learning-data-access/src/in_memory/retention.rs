//! In-memory retention lifecycle scheduling, API, and worker capability.
//!
//! The parent module owns the shared state graph; this module owns every
//! retention Store trait implementation and its mutation helper.

use question_model::UserRole;

use super::*;
use crate::SessionSubject;

#[cfg(test)]
mod tests;
mod worker;

#[async_trait]
impl RetentionStore for MemoryStore {
    async fn configure_retention_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        policy: InstitutionRetentionPolicy,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if !subject.roles().contains(&UserRole::Administrator) {
            return Err(StoreError::Forbidden);
        }
        state.retention_policies.insert(context.tenant_id(), policy);
        Ok(())
    }

    async fn end_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )?;
        let key = (context.tenant_id(), course);
        if let Some(existing) = state.course_retention.get(&key).copied() {
            return Ok(existing);
        }
        let policy = state
            .retention_policies
            .get(&context.tenant_id())
            .copied()
            .unwrap_or_default();
        let snapshot = CourseRetentionSnapshot::new(
            state.authoritative_time,
            policy,
            AssignmentDefinitionDisposition::Retain,
            1,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let record = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                AssignmentDefinitionDisposition::Retain,
            ),
        };
        state.course_retention.insert(key, record);
        for stage in [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ] {
            let due_at = policy
                .due_at(state.authoritative_time, stage)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            state.retention_stages.insert(
                (context.tenant_id(), course, stage, 1),
                StoredRetentionStage {
                    due_at,
                    state: RetentionStageWorkState::Scheduled,
                    job: None,
                    lease: None,
                },
            );
        }
        Ok(record)
    }

    async fn course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionRecord>, StoreError> {
        let state = self.read_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(state
            .course_retention
            .get(&(context.tenant_id(), course))
            .copied())
    }
}

#[async_trait]
impl RetentionScheduleStore for MemoryStore {
    async fn dispatch_due_retention_stages(
        &self,
        batch: RetentionDispatchBatch,
    ) -> Result<u16, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let mut candidates = Vec::new();
        for (key @ (tenant, course, stage, generation), stored) in &state.retention_stages {
            if candidates.len() >= usize::from(batch.get())
                || stored.state != RetentionStageWorkState::Scheduled
                || stored.due_at > now
                || state.retention_dispatches.contains_key(key)
            {
                continue;
            }
            let Some(record) = state.course_retention.get(&(*tenant, *course)) else {
                continue;
            };
            if record.snapshot.generation() != *generation
                || (record.status.state != CourseRetentionState::Active
                    && !(record.status.state == CourseRetentionState::StudentRecordsArchived
                        && *stage == crate::RetentionStage::DeleteStudentRecords))
            {
                continue;
            }
            candidates.push((*key, *stage));
        }
        let mut jobs = Vec::with_capacity(candidates.len());
        for (key, stage) in &candidates {
            jobs.push((
                *key,
                crate::JobId::generate()?,
                JobPayload::Retention {
                    course: key.1,
                    stage: *stage,
                    generation: key.3,
                },
            ));
        }
        for (key, id, payload) in &jobs {
            state.jobs.insert(
                *id,
                StoredJob {
                    tenant: key.0,
                    payload: payload.clone(),
                    state: JobState::Ready,
                    available_at: now,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: RETENTION_JOB_MAX_ATTEMPTS,
                    failure: None,
                },
            );
            state.retention_dispatches.insert(*key, *id);
        }
        u16::try_from(jobs.len()).map_err(|_| {
            StoreError::Unavailable("retention dispatch count exceeds u16".to_string())
        })
    }

    async fn extend_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if !subject.roles().contains(&UserRole::Administrator) {
            return Err(StoreError::Forbidden);
        }
        let key = (context.tenant_id(), course);
        if !state.courses.contains_key(&key) {
            return Err(StoreError::Forbidden);
        }
        let record = state
            .course_retention
            .get(&key)
            .copied()
            // An existing course with no ended schedule is a lifecycle conflict,
            // while the preceding existence guard keeps a missing course
            // nonenumerating. PostgreSQL's broker uses the same distinction.
            .ok_or(StoreError::Conflict)?;
        if record.status.state != CourseRetentionState::Active {
            return Err(StoreError::Conflict);
        }
        let old_generation = record.snapshot.generation();
        let new_generation = old_generation.checked_add(1).ok_or_else(|| {
            StoreError::InvalidRecord("retention generation overflow".to_string())
        })?;
        let stages = [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ];
        let old = stages
            .iter()
            .map(|stage| {
                state
                    .retention_stages
                    .get(&(key.0, key.1, *stage, old_generation))
                    .copied()
                    .ok_or(StoreError::Conflict)
                    .map(|stored| (*stage, stored))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if old
            .iter()
            .any(|(_, stored)| stored.state == RetentionStageWorkState::Started)
        {
            return Err(StoreError::Conflict);
        }
        let shift_millis = i64::from(additional_days.get())
            .checked_mul(86_400_000)
            .ok_or_else(|| {
                StoreError::InvalidRecord("retention extension overflows".to_string())
            })?;
        let mut replacement = Vec::with_capacity(old.len());
        for (stage, stored) in &old {
            let next = match stored.state {
                RetentionStageWorkState::Completed => StoredRetentionStage {
                    due_at: stored.due_at,
                    state: RetentionStageWorkState::Completed,
                    job: None,
                    lease: None,
                },
                RetentionStageWorkState::Scheduled => StoredRetentionStage {
                    due_at: ActivityTimestamp::from_unix_millis(
                        stored
                            .due_at
                            .as_unix_millis()
                            .checked_add(shift_millis)
                            .ok_or_else(|| {
                                StoreError::InvalidRecord(
                                    "retention extension timestamp overflows".to_string(),
                                )
                            })?,
                    ),
                    state: RetentionStageWorkState::Scheduled,
                    job: None,
                    lease: None,
                },
                RetentionStageWorkState::Started | RetentionStageWorkState::Superseded => {
                    return Err(StoreError::Conflict);
                }
            };
            replacement.push((*stage, next));
        }
        for (stage, stored) in &old {
            let old_key = (key.0, key.1, *stage, old_generation);
            if stored.state == RetentionStageWorkState::Scheduled {
                if let Some(job) = state.retention_dispatches.get(&old_key).copied()
                    && let Some(job) = state.jobs.get_mut(&job)
                    && matches!(job.state, JobState::Ready | JobState::Leased)
                {
                    job.state = JobState::Dead;
                    job.lease_token = None;
                    job.lease_expires_at = None;
                    job.failure = Some(JobFailureKind::Permanent);
                }
                state.retention_stages.insert(
                    old_key,
                    StoredRetentionStage {
                        state: RetentionStageWorkState::Superseded,
                        ..*stored
                    },
                );
            }
        }
        let snapshot = record
            .snapshot
            .with_generation_and_disposition(new_generation, record.status.assignment_definitions)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let updated = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                record.status.assignment_definitions,
            ),
        };
        state.course_retention.insert(key, updated);
        for (stage, stored) in replacement {
            state
                .retention_stages
                .insert((key.0, key.1, stage, new_generation), stored);
        }
        Ok(updated)
    }

    async fn set_archive_disposition(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )?;
        let key = (context.tenant_id(), course);
        let record = state
            .course_retention
            .get(&key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        let archive_key = (
            key.0,
            key.1,
            crate::RetentionStage::ArchiveStudentRecords,
            record.snapshot.generation(),
        );
        if record.status.state != CourseRetentionState::Active
            || state
                .retention_stages
                .get(&archive_key)
                .is_none_or(|stage| stage.state != RetentionStageWorkState::Scheduled)
        {
            return Err(StoreError::Conflict);
        }
        let snapshot = record
            .snapshot
            .with_generation_and_disposition(record.snapshot.generation(), disposition)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let updated = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                disposition,
            ),
        };
        state.course_retention.insert(key, updated);
        Ok(updated)
    }
}

#[async_trait]
impl RetentionApiStore for MemoryStore {
    async fn retention_view(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionView>, StoreError> {
        let state = self.read_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )
        .is_err()
        {
            return Ok(None);
        }
        state
            .course_retention
            .get(&(context.tenant_id(), course))
            .copied()
            .map(|record| {
                record
                    .safe_view()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
            })
            .transpose()
    }

    async fn retention_notification(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<crate::RetentionNotificationView>, StoreError> {
        let state = self.read_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(state
            .retention_notifications
            .iter()
            .filter(|((tenant, notification_course, _), _)| {
                *tenant == context.tenant_id() && *notification_course == course
            })
            .max_by_key(|((_, _, generation), notification)| (*generation, notification.created_at))
            .map(|(_, notification)| *notification))
    }

    async fn extend_retention_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionView, StoreError> {
        Ok(self
            .mutate_retention_api(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Extend(additional_days),
            )?
            .retention)
    }

    async fn request_retention_archive_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let mutation = self.mutate_retention_api(
            context,
            session,
            course,
            expected,
            RetentionApiAction::Archive(disposition),
        )?;
        Ok(crate::RetentionRequestResult {
            retention: mutation.retention,
            outcome: mutation.manual_outcome.ok_or(StoreError::Conflict)?,
        })
    }

    async fn request_retention_delete_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let mutation = self.mutate_retention_api(
            context,
            session,
            course,
            expected,
            RetentionApiAction::Delete,
        )?;
        Ok(crate::RetentionRequestResult {
            retention: mutation.retention,
            outcome: mutation.manual_outcome.ok_or(StoreError::Conflict)?,
        })
    }
}

impl MemoryStore {
    fn mutate_retention_api(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        action: RetentionApiAction,
    ) -> Result<RetentionApiMutation, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        let actor = subject.user();
        let administrator = subject.roles().contains(&UserRole::Administrator);
        ensure_retention_course_authority(&state, context, actor, subject.roles(), course)?;
        if matches!(action, RetentionApiAction::Extend(_)) && !administrator {
            return Err(StoreError::Forbidden);
        }
        let key = (context.tenant_id(), course);
        if let Some(receipt) = state
            .retention_api_receipts
            .get(&(key.0, key.1, expected.value()))
            .copied()
        {
            if receipt.actor != actor || receipt.action != action {
                return Err(StoreError::Conflict);
            }
            let stage = state
                .retention_stages
                .get(&(key.0, key.1, receipt.stage, receipt.resulting_generation))
                .copied()
                .ok_or(StoreError::Conflict)?;
            let outcome = match stage.state {
                RetentionStageWorkState::Scheduled => crate::RetentionRequestOutcome::Scheduled,
                RetentionStageWorkState::Started => crate::RetentionRequestOutcome::InProgress,
                RetentionStageWorkState::Completed => crate::RetentionRequestOutcome::Completed,
                RetentionStageWorkState::Superseded => return Err(StoreError::Conflict),
            };
            let retention = state
                .course_retention
                .get(&key)
                .copied()
                .ok_or(StoreError::Conflict)?
                .safe_view()
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            return Ok(RetentionApiMutation {
                retention,
                manual_outcome: Some(outcome),
            });
        }
        let record = state
            .course_retention
            .get(&key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        // The returned revision is also a valid replay key for a queued
        // archive/delete action.  Once the worker archives the course, the
        // lifecycle guard below must not turn that exact completed retry into
        // a conflict.  Bind it to the original receipt rather than accepting
        // a same-revision request from another actor or with another action.
        if matches!(
            action,
            RetentionApiAction::Archive(_) | RetentionApiAction::Delete
        ) && let Some(receipt) = state.retention_api_receipts.iter().find_map(
            |((tenant, receipt_course, _), receipt)| {
                (*tenant == key.0
                    && *receipt_course == key.1
                    && receipt.resulting_generation == expected.value()
                    && receipt.actor == actor
                    && receipt.action == action)
                    .then_some(*receipt)
            },
        ) {
            let stage = state
                .retention_stages
                .get(&(key.0, key.1, receipt.stage, receipt.resulting_generation))
                .copied()
                .ok_or(StoreError::Conflict)?;
            let outcome = match stage.state {
                RetentionStageWorkState::Scheduled => crate::RetentionRequestOutcome::Scheduled,
                RetentionStageWorkState::Started => crate::RetentionRequestOutcome::InProgress,
                RetentionStageWorkState::Completed => crate::RetentionRequestOutcome::Completed,
                RetentionStageWorkState::Superseded => return Err(StoreError::Conflict),
            };
            return Ok(RetentionApiMutation {
                retention: record
                    .safe_view()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                manual_outcome: Some(outcome),
            });
        }
        if record.status.state != CourseRetentionState::Active
            || record.snapshot.generation() != expected.value()
        {
            return Err(StoreError::Conflict);
        }
        let old_generation = record.snapshot.generation();
        let new_generation = old_generation.checked_add(1).ok_or_else(|| {
            StoreError::InvalidRecord("retention generation overflow".to_string())
        })?;
        let stages = [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ];
        let old = stages
            .iter()
            .map(|stage| {
                state
                    .retention_stages
                    .get(&(key.0, key.1, *stage, old_generation))
                    .copied()
                    .ok_or(StoreError::Conflict)
                    .map(|stored| (*stage, stored))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let immediate_stage_for_replay = match action {
            RetentionApiAction::Archive(_) => Some(crate::RetentionStage::ArchiveStudentRecords),
            RetentionApiAction::Delete => Some(crate::RetentionStage::DeleteStudentRecords),
            RetentionApiAction::Extend(_) => None,
        };
        if let Some(stage) = immediate_stage_for_replay
            && let Some((_, stored)) = old.iter().find(|(candidate, _)| *candidate == stage)
        {
            let outcome = match stored.state {
                RetentionStageWorkState::Started => {
                    Some(crate::RetentionRequestOutcome::InProgress)
                }
                RetentionStageWorkState::Completed => {
                    Some(crate::RetentionRequestOutcome::Completed)
                }
                RetentionStageWorkState::Scheduled | RetentionStageWorkState::Superseded => None,
            };
            if let Some(outcome) = outcome {
                return Ok(RetentionApiMutation {
                    retention: record
                        .safe_view()
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                    manual_outcome: Some(outcome),
                });
            }
            if stored.state == RetentionStageWorkState::Scheduled
                && state
                    .retention_dispatches
                    .contains_key(&(key.0, key.1, stage, old_generation))
            {
                if matches!(action, RetentionApiAction::Archive(disposition) if disposition != record.status.assignment_definitions)
                {
                    return Err(StoreError::Conflict);
                }
                return Ok(RetentionApiMutation {
                    retention: record
                        .safe_view()
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                    manual_outcome: Some(crate::RetentionRequestOutcome::Scheduled),
                });
            }
        }
        if old
            .iter()
            .any(|(_, stored)| stored.state == RetentionStageWorkState::Started)
        {
            return Err(StoreError::Conflict);
        }
        let (extension, immediate_stage, next_disposition) = match action {
            RetentionApiAction::Extend(days) => {
                (Some(days), None, record.status.assignment_definitions)
            }
            RetentionApiAction::Archive(disposition) => (
                None,
                Some(crate::RetentionStage::ArchiveStudentRecords),
                disposition,
            ),
            RetentionApiAction::Delete => (
                None,
                Some(crate::RetentionStage::DeleteStudentRecords),
                record.status.assignment_definitions,
            ),
        };
        let shift_millis = match extension {
            Some(days) => i64::from(days.get())
                .checked_mul(86_400_000)
                .ok_or_else(|| {
                    StoreError::InvalidRecord("retention extension overflows".to_string())
                })?,
            None => 0,
        };
        let mut replacements = Vec::with_capacity(old.len());
        for (stage, stored) in &old {
            let due_at = if Some(*stage) == immediate_stage {
                state.authoritative_time
            } else if stored.state == RetentionStageWorkState::Completed {
                stored.due_at
            } else {
                ActivityTimestamp::from_unix_millis(
                    stored
                        .due_at
                        .as_unix_millis()
                        .checked_add(shift_millis)
                        .ok_or_else(|| {
                            StoreError::InvalidRecord(
                                "retention extension timestamp overflows".to_string(),
                            )
                        })?,
                )
            };
            let next_state = match stored.state {
                RetentionStageWorkState::Completed => RetentionStageWorkState::Completed,
                RetentionStageWorkState::Scheduled => RetentionStageWorkState::Scheduled,
                RetentionStageWorkState::Started | RetentionStageWorkState::Superseded => {
                    return Err(StoreError::Conflict);
                }
            };
            if Some(*stage) == immediate_stage && next_state != RetentionStageWorkState::Scheduled {
                return Err(StoreError::Conflict);
            }
            replacements.push((
                *stage,
                StoredRetentionStage {
                    due_at,
                    state: next_state,
                    job: None,
                    lease: None,
                },
            ));
        }
        for (stage, stored) in old {
            let old_key = (key.0, key.1, stage, old_generation);
            if stored.state == RetentionStageWorkState::Scheduled {
                if let Some(job_id) = state.retention_dispatches.get(&old_key).copied()
                    && let Some(job) = state.jobs.get_mut(&job_id)
                    && matches!(job.state, JobState::Ready | JobState::Leased)
                {
                    job.state = JobState::Dead;
                    job.lease_token = None;
                    job.lease_expires_at = None;
                    job.failure = Some(JobFailureKind::Permanent);
                }
                state.retention_stages.insert(
                    old_key,
                    StoredRetentionStage {
                        state: RetentionStageWorkState::Superseded,
                        ..stored
                    },
                );
            }
        }
        let snapshot = record
            .snapshot
            .with_generation_and_disposition(new_generation, next_disposition)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let updated = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                next_disposition,
            ),
        };
        state.course_retention.insert(key, updated);
        for (stage, stored) in replacements {
            state
                .retention_stages
                .insert((key.0, key.1, stage, new_generation), stored);
        }
        if let Some(stage) = immediate_stage {
            let dispatch_key = (key.0, key.1, stage, new_generation);
            let job_id = crate::JobId::generate()?;
            let available_at = state.authoritative_time;
            state.jobs.insert(
                job_id,
                StoredJob {
                    tenant: key.0,
                    payload: JobPayload::Retention {
                        course,
                        stage,
                        generation: new_generation,
                    },
                    state: JobState::Ready,
                    available_at,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: RETENTION_JOB_MAX_ATTEMPTS,
                    failure: None,
                },
            );
            state.retention_dispatches.insert(dispatch_key, job_id);
            state.retention_api_receipts.insert(
                (key.0, key.1, expected.value()),
                RetentionApiReceipt {
                    actor,
                    action,
                    resulting_generation: new_generation,
                    stage,
                },
            );
        }
        Ok(RetentionApiMutation {
            retention: updated
                .safe_view()
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
            manual_outcome: immediate_stage.map(|_| crate::RetentionRequestOutcome::Scheduled),
        })
    }
}

fn active_retention_session(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<&SessionSubject, StoreError> {
    let stored = state.sessions.get(&session).ok_or(StoreError::Forbidden)?;
    if stored.revoked
        || stored.record.expires_at <= state.authoritative_time
        || stored.record.subject.tenant() != context.tenant_id()
    {
        return Err(StoreError::Forbidden);
    }
    Ok(&stored.record.subject)
}

fn ensure_retention_course_authority(
    state: &State,
    context: TenantContext,
    user: UserId,
    roles: &[UserRole],
    course: CourseId,
) -> Result<(), StoreError> {
    let course_record = state
        .courses
        .get(&(context.tenant_id(), course))
        .ok_or(StoreError::Forbidden)?;
    if roles.contains(&UserRole::Administrator)
        || course_record.role_for(user) == Some(CourseRole::Instructor)
    {
        Ok(())
    } else {
        Err(StoreError::Forbidden)
    }
}
