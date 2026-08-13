use super::*;

#[async_trait]
impl ExportJobStore for MemoryStore {
    async fn create_assignment_export(
        &self,
        context: TenantContext,
        request: CreateAssignmentExport,
    ) -> Result<StudentExportView, StoreError> {
        if !(1..=20).contains(&request.max_attempts) {
            return Err(StoreError::InvalidRecord(
                "job max attempts must be between 1 and 20".to_string(),
            ));
        }
        let export = ExportId::generate()?;
        let manifest = fresh_export_object_id()?;
        let job = JobId::generate()?;
        let mut expected = BTreeMap::new();
        for kind in ExportArtifactKind::ALL {
            expected.insert(kind, fresh_export_object_id()?);
        }
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&(context.tenant_id(), request.assignment))
            .ok_or(StoreError::NotFound)?;
        if assignment.tenant != context.tenant_id() {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let record = StoredExport {
            course: assignment.course_id,
            assignment: assignment.id,
            title: assignment.title.clone(),
            requested_by: request.requested_by,
            manifest,
            problems: assignment.active_references().collect(),
            job,
            state: StudentExportState::Queued,
            expected,
            artifacts: None,
        };
        state.exports.insert((context.tenant_id(), export), record);
        let available_at = state.authoritative_time;
        state.jobs.insert(
            job,
            StoredJob {
                tenant: context.tenant_id(),
                payload: JobPayload::Export {
                    delivery_object: manifest,
                },
                state: JobState::Ready,
                available_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: request.max_attempts,
                failure: None,
            },
        );
        Ok(StudentExportView {
            id: export,
            assignment: request.assignment,
            state: StudentExportState::Queued,
            artifacts: None,
        })
    }

    async fn get_assignment_export(
        &self,
        context: TenantContext,
        export: ExportId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .exports
            .get(&(context.tenant_id(), export))
            .filter(|stored| course_records_accessible(&state, context.tenant_id(), stored.course))
            .map(|stored| export_view(export, stored)))
    }

    async fn get_assignment_export_for_requester(
        &self,
        context: TenantContext,
        export: ExportId,
        requester: UserId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .exports
            .get(&(context.tenant_id(), export))
            .filter(|stored| {
                stored.requested_by == requester
                    && course_records_accessible(&state, context.tenant_id(), stored.course)
            })
            .map(|stored| export_view(export, stored)))
    }

    async fn load_export_job(
        &self,
        context: TenantContext,
        manifest: question_model::ObjectId,
    ) -> Result<Option<StudentExportJob>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .exports
            .iter()
            .find(|((tenant, _), stored)| {
                *tenant == context.tenant_id()
                    && stored.manifest == manifest
                    && course_records_accessible(&state, *tenant, stored.course)
            })
            .map(|((tenant, export), stored)| StudentExportJob {
                id: *export,
                tenant: *tenant,
                assignment: stored.assignment,
                course: stored.course,
                title: stored.title.clone(),
                requested_by: stored.requested_by,
                manifest: stored.manifest,
                problems: stored.problems.clone(),
                expected_artifacts: stored
                    .expected
                    .iter()
                    .map(|(kind, object)| (*kind, *object))
                    .collect(),
            }))
    }

    async fn commit_export_effect(
        &self,
        context: TenantContext,
        commit: ExportJobCommit,
    ) -> Result<ExportCommitDisposition, StoreError> {
        validate_export_artifacts(context.tenant_id(), &commit.artifacts)?;
        let mut state = self.write_state()?;
        let (export, stored) = state
            .exports
            .iter()
            .find(|((tenant, _), stored)| {
                *tenant == context.tenant_id() && stored.manifest == commit.manifest
            })
            .map(|((_, export), stored)| (*export, stored.clone()))
            .ok_or(StoreError::NotFound)?;
        require_course_records_accessible(&state, context.tenant_id(), stored.course)?;
        if stored.job != commit.job {
            return Err(StoreError::Conflict);
        }
        if stored.state == StudentExportState::Ready {
            return if stored.artifacts.as_ref() == Some(&commit.artifacts) {
                Ok(ExportCommitDisposition::AlreadyCommitted)
            } else {
                Err(StoreError::Conflict)
            };
        }
        validate_expected_export_artifacts(&stored.expected, &commit.artifacts)?;
        let now = state.authoritative_time;
        let job = state.jobs.get(&commit.job).ok_or(StoreError::NotFound)?;
        if job.tenant != context.tenant_id()
            || job.payload
                != (JobPayload::Export {
                    delivery_object: commit.manifest,
                })
            || job.state != JobState::Leased
            || job.lease_token != Some(commit.lease)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        for artifact in &commit.artifacts {
            let delivery = crate::AssetDeliveryRecord {
                id: crate::AssetDeliveryId::from_object(artifact.object.id),
                object: artifact.object.clone(),
                intrinsic_width: None,
                intrinsic_height: None,
                scope: crate::AssetDeliveryScope::StudentRecord {
                    tenant: context.tenant_id(),
                    course: stored.course,
                    authorized_users: vec![stored.requested_by],
                },
            };
            crate::validate_asset_delivery(&delivery)?;
            if state.asset_deliveries.contains_key(&delivery.id) {
                return Err(StoreError::Conflict);
            }
        }
        for artifact in &commit.artifacts {
            let id = crate::AssetDeliveryId::from_object(artifact.object.id);
            state.asset_deliveries.insert(
                id,
                crate::AssetDeliveryRecord {
                    id,
                    object: artifact.object.clone(),
                    intrinsic_width: None,
                    intrinsic_height: None,
                    scope: crate::AssetDeliveryScope::StudentRecord {
                        tenant: context.tenant_id(),
                        course: stored.course,
                        authorized_users: vec![stored.requested_by],
                    },
                },
            );
        }
        let stored = state
            .exports
            .get_mut(&(context.tenant_id(), export))
            .expect("export selected from this state remains present");
        stored.state = StudentExportState::Ready;
        stored.artifacts = Some(commit.artifacts);
        let job = state
            .jobs
            .get_mut(&commit.job)
            .expect("job selected from this state remains present");
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(ExportCommitDisposition::Committed)
    }
}

