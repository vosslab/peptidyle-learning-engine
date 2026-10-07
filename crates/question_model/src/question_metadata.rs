//! Ordinary in-place metadata corrections for the current Published Question Revision.

use serde::{Deserialize, Serialize};

use crate::{
    BloomCognitiveProcess, BloomKnowledgeDimension, PublishedQuestionRevisionTuple, QuestionType,
    Tag,
};

/// Complete editable metadata replacement for one current Published Question Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionMetadataReplacement {
    /// Human-facing Question Title.
    pub question_title: String,
    /// Human-facing Question Description.
    pub question_description: String,
    /// Question Type; WeBWorK metadata may correct this without a content Revision.
    pub question_type: QuestionType,
    /// Complete replacement Tag set; an empty set clears Tags.
    pub tags: Vec<Tag>,
    /// Required Discipline classification.
    pub discipline_uuid: uuid::Uuid,
    /// Required Subject classification.
    pub subject_uuid: uuid::Uuid,
    /// Optional Topic classification.
    #[serde(deserialize_with = "required_nullable")]
    pub topic_uuid: Option<uuid::Uuid>,
    /// Optional Subtopic classification.
    #[serde(deserialize_with = "required_nullable")]
    pub subtopic_uuid: Option<uuid::Uuid>,
    /// Optional Bloom Cognitive Process; `None` clears the stored value.
    #[serde(deserialize_with = "required_nullable")]
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    /// Optional Bloom Knowledge Dimension; `None` clears the stored value.
    #[serde(deserialize_with = "required_nullable")]
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
}

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloom_dimensions_are_independent_and_null_is_an_explicit_replacement() {
        let metadata: QuestionMetadataReplacement = serde_json::from_value(serde_json::json!({
            "questionTitle": "Question",
            "questionDescription": "Description",
            "questionType": "multipleChoice",
            "tags": [],
            "disciplineUuid": "00000000-0000-0000-0000-000000000001",
            "subjectUuid": "00000000-0000-0000-0000-000000000002",
            "topicUuid": null,
            "subtopicUuid": null,
            "bloomCognitiveProcess": "Analyze",
            "bloomKnowledgeDimension": null
        }))
        .expect("one Bloom dimension may be absent");
        assert_eq!(
            metadata.bloom_cognitive_process,
            Some(BloomCognitiveProcess::Analyze)
        );
        assert_eq!(metadata.bloom_knowledge_dimension, None);

        let mut incomplete = serde_json::to_value(metadata).expect("serialize replacement");
        incomplete
            .as_object_mut()
            .expect("object")
            .remove("bloomKnowledgeDimension");
        assert!(serde_json::from_value::<QuestionMetadataReplacement>(incomplete).is_err());
    }
}

/// One ordinary metadata correction bound to the exact current Question Revision and edit counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveQuestionMetadataRequest {
    /// The exact current Published Question Revision selected for editing.
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Current metadata counter shown when the edit began.
    pub expected_metadata_edit_number: u64,
    /// Complete replacement of the fields editable through the ordinary detail editor.
    pub metadata: QuestionMetadataReplacement,
}

/// Receipt from one accepted ordinary Question metadata correction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedQuestionMetadata {
    /// Exact Published Question Revision whose metadata was changed.
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Metadata counter after the accepted replacement.
    pub metadata_edit_number: u64,
}
