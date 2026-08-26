//! Connected B2 import-update, provenance, and receipt-led repair relationships.

use learning_data_access::{
    CurriculumAdoptionStore, ReplaceAlphaCourseCommand, ReusableCurriculumStore,
};
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationPreviewRequest, AssignmentDefinitionSourceView,
    AssignmentFastForwardCommand, AssignmentFastForwardPreviewRequest,
    CreateSourceDerivedAssignmentCommand, CurriculumAdoptionIdempotencyKey,
    CurriculumAdoptionReconciliationResult, CurriculumAdoptionTitle, CurriculumPinReplacements,
    ObservedAlphaAssignmentSource, ObservedAlphaSource, ObservedAssignmentRevision,
    ReconcileCurriculumAdoptionCommand, SourceDerivedAssignmentPreviewRequest,
};
use sqlx::Row;

use super::fixture::{AdoptionFixture, definition};

/// Proves that the connected Store advances an eligible import, preserves a
/// divergent assignment while creating an explicit source-derived draft, and
/// restores only its repairable import-current pointer from immutable receipt
/// evidence.
pub(super) async fn assert_import_updates_inspection_and_reconciliation(fixture: &AdoptionFixture) {
    let applied = instantiate(fixture).await;
    let initial = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.instructor_session, applied.course)
        .await
        .expect("initial import inspection")
        .expect("Alpha course has imported assignments");
    let imported = initial
        .assignments
        .first()
        .cloned()
        .expect("fixture Alpha contains one imported assignment");
    let assignment = initial
        .witness
        .assignment_revisions()
        .iter()
        .find(|candidate| candidate.assignment == imported.assignment)
        .copied()
        .expect("inspection witness contains import assignment");

    let revised_source = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.instructor_session,
            ReplaceAlphaCourseCommand {
                reference: Some(fixture.alpha.reference),
                expected_revision: Some(fixture.alpha.revision),
                definition: question_model::AlphaCourseDefinitionInput {
                    title: "B2 public Alpha revised".into(),
                    modules: vec![question_model::AlphaCourseModuleInput {
                        label: "B2 module".into(),
                        definitions: vec![
                            definition(fixture.public_question.clone(), "B2 fast-forward source")
                                .definition,
                        ],
                    }],
                },
            },
        )
        .await
        .expect("source Alpha revision");
    let source = AssignmentDefinitionSourceView::Alpha(
        ObservedAlphaAssignmentSource::new(
            ObservedAlphaSource {
                reference: revised_source.reference,
                revision: revised_source.revision,
            },
            0,
            0,
        )
        .expect("source assignment locator"),
    );
    let fast_forward_preview = fixture
        .store
        .preview_assignment_fast_forward(
            fixture.context,
            fixture.instructor_session,
            AssignmentFastForwardPreviewRequest {
                course: applied.course,
                assignment: ObservedAssignmentRevision {
                    assignment: assignment.assignment,
                    revision: assignment.revision,
                },
                import_revision: imported.revision,
                source,
            },
        )
        .await
        .expect("eligible fast-forward preview");
    let fast_forward = fixture
        .store
        .apply_assignment_fast_forward(
            fixture.context,
            fixture.instructor_session,
            AssignmentFastForwardCommand::from_preview(
                &fast_forward_preview,
                key("b2-live-fast-forward"),
            )
            .expect("eligible fast-forward command"),
        )
        .await
        .expect("eligible fast-forward apply");
    assert_eq!(fast_forward.assignment, imported.assignment);
    assert!(fast_forward.import_revision.value() > imported.revision.value());

    let derived_preview = fixture
        .store
        .preview_source_derived_assignment(
            fixture.context,
            fixture.instructor_session,
            SourceDerivedAssignmentPreviewRequest {
                course: applied.course,
                source: AssignmentDefinitionSourceView::Blueprint(fixture.blueprint),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("source-derived preview after divergent import path");
    let derived = fixture
        .store
        .create_source_derived_assignment(
            fixture.context,
            fixture.instructor_session,
            CreateSourceDerivedAssignmentCommand::from_preview(
                &derived_preview,
                key("b2-live-source-derived"),
            )
            .expect("source-derived command"),
        )
        .await
        .expect("source-derived assignment creation");
    assert_ne!(derived.assignment, imported.assignment);

    let after = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.instructor_session, applied.course)
        .await
        .expect("post-update inspection")
        .expect("imports remain inspectable");
    assert!(
        after
            .assignments
            .iter()
            .any(|entry| entry.assignment == imported.assignment)
    );
    assert!(
        after
            .assignments
            .iter()
            .any(|entry| entry.assignment == derived.assignment)
    );
    assert_answer_free_inspection(&after);

    let unchanged = fixture
        .store
        .reconcile_curriculum_adoption(
            fixture.context,
            fixture.instructor_session,
            ReconcileCurriculumAdoptionCommand {
                receipt: fast_forward.receipt.clone(),
            },
        )
        .await
        .expect("intact reconciliation");
    assert!(matches!(
        unchanged,
        CurriculumAdoptionReconciliationResult::AlreadyConsistent { receipt }
            if receipt == fast_forward.receipt
    ));

    let immutable_before = immutable_evidence(fixture, imported.assignment).await;
    let removed = sqlx::query(
        "DELETE FROM public.curriculum_assignment_import_current AS current_row \
         USING public.assignment AS assignment \
         WHERE current_row.tenant_id = $1 AND current_row.assignment_id = assignment.assignment_id \
           AND assignment.tenant_id = $1 AND assignment.public_id = $2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(i32::try_from(imported.assignment.number()).expect("fixture route number"))
    .execute(&fixture.pool)
    .await
    .expect("privileged fixture removes only the repairable pointer");
    assert_eq!(
        removed.rows_affected(),
        1,
        "the fixture removes exactly the imported assignment current pointer"
    );

    let repaired = fixture
        .store
        .reconcile_curriculum_adoption(
            fixture.context,
            fixture.instructor_session,
            ReconcileCurriculumAdoptionCommand {
                receipt: fast_forward.receipt.clone(),
            },
        )
        .await
        .expect("receipt-led current-pointer repair");
    assert!(matches!(
        repaired,
        CurriculumAdoptionReconciliationResult::Repaired { receipt, projections }
            if receipt == fast_forward.receipt
                && projections.as_slice() == [
                    question_model::CurriculumAdoptionRepairedProjection::AssignmentImportCurrent {
                        assignment: imported.assignment,
                    },
                ]
    ));
    let repaired_inspection = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.instructor_session, applied.course)
        .await
        .expect("repaired import inspection")
        .expect("repaired current pointer restores import inspection");
    assert_eq!(repaired_inspection, after);
    assert_eq!(
        immutable_evidence(fixture, imported.assignment).await,
        immutable_before
    );
}

