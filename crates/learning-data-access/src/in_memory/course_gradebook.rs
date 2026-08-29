//! In-memory reference implementation of the isolated course-grade capability.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use domain::course_grade::{CourseGradeAssignment, CourseGradeError, calculate_course_grade};
use question_model::CourseMembershipRole;
use question_model::{
    AssignmentId, CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme, CourseId,
    GradingOperationReference, PointValue,
};

use super::course_roster::require_course_instructor;
use super::{MemoryStore, State, require_course_records_accessible};
use crate::course_gradebook::{
    course_grade_assignment_points, validate_course_grade_scheme_update,
};
#[cfg(feature = "test-support")]
use crate::gradebook_cursor::CalculatedGradebookCursor;
#[cfg(feature = "test-support")]
use crate::{
    AssignmentScoringWitness, CalculatedAssignmentCell, CalculatedAssignmentCellAvailability,
    CalculatedGradebookPage, CalculatedGradebookRequest, CalculatedGradebookResult,
    CalculatedGradebookRow, GradebookFilter, GradebookReloadReason, GradebookSelectionRequest,
    GradebookSelectionResult, SubmittedRunChoicesPage, SubmittedRunChoicesRequest,
};
use crate::{
    CourseGradeAssignmentRecord, CourseGradeExport, CourseGradeExportAudit, CourseGradeExportId,
    CourseGradeExportRow, CourseGradeSchemeRecord, CourseGradeSchemeRevision, CourseGradebookStore,
    CourseGradebookTotalRow, CourseGradebookTotals, GradebookOperationSelection,
    MAX_COURSE_GRADE_EXPORT_ROWS, SessionTokenHash, StoreError, TenantContext,
    UpdateCourseGradeScheme,
};

#[cfg(feature = "test-support")]
#[path = "course_gradebook/selection.rs"]
mod selection;

