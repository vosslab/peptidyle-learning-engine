//! Shared live-test fixture for the public Draft-to-Published assignment path.

use learning_data_access::{
    AssignmentRecord, PutAssignmentTeachingSettingsCommand, Store, StoreError, StoredAssignment,
    TenantContext,
};
use question_model::{AssignmentLifecycle, AssignmentTeachingSettings, UserId};

/// Creates a Draft assignment, then publishes it through the same revisioned
/// teaching-settings command used by production instructors.
pub async fn create_published_assignment<S: Store>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    mut assignment: AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
) -> Result<StoredAssignment, StoreError> {
    assert_eq!(
        assignment.lifecycle,
        AssignmentLifecycle::Published,
        "fixture describes its intended final lifecycle"
    );
    let instructions = assignment.instructions.clone();
    assignment.lifecycle = AssignmentLifecycle::Draft;
    let created = store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment,
                base_policy,
            },
        )
        .await?;
    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course: created.record.course_id,
                assignment: created.record.id,
                expected_revision: created.revision,
                settings: AssignmentTeachingSettings {
                    lifecycle: AssignmentLifecycle::Published,
                    instructions,
                    base_policy,
                },
            },
        )
        .await?;
    store
        .get_assignment_for_edit(context, created.record.id)
        .await?
        .ok_or(StoreError::NotFound)
}
