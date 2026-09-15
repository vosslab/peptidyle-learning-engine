//! Current Assessment Attempt preparation and pool selection.

use super::PostgresAssessmentAttemptStore;
use super::assessment_delivery::PostgresLiveAssessmentDeliveryStore;
use super::connection::map_sqlx_error;
use crate::{
    AssessmentAttemptStart, AssessmentAttemptStore, PreparedIssuedQuestion,
    PreparedQuestionPoolSelection, SessionTokenHash, StoreError,
};
use question_model::{
    AssessmentEntryId, AssessmentId, AssessmentReference, CourseInstanceReference,
    PoolRevisionMemberReference, QuestionBackend, QuestionPoolRevisionNumber,
    QuestionPoolRevisionReference, QuestionPoolSelectedItem, QuestionPoolSelectionId,
    QuestionRevisionNumber, QuestionRevisionReference, StudentRecordId,
};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug)]
struct CurrentPoolEntry {
    id: AssessmentEntryId,
    authored_position: i32,
    selection_count: usize,
    reuse_selection: bool,
    random_selected_order: bool,
    candidates: Vec<QuestionPoolSelectedItem>,
    question_pool_revision: QuestionPoolRevisionReference,
    candidate_backends: BTreeMap<PoolRevisionMemberReference, QuestionBackend>,
}

pub(super) async fn start_current_assessment_attempt(
    store: &PostgresLiveAssessmentDeliveryStore,
    token: SessionTokenHash,
    course: CourseInstanceReference,
    assessment: AssessmentReference,
) -> Result<crate::AssessmentAttemptStartResult, StoreError> {
    let mut tx = store.begin(token).await?;
    let rows = sqlx::query(
        "SELECT student_record_id, assessment_id, assessment_entry_id, entry_kind, authored_position, \
         fixed_question_id, fixed_revision_number, question_pool_public_id, question_pool_revision_number, \
         member_position, pool_question_id, pool_revision_number, question_backend, selection_count, \
         pool_selection_rule, question_pool_reuse_rule \
         FROM ple_api.prepare_current_assessment_attempt_start($1, $2)",
    )
        .bind(course.as_string())
    .bind(assessment.as_string())
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let start = current_attempt_start_from_rows(&mut tx, rows).await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    PostgresAssessmentAttemptStore::new(store.pool.clone())
        .start_assessment_attempt(token, start)
        .await
}

async fn current_attempt_start_from_rows(
    tx: &mut Transaction<'_, Postgres>,
    rows: Vec<sqlx::postgres::PgRow>,
) -> Result<AssessmentAttemptStart, StoreError> {
    let first = rows.first().ok_or(StoreError::NotFound)?;
    let student_record =
        StudentRecordId::from_uuid(first.try_get("student_record_id").map_err(map_sqlx_error)?);
    let assessment =
        AssessmentId::from_uuid(first.try_get("assessment_id").map_err(map_sqlx_error)?);
    let mut fixed = Vec::new();
    let mut pools = BTreeMap::<Uuid, CurrentPoolEntry>::new();
    for row in rows {
        let entry = AssessmentEntryId::from_uuid(
            row.try_get("assessment_entry_id").map_err(map_sqlx_error)?,
        );
        let authored_position: i32 = row.try_get("authored_position").map_err(map_sqlx_error)?;
        match row
            .try_get::<String, _>("entry_kind")
            .map_err(map_sqlx_error)?
            .as_str()
        {
            "fixed_question" => fixed.push((
                authored_position,
                PreparedIssuedQuestion::FixedQuestion {
                    assessment_entry: entry,
                    reference: row_question_revision(
                        &row,
                        "fixed_question_id",
                        "fixed_revision_number",
                    )?,
                    backend: row_question_backend(&row)?,
                },
            )),
            "question_pool" => {
                let question_pool_revision = row_pool_revision(&row)?;
                let pool = pools
                    .entry(entry.as_uuid())
                    .or_insert_with(|| CurrentPoolEntry {
                        id: entry,
                        authored_position,
                        selection_count: 0,
                        reuse_selection: false,
                        random_selected_order: false,
                        candidates: Vec::new(),
                        question_pool_revision,
                        candidate_backends: BTreeMap::new(),
                    });
                pool.selection_count = usize::try_from(
                    row.try_get::<i32, _>("selection_count")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question Pool selection count is invalid".to_string(),
                    )
                })?;
                pool.reuse_selection = row
                    .try_get::<String, _>("question_pool_reuse_rule")
                    .map_err(map_sqlx_error)?
                    == "reuse_selection";
                pool.random_selected_order = row
                    .try_get::<String, _>("pool_selection_rule")
                    .map_err(map_sqlx_error)?
                    == "random_order";
                let candidate = QuestionPoolSelectedItem {
                    pool_revision_member: PoolRevisionMemberReference {
                        question_pool_revision: pool.question_pool_revision.clone(),
                        member_position: row_member_position(&row)?,
                    },
                    reference: row_question_revision(
                        &row,
                        "pool_question_id",
                        "pool_revision_number",
                    )?,
                };
                pool.candidate_backends.insert(
                    candidate.pool_revision_member.clone(),
                    row_question_backend(&row)?,
                );
                pool.candidates.push(candidate);
            }
            _ => {
                return Err(StoreError::InvalidRecord(
                    "Assessment Entry kind is invalid".to_string(),
                ));
            }
        }
    }
    let mut selections = Vec::new();
    let mut issued = fixed;
    for pool in pools.into_values() {
        let (reused_from_question_pool_selection, selected_items) = if pool.reuse_selection {
            reusable_pool_selection(tx, assessment, student_record, pool.id)
                .await?
                .unwrap_or((None, select_pool_items(&pool)?))
        } else {
            (None, select_pool_items(&pool)?)
        };
        let index = selections.len();
        for item in &selected_items {
            let backend = *pool
                .candidate_backends
                .get(&item.pool_revision_member)
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "Question Pool Item backend is unavailable".to_string(),
                    )
                })?;
            issued.push((
                pool.authored_position,
                PreparedIssuedQuestion::QuestionPoolItem {
                    assessment_entry: pool.id,
                    question_pool_selection_index: index,
                    pool_revision_member: item.pool_revision_member.clone(),
                    reference: item.reference.clone(),
                    backend,
                },
            ));
        }
        selections.push(PreparedQuestionPoolSelection {
            question_pool_assessment_entry: pool.id,
            question_pool_revision: pool.question_pool_revision,
            reused_from_question_pool_selection,
            selected_items,
        });
    }
    issued.sort_by_key(|(position, _)| *position);
    Ok(AssessmentAttemptStart {
        student_record,
        assessment,
        question_pool_selections: selections,
        issued_questions: issued.into_iter().map(|(_, question)| question).collect(),
    })
}

