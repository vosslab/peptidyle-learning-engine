//! Backend-neutral activity, scoring, and course policy.

use crate::score_precision;
use crate::{
    ActivityTransition, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssignmentRecord,
    COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH, CourseGroupRecord, CourseRecord, StoreError,
    TenantContext, current_attempt_points,
};
use domain::completion::{RequiredQuestionState, derive_within_run_completion};
use domain::run::RunModelError;
use domain::scoring::RunTransition;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord};
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentItemId, AssignmentRunItem, AttemptResult,
    GradePolicy, QuestionAttempt, QuestionAttemptId, RunCompletionStatus, RunId,
    StudentAssignmentSummary, TenantId,
};

pub(crate) fn grade_policy(assignment: &AssignmentRecord) -> GradePolicy {
    assignment.policies.grade
}

/// Maps a storage activity write to the pure domain transition.
pub(crate) fn summary_transition(transition: &ActivityTransition) -> RunTransition {
    match transition {
        ActivityTransition::StartRun { run } => RunTransition::Started { at: run.started_at },
        ActivityTransition::RecordQuestionAttempt { attempt } => {
            RunTransition::QuestionAttemptRecorded {
                at: attempt
                    .timer
                    .submitted_at
                    .unwrap_or(attempt.timer.issued_at),
            }
        }
        ActivityTransition::CompleteRun { score, at, .. } => RunTransition::Completed {
            score: *score,
            at: *at,
        },
    }
}

/// Refuses a tenant-owned record outside the authenticated context.
pub(crate) fn ensure_tenant(
    context: TenantContext,
    record_tenant: TenantId,
) -> Result<(), StoreError> {
    if context.tenant_id() == record_tenant {
        Ok(())
    } else {
        Err(StoreError::TenantMismatch)
    }
}

/// Validates a course record before either backend persists it.
pub(crate) fn validate_course(course: &CourseRecord) -> Result<(), StoreError> {
    validate_title("course", &course.title)?;
    Ok(())
}

/// Validates one current course group independently of backend authority.
pub(crate) fn validate_course_group(group: &CourseGroupRecord) -> Result<(), StoreError> {
    validate_title("course group", &group.title)?;
    let unique_members: std::collections::BTreeSet<_> = group.members.iter().copied().collect();
    if unique_members.len() != group.members.len() {
        return Err(StoreError::InvalidRecord(
            "course group members must be unique".to_string(),
        ));
    }
    Ok(())
}

