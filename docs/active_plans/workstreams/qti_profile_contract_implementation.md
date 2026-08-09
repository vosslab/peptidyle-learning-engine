# QTI Profile Contract Implementation

## Status

Complete. WP-QTI-1 passed its focused gates and independent P0/P1 re-review on 2026-08-09.

This package freezes the identities and evidence contract that the Canvas and Blackboard parsers
must satisfy. It does not parse vendor items, create flat-question source, change persistence, or
add browser routes.

## Implemented boundary

- Added the exact persisted profile identities:
  - `canvas-qti-1.2-static-single-choice/v1`;
  - `blackboard-qti-2.1-static-single-choice-pool/v1`;
  - `ple-qti-assessment-item-single-choice/v1` for the existing generic compatibility path.
- Replaced the misleading generic `qti-1.2-subset` runtime/import label with the honest generic
  profile identity.
- Added one authoritative vendor-profile matrix. It records the manifest and item namespaces,
  schemas, resource types, normalized paths, reciprocal `assessment_meta` dependency graph, field
  mappings, default/refusal policy, and positive/near-miss fixture paths.
- Matched retained package evidence exactly: Canvas uses `IMS Content` and
  `canvas_qti12_questions/assessment_meta.xml`; Blackboard uses `QTIv2.1` and
  `qti21_items/assessment_meta.xml`.
- Added deterministic detection that refuses malformed or mixed vendor evidence without claiming a
  vendor profile. An unrelated IMS Content Package remains eligible for the existing generic path.
- Added versioned canonical digests for the safe profile report, public mapping, private mapping,
  combined mapping, and visible warnings/defaults.
- Kept private choice mappings non-serializable and non-debuggable. Digest construction rejects
  inconsistent profile versions, detector outcomes, accepted-item dispositions, public mapping
  digests, and missing private correct-choice bindings.

## Focused owners

- `crates/adapters/qti/src/profiles.rs`: closed identities, evidence, outcomes, diagnostics, and
  contract errors.
- `crates/adapters/qti/src/profiles/matrix.rs`: sole vendor matrix and exact graph/path validation.
- `crates/adapters/qti/src/profiles/digests.rs`: public and server-only canonical digest inputs.
- `crates/adapters/qti/tests/fixtures/profiles/`: readable detection fixtures derived from retained
  Canvas and Blackboard syntax.

Each production owner remains below 350 lines. The completed WP-QTI-2 corpus now supplies the
parser-ready items and reusable logical-package builder while leaving this production contract
unchanged.

## Validation evidence

- `cargo test -p adapter_qti`: 22 unit tests and one compile-fail doctest passed.
- Focused server tests for the runtime identity and import registry passed.
- `cargo clippy -p adapter_qti -p server_core --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked, staged, and new-file whitespace checks passed.
- Independent re-review reported PASS with no remaining P0/P1 finding.

## Next package

Proceed to the Canvas and Blackboard parser packages using the completed WP-QTI-2 corpus. The
profile parsers must consume this frozen contract; they must not relax the generic archive-safety
grammar or invent a second profile identity or digest encoding.
