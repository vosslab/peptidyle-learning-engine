//! Deterministic Memory parity for the Student-work inspection boundary.

use async_trait::async_trait;
use question_model::{
    AttemptStatus, CourseMembershipRole, ScoringStatus,
    presentation::{project_durable_response_to_rendered_v1, reproduce_presentation_v1},
};

use super::{MemoryStore, State, require_course_records_accessible};
use crate::{
    CourseMemberStatus, InspectStudentWorkRequest, InspectedStudentSubmissionV1,
    InspectedStudentWorkDetailV1, SessionTokenHash, StoreError, StudentWorkInspectionAudit,
    StudentWorkInspectionAuditIntent, StudentWorkInspectionRecordAccess,
    StudentWorkInspectionReturnContext, StudentWorkInspectionStore, TenantContext,
};

#[async_trait]
impl StudentWorkInspectionStore for MemoryStore {
    async fn inspect_student_work(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: InspectStudentWorkRequest,
    ) -> Result<InspectedStudentWorkDetailV1, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let course = resolve_course(&state, tenant, request.course)?;
        require_course_records_accessible(&state, tenant, course)?;
        let actor =
            super::course_roster::require_course_instructor(&state, context, session, course)
                .map_err(conceal)?;
        let assignment = resolve_assignment(&state, tenant, request.assignment, course)?;
        let membership = resolve_student_membership(&state, tenant, request.membership, course)?;
        let run = resolve_run(&state, tenant, request.run, assignment, membership)?;
        validate_return_context(
            request.return_context,
            request.course,
            request.membership,
            request.assignment,
        )?;

