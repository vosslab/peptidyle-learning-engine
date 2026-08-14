use super::*;
use crate::{OwnerCorrectionAuthority, OwnerCorrectionStore};

#[async_trait]
impl CatalogStore for MemoryStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        if command.expected_draft.revises.is_some() && context.owner_correction_session().is_none()
        {
            return Err(StoreError::Forbidden);
        }
        ensure_tenant(context, command.expected_draft.tenant)?;
        validate_draft(&command.expected_draft)?;
        crate::validate_publication_source(&command.expected_draft, &command.published_source)?;
        crate::validate_source_artifact_for_publication(
            command.publication,
            &command.published_source,
            command.source_artifact.as_ref(),
            command.flat_question_promotion.is_some(),
        )?;
        let qti_promotion = match (
            &command.expected_draft.question.source,
            command.qti_promotion.as_ref(),
        ) {
            (question_model::DraftQuestionSource::Qti { .. }, Some(promotion)) => Some(promotion),
            (question_model::DraftQuestionSource::Qti { .. }, None) | (_, Some(_)) => {
                return Err(StoreError::InvalidRecord(
                    "QTI publication requires dedicated committed staging evidence".to_string(),
                ));
            }
            (_, None) => None,
        };
        let flat_promotion = match (
            &command.expected_draft.question.source,
            command.flat_question_promotion.as_ref(),
        ) {
            (question_model::DraftQuestionSource::Native { family }, Some(promotion))
                if grading::flat_question::is_flat_question_family(family) =>
            {
                Some(promotion)
            }
            (question_model::DraftQuestionSource::Native { family }, None)
                if grading::flat_question::is_flat_question_family(family) =>
            {
                return Err(StoreError::InvalidRecord(
                    "flat-question publication requires dedicated committed staging evidence"
                        .to_string(),
                ));
            }
            (_, Some(_)) => {
                return Err(StoreError::InvalidRecord(
                    "flat-question promotion requires a supported native flat family".to_string(),
                ));
            }
            (_, None) => None,
        };
        if (qti_promotion.is_some() || flat_promotion.is_some()) && command.publisher != actor {
            return Err(StoreError::InvalidRecord(
                "publication promotion actor must be the authenticated publisher".to_string(),
            ));
        }
        if qti_promotion.is_some() && flat_promotion.is_some() {
            return Err(StoreError::InvalidRecord(
                "publication cannot contain both QTI and flat-question promotion".to_string(),
            ));
        }
        let has_pending_public_assets = command.scope == PublicationScope::Public
            && (qti_promotion.is_some_and(|promotion| !promotion.assets.is_empty())
                || flat_promotion.is_some_and(|promotion| !promotion.assets.is_empty()));
        let mut state = self.write_state()?;
        let draft_key = (
            context.tenant_id(),
            command.expected_draft.question.workspace,
        );
        if command.publisher != actor
            || state.draft_access.get(&(
                context.tenant_id(),
                command.expected_draft.question.workspace,
                actor,
            )) != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        match state.drafts.get(&draft_key) {
            Some(stored) if stored == &command.expected_draft => {}
            Some(_) => {
                #[cfg(test)]
                eprintln!("flat memory publication stored draft mismatch");
                return Err(StoreError::Conflict);
            }
            None => return Err(StoreError::NotFound),
        }
        if state.draft_revisions.get(&draft_key).copied() != Some(command.expected_revision) {
            #[cfg(test)]
            eprintln!("flat memory publication revision mismatch");
            return Err(StoreError::Conflict);
        }
        let qti_grading = if let Some(promotion) = qti_promotion {
            let registry = state
                .qti_imports
                .get(&(
                    promotion.staging.tenant,
                    promotion.staging.workspace,
                    promotion.staging.import,
                ))
                .ok_or(StoreError::NotFound)?;
            validate_qti_publication_promotion(context, &command, promotion, registry)?;
            let question_model::DraftQuestionSource::Qti { item_id, .. } =
                &command.expected_draft.question.source
            else {
                unreachable!("QTI promotion was matched against a QTI draft");
            };
            let material = state
                .qti_grading
                .get(&(
                    promotion.staging.tenant,
                    promotion.staging.workspace,
                    promotion.staging.import,
                    item_id.clone(),
                ))
                .cloned()
                .ok_or(StoreError::Conflict)?;
            for asset in &promotion.assets {
                if state.asset_deliveries.contains_key(&asset.id)
                    || state
                        .asset_deliveries
                        .values()
                        .any(|existing| existing.object.id == asset.object.id)
                {
                    return Err(StoreError::AlreadyExists);
                }
            }
            Some((item_id.clone(), material))
        } else {
            None
        };
        let (flat_grading, published_flat_import_origin) = if let Some(promotion) = flat_promotion {
            let staged_source = state
                .flat_question_sources
                .get(&(
                    context.tenant_id(),
                    command.expected_draft.question.workspace,
                ))
                .ok_or(StoreError::NotFound)?;
            let stored_grading = state
                .workspace_flat_question_grading
                .get(&draft_key)
                .ok_or(StoreError::Conflict)?;
            crate::validate_flat_question_publication(context, &command, staged_source)?;
            let published_grading =
                crate::publication_validation::validate_flat_question_publication_grading(
                    &command,
                    staged_source,
                    stored_grading,
                )?;
            let current_origin = state.workspace_flat_import_origins.get(&draft_key);
            let published_origin = match (current_origin, promotion.import_origin.as_ref()) {
                (Some(current), Some(import_promotion))
                    if &current.identity() == import_promotion.expected_current_origin() =>
                {
                    Some(crate::PublishedFlatImportOrigin::from_current(
                        current,
                        command.publication,
                        import_promotion.published_archive().clone(),
                    )?)
                }
                (None, None) => None,
                _ => return Err(StoreError::Conflict),
            };
            (Some(published_grading), published_origin)
        } else {
            (None, None)
        };
        let publication = command.publication;
        if let Some(promotion) = flat_promotion {
            for asset in &promotion.assets {
                if state.asset_deliveries.contains_key(&asset.id)
                    || state
                        .asset_deliveries
                        .values()
                        .any(|existing| existing.object.id == asset.object.id)
                {
                    return Err(StoreError::AlreadyExists);
                }
            }
        }
        if state
            .published
            .contains_key(&(publication.problem, publication.version))
            || state
                .published_flat_import_origins
                .contains_key(&(publication.problem, publication.version))
        {
            return Err(StoreError::AlreadyExists);
        }

        let (authors, previous_version, derived_from, question_id, public_id, version_number) =
            if let Some(revises) = command.expected_draft.revises {
                if publication.problem != revises.problem {
                    return Err(StoreError::InvalidRecord(
                        "revision must remain in its existing problem chain".to_string(),
                    ));
                }
                let base = state
                    .published
                    .get(&(revises.problem, revises.version))
                    .ok_or(StoreError::NotFound)?;
                if !catalog_record_visible(&state, context.tenant_id(), base)
                    || state.problem_owner_tenants.get(&revises.problem)
                        != Some(&context.tenant_id())
                {
                    return Err(StoreError::NotFound);
                }
                if base.scope != command.scope
                    || !matches!(base.lifecycle, CatalogLifecycle::Published)
                {
                    return Err(StoreError::Forbidden);
                }
                if !base.authors.contains(&command.publisher) {
                    return Err(StoreError::Forbidden);
                }
                if state.problem_owner_users.get(&revises.problem) != Some(&command.publisher) {
                    return Err(StoreError::Forbidden);
                }
                if state.published.values().any(|record| {
                    record.problem == revises.problem
                        && record.previous_version == Some(revises.version)
                }) {
                    return Err(StoreError::Conflict);
                }
                (
                    base.authors.clone(),
                    Some(revises.version),
                    base.derived_from,
                    base.question_id.clone(),
                    base.public_id,
                    base.version_number
                        .value()
                        .checked_add(1)
                        .and_then(|value| ProblemVersionNumber::new(u64::from(value)))
                        .ok_or_else(|| {
                            StoreError::Unavailable(
                                "problem version number limit reached".to_string(),
                            )
                        })?,
                )
            } else {
                if state.problem_owner_tenants.len() as u64 >= question_model::MAX_QUESTION_ID_COUNT
                {
                    return Err(StoreError::Unavailable(
                        "Question ID product limit reached".to_string(),
                    ));
                }
                if state
                    .published
                    .keys()
                    .any(|(problem, _)| *problem == publication.problem)
                {
                    return Err(StoreError::AlreadyExists);
                }
                if let Some(source) = command.expected_draft.derived_from {
                    let source_record = state
                        .published
                        .get(&(source.problem, source.version))
                        .ok_or(StoreError::NotFound)?;
                    if !catalog_record_visible(&state, context.tenant_id(), source_record) {
                        return Err(StoreError::NotFound);
                    }
                }
                state.next_problem_public_id =
                    state.next_problem_public_id.checked_add(1).ok_or_else(|| {
                        StoreError::Unavailable("problem public ID limit reached".to_string())
                    })?;
                let codec = crate::QuestionIdCodec::from_server_secret([0x42; 32]);
                let question_id = (0..64)
                    .map(|_| codec.issue())
                    .find_map(|candidate| match candidate {
                        Ok(candidate)
                            if !state
                                .published
                                .values()
                                .any(|record| record.question_id == candidate) =>
                        {
                            Some(Ok(candidate))
                        }
                        Ok(_) => None,
                        Err(error) => Some(Err(error)),
                    })
                    .transpose()?
                    .ok_or_else(|| {
                        StoreError::Unavailable("Question ID collision retry exhausted".to_string())
                    })?;
                (
                    vec![command.publisher],
                    None,
                    command.expected_draft.derived_from,
                    question_id,
                    ProblemPublicId::new(state.next_problem_public_id).ok_or_else(|| {
                        StoreError::Unavailable("problem public ID limit reached".to_string())
                    })?,
                    ProblemVersionNumber::new(1).expect("one is positive"),
                )
            };

        let published_draft_question = command
            .flat_question_promotion
            .as_ref()
            .map(|promotion| promotion.published_question.clone())
            .unwrap_or_else(|| command.expected_draft.question.clone());
        let question = question_model::QuestionDefinition::from_draft(
            published_draft_question,
            publication.problem,
            publication.version,
            command.published_source.clone(),
        );
        let record = PublishedProblemRecord {
            problem: publication.problem,
            question_id,
            public_id,
            version: publication.version,
            version_number,
            question,
            capabilities: command.capabilities,
            scope: command.scope,
            lifecycle: CatalogLifecycle::Published,
            authors,
            previous_version,
            derived_from,
            published_at: state.authoritative_time,
        };
        validate_published(&record)?;
        if record.scope == PublicationScope::Institution {
            state
                .catalog_grants
                .insert((context.tenant_id(), record.problem, record.version));
        }
        state
            .problem_owner_tenants
            .entry(record.problem)
            .or_insert(context.tenant_id());
        state
            .problem_owner_users
            .entry(record.problem)
            .or_insert(command.publisher);
        state
            .published
            .insert((record.problem, record.version), record.clone());
        if let Some(revises) = command.expected_draft.revises {
            let predecessor_grants = state
                .catalog_grants
                .iter()
                .filter(|(_, problem, version)| {
                    *problem == revises.problem && *version == revises.version
                })
                .map(|(tenant, _, _)| *tenant)
                .collect::<Vec<_>>();
            let previous = state
                .published
                .get_mut(&(revises.problem, revises.version))
                .expect("validated correction source remains present");
            previous.lifecycle = CatalogLifecycle::Archived {
                reason: "Superseded by an owner correction".to_string(),
            };

            let replacement = ProblemVersionRef {
                problem: record.problem,
                version: record.version,
            };
            for tenant in predecessor_grants {
                state
                    .catalog_grants
                    .insert((tenant, record.problem, record.version));
            }
            let assignment_keys = state.assignments.keys().copied().collect::<Vec<_>>();
            for key in assignment_keys {
                let assignment = state
                    .assignments
                    .get_mut(&key)
                    .expect("collected assignment key remains present");
                let mut changed = false;
                for item in &mut assignment.items {
                    if item.reference == revises {
                        item.reference = replacement;
                        changed = true;
                    }
                }
                for group in &mut assignment.selection_groups {
                    for candidate in &mut group.candidates {
                        if candidate.reference == revises {
                            candidate.reference = replacement;
                            changed = true;
                        }
                    }
                }
                if changed {
                    let revision = state.assignment_revisions.get_mut(&key).ok_or_else(|| {
                        StoreError::Unavailable("stored assignment revision is missing".to_string())
                    })?;
                    *revision = revision.next()?;
                }
            }
        }
        if let Some(artifact) = command.source_artifact {
            state
                .source_artifacts
                .insert((publication.problem, publication.version), artifact);
        }
        if let Some(promotion) = command.qti_promotion {
            for asset in promotion.assets {
                state.asset_deliveries.insert(asset.id, asset);
            }
        }
        if let Some(promotion) = command.flat_question_promotion.as_ref() {
            for asset in &promotion.assets {
                state.asset_deliveries.insert(asset.id, asset.clone());
            }
        }
        if let Some((item_id, material)) = qti_grading {
            state.published_qti_grading.insert(
                (publication.problem, publication.version, item_id),
                material,
            );
        }
        if let Some(material) = flat_grading {
            state
                .published_flat_question_grading
                .insert((record.problem, record.version), material);
        }
        if let Some(origin) = published_flat_import_origin {
            let replaced = state
                .published_flat_import_origins
                .insert((record.problem, record.version), origin);
            debug_assert!(replaced.is_none(), "published origin must be immutable");
        }
        if has_pending_public_assets {
            let job = JobId::generate()?;
            let available_at = state.authoritative_time;
            let replaced = state.jobs.insert(
                job,
                StoredJob {
                    tenant: context.tenant_id(),
                    payload: JobPayload::PublishPublicAssets {
                        reference: publication,
                    },
                    state: JobState::Ready,
                    available_at,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: 20,
                    failure: None,
                },
            );
            debug_assert!(replaced.is_none(), "generated outbox job must be unique");
        }
        state.drafts.remove(&draft_key);
        state.draft_revisions.remove(&draft_key);
        state
            .draft_access
            .retain(|(tenant, workspace, _), _| (*tenant, *workspace) != draft_key);
        state.flat_question_sources.remove(&draft_key);
        state.workspace_flat_question_grading.remove(&draft_key);
        state.workspace_flat_import_origins.remove(&draft_key);
        Ok(record)
    }

    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
            .cloned())
    }

    async fn resolve_catalog_problem(
        &self,
        context: TenantContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let codec = crate::QuestionIdCodec::from_server_secret([0x42; 32]);
        if !codec.validates(&reference.question_id) {
            return Ok(None);
        }
        let state = self.read_state()?;
        Ok(state
            .published
            .values()
            .filter(|record| {
                record.question_id == reference.question_id
                    && record.lifecycle.is_assignable()
                    && catalog_record_visible(&state, context.tenant_id(), record)
            })
            .max_by_key(|record| record.version_number)
            .cloned())
    }

    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .filter(|(_, record)| {
                record.lifecycle.is_discoverable()
                    && catalog_record_visible(&state, context.tenant_id(), record)
            })
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.summary()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError> {
        let state = self.read_state()?;
        let mut distinct = BTreeMap::new();
        for record in state.published.values().filter(|record| {
            record.lifecycle.is_discoverable()
                && catalog_record_visible(&state, context.tenant_id(), record)
        }) {
            for term in &record.question.metadata.taxonomy {
                distinct
                    .entry(taxonomy_cursor_key(term))
                    .or_insert_with(|| term.clone());
            }
        }
        Ok(page_records(distinct.into_iter().collect(), &page))
    }

    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        let query = query
            .normalized()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let page = search_page_request(&query)?;
        let fingerprint = catalog_search_fingerprint(&query);
        let after = page
            .after
            .as_ref()
            .map(|cursor| decode_catalog_search_cursor(cursor.as_str(), &fingerprint))
            .transpose()?
            .map(|(problem, version)| format!("{problem}/{version}"));
        let state = self.read_state()?;
        let matching = state
            .published
            .iter()
            .filter_map(|((problem, version), record)| {
                if !record.lifecycle.is_discoverable()
                    || !catalog_record_visible(&state, context.tenant_id(), record)
                {
                    return None;
                }
                let statistics_available = state
                    .question_statistics
                    .get(&(*problem, *version))
                    .is_some_and(|aggregate| {
                        matches!(
                            aggregate.disclose(StatisticsDisclosurePolicy::default()),
                            QuestionStatisticsDisclosure::Available(_)
                        )
                    });
                catalog_search_matches(record, &query, statistics_available)
                    .then(|| (format!("{problem}/{version}"), record, statistics_available))
            })
            .collect::<Vec<_>>();
        let facets = catalog_search_facets(
            matching
                .iter()
                .map(|(_, record, available)| (*record, *available)),
        );
        let mut selected = matching
            .into_iter()
            .filter(|(key, _, _)| after.as_ref().is_none_or(|cursor| key > cursor))
            .take(usize::from(page.size.get()) + 1)
            .collect::<Vec<_>>();
        let has_more = selected.len() > usize::from(page.size.get());
        if has_more {
            selected.pop();
        }
        let next_cursor = if has_more {
            selected.last().map(|(_, record, _)| {
                encode_catalog_search_cursor(
                    &fingerprint,
                    record.problem.as_uuid(),
                    record.version.as_uuid(),
                )
            })
        } else {
            None
        };
        Ok(CatalogSearchPage {
            items: selected
                .into_iter()
                .map(|(_, record, _)| record.summary())
                .collect(),
            next_cursor,
            facets,
        })
    }

    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
            .map(|record| {
                let statistics = state
                    .question_statistics
                    .get(&(reference.problem, reference.version))
                    .map(|aggregate| aggregate.disclose(StatisticsDisclosurePolicy::default()))
                    .unwrap_or(QuestionStatisticsDisclosure::Suppressed);
                CatalogProblemDetail {
                    summary: record.summary(),
                    prompt: record.question.prompt.clone(),
                    statistics: match statistics {
                        QuestionStatisticsDisclosure::Suppressed => {
                            question_model::CatalogStatisticsStatus::Unavailable
                        }
                        QuestionStatisticsDisclosure::Available(view) => {
                            question_model::CatalogStatisticsStatus::Available(view)
                        }
                    },
                }
            }))
    }

    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError> {
        let mut state = self.write_state()?;
        let key = (reference.problem, reference.version);
        let visible = state
            .published
            .get(&key)
            .is_some_and(|record| catalog_record_visible(&state, context.tenant_id(), record));
        if !visible
            || state.problem_owner_tenants.get(&reference.problem) != Some(&context.tenant_id())
        {
            return Err(StoreError::NotFound);
        }
        let record = state.published.get_mut(&key).ok_or(StoreError::NotFound)?;
        if !record.authors.contains(&actor) {
            return Err(StoreError::Forbidden);
        }
        record.lifecycle = match (&record.lifecycle, transition) {
            (CatalogLifecycle::Published, CatalogTransition::Deprecate { reason }) => {
                let reason = validated_deprecation_reason(reason)?;
                CatalogLifecycle::Deprecated { reason }
            }
            (CatalogLifecycle::Deprecated { reason }, CatalogTransition::Archive) => {
                CatalogLifecycle::Archived {
                    reason: reason.clone(),
                }
            }
            _ => {
                return Err(StoreError::InvalidRecord(
                    "catalog lifecycle transition is not allowed".to_string(),
                ));
            }
        };
        Ok(record.clone())
    }
}

