//! One issuance and grading interface for the production Question Backends.
//!
//! PLE and WeBWorK keep their source, renderer, and response details behind
//! this boundary. Delivery selects an adapter and calls these methods.

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::renderer_contract::WebworkRenderer;
use adapter_webwork::{
    ResolvedWebworkQuestionSource, WebworkAdapter, WebworkQuestionSourceBinding,
};
use async_trait::async_trait;
use learning_data_access::{
    NativePleIssuanceSource, NativePresentationInput, NativeWebworkIssuanceSource,
    QuestionIssuanceReproductionInput, StoreError, StudentAssessmentAttemptFinalizationBackend,
    StudentAssessmentAttemptFinalizationSource,
};
use objects::ObjectStore;
use question_model::presentation::build_question_presentation;
use question_model::question_library::QuestionBackendInterface;
use question_model::{
    ObjectId, PublishedQuestionRevisionTuple, QuestionBackend, QuestionBackendCapabilities,
    QuestionReproduction, QuestionRevisionNumber, SourceObjectChecksum,
};

use super::{
    StartError, question_image_renditions_from_ready, resolve_source, resolve_webwork_source,
};

/// The basic interface every production Question Backend implements.
///
/// iMathAS and H5P stay deferred, so this closed pair is the production set.
#[async_trait]
pub(super) trait BasicQuestionBackend<S: ObjectStore> {
    type IssuanceSource: Sync;

    async fn issue_one(
        &self,
        objects: &S,
        source: &Self::IssuanceSource,
    ) -> Result<NativePresentationInput, StartError>;

    async fn grade_saved_response(
        &self,
        objects: &S,
        response: &StudentAssessmentAttemptFinalizationSource,
    ) -> Result<f64, StoreError>;
}

pub(super) struct PleQuestionBackendAdapter;

pub(super) struct WebworkQuestionBackendAdapter<'a, R: WebworkRenderer> {
    pub(super) adapter: &'a WebworkAdapter<R>,
}

pub(super) async fn grade_saved_response<S: ObjectStore, R: WebworkRenderer>(
    objects: &S,
    webwork: &WebworkAdapter<R>,
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<f64, StoreError> {
    match &response.backend {
        StudentAssessmentAttemptFinalizationBackend::Ple => {
            PleQuestionBackendAdapter
                .grade_saved_response(objects, response)
                .await
        }
        StudentAssessmentAttemptFinalizationBackend::Webwork { .. } => {
            WebworkQuestionBackendAdapter { adapter: webwork }
                .grade_saved_response(objects, response)
                .await
        }
    }
}

#[async_trait]
impl<S: ObjectStore> BasicQuestionBackend<S> for PleQuestionBackendAdapter {
    type IssuanceSource = NativePleIssuanceSource;

    async fn issue_one(
        &self,
        objects: &S,
        source: &NativePleIssuanceSource,
    ) -> Result<NativePresentationInput, StartError> {
        if source.reproduction != QuestionIssuanceReproductionInput::Static {
            return Err(StartError::Invalid);
        }
        let backend = super::ple_shell::native_ple_backend(QuestionBackendInterface::new(
            QuestionBackend::Ple,
            QuestionBackendCapabilities::none(),
        ))?;
        let resolved = resolve_source(objects, source).await?;
        let issued = backend
            .issue_question_json(&resolved, &source.question_title)
            .map_err(|_| StartError::Invalid)?;
        let retained_renditions = super::presentation_images::presentation_renditions(
            &issued.presentation,
            &source.question_image_renditions,
        );
        let presentation = build_question_presentation(
            &issued.presentation,
            &question_image_renditions_from_ready(&retained_renditions),
        )
        .map_err(|_| StartError::Unavailable)?;
        if presentation.reproduction != QuestionReproduction::Static {
            return Err(StartError::Invalid);
        }
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&presentation)
                .map_err(|_| StartError::Invalid)?;
        Ok(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assessment_entry_id: source.assessment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            reproduction: presentation.reproduction,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            author_content: presentation.author_content.clone(),
            response_item_bindings,
            question_image_renditions: retained_renditions,
            issued_capability: "ple_question_json_presentation".to_string(),
            backend_document: None,
        })
    }

    async fn grade_saved_response(
        &self,
        objects: &S,
        response: &StudentAssessmentAttemptFinalizationSource,
    ) -> Result<f64, StoreError> {
        if response.reproduction != QuestionReproduction::Static {
            return Err(unavailable());
        }
        let published_question_revision_tuple = revision_tuple(response)?;
        let source = ResolvedPleQuestionJsonSource::resolve_with_images(
            objects,
            published_question_revision_tuple,
            object_id(response)?,
            checksum(response)?,
            &super::presentation_images::question_html_image_bindings(
                &response.question_image_renditions,
            ),
        )
        .await
        .map_err(|_| unavailable())?;
        PleQuestionBackend::new()
            .grade_question_json(&source, &response.student_response)
            .map(|evaluation| evaluation.evaluation.normalized_credit())
            .map_err(|_| unavailable())
    }
}

