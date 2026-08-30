//! In-memory retention worker transition capability.

use super::*;
use crate::AssetDeliveryScope;

#[async_trait]
impl RetentionWorkerStore for MemoryStore {
    async fn prepare_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<RetentionWork, StoreError> {
        let mut state = self.write_state()?;
        let job = state.jobs.get(&command.job).ok_or(StoreError::NotFound)?;
        if job.state != crate::JobState::Leased
            || job.lease_token != Some(command.lease)
            || job.lease_expires_at <= Some(state.authoritative_time)
            || job.payload
                != (crate::JobPayload::Retention {
                    course: command.course,
                    stage: command.stage,
                    generation: command.generation,
                })
        {
            return Err(StoreError::Conflict);
        }
        let key = command.course;
        let current = state
            .course_retention
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current.snapshot.generation() != command.generation {
            return Err(StoreError::Conflict);
        }
        if command.stage == crate::RetentionStage::DeleteStudentRecords
            && current.status.state == CourseRetentionState::StudentRecordsDeleted
        {
            return Err(StoreError::Conflict);
        }
        let stage_key = (command.course, command.stage, command.generation);
        let manifest_key = (command.course, command.generation, command.stage);
        let stage = state
            .retention_stages
            .get(&stage_key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        if state.authoritative_time < stage.due_at
            || !matches!(
                stage.state,
                RetentionStageWorkState::Scheduled | RetentionStageWorkState::Started
            )
            || (stage.state == RetentionStageWorkState::Started && stage.job != Some(command.job))
            || state.retention_dispatches.get(&stage_key) != Some(&command.job)
        {
            return Err(StoreError::Conflict);
        }
        match command.stage {
            crate::RetentionStage::Notify => {
                state.retention_stages.insert(
                    stage_key,
                    StoredRetentionStage {
                        due_at: stage.due_at,
                        state: RetentionStageWorkState::Started,
                        job: Some(command.job),
                        lease: Some(command.lease),
                    },
                );
                Ok(RetentionWork::Notify)
            }
            crate::RetentionStage::ArchiveStudentRecords => {
                if let Some(manifest) = state.retention_cleanup_manifests.get(&manifest_key) {
                    let manifest = manifest.clone();
                    if manifest.job != command.job
                        || manifest.state != RetentionCleanupManifestState::Prepared
                    {
                        return Err(StoreError::Conflict);
                    }
                    state.retention_stages.insert(
                        stage_key,
                        StoredRetentionStage {
                            due_at: stage.due_at,
                            state: RetentionStageWorkState::Started,
                            job: Some(command.job),
                            lease: Some(command.lease),
                        },
                    );
                    return Ok(RetentionWork::Cleanup(cleanup_manifest_record_to_work(
                        &manifest,
                    )));
                }
                if stage.state == RetentionStageWorkState::Started
                    && stage.lease != Some(command.lease)
                {
                    return Err(StoreError::Conflict);
                }
                let mut records = BTreeSet::new();
                let mut deliveries = Vec::new();
                let mut terminalize = Vec::new();
                for (export_id, export) in &state.exports {
                    if export.course != command.course {
                        continue;
                    }
                    if let Some(artifacts) = &export.artifacts {
                        for artifact in artifacts {
                            let objects::ObjectKey::StudentRecord { course, .. } =
                                &artifact.object.key
                            else {
                                return Err(StoreError::InvalidRecord(
                                    "retention manifest contains a non-student object".to_string(),
                                ));
                            };
                            if *course != command.course {
                                return Err(StoreError::OwnershipMismatch);
                            }
                            deliveries
                                .push(crate::AssetDeliveryId::from_object(artifact.object.id));
                            records.insert(artifact.object.key.clone());
                        }
                    } else {
                        for object in export.expected.values() {
                            records.insert(objects::ObjectKey::StudentRecord {
                                course: command.course,
                                object: *object,
                            });
                        }
                        terminalize.push((*export_id, export.job));
                    }
                }
                // Delivery and artifact scans are validated before any mutation
                // to avoid partial manifests on non-retryable failures.
                if matches!(
                    current.status.state,
                    CourseRetentionState::Active | CourseRetentionState::StudentRecordsArchived
                ) {
                    state.retention_stages.insert(
                        stage_key,
                        StoredRetentionStage {
                            due_at: stage.due_at,
                            state: RetentionStageWorkState::Started,
                            job: Some(command.job),
                            lease: Some(command.lease),
                        },
                    );
                    for (export, export_job) in terminalize {
                        if let Some(export) = state.exports.get_mut(&export) {
                            export.state = crate::StudentExportState::Failed;
                        }
                        if let Some(export_job) = state.jobs.get_mut(&export_job) {
                            export_job.state = crate::JobState::Dead;
                            export_job.lease_token = None;
                            export_job.lease_expires_at = None;
                        }
                    }
                    for delivery in deliveries {
                        state.asset_deliveries.remove(&delivery);
                    }
                    state.retention_cleanup_manifests.insert(
                        manifest_key,
                        StoredRetentionCleanupManifest {
                            job: command.job,
                            state: RetentionCleanupManifestState::Prepared,
                            objects: records.clone(),
                        },
                    );
                    Ok(RetentionWork::Cleanup(RetentionCleanupManifest::from_iter(
                        records,
                    )))
                } else {
                    Err(StoreError::Conflict)
                }
            }
            crate::RetentionStage::DeleteStudentRecords => {
                if let Some(manifest) = state.retention_cleanup_manifests.get(&manifest_key) {
                    let manifest = manifest.clone();
                    if manifest.job != command.job
                        || manifest.state != RetentionCleanupManifestState::Prepared
                    {
                        return Err(StoreError::Conflict);
                    }
                    state.retention_stages.insert(
                        stage_key,
                        StoredRetentionStage {
                            due_at: stage.due_at,
                            state: RetentionStageWorkState::Started,
                            job: Some(command.job),
                            lease: Some(command.lease),
                        },
                    );
                    return Ok(RetentionWork::Cleanup(cleanup_manifest_record_to_work(
                        &manifest,
                    )));
                }
                if stage.state == RetentionStageWorkState::Started
                    && stage.lease != Some(command.lease)
                {
                    return Err(StoreError::Conflict);
                }
                let mut records = BTreeSet::new();
                let mut terminalize = Vec::new();
                let mut deliveries = Vec::new();
                for (export_id, export) in &state.exports {
                    if export.course != command.course {
                        continue;
                    }
                    if let Some(artifacts) = &export.artifacts {
                        for artifact in artifacts {
                            let objects::ObjectKey::StudentRecord { course, .. } =
                                &artifact.object.key
                            else {
                                return Err(StoreError::InvalidRecord(
                                    "retention manifest contains a non-student object".to_string(),
                                ));
                            };
                            if *course != command.course {
                                return Err(StoreError::OwnershipMismatch);
                            }
                            records.insert(artifact.object.key.clone());
                            deliveries.push(AssetDeliveryId::from_object(artifact.object.id));
                        }
                    } else {
                        for object in export.expected.values() {
                            records.insert(objects::ObjectKey::StudentRecord {
                                course: command.course,
                                object: *object,
                            });
                        }
                        terminalize.push((*export_id, export.job));
                    }
                }
                for (delivery_id, delivery) in &state.asset_deliveries {
                    if let AssetDeliveryScope::StudentRecord { course, .. } = &delivery.scope
                        && *course == command.course
                    {
                        records.insert(delivery.object.key.clone());
                        deliveries.push(*delivery_id);
                    }
                }
                if matches!(
                    current.status.state,
                    CourseRetentionState::Active | CourseRetentionState::StudentRecordsArchived
                ) {
                    if current.status.state == CourseRetentionState::Active {
                        let record = state.course_retention.get_mut(&key).expect(
                            "course retention for valid delete-preparation command must exist",
                        );
                        let disposition = record.status.assignment_definitions;
                        record.status = crate::CourseRetentionStatus::from_persisted(
                            CourseRetentionState::StudentRecordsArchived,
                            disposition,
                        );
                    }
                    state.retention_stages.insert(
                        stage_key,
                        StoredRetentionStage {
                            due_at: stage.due_at,
                            state: RetentionStageWorkState::Started,
                            job: Some(command.job),
                            lease: Some(command.lease),
                        },
                    );
                    for (export, export_job) in terminalize {
                        if let Some(export) = state.exports.get_mut(&export) {
                            export.state = crate::StudentExportState::Failed;
                        }
                        if let Some(export_job) = state.jobs.get_mut(&export_job) {
                            export_job.state = crate::JobState::Dead;
                            export_job.lease_token = None;
                            export_job.lease_expires_at = None;
                        }
                    }
                    for delivery in deliveries {
                        state.asset_deliveries.remove(&delivery);
                    }
                    state.retention_cleanup_manifests.insert(
                        manifest_key,
                        StoredRetentionCleanupManifest {
                            job: command.job,
                            state: RetentionCleanupManifestState::Prepared,
                            objects: records.clone(),
                        },
                    );
                    Ok(RetentionWork::Cleanup(RetentionCleanupManifest::from_iter(
                        records,
                    )))
                } else {
                    Err(StoreError::Conflict)
                }
            }
        }
    }

    async fn commit_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut state_guard = self.write_state()?;
        let now = state_guard.authoritative_time;
        let job = state_guard
            .jobs
            .get(&command.job)
            .ok_or(StoreError::NotFound)?;
        if job.state != crate::JobState::Leased
            || job.lease_token != Some(command.lease)
            || job.lease_expires_at <= Some(now)
            || job.payload
                != (crate::JobPayload::Retention {
                    course: command.course,
                    stage: command.stage,
                    generation: command.generation,
                })
        {
            return Err(StoreError::Conflict);
        }
        let record = state_guard
            .course_retention
            .get(&command.course)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if record.snapshot.generation() != command.generation {
            return Err(StoreError::Conflict);
        }
        let stage_key = (command.course, command.stage, command.generation);
        let stage = state_guard
            .retention_stages
            .get(&stage_key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        if stage.state != RetentionStageWorkState::Started
            || stage.job != Some(command.job)
            || state_guard.retention_dispatches.get(&stage_key) != Some(&command.job)
            || stage.lease != Some(command.lease)
        {
            return Err(StoreError::Conflict);
        }
        let manifest_key = (command.course, command.generation, command.stage);
        let manifest = if command.stage != crate::RetentionStage::Notify {
            let manifest = state_guard
                .retention_cleanup_manifests
                .get(&manifest_key)
                .cloned()
                .ok_or(StoreError::Conflict)?;
            if manifest.job != command.job
                || manifest.state != RetentionCleanupManifestState::Prepared
            {
                return Err(StoreError::Conflict);
            }
            Some(manifest)
        } else {
            None
        };
        if command.stage == crate::RetentionStage::ArchiveStudentRecords
            && record.status.state != CourseRetentionState::Active
        {
            return Err(StoreError::Conflict);
        }
        if command.stage == crate::RetentionStage::DeleteStudentRecords
            && record.status.state != CourseRetentionState::StudentRecordsArchived
        {
            return Err(StoreError::Conflict);
        }
        // Every mutation below
        // is staged and published only after the retention job is complete.
        // This preserves the existing worker's retry contract when any late
        // cleanup or immutable evidence-integrity check fails.
        let mut state = state_guard.clone();
        if let Some(manifest) = manifest {
            // Compute purge dependencies before any mutation.
            if command.stage == crate::RetentionStage::DeleteStudentRecords {
                let course = command.course;
                let assignment_disposition = record.status.assignment_definitions;
                let assignment_ids = state
                    .assignments
                    .iter()
                    .filter_map(|(id, record)| {
                        if record.course_id == course {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect::<BTreeSet<_>>();
                let enrollment_ids = state
                    .enrollments
                    .iter()
                    .filter_map(|(enrollment_id, enrollment)| {
                        if assignment_ids.contains(&enrollment.assignment) {
                            Some(*enrollment_id)
                        } else {
                            None
                        }
                    })
                    .collect::<BTreeSet<_>>();
                let run_ids = state
                    .runs
                    .iter()
                    .filter_map(|(run_id, run)| {
                        if enrollment_ids.contains(&run.enrollment) {
                            Some(*run_id)
                        } else {
                            None
                        }
                    })
                    .collect::<BTreeSet<_>>();
                let attempt_ids = state
                    .attempts
                    .iter()
                    .filter_map(|(attempt_id, attempt)| {
                        if run_ids.contains(&attempt.run) {
                            Some(*attempt_id)
                        } else {
                            None
                        }
                    })
                    .collect::<BTreeSet<_>>();
                let auto_submit_job_ids = state
                    .attempt_timing
                    .iter()
                    .filter_map(|(attempt_id, timing)| {
                        attempt_ids
                            .contains(attempt_id)
                            .then_some(timing.job)
                            .flatten()
                    })
                    .collect::<BTreeSet<_>>();
                let scoring_job_ids = state
                    .assignment_score_staging
                    .iter()
                    .filter_map(|(job, staging)| {
                        assignment_ids.contains(&staging.assignment).then_some(*job)
                    })
                    .collect::<BTreeSet<_>>();
                let export_ids = state
                    .exports
                    .iter()
                    .filter_map(|(export_id, export)| {
                        if export.course == course {
                            Some(*export_id)
                        } else {
                            None
                        }
                    })
                    .collect::<BTreeSet<_>>();
                let export_job_ids = export_ids
                    .iter()
                    .filter_map(|export_id| state.exports.get(export_id).map(|export| export.job))
                    .collect::<BTreeSet<_>>();

                // Course-grade configuration and both synchronous-export audit
                // streams are course student-record adjuncts. Keep Memory's
                // cleanup parity with the normalized PostgreSQL retention path.
                state.course_grade_schemes.remove(&course);
                state
                    .course_grade_export_audits
                    .retain(|_, audit| audit.course != course);

                state
                    .feedback_releases
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .question_statistics_receipts
                    .retain(|(enrollment_id, _, _), receipt| {
                        !(enrollment_ids.contains(enrollment_id)
                            || run_ids.contains(&receipt.first_completed_run)
                            || attempt_ids.contains(&receipt.attempt))
                    });
                state.submission_next_attempts.retain(|predecessor, next| {
                    !(attempt_ids.contains(predecessor)
                        || next
                            .as_ref()
                            .is_some_and(|next| attempt_ids.contains(&next.id)))
                });
                state
                    .prefetched_questions
                    .retain(|(run_id, attempt_id, _), _| {
                        !(run_ids.contains(run_id) || attempt_ids.contains(attempt_id))
                    });
                state
                    .external_tool_launch_sessions
                    .retain(|_, session| !attempt_ids.contains(&session.attempt));
                state
                    .external_tool_exchanges
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .submissions
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .private_submission_responses
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .student_work_inspection_record_accesses
                    .retain(|fact| fact.course != course);
                state
                    .student_work_inspection_audits
                    .retain(|fact| fact.course != course);
                state
                    .attempt_scores
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_current
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_timing
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .issued_effective_policy_receipts
                    .retain(|(attempt_id, _), _| !attempt_ids.contains(attempt_id));
                state
                    .issued_effective_policy_field_sources
                    .retain(|(attempt_id, _, _, _), _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_effective_policy_current
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_support_actions
                    .retain(|_, action| !attempt_ids.contains(&action.attempt));
                state
                    .attempts
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_issued_question_snapshots
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_presentation_capabilities
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_presentations
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_presentation_snapshots
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_grading_envelopes
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_flat_grading_capabilities
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_flat_grading
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_webwork_grading_capabilities
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_webwork_grading
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_qti_grading_capabilities
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .attempt_qti_grading
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .webwork_grade_replay
                    .retain(|attempt_id, _| !attempt_ids.contains(attempt_id));
                state
                    .summaries
                    .retain(|enrollment_id, _| !enrollment_ids.contains(enrollment_id));
                state.runs.retain(|run_id, _| !run_ids.contains(run_id));
                state
                    .enrollments
                    .retain(|enrollment_id, _| !enrollment_ids.contains(enrollment_id));
                state
                    .asset_access_events
                    .retain(|event| event.course != Some(course));
                state.asset_deliveries.retain(|_, delivery| {
                    !matches!(
                        delivery.scope,
                        AssetDeliveryScope::StudentRecord { course: delivery_course, .. }
                            if delivery_course == course
                    )
                });
                for export_id in &export_ids {
                    state.exports.remove(export_id);
                }
                for export_job in &export_job_ids {
                    state.jobs.remove(export_job);
                }
                for job in auto_submit_job_ids.iter().chain(&scoring_job_ids) {
                    state.jobs.remove(job);
                }
                state
                    .assignment_score_staging
                    .retain(|job, _| !scoring_job_ids.contains(job));
                let revoked_at = state.authoritative_time;
                let mut revoked_membership_ids = BTreeSet::new();
                for membership in state.course_memberships.values_mut() {
                    if membership.course == course
                        && membership.role == question_model::CourseMembershipRole::Student
                        && membership.status == crate::CourseMemberStatus::Active
                    {
                        membership.status = crate::CourseMemberStatus::Revoked;
                        membership.revoked_at = Some(revoked_at);
                        revoked_membership_ids.insert(membership.id);
                    }
                }
                state.active_course_membership_by_user.retain(
                    |(record_course, _), membership_id| {
                        !(*record_course == course
                            && revoked_membership_ids.contains(membership_id))
                    },
                );
                for group in state.course_groups.values_mut() {
                    if group.course == course {
                        group.members.clear();
                    }
                }
                state
                    .assignment_individual_policy_exceptions
                    .retain(|(assignment, _), _| !assignment_ids.contains(assignment));
                if assignment_disposition == AssignmentDefinitionDisposition::Delete {
                    for assignment_id in &assignment_ids {
                        if let Some(reference) = state.assignment_references.remove(assignment_id) {
                            state.assignments_by_reference.remove(&reference);
                        }
                        state.assignments.remove(assignment_id);
                        state.assignment_revisions.remove(assignment_id);
                        state.assignment_base_policy.remove(assignment_id);
                        state
                            .assignment_group_schedule_offsets
                            .retain(|(assignment, _), _| assignment != assignment_id);
                        state
                            .assignment_group_accommodations
                            .retain(|(assignment, _), _| assignment != assignment_id);
                        state.assignment_scoring.remove(assignment_id);
                    }
                    state
                        .assignments_by_reference
                        .retain(|_, assignment| !assignment_ids.contains(assignment));
                    state
                        .assignment_group_schedule_offsets
                        .retain(|(assignment, _), _| !assignment_ids.contains(assignment));
                    state
                        .assignment_group_accommodations
                        .retain(|(assignment, _), _| !assignment_ids.contains(assignment));
                }
            }
            state.retention_cleanup_manifests.insert(
                manifest_key,
                StoredRetentionCleanupManifest {
                    job: command.job,
                    state: RetentionCleanupManifestState::Completed,
                    objects: manifest.objects,
                },
            );
        }
        if command.stage == crate::RetentionStage::Notify {
            let created_at = state.authoritative_time;
            state.retention_notifications.insert(
                (command.course, command.generation),
                crate::RetentionNotificationView {
                    intent: crate::RetentionNotificationIntent::Archive,
                    created_at,
                },
            );
        }
        state.retention_stages.insert(
            stage_key,
            StoredRetentionStage {
                due_at: stage.due_at,
                state: RetentionStageWorkState::Completed,
                job: Some(command.job),
                lease: Some(command.lease),
            },
        );
        if command.stage == crate::RetentionStage::ArchiveStudentRecords {
            let record = state
                .course_retention
                .get_mut(&command.course)
                .ok_or(StoreError::NotFound)?;
            record.status = crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::StudentRecordsArchived,
                record.status.assignment_definitions,
            );
        }
        if command.stage == crate::RetentionStage::DeleteStudentRecords {
            let record = state
                .course_retention
                .get_mut(&command.course)
                .ok_or(StoreError::NotFound)?;
            record.status = crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::StudentRecordsDeleted,
                record.status.assignment_definitions,
            );
        }
        let job = state
            .jobs
            .get_mut(&command.job)
            .ok_or(StoreError::NotFound)?;
        job.state = crate::JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        *state_guard = state;
        Ok(())
    }
}