        let (scoring_generation, scoring_status) = state
            .assignment_scoring
            .get(&(tenant, assignment))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let mut submissions = state
            .attempts
            .values()
            .filter(|attempt| attempt.tenant == tenant && attempt.run == run)
            .filter(|attempt| {
                matches!(
                    attempt.status,
                    AttemptStatus::Submitted
                        | AttemptStatus::AutoSubmitted
                        | AttemptStatus::NeedsManualGrading
                ) && attempt.response.is_some()
            })
            .map(|attempt| {
                inspect_attempt(
                    &state,
                    tenant,
                    attempt.id,
                    run,
                    scoring_generation,
                    scoring_status,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        submissions.sort_by_key(|submission| submission.submitted_at);
        if submissions.is_empty() {
            return Err(StoreError::NotFound);
        }

        let presentation_digests = submissions
            .iter()
            .map(|submission| submission.issued_presentation_digest)
            .collect::<Vec<_>>();
        // These two closed facts are appended together only after every
        // response has passed the immutable-record checks. They carry public
        // references and scoring state, never response or grading material.
        let occurred_at = state.authoritative_time;
        state
            .student_work_inspection_record_accesses
            .push(StudentWorkInspectionRecordAccess {
                actor,
                intent: StudentWorkInspectionAuditIntent::GradebookInspection,
                occurred_at,
                course: request.course,
                membership: request.membership,
                assignment: request.assignment,
                run: request.run,
                issued_presentation_digests: presentation_digests.clone(),
                scoring_generation,
                scoring_status,
            });
        state
            .student_work_inspection_audits
            .push(StudentWorkInspectionAudit {
                actor,
                intent: StudentWorkInspectionAuditIntent::GradebookInspection,
                occurred_at,
                course: request.course,
                membership: request.membership,
                assignment: request.assignment,
                run: request.run,
                issued_presentation_digests: presentation_digests,
                scoring_generation,
                scoring_status,
            });
        Ok(InspectedStudentWorkDetailV1 {
            course: request.course,
            membership: request.membership,
            assignment: request.assignment,
            run: request.run,
            submissions,
            return_context: request.return_context,
        })
    }
}

fn resolve_course(
    state: &State,
    tenant: question_model::TenantId,
    reference: question_model::CourseReference,
) -> Result<question_model::CourseId, StoreError> {
    state
        .courses_by_reference
        .get(&(tenant, reference))
        .copied()
        .ok_or(StoreError::NotFound)
}

fn resolve_assignment(
    state: &State,
    tenant: question_model::TenantId,
    reference: question_model::AssignmentReference,
    course: question_model::CourseId,
) -> Result<question_model::AssignmentId, StoreError> {
    let assignment = state
        .assignments_by_reference
        .get(&(tenant, reference))
        .copied()
        .ok_or(StoreError::NotFound)?;
    state
        .assignments
        .get(&(tenant, assignment))
        .filter(|record| record.course_id == course)
        .map(|_| assignment)
        .ok_or(StoreError::NotFound)
}

fn resolve_student_membership(
    state: &State,
    tenant: question_model::TenantId,
    reference: question_model::CourseMembershipReference,
    course: question_model::CourseId,
) -> Result<question_model::CourseMembershipId, StoreError> {
    let membership = state
        .course_memberships_by_reference
        .get(&(tenant, reference))
        .copied()
        .ok_or(StoreError::NotFound)?;
    state
        .course_memberships
        .get(&(tenant, membership))
        .filter(|record| {
            record.course == course
                && record.role == CourseMembershipRole::Student
                && record.status == CourseMemberStatus::Active
        })
        .map(|_| membership)
        .ok_or(StoreError::NotFound)
}

fn resolve_run(
    state: &State,
    tenant: question_model::TenantId,
    reference: question_model::RunReference,
    assignment: question_model::AssignmentId,
    membership: question_model::CourseMembershipId,
) -> Result<question_model::RunId, StoreError> {
    let run = state
        .runs_by_reference
        .get(&(tenant, reference))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let record = state.runs.get(&(tenant, run)).ok_or(StoreError::NotFound)?;
    let enrollment = state
        .enrollments
        .get(&(tenant, record.enrollment))
        .ok_or(StoreError::NotFound)?;
    let materialization = state
        .entitlement_materializations
        .get(&(tenant, enrollment.id))
        .ok_or(StoreError::NotFound)?;
    (enrollment.assignment == assignment && materialization.membership == membership)
        .then_some(run)
        .ok_or(StoreError::NotFound)
}

fn validate_return_context(
    context: StudentWorkInspectionReturnContext,
    course: question_model::CourseReference,
    membership: question_model::CourseMembershipReference,
    assignment: question_model::AssignmentReference,
) -> Result<(), StoreError> {
    let (returned_course, returned_membership, returned_assignment, focus_matches) = match context {
        StudentWorkInspectionReturnContext::Gradebook {
            course,
            membership,
            assignment,
            focus:
                crate::StudentWorkInspectionFocusTarget::GradebookCell {
                    membership: focused_membership,
                    assignment: focused_assignment,
                },
        } => (
            course,
            membership,
            assignment,
            focused_membership == membership && focused_assignment == assignment,
        ),
        StudentWorkInspectionReturnContext::GradingOperation {
            course,
            membership,
            assignment,
            operation,
            focus:
                crate::StudentWorkInspectionFocusTarget::GradingOperationControl {
                    membership: focused_membership,
                    assignment: focused_assignment,
                    operation: focused_operation,
                },
        } => (
            course,
            membership,
            assignment,
            focused_membership == membership
                && focused_assignment == assignment
                && focused_operation == operation,
        ),
        _ => return Err(StoreError::NotFound),
    };
    (returned_course == course
        && returned_membership == membership
        && returned_assignment == assignment
        && focus_matches)
        .then_some(())
        .ok_or(StoreError::NotFound)
}

fn inspect_attempt(
    state: &State,
    tenant: question_model::TenantId,
    attempt_id: question_model::QuestionAttemptId,
    run: question_model::RunId,
    scoring_generation: question_model::ScoringGeneration,
    scoring_status: ScoringStatus,
) -> Result<InspectedStudentSubmissionV1, StoreError> {
    let attempt = state
        .attempts
        .get(&(tenant, attempt_id))
        .ok_or(StoreError::NotFound)?;
    let private_response = state
        .private_submission_responses
        .get(&(tenant, attempt_id))
        .ok_or(StoreError::NotFound)?;
    let receipt = state
        .submissions
        .get(&(tenant, attempt_id))
        .and_then(super::StoredSubmission::completed_record_opt)
        .ok_or(StoreError::NotFound)?;
    let public_response = attempt.response.as_ref().ok_or(StoreError::NotFound)?;
    if !super::private_submission::stored_submission_matches_response(
        state,
        tenant,
        attempt_id,
        public_response,
    )? {
        return Err(StoreError::NotFound);
    }
    let submitted_at = attempt.timer.submitted_at.ok_or(StoreError::NotFound)?;
    let envelope = state
        .attempt_grading_envelopes
        .get(&(tenant, attempt_id))
        .ok_or(StoreError::NotFound)?;
    let binding = state
        .attempt_presentations
        .get(&(tenant, attempt_id))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let snapshot = state
        .attempt_presentation_snapshots
        .get(&(tenant, attempt_id))
        .ok_or(StoreError::NotFound)?;
    let receipt_presentation = receipt.presentation.as_ref().ok_or(StoreError::NotFound)?;
    if receipt.attempt.id != attempt_id
        || receipt.run.id != run
        || receipt.attempt.timer.submitted_at != Some(submitted_at)
        || receipt_presentation != snapshot
    {
        return Err(StoreError::NotFound);
    }
    let presentation = reproduce_presentation_v1(envelope, &snapshot.asset_bindings, binding)
        .map_err(|_| StoreError::NotFound)?;
    if presentation.envelope != snapshot.envelope || presentation.digest != binding.digest() {
        return Err(StoreError::NotFound);
    }
    let projection =
        project_durable_response_to_rendered_v1(&private_response.response, &presentation)
            .map_err(|_| StoreError::NotFound)?;
    Ok(InspectedStudentSubmissionV1 {
        submitted_at,
        presentation: snapshot.clone(),
        issued_presentation_digest: binding.digest(),
        scoring_generation,
        response: projection,
        scoring_status,
    })
}

fn conceal(_: StoreError) -> StoreError {
    StoreError::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StudentWorkInspectionFocusTarget;

    fn course() -> question_model::CourseReference {
        "C-1".parse().expect("course reference")
    }

    fn membership() -> question_model::CourseMembershipReference {
        "M-2".parse().expect("membership reference")
    }

    fn assignment() -> question_model::AssignmentReference {
        "A-3".parse().expect("assignment reference")
    }

    #[test]
    fn gradebook_return_context_requires_the_resolved_cell_focus() {
        let valid = StudentWorkInspectionReturnContext::Gradebook {
            course: course(),
            membership: membership(),
            assignment: assignment(),
            focus: StudentWorkInspectionFocusTarget::GradebookCell {
                membership: membership(),
                assignment: assignment(),
            },
        };
        assert_eq!(
            validate_return_context(valid, course(), membership(), assignment()),
            Ok(())
        );

        let invalid = StudentWorkInspectionReturnContext::Gradebook {
            course: course(),
            membership: membership(),
            assignment: assignment(),
            focus: StudentWorkInspectionFocusTarget::GradebookCell {
                membership: "M-4".parse().expect("other membership reference"),
                assignment: assignment(),
            },
        };
        assert_eq!(
            validate_return_context(invalid, course(), membership(), assignment()),
            Err(StoreError::NotFound)
        );
    }
}