#[async_trait]
impl<S: ObjectStore, R: WebworkRenderer> BasicQuestionBackend<S>
    for WebworkQuestionBackendAdapter<'_, R>
{
    type IssuanceSource = NativeWebworkIssuanceSource;

    async fn issue_one(
        &self,
        objects: &S,
        source: &NativeWebworkIssuanceSource,
    ) -> Result<NativePresentationInput, StartError> {
        let seed = match &source.reproduction {
            QuestionIssuanceReproductionInput::Seeded { question_seed } => *question_seed,
            QuestionIssuanceReproductionInput::Static => return Err(StartError::Invalid),
        };
        let issued = self
            .adapter
            .issue(seed, &resolve_webwork_source(objects, source).await?)
            .await
            .map_err(|_| StartError::Unavailable)?;
        let document = String::from_utf8(issued.document).map_err(|_| StartError::Invalid)?;
        let presentation = build_question_presentation(
            &issued.presentation,
            &question_image_renditions_from_ready(&source.question_image_renditions),
        )
        .map_err(|_| StartError::Unavailable)?;
        if presentation.reproduction.question_seed() != Some(seed) {
            return Err(StartError::Invalid);
        }
        let response_item_bindings =
            question_model::presentation::extract_durable_response_item_bindings(&presentation)
                .map_err(|_| StartError::Invalid)?;
        Ok(NativePresentationInput {
            issued_question_id: source
                .issued_question_id
                .ok_or(StartError::Invalid)?
                .to_string(),
            assessment_entry_id: source.assessment_entry_id.clone(),
            position: source.position,
            question_id: source.question_id.clone(),
            revision_number: source.revision_number,
            reproduction: presentation.reproduction,
            reproduction_details: serde_json::to_value(issued.reproduction_details)
                .map_err(|_| StartError::Invalid)?,
            presentation: serde_json::to_value(&presentation.presentation)
                .map_err(|_| StartError::Invalid)?,
            presentation_nonce: presentation.presentation.presentation_nonce.to_hex(),
            presentation_checksum: presentation.checksum.to_hex(),
            author_content: None,
            response_item_bindings,
            question_image_renditions: source.question_image_renditions.clone(),
            issued_capability: "webwork_presentation".to_string(),
            backend_document: Some(document),
        })
    }

    async fn grade_saved_response(
        &self,
        objects: &S,
        response: &StudentAssessmentAttemptFinalizationSource,
    ) -> Result<f64, StoreError> {
        let StudentAssessmentAttemptFinalizationBackend::Webwork { pg_path } = &response.backend
        else {
            return Err(unavailable());
        };
        let seed = response
            .reproduction
            .question_seed()
            .ok_or_else(unavailable)?;
        let binding = WebworkQuestionSourceBinding::new(revision_tuple(response)?, pg_path.clone())
            .map_err(|_| unavailable())?;
        let source = ResolvedWebworkQuestionSource::resolve(
            objects,
            binding,
            object_id(response)?,
            checksum(response)?,
        )
        .await
        .map_err(|_| unavailable())?;
        match self
            .adapter
            .grade(seed, &source, &response.student_response)
            .await
            .map_err(|_| unavailable())?
        {
            grading::QuestionGradingOutcome::Evaluated(evaluation) => {
                Ok(evaluation.normalized_credit())
            }
            grading::QuestionGradingOutcome::Ungraded => Err(unavailable()),
        }
    }
}

fn revision_tuple(
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<PublishedQuestionRevisionTuple, StoreError> {
    Ok(PublishedQuestionRevisionTuple {
        published_question_id: response.question_id.clone(),
        revision_number: QuestionRevisionNumber::new(response.revision_number)
            .map_err(|_| unavailable())?,
    })
}

fn object_id(
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<ObjectId, StoreError> {
    let source_object_id =
        uuid::Uuid::parse_str(&response.source_object_id).map_err(|_| unavailable())?;
    Ok(ObjectId::from_uuid(source_object_id))
}

fn checksum(
    response: &StudentAssessmentAttemptFinalizationSource,
) -> Result<SourceObjectChecksum, StoreError> {
    SourceObjectChecksum::parse(response.source_object_checksum.clone()).map_err(|_| unavailable())
}

fn unavailable() -> StoreError {
    StoreError::Unavailable("Question response unavailable".to_string())
}
