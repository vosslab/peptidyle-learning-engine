use super::*;

impl MemoryStore {
    /// Inserts a pre-validation legacy draft for route-boundary tests only.
    ///
    /// This exists to prove current HTTP handlers fail safely when a database
    /// contains historical corrupt data. It is compiled only with the
    /// `test-support` feature and must not be enabled by production code.
    #[cfg(feature = "test-support")]
    pub fn insert_legacy_draft_for_test(&self, draft: DraftRecord) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        state
            .drafts
            .insert((draft.tenant, draft.question.workspace), draft);
        Ok(())
    }

    /// Seeds a closed, due archive-cleanup job for server worker tests only.
    #[cfg(feature = "test-support")]
    pub fn seed_retention_cleanup_for_test(
        &self,
        tenant: TenantId,
        course: CourseId,
        objects: Vec<question_model::ObjectId>,
    ) -> Result<Vec<objects::ObjectKey>, StoreError> {
        self.seed_retention_cleanup_stage_for_test(
            tenant,
            course,
            objects,
            crate::RetentionStage::ArchiveStudentRecords,
            AssignmentDefinitionDisposition::Retain,
        )
    }

    #[cfg(feature = "test-support")]
    fn seed_retention_cleanup_stage_for_test(
        &self,
        tenant: TenantId,
        course: CourseId,
        objects: Vec<question_model::ObjectId>,
        stage: crate::RetentionStage,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<Vec<objects::ObjectKey>, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = crate::JobId::generate()?;
        let export_job = crate::JobId::generate()?;
        let export = crate::ExportId::generate()?;
        let snapshot = CourseRetentionSnapshot::new(
            now,
            InstitutionRetentionPolicy::default(),
            disposition,
            1,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        state
            .courses
            .entry((tenant, course))
            .or_insert_with(|| CourseRecord {
                id: course,
                tenant,
                title: "Retention test course".to_string(),
                term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("explicit fixture course term"),
            });
        state.course_retention.insert(
            (tenant, course),
            CourseRetentionRecord {
                snapshot,
                status: crate::CourseRetentionStatus::from_persisted(
                    CourseRetentionState::Active,
                    disposition,
                ),
            },
        );
        state.retention_stages.insert(
            (tenant, course, stage, 1),
            StoredRetentionStage {
                due_at: now,
                state: RetentionStageWorkState::Scheduled,
                job: None,
                lease: None,
            },
        );
        state
            .retention_dispatches
            .insert((tenant, course, stage, 1), job);
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::Retention {
                    course,
                    stage,
                    generation: 1,
                },
                state: crate::JobState::Ready,
                available_at: now,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 2,
                failure: None,
            },
        );
        let expected = objects
            .iter()
            .copied()
            .enumerate()
            .map(|(index, object)| (crate::ExportArtifactKind::ALL[index], object))
            .collect();
        let fixture_uuid =
            |suffix| Uuid::from_u128(course.as_uuid().as_u128().wrapping_add(suffix));
        let manifest = question_model::ObjectId::from_uuid(fixture_uuid(1));
        state.jobs.insert(
            export_job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::Export {
                    delivery_object: manifest,
                },
                state: crate::JobState::Ready,
                available_at: ActivityTimestamp::from_unix_millis(now.as_unix_millis() + 1),
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 2,
                failure: None,
            },
        );
        state.exports.insert(
            (tenant, export),
            StoredExport {
                course,
                assignment: AssignmentId::from_uuid(fixture_uuid(2)),
                title: "retention test".to_string(),
                requested_by: UserId::from_uuid(fixture_uuid(3)),
                manifest,
                problems: Vec::new(),
                job: export_job,
                state: crate::StudentExportState::Queued,
                expected,
                artifacts: None,
            },
        );
        Ok(objects
            .into_iter()
            .map(|object| objects::ObjectKey::StudentRecord { tenant, object })
            .collect())
    }

    /// Sets the stub backend clock used by session tests and local development.
    pub fn set_authoritative_time(&self, now: ActivityTimestamp) -> Result<(), StoreError> {
        self.write_state()?.authoritative_time = now;
        Ok(())
    }

    /// Returns protected asset access events for conformance assertions.
    pub fn asset_access_events(&self) -> Result<Vec<AssetAccessEvent>, StoreError> {
        Ok(self.read_state()?.asset_access_events.clone())
    }

    /// Returns Sysadmin roster-support audit events for conformance assertions.
    pub fn roster_support_audits(
        &self,
    ) -> Result<Vec<crate::CourseRosterSupportAudit>, StoreError> {
        Ok(self.read_state()?.roster_support_audits.clone())
    }

    /// Test-only equivalent of the later submission-completion capability.
    ///
    /// No public Store trait method accepts a collapsed observation.  Keeping
    /// this seam inside the backend proves receipt idempotency and aggregate
    /// atomicity without creating a route-callable statistics writer.
    #[cfg(test)]
    pub(super) fn record_question_statistics_contribution(
        &self,
        tenant: TenantId,
        enrollment: EnrollmentId,
        first_completed_run: RunId,
        attempt: QuestionAttemptId,
        reference: ProblemVersionRef,
        observation: CollapsedQuestionObservation,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let receipt_key = (tenant, enrollment, reference.problem, reference.version);
        if let Some(receipt) = state.question_statistics_receipts.get(&receipt_key) {
            return if receipt.first_completed_run == first_completed_run
                && receipt.attempt == attempt
                && receipt.observation == observation
            {
                Ok(false)
            } else {
                Err(StoreError::Conflict)
            };
        }
        let aggregate_key = (reference.problem, reference.version);
        let mut aggregate = state
            .question_statistics
            .get(&aggregate_key)
            .cloned()
            .unwrap_or_default();
        aggregate
            .record(observation)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let was_disclosed = state
            .question_statistics
            .get(&aggregate_key)
            .is_some_and(|current| {
                matches!(
                    current.disclose(question_model::StatisticsDisclosurePolicy::default()),
                    question_model::QuestionStatisticsDisclosure::Available(_)
                )
            });
        let is_disclosed = matches!(
            aggregate.disclose(question_model::StatisticsDisclosurePolicy::default()),
            question_model::QuestionStatisticsDisclosure::Available(_)
        );
        if is_disclosed && !was_disclosed {
            let sequence = state.next_catalog_publication_sequence;
            state.next_catalog_publication_sequence = sequence.checked_add(1).ok_or_else(|| {
                StoreError::Unavailable("catalog event sequence exhausted".to_string())
            })?;
            state
                .catalog_statistics_disclosure_sequences
                .entry(aggregate_key)
                .or_insert(sequence);
        }
        state.question_statistics.insert(aggregate_key, aggregate);
        state.question_statistics_receipts.insert(
            receipt_key,
            StatisticsContributionReceipt {
                first_completed_run,
                attempt,
                #[cfg(test)]
                observation,
                checksum: objects::Sha256Digest::compute(b"statistics test contribution"),
            },
        );
        Ok(true)
    }

    /// Acquires immutable backend state.
    pub(super) fn read_state(&self) -> Result<std::sync::RwLockReadGuard<'_, State>, StoreError> {
        self.state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))
    }

    /// Acquires mutable backend state for one atomic operation.
    pub(super) fn write_state(&self) -> Result<std::sync::RwLockWriteGuard<'_, State>, StoreError> {
        self.state
            .write()
            .map_err(|error| StoreError::Unavailable(error.to_string()))
    }
}
