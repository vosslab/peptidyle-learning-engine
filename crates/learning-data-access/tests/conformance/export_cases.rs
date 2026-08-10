use super::*;

#[tokio::test]
async fn memory_export_commits_exact_four_private_artifacts_atomically() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    let tenant = TenantId::from_uuid(uuid(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let view = store
        .create_assignment_export(
            context,
            CreateAssignmentExport {
                assignment: AssignmentId::from_uuid(uuid(8)),
                requested_by: UserId::from_uuid(uuid(18)),
                max_attempts: 2,
            },
        )
        .await
        .expect("assignment export should freeze and queue");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("export job should claim")
        .expect("queued export job");
    let JobPayload::Export { delivery_object } = claim.payload else {
        panic!("assignment export must have the closed export payload");
    };
    let frozen = store
        .load_export_job(context, delivery_object)
        .await
        .expect("frozen export lookup")
        .expect("manifest resolves only its request");
    assert_eq!(frozen.expected_artifacts.len(), 4);
    let artifacts = frozen
        .expected_artifacts
        .iter()
        .map(|(kind, object)| {
            let (filename, media_type) = match kind {
                ExportArtifactKind::Docx => ("exam.docx", kind.media_type()),
                ExportArtifactKind::Pdf => ("exam.pdf", kind.media_type()),
                ExportArtifactKind::AccessibleDocx => ("exam-accessible.docx", kind.media_type()),
                ExportArtifactKind::AccessiblePdf => ("exam-accessible.pdf", kind.media_type()),
            };
            let key = ObjectKey::StudentRecord {
                tenant,
                object: *object,
            };
            ExportArtifactRecord {
                kind: *kind,
                filename: filename.to_string(),
                object: ObjectRecord {
                    id: *object,
                    bucket: key.bucket(),
                    key,
                    sha256: Sha256Digest::compute(filename.as_bytes()),
                    size_bytes: u64::try_from(filename.len()).expect("fixture length"),
                    media_type: media_type.to_string(),
                    category: ObjectCategory::Export,
                    version: None,
                    license: "educational-record".to_string(),
                    provenance: "export conformance fixture".to_string(),
                    created_at: ActivityTimestamp::from_unix_millis(1),
                },
            }
        })
        .collect::<Vec<_>>();
    let commit = ExportJobCommit {
        job: claim.id,
        lease: claim.lease_token,
        manifest: delivery_object,
        artifacts,
    };
    assert_eq!(
        store
            .commit_export_effect(context, commit.clone())
            .await
            .expect("all artifacts and completion commit together"),
        ExportCommitDisposition::Committed
    );
    assert_eq!(
        store
            .commit_export_effect(context, commit)
            .await
            .expect("same effect replay is safe"),
        ExportCommitDisposition::AlreadyCommitted
    );
    let ready = store
        .get_assignment_export_for_requester(context, view.id, UserId::from_uuid(uuid(18)))
        .await
        .expect("requester status lookup")
        .expect("requester sees export");
    assert_eq!(ready.artifacts.expect("ready has all deliveries").len(), 4);
    assert!(
        store
            .get_assignment_export_for_requester(context, view.id, UserId::from_uuid(uuid(19)))
            .await
            .expect("nonrequester lookup")
            .is_none()
    );

    let failed = store
        .create_assignment_export(
            context,
            CreateAssignmentExport {
                assignment: AssignmentId::from_uuid(uuid(8)),
                requested_by: UserId::from_uuid(uuid(18)),
                max_attempts: 1,
            },
        )
        .await
        .expect("second export queues independently");
    let failed_claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("second export claim")
        .expect("second export ready");
    assert_eq!(
        store
            .fail_job(
                failed_claim.id,
                failed_claim.lease_token,
                JobFailureKind::Permanent,
            )
            .await
            .expect("permanent refusal records terminal failure"),
        JobFailureDisposition::Dead
    );
    assert_eq!(
        store
            .get_assignment_export_for_requester(context, failed.id, UserId::from_uuid(uuid(18)))
            .await
            .expect("failed requester status")
            .expect("failed request remains visible")
            .state,
        learning_data_access::StudentExportState::Failed
    );
}
