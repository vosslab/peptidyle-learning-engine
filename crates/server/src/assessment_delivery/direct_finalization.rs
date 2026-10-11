//! Server-only evaluation of one immutable saved-response snapshot.
//!
//! PostgreSQL authorizes and captures the snapshot, this module calls the
//! Question Backend without holding a database transaction, and PostgreSQL
//! later accepts it only when the snapshot remains current.

use adapter_webwork::{HttpWebworkRenderer, WebworkAdapter};
use learning_data_access::{
    StoreError, StudentAssessmentAttemptFinalizationEvaluation,
    StudentAssessmentAttemptFinalizationSource,
};
use objects::ObjectStore;

/// Evaluates every saved response from its exact immutable source.
///
/// No transaction is held across object-store or renderer I/O. Any failure
/// leaves acceptance to the caller's PostgreSQL commit step, which therefore
/// records no partial submission. ASVS 2.3.3.
pub(crate) async fn evaluate_saved_responses<S: ObjectStore>(
    objects: &S,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
    responses: &[StudentAssessmentAttemptFinalizationSource],
) -> Result<Vec<StudentAssessmentAttemptFinalizationEvaluation>, StoreError> {
    let mut evaluations = Vec::with_capacity(responses.len());
    for response in responses {
        evaluations.push(StudentAssessmentAttemptFinalizationEvaluation {
            question_attempt_id: response.question_attempt_id,
            saved_at: response.saved_at,
            student_response: response.student_response.clone(),
            normalized_credit: super::question_backend::grade_saved_response(
                objects, webwork, response,
            )
            .await?,
        });
    }
    Ok(evaluations)
}
