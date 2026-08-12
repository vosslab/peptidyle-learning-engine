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

pub(super) fn supports_partial_credit(pg_path: &str, source_sha256: &str) -> bool {
    REVIEWED_PARTIAL_CREDIT_MATCHING_SOURCES.contains(&(pg_path, source_sha256))
}
