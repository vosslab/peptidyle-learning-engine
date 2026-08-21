use super::*;

#[tokio::test]
async fn memory_catalog_keeps_question_identity_exact() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(301));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(uuid(303));
    let course = CourseId::from_uuid(uuid(304));
    store
        .create_course(
            context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Catalog course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: publisher,
            },
        )
        .await
        .expect("course saves");

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(305)),
        version: VersionId::from_uuid(uuid(306)),
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question(WorkspaceId::from_uuid(uuid(307))),
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft saves");
    let published = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Institution,
                byline: reviewed_byline(),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("institution question publishes");
    assert_eq!(
        store
            .resolve_catalog_problem(
                context,
                ProblemDisplayRef {
                    question_id: published.question_id.clone(),
                },
            )
            .await,
        Ok(Some(published.clone()))
    );
    assert_eq!(
        store
            .resolve_catalog_problem(
                TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(308))),
                ProblemDisplayRef {
                    question_id: published.question_id.clone(),
                },
            )
            .await,
        Ok(None),
        "Question IDs retain catalog visibility boundaries"
    );
    assert_eq!(
        store
            .create_assignment_with_default_policy(
                context,
                publisher,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(309)),
                    tenant,
                    course_id: course,
                    title: "Exact catalog reference".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    audience: question_model::AssignmentAudience::CourseWide,
                    items: fixed_items(vec![reference]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            )
            .await
            .map(|stored| stored.record.references().collect::<Vec<_>>()),
        Ok(vec![reference]),
        "assignment creation stores the resolved immutable publication"
    );
}