fn row_question_revision(
    row: &sqlx::postgres::PgRow,
    id: &str,
    revision: &str,
) -> Result<QuestionRevisionReference, StoreError> {
    let question_id = row
        .try_get::<String, _>(id)
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Question ID is invalid".to_string()))?;
    let revision_number = QuestionRevisionNumber::new(
        u32::try_from(row.try_get::<i32, _>(revision).map_err(map_sqlx_error)?).map_err(|_| {
            StoreError::InvalidRecord("Question Revision number is invalid".to_string())
        })?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question Revision number is invalid".to_string()))?;
    Ok(QuestionRevisionReference {
        question_id,
        revision_number,
    })
}

fn row_pool_revision(
    row: &sqlx::postgres::PgRow,
) -> Result<QuestionPoolRevisionReference, StoreError> {
    let question_pool_id = row
        .try_get::<String, _>("question_pool_public_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Question Pool ID is invalid".to_string()))?;
    let revision_number = QuestionPoolRevisionNumber::new(
        u64::try_from(
            row.try_get::<i64, _>("question_pool_revision_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| {
            StoreError::InvalidRecord("Question Pool Revision number is invalid".to_string())
        })?,
    )
    .map_err(|_| {
        StoreError::InvalidRecord("Question Pool Revision number is invalid".to_string())
    })?;
    Ok(QuestionPoolRevisionReference {
        question_pool_id,
        revision_number,
    })
}

fn row_member_position(row: &sqlx::postgres::PgRow) -> Result<u32, StoreError> {
    let stored = row
        .try_get::<i32, _>("member_position")
        .map_err(map_sqlx_error)?;
    let stored = u32::try_from(stored).map_err(|_| {
        StoreError::InvalidRecord("Pool Revision member position is invalid".to_string())
    })?;
    stored.checked_sub(1).ok_or_else(|| {
        StoreError::InvalidRecord("Pool Revision member position is invalid".to_string())
    })
}

fn row_question_backend(row: &sqlx::postgres::PgRow) -> Result<QuestionBackend, StoreError> {
    match row
        .try_get::<String, _>("question_backend")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "ple" => Ok(QuestionBackend::Ple),
        "webwork" => Ok(QuestionBackend::Webwork),
        "imathas" => Ok(QuestionBackend::Imathas),
        _ => Err(StoreError::InvalidRecord(
            "Question backend is invalid".to_string(),
        )),
    }
}

fn select_pool_items(pool: &CurrentPoolEntry) -> Result<Vec<QuestionPoolSelectedItem>, StoreError> {
    select_pool_items_with_rng(pool, || {
        crate::random_uuid::random_u64(|error| {
            StoreError::Unavailable(format!("Question Pool randomness unavailable: {error}"))
        })
    })
}

