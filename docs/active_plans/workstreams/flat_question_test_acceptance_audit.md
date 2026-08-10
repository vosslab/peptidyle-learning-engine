# Flat-question persistence acceptance audit

> **Historical audit.** This concluded acceptance audit is retained as evidence, not current task
> direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

Audit date: 2026-08-09. Scope is the next vertical package: private flat JSON
workspace source -> immutable published source -> answer-free catalog record ->
grader-only material -> runtime grading. This is a behavior matrix, not a
request for mock-heavy wiring tests.

## Current evidence

- `adapter_native::flat_question` already has seven focused unit tests. They
  prove deterministic canonicalization, duplicate/unknown-member refusal,
  source size rejection, public/private compile separation, public-binding
  rejection, canonical private-byte reload, and direct correct/wrong grading.
- `learning-data-access::flat_question` and the in-memory implementation have
  seven focused tests. They validate bounded/redacted grading payloads,
  workspace source record shape, and an unauthorized actor's inability to read
  staged source metadata.
- `MemoryStore::publish_draft` has a flat-promotion branch and removes draft
  staging on successful publication. Its generic catalog conformance suite has
  no flat-question scenario, so atomicity, conflicts, successful separate
  private material read, and source preservation are not currently locked by
  behavior tests.
- `PostgresStore` has a new flat source/grader module with private-byte envelope
  tests, but no Postgres conformance scenario or disposable live test currently
  exercises it. The baseline runner does not invoke a flat-question oracle.
- `workspace_flat_question_source` and `ple_promote_flat_question_grading` are
  present in the six-file baseline. There is presently no
  `ple_flat_question_grading_material` definition in the migration search
  result, despite the Postgres grader implementation querying it; the schema
  implementation must supply it and the live gate must prove it.
- The server has no `FlatQuestion` route or runtime-backend references. Current
  flat fields in generic publication literals are all `None`; therefore there
  is no HTTP or run-lifecycle proof yet.
- Typed object keys already make `WorkspaceQuestionSource` and `ProblemSource`
  non-signable. Existing object conformance covers the generic signed-URL
  policy; add one semantic regression assertion for both flat source keys.

Focused checks run during this audit:

```text
cargo test -p adapter_native flat_question                         # 7 passed
cargo test -p learning-data-access flat_question --no-default-features  # 7 passed
```

## Small permanent behavior matrix

| Layer                                   | Keep one or a small handful of tests that prove                                                                                                                                                                                                                                                                                                                                                                                                                                       | Do not test                                                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Native adapter                          | Equivalent JSON whitespace/member order yields identical canonical bytes/digest; invalid duplicate/unknown fields, invalid binding, and over-limit source fail; compilation emits no answer/feedback in the draft; canonical private bytes grade both a correct and wrong choice.                                                                                                                                                                                                     | Serde field order, every individual validation branch, debug formatting beyond redaction.               |
| Object store                            | Private workspace source and published `ProblemSource` cannot receive a signed URL; exact bytes written under each typed key hash to the returned record.                                                                                                                                                                                                                                                                                                                             | Provider SDK request shape.                                                                             |
| Store conformance (Memory and Postgres) | Owner saves a staged canonical source tied to current draft revision; collaborator can save/read, unrelated actor and tenant cannot; a stale expected revision conflicts; publishing atomically creates visible answer-free record plus immutable copied source plus grader-only payload, then removes staging; a conflict leaves no partial catalog/key/staging mutation; private material is readable only through the dedicated grader capability and binds to the public version. | Internal map/table layout, SQL statement count, fixed UUIDs/timestamps.                                 |
| Publication validation                  | Reject source-family mismatch, public-binding mismatch, workspace/source/revision mismatch, wrong published object identity, changed byte digest/media/size, missing artifact, and simultaneous QTI + flat promotion.                                                                                                                                                                                                                                                                 | Repeating all generic catalog cases.                                                                    |
| PostgreSQL role/RLS oracle              | `ple_app` can stage/publish only its tenant and cannot select `answer_key`; `ple_grading_reader` receives only authorized grader material through the function; wrong tenant, ungranted institution version, and arbitrary direct table reads reveal nothing; immutable `answer_key` and published source reject mutation.                                                                                                                                                            | Role names as a hardcoded product behavior except where the security contract explicitly requires them. |
| HTTP/server                             | Bounded JSON source save and publish success return `Cache-Control: no-store`; malformed/over-limit bodies are rejected before persistence; instructor/collaborator authorization and foreign-tenant non-enumeration; JSON responses and logs/errors never contain `correctChoice`, private feedback, payload base64, or answer-key bytes.                                                                                                                                            | Router nesting or extractor implementation.                                                             |
| Runtime                                 | Issued learner question is answer-free; a submission evaluates through only the injected grader capability, returns policy-permitted feedback, persists normal answer/result behavior, rejects public/private binding substitutions, and does not disclose grader material on error/replay.                                                                                                                                                                                           | A mock assertion that a particular helper was called.                                                   |
| Retention                               | Deleting/purging a draft clears staged flat source metadata; published immutable source/key retain or purge exactly according to existing published-content/tenant retention policy, with a dedicated policy assertion once the ownership decision is encoded.                                                                                                                                                                                                                        | A brittle exact deletion ordering assertion.                                                            |

