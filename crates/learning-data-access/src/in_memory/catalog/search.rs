use super::*;

pub(super) async fn search_catalog(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    query: CatalogSearchQuery,
) -> Result<CatalogSearchPage, StoreError> {
    let query = query
        .normalized()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let page = super::search_page_request(&query)?;
    let state = store.read_state()?;
    let actor = super::catalog_search_actor(&state, context, session)?;
    let fingerprint = super::catalog_search_fingerprint(&query, actor);
    let after = page
        .after
        .as_ref()
        .map(|cursor| {
            decode_catalog_search_cursor(&store.catalog_cursors, cursor.as_str(), &fingerprint)
        })
        .transpose()?
        .map(|key| {
            (
                key.snapshot_boundary,
                key.full_text_rank,
                key.similarity,
                key.quality,
                ProblemId::from_uuid(key.problem),
                VersionId::from_uuid(key.version),
                key.actor_usage_snapshot,
                key.actor_usage_snapshot_expires_at_millis,
            )
        });
    let snapshot_boundary = after
        .as_ref()
        .map(|after| after.0)
        .unwrap_or_else(|| state_catalog_snapshot_boundary(&state));
    let now_millis = u64::try_from(state.authoritative_time.as_unix_millis()).unwrap_or(0);
    let (actor_usage_snapshot, actor_usage_snapshot_expires_at_millis, used_publications) =
        if let Some(after) = after.as_ref() {
            let snapshots = store.catalog_usage_snapshots.read().map_err(|_| {
                StoreError::Unavailable("catalog usage snapshot lock failed".to_string())
            })?;
            let snapshot = snapshots.get(&after.6).ok_or_else(|| {
                StoreError::InvalidRecord(
                    "catalog actor-usage snapshot must be restarted".to_string(),
                )
            })?;
            if snapshot.tenant != context.tenant_id() || snapshot.actor != actor {
                return Err(StoreError::InvalidRecord(
                    "catalog actor-usage snapshot is not authorized".to_string(),
                ));
            }
            if now_millis >= snapshot.expires_at_millis {
                return Err(StoreError::InvalidRecord(
                    "catalog actor-usage snapshot has expired".to_string(),
                ));
            }
            if !super::catalog_snapshot_courses_are_authorized(
                &state,
                context.tenant_id(),
                actor,
                &snapshot.instructor_courses,
            ) {
                return Err(StoreError::InvalidRecord(
                    "catalog actor-usage snapshot must be restarted".to_string(),
                ));
            }
            (
                after.6,
                snapshot.expires_at_millis,
                snapshot.publications.clone(),
            )
        } else {
            let (used_publications, instructor_courses) =
                super::catalog_usage_snapshot_values(&state, context.tenant_id(), actor);
            if used_publications.len() > MAX_CATALOG_USAGE_SNAPSHOT_ROWS {
                return Err(StoreError::Unavailable(
                    "catalog actor-usage snapshot exceeds its bound".to_string(),
                ));
            }
            let expires_at_millis = now_millis.saturating_add(60_000);
            let actor_usage_snapshot = super::catalog_usage_snapshot_token(
                &fingerprint,
                context.tenant_id(),
                actor,
                expires_at_millis,
                &used_publications,
                &instructor_courses,
            );
            let snapshot = CatalogUsageSnapshot {
                tenant: context.tenant_id(),
                actor,
                created_at_millis: now_millis,
                expires_at_millis,
                instructor_courses,
                publications: used_publications.clone(),
            };
            let mut snapshots = store.catalog_usage_snapshots.write().map_err(|_| {
                StoreError::Unavailable("catalog usage snapshot lock failed".to_string())
            })?;
            snapshots.retain(|_, existing| existing.expires_at_millis > now_millis);
            let actor_snapshots = snapshots
                .iter()
                .filter(|(_, existing)| {
                    existing.tenant == context.tenant_id() && existing.actor == actor
                })
                .map(|(token, existing)| (*token, existing.created_at_millis))
                .collect::<Vec<_>>();
            if actor_snapshots.len() >= MAX_CATALOG_USAGE_SNAPSHOTS_PER_ACTOR {
                let oldest = actor_snapshots
                    .into_iter()
                    .min_by_key(|(_, created)| *created)
                    .map(|(token, _)| token);
                snapshots.retain(|token, _| Some(*token) != oldest);
            }
            snapshots.insert(actor_usage_snapshot, snapshot);
            (actor_usage_snapshot, expires_at_millis, used_publications)
        };
    let matching = state
        .published
        .iter()
        .filter_map(|((problem, version), record)| {
            if state
                .catalog_publication_sequences
                .get(&(*problem, *version))
                .is_some_and(|sequence| *sequence > snapshot_boundary)
            {
                return None;
            }
            if !record.lifecycle.is_discoverable()
                || !super::catalog_record_visible(&state, context.tenant_id(), record)
            {
                return None;
            }
            let (evidence, quality) =
                super::catalog_discovery_evidence(&state, (*problem, *version), snapshot_boundary);
            let evidence_available = matches!(evidence, CatalogDiscoveryEvidence::Available { .. });
            super::super::catalog_search::catalog_search_score(
                record,
                &query,
                &store.question_ids,
                evidence_available,
                used_publications.contains(&(*problem, *version)),
                actor,
            )
            .map(|(rank, similarity)| {
                (
                    rank, similarity, quality, *problem, *version, record, evidence,
                )
            })
        })
        .collect::<Vec<_>>();
    let facets = search_facets(matching.iter().map(
        |(_, _, _, problem, version, record, evidence)| {
            (
                *record,
                matches!(evidence, CatalogDiscoveryEvidence::Available { .. }),
                used_publications.contains(&(*problem, *version)),
            )
        },
    ));
    let mut matching = matching;
    matching.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.cmp(&right.4))
    });
    let mut selected = matching
        .into_iter()
        .filter(|(rank, similarity, quality, problem, version, _, _)| {
            after.as_ref().is_none_or(|after| {
                *rank < after.1
                    || (*rank == after.1
                        && (*similarity < after.2
                            || (*similarity == after.2
                                && (*quality < after.3
                                    || (*quality == after.3
                                        && (*problem, *version) > (after.4, after.5))))))
            })
        })
        .take(usize::from(page.size.get()) + 1)
        .collect::<Vec<_>>();
    let has_more = selected.len() > usize::from(page.size.get());
    if has_more {
        selected.pop();
    }
    let next_cursor = if has_more {
        selected
            .last()
            .map(|(rank, similarity, quality, problem, version, _, _)| {
                encode_catalog_search_cursor(
                    &store.catalog_cursors,
                    &fingerprint,
                    CatalogSearchCursorKey {
                        snapshot_boundary,
                        full_text_rank: *rank,
                        similarity: *similarity,
                        quality: *quality,
                        actor_usage_snapshot,
                        actor_usage_snapshot_expires_at_millis,
                        problem: problem.as_uuid(),
                        version: version.as_uuid(),
                    },
                )
            })
            .transpose()?
    } else {
        None
    };
    Ok(CatalogSearchPage {
        items: selected
            .into_iter()
            .map(|(_, _, _, _, _, record, evidence)| CatalogDiscoveryItem {
                summary: record.summary(),
                evidence,
            })
            .collect(),
        next_cursor,
        facets,
    })
}

