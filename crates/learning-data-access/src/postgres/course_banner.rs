//! PostgreSQL adapter for the Course Banner promotion boundary.

use async_trait::async_trait;
use objects::{ObjectAddress, Sha256Checksum};
use question_model::{
    CourseBanner, CourseBannerAlternativeText, CourseBannerReference, CourseBannerUploadReference,
    CourseId,
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    ClaimedCourseBannerUpload, CourseBannerDeleteWork, CourseBannerStore,
    FinalizedCourseBannerPromotion, PrepareCourseBannerPromotion, PreparedCourseBannerPromotion,
    PreparedCourseBannerRemoval, SessionTokenHash, StageCourseBannerUpload,
    StagedCourseBannerUpload, StoreError,
};

/// PostgreSQL Store for Course Banner staging, promotion, and cleanup work.
#[derive(Clone)]
pub struct PostgresCourseBannerStore {
    pool: Pool,
}

impl PostgresCourseBannerStore {
    /// Binds Course Banner operations to the supplied application pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl CourseBannerStore for PostgresCourseBannerStore {
    async fn stage_course_banner_upload(
        &self,
        token: SessionTokenHash,
        request: StageCourseBannerUpload,
    ) -> Result<StagedCourseBannerUpload, StoreError> {
        let StageCourseBannerUpload {
            course,
            upload,
            metadata,
            width,
            height,
            expires_at_unix_millis: expires,
        } = request;
        let size = i64::try_from(metadata.byte_length).map_err(|_| {
            StoreError::InvalidRecord("Course Banner object exceeds PostgreSQL bigint".to_string())
        })?;
        let mut tx = self.begin(token).await?;
        let work_id =
            sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT ple_api.stage_course_banner_upload($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(course.as_uuid())
            .bind(upload.as_uuid())
            .bind(metadata.object_id.as_uuid())
            .bind(metadata.media_type)
            .bind(size)
            .bind(metadata.sha256.as_bytes().to_vec())
            .bind(i32::try_from(width).map_err(|_| {
                StoreError::InvalidRecord("invalid Course Banner width".to_string())
            })?)
            .bind(i32::try_from(height).map_err(|_| {
                StoreError::InvalidRecord("invalid Course Banner height".to_string())
            })?)
            .bind(expires)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(StagedCourseBannerUpload {
            address: ObjectAddress::CourseBannerUpload { course, upload },
            put_work_id: work_id,
        })
    }

    async fn finalize_course_banner_upload_stage(
        &self,
        token: SessionTokenHash,
        course: CourseId,
        upload: CourseBannerUploadReference,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.finalize_course_banner_upload_stage($1,$2)",
        )
        .bind(course.as_uuid())
        .bind(upload.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn prepare_course_banner_promotion(
        &self,
        token: SessionTokenHash,
        request: PrepareCourseBannerPromotion,
    ) -> Result<PreparedCourseBannerPromotion, StoreError> {
        let PrepareCourseBannerPromotion {
            course,
            upload,
            banner,
            update,
            source,
            hero,
            card,
        } = request;
        let (kind, text): (&str, Option<&str>) = match &update.alternative_text {
            CourseBannerAlternativeText::Decorative => ("decorative", None),
            CourseBannerAlternativeText::Informative { text } => {
                ("informative", Some(text.as_str()))
            }
        };
        let convert = |value: u64| {
            i64::try_from(value).map_err(|_| {
                StoreError::InvalidRecord(
                    "Course Banner object exceeds PostgreSQL bigint".to_string(),
                )
            })
        };
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT source_put_work_id, hero_put_work_id, card_put_work_id FROM ple_api.prepare_course_banner_promotion($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)")
            .bind(course.as_uuid()).bind(upload.as_uuid()).bind(banner.as_uuid()).bind(kind).bind(text)
            .bind(source.object_id.as_uuid()).bind(source.sha256.as_bytes().to_vec()).bind(convert(source.byte_length)?).bind(source.media_type)
            .bind(hero.object_id.as_uuid()).bind(hero.sha256.as_bytes().to_vec()).bind(convert(hero.byte_length)?).bind(hero.media_type)
            .bind(card.object_id.as_uuid()).bind(card.sha256.as_bytes().to_vec()).bind(convert(card.byte_length)?).bind(card.media_type)
            .fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(PreparedCourseBannerPromotion {
            banner,
            source: ObjectAddress::CourseBannerSource { course, banner },
            hero: ObjectAddress::CourseBannerRendition {
                course,
                banner,
                rendition: question_model::CourseBannerRendition::Hero,
            },
            card: ObjectAddress::CourseBannerRendition {
                course,
                banner,
                rendition: question_model::CourseBannerRendition::Card,
            },
            source_put_work_id: row.try_get("source_put_work_id").map_err(map_sqlx_error)?,
            hero_put_work_id: row.try_get("hero_put_work_id").map_err(map_sqlx_error)?,
            card_put_work_id: row.try_get("card_put_work_id").map_err(map_sqlx_error)?,
        })
    }

    async fn prepare_course_banner_object_deletion(
        &self,
        token: SessionTokenHash,
        put_work_id: Uuid,
    ) -> Result<CourseBannerDeleteWork, StoreError> {
        let mut tx = self.begin(token).await?;
        let work_id = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT ple_api.prepare_course_banner_object_deletion($1)",
        )
        .bind(put_work_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseBannerDeleteWork { work_id })
    }

    async fn complete_prepared_course_banner_object(
        &self,
        token: SessionTokenHash,
        course: CourseId,
        banner: CourseBannerReference,
        object_id: question_model::ObjectId,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.complete_prepared_course_banner_object($1,$2,$3)",
        )
        .bind(course.as_uuid())
        .bind(banner.as_uuid())
        .bind(object_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn require_course_banner_object_repair(
        &self,
        token: SessionTokenHash,
        course: CourseId,
        banner: Option<CourseBannerReference>,
        object_id: question_model::ObjectId,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.require_course_banner_object_repair($1,$2,$3)",
        )
        .bind(course.as_uuid())
        .bind(banner.map(CourseBannerReference::as_uuid))
        .bind(object_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn complete_course_banner_object_deletion(
        &self,
        token: SessionTokenHash,
        delete_work: CourseBannerDeleteWork,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.complete_course_banner_object_deletion($1)",
        )
        .bind(delete_work.work_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn require_course_banner_deletion_repair(
        &self,
        token: SessionTokenHash,
        delete_work: CourseBannerDeleteWork,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.require_course_banner_deletion_repair($1)",
        )
        .bind(delete_work.work_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn record_course_banner_cleanup_check(
        &self,
        token: SessionTokenHash,
        delete_work: CourseBannerDeleteWork,
        object_present: bool,
        observed_checksum: Option<Sha256Checksum>,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.record_course_banner_cleanup_check($1,$2,$3)",
        )
        .bind(delete_work.work_id)
        .bind(object_present)
        .bind(observed_checksum.map(|checksum| checksum.as_bytes().to_vec()))
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn finalize_course_banner_promotion(
        &self,
        token: SessionTokenHash,
        course: CourseId,
        upload: CourseBannerUploadReference,
        banner: CourseBannerReference,
    ) -> Result<FinalizedCourseBannerPromotion, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT alternative_kind, alternative_text, retired_course_banner_id, upload_put_work_id, retired_source_put_work_id, retired_hero_put_work_id, retired_card_put_work_id FROM ple_api.finalize_course_banner_promotion($1,$2,$3)")
            .bind(course.as_uuid()).bind(upload.as_uuid()).bind(banner.as_uuid()).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let kind: String = row.try_get("alternative_kind").map_err(map_sqlx_error)?;
        let alternative_text = match kind.as_str() {
            "decorative" => CourseBannerAlternativeText::Decorative,
            "informative" => CourseBannerAlternativeText::Informative {
                text: row
                    .try_get::<String, _>("alternative_text")
                    .map_err(map_sqlx_error)?
                    .try_into()
                    .map_err(|_| {
                        StoreError::InvalidRecord(
                            "invalid Course Banner alternative text".to_string(),
                        )
                    })?,
            },
            _ => {
                return Err(StoreError::InvalidRecord(
                    "invalid Course Banner alternative kind".to_string(),
                ));
            }
        };
        let retired_banner = row
            .try_get::<Option<Uuid>, _>("retired_course_banner_id")
            .map_err(map_sqlx_error)?
            .map(CourseBannerReference::from_uuid);
        let upload_put_work_id = row.try_get("upload_put_work_id").map_err(map_sqlx_error)?;
        let retired_work_ids = match retired_banner {
            Some(_) => Some((
                row.try_get("retired_source_put_work_id")
                    .map_err(map_sqlx_error)?,
                row.try_get("retired_hero_put_work_id")
                    .map_err(map_sqlx_error)?,
                row.try_get("retired_card_put_work_id")
                    .map_err(map_sqlx_error)?,
            )),
            None => None,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        let appearance = CourseBanner {
            reference: banner,
            alternative_text,
        };
        let retired = retired_banner.map(|banner| {
            let (source_put_work_id, hero_put_work_id, card_put_work_id) =
                retired_work_ids.expect("retired work ids are loaded");
            PreparedCourseBannerRemoval {
                banner,
                source: ObjectAddress::CourseBannerSource { course, banner },
                hero: ObjectAddress::CourseBannerRendition {
                    course,
                    banner,
                    rendition: question_model::CourseBannerRendition::Hero,
                },
                card: ObjectAddress::CourseBannerRendition {
                    course,
                    banner,
                    rendition: question_model::CourseBannerRendition::Card,
                },
                source_put_work_id,
                hero_put_work_id,
                card_put_work_id,
            }
        });
        Ok(FinalizedCourseBannerPromotion {
            banner: appearance,
            upload: ObjectAddress::CourseBannerUpload { course, upload },
            upload_put_work_id,
            retired,
        })
    }

    async fn read_current_course_banner(
        &self,
        token: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseBanner>, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT course_banner_id, alternative_kind, alternative_text FROM ple_api.read_course_banner($1)")
            .bind(course.as_uuid()).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?;
        let value = row
            .map(|row| {
                let id: Uuid = row.try_get("course_banner_id").map_err(map_sqlx_error)?;
                let kind: String = row.try_get("alternative_kind").map_err(map_sqlx_error)?;
                let alternative_text = match kind.as_str() {
                    "decorative" => CourseBannerAlternativeText::Decorative,
                    "informative" => CourseBannerAlternativeText::Informative {
                        text: row
                            .try_get::<String, _>("alternative_text")
                            .map_err(map_sqlx_error)?
                            .try_into()
                            .map_err(|_| {
                                StoreError::InvalidRecord(
                                    "invalid Course Banner alternative text".to_string(),
                                )
                            })?,
                    },
                    _ => {
                        return Err(StoreError::InvalidRecord(
                            "invalid Course Banner alternative kind".to_string(),
                        ));
                    }
                };
                Ok(CourseBanner {
                    reference: CourseBannerReference::from_uuid(id),
                    alternative_text,
                })
            })
            .transpose()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(value)
    }

    async fn resolve_current_course_banner(
        &self,
        token: SessionTokenHash,
        banner: CourseBannerReference,
    ) -> Result<CourseId, StoreError> {
        let mut tx = self.begin(token).await?;
        let course = sqlx::query_scalar::<_, Uuid>(
            "SELECT course_id FROM ple_api.resolve_current_course_banner($1)",
        )
        .bind(banner.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseId::from_uuid(course))
    }

    async fn read_staged_course_banner_upload(
        &self,
        token: SessionTokenHash,
        course: CourseId,
        upload: CourseBannerUploadReference,
    ) -> Result<ClaimedCourseBannerUpload, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT object_id, sha256, byte_length, canonical_media_type, width, height, put_work_id FROM ple_api.read_staged_course_banner_upload($1,$2)")
            .bind(course.as_uuid()).bind(upload.as_uuid()).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let object_id =
            question_model::ObjectId::from_uuid(row.try_get("object_id").map_err(map_sqlx_error)?);
        let digest: Vec<u8> = row.try_get("sha256").map_err(map_sqlx_error)?;
        let digest: [u8; 32] = digest
            .try_into()
            .map_err(|_| StoreError::InvalidRecord("invalid Course Banner checksum".to_string()))?;
        let size: i64 = row.try_get("byte_length").map_err(map_sqlx_error)?;
        let byte_length = u64::try_from(size)
            .map_err(|_| StoreError::InvalidRecord("invalid Course Banner size".to_string()))?;
        let canonical_media_type = row
            .try_get("canonical_media_type")
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(ClaimedCourseBannerUpload {
            upload,
            address: ObjectAddress::CourseBannerUpload { course, upload },
            object_id,
            sha256: Sha256Checksum::from_bytes(digest),
            byte_length,
            canonical_media_type,
            put_work_id: row.try_get("put_work_id").map_err(map_sqlx_error)?,
            width: u32::try_from(row.try_get::<i32, _>("width").map_err(map_sqlx_error)?).map_err(
                |_| StoreError::InvalidRecord("invalid Course Banner width".to_string()),
            )?,
            height: u32::try_from(row.try_get::<i32, _>("height").map_err(map_sqlx_error)?)
                .map_err(|_| {
                    StoreError::InvalidRecord("invalid Course Banner height".to_string())
                })?,
        })
    }

    async fn prepare_course_banner_removal(
        &self,
        token: SessionTokenHash,
        course: CourseId,
    ) -> Result<PreparedCourseBannerRemoval, StoreError> {
        let mut tx = self.begin(token).await?;
        let row =
            sqlx::query("SELECT course_banner_id, source_put_work_id, hero_put_work_id, card_put_work_id FROM ple_api.prepare_course_banner_removal($1)")
                .bind(course.as_uuid())
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
        let banner = CourseBannerReference::from_uuid(
            row.try_get("course_banner_id").map_err(map_sqlx_error)?,
        );
        let source_put_work_id = row.try_get("source_put_work_id").map_err(map_sqlx_error)?;
        let hero_put_work_id = row.try_get("hero_put_work_id").map_err(map_sqlx_error)?;
        let card_put_work_id = row.try_get("card_put_work_id").map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(PreparedCourseBannerRemoval {
            banner,
            source: ObjectAddress::CourseBannerSource { course, banner },
            hero: ObjectAddress::CourseBannerRendition {
                course,
                banner,
                rendition: question_model::CourseBannerRendition::Hero,
            },
            card: ObjectAddress::CourseBannerRendition {
                course,
                banner,
                rendition: question_model::CourseBannerRendition::Card,
            },
            source_put_work_id,
            hero_put_work_id,
            card_put_work_id,
        })
    }
}
