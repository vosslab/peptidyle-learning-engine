//! Current Assessment Attempt preparation and pool selection.

use super::PostgresAssessmentAttemptStore;
use super::assessment_delivery::PostgresLiveAssessmentDeliveryStore;
use super::connection::{map_sqlx_error, parse_assessment_id};
use crate::{
    AssessmentAttemptStart, PreparedIssuedQuestion, PreparedQuestionPoolSelection,
    SessionTokenHash, StoreError,
};
use question_model::{
    AssessmentAttemptId, AssessmentEntryId, AssessmentId, CourseInstanceId,
    PoolRevisionMemberReference, QuestionBackend, QuestionPoolRevisionNumber,
    QuestionPoolRevisionReference, QuestionPoolSelectedItem, QuestionRevisionNumber,
    QuestionRevisionReference, StudentRecordId,
};
use sqlx::Row;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug)]
struct CurrentPoolEntry {
    id: AssessmentEntryId,
    authored_position: i32,
    selection_count: usize,
    random_selected_order: bool,
    candidates: Vec<QuestionPoolSelectedItem>,
    question_pool_revision: QuestionPoolRevisionReference,
    candidate_backends: BTreeMap<PoolRevisionMemberReference, QuestionBackend>,
}

pub(super) async fn start_current_assessment_attempt(
    store: &PostgresLiveAssessmentDeliveryStore,
    token: SessionTokenHash,
    course: CourseInstanceId,
    assessment: AssessmentId,
) -> Result<crate::AssessmentAttemptStartResult, StoreError> {
    let mut tx = store.begin(token).await?;
    let decision = sqlx::query(
        "SELECT resumable_assessment_attempt_id, resumable_assessment_attempt_number \
         FROM ple_api.prepare_current_assessment_attempt_start_decision($1, $2)",
    )
    .bind(course.as_string())
    .bind(assessment.as_string())
    .fetch_one(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let resumable_id: Option<Uuid> = decision
        .try_get("resumable_assessment_attempt_id")
        .map_err(map_sqlx_error)?;
    let resumable_number: Option<i32> = decision
        .try_get("resumable_assessment_attempt_number")
        .map_err(map_sqlx_error)?;
    match (resumable_id, resumable_number) {
        (Some(id), Some(number)) => {
            let attempt_number = u32::try_from(number)
                .ok()
                .filter(|number| *number > 0)
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "resumable Assessment Attempt number is invalid".to_string(),
                    )
                })?;
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AssessmentAttemptStartResult {
                assessment_attempt: AssessmentAttemptId::from_uuid(id),
                attempt_number,
                resumed: true,
            });
        }
        (None, None) => {}
        _ => {
            return Err(StoreError::InvalidRecord(
                "resumable Assessment Attempt decision is invalid".to_string(),
            ));
        }
    }
    let rows = sqlx::query(
        "SELECT student_record_id, assessment_id, assessment_entry_id, entry_kind, authored_position, \
         fixed_question_id, fixed_revision_number, question_pool_public_id, question_pool_revision_number, \
         member_position, pool_question_id, pool_revision_number, question_backend, selection_count, \
         pool_selection_rule, assessment_question_order_rule \
         FROM ple_api.prepare_current_assessment_attempt_start($1, $2)",
    )
        .bind(course.as_string())
    .bind(assessment.as_string())
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let start = current_attempt_start_from_rows(rows)?;
    let result =
        PostgresAssessmentAttemptStore::start_assessment_attempt_in_transaction(&mut tx, start)
            .await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

fn current_attempt_start_from_rows(
    rows: Vec<sqlx::postgres::PgRow>,
) -> Result<AssessmentAttemptStart, StoreError> {
    let first = rows.first().ok_or(StoreError::NotFound)?;
    let student_record =
        StudentRecordId::from_uuid(first.try_get("student_record_id").map_err(map_sqlx_error)?);
    let assessment = parse_assessment_id(first.try_get("assessment_id").map_err(map_sqlx_error)?)?;
    let question_order_rule = first
        .try_get::<String, _>("assessment_question_order_rule")
        .map_err(map_sqlx_error)?;
    let shuffle_questions = match question_order_rule.as_str() {
        "authored_order" => false,
        "shuffled" => true,
        _ => {
            return Err(StoreError::InvalidRecord(
                "Assessment Question order rule is invalid".to_string(),
            ));
        }
    };
    let mut fixed = Vec::new();
    let mut pools = BTreeMap::<Uuid, CurrentPoolEntry>::new();
    for row in rows {
        if row
            .try_get::<String, _>("assessment_question_order_rule")
            .map_err(map_sqlx_error)?
            != question_order_rule
        {
            return Err(StoreError::InvalidRecord(
                "Assessment Question order rule is inconsistent".to_string(),
            ));
        }
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
        let selected_items = select_pool_items(&pool)?;
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
            selected_items,
        });
    }
    issued.sort_by_key(|(position, _)| *position);
    if shuffle_questions {
        shuffle_issued_questions(&mut issued)?;
    }
    Ok(AssessmentAttemptStart {
        student_record,
        assessment,
        question_pool_selections: selections,
        issued_questions: issued.into_iter().map(|(_, question)| question).collect(),
    })
}

fn shuffle_issued_questions<T>(issued: &mut [T]) -> Result<(), StoreError> {
    let mut random_u64 = || {
        crate::random_uuid::random_u64(|error| {
            StoreError::Unavailable(format!(
                "Assessment Question order randomness unavailable: {error}"
            ))
        })
    };
    for position in 0..issued.len() {
        let index = position + unbiased_index(issued.len() - position, &mut random_u64)?;
        issued.swap(position, index);
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{QuestionId, QuestionPoolRevisionNumber, QuestionPoolRevisionReference};

    fn pool_revision() -> QuestionPoolRevisionReference {
        QuestionPoolRevisionReference {
            question_pool_id: "0000-4000".parse::<QuestionId>().expect("Pool ID"),
            revision_number: QuestionPoolRevisionNumber::new(1).expect("revision"),
        }
    }

    fn pool_item(value: u32) -> QuestionPoolSelectedItem {
        QuestionPoolSelectedItem {
            pool_revision_member: PoolRevisionMemberReference {
                question_pool_revision: pool_revision(),
                member_position: value,
            },
            reference: QuestionRevisionReference {
                question_id: "0000-4000".parse::<QuestionId>().expect("question ID"),
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
            random_selected_order: false,
            candidates: vec![pool_item(1), pool_item(2), pool_item(3)],
            question_pool_revision: pool_revision(),
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
            random_selected_order: true,
            candidates: vec![pool_item(1), pool_item(2), pool_item(3)],
            question_pool_revision: pool_revision(),
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
