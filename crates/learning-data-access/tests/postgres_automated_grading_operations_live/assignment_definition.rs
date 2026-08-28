//! Assignment-definition invalidation evidence through the revision capability.

use learning_data_access::{AssignmentUpdate, ReplaceAssignmentCommand, Store, TenantContext};
use question_model::{AssignmentId, CourseId, PointValue, TenantId, UserId};
use sqlx::{PgPool, Row};

pub(super) struct AssignmentDefinitionScenario<'a> {
    pub(super) store: &'a learning_data_access::postgres::PostgresStore,
    pub(super) pool: &'a PgPool,
    pub(super) context: TenantContext,
    pub(super) tenant: TenantId,
    pub(super) instructor: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
}

pub(super) async fn prove_assignment_definition_origin(scenario: AssignmentDefinitionScenario<'_>) {
    let AssignmentDefinitionScenario {
        store,
        pool,
        context,
        tenant,
        instructor,
        course,
        assignment,
    } = scenario;
    let current = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("read scored assignment for definition replacement")
        .expect("scored assignment exists");
    let mut items = current.record.items.clone();
    let item = items
        .first_mut()
        .expect("connected assignment has one fixed item");
    item.points_possible = PointValue::from_whole(2);
    let replacement = store
        .replace_assignment(
            context,
            ReplaceAssignmentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: current.revision,
                update: AssignmentUpdate {
                    title: current.record.title.clone(),
                    audience: current.record.audience.clone(),
                    items,
                    selection_groups: current.record.selection_groups.clone(),
                    disclosure_policy: current.record.disclosure_policy,
                    policies: current.record.policies,
                },
            },
        )
        .await
        .expect("replace score-relevant assignment definition");
    let row = sqlx::query(
        "SELECT origin_id, actor_id, scoring_generation, recalculation_job_id, grading_operation_id \
         FROM public.scoring_invalidation_origin \
         WHERE tenant_id=$1 AND origin_kind='assignment_definition' \
           AND assignment_id=$2 AND scoring_generation=$3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(replacement.scoring_generation.value()).expect("generation fits bigint"))
    .fetch_one(pool)
    .await
    .expect("read assignment-definition invalidation origin");
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("actor_id").unwrap(),
        instructor.as_uuid(),
        "definition source retains its authorized editor"
    );
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("origin_id").unwrap(),
        row.try_get::<uuid::Uuid, _>("recalculation_job_id")
            .unwrap(),
        "definition uses one canonical SHA-derived origin and recalculation job"
    );
    assert!(
        row.try_get::<i64, _>("grading_operation_id").unwrap() > 0,
        "definition replacement creates one Instructor-visible operation"
    );
}
