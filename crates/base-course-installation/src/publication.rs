//! Deterministic course, publication, assignment, roster, and entitlement convergence.

use learning_data_access::{
    AssignmentEntitlementMaterialization, CatalogSourceStore, CatalogStore, CourseMemberStatus,
    CourseRecord, CourseRosterMember, CourseRosterStore, CreateAssignmentCommand, DraftRecord,
    MaterializeAssignmentEntitlementCommand, PublishDraftCommand,
    PutAssignmentTeachingSettingsCommand, Store, StoreError, TenantContext, UpsertCourseMember,
};
use question_model::{
    AssignmentEnrollment, CatalogLifecycle, CourseMembershipRole, ProblemVersionRef,
    PublicationScope, QuestionSource,
};

use crate::accounts::{JACK_NAME, MARY_NAME, PRIMARY_INSTRUCTOR_NAME};
use crate::activity::{ActivityRecords, ensure_activity};
use crate::receipt::BaseCourseManifest;
use crate::records::{
    BaseCourseIds, PublicationState, assignment, base_course, base_course_native_draft,
    native_capabilities, practice_course, publication_state,
};
use crate::{AcceptedSubmissionSeedExecutor, BaseCourseInstallError, BaseCourseParticipants};

pub(crate) struct VerifiedCompletion {
    pub(crate) manifest: BaseCourseManifest,
    pub(crate) mary_enrollment: AssignmentEnrollment,
    pub(crate) jack_enrollment: AssignmentEnrollment,
    pub(crate) mary_membership: CourseRosterMember,
    pub(crate) jack_membership: CourseRosterMember,
    pub(crate) avery_membership: CourseRosterMember,
}

pub(crate) async fn converge(
    store: &learning_data_access::postgres::PostgresStore,
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    participants: BaseCourseParticipants,
) -> Result<VerifiedCompletion, BaseCourseInstallError> {
    let context = TenantContext::from_authenticated_session(participants.tenant());
    let ids = BaseCourseIds::for_tenant(participants.tenant());
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let expected_base_course = base_course(participants, ids.base_course)?;
    let expected_practice_course = practice_course(participants, ids.practice_course)?;
    let expected_draft = DraftRecord {
        tenant: participants.tenant(),
        question: base_course_native_draft(ids.workspace),
        derived_from: None,
    };
    let expected_assignment = assignment(participants, ids, reference)?;

    let existing_course = store
        .get_course(context, ids.base_course)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("reading the deterministic Base Course", source)
        })?;
    let existing_draft = store
        .get_draft(context, participants.primary_instructor(), ids.workspace)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "reading the deterministic Base Course draft",
                source,
            )
        })?;
    let existing_publication = store
        .get_catalog_problem(context, reference)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "reading the deterministic Base Course publication",
                source,
            )
        })?;
    let existing_assignment = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "reading the deterministic Base Course assignment",
                source,
            )
        })?;
    let state = publication_state(
        existing_course.is_some(),
        existing_draft.is_some(),
        existing_publication.is_some(),
        existing_assignment.is_some(),
    )?;

    if let Some(actual) = existing_course.as_ref()
        && !course_matches(actual, &expected_base_course)
    {
        return Err(BaseCourseInstallError::baseline(
            "the retained course differs from the deterministic recipe",
        ));
    }
    if let Some(actual) = existing_draft.as_ref()
        && actual.record != expected_draft
    {
        return Err(BaseCourseInstallError::baseline(
            "the retained draft differs from the deterministic recipe",
        ));
    }
    if let Some(actual) = existing_publication.as_ref() {
        verify_publication(
            store,
            context,
            actual,
            participants,
            expected_draft.question.clone(),
        )
        .await?;
    }
    if state == PublicationState::Assignment {
        let actual = existing_assignment
            .as_ref()
            .expect("assignment prefix contains an assignment");
        if actual.record != expected_assignment {
            return Err(BaseCourseInstallError::baseline(
                "the retained assignment differs from the deterministic recipe",
            ));
        }
    }

    verify_installer_courses(
        store,
        participants,
        expected_base_course,
        expected_practice_course,
    )
    .await?;
    let published = ensure_publication(
        store,
        context,
        participants,
        ids,
        expected_draft,
        existing_publication,
    )
    .await?;
    ensure_assignment(store, context, participants, expected_assignment.clone()).await?;
    let (mary_enrollment, mary_membership) = ensure_enrollment(
        store,
        context,
        participants.primary_instructor(),
        participants.mary(),
        ids,
        MARY_NAME,
    )
    .await?;
    let (jack_enrollment, jack_membership) = ensure_enrollment(
        store,
        context,
        participants.primary_instructor(),
        participants.jack(),
        ids,
        JACK_NAME,
    )
    .await?;
    let avery_membership = ensure_student_membership(
        store,
        context,
        participants.sysadmin(),
        ids.practice_course,
        participants.approval_candidate(),
        crate::accounts::APPROVAL_CANDIDATE_NAME,
        "approval candidate",
    )
    .await?;
    verify_participant_membership_matrix(store, context, participants, ids).await?;
    ensure_activity(
        store,
        seed_executor,
        context,
        participants,
        ids,
        ActivityRecords {
            assignment: &expected_assignment,
            question: &published.question,
            mary_enrollment: &mary_enrollment,
            jack_enrollment: &jack_enrollment,
        },
    )
    .await?;

    Ok(VerifiedCompletion {
        manifest: BaseCourseManifest::new(
            ids.assignment,
            mary_enrollment.id,
            published.question_id,
            published.problem,
            published.version,
        ),
        mary_enrollment,
        jack_enrollment,
        mary_membership,
        jack_membership,
        avery_membership,
    })
}