fn select_pool_items_with_rng(
    pool: &CurrentPoolEntry,
    mut random_u64: impl FnMut() -> Result<u64, StoreError>,
) -> Result<Vec<QuestionPoolSelectedItem>, StoreError> {
    if pool.selection_count == 0 || pool.candidates.len() < pool.selection_count {
        return Err(StoreError::InvalidRecord(
            "Question Pool has insufficient available Items".to_string(),
        ));
    }
    let mut selected = pool.candidates.clone();
    for position in 0..pool.selection_count {
        let index = position + unbiased_index(selected.len() - position, &mut random_u64)?;
        selected.swap(position, index);
    }
    selected.truncate(pool.selection_count);
    if pool.random_selected_order {
        for position in 0..selected.len() {
            let index = position + unbiased_index(selected.len() - position, &mut random_u64)?;
            selected.swap(position, index);
        }
    } else {
        selected.sort_by_key(|item| {
            pool.candidates
                .iter()
                .position(|candidate| candidate.pool_revision_member == item.pool_revision_member)
                .expect("selected pool item came from candidates")
        });
    }
    Ok(selected)
}

fn unbiased_index(
    bound: usize,
    random_u64: &mut impl FnMut() -> Result<u64, StoreError>,
) -> Result<usize, StoreError> {
    let bound = u64::try_from(bound)
        .map_err(|_| StoreError::InvalidRecord("Question Pool size is invalid".to_string()))?;
    let acceptance_limit = u64::MAX - (u64::MAX % bound);
    loop {
        let value = random_u64()?;
        if value < acceptance_limit {
            return usize::try_from(value % bound).map_err(|_| {
                StoreError::InvalidRecord("Question Pool selection is invalid".to_string())
            });
        }
    }
}

async fn reusable_pool_selection(
    tx: &mut Transaction<'_, Postgres>,
    assessment: AssessmentId,
    student_record: StudentRecordId,
    entry: AssessmentEntryId,
) -> Result<
    Option<(
        Option<QuestionPoolSelectionId>,
        Vec<QuestionPoolSelectedItem>,
    )>,
    StoreError,
> {
    let rows = sqlx::query(
        "SELECT question_pool_selection_id, question_pool_public_id, question_pool_revision_number, member_position, question_id, revision_number \
         FROM ple_api.read_reusable_question_pool_selection($1, $2, $3) ORDER BY selection_position",
    )
    .bind(assessment.as_uuid()).bind(student_record.as_uuid()).bind(entry.as_uuid())
    .fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let selection = QuestionPoolSelectionId::from_uuid(
        first
            .try_get("question_pool_selection_id")
            .map_err(map_sqlx_error)?,
    );
    let items = rows
        .iter()
        .map(|row| {
            Ok(QuestionPoolSelectedItem {
                pool_revision_member: PoolRevisionMemberReference {
                    question_pool_revision: row_pool_revision(row)?,
                    member_position: row_member_position(row)?,
                },
                reference: row_question_revision(row, "question_id", "revision_number")?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Some((Some(selection), items)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{QuestionId, QuestionPoolRevisionNumber, QuestionPoolRevisionReference};

    fn pool_item(value: u32) -> QuestionPoolSelectedItem {
        QuestionPoolSelectedItem {
            pool_revision_member: PoolRevisionMemberReference {
                question_pool_revision: QuestionPoolRevisionReference {
                    question_pool_id: "0000-X00N".parse::<QuestionId>().expect("Pool ID"),
                    revision_number: QuestionPoolRevisionNumber::new(1).expect("revision"),
                },
                member_position: value,
            },
            reference: QuestionRevisionReference {
                question_id: "0000-X00N".parse::<QuestionId>().expect("question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("revision"),
            },
        }
    }

    #[test]
    fn pool_selection_uses_injected_entropy_without_replacement_then_keeps_pool_order() {
        let pool = CurrentPoolEntry {
            id: AssessmentEntryId::from_uuid(Uuid::nil()),
            authored_position: 0,
            selection_count: 2,
            reuse_selection: false,
            random_selected_order: false,
            candidates: vec![pool_item(1), pool_item(2), pool_item(3)],
            candidate_backends: BTreeMap::new(),
        };
        let mut entropy = [1_u64, 1].into_iter();
        let selected = select_pool_items_with_rng(&pool, || Ok(entropy.next().expect("entropy")))
            .expect("selection");
        assert_eq!(
            selected
                .iter()
                .map(|item| item.pool_revision_member.member_position)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn pool_selection_uses_injected_entropy_for_configured_random_order() {
        let pool = CurrentPoolEntry {
            id: AssessmentEntryId::from_uuid(Uuid::nil()),
            authored_position: 0,
            selection_count: 2,
            reuse_selection: false,
            random_selected_order: true,
            candidates: vec![pool_item(1), pool_item(2), pool_item(3)],
            candidate_backends: BTreeMap::new(),
        };
        let mut entropy = [1_u64, 1, 1, 0].into_iter();
        let selected = select_pool_items_with_rng(&pool, || Ok(entropy.next().expect("entropy")))
            .expect("selection");
        assert_eq!(
            selected
                .iter()
                .map(|item| item.pool_revision_member.member_position)
                .collect::<Vec<_>>(),
            vec![3, 2]
        );
    }
}
