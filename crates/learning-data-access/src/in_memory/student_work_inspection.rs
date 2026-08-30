//! Deterministic Memory parity for the Student-work inspection boundary.

use async_trait::async_trait;
use domain::disclosure_policy::project_inspected_student_score_feedback;
use question_model::{
    CourseMembershipRole, RunCompletionStatus, ScoringStatus, StudentResponse,
    TeachingDisplayLabel, presentation,
};

use super::{MemoryStore, State};
use crate::{
    ActorContext, CourseMemberStatus, InspectStudentWorkRequest, InspectedStudentSubmissionV1,
    InspectedStudentWorkDetailV1, InspectedSubmissionEvidenceV1, SessionTokenHash, StoreError,
    StudentWorkInspectionAudit, StudentWorkInspectionAuditIntent,
    StudentWorkInspectionEvidenceWitness, StudentWorkInspectionRecordAccess,
    StudentWorkInspectionReturnContext, StudentWorkInspectionStore,
    StudentWorkInspectionSubmissionWitness, SubmissionReceiptRead,
};

#[async_trait]
impl StudentWorkInspectionStore for MemoryStore {
    async fn inspect_student_work(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        request: InspectStudentWorkRequest,
    ) -> Result<InspectedStudentWorkDetailV1, StoreError> {
        let mut state = self.write_state()?;
        let course = resolve_course(&state, request.course)?;
        super::require_course_records_accessible(&state, course)?;
        let actor =
            super::course_roster::require_course_instructor(&state, context, session, course)
                .map_err(conceal)?;
        let assignment = resolve_assignment(&state, request.assignment, course)?;
        let membership = resolve_student_membership(&state, request.membership, course)?;
        let run = resolve_run(&state, request.run, assignment, membership)?;
        let student_display_label = state
            .roster_profiles
            .get(&(course, membership))
            .map(|profile| TeachingDisplayLabel::try_from(profile.display_name.clone()))
            .transpose()
            .map_err(|_| StoreError::NotFound)?
            .ok_or(StoreError::NotFound)?;
        let assignment_title = state
            .assignments
            .get(&assignment)
            .filter(|record| record.course_id == course)
            .map(|record| record.title.clone())
            .ok_or(StoreError::NotFound)?;
        if state
            .runs
            .get(&run)
            .is_none_or(|record| record.completion_status() != RunCompletionStatus::Completed)
        {
            return Err(StoreError::NotFound);
        }
        validate_return_context(
            request.return_context,
            request.course,
            request.membership,
            request.assignment,
        )?;

        let (scoring_generation, scoring_status) = state
            .assignment_scoring
            .get(&assignment)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let mut submissions = state
            .attempts
            .values()
            .filter(|attempt| attempt.run == run)
            .map(|attempt| {
                inspect_attempt(&state, attempt.id, run, scoring_generation, scoring_status)
            })
            .collect::<Result<Vec<_>, _>>()?;
        submissions.sort_by_key(|submission| {
            inspection_submission_order_key(
                submission.submitted_at,
                submission.assignment_position,
                submission.attempt,
            )
        });
        if submissions.is_empty() {
            return Err(StoreError::NotFound);
        }

        let submission_witnesses = submissions
            .iter()
            .map(|submission| StudentWorkInspectionSubmissionWitness {
                attempt: submission.attempt,
                submitted_at: submission.submitted_at,
                evidence: match submission.evidence {
                    InspectedSubmissionEvidenceV1::IssuedPresentation {
                        issued_presentation_digest,
                        ..
                    } => StudentWorkInspectionEvidenceWitness::IssuedPresentation {
                        digest: issued_presentation_digest,
                    },
                    InspectedSubmissionEvidenceV1::PresentationNotApplicable => {
                        StudentWorkInspectionEvidenceWitness::PresentationNotApplicable
                    }
                },
            })
            .collect::<Vec<_>>();
        // These two closed facts are appended together only after every
        // response has passed the immutable-record checks. They retain internal
        // identity witnesses and scoring state, never response or grading material.
        let occurred_at = state.authoritative_time;
        state
            .student_work_inspection_record_accesses
            .push(StudentWorkInspectionRecordAccess {
                actor,
                intent: StudentWorkInspectionAuditIntent::GradebookInspection,
                occurred_at,
                course,
                membership,
                assignment,
                run,
                submissions: submission_witnesses.clone(),
                scoring_generation,
                scoring_status,
            });
        state
            .student_work_inspection_audits
            .push(StudentWorkInspectionAudit {
                actor,
                intent: StudentWorkInspectionAuditIntent::GradebookInspection,
                occurred_at,
                course,
                membership,
                assignment,
                run,
                submissions: submission_witnesses,
                scoring_generation,
                scoring_status,
            });
        Ok(InspectedStudentWorkDetailV1 {
            course: request.course,
            membership: request.membership,
            assignment: request.assignment,
            run: request.run,
            student_display_label,
            assignment_title,
            submissions,
            return_context: request.return_context,
        })
    }
}

fn inspection_submission_order_key(
    submitted_at: question_model::ActivityTimestamp,
    assignment_position: u32,
    attempt: question_model::QuestionAttemptId,
) -> (
    question_model::ActivityTimestamp,
    u32,
    question_model::QuestionAttemptId,
) {
    (submitted_at, assignment_position, attempt)
}

