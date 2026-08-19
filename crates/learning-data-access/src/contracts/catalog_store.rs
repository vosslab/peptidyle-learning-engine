use super::*;

/// Catalog operations that require visibility, ownership, and atomic publish.
#[async_trait]
pub trait CatalogStore: Send + Sync {
    /// Validates the stored draft expectation and atomically publishes it.
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError>;

    /// Resolves an exact visible version, including deprecated or archived ones.
    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Resolves a visible Question ID to its exact immutable publication under
    /// the caller's authorization.
    ///
    /// A Question ID identifies one publication. This lookup neither follows
    /// a successor nor selects a latest version.
    async fn resolve_catalog_problem(
        &self,
        context: TenantContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists discoverable hot metadata in stable cursor order.
    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError>;

    /// Lists distinct controlled taxonomy terms in stable cursor order.
    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError>;

    /// Searches hot discoverable metadata and returns rows plus server-side
    /// facets from one normalized-query snapshot. Implementations must reject
    /// a cursor issued for a different normalized query and must never load
    /// `problem_version_payload` merely to browse or aggregate.
    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError>;

    /// Returns a safe exact immutable catalog-detail projection. This default
    /// retains compatibility for focused test stores while production stores
    /// may use a hot metadata projection instead of loading source bindings.
    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        Ok(self
            .get_catalog_problem(context, reference)
            .await?
            .map(|record| CatalogProblemDetail {
                summary: record.summary(),
                prompt: record.question.prompt,
                statistics: question_model::CatalogStatisticsStatus::Unavailable,
            }))
    }

    /// Applies an author-owned, one-way post-publication transition.
    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError>;
}

/// Private catalog bridge from an exact visible version to its source bytes.
///
/// This trait is intentionally not part of any browser DTO or public asset
/// delivery API. A foreign tenant receives `None` before an object store is
/// consulted, which keeps source-object existence tenant-isolated.
#[async_trait]
pub trait CatalogSourceStore: Send + Sync {
    /// Resolves the exact source binding for one visible immutable version.
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError>;
}
