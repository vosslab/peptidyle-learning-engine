# Flat-question persistence/publication/runtime package review

> **Historical review.** This concluded review is retained as evidence, not current task direction.
> Current authority is the [release completion plan](../active/release_completion_plan.md) and
> [implementation status](../implementation_status.md).

Date: 2026-08-09

## Status

**NEEDS_FIXES**

The HTTP path, PostgreSQL role separation, canonical byte envelope, source-key
non-signability, atomic catalog promotion, and runtime fail-closed behavior are
substantially in place. The production live gate passed. One P1 integrity gap
remains at the reusable Store boundary: it accepts and publishes an object that
claims to be a closed flat single-choice question without proving that either
the public draft or private payload is a valid compiled flat question.

## Findings

### P1 - Flat persistence contracts allow invalid "flat" publications

`FlatQuestionGradingPayload::new` accepts any bounded JSON object containing a
lowercase `publicSha256`; it does not require canonical
`FlatQuestionPrivate` bytes or the private shape. `PostgresStore` then accepts
any generic-valid native `flat_single_choice_v1` draft and stores that payload.
The analogous Memory path has the same gap. The promotion validator verifies
tenant, revision, source-record metadata, digest binding, and artifact identity,
but it does not validate compiled flat semantics.

Evidence:

- `crates/learning-data-access/src/flat_question.rs:28-63` accepts an arbitrary
  JSON object as grader payload.
- `crates/learning-data-access/src/postgres/flat_question.rs:35-47` calls only
  generic draft validation before staging; `:50-60` only checks the source
  family string.
- `crates/learning-data-access/src/in_memory/flat_question.rs:22-43` has the
  same generic-only draft validation and source-family check.
- `crates/learning-data-access/src/publication_validation.rs:460-515` validates
  promotion metadata but does not decode/validate the private payload or
  confirm a flat multiple-choice public shape.
- The supposed production proof deliberately exercises this invalid contract:
  `crates/learning-data-access/tests/postgres_flat_question_live.rs:38-67`
  creates `flat_single_choice_v1` with `ResponseDefinition::ExternalTool`, and
  `:121-126` uses a made-up `{ publicSha256, answerKey }` payload rather than
  `FlatQuestionPrivate::canonical_bytes`. It publishes successfully at
  `:367-389`.

Impact: a trusted caller outside the dedicated HTTP route can persist a
catalog record that advertises the closed flat family but cannot issue/grade as
that family. This contradicts the package's closed, compiler-owned flat source
contract and leaves the database gate unable to prove the real format works.
The browser route currently prevents this by compiling first
(`crates/server/src/flat_question_publication.rs:129-157`), but the Store is a
cross-layer contract and is already called directly by import/test/worker-style
code elsewhere in the repository.

Required remediation:

1. Add a dependency-neutral flat public-shape validation to
   `learning-data-access` (native family, static randomization, exactly-one
   multiple choice, valid all-or-nothing grading), and enforce it in both
   `FlatQuestionStore` backends and promotion validation. Keep the adapter as
   the authoritative parser/compiler; this check is an integrity backstop, not
   a second parser.
2. Make the private material contract opaque-but-valid: either move the
   canonical private-byte validation behind a narrowly injected compiler
   capability at the server promotion boundary, or add a typed
   adapter-validated private payload constructor that the Store receives only
   after validation. Do not bless arbitrary JSON as flat grader material.
3. Replace the PostgreSQL live fixture with a real `FlatQuestionDocument`:
   canonicalize it, compile it, stage the compiled draft/private bytes, read
   the base64 envelope through `PostgresGraderStore`, decode
   `FlatQuestionPrivate`, and grade both a correct and an incorrect choice.
   Retain the direct role/RLS assertions.

## Verified strengths

- The author-only route canonicalizes before writing an immutable workspace
  source, returns only an answer-free draft, and uses a CAS revision:
  `crates/server/src/flat_question_publication.rs:120-231`.
- Publication rereads/compiles the private source, binds it to the current
  draft, writes a distinct non-signable `ProblemSource`, and submits one
  catalog command: `crates/server/src/flat_question_publication.rs:278-441`.
- The PostgreSQL promotion locks the draft, verifies staged source metadata,
  inserts only through `ple_grader`, and uses a bounded base64 envelope:
  `schemas/migrations/2026080805_operations_analytics.sql:427-527`.
