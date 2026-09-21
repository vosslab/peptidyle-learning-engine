//! Server-only evaluation of one immutable saved-response snapshot.
//!
//! PostgreSQL authorizes and captures the snapshot, this module calls the
//! Question Backend without holding a database transaction, and PostgreSQL
//! later accepts it only when the snapshot remains current.

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use learning_data_access::{
    StoreError, StudentAssessmentAttemptFinalizationBackend,
    StudentAssessmentAttemptFinalizationEvaluation, StudentAssessmentAttemptFinalizationSource,
};
use objects::s3::S3ObjectStore;
use question_model::{
    ObjectId, PublishedQuestionRevisionTuple, QuestionReproduction, QuestionRevisionNumber,
    SourceObjectChecksum,
};
use uuid::Uuid;

/// Evaluates every saved response from its exact immutable source.
///
/// No transaction is held across object-store or renderer I/O. Any failure
/// leaves acceptance to the caller's PostgreSQL commit step, which therefore
/// records no partial submission. ASVS 2.3.3.
pub(crate) async fn evaluate_saved_responses(
    objects: &S3ObjectStore,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
    responses: &[StudentAssessmentAttemptFinalizationSource],
) -> Result<Vec<StudentAssessmentAttemptFinalizationEvaluation>, StoreError> {
    let mut evaluations = Vec::with_capacity(responses.len());
    for response in responses {
        evaluations.push(StudentAssessmentAttemptFinalizationEvaluation {
            question_attempt_id: response.question_attempt_id,
            saved_at: response.saved_at,
            student_response: response.student_response.clone(),
            normalized_credit: evaluate_one(objects, webwork, response).await?,
        });
    }
    Ok(evaluations)
}

async fn evaluate_one(
    objects: &S3ObjectStore,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<f64, StoreError> {
    let published_question_revision_tuple = PublishedQuestionRevisionTuple {
        published_question_id: response.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(response.revision_number)
            .map_err(|_| finalization_unavailable())?,
    };
    let source_object_id =
        Uuid::parse_str(&response.source_object_id).map_err(|_| finalization_unavailable())?;
    let source = ObjectId::from_uuid(source_object_id);
    let checksum = SourceObjectChecksum::parse(response.source_object_checksum.clone())
        .map_err(|_| finalization_unavailable())?;
    match &response.backend {
        StudentAssessmentAttemptFinalizationBackend::Ple => {
            if response.reproduction != QuestionReproduction::Static {
                return Err(finalization_unavailable());
            }
            let source = ResolvedPleQuestionJsonSource::resolve(
                objects,
                published_question_revision_tuple,
                source,
                checksum,
            )
            .await
            .map_err(|_| finalization_unavailable())?;
            PleQuestionBackend::new()
                .grade_question_json(&source, &response.student_response)
                .map(|evaluation| evaluation.evaluation.normalized_credit())
                .map_err(|_| finalization_unavailable())
        }
        StudentAssessmentAttemptFinalizationBackend::Webwork { pg_path } => {
            let seed = response
                .reproduction
                .question_seed()
                .ok_or_else(finalization_unavailable)?;
            let binding = WebworkQuestionSourceBinding::new(
                published_question_revision_tuple,
                pg_path.clone(),
            )
            .map_err(|_| finalization_unavailable())?;
            let source = ResolvedWebworkQuestionSource::resolve(objects, binding, source, checksum)
                .await
                .map_err(|_| finalization_unavailable())?;
            match webwork
                .grade(seed, &source, &response.student_response)
                .await
                .map_err(|_| finalization_unavailable())?
            {
                grading::QuestionGradingOutcome::Evaluated(evaluation) => {
                    Ok(evaluation.normalized_credit())
                }
                grading::QuestionGradingOutcome::Ungraded => Err(finalization_unavailable()),
            }
        }
    }
}

fn finalization_unavailable() -> StoreError {
    StoreError::Unavailable("Question response unavailable".to_string())
}
