# QTI native flat bridge

## Status

Complete. Q3/WP-QTI-5 passed focused/full gates and independent P0/P1 review on 2026-08-09.

This package performs pure mapping from a validated private QTI item into canonical PLE flat-question
v1 source and its existing public/private compilation. It adds no Store, object, schema, HTTP, UI,
Wasm, or persistence behavior.

## Architecture boundary

`adapter_native::flat_question::imported` accepts only a bounded, trusted ordered mapped input. It
sets the fixed imported v1 defaults: empty feedback, unlimited attempts, `immediateFull`, untimed,
empty tags and taxonomy, `allRightsReserved`, and `en-US`. It requires a canonical finite,
nonnegative point string; validates the complete native document; canonicalizes it; reparses its
canonical bytes; and uses the existing native compiler for the answer-free draft and grader-only
private material.

The factory enforces the shared 256 KiB whole canonical-source cap, along with native title, prompt,
choice, identifier, count, and correct-binding limits. It does not duplicate the native parser or
serializer. Its canonical bytes and compiled public/private parts equal an equivalent hand-authored
flat source.

The crate-private server bridge consumes the already owner-bound `QtiMappedItem`, accepts only the
Canvas and Blackboard v1 profile/mapping versions, and carries its `QtiMappedItemServerParts` beside
the canonical source, draft, and private compiler result. That retains the ordered private map and
owner-bound mapping evidence for the later atomic provenance command without turning them into a
safe DTO or detached value.

The bridge has real minimized Canvas and Blackboard fixture tests. Both produce the same canonical
source, public draft, and private binding as the equivalent hand-authored PLE source; Blackboard's
reviewed 1.0-point default remains exact. No private input, factory product, bridge result, or
mapping part is `Debug` or serializable, and no QTI profile/private type reaches the Wasm boundary.

## Ownership and size

- `crates/adapters/native/src/flat_question/imported.rs`: 422 lines.
- `crates/server/src/qti_profile_flat_bridge.rs`: 288 lines.

Native construction and server-only QTI translation remain separate capability owners. The server
does not absorb native validation, and the native adapter does not acquire QTI archive parsing.

## Validation evidence

- `cargo test -p adapter_native`: 31 unit tests and 6 compile-fail doctests passed.
- `cargo test -p adapter_qti`: 79 unit tests, 6 fixture integration tests, and 9 compile-fail
  doctests passed.
- Focused server bridge tests: 3 passed; full server gate: 162 unit tests and 1 doctest passed.
- Strict Clippy, formatting, crate-boundary, staged/new-file whitespace, and diff checks passed.
- Independent review reported PASS with no P0/P1 finding.

## Historical successor

The immediate successor was Q4/WP-QTI-6, the provenance contract and typed non-signable archive
object key in [qti_profile_mapping_plan.md](../decisions/qti_profile_mapping_plan.md). It froze the
current and published origin types, object identity, lifecycle, lock order, and one atomic Store
command shape before backend mutation.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
