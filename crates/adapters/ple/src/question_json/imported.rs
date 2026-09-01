//! PLE-owned construction of canonical PLE Question JSON from trusted imports.
//!
//! This module is deliberately not a QTI adapter. Its input is the already
//! mapped, server-only PLE Question JSON shape. It fixes the PLE defaults required
//! for an imported v2 static single-choice item, then delegates validation,
//! canonical serialization, and compilation to the PLE Question JSON owner.

use std::fmt;

use question_model::{DraftQuestionContent, WorkspaceId};

use super::{
    CompiledPleQuestionJson, PleQuestionJsonChoice, PleQuestionJsonDocument, PleQuestionJsonError,
};

/// One ordered PLE choice from a trusted profile mapping.
///
/// ```compile_fail
/// use adapter_ple::question_json::imported::ImportedChoice;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<ImportedChoice>();
/// ```
///
/// ```compile_fail
/// use adapter_ple::question_json::imported::ImportedChoice;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<ImportedChoice>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct ImportedChoice {
    id: String,
    text: String,
}

impl ImportedChoice {
    /// Creates one trusted ordered PLE choice.
    pub fn new(id: String, text: String) -> Self {
        Self { id, text }
    }
}

/// Trusted mapped fields for one imported static single-choice question.
///
/// The input owns an answer binding and therefore intentionally implements
/// neither `Debug` nor serialization.
///
/// ```compile_fail
/// use adapter_ple::question_json::imported::ImportedSingleChoiceInput;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<ImportedSingleChoiceInput>();
/// ```
///
/// ```compile_fail
/// use adapter_ple::question_json::imported::ImportedSingleChoiceInput;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<ImportedSingleChoiceInput>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct ImportedSingleChoiceInput {
    title: String,
    question_description: String,
    prompt: String,
    choices: Vec<ImportedChoice>,
    correct_choice: String,
    canonical_points: String,
}

impl ImportedSingleChoiceInput {
    /// Creates the bounded trusted input to the PLE Question JSON import.
    pub fn new(
        title: String,
        question_description: String,
        prompt: String,
        choices: Vec<ImportedChoice>,
        correct_choice: String,
        canonical_points: String,
    ) -> Self {
        Self {
            title,
            question_description,
            prompt,
            choices,
            correct_choice,
            canonical_points,
        }
    }
}

/// A validated canonical imported source with no direct document access.
///
/// It exposes only canonical source bytes and the normal split compiler, so
/// a caller cannot bypass PLE Question JSON validation or defaults.
///
/// ```compile_fail
/// use adapter_ple::question_json::imported::ImportedPleQuestionJson;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<ImportedPleQuestionJson>();
/// ```
///
/// ```compile_fail
/// use adapter_ple::question_json::imported::ImportedPleQuestionJson;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<ImportedPleQuestionJson>();
/// ```
#[derive(Clone, PartialEq)]
pub struct ImportedPleQuestionJson {
    document: PleQuestionJsonDocument,
}

/// Input failure for trusted PLE Question JSON import construction.
///
/// This error deliberately records only the failure class, never a title,
/// prompt, choice, correct answer, or other imported value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportedPleQuestionJsonError {
    /// The supplied points string is not the canonical finite nonnegative form.
    InvalidCanonicalPoints,
    /// The supplied mapped fields violate the PLE Question JSON schema-version-2 contract.
    InvalidQuestionJson,
}

impl fmt::Display for ImportedPleQuestionJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanonicalPoints => {
                formatter.write_str("imported points must use canonical finite nonnegative form")
            }
            Self::InvalidQuestionJson => {
                formatter.write_str("imported fields do not form valid PLE Question JSON")
            }
        }
    }
}

impl std::error::Error for ImportedPleQuestionJsonError {}

