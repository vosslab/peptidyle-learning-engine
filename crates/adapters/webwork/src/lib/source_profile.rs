//! Evidence-bounded capability profiles for reviewed WeBWorK sources.
//!
//! The renderer can encounter arbitrary author-controlled PG. A successful
//! projection for one reviewed source therefore does not widen every PG
//! question's publication capabilities.

/// Exact path-and-content identities whose graders have accepted
/// partial-credit matching evidence in the Chapter 1 pilot.
const REVIEWED_PARTIAL_CREDIT_MATCHING_SOURCES: [(&str, &str); 2] = [
    (
        "content/pilot/sources/genetics/genetic_disorders-matching.pgml",
        "ae59425dce95bbffe0992aa5e072cd01370b736ef958685e409004d7580d2718",
    ),
    (
        "content/pilot/sources/biochemistry/biochemical_functional_groups-matching.pgml",
        "42c52281516511410623e56a315ed74f687f412a24c6ca1d028ffbe3eab12f17",
    ),
];

/// Exact Chapter 1 sources whose reviewed renderer path can return the
/// minimal correct/incorrect signal used by immediate-correctness feedback.
/// `Capability::Hints` is the question-model admission capability for that
/// student-facing signal; this allowlist does not claim arbitrary PG hints.
const REVIEWED_IMMEDIATE_CORRECTNESS_SOURCES: [(&str, &str); 4] = [
    (
        "content/pilot/sources/genetics/genetic_disorders-which_one.pgml",
        "810fc1ed93a5ed60ec79e94aa86ded3caebe2bdf8627fb71d6fecd7c6b4f062c",
    ),
    (
        "content/pilot/sources/genetics/genetic_disorders-matching.pgml",
        "ae59425dce95bbffe0992aa5e072cd01370b736ef958685e409004d7580d2718",
    ),
    (
        "content/pilot/sources/biochemistry/biochemical_functional_groups-which_one.pgml",
        "7e27357885fc8d71410bf42431105a515bdc75a776359a2d02013813e362b5fa",
    ),
    (
        "content/pilot/sources/biochemistry/biochemical_functional_groups-matching.pgml",
        "42c52281516511410623e56a315ed74f687f412a24c6ca1d028ffbe3eab12f17",
    ),
];

pub(super) fn supports_partial_credit(pg_path: &str, source_sha256: &str) -> bool {
    REVIEWED_PARTIAL_CREDIT_MATCHING_SOURCES.contains(&(pg_path, source_sha256))
}

pub(super) fn supports_immediate_correctness(pg_path: &str, source_sha256: &str) -> bool {
    REVIEWED_IMMEDIATE_CORRECTNESS_SOURCES.contains(&(pg_path, source_sha256))
}
