//! In-memory course appearance and banner lifecycle persistence.

use async_trait::async_trait;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord};
use question_model::{
    ActivityTimestamp, CourseAppearance, CourseBannerMutation, CourseBannerPresentation, CourseId,
    CourseRole, UserId, UserRole,
};

use super::{MemoryStore, State, course_records_accessible};
use crate::{
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope,
    AuthorizedAssetDelivery, COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH, CourseAppearanceStore,
    CourseBannerCleanupBatch, CourseBannerCleanupClaim, CourseBannerCleanupToken,
    CourseBannerPromotion, RegisterCourseBannerCandidate, SaveCourseAppearance, SessionSubject,
    SessionTokenHash, StoreError, TenantContext, validate_asset_delivery,
};

const CLEANUP_CLAIM_MILLIS: i64 = 5 * 60 * 1_000;

#[derive(Debug, Clone)]
pub(super) struct StoredCourseBannerCandidate {
    pub(super) creator: UserId,
    pub(super) object: ObjectRecord,
    pub(super) banner: question_model::CourseBannerId,
    pub(super) expires_at: ActivityTimestamp,
    pub(super) promoted: Option<ObjectRecord>,
    pub(super) consumed: bool,
    pub(super) candidate_deleted: bool,
    pub(super) cleanup: Option<(CourseBannerCleanupToken, ActivityTimestamp)>,
}