impl ImportedPleQuestionJson {
    /// Constructs canonical schema-version-2 PLE Question JSON using fixed import defaults.
    ///
    /// # Errors
    ///
    /// Refuses noncanonical points and any mapped field that does not satisfy
    /// the PLE Question JSON schema-version-2 validation contract.
    pub fn from_imported(
        input: ImportedSingleChoiceInput,
    ) -> Result<Self, ImportedPleQuestionJsonError> {
        let points = parse_canonical_points(&input.canonical_points)?;
        let document = PleQuestionJsonDocument(
            super::schema_v2::PleQuestionJsonDocumentBody::imported_single_choice(
                input.title,
                input.question_description,
                input.prompt,
                input
                    .choices
                    .into_iter()
                    .map(|choice| PleQuestionJsonChoice {
                        id: choice.id,
                        text: choice.text,
                        feedback: None,
                    })
                    .collect(),
                input.correct_choice,
                points,
            ),
        );
        document
            .validate()
            .map_err(|_| ImportedPleQuestionJsonError::InvalidQuestionJson)?;
        let canonical = document
            .canonical_bytes()
            .map_err(|_| ImportedPleQuestionJsonError::InvalidQuestionJson)?;
        let document = PleQuestionJsonDocument::parse(&canonical)
            .map_err(|_| ImportedPleQuestionJsonError::InvalidQuestionJson)?;
        Ok(Self { document })
    }

    /// Returns the PLE canonical source bytes for immutable source storage.
    ///
    /// # Errors
    ///
    /// Propagates the PLE canonical serializer's encoding failure.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PleQuestionJsonError> {
        self.document.canonical_bytes()
    }

    /// Compiles the source into its public draft, grader-only key, and optional Question Hint.
    ///
    /// # Errors
    ///
    /// Propagates the authoritative PLE Question JSON compiler failure.
    pub fn compile_parts(
        &self,
        workspace: WorkspaceId,
    ) -> Result<
        (
            DraftQuestionContent,
            grading::ple_question_json::PleQuestionJsonPrivateGrading,
            Option<question_model::QuestionHint>,
        ),
        PleQuestionJsonError,
    > {
        let CompiledPleQuestionJson {
            draft,
            private,
            question_hint,
        } = self.document.compile(workspace)?;
        Ok((draft, private, question_hint))
    }
}

