use super::*;

/// Catalog operations that require visibility, ownership, and atomic publish.
#[async_trait]
pub trait CatalogStore: Send + Sync {
    /// Validates the stored draft expectation and atomically publishes it.
    async fn publish_draft(
        &self,
        context: ActorContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError>;

    /// Resolves an exact visible version, including deprecated or archived ones.
    async fn get_catalog_problem(
        &self,
        context: ActorContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Resolves a visible Question ID to its exact immutable publication under
    /// the caller's authorization.
    ///
    /// A Question ID identifies one publication. This lookup neither follows
    /// a successor nor selects a latest version.
    async fn resolve_catalog_problem(
        &self,
        context: ActorContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists discoverable hot metadata in stable cursor order.
    async fn list_catalog(
        &self,
        context: ActorContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError>;

    /// Lists distinct controlled taxonomy terms in stable cursor order.
    async fn list_catalog_taxonomy(
        &self,
        context: ActorContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError>;

    /// Searches hot discoverable metadata and returns rows plus server-side
    /// facets from one normalized-query snapshot. Implementations must reject
    /// a cursor issued for a different normalized query and must never load
    /// `problem_version_payload` merely to browse or aggregate.
    async fn search_catalog(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError>;

    /// Returns a safe exact immutable catalog-detail projection. This default
    /// retains compatibility for focused test stores while production stores
    /// may use a hot metadata projection instead of loading source bindings.
    async fn get_catalog_detail(
        &self,
        context: ActorContext,
        _session: SessionTokenHash,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        let Some(record) = self.get_catalog_problem(context, reference).await? else {
            return Ok(None);
        };
        let prompt = crate::catalog_prompt::catalog_prompt_projection(&record.question)?;
        Ok(Some(CatalogProblemDetail {
            summary: record.summary(),
            prompt,
            evidence: question_model::CatalogDiscoveryEvidence::InsufficientEvidence,
            usage: question_model::CatalogUsageDetail {
                summary: question_model::CatalogUsageSummary {
                    institution_course_count: 0,
                    institution_assignment_count: 0,
                    own_course_count: 0,
                    own_assignment_count: 0,
                },
                own_courses: Vec::new(),
                own_courses_truncated: false,
            },
        }))
    }

    /// Applies an author-owned, one-way post-publication transition.
    async fn transition_catalog_problem(
        &self,
        context: ActorContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError>;
}

/// Private catalog bridge from an exact visible version to its source bytes.
///
/// This trait is intentionally not part of any browser DTO or public asset
/// delivery API. An unauthorized actor receives `None` before an object store
/// is consulted, which keeps source-object existence concealed.
#[async_trait]
pub trait CatalogSourceStore: Send + Sync {
    /// Resolves the exact source binding for one visible immutable version.
    async fn catalog_source_artifact(
        &self,
        context: ActorContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError>;
}
