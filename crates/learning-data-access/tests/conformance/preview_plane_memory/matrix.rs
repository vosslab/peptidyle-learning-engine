//! Negative and group-audience cases for the Memory T3 preview contract.

use super::*;
use learning_data_access::{
    CourseGroupManagementStore, CourseRosterStore, PreviewPlaneStore, RevokeCourseMember,
    SessionLifetime, SessionStore, SessionSubject, TeachingAuthorityReferenceStore,
};
use question_model::{
    AssignmentAudience, AssignmentReference, CourseGroupId, CourseLocalDateTime, IanaTimeZone,
    PreviewDenialReason, PreviewEvaluation, PreviewSelectedMoment, PreviewSyntheticGroupReferences,
    SyntheticPreviewSubjectRequest, UserRole,
};

pub(super) async fn exercise_residual_memory_matrix(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
) {
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("assignment reference");
    let selected = selected_moment();
    let audits = store.preview_subject_audits().expect("audit seam");

    // The active same-course schedule group is a valid G- reference.  Make it
    // the assignment audience, then contrast it with a different active G-.
    let schedule_group_id = CourseGroupId::from_uuid(uuid(99_020));
    let schedule_group = store
        .get_course_group(fixture.context, schedule_group_id)
        .await
        .expect("schedule group")
        .expect("schedule group");
    let accommodation_group_id = CourseGroupId::from_uuid(uuid(99_021));
    let _accommodation_group = store
        .get_course_group(fixture.context, accommodation_group_id)
        .await
        .expect("accommodation group")
        .expect("accommodation group");
    let schedule_reference = store
        .get_course_group_by_id_for_instructor(
            fixture.context,
            fixture.instructor,
            fixture.course,
            schedule_group_id,
        )
        .await
        .expect("schedule reference lookup")
        .expect("schedule reference")
        .reference;
    let accommodation_reference = store
        .get_course_group_by_id_for_instructor(
            fixture.context,
            fixture.instructor,
            fixture.course,
            accommodation_group_id,
        )
        .await
        .expect("accommodation reference lookup")
        .expect("accommodation reference")
        .reference;
    let current = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("assignment");
    let narrowed = store
        .replace_assignment(
            fixture.context,
            ReplaceAssignmentCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: current.revision,
                update: assignment_update(
                    &current.record,
                    AssignmentAudience::any_of_groups(vec![schedule_group.record.id])
                        .expect("audience"),
                ),
            },
        )
        .await
        .expect("narrow assignment audience");
    let narrowed_revision =
        question_model::TeachingOperationRevision::new(narrowed.revision.value())
            .expect("revision");
    let group_request = |groups| SyntheticPreviewSubjectRequest {
        assignment,
        revision: narrowed_revision,
        selected_moment: selected.clone(),
        groups: PreviewSyntheticGroupReferences::try_from(groups).expect("group refs"),
        modifiers: inherit_modifiers(),
    };
    let allowed = store
        .construct_synthetic_preview(
            fixture.context,
            fixture.instructor,
            fixture.course,
            group_request(vec![schedule_reference]),
        )
        .await
        .expect("same-course active group preview");
    assert!(matches!(
        allowed.evaluation,
        PreviewEvaluation::Allowed {
            entitlement: question_model::PreviewEntitlementGrantReason::GroupAudience,
            ..
        }
    ));
    assert!(matches!(
        store
            .construct_synthetic_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                group_request(vec![accommodation_reference])
            )
            .await,
        Ok(learning_data_access::PreviewPlaneResult {
            evaluation: PreviewEvaluation::Denied {
                reason: PreviewDenialReason::NotEntitled
            },
            accommodation: None
        })
    ));
    assert_eq!(
        store.preview_subject_audits().expect("audit seam"),
        audits,
        "synthetic paths never audit"
    );

    // Unknown/foreign-shaped public locators, stale revisions, wrong courses,
    // wrong zones, out-of-term and DST-gap moments all fail before an audit.
    let bogus_group = question_model::CourseGroupReference::new(9_999_999).expect("public ref");
    assert!(matches!(
        store
            .construct_synthetic_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                group_request(vec![bogus_group])
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store
            .construct_synthetic_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                SyntheticPreviewSubjectRequest {
                    assignment,
                    revision: question_model::TeachingOperationRevision::new(
                        narrowed_revision.value() + 1
                    )
                    .expect("stale"),
                    selected_moment: selected.clone(),
                    groups: PreviewSyntheticGroupReferences::try_from(Vec::new()).expect("empty"),
                    modifiers: inherit_modifiers()
                }
            )
            .await,
        Err(StoreError::Conflict)
    ));
    for bad in [
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
            time_zone: IanaTimeZone::parse("America/New_York").expect("zone"),
        },
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-08-01T09:00:00.000").expect("time"),
            time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
        },
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-11-01T01:30:00.000").expect("time"),
            time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
        },
    ] {
        assert!(matches!(
            store
                .construct_synthetic_preview(
                    fixture.context,
                    fixture.instructor,
                    fixture.course,
                    SyntheticPreviewSubjectRequest {
                        assignment,
                        revision: narrowed_revision,
                        selected_moment: bad,
                        groups: PreviewSyntheticGroupReferences::try_from(Vec::new())
                            .expect("empty"),
                        modifiers: inherit_modifiers()
                    }
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    assert_eq!(store.preview_subject_audits().expect("audit seam"), audits);

    // Store-boundary refusals use real public references.  Each case retains
    // the complete Memory fingerprint: only a valid derived construction may
    // append an audit.
    let foreign_course = CourseId::from_uuid(uuid(99_040));
    let foreign_learner = UserId::from_uuid(uuid(99_041));
    let foreign_course_creation_authority = sysadmin_course_creation_authority(
        store,
        fixture.context.tenant_id(),
        foreign_course,
        fixture.instructor,
    )
    .await;
    store
        .create_course(
            fixture.context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: foreign_course,
                    title: "Preview foreign-reference fixture".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("foreign term"),
                },
                authority: foreign_course_creation_authority,
            },
        )
        .await
        .expect("foreign course");
    let foreign_member = store
        .upsert_course_member(
            fixture.context,
            fixture.instructor,
            UpsertCourseMember {
                course: foreign_course,
                user: foreign_learner,
                display_name: "Foreign preview learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("foreign member");
    let foreign_membership_id =
        question_model::CourseMembershipId::from_uuid(foreign_member.member.id.as_uuid());
    let foreign_membership = store
        .course_membership_reference(
            fixture.context,
            fixture.instructor,
            foreign_course,
            foreign_membership_id,
        )
        .await
        .expect("foreign membership reference")
        .expect("foreign membership reference");
    let foreign_group_id = CourseGroupId::from_uuid(uuid(99_042));
    store
        .put_course_group(
            fixture.context,
            PutCourseGroupCommand {
                actor: fixture.instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: foreign_group_id,
                    course: foreign_course,
                    purpose: question_model::CourseGroupPurpose::Section,
                    title: "Foreign preview group".into(),
                    members: vec![foreign_membership_id],
                },
            },
        )
        .await
        .expect("foreign group");
    let foreign_group = store
        .get_course_group_by_id_for_instructor(
            fixture.context,
            fixture.instructor,
            foreign_course,
            foreign_group_id,
        )
        .await
        .expect("foreign group reference")
        .expect("foreign group reference")
        .reference;
    let revoked_user = UserId::from_uuid(uuid(99_043));
    let revoked_member = store
        .upsert_course_member(
            fixture.context,
            fixture.instructor,
            UpsertCourseMember {
                course: fixture.course,
                user: revoked_user,
                display_name: "Revoked preview learner".into(),
                roster_contact: None,
            },
        )
        .await
        .expect("revoked-member setup");
    let revoked_membership_id =
        question_model::CourseMembershipId::from_uuid(revoked_member.member.id.as_uuid());
    let revoked_membership = store
        .course_membership_reference(
            fixture.context,
            fixture.instructor,
            fixture.course,
            revoked_membership_id,
        )
        .await
        .expect("revoked membership reference")
        .expect("revoked membership reference");
    let revoke_session = learning_data_access::SessionTokenHash::compute(b"preview-revoke");
    store
        .create_session(
            revoke_session,
            SessionSubject::new(
                fixture.context.tenant_id(),
                fixture.instructor,
                "Preview fixture instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("revoke session");
    store
        .revoke_course_member(
            fixture.context,
            revoke_session,
            RevokeCourseMember {
                course: fixture.course,
                member: revoked_member.member.id,
                expected_revision: revoked_member.roster_revision,
            },
        )
        .await
        .expect("revoke preview membership");
    let instructor_membership = store
        .get_current_course_membership(fixture.context, fixture.course, fixture.instructor)
        .await
        .expect("instructor membership")
        .expect("active instructor membership");
    let nonstudent_membership = store
        .course_membership_reference(
            fixture.context,
            fixture.instructor,
            fixture.course,
            instructor_membership.id,
        )
        .await
        .expect("instructor membership reference")
        .expect("instructor membership reference");
    for (case, membership) in [
        ("foreign active M-reference", foreign_membership),
        ("revoked M-reference", revoked_membership),
        ("active non-Student M-reference", nonstudent_membership),
    ] {
        let before = store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint");
        assert_eq!(
            store
                .construct_derived_preview(
                    fixture.context,
                    fixture.instructor,
                    fixture.course,
                    question_model::DerivedPreviewSubjectRequest {
                        assignment,
                        revision: narrowed_revision,
                        selected_moment: selected.clone(),
                        membership,
                    },
                )
                .await,
            Err(StoreError::NotFound),
            "{case} is concealed at the derived Store boundary"
        );
        assert!(
            store
                .preview_plane_state_effect_fingerprint()
                .expect("state-effect fingerprint")
                .is_unchanged_from(&before),
            "{case} creates no audit or other Memory effect"
        );
    }
    let before_foreign_group = store
        .preview_plane_state_effect_fingerprint()
        .expect("state-effect fingerprint");
    assert_eq!(
        store
            .construct_synthetic_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                group_request(vec![foreign_group]),
            )
            .await,
        Err(StoreError::NotFound),
        "foreign active G-reference is concealed at the synthetic Store boundary"
    );
    assert!(
        store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint")
            .is_unchanged_from(&before_foreign_group),
        "foreign G-reference creates no audit or other Memory effect"
    );

    // A published group preview has no learner dependency; a derived target
    // is checked exactly at the M- boundary and records only one private audit.
    let page = store
        .list_instructor_preview_schedule(
            fixture.context,
            fixture.instructor,
            fixture.course,
            assignment,
            narrowed_revision,
            PageRequest::first(PageSize::new(20).expect("page size")),
        )
        .await
        .expect("schedule");
    let membership = match &page.rows[0] {
        question_model::InstructorPreviewScheduleRow::Granted { membership, .. }
        | question_model::InstructorPreviewScheduleRow::Denied { membership, .. } => *membership,
    };
    // Derived construction resolves the same authoritative course-local time
    // before its M- boundary.  Malformed/DST moments therefore leave the
    // complete Store state and private audit sequence unchanged.
    for invalid_selected_moment in [
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
            time_zone: IanaTimeZone::parse("America/New_York").expect("zone"),
        },
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-11-01T01:30:00.000").expect("time"),
            time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
        },
    ] {
        let before = store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint");
        assert!(matches!(
            store
                .construct_derived_preview(
                    fixture.context,
                    fixture.instructor,
                    fixture.course,
                    question_model::DerivedPreviewSubjectRequest {
                        assignment,
                        revision: narrowed_revision,
                        selected_moment: invalid_selected_moment,
                        membership,
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
        assert!(
            store
                .preview_plane_state_effect_fingerprint()
                .expect("state-effect fingerprint")
                .is_unchanged_from(&before),
            "derived malformed/DST time refusal creates no audit or other Memory effect"
        );
    }
    let derived = store
        .construct_derived_preview(
            fixture.context,
            fixture.instructor,
            fixture.course,
            question_model::DerivedPreviewSubjectRequest {
                assignment,
                revision: narrowed_revision,
                selected_moment: selected.clone(),
                membership,
            },
        )
        .await;
    assert!(
        matches!(
            derived,
            Ok(learning_data_access::PreviewPlaneResult {
                evaluation: PreviewEvaluation::Denied { .. },
                accommodation: None
            })
        ),
        "audience-excluded M- denies without audit"
    );
    assert_eq!(store.preview_subject_audits().expect("audit seam"), audits);

    // A valid active Student target remains a closed denial while the
    // assignment is Draft, with no derived-read audit or other state effect.
    let current = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("assignment");
    let mut draft_record = current.record.clone();
    draft_record.id = question_model::AssignmentId::from_uuid(uuid(99_044));
    draft_record.title = "Draft preview lifecycle fixture".into();
    draft_record.lifecycle = question_model::AssignmentLifecycle::Draft;
    let draft = store
        .create_assignment_with_default_policy(fixture.context, fixture.instructor, draft_record)
        .await
        .expect("draft assignment");
    let draft_assignment = store
        .assignment_reference(fixture.context, fixture.instructor, draft.record.id)
        .await
        .expect("draft assignment reference")
        .expect("draft assignment reference");
    let draft_revision = question_model::TeachingOperationRevision::new(draft.revision.value())
        .expect("draft revision");
    let before_draft = store
        .preview_plane_state_effect_fingerprint()
        .expect("state-effect fingerprint");
    assert_eq!(
        store
            .construct_derived_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment: draft_assignment,
                    revision: draft_revision,
                    selected_moment: selected.clone(),
                    membership,
                },
            )
            .await,
        Ok(learning_data_access::PreviewPlaneResult {
            evaluation: PreviewEvaluation::Denied {
                reason: PreviewDenialReason::NotEntitled,
            },
            accommodation: None,
        }),
        "Draft lifecycle exposes only the closed denial union"
    );
    assert!(
        store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint")
            .is_unchanged_from(&before_draft),
        "Draft refusal creates no audit or other Memory effect"
    );

    // Reopen course-wide, then verify exact private-audit shape and M4 source.
    let reopened = store
        .replace_assignment(
            fixture.context,
            ReplaceAssignmentCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: current.revision,
                update: assignment_update(&current.record, AssignmentAudience::CourseWide),
            },
        )
        .await
        .expect("course-wide");
    let reopened_revision =
        question_model::TeachingOperationRevision::new(reopened.revision.value())
            .expect("revision");
    let result = store
        .construct_derived_preview(
            fixture.context,
            fixture.instructor,
            fixture.course,
            question_model::DerivedPreviewSubjectRequest {
                assignment,
                revision: reopened_revision,
                selected_moment: selected,
                membership,
            },
        )
        .await
        .expect("derived allowed");
    let comparison = result.accommodation.expect("allowed comparison");
    assert_ne!(
        comparison.before.time_limit_seconds, comparison.after.time_limit_seconds,
        "M4 changes Before/After"
    );
    let after = comparison.after.time_limit_seconds;
    assert_eq!(after.value.expect("M4 limit"), 300);
    assert!(matches!(
        after.source,
        question_model::PreviewPolicySourceLayer::IndividualException
    ));
    let after_audits = store.preview_subject_audits().expect("audit seam");
    assert_eq!(after_audits.len(), audits.len() + 1);
    let audit = after_audits.last().expect("derived audit");
    assert_eq!(audit.actor, fixture.instructor);
    assert_eq!(audit.course, fixture.course);
    assert_eq!(audit.assignment, assignment);
    assert_eq!(audit.action, "preview.subject.derived");
    assert_eq!(audit.schema_version, 1);
    let expected = objects::Sha256Digest::compute(
        format!(
            "previewSubjectDerived:v1:{}:{}:{}",
            audit.actor, audit.course, fixture.assignment
        )
        .as_bytes(),
    );
    assert_eq!(
        audit.payload_sha256, expected,
        "checksum is canonical and contains no target/PII"
    );
    let subject_json = serde_json::to_string(&match result.evaluation {
        PreviewEvaluation::Allowed { subject, .. } => subject,
        PreviewEvaluation::Denied { .. } => panic!("allowed"),
    })
    .expect("serialize");
    for forbidden in [
        "M-",
        "U-",
        "CI-",
        "PV-",
        "Policy learner",
        "Other policy learner",
    ] {
        assert!(
            !subject_json.contains(forbidden),
            "subject leaks {forbidden}"
        );
    }

    // Bad assignment and stale revision remain no-audit after the successful baseline.
    let count = after_audits.len();
    assert!(matches!(
        store
            .construct_derived_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment: AssignmentReference::new(9_999_999).expect("ref"),
                    revision: reopened_revision,
                    selected_moment: selected_moment(),
                    membership
                }
            )
            .await,
        Err(StoreError::NotFound)
    ));
    assert_eq!(
        store.preview_subject_audits().expect("audit seam").len(),
        count
    );
}

fn selected_moment() -> PreviewSelectedMoment {
    PreviewSelectedMoment {
        value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
        time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
    }
}
fn inherit_modifiers() -> SyntheticPreviewModifiers {
    SyntheticPreviewModifiers {
        mode: PolicyModificationModeView::ExtendOnly,
        patch: PolicyPatchView {
            available_at: TeachingTimeFieldPatch::Inherit,
            due_at: TeachingTimeFieldPatch::Inherit,
            closes_at: TeachingTimeFieldPatch::Inherit,
            time_limit_seconds: TeachingLimitFieldPatch::Inherit,
            attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
        },
    }
}
fn assignment_update(record: &AssignmentRecord, audience: AssignmentAudience) -> AssignmentUpdate {
    AssignmentUpdate {
        title: record.title.clone(),
        audience,
        items: record.items.clone(),
        selection_groups: record.selection_groups.clone(),
        disclosure_policy: record.disclosure_policy,
        policies: record.policies,
    }
}