#[async_trait]
impl CourseAppearanceStore for MemoryStore {
    async fn course_appearance(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseAppearance>, StoreError> {
        let state = self.read_state()?;
        let Some(subject) = active_subject(&state, context, session) else {
            return Ok(None);
        };
        let Some(role) = appearance_role(&state, context, subject, course) else {
            return Ok(None);
        };
        if role == CourseRole::Student
            && !course_records_accessible(&state, context.tenant_id(), course)
        {
            return Ok(None);
        }
        Ok(state
            .course_appearances
            .get(&(context.tenant_id(), course))
            .cloned())
    }

    async fn register_course_banner_candidate(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        command: RegisterCourseBannerCandidate,
    ) -> Result<(), StoreError> {
        validate_candidate(context, course, &command)?;
        let mut state = self.write_state()?;
        let actor = require_manager(&state, context, session, course)?;
        if command.expires_at <= state.authoritative_time {
            return Err(StoreError::InvalidRecord(
                "course banner candidate expiry must be in the future".to_string(),
            ));
        }
        let key = (context.tenant_id(), course, command.candidate);
        if state.course_banner_candidates.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        state.course_banner_candidates.insert(
            key,
            StoredCourseBannerCandidate {
                creator: actor,
                object: command.object,
                banner: command.banner,
                expires_at: command.expires_at,
                promoted: None,
                consumed: false,
                candidate_deleted: false,
                cleanup: None,
            },
        );
        Ok(())
    }

    async fn course_banner_promotion(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        candidate: question_model::CourseBannerCandidateId,
    ) -> Result<CourseBannerPromotion, StoreError> {
        let state = self.read_state()?;
        let actor = require_manager(&state, context, session, course)?;
        let stored = state
            .course_banner_candidates
            .get(&(context.tenant_id(), course, candidate))
            .ok_or(StoreError::NotFound)?;
        if stored.creator != actor
            || stored.cleanup.is_some()
            || stored.candidate_deleted
            || stored.consumed
        {
            return Err(StoreError::NotFound);
        }
        if stored.expires_at <= state.authoritative_time {
            return Err(StoreError::Conflict);
        }
        Ok(CourseBannerPromotion {
            candidate,
            banner: stored.banner,
            sha256: stored.object.sha256,
            size_bytes: stored.object.size_bytes,
        })
    }

    async fn save_course_appearance(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        command: SaveCourseAppearance,
    ) -> Result<CourseAppearance, StoreError> {
        let mut state = self.write_state()?;
        let actor = require_manager(&state, context, session, course)?;
        let key = (context.tenant_id(), course);
        let current = state
            .course_appearances
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;

        let replacement = match (&command.update.banner, &command.promoted_object) {
            (CourseBannerMutation::Replace { candidate, .. }, Some(promoted_object)) => {
                let candidate_key = (context.tenant_id(), course, *candidate);
                let stored = state
                    .course_banner_candidates
                    .get(&candidate_key)
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                if stored.creator != actor || stored.cleanup.is_some() {
                    return Err(StoreError::NotFound);
                }
                validate_promoted(context, course, &stored, promoted_object)?;
                let delivery = AssetDeliveryRecord {
                    id: AssetDeliveryId::from_course_banner(stored.banner),
                    object: promoted_object.clone(),
                    scope: AssetDeliveryScope::CourseBanner {
                        tenant: context.tenant_id(),
                        course,
                        banner: stored.banner,
                    },
                };
                validate_asset_delivery(&delivery)?;
                if let Some(existing) = state.asset_deliveries.get(&delivery.id) {
                    if existing != &delivery {
                        return Err(StoreError::Conflict);
                    }
                } else {
                    state.asset_deliveries.insert(delivery.id, delivery);
                }
                let stored_mut = state
                    .course_banner_candidates
                    .get_mut(&candidate_key)
                    .ok_or(StoreError::NotFound)?;
                if let Some(existing) = &stored_mut.promoted
                    && existing != promoted_object
                {
                    return Err(StoreError::Conflict);
                }
                stored_mut.promoted = Some(promoted_object.clone());
                if stored.consumed || stored.expires_at <= state.authoritative_time {
                    return Err(StoreError::Conflict);
                }
                Some((*candidate, stored.banner))
            }
            (CourseBannerMutation::Replace { .. }, None) => {
                return Err(StoreError::InvalidRecord(
                    "course banner replacement requires its bytes-first promoted object"
                        .to_string(),
                ));
            }
            (CourseBannerMutation::Keep { .. } | CourseBannerMutation::Remove, Some(_)) => {
                return Err(StoreError::InvalidRecord(
                    "keep and remove cannot carry a promoted object".to_string(),
                ));
            }
            (CourseBannerMutation::Keep { .. } | CourseBannerMutation::Remove, None) => None,
        };

        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let revision = current.revision.checked_next().ok_or_else(|| {
            StoreError::Unavailable("course appearance revision limit reached".to_string())
        })?;
        let banner = match command.update.banner {
            CourseBannerMutation::Keep { alternative_text } => {
                let current_banner = current.banner.ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "cannot keep alternative text when no current banner exists".to_string(),
                    )
                })?;
                Some(CourseBannerPresentation {
                    id: current_banner.id,
                    alternative_text,
                })
            }
            CourseBannerMutation::Remove => None,
            CourseBannerMutation::Replace {
                candidate,
                alternative_text,
            } => {
                let (replacement_candidate, banner) = replacement.ok_or_else(|| {
                    StoreError::InvalidRecord("replacement candidate is unavailable".to_string())
                })?;
                if replacement_candidate != candidate {
                    return Err(StoreError::InvalidRecord(
                        "replacement candidate identity changed".to_string(),
                    ));
                }
                state
                    .course_banner_candidates
                    .get_mut(&(context.tenant_id(), course, candidate))
                    .ok_or(StoreError::NotFound)?
                    .consumed = true;
                Some(CourseBannerPresentation {
                    id: banner,
                    alternative_text,
                })
            }
        };
        let saved = CourseAppearance {
            theme: command.update.theme,
            revision,
            banner,
        };
        state.course_appearances.insert(key, saved.clone());
        Ok(saved)
    }

    async fn authorize_course_banner_delivery(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        banner: question_model::CourseBannerId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut state = self.write_state()?;
        let Some(subject) = active_subject(&state, context, session).cloned() else {
            return Err(StoreError::NotFound);
        };
        let delivery_id = AssetDeliveryId::from_course_banner(banner);
        let record = state
            .asset_deliveries
            .get(&delivery_id)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let AssetDeliveryScope::CourseBanner {
            tenant,
            course,
            banner: scoped_banner,
        } = record.scope
        else {
            return Err(StoreError::NotFound);
        };
        if tenant != context.tenant_id() || scoped_banner != banner {
            return Err(StoreError::NotFound);
        }
        let Some(role) = appearance_role(&state, context, &subject, course) else {
            return Err(StoreError::NotFound);
        };
        if role == CourseRole::Student
            && !course_records_accessible(&state, context.tenant_id(), course)
        {
            return Err(StoreError::NotFound);
        }
        let current = state
            .course_appearances
            .get(&(context.tenant_id(), course))
            .and_then(|appearance| appearance.banner.as_ref())
            .is_some_and(|presentation| presentation.id == banner);
        if !current {
            return Err(StoreError::NotFound);
        }
        let authorized_at = state.authoritative_time;
        state.asset_access_events.push(AssetAccessEvent {
            tenant: context.tenant_id(),
            actor: subject.user(),
            delivery: delivery_id,
            object: record.object.id,
            bucket: record.object.bucket,
            course: Some(course),
            occurred_at: authorized_at,
        });
        Ok(AuthorizedAssetDelivery {
            record,
            authorized_at,
        })
    }

    async fn claim_course_banner_cleanup(
        &self,
        context: TenantContext,
        batch: CourseBannerCleanupBatch,
    ) -> Result<Vec<CourseBannerCleanupClaim>, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let tenant = context.tenant_id();
        let keys = state
            .course_banner_candidates
            .keys()
            .filter(|(candidate_tenant, _, _)| *candidate_tenant == tenant)
            .copied()
            .collect::<Vec<_>>();
        let mut claims = Vec::new();
        for key @ (_, course, candidate) in keys {
            if claims.len() >= usize::from(batch.get()) {
                break;
            }
            let current_banner = state
                .course_appearances
                .get(&(tenant, course))
                .and_then(|appearance| appearance.banner.as_ref())
                .map(|presentation| presentation.id);
            let stored = state
                .course_banner_candidates
                .get_mut(&key)
                .ok_or(StoreError::NotFound)?;
            if stored.expires_at > now
                || stored
                    .cleanup
                    .is_some_and(|(_, claim_expires)| claim_expires > now)
            {
                continue;
            }
            let candidate_object = (!stored.candidate_deleted).then(|| stored.object.key.clone());
            let promoted_object = stored
                .promoted
                .as_ref()
                .filter(|_| current_banner != Some(stored.banner))
                .map(|record| record.key.clone());
            if candidate_object.is_none() && promoted_object.is_none() {
                continue;
            }
            let token = CourseBannerCleanupToken::generate()?;
            let claim_expires_at = ActivityTimestamp::from_unix_millis(
                now.as_unix_millis()
                    .checked_add(CLEANUP_CLAIM_MILLIS)
                    .ok_or_else(|| {
                        StoreError::Unavailable("banner cleanup lease overflow".to_string())
                    })?,
            );
            stored.cleanup = Some((token, claim_expires_at));
            claims.push(CourseBannerCleanupClaim {
                course,
                candidate,
                token,
                candidate_object,
                promoted_object,
            });
        }
        Ok(claims)
    }

    async fn complete_course_banner_cleanup(
        &self,
        context: TenantContext,
        claim: CourseBannerCleanupClaim,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let key = (context.tenant_id(), claim.course, claim.candidate);
        let current_banner = state
            .course_appearances
            .get(&(context.tenant_id(), claim.course))
            .and_then(|appearance| appearance.banner.as_ref())
            .map(|presentation| presentation.id);
        let stored = state
            .course_banner_candidates
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if stored.cleanup.map(|(token, _)| token) != Some(claim.token) {
            return Ok(false);
        }
        let expected_candidate = (!stored.candidate_deleted).then(|| stored.object.key.clone());
        let expected_promoted = stored
            .promoted
            .as_ref()
            .filter(|_| current_banner != Some(stored.banner))
            .map(|record| record.key.clone());
        if claim.candidate_object != expected_candidate
            || claim.promoted_object != expected_promoted
        {
            return Err(StoreError::InvalidRecord(
                "banner cleanup claim no longer matches persisted ownership".to_string(),
            ));
        }
        let delivery_to_remove = expected_promoted
            .as_ref()
            .map(|_| AssetDeliveryId::from_course_banner(stored.banner));
        let stored = state
            .course_banner_candidates
            .get_mut(&key)
            .ok_or(StoreError::NotFound)?;
        if expected_candidate.is_some() {
            stored.candidate_deleted = true;
        }
        if expected_promoted.is_some() {
            stored.promoted = None;
        }
        stored.cleanup = None;
        if let Some(delivery) = delivery_to_remove {
            state.asset_deliveries.remove(&delivery);
        }
        let remove_candidate = state
            .course_banner_candidates
            .get(&key)
            .is_some_and(|candidate| candidate.candidate_deleted && candidate.promoted.is_none());
        if remove_candidate {
            state.course_banner_candidates.remove(&key);
        }
        Ok(true)
    }
}

