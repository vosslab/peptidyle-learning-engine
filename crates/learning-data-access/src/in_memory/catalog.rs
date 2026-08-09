use super::*;

#[async_trait]
impl CatalogStore for MemoryStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
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
                if family == "flat_single_choice_v1" =>
            {
                Some(promotion)
            }
            (question_model::DraftQuestionSource::Native { family }, None)
                if family == "flat_single_choice_v1" =>
            {
                return Err(StoreError::InvalidRecord(
                    "flat-question publication requires dedicated committed staging evidence"
                        .to_string(),
                ));
            }
            (_, Some(_)) => {
                return Err(StoreError::InvalidRecord(
                    "flat-question promotion requires the flat_single_choice_v1 native family"
                        .to_string(),
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
            Some(_) => return Err(StoreError::Conflict),
            None => return Err(StoreError::NotFound),
        }
        if state.draft_revisions.get(&draft_key).copied() != Some(command.expected_revision) {
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
            (Some(stored_grading.clone()), published_origin)
        } else {
            (None, None)
        };
        let publication = command.publication;
        if state
            .published
            .contains_key(&(publication.problem, publication.version))
            || state
                .published_flat_import_origins
                .contains_key(&(publication.problem, publication.version))
        {
            return Err(StoreError::AlreadyExists);
        }

        let (authors, previous_version, derived_from, public_id, version_number) =
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
                if !base.authors.contains(&command.publisher) {
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
                    base.public_id,
                    ProblemVersionNumber::new(
                        base.version_number.value().checked_add(1).ok_or_else(|| {
                            StoreError::Unavailable(
                                "problem version number limit reached".to_string(),
                            )
                        })?,
                    )
                    .expect("incremented problem version remains positive"),
                )
            } else {
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
                (
                    vec![command.publisher],
                    None,
                    command.expected_draft.derived_from,
                    ProblemPublicId::new(state.next_problem_public_id)
                        .expect("incremented public ID remains positive"),
                    ProblemVersionNumber::new(1).expect("one is positive"),
                )
            };

        let question = question_model::QuestionDefinition::from_draft(
            command.expected_draft.question.clone(),
            publication.problem,
            publication.version,
            command.published_source.clone(),
        );
        let record = PublishedProblemRecord {
            problem: publication.problem,
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
            .published
            .insert((record.problem, record.version), record.clone());
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
        let state = self.read_state()?;
        Ok(state
            .published
            .values()
            .filter(|record| {
                record.public_id == reference.problem
                    && record.lifecycle.is_assignable()
                    && catalog_record_visible(&state, context.tenant_id(), record)
                    && reference
                        .version
                        .is_none_or(|version| record.version_number == version)
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