/// Confirms that the closed installer broker created exactly the two recipe courses.
pub(crate) async fn verify_installer_courses(
    store: &learning_data_access::postgres::PostgresStore,
    participants: BaseCourseParticipants,
    expected_base_course: CourseRecord,
    expected_practice_course: CourseRecord,
) -> Result<(), BaseCourseInstallError> {
    let context = TenantContext::from_authenticated_session(participants.tenant());
    for (expected, label) in [
        (expected_base_course, "Base Course"),
        (expected_practice_course, "Genetics Practice Course"),
    ] {
        match store
            .get_course(context, expected.id)
            .await
            .map_err(|source| {
                BaseCourseInstallError::persistence(
                    "rereading a deterministic baseline course",
                    source,
                )
            })? {
            Some(actual) if course_matches(&actual, &expected) => Ok(()),
            Some(_) => Err(BaseCourseInstallError::baseline(format!(
                "the retained {label} differs from the deterministic recipe"
            ))),
            None => Err(BaseCourseInstallError::baseline(format!(
                "the installer did not create the deterministic {label}"
            ))),
        }?;
    }
    Ok(())
}

fn course_matches(actual: &CourseRecord, expected: &CourseRecord) -> bool {
    actual.id == expected.id
        && actual.tenant == expected.tenant
        && actual.title == expected.title
        && actual.term == expected.term
}

async fn ensure_publication(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    participants: BaseCourseParticipants,
    ids: BaseCourseIds,
    expected_draft: DraftRecord,
    existing_publication: Option<learning_data_access::PublishedProblemRecord>,
) -> Result<learning_data_access::PublishedProblemRecord, BaseCourseInstallError> {
    if let Some(existing) = existing_publication {
        verify_publication(
            store,
            context,
            &existing,
            participants,
            expected_draft.question,
        )
        .await?;
        return Ok(existing);
    }

    let saved = match store
        .get_draft(context, participants.primary_instructor(), ids.workspace)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("rereading the Base Course draft", source)
        })? {
        Some(existing) if existing.record == expected_draft => existing,
        Some(_) => {
            return Err(BaseCourseInstallError::baseline(
                "the retained draft differs from the deterministic recipe",
            ));
        }
        None => store
            .upsert_draft(
                context,
                participants.primary_instructor(),
                None,
                expected_draft.clone(),
            )
            .await
            .map_err(|source| {
                BaseCourseInstallError::persistence(
                    "writing the deterministic Base Course draft",
                    source,
                )
            })?,
    };
    if saved.record != expected_draft {
        return Err(BaseCourseInstallError::baseline(
            "the saved draft differs from the deterministic recipe",
        ));
    }
    let capabilities = native_capabilities()?;
    let violations =
        domain::policy::validate_draft_for_publication(&expected_draft.question, &capabilities);
    if !violations.is_empty() {
        return Err(BaseCourseInstallError::baseline(
            "the versioned native question fails publication capability admission",
        ));
    }
    let published = store
        .publish_draft(
            context,
            participants.primary_instructor(),
            PublishDraftCommand {
                expected_draft: expected_draft.clone(),
                expected_revision: saved.revision,
                publication: ProblemVersionRef {
                    problem: ids.problem,
                    version: ids.version,
                },
                published_source: QuestionSource::Native {
                    family: "peptide_bond_geometry".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: participants.primary_instructor(),
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new(PRIMARY_INSTRUCTOR_NAME.to_string())
                        .map_err(|error| {
                            BaseCourseInstallError::baseline(format!(
                                "the versioned Base Course byline is invalid: {error}"
                            ))
                        })?,
                ])
                .map_err(|error| {
                    BaseCourseInstallError::baseline(format!(
                        "the versioned Base Course byline is invalid: {error}"
                    ))
                })?,
                capabilities,
            },
        )
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "publishing the deterministic Base Course question",
                source,
            )
        })?;
    verify_publication(
        store,
        context,
        &published,
        participants,
        expected_draft.question,
    )
    .await?;
    Ok(published)
}