#[async_trait]
impl CourseGradebookStore for MemoryStore {
    async fn course_grade_scheme(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeSchemeRecord, StoreError> {
        let state = self.read_state()?;
        require_course_records_accessible(&state, context.tenant_id(), course)?;
        require_course_instructor(&state, context, session, course)?;
        Ok(course_grade_scheme(&state, context.tenant_id(), course))
    }

    async fn update_course_grade_scheme(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: UpdateCourseGradeScheme,
    ) -> Result<CourseGradeSchemeRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, command.course)?;
        require_course_instructor(&state, context, session, command.course)?;
        let current = course_grade_scheme(&state, tenant, command.course);
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let current_assignments = state
            .assignments
            .values()
            .filter(|assignment| {
                assignment.tenant == tenant && assignment.course_id == command.course
            })
            .map(|assignment| assignment.id)
            .collect();
        validate_course_grade_scheme_update(&command, &current_assignments)?;
        let mut assignments = command
            .assignments
            .into_iter()
            .map(|membership| {
                let title = state
                    .assignments
                    .get(&(tenant, membership.assignment))
                    .expect("validated current assignment")
                    .title
                    .clone();
                CourseGradeAssignmentRecord {
                    assignment: membership.assignment,
                    title,
                    included: membership.included,
                    category: membership.category,
                    position: membership.position,
                }
            })
            .collect::<Vec<_>>();
        assignments.sort_by_key(|membership| membership.assignment);
        let record = CourseGradeSchemeRecord {
            course: command.course,
            revision: current.revision.next()?,
            scheme: command.scheme,
            assignments,
        };
        state
            .course_grade_schemes
            .insert((tenant, record.course), record.clone());
        Ok(record)
    }

    async fn course_gradebook_totals(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradebookTotals, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, course)?;
        require_course_instructor(&state, context, session, course)?;
        let scheme = course_grade_scheme(&state, tenant, course);
        let rows = course_gradebook_export_rows(&state, tenant, course)?
            .into_iter()
            .map(|row| CourseGradebookTotalRow {
                display_name: row.display_name,
                outcome: row.outcome,
            })
            .collect();
        Ok(CourseGradebookTotals {
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            rows,
        })
    }

    #[cfg(feature = "test-support")]
    async fn calculated_gradebook_page(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        request: CalculatedGradebookRequest,
    ) -> Result<CalculatedGradebookResult, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, course)?;
        require_course_instructor(&state, context, session, course)?;
        let scheme = course_grade_scheme(&state, tenant, course);
        let roster_revision = super::course_roster::roster_policy(&state, tenant, course).revision;
        let after = request
            .page
            .after
            .as_ref()
            .map(CalculatedGradebookCursor::decode)
            .transpose()?;
        if let Some(after) = after {
            if after.scheme_revision != scheme.revision {
                return Ok(CalculatedGradebookResult::ReloadRequired {
                    reason: GradebookReloadReason::SchemeChanged,
                });
            }
            if after.roster_revision != roster_revision {
                return Ok(CalculatedGradebookResult::ReloadRequired {
                    reason: GradebookReloadReason::RosterChanged,
                });
            }
            if after.filter != request.filter {
                return Ok(CalculatedGradebookResult::ReloadRequired {
                    reason: GradebookReloadReason::FilterChanged,
                });
            }
        }

        let mut memberships = active_student_memberships(&state, tenant, course, request.filter)?;
        memberships.sort_by_key(|membership| {
            state
                .course_membership_references
                .get(&(tenant, membership.id))
                .map(|reference| reference.number())
                .unwrap_or_default()
        });
        let after_membership = after.map(|cursor| cursor.last_membership);
        let mut rows = memberships
            .into_iter()
            .filter(|membership| {
                after_membership.is_none_or(|after| {
                    state
                        .course_membership_references
                        .get(&(tenant, membership.id))
                        .is_some_and(|reference| *reference > after)
                })
            })
            .take(usize::from(request.page.size.get()) + 1)
            .collect::<Vec<_>>();
        let has_more = rows.len() > usize::from(request.page.size.get());
        if has_more {
            rows.pop();
        }
        let next_cursor = if has_more {
            let membership = rows.last().expect("a following page has a final row");
            let last_membership = *state
                .course_membership_references
                .get(&(tenant, membership.id))
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "active student membership lacks public reference".to_string(),
                    )
                })?;
            Some(
                CalculatedGradebookCursor {
                    scheme_revision: scheme.revision,
                    roster_revision,
                    filter: request.filter,
                    last_membership,
                }
                .encode(),
            )
        } else {
            None
        };
        let scoring_witnesses =
            calculated_scoring_witnesses(&state, tenant, course, &scheme, request.filter)?;
        let rows = rows
            .into_iter()
            .map(|membership| {
                calculated_gradebook_row(
                    &state,
                    tenant,
                    course,
                    membership,
                    &scheme,
                    request.filter,
                )
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(CalculatedGradebookResult::Page(CalculatedGradebookPage {
            scheme_revision: scheme.revision,
            roster_revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            observation_time: state.authoritative_time,
            scoring_witnesses,
            next_cursor,
            rows,
        }))
    }

    async fn resolve_gradebook_operation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        operation: GradingOperationReference,
    ) -> Result<GradebookOperationSelection, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, course)?;
        require_course_instructor(&state, context, session, course)?;
        let operation = state
            .automated_grading_operations
            .get(&(tenant, operation))
            .copied()
            .filter(|value| value.course == course)
            .ok_or(StoreError::NotFound)?;
        let assignment = state
            .assignment_references
            .get(&(tenant, operation.assignment))
            .copied()
            .ok_or(StoreError::NotFound)?;
        match operation.target {
            crate::GradingOperationTarget::AssignmentScoringGeneration { .. } => {
                Ok(GradebookOperationSelection::Assignment { assignment })
            }
            crate::GradingOperationTarget::SubmissionRecovery { submission } => {
                let attempt = state
                    .automated_grading_executions
                    .iter()
                    .find_map(|((stored_tenant, attempt), execution)| {
                        (*stored_tenant == tenant && execution.submission == submission)
                            .then_some(*attempt)
                    })
                    .ok_or(StoreError::NotFound)?;
                let run = state
                    .attempts
                    .get(&(tenant, attempt))
                    .and_then(|attempt| state.runs.get(&(tenant, attempt.run)))
                    .ok_or(StoreError::NotFound)?;
                let enrollment = state
                    .enrollments
                    .get(&(tenant, run.enrollment))
                    .ok_or(StoreError::NotFound)?;
                let membership = state
                    .course_memberships
                    .values()
                    .find(|membership| {
                        membership.tenant == tenant
                            && membership.course == course
                            && membership.student == Some(enrollment.student)
                            && membership.role == CourseMembershipRole::Student
                            && membership.status == crate::CourseMemberStatus::Active
                    })
                    .and_then(|membership| {
                        state
                            .course_membership_references
                            .get(&(tenant, membership.id))
                    })
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                Ok(GradebookOperationSelection::SingleStudent {
                    membership,
                    assignment,
                })
            }
        }
    }

    #[cfg(feature = "test-support")]
    async fn gradebook_selection(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        request: GradebookSelectionRequest,
    ) -> Result<GradebookSelectionResult, StoreError> {
        selection::gradebook_selection(self, context, session, course, request).await
    }

    #[cfg(feature = "test-support")]
    async fn submitted_run_choices(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        request: SubmittedRunChoicesRequest,
    ) -> Result<SubmittedRunChoicesPage, StoreError> {
        selection::submitted_run_choices(self, context, session, course, request).await
    }

    async fn create_course_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeExport, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, course)?;
        let actor = require_course_instructor(&state, context, session, course)?;
        let rows = course_gradebook_export_rows(&state, tenant, course)?;
        let scheme = course_grade_scheme(&state, tenant, course);
        let id = CourseGradeExportId::generate()?;
        let audit = CourseGradeExportAudit {
            id,
            tenant,
            course,
            requested_by: actor,
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            row_count: rows.len(),
        };
        state.course_grade_export_audits.insert(id, audit);
        Ok(CourseGradeExport { audit, rows })
    }
}

