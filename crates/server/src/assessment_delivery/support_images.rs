//! Image bindings from already projected Student support fields.

use learning_data_access::StudentAssessmentAttemptHistoryQuestion;
use question_model::{QuestionContentBlock, QuestionHtmlImageBinding};

/// Only returned support content contributes authorization. Prompt and response
/// assets retain their existing issuance boundary; hidden source fields never
/// enter this function. ASVS 8.2.3 and 8.3.1.
pub(super) fn disclosed_images(
    question: &StudentAssessmentAttemptHistoryQuestion,
) -> Vec<QuestionHtmlImageBinding> {
    bindings([
        question.hints.as_deref(),
        question.worked_solution.as_deref(),
        question.feedback.choice_feedback.as_deref(),
        question.feedback.correct_feedback.as_deref(),
        question.feedback.incorrect_feedback.as_deref(),
        question.feedback.general_feedback.as_deref(),
        question.feedback.question_answer.as_deref(),
        question.feedback.question_answer_explanation.as_deref(),
    ])
}

fn bindings<'a>(
    fields: impl IntoIterator<Item = Option<&'a [QuestionContentBlock]>>,
) -> Vec<QuestionHtmlImageBinding> {
    let mut images = Vec::new();
    for block in fields.into_iter().flatten().flatten() {
        if let QuestionContentBlock::Html { images: bound, .. } = block {
            for image in bound {
                if !images.contains(image) {
                    images.push(image.clone());
                }
            }
        }
    }
    images
}

#[cfg(test)]
mod tests {
    use domain::student_feedback_release::{
        StudentFeedbackReleaseDecision, project_disclosed_support, project_student_feedback,
    };
    use question_model::{
        PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionFeedback,
        QuestionImageAssetId, QuestionImageAssetTuple, QuestionRevisionNumber, StudentFeedback,
    };

    use super::*;

    #[test]
    fn image_authorization_follows_actual_support_disclosure() {
        let hint = html_image("hint.png", 1);
        let solution = html_image("solution.png", 2);
        let feedback = html_image("selected-feedback.png", 3);
        let mut decision = StudentFeedbackReleaseDecision {
            score: false,
            per_item_correctness: false,
            submitted_response: false,
            question_answer: false,
            question_answer_explanation: false,
            class_statistics: false,
            hints: false,
            worked_solutions: false,
        };
        let hidden = project_disclosed_support(decision, Some(&hint), Some(&solution));
        let mut question = StudentAssessmentAttemptHistoryQuestion {
            position: 1,
            published_question_revision_tuple: PublishedQuestionRevisionTuple {
                published_question_id: PublishedQuestionId::from_random_identifier("1234567")
                    .expect("valid identifier"),
                revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
            },
            response_state: learning_data_access::LiveAssessmentPreviousAttemptState::Submitted,
            response: Some(html_image("recorded-response.png", 4)),
            presentation: None,
            backend_review: None,
            backend_answer_review: None,
            feedback: StudentFeedback::empty(),
            hints: hidden.hints,
            worked_solution: hidden.worked_solutions,
        };
        assert!(disclosed_images(&question).is_empty());

        decision.hints = true;
        let support = project_disclosed_support(decision, Some(&hint), Some(&solution));
        let teaching = project_student_feedback(
            decision,
            None,
            &QuestionFeedback {
                choice_feedback: Some(feedback.clone()),
                ..Default::default()
            },
            None,
            None,
        )
        .expect("selected feedback is disclosed");
        question.hints = support.hints;
        question.worked_solution = support.worked_solutions;
        question.feedback = teaching;
        question.feedback.general_feedback = Some(hint.clone());
        let retained = disclosed_images(&question);
        assert_eq!(
            retained
                .iter()
                .map(|binding| binding.source_relative_name.as_str())
                .collect::<Vec<_>>(),
            ["hint.png", "selected-feedback.png"]
        );
    }

    fn html_image(name: &str, id: u128) -> Vec<QuestionContentBlock> {
        vec![QuestionContentBlock::Html {
            html: format!("<img src=\"{name}\" alt=\"Instruction\">"),
            images: vec![QuestionHtmlImageBinding {
                source_relative_name: name.to_owned(),
                question_image_asset_tuple: QuestionImageAssetTuple {
                    question_image_asset_id: QuestionImageAssetId::from_uuid(
                        uuid::Uuid::from_u128(id),
                    ),
                    checksum: "a".repeat(64),
                },
            }],
        }]
    }
}
