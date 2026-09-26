//! Public Question Library facets assembled from database aggregates.

use std::collections::BTreeMap;

use learning_data_access::QuestionLibrarySearchFacets;
use question_model::{
    Capability, QuestionBackend, QuestionBackendCapabilities, QuestionSearchCapabilityFacet,
    QuestionSearchFacets,
};

/// Folds database backend counts through the existing adapter declarations.
/// The database owns every metadata aggregate; capability declarations remain
/// at this adapter boundary because they are not Question-source metadata.
pub(super) fn from_store(facets: QuestionLibrarySearchFacets) -> QuestionSearchFacets {
    let mut capabilities = BTreeMap::<Capability, u64>::new();
    for backend_facet in &facets.backends {
        for capability in backend_capabilities(backend_facet.backend).declared() {
            *capabilities.entry(capability).or_default() += backend_facet.count;
        }
    }
    QuestionSearchFacets {
        author_names: facets.author_names,
        author_names_truncated: facets.author_names_truncated,
        backends: facets.backends,
        tags: facets.tags,
        tags_truncated: facets.tags_truncated,
        subjects: facets.subjects,
        subjects_truncated: facets.subjects_truncated,
        topics: facets.topics,
        topics_truncated: facets.topics_truncated,
        question_types: facets.question_types,
        capabilities: capabilities
            .into_iter()
            .map(|(capability, count)| QuestionSearchCapabilityFacet { capability, count })
            .collect(),
        question_licenses: facets.question_licenses,
        used_in_my_courses: facets.used_in_my_courses,
        bloom_cognitive_processes: facets.bloom_cognitive_processes,
        bloom_knowledge_dimensions: facets.bloom_knowledge_dimensions,
    }
}

pub(super) fn backend_capabilities(backend: QuestionBackend) -> QuestionBackendCapabilities {
    match backend {
        QuestionBackend::Ple => QuestionBackendCapabilities::from_iter([
            Capability::ClientRendering,
            Capability::ServerGrading,
        ]),
        QuestionBackend::Webwork => adapter_webwork::webwork_source_capabilities(backend)
            .expect("WebWork capability declaration accepts WebWork"),
        // iMathAS capabilities are source-profile-specific, so its backend
        // declaration cannot honestly satisfy a global capability predicate.
        QuestionBackend::Imathas => QuestionBackendCapabilities::none(),
    }
}
