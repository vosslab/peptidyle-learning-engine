//! Immutable catalog-only WebWork baseline for the real browser suite.
//!
//! The host publishes reviewed provider material before browser work starts.
//! It deliberately creates no teaching or learner records: instructors and
//! learners create those records through the visible PLE interface.

use super::*;

/// Browser-safe hand-off for locating the reviewed catalog item.
///
/// The receipt contains only the public Question ID and title. It excludes
/// problem/version/object identifiers, source paths, renderer configuration,
/// credentials, and answer material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WebworkCatalogBaselineReceipt {
    question_id: QuestionId,
    title: String,
}

impl WebworkCatalogBaselineReceipt {
    pub(super) fn from_published(
        published: &learning_data_access::PublishedProblemRecord,
    ) -> Result<Self> {
        let title = published.question.metadata.title.clone();
        if title.is_empty() {
            bail!("WebWork catalog baseline publication has no public title");
        }
        Ok(Self {
            question_id: published.question_id.clone(),
            title,
        })
    }
}

/// Publishes or verifies exactly one reviewed WebWork catalog item.
///
/// The deterministic catalog identities make reruns reconciliation rather
/// than a new publication. The public receipt is intentionally the only host
/// value a browser owner needs to locate the catalog item. ASVS 2.1.1 and
/// 2.3.1: the documented host workflow validates its fixed inputs and only
/// advances through provenance, source, and publication reconciliation.
pub(super) async fn seed_webwork_catalog_baseline(
    arguments: &SeedArguments,
) -> Result<WebworkCatalogBaselineReceipt> {
    let storage = arguments
        .webwork_catalog_baseline
        .as_ref()
        .expect("WebWork catalog storage exists after explicit flag dispatch");
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for WebWork catalog baseline")?;
    learning_data_access::postgres::apply_migrations(&pool)
        .await
        .context("applying embedded migrations for WebWork catalog baseline")?;
    let store = crate::postgres_store::configured_postgres_store(pool)?;
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let ids = WebworkCatalogBaselineIds::for_installation();
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source_record =
        put_webwork_pilot_source(&store, context, storage, reference, ids.source_object)
            .await
            .context("reconciling immutable WebWork catalog baseline source")?;
    let draft = DraftRecord {
        question: webwork_pilot_draft(ids.workspace),
        derived_from: None,
    };
    let capabilities = webwork_capabilities();
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!(
            "WebWork catalog baseline draft failed publication capability admission: {violations:?}"
        );
    }
    let published = ensure_webwork_pilot_publication(
        &store,
        context,
        arguments.instructor,
        draft,
        reference,
        source_record,
        capabilities,
    )
    .await
    .context("reconciling immutable WebWork catalog baseline publication")?;
    WebworkCatalogBaselineReceipt::from_published(&published)
}