async fn verify_publication(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    record: &learning_data_access::PublishedProblemRecord,
    participants: BaseCourseParticipants,
    draft: question_model::DraftQuestionDefinition,
) -> Result<(), BaseCourseInstallError> {
    let source = QuestionSource::Native {
        family: "peptide_bond_geometry".to_string(),
    };
    let expected = question_model::QuestionDefinition::from_draft(
        draft,
        record.problem,
        record.version,
        source,
    );
    let canonical_question_id: question_model::QuestionId =
        record.question_id.to_string().parse().map_err(|_| {
            BaseCourseInstallError::baseline(
                "the retained publication has a noncanonical Question ID",
            )
        })?;
    if canonical_question_id != record.question_id
        || record.question != expected
        || record.capabilities != native_capabilities()?
        || record.scope != PublicationScope::Institution
        || record.lifecycle != CatalogLifecycle::Published
        || record.author_ids.as_slice() != [participants.primary_instructor()]
        || record.derived_from.is_some()
    {
        return Err(BaseCourseInstallError::baseline(
            "the retained publication differs from the reviewed immutable recipe",
        ));
    }
    let reference = ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    };
    if store
        .catalog_source_artifact(context, reference)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "reading the retained Base Course source binding",
                source,
            )
        })?
        .is_some()
    {
        return Err(BaseCourseInstallError::baseline(
            "the native publication unexpectedly binds a private source artifact",
        ));
    }
    Ok(())
}

async fn ensure_assignment(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    participants: BaseCourseParticipants,
    expected: learning_data_access::AssignmentRecord,
) -> Result<(), BaseCourseInstallError> {
    if expected.lifecycle != question_model::AssignmentLifecycle::Published {
        return Err(BaseCourseInstallError::baseline(
            "the deterministic assignment must converge to Published",
        ));
    }
    let mut draft = expected.clone();
    draft.lifecycle = question_model::AssignmentLifecycle::Draft;
    let stored = store
        .get_assignment_for_edit(context, expected.id)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "rereading the deterministic Base Course assignment",
                source,
            )
        })?;
    let current = match stored {
        Some(actual) if actual.record == expected => return Ok(()),
        Some(actual) if actual.record == draft => actual,
        Some(_) => {
            return Err(BaseCourseInstallError::baseline(
                "the retained assignment differs from the deterministic recipe",
            ));
        }
        None => match store
            .create_assignment(
                context,
                CreateAssignmentCommand {
                    actor: participants.primary_instructor(),
                    assignment: draft.clone(),
                    base_policy: question_model::BaseAssignmentPolicy::default(),
                },
            )
            .await
        {
            Ok(record) => record,
            Err(StoreError::AlreadyExists) => store
                .get_assignment_for_edit(context, expected.id)
                .await
                .map_err(|source| {
                    BaseCourseInstallError::persistence(
                        "reading a concurrently created Base Course assignment",
                        source,
                    )
                })?
                .ok_or_else(|| {
                    BaseCourseInstallError::baseline(
                        "assignment creation conflicted without creating its deterministic ID",
                    )
                })?,
            Err(source) => {
                return Err(BaseCourseInstallError::persistence(
                    "creating the deterministic Base Course assignment",
                    source,
                ));
            }
        },
    };
    if current.record == expected {
        return Ok(());
    }
    if current.record != draft {
        return Err(BaseCourseInstallError::baseline(
            "the created assignment differs from the deterministic draft recipe",
        ));
    }
    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: participants.primary_instructor(),
                course: expected.course_id,
                assignment: expected.id,
                expected_revision: current.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: expected.instructions.clone(),
                    base_policy: question_model::BaseAssignmentPolicy::default(),
                },
            },
        )
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "publishing the deterministic Base Course assignment",
                source,
            )
        })?;
    let published = store
        .get_assignment_for_edit(context, expected.id)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "reloading the published Base Course assignment",
                source,
            )
        })?
        .ok_or_else(|| BaseCourseInstallError::baseline("the published assignment disappeared"))?;
    if published.record != expected {
        return Err(BaseCourseInstallError::baseline(
            "the published assignment differs from the deterministic recipe",
        ));
    }
    Ok(())
}

