//! PostgreSQL course appearance, banner promotion, delivery, and cleanup.

use async_trait::async_trait;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    ActivityTimestamp, CourseAppearance, CourseAppearanceRevision, CourseBannerAltText,
    CourseBannerAlternativeText, CourseBannerCandidateId, CourseBannerId, CourseBannerMutation,
    CourseBannerPresentation, CourseId, CourseThemeId, UserId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::{
    PostgresStore, database_timestamp, decode_payload_row_named, encode_payload, map_sqlx_error,
};
use crate::{
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope,
    AuthorizedAssetDelivery, COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH, CourseAppearanceStore,
    CourseBannerCleanupBatch, CourseBannerCleanupClaim, CourseBannerCleanupToken,
    CourseBannerPromotion, RegisterCourseBannerCandidate, SaveCourseAppearance, SessionTokenHash,
    StoreError, TenantContext, validate_asset_delivery,
};

#[derive(Debug)]
struct StoredCandidate {
    candidate: CourseBannerCandidateId,
    creator: UserId,
    object_id: question_model::ObjectId,
    checksum: Sha256Digest,
    size_bytes: u64,
    banner: CourseBannerId,
    future_object_id: question_model::ObjectId,
    expires_at: ActivityTimestamp,
    promoted: Option<ObjectRecord>,
    consumed: bool,
    candidate_deleted: bool,
    cleanup_token: Option<CourseBannerCleanupToken>,
}

