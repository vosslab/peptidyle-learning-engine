//! PostgreSQL persistence for Course Instance creation and initial teaching team.

use std::sync::Arc;

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintRevision, CourseInstanceId, CourseMembershipRole, CourseSummary,
    CourseTerm, CourseTheme,
};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::{map_sqlx_error, parse_course_id};
use crate::course_instance::CourseInstanceBlueprintOrigin;
use crate::{
    CourseCreationInstructor, CourseInstanceCreationSource, CourseInstanceLifecycleState,
    CourseInstancePoolIdIssuer, CourseInstanceStore, CourseInstanceSummary, CourseInstanceView,
    CreateCourseInstanceInput, CreatedCourseInstance, SessionTokenHash, StoreError,
};

const ADOPTION_POOL_IDENTITY_ATTEMPTS: usize = 8;

/// PostgreSQL Store for Course Instance creation and current Teaching Team reads.
#[derive(Clone)]
pub struct PostgresCourseInstanceStore {
    pool: Pool,
    pool_id_issuer: Option<Arc<dyn CourseInstancePoolIdIssuer>>,
}

impl PostgresCourseInstanceStore {
    /// Binds the attested API pool to Course Instance procedures.
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            pool_id_issuer: None,
        }
    }

    /// Adds the process-held issuer required only when a Blueprint contains a
    /// reusable Question Pool.  Empty and fixed-Question-only Course creation
    /// deliberately remain independent of this capability.
    pub fn with_question_pool_id_issuer(
        mut self,
        pool_id_issuer: Arc<dyn CourseInstancePoolIdIssuer>,
    ) -> Self {
        self.pool_id_issuer = Some(pool_id_issuer);
        self
    }

    async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
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
impl CourseInstanceStore for PostgresCourseInstanceStore {
    async fn update_course_classification(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceId,
        expected_edit_number: question_model::CourseEditNumber,
        classification: question_model::CourseClassification,
    ) -> Result<crate::course_instance::CourseClassificationUpdate, StoreError> {
        classification
            .validate()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row =
            sqlx::query("SELECT * FROM ple_api.update_course_classification($1,$2,$3,$4,$5,$6,$7)")
                .bind(reference.as_string())
                .bind(expected_edit_number.as_i64())
                .bind(classification.discipline_uuid)
                .bind(classification.subject_uuid)
                .bind(classification.topic_uuid)
                .bind(classification.subtopic_uuid)
                .bind(super::blueprint_course::classification_tags(
                    &classification,
                ))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let result = crate::course_instance::CourseClassificationUpdate {
            classification,
            course_edit_number: question_model::CourseEditNumber::from_edit_number(
                row.try_get("course_edit_number").map_err(map_sqlx_error)?,
            ),
            changed: row.try_get("changed").map_err(map_sqlx_error)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn resolve_course_navigation(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceId,
    ) -> Result<CourseInstanceId, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.3, 8.2.2, and 8.3.1: the procedure resolves an opaque
        // reference only after binding it to the installed active membership.
        let row = sqlx::query("SELECT course_id FROM ple_api.resolve_course_navigation($1)")
            .bind(reference.as_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let course = row
            .map(|row| {
                row.try_get("course_id")
                    .map_err(map_sqlx_error)
                    .and_then(parse_course_id)
            })
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(course)
    }

    async fn read_course_summary(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
    ) -> Result<CourseSummary, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.3, 8.2.2, and 8.3.1: the SECURITY DEFINER procedure binds
        // this opaque Course ID to the installed session's active membership.
        let row = sqlx::query(
            "SELECT course_id, public_reference, short_name, long_name, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, membership_role, discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags \
             FROM ple_api.read_course_summary($1)",
        )
        .bind(course.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let summary = row
            .as_ref()
            .map(decode_course_summary)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(summary)
    }

    async fn list_course_instances(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseInstanceSummary>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query(
            "SELECT public_reference, short_name, long_name, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, course_theme, course_lifecycle_state, course_edit_number, \
             content_discipline_id AS discipline_uuid, content_subject_id AS subject_uuid, \
             content_topic_id AS topic_uuid, content_subtopic_id AS subtopic_uuid, tags \
             FROM ple_api.list_course_instances()",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn create_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateCourseInstanceInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreatedCourseInstance, StoreError> {
        input.validate()?;
        for attempt in 0..ADOPTION_POOL_IDENTITY_ATTEMPTS {
            // A rolled-back identity collision leaves SQL receipts unconsumed.
            let mut attempt_bloom_receipts = bloom_receipts.clone();
            let mut transaction = self
                .begin_authenticated_application_transaction(session_token_hash)
                .await?;
            let assessments = super::course_blueprint_adoption::creation_assessments(
                &mut transaction,
                &input,
                self.pool_id_issuer.as_deref(),
                &mut attempt_bloom_receipts,
            )
            .await?;
            let (source_kind, blueprint_reference, blueprint_revision) = match &input.source {
                CourseInstanceCreationSource::Empty => ("empty", None, None),
                CourseInstanceCreationSource::Adopted {
                    blueprint_course,
                    blueprint_revision,
                } => (
                    "adopted",
                    Some(blueprint_course.to_string()),
                    Some(
                        i64::try_from(blueprint_revision.value())
                            .map_err(|_| invalid("Blueprint Revision"))?,
                    ),
                ),
            };
            let row = sqlx::query(
            "SELECT public_reference, short_name, long_name, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, course_lifecycle_state, course_edit_number, \
             content_discipline_id AS discipline_uuid, content_subject_id AS subject_uuid, \
             content_topic_id AS topic_uuid, content_subtopic_id AS subtopic_uuid, tags \
             FROM ple_api.create_course_instance(\
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10::date, $11::date, $12, $13, $14,$15,$16,$17,$18)",
        )
            .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(source_kind)
        .bind(blueprint_reference)
        .bind(blueprint_revision)
        .bind(&input.short_name)
        .bind(&input.long_name)
        .bind(input.term.start_date().to_string())
        .bind(input.term.end_date().to_string())
        .bind(
            input
                .assigned_instructor
                .as_ref()
                .map(|reference| reference.as_string()),
        )
        .bind(assessments)
        .bind(input.classification.discipline_uuid)
        .bind(input.classification.subject_uuid)
        .bind(input.classification.topic_uuid)
        .bind(input.classification.subtopic_uuid)
        .bind(super::blueprint_course::classification_tags(&input.classification))
            .fetch_one(&mut *transaction)
            .await;
            let row = match row {
                Ok(row) => row,
                Err(error) => {
                    let error = map_sqlx_error(error);
                    if matches!(error, StoreError::AlreadyExists)
                        && attempt + 1 < ADOPTION_POOL_IDENTITY_ATTEMPTS
                    {
                        continue;
                    }
                    return Err(error);
                }
            };
            let record = CreatedCourseInstance {
                course: CourseInstanceSummary {
                    classification: super::blueprint_course::decode_classification(&row)?,
                    lifecycle_state: lifecycle_state(
                        row.try_get("course_lifecycle_state")
                            .map_err(map_sqlx_error)?,
                    )?,
                    course_edit_number: question_model::CourseEditNumber::from_edit_number(
                        row.try_get("course_edit_number").map_err(map_sqlx_error)?,
                    ),
                    reference: course_reference(
                        row.try_get("public_reference").map_err(map_sqlx_error)?,
                    )?,
                    short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
                    long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
                    term: term(
                        row.try_get("term_starts_on").map_err(map_sqlx_error)?,
                        row.try_get("term_ends_on").map_err(map_sqlx_error)?,
                    )?,
                    theme: CourseTheme::default(),
                },
            };
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(record);
        }
        Err(StoreError::Unavailable(
            "Question Pool fork identity collision retries were exhausted".to_string(),
        ))
    }

    async fn add_course_instructor(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
        instructor: AccountId,
    ) -> Result<(), StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // The SQL command verifies the caller's active Instructor membership
        // before adding a peer membership. Neither creator nor assigned
        // Instructor identity establishes this authority.
        sqlx::query("SELECT ple_api.add_course_instructor($1, $2, $3)")
            .bind(random_uuid()?)
            .bind(course.as_string())
            .bind(instructor.as_string())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn load_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceId,
    ) -> Result<CourseInstanceView, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT public_reference, short_name, long_name, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, course_theme, course_lifecycle_state, course_edit_number, \
             content_discipline_id AS discipline_uuid, content_subject_id AS subject_uuid, \
             content_topic_id AS topic_uuid, content_subtopic_id AS subtopic_uuid, tags, \
             active_instructor_count, blueprint_reference, adopted_blueprint_revision, \
             current_blueprint_revision FROM ple_api.load_course_instance($1)",
        )
        .bind(reference.as_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_view)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_course_creation_instructors(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseCreationInstructor>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query("SELECT * FROM ple_api.list_course_creation_instructors()")
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(CourseCreationInstructor {
                    reference: account_reference(
                        row.try_get("public_reference").map_err(map_sqlx_error)?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<CourseInstanceSummary, StoreError> {
    Ok(CourseInstanceSummary {
        classification: super::blueprint_course::decode_classification(row)?,
        lifecycle_state: lifecycle_state(
            row.try_get("course_lifecycle_state")
                .map_err(map_sqlx_error)?,
        )?,
        course_edit_number: question_model::CourseEditNumber::from_edit_number(
            row.try_get("course_edit_number").map_err(map_sqlx_error)?,
        ),
        reference: course_reference(row.try_get("public_reference").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        term: term(
            row.try_get("term_starts_on").map_err(map_sqlx_error)?,
            row.try_get("term_ends_on").map_err(map_sqlx_error)?,
        )?,
        theme: theme(row.try_get("course_theme").map_err(map_sqlx_error)?)?,
    })
}

fn decode_course_summary(row: &sqlx::postgres::PgRow) -> Result<CourseSummary, StoreError> {
    let course = parse_course_id(row.try_get("public_reference").map_err(map_sqlx_error)?)?;
    let stored_membership_role: String = row.try_get("membership_role").map_err(map_sqlx_error)?;
    Ok(CourseSummary {
        classification: super::blueprint_course::decode_classification(row)?,
        id: course.clone(),
        reference: course,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        term: term(
            row.try_get("term_starts_on").map_err(map_sqlx_error)?,
            row.try_get("term_ends_on").map_err(map_sqlx_error)?,
        )?,
        role: membership_role(&stored_membership_role)?,
    })
}

fn decode_view(row: &sqlx::postgres::PgRow) -> Result<CourseInstanceView, StoreError> {
    let active_instructor_count: i64 = row
        .try_get("active_instructor_count")
        .map_err(map_sqlx_error)?;
    let blueprint_reference: Option<String> =
        row.try_get("blueprint_reference").map_err(map_sqlx_error)?;
    let adopted_revision: Option<i64> = row
        .try_get("adopted_blueprint_revision")
        .map_err(map_sqlx_error)?;
    let current_revision: Option<i64> = row
        .try_get("current_blueprint_revision")
        .map_err(map_sqlx_error)?;
    // ASVS 2.2.3: the nullable projection is all-or-nothing and revisions remain ordered.
    let blueprint_origin = match (blueprint_reference, adopted_revision, current_revision) {
        (None, None, None) => None,
        (Some(reference), Some(adopted), Some(current)) if current >= adopted => {
            let revision = |value| {
                u64::try_from(value)
                    .ok()
                    .and_then(BlueprintRevision::new)
                    .ok_or_else(|| invalid("Blueprint Revision"))
            };
            Some(CourseInstanceBlueprintOrigin {
                reference: reference
                    .parse()
                    .map_err(|_| invalid("Blueprint Course Reference"))?,
                adopted_revision: revision(adopted)?,
                current_revision: revision(current)?,
            })
        }
        _ => return Err(invalid("Blueprint origin")),
    };
    Ok(CourseInstanceView {
        course: decode_summary(row)?,
        active_instructor_count: u32::try_from(active_instructor_count)
            .map_err(|_| invalid("Teaching Team size"))?,
        blueprint_origin,
    })
}

fn course_reference(value: String) -> Result<CourseInstanceId, StoreError> {
    CourseInstanceId::new(value).map_err(|_| invalid("Course Instance Reference"))
}

fn account_reference(value: String) -> Result<AccountId, StoreError> {
    AccountId::new(value).map_err(|_| invalid("Account Reference"))
}

fn term(start_date: String, end_date: String) -> Result<CourseTerm, StoreError> {
    CourseTerm::from_parts(&start_date, &end_date).map_err(|_| invalid("Course Term"))
}

fn lifecycle_state(value: String) -> Result<CourseInstanceLifecycleState, StoreError> {
    match value.as_str() {
        "active" => Ok(CourseInstanceLifecycleState::Active),
        "inactive" => Ok(CourseInstanceLifecycleState::Inactive),
        _ => Err(invalid("Course Instance lifecycle state")),
    }
}

fn membership_role(value: &str) -> Result<CourseMembershipRole, StoreError> {
    match value {
        "student" => Ok(CourseMembershipRole::Student),
        "instructor" => Ok(CourseMembershipRole::Instructor),
        _ => Err(invalid("Course Membership Role")),
    }
}

fn theme(value: String) -> Result<CourseTheme, StoreError> {
    value.parse().map_err(|_| invalid("Course Theme"))
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Course Instance UUID randomness unavailable".to_string())
    })
}