#[async_trait]
impl OwnerCorrectionStore for MemoryStore {
    async fn publish_owner_correction(
        &self,
        context: TenantContext,
        authority: OwnerCorrectionAuthority,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        if command.expected_draft.revises.is_none() || authority.actor != command.publisher {
            return Err(StoreError::Forbidden);
        }
        let authorized = {
            let state = self.read_state()?;
            let Some(active) = super::sessions::active_subject(&state, context, authority.session)
            else {
                return Err(StoreError::Forbidden);
            };
            active.user() == authority.actor
                && active
                    .roles()
                    .iter()
                    .any(|role| matches!(role, question_model::UserRole::Instructor))
        };
        if !authorized {
            return Err(StoreError::Forbidden);
        }
        CatalogStore::publish_draft(
            self,
            context.with_owner_correction_session(authority.session),
            authority.actor,
            command,
        )
        .await
    }
}

#[async_trait]
impl CatalogSourceStore for MemoryStore {
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError> {
        let state = self.read_state()?;
        let Some(published) = state.published.get(&(reference.problem, reference.version)) else {
            return Ok(None);
        };
        if !catalog_record_visible(&state, context.tenant_id(), published) {
            return Ok(None);
        }
        Ok(state
            .source_artifacts
            .get(&(reference.problem, reference.version))
            .cloned())
    }
}