fn default_scheme(
    course: CourseId,
    assignments: Vec<(AssignmentId, String)>,
) -> CourseGradeSchemeRecord {
    CourseGradeSchemeRecord {
        course,
        revision: CourseGradeSchemeRevision::INITIAL,
        scheme: CourseGradeScheme {
            mode: CourseGradeMode::TotalPoints,
            rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero,
            categories: Vec::new(),
            letter_bands: Vec::new(),
        },
        assignments: assignments
            .into_iter()
            .map(|(assignment, title)| CourseGradeAssignmentRecord {
                assignment,
                title,
                included: true,
                category: None,
                position: None,
            })
            .collect(),
    }
}

/// The one canonical initial grade scheme used for both provisioning and
/// defensive reads of legacy state.  New courses always persist this record.
pub(super) fn initial_course_grade_scheme(course: CourseId) -> CourseGradeSchemeRecord {
    default_scheme(course, Vec::new())
}

fn course_grade_scheme(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
) -> CourseGradeSchemeRecord {
    let mut record = state
        .course_grade_schemes
        .get(&(tenant, course))
        .cloned()
        .unwrap_or_else(|| {
            let mut assignments: Vec<_> = state
                .assignments
                .values()
                .filter(|assignment| assignment.tenant == tenant && assignment.course_id == course)
                .map(|assignment| (assignment.id, assignment.title.clone()))
                .collect();
            assignments.sort();
            let mut record = initial_course_grade_scheme(course);
            record.assignments = assignments
                .into_iter()
                .map(|(assignment, title)| CourseGradeAssignmentRecord {
                    assignment,
                    title,
                    included: true,
                    category: None,
                    position: None,
                })
                .collect();
            record
        });
    let current: BTreeMap<_, _> = state
        .assignments
        .values()
        .filter(|assignment| assignment.tenant == tenant && assignment.course_id == course)
        .map(|assignment| (assignment.id, assignment.title.clone()))
        .collect();
    record
        .assignments
        .retain(|membership| current.contains_key(&membership.assignment));
    let configured: BTreeSet<_> = record
        .assignments
        .iter()
        .map(|membership| membership.assignment)
        .collect();
    for (assignment, title) in &current {
        if configured.contains(assignment) {
            continue;
        }
        record.assignments.push(CourseGradeAssignmentRecord {
            assignment: *assignment,
            title: title.clone(),
            included: true,
            category: None,
            position: None,
        });
    }
    for membership in &mut record.assignments {
        membership.title = current
            .get(&membership.assignment)
            .expect("retained current assignment")
            .clone();
    }
    record
        .assignments
        .sort_by_key(|membership| membership.assignment);
    record
}