fn search_facets<'a>(
    records: impl Iterator<Item = (&'a PublishedProblemRecord, bool, bool)>,
) -> CatalogSearchFacets {
    let mut bylines = BTreeMap::new();
    let mut backends = BTreeMap::new();
    let mut tags = BTreeMap::new();
    let mut response_families = BTreeMap::new();
    let mut taxonomy = BTreeMap::<String, (TaxonomyTerm, u64)>::new();
    let mut capabilities = BTreeMap::new();
    let mut licenses = BTreeMap::new();
    let mut unavailable = 0_u64;
    let mut available = 0_u64;
    let mut used = 0_u64;
    for (record, evidence_available, used_in_my_courses) in records {
        if evidence_available {
            available += 1;
        } else {
            unavailable += 1;
        }
        if used_in_my_courses {
            used += 1;
        }
        for byline in &record.byline.names {
            *bylines.entry(byline.as_str().to_string()).or_insert(0_u64) += 1;
        }
        *backends
            .entry(question_model::QuestionBackend::from(
                &record.question.source,
            ))
            .or_insert(0_u64) += 1;
        for tag in &record.question.metadata.tags {
            *tags.entry(tag.as_str().to_string()).or_insert(0_u64) += 1;
        }
        *response_families
            .entry(question_model::CatalogResponseFamily::from(
                &record.question.response,
            ))
            .or_insert(0_u64) += 1;
        for term in &record.question.metadata.taxonomy {
            let entry = taxonomy
                .entry(super::taxonomy_cursor_key(term))
                .or_insert_with(|| (term.clone(), 0));
            entry.1 += 1;
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
    let mut bylines = bylines
        .into_iter()
        .map(|(byline, count)| question_model::CatalogBylineFacet { byline, count })
        .collect::<Vec<_>>();
    bylines.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.byline.to_lowercase().cmp(&right.byline.to_lowercase()))
            .then_with(|| left.byline.cmp(&right.byline))
    });
    bylines.truncate(question_model::MAX_CATALOG_BYLINE_FACETS);
    let mut backends = backends
        .into_iter()
        .map(|(backend, count)| question_model::CatalogBackendFacet { backend, count })
        .collect::<Vec<_>>();
    backends.sort_by_key(|facet| facet.backend.as_str());
    backends.truncate(question_model::MAX_CATALOG_BACKEND_FACETS);
    let mut tags = tags
        .into_iter()
        .map(|(tag, count)| question_model::CatalogTagFacet { tag, count })
        .collect::<Vec<_>>();
    tags.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.tag.to_lowercase().cmp(&right.tag.to_lowercase()))
            .then_with(|| left.tag.cmp(&right.tag))
    });
    tags.truncate(question_model::MAX_CATALOG_TAG_FACETS);
    let mut response_families = response_families
        .into_iter()
        .map(
            |(response_family, count)| question_model::CatalogResponseFamilyFacet {
                response_family,
                count,
            },
        )
        .collect::<Vec<_>>();
    response_families
        .sort_by_key(|facet| super::catalog_response_family_key(facet.response_family));
    response_families.truncate(question_model::MAX_CATALOG_RESPONSE_FAMILY_FACETS);
    CatalogSearchFacets {
        bylines,
        backends,
        tags,
        response_families,
        taxonomy,
        capabilities: capabilities
            .into_iter()
            .map(|(capability, count)| CatalogCapabilityFacet { capability, count })
            .collect(),
        licenses: licenses
            .into_iter()
            .map(|(license, count)| CatalogLicenseFacet { license, count })
            .collect(),
        evidence: CatalogEvidenceFacet {
            available,
            unavailable,
        },
        used_in_my_courses: CatalogUsedInMyCoursesFacet { used },
    }
}