pub(super) fn catalog_record_visible(
    state: &State,
    tenant: TenantId,
    record: &PublishedProblemRecord,
) -> bool {
    // Exact-version reads intentionally retain deprecated and archived content
    // for historical assignments. This is the same scope/grant predicate used
    // by PostgreSQL RLS and the published-QTI grader capability.
    record.scope == PublicationScope::Public
        || state
            .catalog_grants
            .contains(&(tenant, record.problem, record.version))
}

/// Converts the wire-owned bounded search request into the shared pagination
/// primitive.  The opaque token is checked for query binding below rather than
/// treated as an untrusted stable key.
pub(super) fn search_page_request(query: &CatalogSearchQuery) -> Result<PageRequest, StoreError> {
    let size = PageSize::new(query.page_size.unwrap_or(50))
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    match query.cursor.clone() {
        Some(cursor) => Cursor::parse(cursor)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|error| StoreError::InvalidRecord(error.to_string())),
        None => Ok(PageRequest::first(size)),
    }
}

/// Stable digest of filters only. The digest avoids exposing title/taxonomy
/// contents through a cursor and makes a cursor from a different filter set a
/// deterministic client error rather than a subtly stale page.
pub(super) fn catalog_search_fingerprint(query: &CatalogSearchQuery) -> String {
    let mut canonical = String::new();
    canonical.push_str(query.text.as_deref().unwrap_or(""));
    canonical.push('\u{1f}');
    for term in &query.taxonomy {
        canonical.push_str(&term.scheme);
        canonical.push('\u{1e}');
        canonical.push_str(&term.code);
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for capability in &query.capabilities {
        canonical.push_str(capability.as_str());
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for license in &query.licenses {
        canonical.push_str(&format!("{license:?}"));
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    canonical.push_str(&format!("{:?}", query.statistics));
    Sha256Digest::compute(canonical.as_bytes()).to_string()
}

fn catalog_record_matches_text(record: &PublishedProblemRecord, text: &str) -> bool {
    std::iter::once(record.question.metadata.title.as_str())
        .chain(record.question.metadata.language.split_whitespace())
        .chain(record.question.metadata.tags.iter().map(|tag| tag.as_str()))
        .chain(record.question.metadata.taxonomy.iter().flat_map(|term| {
            [
                term.scheme.as_str(),
                term.code.as_str(),
                term.label.as_str(),
            ]
        }))
        .any(|value| value.to_lowercase().contains(text))
}

pub(super) fn catalog_search_matches(
    record: &PublishedProblemRecord,
    query: &CatalogSearchQuery,
    statistics_available: bool,
) -> bool {
    if matches!(query.statistics, CatalogStatisticsAvailability::Available) && !statistics_available
    {
        return false;
    }
    if matches!(query.statistics, CatalogStatisticsAvailability::Unavailable)
        && statistics_available
    {
        return false;
    }
    if let Some(text) = &query.text {
        if let Some(question_id) = query.exact_question_id() {
            let codec = crate::QuestionIdCodec::from_server_secret([0x42; 32]);
            if codec.validates(&question_id) {
                if record.question_id != question_id {
                    return false;
                }
            } else if !catalog_record_matches_text(record, text) {
                return false;
            }
        } else {
            if !catalog_record_matches_text(record, text) {
                return false;
            }
        }
    }
    if !query.taxonomy.iter().all(|wanted| {
        record
            .question
            .metadata
            .taxonomy
            .iter()
            .any(|term| term.scheme == wanted.scheme && term.code == wanted.code)
    }) {
        return false;
    }
    if !query
        .capabilities
        .iter()
        .all(|capability| record.capabilities.supports(*capability))
    {
        return false;
    }
    query.licenses.is_empty()
        || query
            .licenses
            .iter()
            .any(|license| license.matches(&record.question.metadata.license))
}

fn catalog_search_facets<'a>(
    records: impl Iterator<Item = (&'a PublishedProblemRecord, bool)>,
) -> CatalogSearchFacets {
    let mut taxonomy = BTreeMap::<String, (TaxonomyTerm, u64)>::new();
    let mut capabilities = BTreeMap::new();
    let mut licenses = BTreeMap::new();
    let mut unavailable = 0_u64;
    let mut available = 0_u64;
    for (record, statistics_available) in records {
        if statistics_available {
            available += 1;
        } else {
            unavailable += 1;
        }
        for term in &record.question.metadata.taxonomy {
            let entry = taxonomy
                .entry(taxonomy_cursor_key(term))
                .or_insert_with(|| (term.clone(), 0));
            entry.1 += 1;
            // A controlled identity is `(scheme, code)`. Legacy imports may
            // disagree on display text; choose the lexicographically smallest
            // label so Memory and PostgreSQL remain deterministic.
            if term.label < entry.0.label {
                entry.0.label = term.label.clone();
            }
        }
        for capability in record.capabilities.declared() {
            *capabilities.entry(capability).or_insert(0_u64) += 1;
        }
        *licenses
            .entry(CatalogLicenseValue::from_license(
                &record.question.metadata.license,
            ))
            .or_insert(0_u64) += 1;
    }
    let mut taxonomy = taxonomy
        .into_values()
        .map(|(term, count)| CatalogTaxonomyFacet { term, count })
        .collect::<Vec<_>>();
    taxonomy.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.term.scheme.cmp(&right.term.scheme))
            .then_with(|| left.term.code.cmp(&right.term.code))
    });
    taxonomy.truncate(MAX_CATALOG_TAXONOMY_FACETS);
    CatalogSearchFacets {
        taxonomy,
        capabilities: capabilities
            .into_iter()
            .map(|(capability, count)| CatalogCapabilityFacet { capability, count })
            .collect(),
        licenses: licenses
            .into_iter()
            .map(|(license, count)| CatalogLicenseFacet { license, count })
            .collect(),
        statistics: CatalogStatisticsFacet {
            available,
            unavailable,
        },
    }
}