#[cfg(feature = "test-support")]
fn active_student_memberships(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    filter: GradebookFilter,
) -> Result<Vec<&crate::CourseMembershipRecord>, StoreError> {
    if let GradebookFilter::Assignment(reference) = filter {
        state
            .assignments_by_reference
            .get(&(tenant, reference))
            .filter(|assignment| {
                state
                    .assignments
                    .get(&(tenant, **assignment))
                    .is_some_and(|record| record.course_id == course)
            })
            .ok_or(StoreError::NotFound)?;
    }
    let selected_membership = match filter {
        GradebookFilter::Student(reference) => Some(
            *state
                .course_memberships_by_reference
                .get(&(tenant, reference))
                .filter(|id| {
                    state
                        .course_memberships
                        .get(&(tenant, **id))
                        .is_some_and(|membership| {
                            membership.course == course
                                && membership.status == crate::CourseMemberStatus::Active
                                && membership.role == CourseMembershipRole::Student
                        })
                })
                .ok_or(StoreError::NotFound)?,
        ),
        GradebookFilter::All | GradebookFilter::Assignment(_) => None,
    };
    Ok(state
        .course_memberships
        .values()
        .filter(|membership| {
            membership.tenant == tenant
                && membership.course == course
                && membership.status == crate::CourseMemberStatus::Active
                && membership.role == CourseMembershipRole::Student
                && selected_membership.is_none_or(|id| membership.id == id)
        })
        .collect())
}

#[cfg(feature = "test-support")]
fn calculated_scoring_witnesses(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    scheme: &CourseGradeSchemeRecord,
    filter: GradebookFilter,
) -> Result<Vec<AssignmentScoringWitness>, StoreError> {
    let selected = match filter {
        GradebookFilter::Assignment(reference) => Some(reference),
        GradebookFilter::All | GradebookFilter::Student(_) => None,
    };
    let mut result = Vec::new();
    for configured in &scheme.assignments {
        let assignment = state
            .assignments
            .get(&(tenant, configured.assignment))
            .filter(|assignment| assignment.course_id == course)
            .ok_or(StoreError::NotFound)?;
        let reference = *state
            .assignment_references
            .get(&(tenant, configured.assignment))
            .ok_or_else(|| {
                StoreError::Unavailable("course assignment lacks public reference".to_string())
            })?;
        if selected.is_some_and(|selected| selected != reference) {
            continue;
        }
        let (generation, status) = state
            .assignment_scoring
            .get(&(tenant, assignment.id))
            .copied()
            .ok_or_else(|| {
                StoreError::Unavailable("assignment scoring state is missing".to_string())
            })?;
        result.push(AssignmentScoringWitness {
            assignment: reference,
            generation,
            status,
        });
    }
    Ok(result)
}