async fn ensure_enrollment(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: question_model::UserId,
    student: question_model::UserId,
    ids: BaseCourseIds,
    display_name: &str,
) -> Result<(AssignmentEnrollment, CourseRosterMember), BaseCourseInstallError> {
    let membership = ensure_student_membership(
        store,
        context,
        instructor,
        ids.base_course,
        student,
        display_name,
        "Base Course learner",
    )
    .await?;
    let command = MaterializeAssignmentEntitlementCommand::for_instructor_action(
        student,
        ids.base_course,
        ids.assignment,
        instructor,
        question_model::EntitlementPurpose::InstructorIssue,
    )
    .map_err(|source| {
        BaseCourseInstallError::persistence(
            "forming the Base Course learner entitlement request",
            source,
        )
    })?;
    match store
        .issue_assignment_entitlement(context, command)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "materializing a Base Course learner entitlement",
                source,
            )
        })? {
        AssignmentEntitlementMaterialization::Granted(materialized) => {
            Ok((materialized.enrollment, membership))
        }
        AssignmentEntitlementMaterialization::Denied(_) => Err(BaseCourseInstallError::baseline(
            "a seeded learner is not currently entitled to the Base Course assignment",
        )),
    }
}

async fn ensure_student_membership(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    actor: question_model::UserId,
    course: question_model::CourseId,
    student: question_model::UserId,
    display_name: &str,
    label: &'static str,
) -> Result<CourseRosterMember, BaseCourseInstallError> {
    let membership = store
        .upsert_course_member(
            context,
            actor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: display_name.to_string(),
                roster_contact: None,
            },
        )
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "establishing a baseline Student in the canonical roster",
                source,
            )
        })?;
    if membership.tenant != context.tenant_id()
        || membership.course != course
        || membership.member.tenant != context.tenant_id()
        || membership.member.course != course
        || membership.member.user != student
        || membership.member.display_name != display_name
        || membership.member.status != CourseMemberStatus::Active
    {
        return Err(BaseCourseInstallError::baseline(format!(
            "the {label} membership differs from the deterministic recipe"
        )));
    }
    Ok(membership.member)
}

async fn verify_participant_membership_matrix(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    participants: BaseCourseParticipants,
    ids: BaseCourseIds,
) -> Result<(), BaseCourseInstallError> {
    let required = [
        (
            ids.base_course,
            participants.primary_instructor(),
            CourseMembershipRole::Instructor,
            "primary Instructor in the Base Course",
        ),
        (
            ids.base_course,
            participants.mary(),
            CourseMembershipRole::Student,
            "Mary in the Base Course",
        ),
        (
            ids.base_course,
            participants.jack(),
            CourseMembershipRole::Student,
            "Jack in the Base Course",
        ),
        (
            ids.practice_course,
            participants.sysadmin(),
            CourseMembershipRole::Instructor,
            "Sysadmin in the Genetics Practice Course",
        ),
        (
            ids.practice_course,
            participants.approval_candidate(),
            CourseMembershipRole::Student,
            "approval candidate in the Genetics Practice Course",
        ),
    ];
    for (course, user, role, label) in required {
        let membership = store
            .get_current_course_membership(context, course, user)
            .await
            .map_err(|source| {
                BaseCourseInstallError::persistence(
                    "verifying a deterministic baseline membership",
                    source,
                )
            })?;
        let Some(membership) = membership else {
            return Err(BaseCourseInstallError::baseline(format!(
                "the required {label} membership is absent"
            )));
        };
        if membership.tenant != participants.tenant()
            || membership.course != course
            || membership.user != user
            || membership.role != role
            || membership.status != CourseMemberStatus::Active
        {
            return Err(BaseCourseInstallError::baseline(format!(
                "the {label} membership differs from the deterministic recipe"
            )));
        }
    }

    let absent = [
        (
            ids.base_course,
            participants.approval_candidate(),
            "approval candidate in the Base Course",
        ),
        (
            ids.base_course,
            participants.sysadmin(),
            "Sysadmin in the Base Course",
        ),
        (
            ids.practice_course,
            participants.primary_instructor(),
            "primary Instructor in the Genetics Practice Course",
        ),
        (
            ids.practice_course,
            participants.mary(),
            "Mary in the Genetics Practice Course",
        ),
        (
            ids.practice_course,
            participants.jack(),
            "Jack in the Genetics Practice Course",
        ),
    ];
    for (course, user, label) in absent {
        if store
            .get_current_course_membership(context, course, user)
            .await
            .map_err(|source| {
                BaseCourseInstallError::persistence(
                    "verifying a deterministic absent baseline membership",
                    source,
                )
            })?
            .is_some()
        {
            return Err(BaseCourseInstallError::baseline(format!(
                "the recipe unexpectedly contains {label}"
            )));
        }
    }
    Ok(())
}
