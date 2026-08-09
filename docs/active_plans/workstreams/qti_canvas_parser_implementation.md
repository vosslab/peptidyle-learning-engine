# QTI Canvas parser implementation

## Status

Complete. WP-QTI-3 passed focused/full adapter gates and independent P0/P1 review on 2026-08-09.

This package adds the bounded Canvas QTI 1.2 parser only. It does not add a Blackboard parser,
native flat-question factory, persistence, schema, route, UI, or export behavior.

## Accepted boundary

The parser reads only the bounded Canvas archive grammar: `imsmanifest.xml`, the exact
`canvas_qti12_questions/assessment_meta.xml`, and normalized XML item paths beneath that directory.
It parses the exact manifest/resource/dependency tree and assessment metadata, derives detector
evidence from those parsed structures, and refuses unexpected entries before item mapping.

Each candidate item is mapped independently. The parser accepts only the documented static
single-choice Canvas tree: `multiple_choice_question`, one single-cardinality `response_lid`, two
through 100 ordered response labels, declared finite nonnegative points, exact source-order
consistency, one correct `varequal`, and one `SCORE=100` action. Prompt and choice `mattext` use
the shared strict markup projector; choice identifiers use the shared deterministic mapping.

Unsupported scoring, feedback, media, table/style markup, unexpected attributes, and structural
extensions become bounded per-item safe refusals, so a valid sibling remains usable. Package-level
manifest, namespace, resource-graph, duplicate-item, or unexpected-entry failures reject the
package before item results are produced.

`CanvasQtiPackage` retains parsed detection evidence, safe item reports, and private mapped items.
It implements neither `Debug` nor serialization. Correct answers, ordered vendor-to-PLE mappings,
archive bytes, and mapping digests remain outside the safe report and public API surface.

The exact IMSMD LOM element in the observed Canvas manifest is accepted as inert provenance
evidence. Its contents do not become PLE metadata, authoring policy, or a generic extension path.

## Ownership and size

- `crates/adapters/qti/src/profiles/canvas.rs`: 553 lines.
- `crates/adapters/qti/src/profiles/canvas/shape.rs`: 234 lines.
- `crates/adapters/qti/src/profiles/canvas/tests.rs`: 135 lines.
- `crates/adapters/qti/src/profiles/canvas/tests/negatives.rs`: 214 lines.

The structural manifest/archive grammar, item mapping, and negative corpus remain separate so no
single owner becomes a QTI catch-all module.

## Validation evidence

- `cargo test -p adapter_qti`: 60 unit tests, 6 fixture integration tests, and 7 compile-fail
  doctests passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked/staged/new-file whitespace checks and `git diff --check` passed.
- Independent review reported PASS with no P0/P1 finding.

## Next package

Implement the exact Blackboard QTI 2.1 static-single-choice parser described in
[qti_profile_mapping_plan.md](../decisions/qti_profile_mapping_plan.md). It must reuse the bounded
archive/XML, strict markup, deterministic choice-ID, mapped-item, and safe-report contracts without
widening the Canvas grammar.