#[cfg(feature = "test-support")]
fn calculated_gradebook_row(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    membership: &crate::CourseMembershipRecord,
    scheme: &CourseGradeSchemeRecord,
    filter: GradebookFilter,
) -> Result<CalculatedGradebookRow, StoreError> {
    let student = membership.student.ok_or_else(|| {
        StoreError::Unavailable("active student membership lacks student identity".to_string())
    })?;
    let membership_reference = *state
        .course_membership_references
        .get(&(tenant, membership.id))
        .ok_or_else(|| {
            StoreError::Unavailable("active student membership lacks public reference".to_string())
        })?;
    let profile = state
        .roster_profiles
        .get(&(tenant, course, membership.id))
        .ok_or_else(|| {
            StoreError::Unavailable("active student membership lacks roster profile".to_string())
        })?;
    let assignments = course_grade_assignments(state, tenant, course, student, scheme)?;
    let outcome =
        calculate_course_grade(&scheme.scheme, &assignments).map_err(course_grade_error)?;
    let dropped_assignments = outcome
        .dropped_assignment_ids
        .iter()
        .map(|assignment| {
            state
                .assignment_references
                .get(&(tenant, *assignment))
                .copied()
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "dropped course assignment lacks public reference".to_string(),
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected = match filter {
        GradebookFilter::Assignment(reference) => Some(reference),
        GradebookFilter::All | GradebookFilter::Student(_) => None,
    };
    let assignment_cells = scheme
        .assignments
        .iter()
        .filter_map(|configured| {
            let reference = state
                .assignment_references
                .get(&(tenant, configured.assignment))?;
            selected
                .is_none_or(|selected| selected == *reference)
                .then_some((configured, *reference))
        })
        .map(|(configured, reference)| {
            calculated_assignment_cell(
                state,
                tenant,
                course,
                student,
                configured,
                reference,
                &assignments,
            )
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(CalculatedGradebookRow {
        membership: membership_reference,
        display_label: profile.display_name.clone(),
        outcome,
        dropped_assignments,
        assignment_cells,
    })
}

#[cfg(feature = "test-support")]
fn calculated_assignment_cell(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    student: question_model::StudentId,
    configured: &CourseGradeAssignmentRecord,
    reference: question_model::AssignmentReference,
    grade_assignments: &[CourseGradeAssignment],
) -> Result<CalculatedAssignmentCell, StoreError> {
    let assignment = state
        .assignments
        .get(&(tenant, configured.assignment))
        .filter(|assignment| assignment.course_id == course)
        .ok_or(StoreError::NotFound)?;
    let enrollment = state.enrollments.values().find(|enrollment| {
        enrollment.tenant == tenant
            && enrollment.assignment == configured.assignment
            && enrollment.student == student
    });
    let grade_assignment = grade_assignments
        .iter()
        .find(|item| item.assignment == configured.assignment)
        .ok_or_else(|| StoreError::Unavailable("course grade assignment is missing".to_string()))?;
    let (_, scoring_status) = state
        .assignment_scoring
        .get(&(tenant, configured.assignment))
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("assignment scoring state is missing".to_string())
        })?;
    Ok(CalculatedAssignmentCell {
        assignment: reference,
        title: assignment.title.clone(),
        included: configured.included,
        category: configured.category,
        availability: if enrollment.is_some() {
            CalculatedAssignmentCellAvailability::Available
        } else {
            CalculatedAssignmentCellAvailability::Unavailable
        },
        selected_score: (scoring_status == question_model::ScoringStatus::Current)
            .then_some(grade_assignment.selected_current_score)
            .flatten(),
        scoring_status,
        inspection_choice: selection::inspection_choice(
            state,
            tenant,
            enrollment,
            assignment.policies.grade,
        )?,
    })
}

#[cfg(feature = "test-support")]
fn course_grade_error(error: CourseGradeError) -> StoreError {
    match error {
        CourseGradeError::MissingCategory { .. } | CourseGradeError::UnknownCategory { .. } => {
            StoreError::Unavailable(
                "weighted course grade scheme requires a mapping for each included assignment"
                    .to_string(),
            )
        }
        other => StoreError::InvalidRecord(other.to_string()),
    }
}

/// Advances the scheme token whenever its title-bearing assignment projection
/// changes outside the scheme editor.
pub(super) fn advance_course_grade_scheme_revision(
    state: &mut State,
    tenant: question_model::TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    let mut record = course_grade_scheme(state, tenant, course);
    record.revision = record.revision.next()?;
    state.course_grade_schemes.insert((tenant, course), record);
    Ok(())
}

fn course_gradebook_export_rows(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
) -> Result<Vec<CourseGradeExportRow>, StoreError> {
    let scheme = course_grade_scheme(state, tenant, course);
    let active_students: Vec<_> = state
        .course_memberships
        .values()
        .filter(|membership| {
            membership.tenant == tenant
                && membership.course == course
                && membership.status == crate::CourseMemberStatus::Active
                && membership.role == question_model::CourseMembershipRole::Student
        })
        .collect();
    if active_students.len() > MAX_COURSE_GRADE_EXPORT_ROWS {
        return Err(StoreError::InvalidRecord(
            "course grade export exceeds the row limit".to_string(),
        ));
    }
    let mut rows = Vec::with_capacity(active_students.len());
    for membership in active_students {
        let student = membership.student.ok_or_else(|| {
            StoreError::Unavailable("active student membership lacks student identity".to_string())
        })?;
        let profile = state
            .roster_profiles
            .get(&(tenant, course, membership.id))
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "active student membership lacks roster profile".to_string(),
                )
            })?;
        let assignments = course_grade_assignments(state, tenant, course, student, &scheme)?;
        let outcome =
            calculate_course_grade(&scheme.scheme, &assignments).map_err(|error| match error {
                CourseGradeError::MissingCategory { .. }
                | CourseGradeError::UnknownCategory { .. } => StoreError::Unavailable(
                    "weighted course grade scheme requires a mapping for each included assignment"
                        .to_string(),
                ),
                other => StoreError::InvalidRecord(other.to_string()),
            })?;
        rows.push(CourseGradeExportRow {
            roster_id: membership.roster_id.clone(),
            roster_email: profile.roster_email.clone(),
            display_name: profile.display_name.clone(),
            outcome,
        });
    }
    rows.sort_by(|left, right| {
        left.roster_id
            .cmp(&right.roster_id)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    Ok(rows)
}

fn course_grade_assignments(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    student: question_model::StudentId,
    scheme: &CourseGradeSchemeRecord,
) -> Result<Vec<CourseGradeAssignment>, StoreError> {
    let memberships: BTreeMap<_, _> = scheme
        .assignments
        .iter()
        .map(|membership| (membership.assignment, membership))
        .collect();
    let mut result = Vec::new();
    for assignment in state
        .assignments
        .values()
        .filter(|assignment| assignment.tenant == tenant && assignment.course_id == course)
    {
        let default_membership = CourseGradeAssignmentRecord {
            assignment: assignment.id,
            title: assignment.title.clone(),
            included: true,
            category: None,
            position: None,
        };
        let membership = match memberships.get(&assignment.id) {
            Some(membership) => *membership,
            None if scheme.scheme.mode == CourseGradeMode::TotalPoints => &default_membership,
            None => {
                return Err(StoreError::Unavailable(
                    "weighted course grade scheme requires a mapping for each new assignment"
                        .to_string(),
                ));
            }
        };
        let (selected_current_score, points_possible, scoring_status) = if membership.included {
            let enrollment = state.enrollments.values().find(|enrollment| {
                enrollment.tenant == tenant
                    && enrollment.assignment == assignment.id
                    && enrollment.student == student
            });
            let selected_current_score = enrollment
                .and_then(|enrollment| state.summaries.get(&(tenant, enrollment.id)))
                .and_then(|summary| summary.current_score);
            let (_, scoring_status) = state
                .assignment_scoring
                .get(&(tenant, assignment.id))
                .copied()
                .ok_or_else(|| {
                    StoreError::Unavailable("assignment scoring state is missing".to_string())
                })?;
            (
                selected_current_score,
                course_grade_assignment_points(assignment)?,
                scoring_status,
            )
        } else {
            (
                None,
                PointValue::ZERO,
                question_model::ScoringStatus::Current,
            )
        };
        result.push(CourseGradeAssignment {
            assignment: assignment.id,
            position: membership.position.unwrap_or_default(),
            included: membership.included,
            category: membership.category,
            selected_current_score,
            points_possible,
            scoring_status,
        });
    }
    result.sort_by_key(|assignment| assignment.assignment);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CourseGradeAssignmentMembership;
    use crate::course_gradebook::validate_course_grade_scheme_update_shape;
    use question_model::{GradeCategoryId, GradeCategoryTitle, WeightedGradeCategory};
    use uuid::Uuid;

    fn course() -> CourseId {
        CourseId::from_uuid(Uuid::from_u128(1))
    }

    fn weighted_scheme(drop_lowest: u32) -> CourseGradeScheme {
        CourseGradeScheme {
            mode: CourseGradeMode::WeightedCategories,
            rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero,
            categories: vec![WeightedGradeCategory {
                id: GradeCategoryId::from_uuid(Uuid::from_u128(2)),
                title: GradeCategoryTitle::new("Labs").expect("title"),
                position: 0,
                weight_basis_points: 10_000,
                drop_lowest,
            }],
            letter_bands: Vec::new(),
        }
    }

    #[test]
    fn implicit_default_is_total_points_with_unmapped_included_assignments() {
        let record = default_scheme(
            course(),
            vec![(
                AssignmentId::from_uuid(Uuid::from_u128(4)),
                "Quiz".to_string(),
            )],
        );
        assert_eq!(record.revision, CourseGradeSchemeRevision::INITIAL);
        assert_eq!(record.scheme.mode, CourseGradeMode::TotalPoints);
        assert_eq!(record.assignments[0].category, None);
        assert_eq!(record.assignments[0].position, None);
        assert!(record.assignments[0].included);
        assert_eq!(record.assignments[0].title, "Quiz");
    }

    #[test]
    fn total_points_rejects_every_category_mapping_field() {
        let assignment = AssignmentId::from_uuid(Uuid::from_u128(4));
        let category = GradeCategoryId::from_uuid(Uuid::from_u128(2));
        let command = UpdateCourseGradeScheme {
            course: course(),
            expected_revision: CourseGradeSchemeRevision::INITIAL,
            scheme: CourseGradeScheme {
                mode: CourseGradeMode::TotalPoints,
                rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero,
                categories: Vec::new(),
                letter_bands: Vec::new(),
            },
            assignments: vec![CourseGradeAssignmentMembership {
                assignment,
                included: true,
                category: Some(category),
                position: None,
            }],
        };
        assert!(validate_course_grade_scheme_update_shape(&command).is_err());
    }

    #[test]
    fn weighted_mapping_requires_paired_canonical_positions_and_drop_evidence() {
        let assignment = AssignmentId::from_uuid(Uuid::from_u128(4));
        let category = GradeCategoryId::from_uuid(Uuid::from_u128(2));
        let command = UpdateCourseGradeScheme {
            course: course(),
            expected_revision: CourseGradeSchemeRevision::INITIAL,
            scheme: weighted_scheme(1),
            assignments: vec![CourseGradeAssignmentMembership {
                assignment,
                included: true,
                category: Some(category),
                position: Some(0),
            }],
        };
        assert!(validate_course_grade_scheme_update_shape(&command).is_err());
    }
}
