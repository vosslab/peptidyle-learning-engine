# QTI Profile Fixture Corpus Implementation

> **Historical accepted package.** WP-QTI-1 through WP-QTI-12 are accepted history. Current
> dependency order and remaining QTI scope are in the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Status

Complete. WP-QTI-2 passed its focused gates and independent P0/P1 review on 2026-08-09.

This package supplies readable, parser-ready Canvas and Blackboard evidence for the frozen
WP-QTI-1 profile contract. It adds no vendor parser, production archive behavior, persistence,
route, schema, or browser behavior.

## Corpus

The checked-in test corpus contains:

- a Canvas QTI 1.2 manifest, `assessment_meta`, and one static single-choice item;
- a Blackboard QTI 2.1 pool manifest, `assessment_meta`, and one static single-choice item;
- one manifest near miss and one item near miss for each profile;
- one harmless asset used to keep the Blackboard unexpected-resource near-miss package complete.

The fixtures retain the observed vendor package structure while keeping content intentionally
small. Canvas uses nested `IMS Content` metadata, vendor-local resources, HTML `mattext`, one
single-cardinality interaction, matching `original_answer_ids`, exact points, and one SCORE-100
condition. Blackboard uses nested `QTIv2.1` metadata, one pool item, one single response declaration,
the retained static-order pattern `shuffle=true` with every choice `fixed=true`, and the exact
no-extra-semantics match processing shape.

Each item near miss changes one semantic fact: Canvas changes response cardinality, and Blackboard
changes one choice from fixed to non-fixed. The manifest near misses isolate unsafe Canvas path
normalization and a complete but unsupported Blackboard resource.

## Test support

`crates/adapters/qti/tests/support/mod.rs` owns reusable fixture constants, safe normalized member
checks, sorted logical ZIP construction, and archive readback. Tests compare member paths and
contents rather than ZIP container bytes or timestamps.

The XML fixture check validates tokenizer errors, pending starts, attribute placement, balanced and
matching element nesting, one root, text placement, and complete EOF closure. A malformed-close
regression prevents a tokenizer-only check from being mistaken for XML well-formedness.

No permanent test reads `OTHER_REPOS`; retained packages were evidence used to minimize this local
corpus.

## Validation evidence

- `cargo test -p adapter_qti --test profile_fixture_corpus`: 6 passed.
- `cargo test -p adapter_qti`: 22 unit tests, 6 integration tests, and one compile-fail doctest
  passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked, staged, and every new fixture/test-file whitespace check passed.
- Independent review reported PASS with no remaining P0/P1 finding.

## Historical successor

The immediate successors were the exact profile parsers. The Canvas and Blackboard owners shared a
bounded markup/choice-ID helper with one explicit owner. They consumed this fixture corpus and frozen
matrix, produced stable diagnostics and server-only mapped Answer Key data, and preserved partial
package success without widening the generic archive-safety grammar or persisting vendor XML as PLE
source.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
