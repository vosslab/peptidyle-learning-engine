use super::*;

#[test]
fn freshness_refusal_decoder_accepts_only_closed_safe_witnesses() {
    assert!(matches!(
        decode_freshness_refusal(Some(UNCONSUMED_NAMESPACE_FAILURE), None),
        Ok(Some(
            BaseCourseFreshnessRefusal::UnconsumedQuestionNamespace
        ))
    ));
    assert!(matches!(
        decode_freshness_refusal(
            Some(NONEMPTY_RELATION_FAILURE),
            Some("public.authentication_rate_limit")
        ),
        Ok(Some(
            BaseCourseFreshnessRefusal::NonemptyApplicationRelation(_)
        ))
    ));
    for (kind, relation) in [
        (Some("unknown"), None),
        (Some(UNCONSUMED_NAMESPACE_FAILURE), Some("public.course")),
        (Some(NONEMPTY_RELATION_FAILURE), None),
        (
            Some(NONEMPTY_RELATION_FAILURE),
            Some("public.course;select"),
        ),
        (Some(NONEMPTY_RELATION_FAILURE), Some("private.course")),
        (None, Some("public.course")),
    ] {
        assert!(decode_freshness_refusal(kind, relation).is_err());
    }
}

#[test]
fn seed_decoder_accepts_only_closed_non_nil_witnesses() {
    let course = uuid::Uuid::from_u128(1);
    let membership = uuid::Uuid::from_u128(2);
    for (outcome, disposition) in [
        ("created", BaseCourseInstallCourseDisposition::Created),
        (
            "exact_prefix",
            BaseCourseInstallCourseDisposition::ExactPrefix,
        ),
    ] {
        let receipt =
            decode_seed_course_values(outcome, Some(course), Some(membership), None).unwrap();
        assert_eq!(receipt.disposition, disposition);
        assert_eq!(receipt.course_id.as_uuid(), course);
        assert_eq!(receipt.instructor_membership_id.as_uuid(), membership);
    }
    assert!(matches!(
        decode_seed_course_values("refused", None, None, Some(COURSE_AGGREGATE_CONFLICT)),
        Err(StoreError::InvalidRecord(_))
    ));
    for witness in [
        ("created", None, Some(membership), None),
        ("created", Some(uuid::Uuid::nil()), Some(membership), None),
        ("exact_prefix", Some(course), Some(uuid::Uuid::nil()), None),
        (
            "refused",
            Some(course),
            None,
            Some(COURSE_AGGREGATE_CONFLICT),
        ),
        ("refused", None, None, None),
        ("foreign", Some(course), Some(membership), None),
        ("refused", None, None, Some("foreign")),
    ] {
        assert!(matches!(
            decode_seed_course_values(witness.0, witness.1, witness.2, witness.3),
            Err(StoreError::Unavailable(_))
        ));
    }
}

fn expectation() -> BaseCourseCompletionExpectation {
    let uuid = |value| uuid::Uuid::from_u128(value);
    BaseCourseCompletionExpectation::new(
        TenantId::from_uuid(uuid(1)),
        uuid(2),
        "b".repeat(64),
        BaseCourseCompletionCourseExpectation {
            base_course_id: CourseId::from_uuid(uuid(3)),
            practice_course_id: CourseId::from_uuid(uuid(4)),
            base_instructor_membership_id: CourseMembershipId::from_uuid(uuid(5)),
            mary_membership_id: CourseMembershipId::from_uuid(uuid(6)),
            mary_student_id: StudentId::from_uuid(uuid(7)),
            jack_membership_id: CourseMembershipId::from_uuid(uuid(8)),
            jack_student_id: StudentId::from_uuid(uuid(9)),
            practice_instructor_membership_id: CourseMembershipId::from_uuid(uuid(10)),
            avery_membership_id: CourseMembershipId::from_uuid(uuid(11)),
            avery_student_id: StudentId::from_uuid(uuid(12)),
        },
        BaseCourseCompletionContentExpectation {
            question_id: QuestionId::from_canonical_parts("000000", '0').unwrap(),
            problem_id: ProblemId::from_uuid(uuid(13)),
            version_id: VersionId::from_uuid(uuid(14)),
            assignment_id: AssignmentId::from_uuid(uuid(15)),
            assignment_item_id: AssignmentItemId::from_uuid(uuid(16)),
        },
        BaseCourseCompletionEntitlementExpectation {
            mary_enrollment_id: EnrollmentId::from_uuid(uuid(17)),
            jack_enrollment_id: EnrollmentId::from_uuid(uuid(18)),
        },
        BaseCourseCompletionActivityExpectation {
            mary_run_id: RunId::from_uuid(uuid(19)),
            mary_attempt_id: QuestionAttemptId::from_uuid(uuid(20)),
            mary_submission_id: uuid(20),
            jack_run_id: RunId::from_uuid(uuid(21)),
            jack_attempt_id: QuestionAttemptId::from_uuid(uuid(22)),
        },
    )
}