#[async_trait]
impl CourseAppearanceStore for PostgresStore {
    async fn course_appearance(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseAppearance>, StoreError> {
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        if !appearance_authorized(&mut transaction, session, course, false).await? {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        let row = sqlx::query(
            "SELECT theme_id, current_banner_delivery_id, banner_alt_kind, banner_alt_text, revision \
             FROM course_appearance WHERE tenant_id = $1 AND course_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let appearance = row.as_ref().map(decode_appearance).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(appearance)
    }

    async fn register_course_banner_candidate(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        command: RegisterCourseBannerCandidate,
    ) -> Result<(), StoreError> {
        validate_candidate(context, course, &command)?;
        let mut transaction = self.begin_tenant(context).await?;
        let actor = require_manager(&mut transaction, session, course).await?;
        let now = database_timestamp(&mut transaction).await?;
        if command.expires_at <= now {
            return Err(StoreError::InvalidRecord(
                "course banner candidate expiry must be in the future".to_string(),
            ));
        }
        let size_bytes = i64::try_from(command.object.size_bytes).map_err(|_| {
            StoreError::InvalidRecord("banner candidate byte size is too large".to_string())
        })?;
        let future_object =
            objects::course_banner_object_id(context.tenant_id(), course, command.banner);
        sqlx::query(
            "INSERT INTO course_banner_candidate \
             (tenant_id, course_id, candidate_id, created_by, candidate_object_id, \
              normalized_sha256, size_bytes, width, height, future_banner_id, future_object_id, \
              expires_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, \
                     to_timestamp($12::double precision / 1000.0), \
                     to_timestamp($13::double precision / 1000.0))",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(command.candidate.as_uuid())
        .bind(actor.as_uuid())
        .bind(command.object.id.as_uuid())
        .bind(command.object.sha256.to_string())
        .bind(size_bytes)
        .bind(
            i32::try_from(command.width)
                .map_err(|_| StoreError::InvalidRecord("banner width is too large".to_string()))?,
        )
        .bind(
            i32::try_from(command.height)
                .map_err(|_| StoreError::InvalidRecord("banner height is too large".to_string()))?,
        )
        .bind(command.banner.as_uuid())
        .bind(future_object.as_uuid())
        .bind(command.expires_at.as_unix_millis())
        .bind(command.object.created_at.as_unix_millis())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn course_banner_promotion(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        candidate: CourseBannerCandidateId,
    ) -> Result<CourseBannerPromotion, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let actor = require_manager(&mut transaction, session, course).await?;
        let row = sqlx::query(
            "SELECT candidate_id, created_by, candidate_object_id, normalized_sha256, size_bytes, \
                    future_banner_id, future_object_id, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis, \
                    promoted_payload, promoted_payload_sha256, consumed, candidate_deleted, \
                    cleanup_claim_id \
             FROM course_banner_candidate \
             WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(candidate.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let stored = decode_candidate(&row)?;
        if stored.candidate != candidate
            || stored.creator != actor
            || stored.cleanup_token.is_some()
            || stored.candidate_deleted
            || stored.consumed
        {
            return Err(StoreError::NotFound);
        }
        let now = database_timestamp(&mut transaction).await?;
        if stored.expires_at <= now {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseBannerPromotion {
            candidate,
            banner: stored.banner,
            sha256: stored.checksum,
            size_bytes: stored.size_bytes,
        })
    }

    async fn save_course_appearance(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        command: SaveCourseAppearance,
    ) -> Result<CourseAppearance, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let actor = require_manager(&mut transaction, session, course).await?;
        let appearance_row = sqlx::query(
            "SELECT theme_id, current_banner_delivery_id, banner_alt_kind, banner_alt_text, revision \
             FROM course_appearance WHERE tenant_id = $1 AND course_id = $2 FOR UPDATE",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let current = decode_appearance(&appearance_row)?;

        let replacement = match (&command.update.banner, &command.promoted_object) {
            (CourseBannerMutation::Replace { candidate, .. }, Some(promoted_object)) => {
                let row = sqlx::query(
                    "SELECT candidate_id, created_by, candidate_object_id, normalized_sha256, size_bytes, \
                            future_banner_id, future_object_id, \
                            floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis, \
                            promoted_payload, promoted_payload_sha256, consumed, candidate_deleted, \
                            cleanup_claim_id \
                     FROM course_banner_candidate \
                     WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3 FOR UPDATE",
                )
                .bind(context.tenant_id().as_uuid())
                .bind(course.as_uuid())
                .bind(candidate.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
                let stored = decode_candidate(&row)?;
                if stored.creator != actor
                    || stored.cleanup_token.is_some()
                    || stored.candidate_deleted
                {
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
                persist_course_banner_delivery(&mut transaction, &delivery).await?;
                let (payload, checksum) = encode_payload(promoted_object)?;
                if let Some(existing) = &stored.promoted {
                    if existing != promoted_object {
                        return Err(StoreError::Conflict);
                    }
                } else {
                    sqlx::query(
                        "UPDATE course_banner_candidate \
                         SET promoted_payload = $4, promoted_payload_sha256 = $5 \
                         WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3",
                    )
                    .bind(context.tenant_id().as_uuid())
                    .bind(course.as_uuid())
                    .bind(candidate.as_uuid())
                    .bind(payload)
                    .bind(checksum)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                let now = database_timestamp(&mut transaction).await?;
                if stored.consumed || stored.expires_at <= now {
                    transaction.commit().await.map_err(map_sqlx_error)?;
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
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Err(StoreError::Conflict);
        }
        let revision = current.revision.checked_next().ok_or_else(|| {
            StoreError::Unavailable("course appearance revision limit reached".to_string())
        })?;
        let (banner, alt_kind, alt_text) = match command.update.banner {
            CourseBannerMutation::Keep { alternative_text } => {
                let current_banner = current.banner.ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "cannot keep alternative text when no current banner exists".to_string(),
                    )
                })?;
                let (kind, text) = encode_alternative(&alternative_text);
                (Some(current_banner.id), Some(kind), text)
            }
            CourseBannerMutation::Remove => (None, None, None),
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
                let (kind, text) = encode_alternative(&alternative_text);
                sqlx::query(
                    "UPDATE course_banner_candidate SET consumed = true \
                     WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3",
                )
                .bind(context.tenant_id().as_uuid())
                .bind(course.as_uuid())
                .bind(candidate.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                (Some(banner), Some(kind), text)
            }
        };
        sqlx::query(
            "UPDATE course_appearance \
             SET theme_id = $3, current_banner_delivery_id = $4, banner_alt_kind = $5, \
                 banner_alt_text = $6, revision = $7, updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND course_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(command.update.theme.as_str())
        .bind(banner.map(|value| value.as_uuid()))
        .bind(alt_kind)
        .bind(alt_text.clone())
        .bind(i64::try_from(revision.value()).map_err(|_| {
            StoreError::Unavailable("course appearance revision limit reached".to_string())
        })?)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseAppearance {
            theme: command.update.theme,
            revision,
            banner: banner.map(|id| CourseBannerPresentation {
                id,
                alternative_text: decode_alternative(alt_kind, alt_text.as_deref())
                    .expect("validated alternative encoding must decode"),
            }),
        })
    }

    async fn authorize_course_banner_delivery(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        banner: CourseBannerId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let banner_course = course_for_banner(&mut transaction, context, banner).await?;
        let actor = appearance_actor(&mut transaction, session, banner_course, false)
            .await?
            .ok_or(StoreError::NotFound)?;
        let row = sqlx::query(
            "SELECT ad.payload, ad.payload_sha256, ad.course_id \
             FROM asset_delivery AS ad \
             JOIN course_appearance AS appearance \
               ON appearance.tenant_id = ad.tenant_id AND appearance.course_id = ad.course_id \
              AND appearance.current_banner_delivery_id = ad.delivery_id \
             WHERE ad.tenant_id = $1 AND ad.delivery_id = $2 \
               AND ad.delivery_kind = 'course_banner'",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(banner.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let record: AssetDeliveryRecord =
            decode_payload_row_named(&row, "payload", "payload_sha256")?;
        validate_asset_delivery(&record).map_err(|error| {
            StoreError::Unavailable(format!("stored course banner delivery is invalid: {error}"))
        })?;
        let course_uuid: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
        let course = CourseId::from_uuid(course_uuid);
        let authorized_at = database_timestamp(&mut transaction).await?;
        let event = AssetAccessEvent {
            tenant: context.tenant_id(),
            actor,
            delivery: AssetDeliveryId::from_course_banner(banner),
            object: record.object.id,
            bucket: record.object.bucket,
            course: Some(course),
            occurred_at: authorized_at,
        };
        let (payload, checksum) = encode_payload(&event)?;
        sqlx::query(
            "INSERT INTO record_access_log \
             (tenant_id, access_log_id, occurred_at, payload, payload_sha256, \
              delivery_scope, delivery_id, course_id) \
             VALUES ($1, gen_random_uuid(), transaction_timestamp(), $2, $3, \
                     'course_banner', $4, $5)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(payload)
        .bind(checksum)
        .bind(banner.as_uuid())
        .bind(course.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT candidate_id, course_id, candidate_deleted, future_banner_id, \
                    promoted_payload, promoted_payload_sha256 \
             FROM course_banner_candidate \
             WHERE tenant_id = $1 AND expires_at <= transaction_timestamp() \
               AND (cleanup_claim_expires_at IS NULL \
                    OR cleanup_claim_expires_at <= transaction_timestamp()) \
             ORDER BY expires_at, candidate_id FOR UPDATE SKIP LOCKED LIMIT $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(i64::from(batch.get()))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut claims = Vec::new();
        for row in rows {
            let candidate = CourseBannerCandidateId::from_uuid(
                row.try_get("candidate_id").map_err(map_sqlx_error)?,
            );
            let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
            let banner =
                CourseBannerId::from_uuid(row.try_get("future_banner_id").map_err(map_sqlx_error)?);
            let candidate_deleted: bool =
                row.try_get("candidate_deleted").map_err(map_sqlx_error)?;
            let current: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM course_appearance \
                 WHERE tenant_id = $1 AND course_id = $2 \
                   AND current_banner_delivery_id = $3)",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .bind(banner.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let promoted: Option<ObjectRecord> = decode_optional_promoted(&row)?;
            let candidate_object = (!candidate_deleted).then(|| ObjectKey::CourseBannerCandidate {
                tenant: context.tenant_id(),
                course,
                candidate,
            });
            let promoted_object = promoted.filter(|_| !current).map(|record| record.key);
            if candidate_object.is_none() && promoted_object.is_none() {
                continue;
            }
            let token_uuid: Uuid = sqlx::query_scalar("SELECT gen_random_uuid()")
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            sqlx::query(
                "UPDATE course_banner_candidate \
                 SET cleanup_claim_id = $4, \
                     cleanup_claim_expires_at = transaction_timestamp() + interval '5 minutes' \
                 WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .bind(candidate.as_uuid())
            .bind(token_uuid)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            claims.push(CourseBannerCleanupClaim {
                course,
                candidate,
                token: CourseBannerCleanupToken::from_uuid(token_uuid),
                candidate_object,
                promoted_object,
            });
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(claims)
    }

    async fn complete_course_banner_cleanup(
        &self,
        context: TenantContext,
        claim: CourseBannerCleanupClaim,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT candidate_id, created_by, candidate_object_id, normalized_sha256, size_bytes, \
                    future_banner_id, future_object_id, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis, \
                    promoted_payload, promoted_payload_sha256, consumed, candidate_deleted, \
                    cleanup_claim_id \
             FROM course_banner_candidate \
             WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3 FOR UPDATE",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(claim.course.as_uuid())
        .bind(claim.candidate.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Err(StoreError::NotFound);
        };
        let stored = decode_candidate(&row)?;
        if stored.cleanup_token != Some(claim.token) {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(false);
        }
        let current: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course_appearance \
             WHERE tenant_id = $1 AND course_id = $2 AND current_banner_delivery_id = $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(claim.course.as_uuid())
        .bind(stored.banner.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let expected_candidate =
            (!stored.candidate_deleted).then(|| ObjectKey::CourseBannerCandidate {
                tenant: context.tenant_id(),
                course: claim.course,
                candidate: claim.candidate,
            });
        let expected_promoted = stored
            .promoted
            .as_ref()
            .filter(|_| !current)
            .map(|record| record.key.clone());
        if claim.candidate_object != expected_candidate
            || claim.promoted_object != expected_promoted
        {
            return Err(StoreError::InvalidRecord(
                "banner cleanup claim no longer matches persisted ownership".to_string(),
            ));
        }
        if expected_promoted.is_some() {
            sqlx::query(
                "DELETE FROM asset_delivery \
                 WHERE tenant_id = $1 AND course_id = $2 AND delivery_id = $3 \
                   AND delivery_kind = 'course_banner'",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(claim.course.as_uuid())
            .bind(stored.banner.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        sqlx::query(
            "UPDATE course_banner_candidate \
             SET candidate_deleted = candidate_deleted OR $4, \
                 promoted_payload = CASE WHEN $5 THEN NULL ELSE promoted_payload END, \
                 promoted_payload_sha256 = CASE WHEN $5 THEN NULL ELSE promoted_payload_sha256 END, \
                 cleanup_claim_id = NULL, cleanup_claim_expires_at = NULL \
             WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(claim.course.as_uuid())
        .bind(claim.candidate.as_uuid())
        .bind(expected_candidate.is_some())
        .bind(expected_promoted.is_some())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM course_banner_candidate \
             WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3 \
               AND candidate_deleted AND promoted_payload IS NULL",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(claim.course.as_uuid())
        .bind(claim.candidate.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(true)
    }
}

async fn appearance_authorized(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
    manager_only: bool,
) -> Result<bool, StoreError> {
    sqlx::query_scalar("SELECT ple_course_appearance_authorize($1, $2, $3)")
        .bind(session.to_string())
        .bind(course.as_uuid())
        .bind(manager_only)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)
}

async fn appearance_actor(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
    manager_only: bool,
) -> Result<Option<UserId>, StoreError> {
    let actor: Option<Uuid> = sqlx::query_scalar("SELECT ple_course_appearance_actor($1, $2, $3)")
        .bind(session.to_string())
        .bind(course.as_uuid())
        .bind(manager_only)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    Ok(actor.map(UserId::from_uuid))
}

async fn require_manager(
    transaction: &mut Transaction<'_, Postgres>,
    session: SessionTokenHash,
    course: CourseId,
) -> Result<UserId, StoreError> {
    if let Some(actor) = appearance_actor(transaction, session, course, true).await? {
        return Ok(actor);
    }
    if appearance_authorized(transaction, session, course, false).await? {
        Err(StoreError::Forbidden)
    } else {
        Err(StoreError::NotFound)
    }
}

async fn course_for_banner(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    banner: CourseBannerId,
) -> Result<CourseId, StoreError> {
    let course: Uuid = sqlx::query_scalar(
        "SELECT course_id FROM asset_delivery \
         WHERE tenant_id = $1 AND delivery_id = $2 AND delivery_kind = 'course_banner'",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(banner.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    Ok(CourseId::from_uuid(course))
}

async fn persist_course_banner_delivery(
    transaction: &mut Transaction<'_, Postgres>,
    record: &AssetDeliveryRecord,
) -> Result<(), StoreError> {
    let AssetDeliveryScope::CourseBanner { tenant, course, .. } = record.scope else {
        return Err(StoreError::InvalidRecord(
            "appearance promotion requires a course-banner delivery".to_string(),
        ));
    };
    let (payload, checksum) = encode_payload(record)?;
    sqlx::query(
        "INSERT INTO asset_delivery \
         (delivery_id, delivery_kind, tenant_id, course_id, object_id, problem_id, version_id, \
          asset_id, payload, payload_sha256) \
         VALUES ($1, 'course_banner', $2, $3, $4, NULL, NULL, NULL, $5, $6) \
         ON CONFLICT (delivery_id) DO NOTHING",
    )
    .bind(record.id.as_uuid())
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(record.object.id.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let row =
        sqlx::query("SELECT payload, payload_sha256 FROM asset_delivery WHERE delivery_id = $1")
            .bind(record.id.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    let existing: AssetDeliveryRecord =
        decode_payload_row_named(&row, "payload", "payload_sha256")?;
    if &existing != record {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn decode_appearance(row: &sqlx::postgres::PgRow) -> Result<CourseAppearance, StoreError> {
    let theme: String = row.try_get("theme_id").map_err(map_sqlx_error)?;
    let theme = theme.parse::<CourseThemeId>().map_err(|_| {
        StoreError::Unavailable("stored course appearance theme is invalid".to_string())
    })?;
    let revision: i64 = row.try_get("revision").map_err(map_sqlx_error)?;
    let revision = u64::try_from(revision)
        .ok()
        .and_then(CourseAppearanceRevision::new)
        .ok_or_else(|| {
            StoreError::Unavailable("stored course appearance revision is invalid".to_string())
        })?;
    let banner: Option<Uuid> = row
        .try_get("current_banner_delivery_id")
        .map_err(map_sqlx_error)?;
    let alt_kind: Option<String> = row.try_get("banner_alt_kind").map_err(map_sqlx_error)?;
    let alt_text: Option<String> = row.try_get("banner_alt_text").map_err(map_sqlx_error)?;
    let banner = match banner {
        Some(banner) => Some(CourseBannerPresentation {
            id: CourseBannerId::from_uuid(banner),
            alternative_text: decode_alternative(alt_kind.as_deref(), alt_text.as_deref())?,
        }),
        None if alt_kind.is_none() && alt_text.is_none() => None,
        None => {
            return Err(StoreError::Unavailable(
                "stored course appearance banner state is inconsistent".to_string(),
            ));
        }
    };
    Ok(CourseAppearance {
        theme,
        revision,
        banner,
    })
}

fn decode_alternative(
    kind: Option<&str>,
    text: Option<&str>,
) -> Result<CourseBannerAlternativeText, StoreError> {
    match (kind, text) {
        (Some("decorative"), None) => Ok(CourseBannerAlternativeText::Decorative),
        (Some("informative"), Some(text)) => Ok(CourseBannerAlternativeText::Informative {
            text: CourseBannerAltText::try_from(text.to_string()).map_err(|_| {
                StoreError::Unavailable(
                    "stored informative banner alternative is invalid".to_string(),
                )
            })?,
        }),
        _ => Err(StoreError::Unavailable(
            "stored banner alternative state is invalid".to_string(),
        )),
    }
}

fn encode_alternative(alternative: &CourseBannerAlternativeText) -> (&'static str, Option<String>) {
    match alternative {
        CourseBannerAlternativeText::Decorative => ("decorative", None),
        CourseBannerAlternativeText::Informative { text } => {
            ("informative", Some(text.as_str().to_string()))
        }
    }
}

fn decode_candidate(row: &sqlx::postgres::PgRow) -> Result<StoredCandidate, StoreError> {
    let checksum: String = row.try_get("normalized_sha256").map_err(map_sqlx_error)?;
    let checksum = serde_json::from_str::<Sha256Digest>(&format!("\"{}\"", checksum.trim_end()))
        .map_err(|_| {
            StoreError::Unavailable("stored banner candidate checksum is invalid".to_string())
        })?;
    let size_bytes: i64 = row.try_get("size_bytes").map_err(map_sqlx_error)?;
    let size_bytes = u64::try_from(size_bytes).map_err(|_| {
        StoreError::Unavailable("stored banner candidate size is invalid".to_string())
    })?;
    let expires_at_millis: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
    let cleanup_id: Option<Uuid> = row.try_get("cleanup_claim_id").map_err(map_sqlx_error)?;
    Ok(StoredCandidate {
        candidate: CourseBannerCandidateId::from_uuid(
            row.try_get("candidate_id").map_err(map_sqlx_error)?,
        ),
        creator: UserId::from_uuid(row.try_get("created_by").map_err(map_sqlx_error)?),
        object_id: question_model::ObjectId::from_uuid(
            row.try_get("candidate_object_id").map_err(map_sqlx_error)?,
        ),
        checksum,
        size_bytes,
        banner: CourseBannerId::from_uuid(row.try_get("future_banner_id").map_err(map_sqlx_error)?),
        future_object_id: question_model::ObjectId::from_uuid(
            row.try_get("future_object_id").map_err(map_sqlx_error)?,
        ),
        expires_at: ActivityTimestamp::from_unix_millis(expires_at_millis),
        promoted: decode_optional_promoted(row)?,
        consumed: row.try_get("consumed").map_err(map_sqlx_error)?,
        candidate_deleted: row.try_get("candidate_deleted").map_err(map_sqlx_error)?,
        cleanup_token: cleanup_id.map(CourseBannerCleanupToken::from_uuid),
    })
}

fn decode_optional_promoted(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<ObjectRecord>, StoreError> {
    let payload: Option<serde_json::Value> =
        row.try_get("promoted_payload").map_err(map_sqlx_error)?;
    let checksum: Option<String> = row
        .try_get("promoted_payload_sha256")
        .map_err(map_sqlx_error)?;
    match (payload, checksum) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => {
            decode_payload_row_named(row, "promoted_payload", "promoted_payload_sha256").map(Some)
        }
        _ => Err(StoreError::Unavailable(
            "stored promoted banner payload is incomplete".to_string(),
        )),
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
    candidate: &StoredCandidate,
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
        || promoted.id != candidate.future_object_id
        || promoted.id != promoted.key.object_id()
        || promoted.bucket != Bucket::Content
        || promoted.category != ObjectCategory::CourseContent
        || promoted.version.is_some()
        || promoted.media_type != "image/webp"
        || promoted.sha256 != candidate.checksum
        || promoted.size_bytes != candidate.size_bytes
        || candidate.object_id
            != objects::course_banner_candidate_object_id(
                context.tenant_id(),
                course,
                candidate.candidate,
            )
    {
        return Err(StoreError::InvalidRecord(
            "promoted banner does not match the candidate bytes and future identity".to_string(),
        ));
    }
    Ok(())
}