pub(super) fn validated_deprecation_reason(reason: String) -> Result<String, StoreError> {
    const MAX_REASON_CHARS: usize = 1_000;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(StoreError::InvalidRecord(
            "deprecation requires a nonempty reason".to_string(),
        ));
    }
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "deprecation reason must contain at most {MAX_REASON_CHARS} characters"
        )));
    }
    Ok(reason.to_string())
}

pub(super) fn taxonomy_cursor_key(term: &TaxonomyTerm) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut key = String::with_capacity((term.scheme.len() + term.code.len()) * 2 + 1);
    for byte in term.scheme.bytes() {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    key.push('/');
    for byte in term.code.bytes() {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    key
}

/// Applies stable-key cursor paging without a positional index parameter.
pub(super) fn page_records<T>(mut records: Vec<(String, T)>, request: &PageRequest) -> Page<T> {
    records.sort_by(|left, right| left.0.cmp(&right.0));
    let after = request.after.as_ref().map(Cursor::as_str);
    let mut selected: Vec<(String, T)> = records
        .into_iter()
        .filter(|(key, _)| after.is_none_or(|cursor| key.as_str() > cursor))
        .take(usize::from(request.size.get()) + 1)
        .collect();
    let has_more = selected.len() > usize::from(request.size.get());
    if has_more {
        selected.pop();
    }
    let next_cursor = if has_more {
        selected
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Page {
        items: selected.into_iter().map(|(_, item)| item).collect(),
        next_cursor,
    }
}
