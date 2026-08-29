use super::*;

mod search;

const MAX_CATALOG_USAGE_SNAPSHOT_ROWS: usize = 5_000;
const MAX_CATALOG_USAGE_SNAPSHOTS_PER_ACTOR: usize = 8;

#[derive(Debug, Clone)]
pub(super) struct CatalogUsageSnapshot {
    tenant: TenantId,
    actor: UserId,
    created_at_millis: u64,
    expires_at_millis: u64,
    instructor_courses: BTreeSet<CourseId>,
    publications: BTreeSet<(ProblemId, VersionId)>,
}

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

        if state.published.len() as u64 >= question_model::MAX_QUESTION_ID_COUNT {
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
        if state
            .published
            .keys()
            .any(|(_, version)| *version == publication.version)
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
        let question_id = (0..64)
            .map(|_| self.question_ids.issue())
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
            version: publication.version,
            question,
            capabilities: command.capabilities,
            scope: command.scope,
            lifecycle: CatalogLifecycle::Published,
            author_ids: vec![command.publisher],
            byline: command.byline.clone(),
            derived_from: command.expected_draft.derived_from,
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
        let catalog_sequence = state.next_catalog_publication_sequence;
        state.next_catalog_publication_sequence = state
            .next_catalog_publication_sequence
            .checked_add(1)
            .ok_or_else(|| {
                StoreError::Unavailable("catalog publication sequence exhausted".to_string())
            })?;
        state
            .catalog_publication_sequences
            .insert((record.problem, record.version), catalog_sequence);
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
        self.catalog_resolution_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if !self.question_ids.validates(&reference.question_id) {
            return Ok(None);
        }
        let state = self.read_state()?;
        Ok(state
            .published
            .values()
            .find(|record| {
                record.question_id == reference.question_id
                    && record.lifecycle.is_resolvable_by_stable_question_id()
                    && catalog_record_visible(&state, context.tenant_id(), record)
            })
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
        session: SessionTokenHash,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        search::search_catalog(self, context, session, query).await
    }

    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        let state = self.read_state()?;
        let actor = catalog_search_actor(&state, context, session)?;
        let Some(record) = state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
        else {
            return Ok(None);
        };
        let prompt = crate::catalog_prompt::catalog_prompt_projection(&record.question)?;
        Ok(Some(CatalogProblemDetail {
            summary: record.summary(),
            prompt,
            evidence: catalog_discovery_evidence(
                &state,
                (reference.problem, reference.version),
                state_catalog_snapshot_boundary(&state),
            )
            .0,
            usage: catalog_usage_detail(&state, context.tenant_id(), actor, reference),
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
        if !record.author_ids.contains(&actor) {
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

fn catalog_usage_detail(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    reference: ProblemVersionRef,
) -> CatalogUsageDetail {
    let assignment_uses = state
        .assignments
        .values()
        .filter(|assignment| assignment.tenant == tenant)
        .filter(|assignment| assignment_references(assignment, reference))
        .collect::<Vec<_>>();
    let institution_courses = assignment_uses
        .iter()
        .map(|assignment| assignment.course_id)
        .collect::<BTreeSet<_>>();
    let own_course_ids = state
        .course_memberships
        .values()
        .filter(|membership| {
            membership.tenant == tenant
                && membership.user == actor
                && membership.role == CourseMembershipRole::Instructor
                && membership.status == CourseMemberStatus::Active
                && super::course_records_accessible(state, tenant, membership.course)
        })
        .map(|membership| membership.course)
        .collect::<BTreeSet<_>>();
    let mut own_courses = institution_courses
        .iter()
        .filter(|course| own_course_ids.contains(course))
        .filter_map(|course| {
            let record = state.courses.get(&(tenant, *course))?;
            let reference = state.course_references.get(&(tenant, *course)).copied()?;
            let assignment_count = assignment_uses
                .iter()
                .filter(|assignment| assignment.course_id == *course)
                .count() as u64;
            Some(CatalogOwnCourseUsage {
                course: reference,
                title: record.title.clone(),
                assignment_count,
            })
        })
        .collect::<Vec<_>>();
    own_courses.sort_by_key(|usage| usage.course);
    let own_course_count = own_courses.len() as u64;
    let own_courses_truncated = own_courses.len() > MAX_CATALOG_OWN_COURSE_USAGES;
    own_courses.truncate(MAX_CATALOG_OWN_COURSE_USAGES);
    let own_assignment_count = assignment_uses
        .iter()
        .filter(|assignment| own_course_ids.contains(&assignment.course_id))
        .count() as u64;
    CatalogUsageDetail {
        summary: CatalogUsageSummary {
            institution_course_count: institution_courses.len() as u64,
            institution_assignment_count: assignment_uses.len() as u64,
            own_course_count,
            own_assignment_count,
        },
        own_courses,
        own_courses_truncated,
    }
}

fn catalog_search_actor(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    let subject =
        super::sessions::active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    if subject
        .roles()
        .contains(&question_model::UserRole::Sysadmin)
    {
        return Ok(subject.user());
    }
    if !subject
        .roles()
        .contains(&question_model::UserRole::Instructor)
    {
        return Err(StoreError::Forbidden);
    }
    let approval = state
        .instructor_approvals
        .get(&subject.user())
        .ok_or(StoreError::Forbidden)?;
    domain::teaching_authority::validate_instructor_approval(
        &approval.approval,
        state.authoritative_time,
    )
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid instructor approval: {error:?}"))
    })?;
    if approval.approval.user != subject.user() || approval.approval.revoked_at.is_some() {
        return Err(StoreError::Forbidden);
    }
    Ok(subject.user())
}

fn catalog_usage_snapshot_values(
    state: &State,
    tenant: TenantId,
    actor: UserId,
) -> (BTreeSet<(ProblemId, VersionId)>, BTreeSet<CourseId>) {
    let own_course_ids = state
        .course_memberships
        .values()
        .filter(|membership| {
            membership.tenant == tenant
                && membership.user == actor
                && membership.role == CourseMembershipRole::Instructor
                && membership.status == CourseMemberStatus::Active
                && super::course_records_accessible(state, tenant, membership.course)
        })
        .map(|membership| membership.course)
        .collect::<BTreeSet<_>>();
    let mut used = BTreeSet::new();
    let mut used_courses = BTreeSet::new();
    for assignment in state.assignments.values().filter(|assignment| {
        assignment.tenant == tenant && own_course_ids.contains(&assignment.course_id)
    }) {
        let mut assignment_has_active_reference = false;
        for item in assignment
            .items
            .iter()
            .filter(|item| item.delivery_state == AssignmentDeliveryState::Active)
        {
            used.insert((item.reference.problem, item.reference.version));
            assignment_has_active_reference = true;
        }
        for candidate in assignment
            .selection_groups
            .iter()
            .flat_map(|group| &group.candidates)
            .filter(|candidate| candidate.delivery_state == AssignmentDeliveryState::Active)
        {
            used.insert((candidate.reference.problem, candidate.reference.version));
            assignment_has_active_reference = true;
        }
        if assignment_has_active_reference {
            used_courses.insert(assignment.course_id);
        }
    }
    (used, used_courses)
}

fn catalog_snapshot_courses_are_authorized(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    courses: &BTreeSet<CourseId>,
) -> bool {
    courses.iter().all(|course| {
        state.courses.contains_key(&(tenant, *course))
            && state.course_memberships.values().any(|membership| {
                membership.tenant == tenant
                    && membership.course == *course
                    && membership.user == actor
                    && membership.role == CourseMembershipRole::Instructor
                    && membership.status == CourseMemberStatus::Active
            })
    })
}

fn catalog_usage_snapshot_token(
    fingerprint: &str,
    tenant: TenantId,
    actor: UserId,
    expires_at_millis: u64,
    publications: &BTreeSet<(ProblemId, VersionId)>,
    instructor_courses: &BTreeSet<CourseId>,
) -> [u8; 32] {
    let mut canonical = String::new();
    canonical.push_str(fingerprint);
    canonical.push('|');
    canonical.push_str(&tenant.as_uuid().to_string());
    canonical.push('|');
    canonical.push_str(&actor.as_uuid().to_string());
    canonical.push('|');
    canonical.push_str(&expires_at_millis.to_string());
    for (problem, version) in publications {
        canonical.push('|');
        canonical.push_str(&problem.as_uuid().to_string());
        canonical.push('/');
        canonical.push_str(&version.as_uuid().to_string());
    }
    for course in instructor_courses {
        canonical.push('|');
        canonical.push_str(&course.as_uuid().to_string());
    }
    *objects::Sha256Digest::compute(canonical.as_bytes()).as_bytes()
}

fn assignment_references(assignment: &AssignmentRecord, reference: ProblemVersionRef) -> bool {
    assignment.items.iter().any(|item| {
        item.delivery_state == AssignmentDeliveryState::Active && item.reference == reference
    }) || assignment.selection_groups.iter().any(|group| {
        group.candidates.iter().any(|candidate| {
            candidate.delivery_state == AssignmentDeliveryState::Active
                && candidate.reference == reference
        })
    })
}

pub(super) fn state_catalog_snapshot_boundary(state: &State) -> u64 {
    state.next_catalog_publication_sequence.saturating_sub(1)
}

pub(super) fn catalog_discovery_evidence(
    state: &State,
    reference: (ProblemId, VersionId),
    snapshot_boundary: u64,
) -> (CatalogDiscoveryEvidence, i64) {
    state
        .catalog_discovery_evidence_revisions
        .get(&reference)
        .and_then(|revisions| {
            revisions
                .iter()
                .rev()
                .find(|revision| revision.sequence <= snapshot_boundary)
        })
        .map(|revision| (revision.evidence.clone(), revision.quality))
        .unwrap_or((CatalogDiscoveryEvidence::InsufficientEvidence, 0))
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
pub(super) fn catalog_search_fingerprint(query: &CatalogSearchQuery, actor: UserId) -> String {
    let mut canonical = String::new();
    canonical.push_str(query.text.as_deref().unwrap_or(""));
    canonical.push('\u{1f}');
    for byline in &query.bylines {
        canonical.push_str(byline);
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for backend in &query.backends {
        canonical.push_str(&format!("{backend:?}"));
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for tag in &query.tags {
        canonical.push_str(tag);
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for response_family in &query.response_families {
        canonical.push_str(&format!("{response_family:?}"));
        canonical.push('\u{1f}');
    }
    canonical.push('|');
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
    canonical.push_str(&format!("{:?}", query.evidence));
    canonical.push('|');
    canonical.push_str(&format!("{:?}", query.used_in_my_courses));
    canonical.push('|');
    canonical.push_str(&format!("{:?}", query.authorship));
    canonical.push('|');
    canonical.push_str(&actor.as_uuid().to_string());
    Sha256Digest::compute(canonical.as_bytes()).to_string()
}

fn catalog_response_family_key(
    response_family: question_model::CatalogResponseFamily,
) -> &'static str {
    match response_family {
        question_model::CatalogResponseFamily::Numeric => "numeric",
        question_model::CatalogResponseFamily::MultipleChoice => "multiple_choice",
        question_model::CatalogResponseFamily::ShortText => "short_text",
        question_model::CatalogResponseFamily::MultiBlank => "multi_blank",
        question_model::CatalogResponseFamily::Matching => "matching",
        question_model::CatalogResponseFamily::Ordering => "ordering",
        question_model::CatalogResponseFamily::Hotspot => "hotspot",
        question_model::CatalogResponseFamily::FileUpload => "file_upload",
        question_model::CatalogResponseFamily::ExternalTool => "external_tool",
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
