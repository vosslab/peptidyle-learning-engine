//! Deterministic current-curriculum fixture for Memory adoption behavior tests.
//!
//! The fixture establishes only ordinary public Store inputs: an approved
//! Instructor session, published question records, one owned BlueprintCourse,
//! and one CourseInstance created through the preview/apply envelope. Tests
//! exercise the public Store boundary rather than fabricating commands,
//! receipts, or mutable derived projections.

use crate::{
    AccountRecord, AuthenticationEmail, CreateBlueprintCourseCommand, CurriculumAdoptionStore,
    InstructorApprovalRevision, ReusableCurriculumStore, SessionLifetime, SessionStore,
    SessionSubject, StoredInstructorApproval,
};
use question_model::{
    ActivityTimestamp, AssignmentDefinitionSourceView, BlueprintAssignmentId, BlueprintModuleId,
    CourseReference, CourseTerm, CreateBlueprintCourseDefinitionInput,
    CreateBlueprintCourseModuleInput, CurriculumAdoptionApplyIntent, CurriculumAdoptionCompleted,
    CurriculumAdoptionPreview, CurriculumAdoptionPreviewRequest, CurriculumPinReplacements,
    InstantiateBlueprintCoursePreviewRequest, ObservedBlueprintSource, TenantId, UserId, UserRole,
};
use uuid::Uuid;

use super::adoption_inputs::{definition, key, published_record};
use crate::in_memory::MemoryStore;
use crate::{SessionTokenHash, TenantContext};

/// One deterministic approved-Instructor source and CourseInstance fixture.
pub(super) struct CurriculumAdoptionScenario {
    pub(in crate::in_memory::curriculum_adoption::tests) store: MemoryStore,
    pub(in crate::in_memory::curriculum_adoption::tests) tenant: TenantId,
    pub(in crate::in_memory::curriculum_adoption::tests) context: TenantContext,
    pub(in crate::in_memory::curriculum_adoption::tests) actor: UserId,
    pub(in crate::in_memory::curriculum_adoption::tests) session: SessionTokenHash,
    pub(in crate::in_memory::curriculum_adoption::tests) blueprint: ObservedBlueprintSource,
    pub(in crate::in_memory::curriculum_adoption::tests) blueprint_module: BlueprintModuleId,
    pub(in crate::in_memory::curriculum_adoption::tests) blueprint_assignment:
        BlueprintAssignmentId,
    pub(in crate::in_memory::curriculum_adoption::tests) course: CourseReference,
    pub(in crate::in_memory::curriculum_adoption::tests) term: CourseTerm,
    pub(in crate::in_memory::curriculum_adoption::tests) source_question:
        question_model::QuestionId,
    pub(in crate::in_memory::curriculum_adoption::tests) replacement_question:
        question_model::QuestionId,
}

impl CurriculumAdoptionScenario {
    /// Creates a complete current source tree and one bound CourseInstance.
    pub(in crate::in_memory::curriculum_adoption::tests) async fn new() -> Self {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(121_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let actor = UserId::from_uuid(Uuid::from_u128(121_002));
        let session = SessionTokenHash::compute(b"curriculum-adoption-current");
        let source_record = published_record(121_003);
        let replacement_record = published_record(121_004);
        let source_question = source_record.question_id.clone();
        let replacement_question = replacement_record.question_id.clone();
        {
            let mut state = store.write_state().expect("fixture state");
            state.published.insert(
                (source_record.problem, source_record.version),
                source_record,
            );
            state.published.insert(
                (replacement_record.problem, replacement_record.version),
                replacement_record,
            );
            state.instructor_approvals.insert(
                actor,
                StoredInstructorApproval {
                    approval: question_model::InstructorApproval {
                        user: actor,
                        approved_by: actor,
                        approved_at: ActivityTimestamp::from_unix_millis(0),
                        revoked_at: None,
                    },
                    revision: InstructorApprovalRevision::INITIAL,
                },
            );
            state.accounts.insert(
                actor,
                AccountRecord {
                    user: actor,
                    email: AuthenticationEmail::parse("curriculum@example.edu")
                        .expect("fixture email"),
                    display_name: "Curriculum Instructor".into(),
                    platform_roles: Vec::new(),
                    created_at: ActivityTimestamp::from_unix_millis(0),
                    updated_at: ActivityTimestamp::from_unix_millis(0),
                },
            );
        }
        store
            .create_session(
                session,
                SessionSubject::new(
                    tenant,
                    actor,
                    "Curriculum Instructor",
                    vec![UserRole::Instructor],
                )
                .expect("fixture Instructor session"),
                SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
            )
            .await
            .expect("fixture session");

        let created = store
            .create_blueprint_course(
                context,
                session,
                CreateBlueprintCourseCommand {
                    definition: CreateBlueprintCourseDefinitionInput {
                        title: "Current curriculum source".into(),
                        modules: vec![CreateBlueprintCourseModuleInput {
                            label: "Exact module".into(),
                            definitions: vec![definition(source_question.clone())],
                        }],
                    },
                },
            )
            .await
            .expect("fixture BlueprintCourse");
        let module = created.modules.first().expect("fixture module");
        let assignment = module.definitions.first().expect("fixture assignment");
        let blueprint = ObservedBlueprintSource {
            reference: created.reference,
            revision: created.revision,
        };
        let term = CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("fixture term");
        let course =
            instantiate_course(&store, context, session, blueprint, &term, "fixture-course").await;
        Self {
            store,
            tenant,
            context,
            actor,
            session,
            blueprint,
            blueprint_module: module.module_id,
            blueprint_assignment: assignment.assignment_id,
            course,
            term,
            source_question,
            replacement_question,
        }
    }

    /// Returns one exact stable assignment lineage in the current source revision.
    pub(in crate::in_memory::curriculum_adoption::tests) fn assignment_source(
        &self,
    ) -> AssignmentDefinitionSourceView {
        AssignmentDefinitionSourceView::new(self.blueprint, self.blueprint_assignment)
    }

    /// Builds the browser-safe current request used by preview and apply.
    pub(in crate::in_memory::curriculum_adoption::tests) fn instantiate_request(
        &self,
    ) -> CurriculumAdoptionPreviewRequest {
        instantiate_request(self.blueprint, &self.term)
    }
}

async fn instantiate_course(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    blueprint: ObservedBlueprintSource,
    term: &CourseTerm,
    suffix: &str,
) -> CourseReference {
    let request = instantiate_request(blueprint, term);
    let preview = store
        .preview_curriculum_adoption(context, session, request.clone())
        .await
        .expect("fixture instantiation preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::InstantiateBlueprintCourse { .. }
    ));
    let completed = store
        .apply_curriculum_adoption(
            context,
            session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: key(suffix),
            },
        )
        .await
        .expect("fixture instantiation apply");
    let CurriculumAdoptionCompleted::InstantiateBlueprintCourse { completed } = completed else {
        panic!("fixture instantiation must complete as a CourseInstance");
    };
    completed.course
}

fn instantiate_request(
    blueprint: ObservedBlueprintSource,
    term: &CourseTerm,
) -> CurriculumAdoptionPreviewRequest {
    CurriculumAdoptionPreviewRequest::InstantiateBlueprintCourse {
        request: InstantiateBlueprintCoursePreviewRequest {
            source: blueprint,
            target_term: term.clone(),
            replacements: CurriculumPinReplacements::default(),
        },
    }
}