## Recommended test owners and test shape

1. Add `crates/learning-data-access/tests/conformance/flat_question.rs` and
   call it from the existing conformance facade. Parameterize it over the same
   Store surface as existing catalog conformance, with a separately injected
   `FlatQuestionGradingStore` only for the post-publication read. Use one
   compact real source fixture and compare observable records, not backend
   internals.
2. Keep payload-envelope/base64 integrity tests in
   `src/postgres/flat_question.rs`; add its matching integration scenario only
   after the publication SQL is wired. This is the right narrow place to prove
   JSONB does not normalize away the canonical private bytes.
3. Add `crates/learning-data-access/tests/postgres_flat_question_live.rs`,
   `#[ignore]`, analogous to the current QTI/manual/item-analysis live files.
   It must use a fresh tenant pair, call `verify_application_schema`, exercise
   `PostgresStore` plus a separately connected `PostgresGraderStore`, and make
   direct SQL probes under the application/grader roles. Add this one named
   test to `tests/e2e/e2e_database_baseline.sh` after the QTI live test.
4. Add one focused server module (prefer a new flat-publication/private-tests
   pair following QTI's split) using `MemoryStore`, `MemoryObjectStore`, and a
   real native compiler fixture. Then add the normal learner run lifecycle,
   not merely route registration.
5. Extend object-store conformance with the two typed source keys. This is a
   small regression test, not a new storage suite.

## Disposable PostgreSQL oracle: minimum sequence

1. Apply the six migrations in the isolated baseline cluster and verify SQLx
   compatibility.
2. As tenant A application principal, create owner/collaborator draft access,
   stage canonical source A, and demonstrate a stale revision/source CAS
   conflict without replacement.
3. Copy exact source bytes to a fresh `ProblemSource`, publish once, then prove
   the catalog payload/source artifact are answer-free and checksum-bound,
   `workspace_flat_question_source` is gone, and the `answer_key` exists only
   through the dedicated grader function.
4. Under tenant A's grader login, retrieve payload, base64-decode it in Rust,
   verify the recorded SHA/binding, and grade a correct and incorrect answer.
   Under `ple_app`, a direct `answer_key` query must fail; under tenant B and a
   tenant-A user without an institution grant, the grader function returns no
   row. An arbitrary caller cannot execute the function.
5. Attempt an update/delete of the answer key and a second publication using
   the same problem/version; both fail without changing the first publication.
   Attempt a source-artifact digest mismatch and verify no answer key or
   catalog record is created.

The oracle should use runtime-minted IDs and response content. It should not
assert exact migration count, fixed timestamps, SQL text, row ordering, or
implementation-owned map/table access beyond the explicit direct-security
probes above.

## Acceptance commands after implementation

```bash
cargo fmt --check
cargo clippy -p adapter_native -p learning-data-access -p server_core --all-targets -- -D warnings
cargo test -p adapter_native flat_question
cargo test -p objects --test conformance
cargo test -p learning-data-access --test conformance flat_question
cargo test -p learning-data-access --features postgres --lib flat_question
cargo test -p server_core flat_question
bash tests/e2e/e2e_database_baseline.sh
```

The last command is the required production-schema/RLS proof and needs Podman,
`podman-compose`, Cargo, and an otherwise unused loopback port. Do not replace
it with an in-memory success test.
