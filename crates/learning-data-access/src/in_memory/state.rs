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

    /// Test-only private audit inspection; application code has no audit lookup.
    pub fn preview_subject_audits(&self) -> Result<Vec<crate::PreviewSubjectAudit>, StoreError> {
        Ok(self.read_state()?.preview_subject_audits.clone())
    }

    /// Captures the complete private Memory state for preview-plane
    /// conformance.  This is an opaque, non-route test seam: callers can only
    /// prove equality or the one permitted derived-preview audit delta.
    pub fn preview_plane_state_effect_fingerprint(
        &self,
    ) -> Result<MemoryPreviewPlaneStateEffectFingerprint, StoreError> {
        let state = self.read_state()?;
        Ok(MemoryPreviewPlaneStateEffectFingerprint::from(&*state))
    }

    /// Captures a rehearsal-local effect snapshot without exposing Memory state.
    ///
    /// The snapshot proves that rehearsal calls leave every ordinary learner,
    /// gradebook, analysis, catalog, export, job, and audit collection intact.
    /// It supports comparison only; neither it nor this method exposes the
    /// private state captured for that comparison.
    ///
    /// ```compile_fail
    /// use learning_data_access::in_memory::MemoryStore;
    ///
    /// let store = MemoryStore::default();
    /// let fingerprint = store
    ///     .rehearsal_state_effect_fingerprint()
    ///     .expect("Memory state is available");
    /// let _ = format!("{fingerprint:?}");
    /// ```
    pub fn rehearsal_state_effect_fingerprint(
        &self,
    ) -> Result<MemoryRehearsalStateEffectFingerprint, StoreError> {
        let state = self.read_state()?;
        Ok(MemoryRehearsalStateEffectFingerprint::from(&*state))
    }

    /// Deliberately corrupts one private rehearsal record for conformance
    /// tests. This is not part of any Store trait or route composition.
    #[doc(hidden)]
    #[cfg(feature = "test-support")]
    pub fn rehearsal_test_snapshot(
        &self,
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    ) -> Result<MemoryRehearsalTestSnapshot, StoreError> {
        let state = self.read_state()?;
        let rehearsal_id = state
            .rehearsal_by_reference
            .get(&(tenant, rehearsal))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .rehearsal_runs
            .get(&(tenant, rehearsal_id))
            .ok_or(StoreError::NotFound)?;
        let claims = state
            .rehearsal_submission_claims
            .iter()
            .filter(|((record_tenant, record_rehearsal, _), _)| {
                *record_tenant == tenant && *record_rehearsal == rehearsal_id
            })
            .filter_map(|(_, claim)| {
                claim
                    .events
                    .last()
                    .map(|event| MemoryRehearsalClaimTestSnapshot {
                        phase: event.phase(),
                        generation: event.generation().value(),
                    })
            })
            .collect();
        Ok(MemoryRehearsalTestSnapshot {
            lifecycle: run.lifecycle,
            revision: run.revision,
            claims,
        })
    }

    /// Verifies a retained rehearsal archive without consulting live source
    /// authorization or returning any archived material. This is a private
    /// `test-support` seam for proving that source-context removal preserves
    /// an independently verifiable, tenant-owned aggregate; production code
    /// and Store/browser traits cannot call it.
    #[doc(hidden)]
    #[cfg(feature = "test-support")]
    pub fn verify_rehearsal_archive_for_test(
        &self,
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    ) -> Result<(), StoreError> {
        let state = self.read_state()?;
        let rehearsal_id = state
            .rehearsal_by_reference
            .get(&(tenant, rehearsal))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .rehearsal_runs
            .get(&(tenant, rehearsal_id))
            .ok_or(StoreError::NotFound)?;
        super::rehearsal_integrity::verify_rehearsal_aggregate(&state, tenant, run)
    }

    /// Deliberately corrupts one private rehearsal record for conformance
    #[cfg(feature = "test-support")]
    pub fn corrupt_rehearsal_integrity_for_test(
        &self,
        corruption: MemoryRehearsalIntegrityTestCorruption,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let (tenant, reference) = corruption.binding();
        let rehearsal = state
            .rehearsal_by_reference
            .get(&(tenant, reference))
            .copied()
            .ok_or(StoreError::NotFound)?;
        match corruption {
            MemoryRehearsalIntegrityTestCorruption::RemoveFrozenItem { attempt, .. } => {
                state
                    .rehearsal_frozen_items
                    .remove(&(tenant, rehearsal, attempt))
                    .ok_or(StoreError::NotFound)?;
            }
            MemoryRehearsalIntegrityTestCorruption::DropLatestClaimEvent {
                idempotency_key,
                ..
            } => {
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?;
                claim.events.pop().ok_or(StoreError::NotFound)?;
            }
            MemoryRehearsalIntegrityTestCorruption::DuplicateFrozenEvidence { attempt, .. } => {
                let entries = state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let duplicate = entries
                    .0
                    .iter()
                    .find(|entry| {
                        matches!(
                            &entry.payload,
                            domain::RehearsalEvidencePayload::FrozenItem(frozen)
                                if frozen.attempt == attempt
                        )
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                entries.0.push(duplicate);
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::DuplicateAcceptedEvidence {
                sequence, ..
            } => {
                let entries = state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let duplicate = entries
                    .0
                    .iter()
                    .find(|entry| {
                        entry.record.sequence == sequence
                            && matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                entries.0.push(duplicate);
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::CopyAcceptedEvidenceFromRehearsal {
                source_rehearsal,
                ..
            } => {
                let source = state
                    .rehearsal_by_reference
                    .get(&(tenant, source_rehearsal))
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                let copied = state
                    .rehearsal_evidence
                    .get(&(tenant, source))
                    .and_then(|entries| {
                        entries.0.iter().find(|entry| {
                            matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                        })
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?
                    .0
                    .push(copied);
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::RemoveAllSubmissionClaims { .. } => {
                state.rehearsal_submission_claims.retain(
                    |(record_tenant, record_rehearsal, _), _| {
                        *record_tenant != tenant || *record_rehearsal != rehearsal
                    },
                );
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceAcceptedEvidence {
                source_sequence,
                target_sequence,
                ..
            } => {
                let entries = state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let source = entries
                    .0
                    .iter()
                    .find(|entry| {
                        entry.record.sequence == source_sequence
                            && matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let target = entries
                    .0
                    .iter_mut()
                    .find(|entry| {
                        entry.record.sequence == target_sequence
                            && matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                    })
                    .ok_or(StoreError::NotFound)?;
                target.payload = source.payload;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceClaimRequestWithoutFingerprint {
                idempotency_key,
                response,
                ..
            } => {
                let existing = state
                    .rehearsal_submission_claims
                    .get(&(tenant, rehearsal, idempotency_key.clone()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let frozen = state
                    .rehearsal_frozen_items
                    .get(&(tenant, rehearsal, existing.root.sealed_request().attempt()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let request = domain::RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                    &frozen,
                    frozen.attempt,
                    response,
                )
                .map_err(|error| StoreError::InvalidRecord(format!("test request: {error:?}")))?;
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?;
                claim.root = domain::RehearsalPersistedClaimRoot::from_persisted(
                    existing.root.rehearsal(),
                    existing.root.claim(),
                    existing.root.fingerprint(),
                    request,
                );
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceClaimFingerprintWithoutRequest {
                idempotency_key,
                response,
                ..
            } => {
                let run = state
                    .rehearsal_runs
                    .get(&(tenant, rehearsal))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let existing = state
                    .rehearsal_submission_claims
                    .get(&(tenant, rehearsal, idempotency_key.clone()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let frozen = state
                    .rehearsal_frozen_items
                    .get(&(tenant, rehearsal, existing.root.sealed_request().attempt()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let alternate =
                    domain::RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                        &frozen,
                        frozen.attempt,
                        response,
                    )
                    .map_err(|error| {
                        StoreError::InvalidRecord(format!("test request: {error:?}"))
                    })?;
                let fingerprint = domain::rehearsal_submission_request_fingerprint(
                    super::rehearsal_integrity::genesis(&run, tenant),
                    &frozen,
                    &alternate,
                )
                .map_err(|error| {
                    StoreError::InvalidRecord(format!("test fingerprint: {error:?}"))
                })?;
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?;
                claim.root = domain::RehearsalPersistedClaimRoot::from_persisted(
                    existing.root.rehearsal(),
                    existing.root.claim(),
                    fingerprint,
                    existing.root.sealed_request().clone(),
                );
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenContentDigest {
                attempt,
                digest,
                ..
            } => {
                mutate_frozen_item_for_test(&mut state, tenant, rehearsal, attempt, |frozen| {
                    frozen.canonical_content_digest = digest;
                })?;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenTimestamp {
                attempt,
                frozen_at,
                ..
            } => {
                mutate_frozen_item_for_test(&mut state, tenant, rehearsal, attempt, |frozen| {
                    frozen.frozen_at = frozen_at;
                })?;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenResponseDefinition {
                attempt,
                response_definition,
                ..
            } => {
                mutate_frozen_item_for_test(&mut state, tenant, rehearsal, attempt, |frozen| {
                    frozen.response_definition = response_definition.clone();
                })?;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadDigest {
                digest, ..
            } => {
                let run = state
                    .rehearsal_runs
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                run.evidence_head = domain::RehearsalEvidenceHead::from_persisted(
                    digest,
                    run.evidence_head.length(),
                );
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadLength {
                length, ..
            } => {
                let run = state
                    .rehearsal_runs
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                run.evidence_head = domain::RehearsalEvidenceHead::from_persisted(
                    run.evidence_head.digest(),
                    length,
                );
            }
        }
        Ok(())
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

/// Non-sensitive lifecycle projection for feature-gated Memory conformance.
#[doc(hidden)]
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRehearsalTestSnapshot {
    pub lifecycle: question_model::RehearsalLifecycle,
    pub revision: question_model::TeachingOperationRevision,
    pub claims: Vec<MemoryRehearsalClaimTestSnapshot>,
}

/// One claim's terminal event state, without identity, response, or evidence.
#[doc(hidden)]
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRehearsalClaimTestSnapshot {
    pub phase: domain::RehearsalSubmissionClaimPhase,
    pub generation: u32,
}

/// Narrow corrupt-data selector used only by Memory conformance tests.
#[doc(hidden)]
#[cfg(feature = "test-support")]
#[derive(Debug, Clone)]
pub enum MemoryRehearsalIntegrityTestCorruption {
    RemoveFrozenItem {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
    },
    DropLatestClaimEvent {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
    },
    DuplicateFrozenEvidence {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
    },
    DuplicateAcceptedEvidence {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        sequence: u32,
    },
    CopyAcceptedEvidenceFromRehearsal {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        source_rehearsal: question_model::RehearsalReference,
    },
    RemoveAllSubmissionClaims {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    },
    ReplaceAcceptedEvidence {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        source_sequence: u32,
        target_sequence: u32,
    },
    ReplaceClaimRequestWithoutFingerprint {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
        response: question_model::StudentResponse,
    },
    ReplaceClaimFingerprintWithoutRequest {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
        response: question_model::StudentResponse,
    },
    ReplaceFrozenContentDigest {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
        digest: question_model::RehearsalEvidenceDigest,
    },
    ReplaceFrozenTimestamp {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
        frozen_at: question_model::ActivityTimestamp,
    },
    ReplaceFrozenResponseDefinition {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
        response_definition: question_model::ResponseDefinition,
    },
    ReplaceEvidenceHeadDigest {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        digest: question_model::RehearsalEvidenceDigest,
    },
    ReplaceEvidenceHeadLength {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        length: u32,
    },
}

#[cfg(feature = "test-support")]
impl MemoryRehearsalIntegrityTestCorruption {
    fn binding(&self) -> (TenantId, question_model::RehearsalReference) {
        match self {
            Self::RemoveFrozenItem {
                tenant, rehearsal, ..
            }
            | Self::DropLatestClaimEvent {
                tenant, rehearsal, ..
            }
            | Self::DuplicateFrozenEvidence {
                tenant, rehearsal, ..
            }
            | Self::DuplicateAcceptedEvidence {
                tenant, rehearsal, ..
            }
            | Self::CopyAcceptedEvidenceFromRehearsal {
                tenant, rehearsal, ..
            }
            | Self::RemoveAllSubmissionClaims { tenant, rehearsal }
            | Self::ReplaceAcceptedEvidence {
                tenant, rehearsal, ..
            }
            | Self::ReplaceClaimRequestWithoutFingerprint {
                tenant, rehearsal, ..
            }
            | Self::ReplaceClaimFingerprintWithoutRequest {
                tenant, rehearsal, ..
            }
            | Self::ReplaceFrozenContentDigest {
                tenant, rehearsal, ..
            }
            | Self::ReplaceFrozenTimestamp {
                tenant, rehearsal, ..
            }
            | Self::ReplaceFrozenResponseDefinition {
                tenant, rehearsal, ..
            }
            | Self::ReplaceEvidenceHeadDigest {
                tenant, rehearsal, ..
            }
            | Self::ReplaceEvidenceHeadLength {
                tenant, rehearsal, ..
            } => (*tenant, *rehearsal),
        }
    }
}

#[cfg(feature = "test-support")]
fn mutate_frozen_item_for_test(
    state: &mut State,
    tenant: TenantId,
    rehearsal: question_model::RehearsalRunId,
    attempt: question_model::RehearsalAttemptId,
    mutate: impl Fn(&mut question_model::RehearsalFrozenItemEvidence),
) -> Result<(), StoreError> {
    let persisted = state
        .rehearsal_frozen_items
        .get_mut(&(tenant, rehearsal, attempt))
        .ok_or(StoreError::NotFound)?;
    mutate(persisted);
    let evidence = state
        .rehearsal_evidence
        .get_mut(&(tenant, rehearsal))
        .ok_or(StoreError::NotFound)?;
    let entry = evidence
        .0
        .iter_mut()
        .find(|entry| {
            matches!(
                &entry.payload,
                domain::RehearsalEvidencePayload::FrozenItem(frozen) if frozen.attempt == attempt
            )
        })
        .ok_or(StoreError::NotFound)?;
    let domain::RehearsalEvidencePayload::FrozenItem(frozen) = &mut entry.payload else {
        return Err(StoreError::NotFound);
    };
    mutate(frozen);
    Ok(())
}

#[cfg(feature = "test-support")]
fn rehash_rehearsal_evidence(
    state: &mut State,
    tenant: TenantId,
    rehearsal: question_model::RehearsalRunId,
) -> Result<(), StoreError> {
    let run = state
        .rehearsal_runs
        .get(&(tenant, rehearsal))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let entries = state
        .rehearsal_evidence
        .get_mut(&(tenant, rehearsal))
        .ok_or(StoreError::NotFound)?;
    let mut previous =
        domain::evidence_genesis_digest(super::rehearsal_integrity::genesis(&run, tenant));
    for (index, entry) in entries.0.iter_mut().enumerate() {
        let sequence = u32::try_from(index + 1).map_err(|_| {
            StoreError::Unavailable("rehearsal test evidence sequence exhausted".into())
        })?;
        let kind = match &entry.payload {
            domain::RehearsalEvidencePayload::FrozenItem(_) => {
                question_model::RehearsalEvidenceKind::FrozenItem
            }
            domain::RehearsalEvidencePayload::AcceptedSubmission(_) => {
                question_model::RehearsalEvidenceKind::AcceptedSubmission
            }
        };
        entry.record.sequence = sequence;
        entry.record.kind = kind;
        entry.record.previous_digest = Some(previous);
        entry.record.digest = domain::evidence_entry_digest(
            sequence,
            kind,
            previous,
            domain::private_payload_digest(&entry.payload),
            entry.record.recorded_at,
        );
        previous = entry.record.digest;
    }
    Ok(())
}

/// Opaque Memory-only conformance snapshot for WP-PROF-T3 state effects.
///
/// Application code has no route-callable state snapshot.  Keeping the full
/// state private makes the oracle resilient to new state collections: a
/// preview operation must preserve every collection and current pointer except
/// for one appended private derived-subject audit.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct MemoryPreviewPlaneStateEffectFingerprint {
    state_without_preview_audits: String,
    preview_audits: Vec<crate::PreviewSubjectAudit>,
}

impl From<&State> for MemoryPreviewPlaneStateEffectFingerprint {
    fn from(state: &State) -> Self {
        let mut without_preview_audits = state.clone();
        without_preview_audits.preview_subject_audits.clear();
        Self {
            // The State value is private and has no application serialization.
            // Its complete Debug representation is kept opaque here solely so
            // this conformance seam observes every Memory collection and
            // current pointer without granting test code mutable access.
            state_without_preview_audits: format!("{without_preview_audits:?}"),
            preview_audits: state.preview_subject_audits.clone(),
        }
    }
}

impl MemoryPreviewPlaneStateEffectFingerprint {
    /// Returns whether two Store calls preserved all Memory state exactly.
    pub fn is_unchanged_from(&self, before: &Self) -> bool {
        self.state_without_preview_audits == before.state_without_preview_audits
            && self.preview_audits == before.preview_audits
    }

    /// Returns whether the only state effect is one appended preview audit.
    pub fn has_one_appended_preview_subject_audit_from(&self, before: &Self) -> bool {
        let Some((last, prefix)) = self.preview_audits.split_last() else {
            return false;
        };
        let _ = last;
        self.state_without_preview_audits == before.state_without_preview_audits
            && self.preview_audits.len() == before.preview_audits.len() + 1
            && prefix == before.preview_audits.as_slice()
    }
}

/// Opaque conformance snapshot for the dedicated rehearsal namespace.
#[doc(hidden)]
#[derive(Clone)]
pub struct MemoryRehearsalStateEffectFingerprint {
    ordinary_state: String,
    rehearsal_state: String,
}

impl From<&State> for MemoryRehearsalStateEffectFingerprint {
    fn from(state: &State) -> Self {
        let mut ordinary = state.clone();
        ordinary.next_rehearsal_reference = 0;
        ordinary.rehearsal_runs.clear();
        ordinary.rehearsal_by_reference.clear();
        ordinary.rehearsal_active_by_owner.clear();
        ordinary.rehearsal_frozen_items.clear();
        ordinary.rehearsal_evidence.clear();
        ordinary.rehearsal_submission_claims.clear();
        let rehearsal_state = format!(
            "{:?}{:?}{:?}{:?}{:?}{}{:?}",
            state.next_rehearsal_reference,
            state.rehearsal_runs,
            state.rehearsal_by_reference,
            state.rehearsal_active_by_owner,
            state.rehearsal_frozen_items,
            rehearsal_evidence_fingerprint(&state.rehearsal_evidence),
            state.rehearsal_submission_claims
        );
        Self {
            ordinary_state: format!("{ordinary:?}"),
            rehearsal_state,
        }
    }
}

fn rehearsal_evidence_fingerprint(
    evidence: &BTreeMap<
        (TenantId, question_model::RehearsalRunId),
        super::rehearsal::StoredRehearsalEvidence,
    >,
) -> String {
    let mut value = String::new();
    for (key, entries) in evidence {
        use std::fmt::Write as _;
        let _ = write!(&mut value, "{key:?}");
        for entry in &entries.0 {
            let _ = write!(
                &mut value,
                "{:?}{}",
                entry.record,
                domain::private_payload_digest(&entry.payload).to_hex()
            );
        }
    }
    value
}

impl MemoryRehearsalStateEffectFingerprint {
    /// Returns whether a refused rehearsal call preserved all state.
    pub fn is_unchanged_from(&self, before: &Self) -> bool {
        self.ordinary_state == before.ordinary_state
            && self.rehearsal_state == before.rehearsal_state
    }

    /// Returns whether a successful rehearsal call changed only rehearsal state.
    pub fn has_only_rehearsal_effects_from(&self, before: &Self) -> bool {
        self.ordinary_state == before.ordinary_state
            && self.rehearsal_state != before.rehearsal_state
    }
}
