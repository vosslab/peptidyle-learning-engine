//! Global identity validation and classification predicates for Library search.
use axum::{http::StatusCode, response::Response};
use learning_data_access::{
    ContentClassificationStore, ContentDisciplineDiscoveryStore, SessionTokenHash,
};
use question_model::{PublishedQuestionSharedMetadata, QuestionSearchRequest};

use super::{route_error, store_error_response};

/// ASVS 2.2.1/2.2.2/2.2.3: validate real global identities and their
/// associations after authentication, including the selected cross-mode parent.
// The route returns this response immediately; boxing it would add an allocation
// and require the route boundary to unwrap solely to preserve Axum's `Response`.
#[allow(clippy::result_large_err)]
pub(super) async fn validate(
    store: &(impl ContentClassificationStore + ContentDisciplineDiscoveryStore),
    token: SessionTokenHash,
    query: &QuestionSearchRequest,
) -> Result<(), Response> {
    let Some(discipline) = query.discipline_uuid else {
        return Ok(());
    };
    let invalid = || {
        route_error(
            StatusCode::BAD_REQUEST,
            "Question Library classification is invalid",
        )
    };
    if !store
        .list_disciplines_including_retired(token)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == discipline)
    {
        return Err(invalid());
    }
    let Some(subject) = query.subject_uuid else {
        return Ok(());
    };
    if !store
        .list_subjects(token, discipline)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == subject)
    {
        return Err(invalid());
    }
    let Some(topic) = query.topic_uuid else {
        return Ok(());
    };
    if !store
        .list_topics(token, subject)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == topic)
    {
        return Err(invalid());
    }
    let Some(subtopic) = query.subtopic_uuid else {
        return Ok(());
    };
    if !store
        .list_subtopics(token, topic)
        .await
        .map_err(store_error_response)?
        .iter()
        .any(|item| item.uuid == subtopic)
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn matches(
    metadata: &PublishedQuestionSharedMetadata,
    query: &QuestionSearchRequest,
) -> bool {
    (query.cross_discipline
        || query
            .discipline_uuid
            .is_none_or(|id| id == metadata.discipline_uuid))
        && query
            .subject_uuid
            .is_none_or(|id| id == metadata.subject_uuid)
        && query
            .topic_uuid
            .is_none_or(|id| Some(id) == metadata.topic_uuid)
        && query
            .subtopic_uuid
            .is_none_or(|id| Some(id) == metadata.subtopic_uuid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use learning_data_access::StoreError;
    use question_model::PublishedQuestionId;
    use uuid::Uuid;

    struct Vocabulary {
        unavailable: bool,
    }

    fn test_question_id(identifier: &str) -> PublishedQuestionId {
        PublishedQuestionId::from_random_identifier(identifier).expect("canonical question ID")
    }

    impl Vocabulary {
        fn items(
            &self,
            expected_parent: u128,
            parent: Uuid,
            id: u128,
        ) -> Result<Vec<learning_data_access::ContentClassificationItem>, StoreError> {
            if self.unavailable {
                return Err(StoreError::Unavailable("vocabulary unavailable".into()));
            }
            Ok(if parent == Uuid::from_u128(expected_parent) {
                vec![learning_data_access::ContentClassificationItem {
                    uuid: Uuid::from_u128(id),
                    name: "Current name".into(),
                }]
            } else {
                Vec::new()
            })
        }
    }

    #[async_trait::async_trait]
    impl ContentClassificationStore for Vocabulary {
        async fn list_disciplines(
            &self,
            _: SessionTokenHash,
        ) -> Result<Vec<learning_data_access::ContentClassificationItem>, StoreError> {
            if self.unavailable {
                return Err(StoreError::Unavailable("vocabulary unavailable".into()));
            }
            // The active-choice projection cannot resolve the retired query
            // exercised below; discovery must use the all-status capability.
            Ok(Vec::new())
        }
        async fn list_subjects(
            &self,
            _: SessionTokenHash,
            parent: Uuid,
        ) -> Result<Vec<learning_data_access::ContentClassificationItem>, StoreError> {
            self.items(1, parent, 2)
        }
        async fn list_topics(
            &self,
            _: SessionTokenHash,
            parent: Uuid,
        ) -> Result<Vec<learning_data_access::ContentClassificationItem>, StoreError> {
            self.items(2, parent, 3)
        }
        async fn list_subtopics(
            &self,
            _: SessionTokenHash,
            parent: Uuid,
        ) -> Result<Vec<learning_data_access::ContentClassificationItem>, StoreError> {
            self.items(3, parent, 4)
        }
    }

    #[async_trait::async_trait]
    impl learning_data_access::ContentDisciplineDiscoveryStore for Vocabulary {
        async fn list_disciplines_including_retired(
            &self,
            _: SessionTokenHash,
        ) -> Result<Vec<learning_data_access::ContentDiscipline>, StoreError> {
            if self.unavailable {
                return Err(StoreError::Unavailable("vocabulary unavailable".into()));
            }
            Ok(vec![learning_data_access::ContentDiscipline {
                uuid: Uuid::from_u128(1),
                name: "Current name".into(),
                is_retired: true,
            }])
        }
    }

    #[tokio::test]
    async fn selected_hierarchy_validation_distinguishes_bad_identity_from_unavailable_store() {
        let token = SessionTokenHash::compute(b"instructor");
        let query = QuestionSearchRequest {
            discipline_uuid: Some(Uuid::from_u128(1)),
            subject_uuid: Some(Uuid::from_u128(2)),
            topic_uuid: Some(Uuid::from_u128(3)),
            subtopic_uuid: Some(Uuid::from_u128(4)),
            cross_discipline: true,
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("valid structure");
        assert!(
            validate(&Vocabulary { unavailable: false }, token, &query)
                .await
                .is_ok()
        );
        for field in 0..4 {
            let mut wrong = query.clone();
            match field {
                0 => wrong.discipline_uuid = Some(Uuid::from_u128(10)),
                1 => wrong.subject_uuid = Some(Uuid::from_u128(20)),
                2 => wrong.topic_uuid = Some(Uuid::from_u128(30)),
                _ => wrong.subtopic_uuid = Some(Uuid::from_u128(40)),
            }
            assert_eq!(
                validate(&Vocabulary { unavailable: false }, token, &wrong)
                    .await
                    .expect_err("wrong hierarchy")
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            validate(&Vocabulary { unavailable: true }, token, &query)
                .await
                .expect_err("store unavailable")
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn cross_mode_relaxes_only_discipline_and_keeps_global_identity_restrictions() {
        let metadata = PublishedQuestionSharedMetadata {
            question_id: test_question_id("0000000"),
            metadata_edit_number: 1,
            tags: Vec::new(),
            discipline_uuid: Uuid::from_u128(1),
            subject_uuid: Uuid::from_u128(2),
            topic_uuid: Some(Uuid::from_u128(3)),
            subtopic_uuid: Some(Uuid::from_u128(4)),
        };
        let mut query = QuestionSearchRequest {
            discipline_uuid: Some(Uuid::from_u128(10)),
            subject_uuid: Some(metadata.subject_uuid),
            topic_uuid: metadata.topic_uuid,
            subtopic_uuid: metadata.subtopic_uuid,
            ..QuestionSearchRequest::default()
        };
        assert!(!matches(&metadata, &query));
        query.cross_discipline = true;
        assert!(matches(&metadata, &query));
        for field in 0..3 {
            let mut changed = query.clone();
            match field {
                0 => changed.subject_uuid = Some(Uuid::from_u128(20)),
                1 => changed.topic_uuid = Some(Uuid::from_u128(30)),
                _ => changed.subtopic_uuid = Some(Uuid::from_u128(40)),
            }
            assert!(!matches(&metadata, &changed));
        }
    }
}
