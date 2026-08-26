//! Shared deterministic B2 adoption scenario for behavior modules.

use crate::{
    AccountRecord, AuthenticationEmail, CurriculumAdoptionStore, ReplaceAlphaCourseCommand,
    ReplaceBlueprintCommand, ReusableCurriculumStore, SessionLifetime, SessionStore,
    SessionSubject,
};
use question_model::{
    ActivityTimestamp, AlphaCourseDefinitionInput, AlphaCourseModuleInput,
    AlphaInstantiationCommand, AlphaInstantiationCompleted, AlphaInstantiationPreviewRequest,
    BlueprintDefinitionInput, CourseTerm, CurriculumAdoptionTitle, CurriculumPinReplacements,
    ObservedAlphaSource, ObservedBlueprintSource, TenantId, UserId, UserRole,
};
use uuid::Uuid;

use super::adoption_inputs::{definition, key, published_record};
use crate::in_memory::MemoryStore;
use crate::{SessionTokenHash, TenantContext};

pub(super) struct AdoptionScenario {
    pub(in crate::in_memory::curriculum_adoption::tests) store: MemoryStore,
    pub(in crate::in_memory::curriculum_adoption::tests) tenant: TenantId,
    pub(in crate::in_memory::curriculum_adoption::tests) context: TenantContext,
    pub(in crate::in_memory::curriculum_adoption::tests) actor: UserId,
    pub(in crate::in_memory::curriculum_adoption::tests) session: SessionTokenHash,
    pub(in crate::in_memory::curriculum_adoption::tests) alpha: ObservedAlphaSource,
    pub(in crate::in_memory::curriculum_adoption::tests) blueprint: ObservedBlueprintSource,
    pub(in crate::in_memory::curriculum_adoption::tests) alpha_input: AlphaCourseDefinitionInput,
    pub(in crate::in_memory::curriculum_adoption::tests) term: CourseTerm,
    pub(in crate::in_memory::curriculum_adoption::tests) source_question:
        question_model::QuestionId,
    pub(in crate::in_memory::curriculum_adoption::tests) replacement_question:
        question_model::QuestionId,
}

impl AdoptionScenario {
    pub(in crate::in_memory::curriculum_adoption::tests) async fn new() -> Self {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(121_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let actor = UserId::from_uuid(Uuid::from_u128(121_002));
        let session = SessionTokenHash::compute(b"curriculum-adoption-negative");
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
                crate::StoredInstructorApproval {
                    approval: question_model::InstructorApproval {
                        user: actor,
                        approved_by: actor,
                        approved_at: ActivityTimestamp::from_unix_millis(0),
                        revoked_at: None,
                    },
                    revision: crate::InstructorApprovalRevision::INITIAL,
                },
            );
            state.accounts.insert(
                actor,
                AccountRecord {
                    user: actor,
                    email: AuthenticationEmail::parse("negative@example.edu").expect("email"),
                    display_name: "Negative Instructor".into(),
                    platform_roles: Vec::new(),
                    created_at: ActivityTimestamp::from_unix_millis(0),
                    updated_at: ActivityTimestamp::from_unix_millis(0),
                },
            );
        }
        store
            .create_session(
                session,
                SessionSubject::new(tenant, actor, "Negative", vec![UserRole::Instructor])
                    .expect("subject"),
                SessionLifetime::from_seconds(3_600).expect("lifetime"),
            )
            .await
            .expect("session");
        let alpha_input = AlphaCourseDefinitionInput {
            title: "Adoption source".into(),
            modules: vec![AlphaCourseModuleInput {
                label: "Exact module".into(),
                definitions: vec![definition(source_question.clone())],
            }],
        };
        let alpha = store
            .replace_alpha_course(
                context,
                session,
                ReplaceAlphaCourseCommand {
                    reference: None,
                    expected_revision: None,
                    definition: alpha_input.clone(),
                },
            )
            .await
            .expect("Alpha source");
        let blueprint = store
            .replace_blueprint(
                context,
                session,
                ReplaceBlueprintCommand {
                    reference: None,
                    expected_revision: None,
                    definition: BlueprintDefinitionInput {
                        definition: definition(source_question.clone()),
                    },
                },
            )
            .await
            .expect("Blueprint source");
        Self {
            store,
            tenant,
            context,
            actor,
            session,
            alpha: ObservedAlphaSource {
                reference: alpha.reference,
                revision: alpha.revision,
            },
            blueprint: ObservedBlueprintSource {
                reference: blueprint.reference,
                revision: blueprint.revision,
            },
            alpha_input,
            term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                .expect("term"),
            source_question,
            replacement_question,
        }
    }

    pub(in crate::in_memory::curriculum_adoption::tests) async fn instantiate(
        &self,
        suffix: &str,
    ) -> AlphaInstantiationCompleted {
        let title = format!("Course {suffix}");
        let preview = self
            .store
            .preview_alpha_instantiation(
                self.context,
                self.session,
                AlphaInstantiationPreviewRequest {
                    source: self.alpha,
                    title: CurriculumAdoptionTitle::parse(&title).expect("title"),
                    target_term: self.term.clone(),
                    replacements: CurriculumPinReplacements::default(),
                },
            )
            .await
            .expect("preview");
        self.store
            .apply_alpha_instantiation(
                self.context,
                self.session,
                AlphaInstantiationCommand::from_preview(&preview, key(suffix))
                    .expect("corrected preview"),
            )
            .await
            .expect("apply")
    }
}