async fn immutable_evidence(
    fixture: &AdoptionFixture,
    assignment: question_model::AssignmentReference,
) -> (i64, String) {
    let row = sqlx::query(
        "SELECT count(*)::bigint AS evidence_count, \
                coalesce(string_agg(encode(evidence.semantic_sha256, 'hex'), ',' \
                    ORDER BY evidence.import_revision), '') AS evidence_digest \
         FROM public.curriculum_assignment_adoption_evidence AS evidence \
         JOIN public.assignment AS assignment_row \
           ON assignment_row.tenant_id = evidence.tenant_id \
          AND assignment_row.assignment_id = evidence.assignment_id \
         WHERE evidence.tenant_id = $1 AND assignment_row.public_id = $2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(i32::try_from(assignment.number()).expect("fixture route number"))
    .fetch_one(&fixture.pool)
    .await
    .expect("privileged fixture snapshots immutable evidence");
    (
        row.try_get("evidence_count").expect("evidence count"),
        row.try_get("evidence_digest").expect("evidence digest"),
    )
}

async fn instantiate(fixture: &AdoptionFixture) -> question_model::AlphaInstantiationCompleted {
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.instructor_session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("B2 import update destination")
                    .expect("fixture title"),
                target_term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("fixture term"),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("Alpha preview");
    fixture
        .store
        .apply_alpha_instantiation(
            fixture.context,
            fixture.instructor_session,
            AlphaInstantiationCommand::from_preview(&preview, key("b2-live-import-update"))
                .expect("Alpha command"),
        )
        .await
        .expect("Alpha instantiation")
}

fn assert_answer_free_inspection(inspection: &question_model::CurriculumCourseImportView) {
    let serialized = serde_json::to_value(inspection).expect("answer-free inspection JSON");
    let text = serialized.to_string();
    for prohibited in ["seed", "answer", "response", "grader", "email", "userId"] {
        assert!(
            !text.contains(prohibited),
            "inspection contract keeps {prohibited} outside the Instructor projection"
        );
    }
}

fn key(value: &str) -> CurriculumAdoptionIdempotencyKey {
    CurriculumAdoptionIdempotencyKey::parse(value).expect("fixture key")
}
