use super::*;

#[test]
fn help_has_no_secret_or_browser_seed_endpoint() {
    assert!(USAGE.contains("e2e-seed"));
    assert!(!USAGE.contains("token"));
    assert!(!USAGE.contains("answer"));
    assert!(!USAGE.contains("SECRET_ACCESS_KEY"));
    assert!(!USAGE.contains("secret-access-key"));
}

#[test]
fn parsing_requires_distinct_course_members() {
    let id = "00000000-0000-0000-0000-000000000001".to_string();
    let result = parse_arguments(&[
        "--database-url".to_string(),
        "postgres://example".to_string(),
        "--tenant".to_string(),
        id.clone(),
        "--instructor".to_string(),
        id.clone(),
        "--student".to_string(),
        id,
    ]);
    assert!(result.is_err());
}

#[test]
fn parsing_accepts_child_only_database_url() {
    let parsed = parse_arguments_with_database_url(
        &[
            "--tenant".to_string(),
            "00000000-0000-0000-0000-000000000001".to_string(),
            "--instructor".to_string(),
            "00000000-0000-0000-0000-000000000002".to_string(),
            "--student".to_string(),
            "00000000-0000-0000-0000-000000000003".to_string(),
            "--apply-migrations".to_string(),
        ],
        Some("postgres://child-only".to_string()),
    )
    .expect("child environment supplies the database URL");
    assert_eq!(parsed.database_url, "postgres://child-only");
}

#[test]
fn deterministic_seed_scaffold_keeps_non_question_records_separate() {
    let first = SeedIds::fresh_for_installation();
    let second = SeedIds::fresh_for_installation();
    assert_eq!(first.assignment, second.assignment);
    assert_ne!(first.problem.as_uuid(), first.version.as_uuid());
    assert_ne!(first.problem, second.problem);
}

#[test]
fn native_seed_matches_catalog_publication_capability_admission() {
    let draft = replica_native_draft(WorkspaceId::from_uuid(Uuid::from_u128(12)));
    let violations = domain::policy::validate_draft_for_publication(
        &draft,
        &native_capabilities().expect("registered native family has capabilities"),
    );

    assert!(
        violations.is_empty(),
        "the host seed must pass the same capability check as catalog publication: {violations:?}"
    );
}

#[test]
fn webwork_pilot_requires_all_host_storage_coordinates_without_secret_arguments() {
    let tenant = "00000000-0000-0000-0000-000000000001".to_string();
    let instructor = "00000000-0000-0000-0000-000000000002".to_string();
    let student = "00000000-0000-0000-0000-000000000003".to_string();
    let result = parse_arguments(&[
        "--database-url".to_string(),
        "postgres://example".to_string(),
        "--tenant".to_string(),
        tenant,
        "--instructor".to_string(),
        instructor,
        "--student".to_string(),
        student,
        "--apply-migrations".to_string(),
        "--webwork-pilot".to_string(),
        "--s3-endpoint".to_string(),
        "http://127.0.0.1:9000".to_string(),
    ]);
    let error = result.expect_err("partial WebWork storage settings must refuse");
    assert!(error.to_string().contains("--private-content-bucket"));
    assert!(!error.to_string().contains("AWS_SECRET_ACCESS_KEY"));
}

#[test]
fn webwork_pilot_storage_settings_are_opt_in_and_deterministic() {
    let parsed = parse_arguments(&[
        "--database-url".to_string(),
        "postgres://example".to_string(),
        "--tenant".to_string(),
        "00000000-0000-0000-0000-000000000001".to_string(),
        "--instructor".to_string(),
        "00000000-0000-0000-0000-000000000002".to_string(),
        "--student".to_string(),
        "00000000-0000-0000-0000-000000000003".to_string(),
        "--apply-migrations".to_string(),
        "--webwork-pilot".to_string(),
        "--s3-endpoint".to_string(),
        "http://127.0.0.1:9000".to_string(),
        "--s3-region".to_string(),
        "us-east-1".to_string(),
        "--private-content-bucket".to_string(),
        "private-content".to_string(),
    ])
    .expect("complete opt-in settings parse");
    let storage = parsed
        .webwork_pilot
        .expect("WebWork pilot settings retained");
    assert_eq!(storage.endpoint_url, "http://127.0.0.1:9000/");
    assert_eq!(storage.region, "us-east-1");
    assert_eq!(storage.private_content_bucket, "private-content");
}

#[test]
fn webwork_catalog_baseline_storage_is_explicit_and_exclusive() {
    let arguments = [
        "--database-url",
        "postgres://example",
        "--tenant",
        "00000000-0000-0000-0000-000000000001",
        "--instructor",
        "00000000-0000-0000-0000-000000000002",
        "--apply-migrations",
        "--webwork-catalog-baseline",
        "--s3-endpoint",
        "http://127.0.0.1:9000",
        "--s3-region",
        "us-east-1",
        "--private-content-bucket",
        "private-content",
    ]
    .map(str::to_string);
    let parsed = parse_arguments(&arguments).expect("catalog baseline arguments parse");
    assert!(parsed.webwork_catalog_baseline.is_some());
    assert!(parsed.webwork_pilot.is_none());
    assert!(parsed.chapter_one_pilot.is_none());
    assert!(parsed.student.is_none());

    let mut conflicting = arguments.to_vec();
    conflicting.push("--webwork-pilot".to_string());
    assert!(parse_arguments(&conflicting).is_err());

    let mut with_student = arguments.to_vec();
    with_student.extend([
        "--student".to_string(),
        "00000000-0000-0000-0000-000000000003".to_string(),
    ]);
    let error = parse_arguments(&with_student)
        .expect_err("catalog baseline must not retain a learner identity");
    assert!(error.to_string().contains("accepts no --student"));

    let mut with_scoring = arguments.to_vec();
    with_scoring.push("--exercise-scoring".to_string());
    let error = parse_arguments(&with_scoring)
        .expect_err("catalog baseline must not request learner scoring");
    assert!(error.to_string().contains("--exercise-scoring requires"));
}