fn resolve_course(
    state: &State,
    reference: question_model::CourseReference,
) -> Result<question_model::CourseId, StoreError> {
    state
        .courses_by_reference
        .get(&reference)
        .copied()
        .ok_or(StoreError::NotFound)
}

fn resolve_assignment(
    state: &State,
    reference: question_model::AssignmentReference,
    course: question_model::CourseId,
) -> Result<question_model::AssignmentId, StoreError> {
    let assignment = state
        .assignments_by_reference
        .get(&reference)
        .copied()
        .ok_or(StoreError::NotFound)?;
    state
        .assignments
        .get(&assignment)
        .filter(|record| record.course_id == course)
        .map(|_| assignment)
        .ok_or(StoreError::NotFound)
}

fn resolve_student_membership(
    state: &State,
    reference: question_model::CourseMembershipReference,
    course: question_model::CourseId,
) -> Result<question_model::CourseMembershipId, StoreError> {
    let membership = state
        .course_memberships_by_reference
        .get(&reference)
        .copied()
        .ok_or(StoreError::NotFound)?;
    state
        .course_memberships
        .get(&membership)
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
    reference: question_model::RunReference,
    assignment: question_model::AssignmentId,
    membership: question_model::CourseMembershipId,
) -> Result<question_model::RunId, StoreError> {
    let run = state
        .runs_by_reference
        .get(&reference)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let record = state.runs.get(&run).ok_or(StoreError::NotFound)?;
    let enrollment = state
        .enrollments
        .get(&record.enrollment)
        .ok_or(StoreError::NotFound)?;
    let materialization = state
        .entitlement_materializations
        .get(&enrollment.id)
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
    attempt_id: question_model::QuestionAttemptId,
    run: question_model::RunId,
    scoring_generation: question_model::ScoringGeneration,
    scoring_status: ScoringStatus,
) -> Result<InspectedStudentSubmissionV1, StoreError> {
    let attempt = state
        .attempts
        .get(&attempt_id)
        .ok_or(StoreError::NotFound)?;
    // This is the sole completed-receipt reader. It verifies the immutable
    // issued snapshot, reconstructs the current disclosure input, and never
    // substitutes mutable catalog state for the receipt evidence.
    let SubmissionReceiptRead::Completed(receipt) =
        super::runs::load_submission_record(state, attempt).map_err(conceal)?
    else {
        return Err(StoreError::NotFound);
    };
    let submitted_at = receipt
        .attempt
        .timer
        .submitted_at
        .ok_or(StoreError::NotFound)?;
    let private_response =
        super::private_submission::load_verified_private_submission_response(state, attempt_id)
            .map_err(conceal)?;
    if receipt.attempt.id != attempt_id || receipt.run.id != run {
        return Err(StoreError::NotFound);
    }
    let (evidence, projection) = match receipt.presentation.as_ref() {
        Some(snapshot) => {
            let envelope = state
                .attempt_grading_envelopes
                .get(&attempt_id)
                .ok_or(StoreError::NotFound)?;
            let binding = state
                .attempt_presentations
                .get(&attempt_id)
                .copied()
                .ok_or(StoreError::NotFound)?;
            let presentation = presentation::reproduce_presentation_v1(
                envelope,
                &snapshot.asset_bindings,
                binding,
            )
            .map_err(|_| StoreError::NotFound)?;
            if presentation.envelope != snapshot.envelope || presentation.digest != binding.digest()
            {
                return Err(StoreError::NotFound);
            }
            (
                InspectedSubmissionEvidenceV1::IssuedPresentation {
                    presentation: Box::new(snapshot.clone()),
                    issued_presentation_digest: binding.digest(),
                },
                presentation::project_rendered_response_for_inspection_v1(
                    private_response,
                    &presentation,
                )
                .map_err(|_| StoreError::NotFound)?,
            )
        }
        None => {
            if receipt.attempt.issued_capability
                != question_model::IssuedAttemptCapabilityV1::NotApplicable
                || !matches!(private_response, StudentResponse::ExternalTool {})
            {
                return Err(StoreError::NotFound);
            }
            (
                InspectedSubmissionEvidenceV1::PresentationNotApplicable,
                question_model::presentation::InspectedStudentResponseV1::ExternalTool {
                    completion: question_model::presentation::InspectedExternalToolStateV1::SubmissionRecorded,
                },
            )
        }
    };
    let feedback = project_inspected_student_score_feedback(
        receipt.disclosure.decision(),
        scoring_status,
        receipt.attempt.result,
    );
    Ok(InspectedStudentSubmissionV1 {
        attempt: attempt_id,
        submitted_at,
        assignment_position: receipt.attempt.assignment_position,
        evidence,
        scoring_generation,
        feedback,
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
    fn inspection_order_uses_assignment_position_before_attempt_id_at_equal_timestamps() {
        let submitted_at = question_model::ActivityTimestamp::from_unix_millis(1_000);
        let later_id = question_model::QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(2));
        let earlier_id = question_model::QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(1));
        assert!(
            inspection_submission_order_key(submitted_at, 0, later_id)
                < inspection_submission_order_key(submitted_at, 1, earlier_id)
        );
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
