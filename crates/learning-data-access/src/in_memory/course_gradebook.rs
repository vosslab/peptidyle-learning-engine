//! In-memory reference implementation of the isolated course-grade capability.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use domain::course_grade::{CourseGradeAssignment, CourseGradeError, calculate_course_grade};
use question_model::{
    AssignmentId, CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme, CourseId, PointValue,
};

use super::course_roster::require_course_instructor;
use super::{MemoryStore, State, require_course_records_accessible};
use crate::course_gradebook::{
    course_grade_assignment_points, validate_course_grade_scheme_update,
};
use crate::{
    CourseGradeAssignmentRecord, CourseGradeExport, CourseGradeExportAudit, CourseGradeExportId,
    CourseGradeSchemeRecord, CourseGradeSchemeRevision, CourseGradebookStore,
    CourseGradebookTotalRow, CourseGradebookTotals, MAX_COURSE_GRADE_EXPORT_ROWS, SessionTokenHash,
    StoreError, TenantContext, UpdateCourseGradeScheme,
};

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
        let rows = course_gradebook_totals(&state, tenant, course)?;
        Ok(CourseGradebookTotals {
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            rows,
        })
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
        let rows = course_gradebook_totals(&state, tenant, course)?;
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
            default_scheme(course, assignments)
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

#[cfg(test)]
fn validate_scheme_update(
    state: &State,
    tenant: question_model::TenantId,
    command: &UpdateCourseGradeScheme,
) -> Result<(), StoreError> {
    // Compatibility shim for the focused historical unit tests.
    let _ = (BTreeMap::<u8, u8>::new(), command);
    let actual: BTreeSet<_> = state
        .assignments
        .values()
        .filter(|assignment| assignment.tenant == tenant && assignment.course_id == command.course)
        .map(|assignment| assignment.id)
        .collect();
    validate_course_grade_scheme_update(command, &actual)
}

fn course_gradebook_totals(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
) -> Result<Vec<CourseGradebookTotalRow>, StoreError> {
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
        let roster_id = membership.roster_id.clone().ok_or_else(|| {
            StoreError::Unavailable("active student membership lacks roster identifier".to_string())
        })?;
        let profile = state
            .roster_profiles
            .get(&(tenant, course, membership.id))
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "active student membership lacks roster profile".to_string(),
                )
            })?;
        let roster_email = profile.roster_email.clone().ok_or_else(|| {
            StoreError::Unavailable("active student membership lacks roster email".to_string())
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
        rows.push(CourseGradebookTotalRow {
            roster_id,
            roster_email,
            display_name: profile.display_name.clone(),
            outcome,
        });
    }
    rows.sort_by(|left, right| left.roster_id.cmp(&right.roster_id));
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
        let state = State::default();
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
        assert!(
            validate_scheme_update(
                &state,
                question_model::TenantId::from_uuid(Uuid::from_u128(3)),
                &command
            )
            .is_err()
        );
    }

    #[test]
    fn weighted_mapping_requires_paired_canonical_positions_and_drop_evidence() {
        let assignment = AssignmentId::from_uuid(Uuid::from_u128(4));
        let category = GradeCategoryId::from_uuid(Uuid::from_u128(2));
        let state = State::default();
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
        assert!(
            validate_scheme_update(
                &state,
                question_model::TenantId::from_uuid(Uuid::from_u128(3)),
                &command
            )
            .is_err()
        );
    }
}