fn active_subject(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Option<&SessionSubject> {
    let stored = state.sessions.get(&session)?;
    (!stored.revoked
        && stored.record.expires_at > state.authoritative_time
        && stored.record.subject.tenant() == context.tenant_id())
    .then_some(&stored.record.subject)
}

fn appearance_role(
    state: &State,
    context: TenantContext,
    subject: &SessionSubject,
    course: CourseId,
) -> Option<CourseRole> {
    let record = state.courses.get(&(context.tenant_id(), course))?;
    if subject.roles().contains(&UserRole::Administrator) {
        return Some(CourseRole::Administrator);
    }
    record.role_for(subject.user())
}

fn require_manager(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
    course: CourseId,
) -> Result<UserId, StoreError> {
    let subject = active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    match appearance_role(state, context, subject, course) {
        Some(CourseRole::Administrator | CourseRole::Instructor) => Ok(subject.user()),
        Some(CourseRole::Student) => Err(StoreError::Forbidden),
        None => Err(StoreError::NotFound),
    }
}

fn validate_candidate(
    context: TenantContext,
    course: CourseId,
    command: &RegisterCourseBannerCandidate,
) -> Result<(), StoreError> {
    let ObjectKey::CourseBannerCandidate {
        tenant,
        course: key_course,
        candidate,
    } = command.object.key
    else {
        return Err(StoreError::InvalidRecord(
            "banner candidate must use its typed temporary key".to_string(),
        ));
    };
    if tenant != context.tenant_id()
        || key_course != course
        || candidate != command.candidate
        || command.object.id != command.object.key.object_id()
        || command.object.bucket != Bucket::TempProcessing
        || command.object.category != ObjectCategory::Temporary
        || command.object.version.is_some()
        || command.object.media_type != "image/webp"
        || command.width != COURSE_BANNER_WIDTH
        || command.height != COURSE_BANNER_HEIGHT
        || command.expires_at <= command.object.created_at
    {
        return Err(StoreError::InvalidRecord(
            "banner candidate metadata does not match the normalized object contract".to_string(),
        ));
    }
    Ok(())
}

fn validate_promoted(
    context: TenantContext,
    course: CourseId,
    candidate: &StoredCourseBannerCandidate,
    promoted: &ObjectRecord,
) -> Result<(), StoreError> {
    let ObjectKey::CourseBanner {
        tenant,
        course: key_course,
        banner,
    } = promoted.key
    else {
        return Err(StoreError::InvalidRecord(
            "promoted banner must use its typed immutable key".to_string(),
        ));
    };
    if tenant != context.tenant_id()
        || key_course != course
        || banner != candidate.banner
        || promoted.id != promoted.key.object_id()
        || promoted.bucket != Bucket::Content
        || promoted.category != ObjectCategory::CourseContent
        || promoted.version.is_some()
        || promoted.media_type != "image/webp"
        || promoted.sha256 != candidate.object.sha256
        || promoted.size_bytes != candidate.object.size_bytes
    {
        return Err(StoreError::InvalidRecord(
            "promoted banner does not match the candidate bytes and future identity".to_string(),
        ));
    }
    Ok(())
}
