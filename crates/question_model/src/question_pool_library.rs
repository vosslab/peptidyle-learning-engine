//! Browser-safe read models for reusable published Question Pools.
//!
//! These views expose the Pool owner, current Pool Edit, and exact immutable
//! Question Revision pins for Instructor library and Assessment editing.
//! They contain no Question source, Course, Student, or selection-result facts.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AccountId, BloomClassificationView, BloomCognitiveProcess, BloomKnowledgeDimension,
    PublishedQuestionRevisionTuple, QuestionBackend, QuestionLicense, QuestionPoolEditNumber,
    QuestionPoolId, QuestionPoolMetadataEditNumber, QuestionStatistics, QuestionType,
    ReusableQuestionView,
};

/// Current Pool lineage metadata, independent of immutable membership Revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolMetadata {
    pub title: String,
    pub description: String,
    pub discipline_uuid: Uuid,
    /// Current readable Discipline name for this Pool's existing classification reference.
    pub discipline_name: String,
    /// Whether `discipline_name` is retired. Retired classifications remain readable on existing
    /// references and are not new-selection choices.
    pub discipline_is_retired: bool,
    pub subject_uuid: Uuid,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    pub tags: Vec<String>,
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
}

/// Complete editable metadata replacement for the current Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolMetadataReplacement {
    pub title: String,
    pub description: String,
    #[serde(deserialize_with = "required_nullable")]
    pub topic_uuid: Option<Uuid>,
    #[serde(deserialize_with = "required_nullable")]
    pub subtopic_uuid: Option<Uuid>,
    pub tags: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
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

/// Current editable metadata and concurrency token for one published Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurrentQuestionPoolMetadata {
    pub question_pool_id: QuestionPoolId,
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
    pub title: String,
    pub description: String,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    pub tags: Vec<String>,
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
}

/// Complete current Pool metadata replacement bound to its ordinary metadata CAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveQuestionPoolMetadataRequest {
    pub question_pool_id: QuestionPoolId,
    pub expected_metadata_edit_number: QuestionPoolMetadataEditNumber,
    pub metadata: QuestionPoolMetadataReplacement,
}

/// Receipt from one accepted ordinary Question Pool metadata replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedQuestionPoolMetadata {
    pub question_pool_id: QuestionPoolId,
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
}

/// One reusable published Question Pool available through the Question Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolLibrarySummary {
    pub metadata: QuestionPoolMetadata,
    pub question_pool_id: QuestionPoolId,
    /// Account that created or forked this Pool lineage.
    pub owner_account_id: AccountId,
    /// Immutable Type established by the first Pool member.
    pub question_type: QuestionType,
    /// Immutable Backend established by the first Pool member.
    pub backend: QuestionBackend,
    /// Calculated collection license; each member keeps its own exact Revision license.
    pub license: QuestionLicense,
    /// Membership version used by Assessment and historical membership references.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Current mutable Pool metadata concurrency token.
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
    /// Total members in the current Pool membership.
    pub member_count: NonZeroU32,
    /// Exact Pool-owned Bloom Classification when assigned; member classifications do not substitute.
    pub bloom: Option<BloomClassificationView>,
}

/// One exact Question Revision in the current unordered Pool membership.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolMemberView {
    /// Exact immutable Question Revision tuple.
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Answer-free reusable Question projection for the exact member.
    pub question: ReusableQuestionView,
}

/// Complete contents of one current published Question Pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolView {
    /// Current Pool metadata.
    pub metadata: QuestionPoolMetadata,
    pub question_pool_id: QuestionPoolId,
    /// Account that created or forked this Pool lineage.
    pub owner_account_id: AccountId,
    /// Whether this authenticated viewer may edit the current Pool metadata.
    pub can_edit_metadata: bool,
    /// Immutable Type established by the first Pool member.
    pub question_type: QuestionType,
    /// Immutable Backend established by the first Pool member.
    pub backend: QuestionBackend,
    /// Calculated collection license; each member keeps its own exact Revision license.
    pub license: QuestionLicense,
    /// Current-state concurrency marker; not a historical membership object.
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Exact Pool-owned Bloom Classification when assigned.
    pub bloom: Option<BloomClassificationView>,
    /// Unordered exact Published Question Revision tuple set.
    pub members: Vec<QuestionPoolMemberView>,
    /// Instructor-visible Pool delivery count and outcomes attributed to the
    /// originating Pool recorded when each Question was delivered.
    pub evidence: QuestionStatistics,
}

/// Complete replacement of one ordinary Question Pool's exact member tuple set.
/// ASVS 2.1/2.2: the closed request uses positive typed Edit Numbers and exact typed tuples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveQuestionPoolMembersRequest {
    pub question_pool_id: QuestionPoolId,
    pub expected_question_pool_edit_number: QuestionPoolEditNumber,
    pub members: Vec<PublishedQuestionRevisionTuple>,
}

/// Receipt from an accepted current Question Pool tuple-set save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedQuestionPoolMembers {
    pub question_pool_id: QuestionPoolId,
    pub question_pool_edit_number: QuestionPoolEditNumber,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_replacement_keeps_nullable_bloom_dimensions_under_metadata_cas() {
        let request: SaveQuestionPoolMetadataRequest = serde_json::from_value(serde_json::json!({
            "questionPoolId": "7K3M-19QX",
            "expectedMetadataEditNumber": 5,
            "metadata": {
                "title": "Pool",
                "description": "Description",
                "topicUuid": null,
                "subtopicUuid": null,
                "tags": [],
                "bloomCognitiveProcess": null,
                "bloomKnowledgeDimension": "Conceptual Knowledge"
            }
        }))
        .expect("ordinary Pool metadata replacement");
        assert_eq!(request.expected_metadata_edit_number.get(), 5);
        assert_eq!(request.metadata.bloom_cognitive_process, None);
        assert_eq!(
            request.metadata.bloom_knowledge_dimension,
            Some(BloomKnowledgeDimension::ConceptualKnowledge)
        );

        let mut encoded = serde_json::to_value(request).expect("serialize request");
        encoded["metadata"]
            .as_object_mut()
            .expect("metadata object")
            .remove("bloomCognitiveProcess");
        assert!(serde_json::from_value::<SaveQuestionPoolMetadataRequest>(encoded).is_err());
    }

    #[test]
    fn pool_member_save_request_is_closed_and_keeps_exact_tuple_revisions() {
        let request: SaveQuestionPoolMembersRequest = serde_json::from_value(serde_json::json!({
            "questionPoolId": "3S8B-24DZ",
            "expectedQuestionPoolEditNumber": 7,
            "members": [
                { "publishedQuestionId": "7K3M-79QP", "revisionNumber": 3 },
                { "publishedQuestionId": "2R5X-E7YA", "revisionNumber": 6 }
            ]
        }))
        .expect("closed current Pool tuple-set request");
        assert_eq!(request.expected_question_pool_edit_number.get(), 7);
        assert_eq!(request.members[0].revision_number.get(), 3);
        assert_eq!(request.members[1].revision_number.get(), 6);

        let mut encoded = serde_json::to_value(request).expect("serialize exact tuple request");
        encoded["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<SaveQuestionPoolMembersRequest>(encoded).is_err());
        assert!(
            serde_json::from_value::<SaveQuestionPoolMembersRequest>(serde_json::json!({
                "questionPoolId": "3S8B-24DZ",
                "expectedQuestionPoolEditNumber": 0,
                "members": [
                    { "publishedQuestionId": "7K3M-79QP", "revisionNumber": 3 }
                ]
            }))
            .is_err()
        );
    }
}
