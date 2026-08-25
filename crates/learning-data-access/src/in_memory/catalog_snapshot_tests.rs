use super::state::TestStatisticsContributionScope;
use super::*;

fn seed_catalog(store: &MemoryStore, records: impl IntoIterator<Item = PublishedProblemRecord>) {
    let mut state = store.write_state().expect("catalog fixture state");
    for record in records {
        let sequence = state.next_catalog_publication_sequence;
        state.next_catalog_publication_sequence += 1;
        state
            .catalog_publication_sequences
            .insert((record.problem, record.version), sequence);
        state
            .published
            .insert((record.problem, record.version), record);
    }
}

#[tokio::test]
async fn catalog_search_discovers_broad_terms_and_intent_order_without_backend_score_parity() {
    let store = MemoryStore::default();
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(81_000)));
    let mut focused = catalog_search_tests::record(81_001);
    focused.question.metadata.title = "Peptide binding reaction".to_string();
    let mut broad = catalog_search_tests::record(81_002);
    broad.question.metadata.title = "Peptide reaction overview".to_string();
    seed_catalog(&store, [focused.clone(), broad.clone()]);

    let page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                text: Some("peptide binding".to_string()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("broad catalog discovery");

    assert_eq!(
        page.items
            .first()
            .map(|item| item.summary.question_id.clone()),
        Some(focused.question_id.clone())
    );
    assert!(
        page.items
            .iter()
            .any(|item| item.summary.question_id == broad.question_id)
    );
}

#[tokio::test]
async fn catalog_search_admits_a_deliberate_word_typo() {
    let store = MemoryStore::default();
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(82_000)));
    let record = catalog_search_tests::record(82_001);
    seed_catalog(&store, [record.clone()]);

    let page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                text: Some("peptde".to_string()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("typo catalog discovery");

    assert_eq!(
        page.items
            .first()
            .map(|item| item.summary.question_id.clone()),
        Some(record.question_id.clone())
    );
}

#[tokio::test]
async fn catalog_search_continuation_is_query_bound_and_has_no_equal_score_duplicates() {
    let store = MemoryStore::default();
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(83_000)));
    let first = catalog_search_tests::record(83_001);
    let second = catalog_search_tests::record(83_002);
    seed_catalog(&store, [first.clone(), second.clone()]);
    let query = CatalogSearchQuery {
        text: Some("peptide".to_string()),
        page_size: Some(1),
        ..CatalogSearchQuery::default()
    };
    let initial = store
        .search_catalog_as_instructor(context, query.clone())
        .await
        .expect("first catalog page");
    let cursor = initial
        .next_cursor
        .clone()
        .expect("equal-score continuation");

    let continuation = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                cursor: Some(cursor.clone()),
                ..query.clone()
            },
        )
        .await
        .expect("second catalog page");
    assert_ne!(
        initial.items[0].summary.question_id,
        continuation.items[0].summary.question_id
    );
    assert!(matches!(
        store
            .search_catalog_as_instructor(
                context,
                CatalogSearchQuery {
                    text: Some("different".to_string()),
                    cursor: Some(cursor.clone()),
                    ..query.clone()
                }
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let mut tampered = cursor.into_bytes();
    tampered[0] = if tampered[0] == b'A' { b'B' } else { b'A' };
    assert!(matches!(
        store
            .search_catalog_as_instructor(
                context,
                CatalogSearchQuery {
                    cursor: Some(String::from_utf8(tampered).expect("cursor remains text")),
                    ..query
                }
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn catalog_search_continuation_excludes_later_publication_and_keeps_complete_facets() {
    let store = MemoryStore::default();
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(84_000)));
    let first = catalog_search_tests::record(84_001);
    let second = catalog_search_tests::record(84_002);
    seed_catalog(&store, [first.clone(), second.clone()]);
    let query = CatalogSearchQuery {
        text: Some("peptide".to_string()),
        page_size: Some(1),
        ..CatalogSearchQuery::default()
    };
    let initial = store
        .search_catalog_as_instructor(context, query.clone())
        .await
        .expect("first snapshot page");
    let later = catalog_search_tests::record(84_003);
    seed_catalog(&store, [later.clone()]);

    let continuation = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                cursor: initial
                    .next_cursor
                    .map(|cursor| cursor.as_str().to_string()),
                ..query
            },
        )
        .await
        .expect("snapshot continuation");
    assert!(
        !continuation
            .items
            .iter()
            .any(|item| item.summary.question_id == later.question_id)
    );
    assert_eq!(continuation.facets.evidence.unavailable, 2);
}

#[tokio::test]
async fn catalog_search_continuation_preserves_first_statistics_disclosure_boundary() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(85_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let first = catalog_search_tests::record(85_001);
    let later_disclosure = catalog_search_tests::record(85_002);
    seed_catalog(&store, [first, later_disclosure.clone()]);
    let query = CatalogSearchQuery {
        text: Some("peptide".to_string()),
        page_size: Some(1),
        ..CatalogSearchQuery::default()
    };
    let initial = store
        .search_catalog_as_instructor(context, query.clone())
        .await
        .expect("first snapshot page");
    let reference = ProblemVersionRef {
        problem: later_disclosure.problem,
        version: later_disclosure.version,
    };
    for offset in 0..5_u128 {
        store
            .record_question_statistics_contribution_for_scope(
                TestStatisticsContributionScope::for_course(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(85_100 + offset)),
                    CourseId::from_uuid(Uuid::from_u128(85_400 + offset % 2)),
                    RunId::from_uuid(Uuid::from_u128(85_200 + offset)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(85_300 + offset)),
                ),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5)).expect("observation"),
            )
            .expect("statistics contribution");
    }

    let continuation = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                cursor: initial
                    .next_cursor
                    .map(|cursor| cursor.as_str().to_string()),
                ..query.clone()
            },
        )
        .await
        .expect("snapshot continuation");
    assert_eq!(continuation.facets.evidence.available, 0);
    let fresh = store
        .search_catalog_as_instructor(context, query)
        .await
        .expect("fresh catalog page");
    assert_eq!(fresh.facets.evidence.available, 1);
}

