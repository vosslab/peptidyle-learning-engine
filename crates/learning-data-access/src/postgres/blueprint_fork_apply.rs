//! Atomic selective Blueprint fork saves from trusted exact current trees.

use question_model::blueprint_course::apply_blueprint_fork;
use question_model::{
    BlueprintAssessmentReference, BlueprintModuleReference, RenameBlueprintCourseInput,
    RequestChecksum,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Row, types::Json};
use std::collections::BTreeMap;

use super::blueprint_course::{
    PostgresBlueprintCourseStore, current_actor, decode_revision_content,
};
use super::connection::map_sqlx_error;
use crate::{
    ApplyBlueprintForkInput, ApplyBlueprintForkResult, SessionTokenHash, StoreError,
    StoredBlueprintCourseContent, StoredBlueprintModule,
};

impl PostgresBlueprintCourseStore {
    pub(super) async fn apply_fork_in_transaction(
        &self,
        session: SessionTokenHash,
        input: ApplyBlueprintForkInput,
    ) -> Result<ApplyBlueprintForkResult, StoreError> {
        let encoded_request = serde_json::to_vec(&input)
            .map_err(|_| StoreError::InvalidRecord("Blueprint fork selection".into()))?;
        let mut digest = Sha256::new();
        digest.update(b"ple:blueprint-fork-apply:");
        digest.update(encoded_request);
        let checksum = RequestChecksum::from_bytes(digest.finalize().into());
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 8.2.2, 8.3.1, 2.3.3: lock and authorize both sides, then
        // validate all four current preconditions before ordinary Save replay.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.load_blueprint_fork_apply_sources($1, $2, $3, $4, $5, $6)",
        )
        .bind(input.expected_source.reference.as_string())
        .bind(i64::try_from(input.expected_source.revision.value()).map_err(|_| invalid())?)
        .bind(input.expected_source_metadata_etag.into_uuid())
        .bind(input.expected_fork.reference.as_string())
        .bind(i64::try_from(input.expected_fork.revision.value()).map_err(|_| invalid())?)
        .bind(input.expected_fork_metadata_etag.into_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if rows.len() != 2 {
            return Err(StoreError::Forbidden);
        }
        let mut trees = Vec::new();
        for row in &rows {
            let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
            trees.push(decode_revision_content(
                content,
                &row.try_get::<Vec<u8>, _>("content_checksum")
                    .map_err(map_sqlx_error)?,
            )?);
        }
        let source = &trees[0];
        let fork = &trees[1];
        // Local IDs belong to their containing Blueprint. Only explicit new
        // copies allocate IDs, after authorization, locks, and all four CAS checks.
        let mut new_modules = BTreeMap::new();
        for copy in &input.selection.source_module_labels {
            if copy.target_module_reference.is_none()
                && new_modules
                    .insert(
                        copy.source_module_reference,
                        BlueprintModuleReference::from_uuid(random_uuid()?),
                    )
                    .is_some()
            {
                return Err(invalid());
            }
        }
        let mut new_assessments = BTreeMap::new();
        for copy in &input.selection.source_assessments {
            if copy.target_assessment_reference.is_none()
                && new_assessments
                    .insert(
                        copy.source_assessment_reference,
                        BlueprintAssessmentReference::from_uuid(random_uuid()?),
                    )
                    .is_some()
            {
                return Err(invalid());
            }
        }
        let applied = apply_blueprint_fork(
            &source.to_domain()?,
            &fork.to_domain()?,
            &input.selection,
            &new_modules,
            &new_assessments,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut copied_assessments = BTreeMap::new();
        for copy in &input.selection.source_assessments {
            let target_reference = copy
                .target_assessment_reference
                .or_else(|| {
                    new_assessments
                        .get(&copy.source_assessment_reference)
                        .copied()
                })
                .ok_or_else(invalid)?;
            if copied_assessments
                .insert(target_reference, copy.source_assessment_reference)
                .is_some()
            {
                return Err(invalid());
            }
        }
        // Preserve authored source content until selection validation completes.
        let mut modules = Vec::new();
        for module in applied.modules() {
            let mut assessments = Vec::new();
            for assessment in module.assessments() {
                let target_reference = assessment.blueprint_assessment_reference();
                let (tree, stored_reference) =
                    if let Some(source_reference) = copied_assessments.get(&target_reference) {
                        (source, *source_reference)
                    } else {
                        (fork, target_reference)
                    };
                let mut stored = tree
                    .modules
                    .iter()
                    .flat_map(|module| &module.assessments)
                    .find(|stored| stored.blueprint_assessment_reference == stored_reference)
                    .ok_or_else(invalid)?
                    .clone();
                stored.blueprint_assessment_reference = target_reference;
                assessments.push(stored);
            }
            modules.push(StoredBlueprintModule {
                blueprint_module_reference: module.blueprint_module_reference(),
                label: module.label().to_owned(),
                assessments,
            });
        }
        let mut content = StoredBlueprintCourseContent { modules };
        if content.to_domain()? != applied {
            return Err(invalid());
        }
        // Only explicit source copies import fresh Assessment-owned Pools;
        // untouched target Assessments keep their exact existing Pool pins.
        let replay = sqlx::query("SELECT * FROM ple_api.blueprint_pool_write_receipt($1,$2)")
            .bind(input.expected_fork.reference.as_string())
            .bind(checksum.into_bytes().to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .is_some();
        if !replay {
            for module in &mut content.modules {
                for assessment in &mut module.assessments {
                    if copied_assessments.contains_key(&assessment.blueprint_assessment_reference) {
                        let mut copied = StoredBlueprintCourseContent {
                            modules: vec![StoredBlueprintModule {
                                blueprint_module_reference: module.blueprint_module_reference,
                                label: module.label.clone(),
                                assessments: vec![assessment.clone()],
                            }],
                        };
                        super::blueprint_pools::materialize_imported_pools(
                            &mut transaction,
                            &mut copied,
                            self.pool_id_issuer.as_deref(),
                        )
                        .await?;
                        *assessment = copied.modules.remove(0).assessments.remove(0);
                    }
                }
            }
        }
        let daughters =
            sqlx::query("SELECT course_id FROM ple_api.list_blueprint_daughter_course_ids($1)")
                .bind(input.expected_fork.reference.as_string())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let actor = current_actor(&mut transaction).await?;
        let save = self
            .save_trusted_content(
                &mut transaction,
                input.expected_fork.reference,
                input.expected_fork.revision,
                checksum,
                actor,
                fork,
                &content,
                daughters,
            )
            .await?;
        let row = &rows[if input.source_short_name { 0 } else { 1 }];
        let short_name = row.try_get("short_name").map_err(map_sqlx_error)?;
        let row = &rows[if input.source_long_name { 0 } else { 1 }];
        let long_name = row.try_get("long_name").map_err(map_sqlx_error)?;
        let metadata = self
            .rename_in_transaction(
                &mut transaction,
                input.expected_fork.reference,
                input.expected_fork_metadata_etag,
                RenameBlueprintCourseInput {
                    short_name,
                    long_name,
                },
            )
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ApplyBlueprintForkResult { save, metadata })
    }
}

fn invalid() -> StoreError {
    StoreError::InvalidRecord("Blueprint fork selection".into())
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Blueprint fork apply UUID randomness unavailable".to_string())
    })
}