fn parse_canonical_points(value: &str) -> Result<f64, ImportedPleQuestionJsonError> {
    let points = value
        .parse::<f64>()
        .map_err(|_| ImportedPleQuestionJsonError::InvalidCanonicalPoints)?;
    if !points.is_finite() || points < 0.0 {
        return Err(ImportedPleQuestionJsonError::InvalidCanonicalPoints);
    }
    let normalized = if points == 0.0 { 0.0 } else { points };
    if format!("{normalized:?}") != value {
        return Err(ImportedPleQuestionJsonError::InvalidCanonicalPoints);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{imported_single_choice, imported_single_choice_bytes};
    use question_model::WorkspaceId;
    use uuid::Uuid;

    fn stored_choices() -> Vec<ImportedChoice> {
        imported_single_choice()
            .response
            .choices
            .into_iter()
            .map(|choice| ImportedChoice::new(choice.id, choice.text))
            .collect()
    }

    fn stored_input(points: &str) -> ImportedSingleChoiceInput {
        let stored = imported_single_choice();
        ImportedSingleChoiceInput::new(
            stored.title,
            stored.question_description,
            stored.prompt,
            stored_choices(),
            stored.response.correct_choice,
            points.to_string(),
        )
    }

    fn imported(points: &str) -> ImportedPleQuestionJson {
        ImportedPleQuestionJson::from_imported(stored_input(points))
            .expect("trusted fixture should construct")
    }

    #[test]
    fn imported_source_matches_hand_authored_canonical_and_compiled_parts() {
        let imported = imported("1.0");
        let hand_authored = PleQuestionJsonDocument::parse(&imported_single_choice_bytes())
            .expect("stored authored source");
        assert_eq!(
            imported
                .canonical_bytes()
                .expect("imported canonical bytes"),
            hand_authored
                .canonical_bytes()
                .expect("hand-authored canonical bytes")
        );

        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
        let imported_parts = imported.compile_parts(workspace).expect("imported compile");
        let authored_parts = hand_authored
            .compile(workspace)
            .expect("hand-authored compile")
            .into_parts();
        assert_eq!(imported_parts.0, authored_parts.0);
        assert!(imported_parts.1 == authored_parts.1);
        assert!(imported_parts.2 == authored_parts.2);
    }

    #[test]
    fn canonical_points_require_the_profile_normalized_form() {
        for value in ["1", "01.0", "1.00", "-0.0", "NaN", "inf", "-1.0"] {
            let result = ImportedPleQuestionJson::from_imported(stored_input(value));
            assert_eq!(
                result.err(),
                Some(ImportedPleQuestionJsonError::InvalidCanonicalPoints)
            );
        }
        assert!(ImportedPleQuestionJson::from_imported(stored_input("0.0")).is_ok());
    }

    #[test]
    fn invalid_mapped_ids_choices_and_correct_binding_are_refused() {
        let stored = imported_single_choice();
        let base_choices = stored_choices();
        let mut too_few = base_choices.clone();
        too_few.truncate(1);
        let mut duplicate = base_choices.clone();
        duplicate[1].id = duplicate[0].id.clone();
        let mut invalid_id = base_choices.clone();
        invalid_id[0].id = "not valid".to_string();
        for (choices, correct_choice) in [
            (too_few, stored.response.correct_choice.clone()),
            (duplicate, stored.response.correct_choice.clone()),
            (invalid_id, stored.response.correct_choice.clone()),
            (base_choices, "missing".to_string()),
        ] {
            let result = ImportedPleQuestionJson::from_imported(ImportedSingleChoiceInput::new(
                stored.title.clone(),
                stored.question_description.clone(),
                stored.prompt.clone(),
                choices,
                correct_choice,
                "1.0".to_string(),
            ));
            assert_eq!(
                result.err(),
                Some(ImportedPleQuestionJsonError::InvalidQuestionJson)
            );
        }
    }

    #[test]
    fn blank_or_overlong_mapped_text_is_refused() {
        let stored = imported_single_choice();
        let overlong_prompt = "x".repeat(super::super::MAX_PROMPT_CHARS + 1);
        let overlong_title = "x".repeat(question_model::MAX_QUESTION_TITLE_UNICODE_SCALARS + 1);
        for (title, prompt) in [
            (" ".to_string(), stored.prompt.clone()),
            (stored.title.clone(), " ".to_string()),
            (stored.title.clone(), overlong_prompt),
        ] {
            let result = ImportedPleQuestionJson::from_imported(ImportedSingleChoiceInput::new(
                title,
                stored.question_description.clone(),
                prompt,
                stored_choices(),
                stored.response.correct_choice.clone(),
                "1.0".to_string(),
            ));
            assert_eq!(
                result.err(),
                Some(ImportedPleQuestionJsonError::InvalidQuestionJson)
            );
        }
        for (title, prompt, choice_text) in [
            (
                overlong_title,
                stored.prompt.clone(),
                stored.response.choices[0].text.clone(),
            ),
            (
                stored.title.clone(),
                stored.prompt.clone(),
                "x".repeat(super::super::MAX_CHOICE_TEXT_CHARS + 1),
            ),
        ] {
            let mut choices = stored_choices();
            choices[0].text = choice_text;
            let result = ImportedPleQuestionJson::from_imported(ImportedSingleChoiceInput::new(
                title,
                stored.question_description.clone(),
                prompt,
                choices,
                stored.response.correct_choice.clone(),
                "1.0".to_string(),
            ));
            assert_eq!(
                result.err(),
                Some(ImportedPleQuestionJsonError::InvalidQuestionJson)
            );
        }
    }

    #[test]
    fn aggregate_canonical_source_limit_is_refused() {
        let stored = imported_single_choice();
        let choices = (0..100)
            .map(|index| {
                ImportedChoice::new(
                    format!("choice_{index}"),
                    "x".repeat(super::super::MAX_CHOICE_TEXT_CHARS),
                )
            })
            .collect();
        let result = ImportedPleQuestionJson::from_imported(ImportedSingleChoiceInput::new(
            stored.title,
            stored.question_description,
            stored.prompt,
            choices,
            "choice_0".to_string(),
            "1.0".to_string(),
        ));
        assert_eq!(
            result.err(),
            Some(ImportedPleQuestionJsonError::InvalidQuestionJson)
        );
    }
}
