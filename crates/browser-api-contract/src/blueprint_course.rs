//! Browser-safe response projections for immutable Blueprint Revisions.

use question_model::{BlueprintCourseView, BlueprintModuleView, BlueprintRevisionTuple};
use serde::{Deserialize, Serialize};

/// A saved Revision or an exact recorded metadata state, without actor identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BlueprintHistoryEntryView {
    SavedRevision {
        revision: question_model::BlueprintRevision,
        saved_at: question_model::Timestamp,
    },
    MetadataChange {
        classification: question_model::CourseClassification,
        short_name: String,
        long_name: String,
        availability: question_model::BlueprintAvailability,
        recorded_at: question_model::Timestamp,
    },
}

/// One bounded section of Blueprint history; continuation is course/kind scoped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintHistoryPageView {
    pub items: Vec<BlueprintHistoryEntryView>,
    pub next_cursor: Option<String>,
}

/// Answer-free exact membership of a Pool used in a current Blueprint Assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintPoolMembersView {
    pub question_pool_id: question_model::QuestionId,
    pub question_pool_edit_number: question_model::QuestionPoolEditNumber,
    pub members: Vec<question_model::QuestionRevisionTuple>,
}

/// One readable direct fork and its verified owning Instructor display name.
/// No Account identity, hidden-child count, or source-owner exception is exposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintKnownForkView {
    pub id: question_model::BlueprintCourseId,
    pub short_name: String,
    pub long_name: String,
    pub availability: question_model::BlueprintAvailability,
    pub current_revision: question_model::BlueprintRevision,
    /// Source Revision at fork creation, not a last-applied update marker.
    pub source_revision: question_model::BlueprintRevision,
    pub owner_display_name: String,
}

/// On-request current-Revision comparison under ordinary Blueprint visibility.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintComparisonView {
    pub left: BlueprintComparisonSide,
    pub right: BlueprintComparisonSide,
    pub assessment_relationships: Vec<BlueprintComparisonAssessmentRelationship>,
    pub shared_question_ids: Vec<question_model::QuestionId>,
    pub left_only_question_ids: Vec<question_model::QuestionId>,
    pub right_only_question_ids: Vec<question_model::QuestionId>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintComparisonSide {
    pub current_revision: BlueprintRevisionTuple,
    pub names: BlueprintComparisonNames,
    pub blueprint_edit_number: question_model::BlueprintEditNumber,
    pub modules: Vec<BlueprintComparisonModule>,
    pub assessments: Vec<BlueprintComparisonAssessment>,
}

/// Current names, not invented historical Revision metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintComparisonNames {
    pub short_name: String,
    pub long_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintComparisonModule {
    pub blueprint_module_id: question_model::BlueprintModuleId,
    pub label: String,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintComparisonAssessmentRelationship {
    pub left_assessment_id: question_model::BlueprintAssessmentId,
    pub right_assessment_id: question_model::BlueprintAssessmentId,
    pub shared_question_ids: Vec<question_model::QuestionId>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintComparisonAssessment {
    pub blueprint_assessment_id: question_model::BlueprintAssessmentId,
    pub blueprint_module_id: question_model::BlueprintModuleId,
    pub position: usize,
    pub content: question_model::CanonicalBlueprintAssessment,
    pub question_ids: Vec<question_model::QuestionId>,
}

/// Answer-free immutable content resolved by its exact Blueprint Revision Tuple.
///
/// The reference identifies one saved Revision independently of lineage names
/// and availability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintRevisionView {
    pub blueprint_revision: BlueprintRevisionTuple,
    pub modules: Vec<BlueprintModuleView>,
}

/// Result of a changed or canonical no-op Blueprint Course Save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintCourseSaveResponse {
    pub blueprint_course: BlueprintCourseView,
    pub changed: bool,
}

/// Reviewed references and explicit choices, never client-supplied source content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplyRequest {
    pub expected_source: BlueprintRevisionTuple,
    pub expected_fork: BlueprintRevisionTuple,
    pub expected_source_blueprint_edit_number: question_model::BlueprintEditNumber,
    pub expected_fork_blueprint_edit_number: question_model::BlueprintEditNumber,
    pub source_short_name: bool,
    pub source_long_name: bool,
    pub selection: question_model::blueprint_course::BlueprintForkApplySelection,
}

/// Public projection of the Save and metadata committed together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplyResponse {
    pub blueprint_revision: BlueprintRevisionTuple,
    pub changed: bool,
    pub metadata: question_model::BlueprintMetadataState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{BlueprintCourseId, BlueprintRevision};

    #[test]
    fn revision_view_keeps_the_exact_immutable_reference() {
        let view = BlueprintRevisionView {
            blueprint_revision: BlueprintRevisionTuple {
                blueprint_course_id: "BPABCDEFGJ"
                    .parse::<BlueprintCourseId>()
                    .expect("reference"),
                revision: BlueprintRevision::new(3).expect("revision"),
            },
            modules: Vec::new(),
        };

        let wire = serde_json::to_value(view).expect("view serializes");
        assert_eq!(wire["blueprintRevision"]["revision"], "3");
    }
}