fn fresh_export_object_id() -> Result<question_model::ObjectId, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("export object ID randomness unavailable: {error}"))
    })?;
    Ok(question_model::ObjectId::from_uuid(Uuid::from_bytes(bytes)))
}

fn export_view(export: ExportId, stored: &StoredExport) -> StudentExportView {
    StudentExportView {
        id: export,
        assignment: stored.assignment,
        state: stored.state,
        artifacts: stored.artifacts.as_ref().map(|artifacts| {
            artifacts
                .iter()
                .map(|artifact| StudentExportArtifactView {
                    kind: artifact.kind,
                    filename: artifact.filename.clone(),
                    media_type: artifact.object.media_type.clone(),
                    delivery: crate::AssetDeliveryId::from_object(artifact.object.id),
                })
                .collect()
        }),
    }
}

fn validate_export_artifacts(
    tenant: TenantId,
    artifacts: &[ExportArtifactRecord],
) -> Result<(), StoreError> {
    if artifacts.len() != ExportArtifactKind::ALL.len() {
        return Err(StoreError::InvalidRecord(
            "an export effect must contain exactly four artifacts".to_string(),
        ));
    }
    let mut kinds = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for artifact in artifacts {
        if !kinds.insert(artifact.kind) || !objects.insert(artifact.object.id) {
            return Err(StoreError::InvalidRecord(
                "export artifact kinds and objects must be unique".to_string(),
            ));
        }
        let expected_name = match artifact.kind {
            ExportArtifactKind::Docx => "exam.docx",
            ExportArtifactKind::Pdf => "exam.pdf",
            ExportArtifactKind::AccessibleDocx => "exam-accessible.docx",
            ExportArtifactKind::AccessiblePdf => "exam-accessible.pdf",
        };
        if artifact.filename != expected_name
            || artifact.object.media_type != artifact.kind.media_type()
            || !matches!(
                artifact.object.key,
                objects::ObjectKey::StudentRecord { tenant: key_tenant, object }
                    if key_tenant == tenant && object == artifact.object.id
            )
        {
            return Err(StoreError::InvalidRecord(
                "export artifact does not match its closed private output contract".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_expected_export_artifacts(
    expected: &BTreeMap<ExportArtifactKind, question_model::ObjectId>,
    artifacts: &[ExportArtifactRecord],
) -> Result<(), StoreError> {
    if artifacts.iter().all(|artifact| {
        expected
            .get(&artifact.kind)
            .is_some_and(|object| *object == artifact.object.id)
    }) {
        Ok(())
    } else {
        Err(StoreError::Conflict)
    }
}