#[tokio::test]
async fn catalog_discovery_evidence_requires_distinct_courses_and_breaks_relevance_ties() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(86_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let without_evidence = catalog_search_tests::record(86_001);
    let with_evidence = catalog_search_tests::record(86_002);
    let reference = ProblemVersionRef {
        problem: with_evidence.problem,
        version: with_evidence.version,
    };
    seed_catalog(&store, [without_evidence.clone(), with_evidence.clone()]);
    for offset in 0..5_u128 {
        store
            .record_question_statistics_contribution_for_scope(
                TestStatisticsContributionScope::for_course(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(86_100 + offset)),
                    CourseId::from_uuid(Uuid::from_u128(86_400)),
                    RunId::from_uuid(Uuid::from_u128(86_200 + offset)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(86_300 + offset)),
                ),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                    .expect("valid observation"),
            )
            .expect("single-course contribution");
    }
    let single_course = store
        .search_catalog_as_instructor(context, CatalogSearchQuery::default())
        .await
        .expect("single-course discovery");
    assert!(single_course.items.iter().all(|item| matches!(
        item.evidence,
        CatalogDiscoveryEvidence::InsufficientEvidence
    )));

    store
        .record_question_statistics_contribution_for_scope(
            TestStatisticsContributionScope::for_course(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(86_106)),
                CourseId::from_uuid(Uuid::from_u128(86_401)),
                RunId::from_uuid(Uuid::from_u128(86_206)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(86_306)),
            ),
            reference,
            CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                .expect("valid cross-course observation"),
        )
        .expect("cross-course contribution");
    let page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                text: Some("peptide".to_string()),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("evidence-ranked discovery");
    assert_eq!(page.items[0].summary.question_id, with_evidence.question_id);
    assert!(matches!(
        page.items[0].evidence,
        CatalogDiscoveryEvidence::Available {
            observed_course_count: 2,
            independent_learner_observation_count: 6,
            ..
        }
    ));
}

#[test]
fn discovery_evidence_counts_one_independent_learner_across_enrollments() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(87_000));
    let other_tenant = TenantId::from_uuid(Uuid::from_u128(87_001));
    let publication = catalog_search_tests::record(87_002);
    let reference = ProblemVersionRef {
        problem: publication.problem,
        version: publication.version,
    };
    let learner = StudentId::from_uuid(Uuid::from_u128(87_003));
    let observation =
        CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5)).expect("valid observation");
    for (enrollment, course) in [(87_010, 87_020), (87_011, 87_021)] {
        store
            .record_question_statistics_contribution_for_scope(
                TestStatisticsContributionScope::for_course_and_learner(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(enrollment)),
                    CourseId::from_uuid(Uuid::from_u128(course)),
                    learner,
                    RunId::from_uuid(Uuid::from_u128(enrollment + 20)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(enrollment + 40)),
                ),
                reference,
                observation,
            )
            .expect("per-enrollment replay receipt");
    }
    let state = store.read_state().expect("test state");
    assert_eq!(state.question_statistics_receipts.len(), 2);
    assert_eq!(
        state.question_statistics[&(reference.problem, reference.version)].cohort_size(),
        1
    );
    assert_eq!(
        state.catalog_evidence_courses[&(reference.problem, reference.version)].len(),
        1
    );
    drop(state);
    store
        .record_question_statistics_contribution_for_scope(
            TestStatisticsContributionScope::for_course_and_learner(
                other_tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(87_012)),
                CourseId::from_uuid(Uuid::from_u128(87_022)),
                learner,
                RunId::from_uuid(Uuid::from_u128(87_032)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(87_052)),
            ),
            reference,
            observation,
        )
        .expect("tenant-scoped learner is independent");
    assert_eq!(
        store.read_state().expect("test state").question_statistics
            [&(reference.problem, reference.version)]
            .cohort_size(),
        2
    );
}