/// Validates assignment fields independent of catalog visibility.
pub(crate) fn validate_assignment(assignment: &AssignmentRecord) -> Result<(), StoreError> {
    validate_title("assignment", &assignment.title)?;
    if assignment.items.is_empty() && assignment.selection_groups.is_empty() {
        return Err(StoreError::InvalidRecord(
            "assignment must reference at least one published problem version".to_string(),
        ));
    }
    let mut item_ids = std::collections::BTreeSet::new();
    let mut positions = std::collections::BTreeSet::new();
    for item in &assignment.items {
        if !item_ids.insert(item.id) {
            return Err(StoreError::InvalidRecord(
                "assignment item identities must be unique".to_string(),
            ));
        }
        if !positions.insert(item.position) {
            return Err(StoreError::InvalidRecord(
                "assignment positions must be unique".to_string(),
            ));
        }
        if item.delivery_state == question_model::AssignmentDeliveryState::Retired
            && item.scoring_mode != question_model::AssignmentScoringMode::Excluded
        {
            return Err(StoreError::InvalidRecord(
                "retired assignment items must be excluded from current scoring".to_string(),
            ));
        }
    }
    let mut group_ids = std::collections::BTreeSet::new();
    for group in &assignment.selection_groups {
        if !group_ids.insert(group.id) || !positions.insert(group.position) {
            return Err(StoreError::InvalidRecord(
                "assignment selection identities and positions must be unique".to_string(),
            ));
        }
        let active_candidates = group
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.delivery_state == question_model::AssignmentDeliveryState::Active
            })
            .count();
        if group.draw_count == 0
            || usize::try_from(group.draw_count)
                .map_or(true, |draw_count| draw_count > active_candidates)
            || group.algorithm_version == 0
        {
            return Err(StoreError::InvalidRecord(
                "selection groups need a positive bounded draw and algorithm version".to_string(),
            ));
        }
        let mut candidate_positions = std::collections::BTreeSet::new();
        for candidate in &group.candidates {
            if !item_ids.insert(candidate.id) {
                return Err(StoreError::InvalidRecord(
                    "assignment item and candidate identities must be unique".to_string(),
                ));
            }
            if !candidate_positions.insert(candidate.position) {
                return Err(StoreError::InvalidRecord(
                    "selection candidate positions must be unique within a group".to_string(),
                ));
            }
        }
        if candidate_positions
            .iter()
            .copied()
            .ne(0..u32::try_from(candidate_positions.len()).map_err(|_| {
                StoreError::InvalidRecord("too many selection candidates".to_string())
            })?)
        {
            return Err(StoreError::InvalidRecord(
                "selection candidate positions must be contiguous from zero".to_string(),
            ));
        }
    }
    if positions
        .iter()
        .copied()
        .ne(0..u32::try_from(positions.len())
            .map_err(|_| StoreError::InvalidRecord("too many assignment positions".to_string()))?)
    {
        return Err(StoreError::InvalidRecord(
            "assignment positions must be contiguous from zero".to_string(),
        ));
    }
    if let question_model::CompletionRequirement::ScoreAtLeast { fraction } =
        assignment.policies.completion
        && (!fraction.is_finite() || !(0.0..=1.0).contains(&fraction))
    {
        return Err(StoreError::InvalidRecord(
            "score-at-least completion fraction must be finite and between 0 and 1".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use objects::Sha256Digest;
    use question_model::{
        AssetId, CourseBannerId, CourseId, ObjectId, ProblemId, TenantId, VersionId, WorkspaceId,
        WorkspaceImportId,
    };
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn source_record(key: ObjectKey, category: ObjectCategory) -> ObjectRecord {
        ObjectRecord {
            id: key.object_id(),
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(b"private publication candidate"),
            size_bytes: 29,
            media_type: "image/png".to_string(),
            category,
            version: None,
            license: "CC0-1.0".to_string(),
            provenance: "authoring asset".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        }
    }

    fn pending_catalog_delivery(source: ObjectRecord) -> AssetDeliveryRecord {
        let problem = ProblemId::from_uuid(id(1));
        let version = VersionId::from_uuid(id(2));
        let asset = AssetId::from_uuid(id(3));
        let target_key = ObjectKey::ProblemAsset {
            problem,
            version,
            asset,
            object: ObjectId::from_uuid(id(4)),
        };
        AssetDeliveryRecord {
            id: AssetDeliveryId::from_asset(asset),
            object: ObjectRecord {
                id: target_key.object_id(),
                bucket: target_key.bucket(),
                key: target_key,
                sha256: source.sha256,
                size_bytes: source.size_bytes,
                media_type: source.media_type.clone(),
                category: ObjectCategory::Asset,
                version: Some(version),
                license: "CC0-1.0".to_string(),
                provenance: "published asset".to_string(),
                created_at: source.created_at,
            },
            intrinsic_width: None,
            intrinsic_height: None,
            scope: AssetDeliveryScope::Catalog {
                asset,
                reference: question_model::ProblemVersionRef { problem, version },
            },
            publication: crate::AssetPublication::Pending,
            pending_source: Some(source),
        }
    }

    #[test]
    fn pending_public_asset_accepts_only_typed_private_workspace_assets() {
        let tenant = TenantId::from_uuid(id(10));
        let workspace = WorkspaceId::from_uuid(id(11));
        let asset = AssetId::from_uuid(id(12));
        let imported = source_record(
            ObjectKey::WorkspaceAsset {
                tenant,
                workspace,
                import: WorkspaceImportId::from_uuid(id(13)),
                asset,
                object: ObjectId::from_uuid(id(14)),
            },
            ObjectCategory::Asset,
        );
        assert!(validate_asset_delivery(&pending_catalog_delivery(imported)).is_ok());

        let native = source_record(
            ObjectKey::WorkspaceQuestionAsset {
                tenant,
                workspace,
                asset,
                object: ObjectId::from_uuid(id(15)),
            },
            ObjectCategory::Asset,
        );
        assert!(validate_asset_delivery(&pending_catalog_delivery(native)).is_ok());
    }

    #[test]
    fn pending_public_asset_rejects_student_and_temporary_sources() {
        let tenant = TenantId::from_uuid(id(20));
        for source in [
            source_record(
                ObjectKey::StudentRecord {
                    tenant,
                    object: ObjectId::from_uuid(id(21)),
                },
                ObjectCategory::Export,
            ),
            source_record(
                ObjectKey::Temporary {
                    object: ObjectId::from_uuid(id(22)),
                },
                ObjectCategory::Temporary,
            ),
        ] {
            assert!(matches!(
                validate_asset_delivery(&pending_catalog_delivery(source)),
                Err(StoreError::InvalidRecord(message))
                    if message.contains("private workspace asset source")
            ));
        }
    }

    #[test]
    fn pending_public_asset_rejects_private_non_authoring_and_public_sources() {
        let tenant = TenantId::from_uuid(id(30));
        let workspace = WorkspaceId::from_uuid(id(31));
        let problem = ProblemId::from_uuid(id(32));
        let version = VersionId::from_uuid(id(33));
        let asset = AssetId::from_uuid(id(34));
        for source in [
            source_record(
                ObjectKey::CourseBanner {
                    tenant,
                    course: CourseId::from_uuid(id(35)),
                    banner: CourseBannerId::from_uuid(id(36)),
                },
                ObjectCategory::CourseContent,
            ),
            source_record(
                ObjectKey::RestrictedProblemAsset {
                    problem,
                    version,
                    asset,
                    object: ObjectId::from_uuid(id(37)),
                },
                ObjectCategory::Asset,
            ),
            source_record(
                ObjectKey::ProblemAsset {
                    problem,
                    version,
                    asset,
                    object: ObjectId::from_uuid(id(38)),
                },
                ObjectCategory::Asset,
            ),
            source_record(
                ObjectKey::WorkspaceQuestionAsset {
                    tenant,
                    workspace,
                    asset,
                    object: ObjectId::from_uuid(id(39)),
                },
                ObjectCategory::Asset,
            ),
        ] {
            let mut source = source;
            if matches!(source.key, ObjectKey::WorkspaceQuestionAsset { .. }) {
                source.version = Some(version);
            }
            assert!(matches!(
                validate_asset_delivery(&pending_catalog_delivery(source)),
                Err(StoreError::InvalidRecord(message))
                    if message.contains("private workspace asset source")
            ));
        }
    }
}

/// Validates that delivery metadata agrees with the typed immutable object key.
pub(crate) fn validate_asset_delivery(record: &AssetDeliveryRecord) -> Result<(), StoreError> {
    if record.object.id != record.object.key.object_id()
        || record.object.bucket != record.object.key.bucket()
        || record.object.category != record.object.key.category()
        || record.object.version != record.object.key.version_id()
    {
        return Err(StoreError::InvalidRecord(
            "object metadata must agree with its typed key".to_string(),
        ));
    }
    if record.object.media_type.trim().is_empty()
        || record.object.license.trim().is_empty()
        || record.object.provenance.trim().is_empty()
    {
        return Err(StoreError::InvalidRecord(
            "object media type, license, and provenance must not be empty".to_string(),
        ));
    }
    if record.intrinsic_width.is_some() != record.intrinsic_height.is_some()
        || record.intrinsic_width == Some(0)
        || record.intrinsic_height == Some(0)
    {
        return Err(StoreError::InvalidRecord(
            "asset intrinsic dimensions must be a nonzero width and height pair".to_string(),
        ));
    }
    match (record.publication, &record.pending_source) {
        (crate::AssetPublication::Ready, None) => {}
        (crate::AssetPublication::Ready, Some(_)) => {
            return Err(StoreError::InvalidRecord(
                "ready asset delivery must not retain a publication source".to_string(),
            ));
        }
        (crate::AssetPublication::Pending, Some(source)) => {
            if !matches!(record.scope, AssetDeliveryScope::Catalog { .. })
                || record.object.bucket != objects::Bucket::PublicAssets
                || !is_private_publication_candidate(source)
            {
                return Err(StoreError::InvalidRecord(
                    "pending publication requires an exact private workspace asset source"
                        .to_string(),
                ));
            }
        }
        (crate::AssetPublication::Pending, None) => {
            return Err(StoreError::InvalidRecord(
                "pending publication requires its private immutable source".to_string(),
            ));
        }
    }
    match (&record.scope, &record.object.key) {
        (
            AssetDeliveryScope::Catalog { asset, reference },
            ObjectKey::ProblemAsset {
                problem,
                version,
                asset: key_asset,
                object: _,
            }
            | ObjectKey::RestrictedProblemAsset {
                problem,
                version,
                asset: key_asset,
                object: _,
            },
        ) if record.id == AssetDeliveryId::from_asset(*asset)
            && *asset == *key_asset
            && reference.problem == *problem
            && reference.version == *version
            && record.object.bucket == record.object.key.bucket()
            && record.object.category == ObjectCategory::Asset => {}
        (
            AssetDeliveryScope::StudentRecord {
                tenant,
                course: _,
                authorized_users,
            },
            ObjectKey::StudentRecord {
                tenant: key_tenant,
                object,
            },
        ) if record.id == AssetDeliveryId::from_object(*object)
            && *tenant == *key_tenant
            && record.object.bucket == Bucket::StudentRecords
            && record.object.category == ObjectCategory::Export =>
        {
            if authorized_users.is_empty() {
                return Err(StoreError::InvalidRecord(
                    "student-record delivery must authorize at least one user".to_string(),
                ));
            }
            let unique: std::collections::BTreeSet<_> = authorized_users.iter().copied().collect();
            if unique.len() != authorized_users.len() {
                return Err(StoreError::InvalidRecord(
                    "student-record authorized users must be unique".to_string(),
                ));
            }
        }
        (
            AssetDeliveryScope::CourseBanner {
                tenant,
                course,
                banner,
            },
            ObjectKey::CourseBanner {
                tenant: key_tenant,
                course: key_course,
                banner: key_banner,
            },
        ) if record.id == AssetDeliveryId::from_course_banner(*banner)
            && *tenant == *key_tenant
            && *course == *key_course
            && *banner == *key_banner
            && record.object.bucket == Bucket::PrivateContent
            && record.object.category == ObjectCategory::CourseContent
            && record.object.media_type == "image/webp"
            && record.intrinsic_width == Some(COURSE_BANNER_WIDTH)
            && record.intrinsic_height == Some(COURSE_BANNER_HEIGHT) => {}
        _ => {
            return Err(StoreError::InvalidRecord(
                "only matching catalog, student-record, and current course-banner objects may be delivered"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// Returns whether an object can be copied by the committed public-asset publisher.
///
/// The pending target is a public catalog object, but its source remains a
/// private authoring object until the catalog transaction and outbox commit.
/// Only the two typed workspace-asset key families carry the tenant, workspace,
/// logical asset, and physical-object provenance required for that bridge:
/// imported QTI assets and native workspace-question assets.  In particular,
/// a private bucket alone is not authority to publish: restricted catalog,
/// course, learner-record, and temporary objects have different owners and
/// must never become a public CDN source.
fn is_private_publication_candidate(source: &ObjectRecord) -> bool {
    if source.id != source.key.object_id()
        || source.bucket != Bucket::PrivateContent
        || source.bucket != source.key.bucket()
        || source.category != ObjectCategory::Asset
        || source.category != source.key.category()
        || source.version.is_some()
        || source.version != source.key.version_id()
        || source.media_type.trim().is_empty()
        || source.license.trim().is_empty()
        || source.provenance.trim().is_empty()
    {
        return false;
    }

    matches!(
        &source.key,
        ObjectKey::WorkspaceAsset { object, .. }
            | ObjectKey::WorkspaceQuestionAsset { object, .. }
            if *object == source.id
    )
}

/// One current submitted result resolved against the current assignment scoring definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CurrentRunQuestion {
    pub(crate) assignment_item: AssignmentItemId,
    pub(crate) result: AttemptResult,
    pub(crate) earned_points: f64,
    pub(crate) possible_points: f64,
}

/// Resolves the latest submitted attempt for every immutable delivered position.
pub(crate) fn current_run_questions(
    assignment: &AssignmentRecord,
    run_items: &[AssignmentRunItem],
    attempts: &[QuestionAttempt],
    current: &QuestionAttempt,
) -> Result<Vec<Option<CurrentRunQuestion>>, StoreError> {
    let mut delivered = run_items.iter().collect::<Vec<_>>();
    delivered.sort_by_key(|item| item.issued_position);
    for (position, item) in delivered.iter().enumerate() {
        if item.run != current.run || usize::try_from(item.issued_position).ok() != Some(position) {
            return Err(StoreError::InvalidRecord(
                "immutable run items must have contiguous issued positions".to_string(),
            ));
        }
    }
    let mut latest: Vec<Option<(ActivityTimestamp, QuestionAttemptId, CurrentRunQuestion)>> =
        vec![None; delivered.len()];
    for attempt in attempts
        .iter()
        .filter(|attempt| attempt.id != current.id)
        .chain(std::iter::once(current))
    {
        if attempt.run != current.run {
            return Err(StoreError::InvalidRecord(
                "attempt does not belong to the completed run".to_string(),
            ));
        }
        let position = usize::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("attempt position is outside the delivered run".to_string())
        })?;
        let item = delivered.get(position).ok_or_else(|| {
            StoreError::InvalidRecord("attempt position is outside the delivered run".to_string())
        })?;
        if attempt.problem != item.reference.problem
            || attempt.question_version != item.reference.version
        {
            return Err(StoreError::InvalidRecord(
                "attempt identity disagrees with its immutable run item".to_string(),
            ));
        }
        let (Some(submitted_at), Some(result)) = (attempt.timer.submitted_at, attempt.result)
        else {
            continue;
        };
        let (earned_points, possible_points) =
            current_attempt_points(assignment, item.assignment_item, attempt.status, result)?;
        let question = CurrentRunQuestion {
            assignment_item: item.assignment_item,
            result,
            earned_points,
            possible_points,
        };
        let slot = &mut latest[position];
        if slot
            .as_ref()
            .is_none_or(|(at, id, _)| (submitted_at, attempt.id) > (*at, *id))
        {
            *slot = Some((submitted_at, attempt.id, question));
        }
    }
    Ok(latest
        .into_iter()
        .map(|entry| entry.map(|(_, _, question)| question))
        .collect())
}

/// Derives a completed run score from one current result per delivered position.
pub(crate) fn completed_run_score(
    questions: &[Option<CurrentRunQuestion>],
    requirement: question_model::CompletionRequirement,
) -> Result<Option<f64>, StoreError> {
    if questions.iter().any(Option::is_none) {
        return Ok(None);
    }
    let completion: Vec<_> = questions
        .iter()
        .map(|question| {
            let result = question
                .expect("missing results returned before projection")
                .result;
            RequiredQuestionState {
                answered: true,
                correct: result.correct,
                points_earned: result.points_earned,
                points_possible: result.points_possible,
            }
        })
        .collect();
    if derive_within_run_completion(&completion, requirement)? == RunCompletionStatus::InProgress {
        return Ok(None);
    }
    let earned: f64 = questions
        .iter()
        .map(|question| {
            question
                .expect("missing results returned before projection")
                .earned_points
        })
        .sum();
    let possible: f64 = questions
        .iter()
        .map(|question| {
            question
                .expect("missing results returned before projection")
                .possible_points
        })
        .sum();
    if !earned.is_finite() || !possible.is_finite() || possible < 0.0 {
        return Err(StoreError::RunModel(RunModelError::InvalidQuestionPoints));
    }
    let score = if possible > 0.0 {
        earned / possible
    } else {
        earned
    };
    if !score.is_finite() || !(-1_000.0..=1_000.0).contains(&score) {
        return Err(StoreError::RunModel(RunModelError::InvalidQuestionPoints));
    }
    Ok(Some(score_precision::round_for_persistence(score)))
}

/// Refuses malformed backend grades before they can enter attempt history.
pub(crate) fn validate_attempt_result(result: AttemptResult) -> Result<(), StoreError> {
    let credit = result.points_earned / result.points_possible;
    if !result.points_earned.is_finite()
        || !result.points_possible.is_finite()
        || result.points_possible <= 0.0
        || !credit.is_finite()
        || !(-1_000.0..=1_000.0).contains(&credit)
    {
        return Err(StoreError::InvalidRecord(
            "attempt result must have positive possible points and normalized credit from -1000 to 1000"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_title(kind: &str, title: &str) -> Result<(), StoreError> {
    const MAX_TITLE_CHARS: usize = 200;
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must not be empty"
        )));
    }
    if trimmed.chars().count() > MAX_TITLE_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must contain at most {MAX_TITLE_CHARS} characters"
        )));
    }
    if trimmed != title {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must not have leading or trailing whitespace"
        )));
    }
    Ok(())
}

/// Enforces the draft identity invariant before a backend writes bytes.
pub(crate) fn project_enrollment_completion(
    enrollment: &mut AssignmentEnrollment,
    previous: &StudentAssignmentSummary,
    grade: GradePolicy,
    run: RunId,
    score: f64,
    at: ActivityTimestamp,
) {
    let is_first_completion = previous.completed_run_count == 0;
    let is_new_best = previous.best_score.is_none_or(|best| score > best);

    if enrollment.first_completed_at.is_none() {
        enrollment.first_completed_at = Some(at);
    }
    if is_new_best || enrollment.best_grade_run.is_none() {
        enrollment.best_grade_run = Some(run);
    }
    enrollment.current_grade_run = match grade {
        GradePolicy::First if is_first_completion => Some(run),
        GradePolicy::First => enrollment.current_grade_run,
        GradePolicy::Latest => Some(run),
        GradePolicy::Highest if is_new_best => Some(run),
        GradePolicy::Highest => enrollment.current_grade_run,
        GradePolicy::InstructorSelected => enrollment.current_grade_run,
    };
}