- The grader function is `SECURITY DEFINER`, has an explicit safe search path,
  checks the tenant context and closed native family, and is executable only
  by `ple_grading_reader`: `schemas/migrations/2026080802_catalog_authoring.sql:134-175`,
  `:1041-1042`.
- The runtime receives a separately injected grader capability, refuses its
  absence, replays public provenance first, checks the public binding, and
  leaves browser feedback projection to the normal run policy layer:
  `crates/server/src/native_backend.rs:138-218`, `:285-337`; see
  `crates/server/src/run.rs:500-537`.
- Source keys remain non-signable and the focused route test covers both
  canonical stored bytes and absence of private response fields:
  `crates/server/src/flat_question_publication/tests.rs:211-265`, `:389-465`.

## Validation rerun

All passed in this review checkout:

```text
cargo fmt --check
cargo clippy -p adapter_native -p learning-data-access -p server_core --all-targets -- -D warnings
cargo test -p adapter_native flat_question                         # 7 passed
cargo test -p learning-data-access --test conformance flat_question # 1 passed
cargo test -p learning-data-access --features postgres --lib flat_question # 13 passed
cargo test -p server_core flat_question                            # 6 passed
```

The parent workstream separately reported that the full server suite and the
Podman-backed baseline, including the PostgreSQL flat gate, passed. That proves
the currently exercised route and SQL paths, but not the real compiled private
payload through PostgreSQL described in the P1 finding.

## Re-review addendum - 2026-08-09

**PASS.** The prior P1 is fixed in the current tree. This re-review was limited
to that correction and regression-sensitive boundaries; it did not rerun the
Podman baseline because the parent already reported its passing result and the
static/live fixture evidence no longer conflicts with it.

The parent subsequently confirmed that the complete static/package gates and
the disposable PostgreSQL baseline pass after this remediation.

### P1 remediation verified

- `FlatQuestionGradingPayload` has one public construction route,
  `from_private`, which canonicalizes and validates a
  `FlatQuestionPrivate`. Its raw-byte constructor and byte accessor are both
  `pub(crate)`, so another crate cannot create a payload from a lookalike JSON
  object or extract its bytes (`crates/learning-data-access/src/flat_question.rs:27-80`).
- Both persistence backends validate the closed static single-choice draft
  before any mutable database transaction or in-memory write lock is taken:
  PostgreSQL (`crates/learning-data-access/src/postgres/flat_question.rs:35-64,
268-273`) and Memory (`crates/learning-data-access/src/in_memory/flat_question.rs:14-29`).
  The Memory regression test proves a forged `ExternalTool` response leaves
  both draft and source absent (`.../in_memory/flat_question.rs:386-412`).
- Publication performs the same public-shape check, decodes the typed private
  material, and requires it to bind to the exact staged draft
  (`crates/learning-data-access/src/publication_validation.rs:496-523`).
  This is the required compiler-owned invariant backstop, rather than merely
  trusting a native-family label.
- PostgreSQL reads only the grader function and converts its base64 JSONB
  envelope into `FlatQuestionGradingPayload` after verifying envelope digest,
  database digest, canonical private bytes, and public binding
  (`crates/learning-data-access/src/postgres/flat_question.rs:240-265,
306-352`). No raw persisted JSON reaches the runtime.
- The revised disposable PostgreSQL fixture parses and compiles the real
  authoring document, stores its canonical private bytes, retrieves through
  the isolated grader capability, and evaluates both `blue` as correct and
  `red` as incorrect (`crates/learning-data-access/tests/postgres_flat_question_live.rs:234-270,
386-437`). It retains direct app/student answer-key denial and foreign-tenant
  non-enumeration checks.
- The browser closure remains exactly `wasm_bridge`, `domain`, and
  `question_model`, with no `grading` dependency
  (`tests/test_crate_boundaries.py:158-162`).

### Validation rerun

All passed in this independent re-review:

```text
cargo fmt --check
cargo clippy -p adapter_native -p learning-data-access -p server_core --all-targets -- -D warnings
cargo test -p adapter_native flat_question                         # 8 passed
cargo test -p learning-data-access --test conformance flat_question # 1 passed
cargo test -p learning-data-access --features postgres --lib flat_question # 14 passed
cargo test -p server_core flat_question                            # 6 passed
source source_me.sh && python3 -m pytest -q tests/test_crate_boundaries.py # 5 passed
git diff --check
```
