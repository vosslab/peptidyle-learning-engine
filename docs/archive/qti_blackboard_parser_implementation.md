# QTI Blackboard parser implementation

> **Historical accepted package.** WP-QTI-1 through WP-QTI-12 are accepted history. Current
> dependency order and remaining QTI scope are in the [release completion plan](release_completion_plan.md)
> and [implementation status](implementation_status.md).

## Status

Complete. WP-QTI-4 passed focused/full adapter gates and independent P0/P1 review on 2026-08-09.

This package adds the bounded Blackboard Original QTI 2.1 static-single-choice pool parser only. It
does not add the PLE Question JSON bridge, persistence, schema, routes, UI, or export behavior.

## Accepted boundary

The parser accepts only the bounded Blackboard pool archive grammar: `imsmanifest.xml`, the exact
`qti21_items/assessment_meta.xml`, and normalized XML paths below `qti21_items/`. It derives the
profile from parsed manifest resources, dependencies, assessment-test references, and item roots.
Unexpected entries and invalid package evidence refuse before item results are produced.

Each referenced item maps independently. The accepted subset requires a non-adaptive,
non-time-dependent `assessmentItem`; one single response declaration with one correct value; one
`choiceInteraction` with two through 100 unique choices; and static order. Absent or `false`
shuffle is static, while `shuffle="true"` is accepted only when every choice is `fixed="true"`.
Response processing must be absent or the exact observed no-extra-semantics `match` of the response
against its correct value.

An outcome declaration is absent or exactly the observed inert
`SCORE`/`single`/`float` declaration. This is a compatibility and provenance rule only: it neither
supplies points nor adds scoring behavior. Every accepted Blackboard item receives the explicit PLE
default of `1.0` points and its safe review-required warning.

Strict ordered-XML markup projection and deterministic choice-ID mapping are reused. Real shuffle,
outcome writes, alternate scoring, feedback, media, tables, styles, extensions, test policy, and
unsupported markup refuse only their item, preserving accepted siblings in the same pool.

`BlackboardQtiPackage` retains safe reports and private mapped items but implements neither Debug nor
serialization. Correct answers, raw vendor identifiers, ordered vendor-to-PLE maps, archive bytes,
and mapping digests remain outside the safe report and public API surface.

Root `xsi:schemaLocation` is accepted only as a nonempty schema hint; it does not change detection
or mapping. The exact IMSMD LOM element is retained as opaque archive provenance. Its nested
vocabulary never becomes PLE metadata, grading, assignment policy, or a generic extension path.

## Ownership and size

- `crates/adapters/qti/src/profiles/blackboard.rs`: 371 lines.
- `crates/adapters/qti/src/profiles/blackboard/shape.rs`: 411 lines.
- `crates/adapters/qti/src/profiles/blackboard/tests.rs`: 151 lines.
- `crates/adapters/qti/src/profiles/blackboard/tests/negatives.rs`: 275 lines.

Archive/package evidence, structural validation, item mapping, and negative behavior remain in
separate capability owners.

## Validation evidence

- `cargo test -p adapter_qti`: 79 unit tests, 6 fixture integration tests, and 9 compile-fail
  doctests passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked/staged/new-file whitespace checks and `git diff --check` passed.
- Independent review reported PASS with no P0/P1 finding.

## Historical successor

The immediate successor was the Q3 pure PLE Question JSON bridge in
[implementation_status.md](implementation_status.md). It translated only trusted
mapped items through PLE-owned PLE Question JSON validation and proved canonical public/private
equivalence with hand-authored source, without Store or HTTP mutation.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