fn value() -> Value {
    let uuid = |value| uuid::Uuid::from_u128(value);
    let hash = "a".repeat(64);
    json!({
        "schemaVersion":1,"baselineVersion":BASELINE_VERSION,"installationGeneration":uuid(2),"tenantId":uuid(1),"recipeSha256":"b".repeat(64),
        "courseGraph":{"baseCourseId":uuid(3),"practiceCourseId":uuid(4),"baseInstructorMembershipId":uuid(5),"maryMembershipId":uuid(6),"maryStudentId":uuid(7),"jackMembershipId":uuid(8),"jackStudentId":uuid(9),"practiceInstructorMembershipId":uuid(10),"averyMembershipId":uuid(11),"averyStudentId":uuid(12),"baseRosterRevision":3,"practiceRosterRevision":2},
        "contentGraph":{"questionId":"000-0000","problemId":uuid(13),"versionId":uuid(14),"assignmentId":uuid(15),"assignmentItemId":uuid(16),"contentSha256":hash,"payloadSha256":hash},
        "entitlementGraph":{"maryEnrollmentId":uuid(17),"jackEnrollmentId":uuid(18),"maryBasisSha256":hash,"jackBasisSha256":hash,"applicableScopeSha256":hash,"marySummarySha256":hash,"jackSummarySha256":hash},
        "activityGraph":{"maryRunId":uuid(19),"maryAttemptId":uuid(20),"marySubmissionId":uuid(20),"jackRunId":uuid(21),"jackAttemptId":uuid(22),"maryRunSha256":hash,"jackRunSha256":hash,"maryAttemptSha256":hash,"jackAttemptSha256":hash,"maryPresentationSha256":hash,"jackPresentationSha256":hash,"maryGradingSha256":hash,"jackGradingSha256":hash,"submissionSha256":hash,"idempotencyRequestSha256":hash,"idempotencyPayloadSha256":hash,"evaluationSha256":hash,"feedbackSha256":hash,"snapshotRunSha256":hash,"snapshotSummarySha256":hash,"snapshotPresentationSha256":hash}
    })
}

fn decode(
    value: Value,
    expected: &BaseCourseCompletionExpectation,
) -> Result<BaseCourseCompletionReceipt, StoreError> {
    let text = serde_json::to_string(&value).unwrap();
    let hash = sha256_hex(text.as_bytes());
    decode_completion_values(None, Some(value), Some(&text), Some(&hash), expected)
}

#[test]
fn completion_decoder_accepts_only_the_exact_typed_graph() {
    let expected = expectation();
    assert!(decode(value(), &expected).is_ok());
    let mut malformed = Vec::new();
    let mut candidate = value();
    candidate["unexpected"] = json!(true);
    malformed.push(candidate);
    let mut candidate = value();
    candidate.as_object_mut().unwrap().remove("courseGraph");
    malformed.push(candidate);
    let mut candidate = value();
    candidate["activityGraph"]["marySubmissionId"] = Value::Null;
    malformed.push(candidate);
    for (pointer, replacement) in [
        ("/tenantId", json!(uuid::Uuid::from_u128(99))),
        ("/installationGeneration", json!(uuid::Uuid::from_u128(99))),
        ("/recipeSha256", json!("c".repeat(64))),
        ("/contentGraph/questionId", json!("0000000")),
        (
            "/courseGraph/baseCourseId",
            json!(uuid::Uuid::from_u128(99)),
        ),
        ("/courseGraph/maryMembershipId", json!(uuid::Uuid::nil())),
        (
            "/activityGraph/jackAttemptId",
            json!(uuid::Uuid::from_u128(99)),
        ),
        ("/activityGraph/maryRunSha256", json!("A".repeat(64))),
        ("/activityGraph/jackRunSha256", json!("short")),
    ] {
        let mut candidate = value();
        *candidate.pointer_mut(pointer).unwrap() = replacement;
        malformed.push(candidate);
    }
    let mut candidate = value();
    candidate["contentGraph"]["payloadSha256"] = json!("c".repeat(64));
    malformed.push(candidate);
    for candidate in malformed {
        assert!(decode(candidate, &expected).is_err());
    }
}

#[test]
fn completion_decoder_rejects_inconsistent_or_untyped_evidence() {
    let expected = expectation();
    let value = value();
    let text = serde_json::to_string(&value).unwrap();
    assert!(
        decode_completion_values(
            None,
            Some(value.clone()),
            Some(&text),
            Some(&"c".repeat(64)),
            &expected
        )
        .is_err()
    );
    assert!(
        decode_completion_values(
            None,
            Some(json!({})),
            Some(&text),
            Some(&sha256_hex(text.as_bytes())),
            &expected
        )
        .is_err()
    );
    assert!(
        decode_completion_values(
            Some(COMPLETION_AGGREGATE_INCOMPLETE),
            None,
            None,
            None,
            &expected
        )
        .is_err()
    );
    assert!(decode_completion_values(Some("unknown"), None, None, None, &expected).is_err());
    assert!(decode_completion_values(None, Some(value.clone()), None, None, &expected).is_err());
    let malformed = "not json";
    assert!(
        decode_completion_values(
            None,
            Some(value.clone()),
            Some(malformed),
            Some(&sha256_hex(malformed.as_bytes())),
            &expected
        )
        .is_err()
    );
    let oversized = " ".repeat(MAX_COMPLETION_RECEIPT_BYTES + 1);
    assert!(
        decode_completion_values(
            None,
            Some(value),
            Some(&oversized),
            Some(&sha256_hex(oversized.as_bytes())),
            &expected
        )
        .is_err()
    );
}
