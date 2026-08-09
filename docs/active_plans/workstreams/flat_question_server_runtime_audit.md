# Flat-question server vertical audit

Status: **DONE_WITH_CONCERNS**

This is a bounded server-path audit. It does not prescribe an editor UI; the
separate HCI brief owns that work. The existing contracts support a small,
dedicated flat-question route module and a narrowly extended native runtime,
but the package cannot be exposed until the PostgreSQL promotion/privilege gate
is corrected and live-tested.

## Existing usable seams

- The adapter already parses a closed, bounded author source and compiles it
  into an answer-free draft plus private canonical material. It rejects a
  noncanonical private read and verifies the private/public binding at grading
  time: `crates/adapters/native/src/flat_question.rs:347-445` and
  `497-557`.
- `FlatQuestionStore` separates workspace source metadata from
  `FlatQuestionGradingStore`, whose payload is intentionally redacted in
  debug output: `crates/learning-data-access/src/flat_question.rs:15-76` and
  `256-286`. The promotion validator requires a copied published source,
  exact current draft revision, matching source metadata, and the private
  public-binding checksum: `publication_validation.rs:435-515`.
- Both workspace and published source keys are `Source` objects and are
  categorically ineligible for signed URLs: `crates/objects/src/bucket.rs:60-74`,
  `89-100`, and `240-252`.
- QTI provides the appropriate server pattern: a dedicated route validates a
  current draft, copies immutable candidate objects only after validation, and
  sends one `PublishDraftCommand` to the store
  (`crates/server/src/qti_publication.rs:53-80`, `143-290`, and `446-557`).
  It is the model for source-backed flat publication, not the generic catalog
  route.
- The generic catalog route necessarily cannot publish a flat question today:
  it always sends `source_artifact: None` and
  `flat_question_promotion: None` (`crates/server/src/catalog.rs:658-680`).
  Storage correctly refuses that combination.
- The generic native runtime can issue and reproduce the public flat shape,
  but its `grade`/`submit` calls the adapter's ordinary key derivation
  (`crates/server/src/native_backend.rs:108-167`). The flat family deliberately
  returns no ordinary answer key
  (`crates/adapters/native/src/flat_question.rs:263-288`), so this is not a
  grading path for flat questions.

## Minimal server modules and routes

Add one focused module, `crates/server/src/flat_question_publication.rs`, and
export it from `crates/server/src/lib.rs`. Keep `workspace.rs` and `catalog.rs`
as their generic, answer-free routes; both are already sizeable facades.

The new module should own these two routes and their private state type:

| Route | Body and response | Required bounds |
| --- | --- | --- |
| `PUT /api/workspaces/{workspace}/flat-question` | Body is the complete PLE flat JSON source, received as bounded raw `Bytes`; response is the compiled answer-free `DraftQuestionDefinition` with the new workspace revision as a strong `ETag`. Do not echo source bytes, key, feedback, object record, or checksums. | `S: Store + FlatQuestionStore + SessionStore`; `O: ObjectStore` |
| `POST /api/problems/{workspace}/flat-question-publish` | Strict `{ "scope": ... }`; response is the existing published `QuestionDefinition`, status 201. | `S: Store + CatalogStore + FlatQuestionStore + SessionStore`; `O: ObjectStore`; `B: BackendRegistry`; `R: PublicReviewGate` |

Place both on `DefaultBodyLimit::max(256 * 1024)` (the adapter/store source
limit) and an `axum::middleware::map_response(no_store_response)` layer. The
publication body itself is tiny, but sharing the module-wide limit avoids an
unbounded raw-source extractor. Reuse the existing session resolver, exact
author/publisher-role checks, `If-Match` parsing semantics, stable error
mapping, `no_store`, and public-review gate patterns from
`workspace.rs:87-220`, `workspace.rs:753-826`, and
`qti_publication.rs:143-290`. Missing/non-author workspace access should
remain a non-enumerating not-found response as it does in the author-preview
route (`author_preview.rs:101-114`).

Do not add a browser-supplied source object ID, checksum, private payload,
capabilities, or catalog ID to either DTO. The server derives all of them.

## Save lifecycle

The source-saving handler should perform these steps in this order:

1. Authenticate, authorize an instructor/publisher/administrator, parse the
   strong optional `If-Match`, and parse the raw bytes with
   `FlatQuestionDocument::parse`. Compile against the path `WorkspaceId`, then
   canonicalize the source with `canonical_bytes`; never persist the browser's
   whitespace/order spelling.
2. Read the visible workspace to preserve only server-owned `revises` and
   `derived_from`, construct the compiled answer-free `DraftRecord`, and use
   `Store::upsert_draft` with the requested workspace revision. This CAS is
   the authoritative editor concurrency check.
3. Before the database binding, write canonical source bytes once to a fresh
   `ObjectKey::WorkspaceQuestionSource { tenant, workspace, object }` using
   `FLAT_QUESTION_MEDIA_TYPE`, the draft license, a server-owned provenance
   string, and a server timestamp. A write that loses the later CAS/binding
   race is an unreachable immutable orphan, not a catalog-visible source.
   Conversely, never bind metadata for a failed object write.
4. Call `FlatQuestionStore::upsert_flat_question` with the saved exact draft,
   fresh object record, the canonical source SHA-256, and the private
   `public_binding_sha256`. The ordinary draft update clears stale binding
   rows by trigger (`schemas/migrations/2026080802_catalog_authoring.sql:868-909`),
   so a normal replacement carries `expected_revision: None`; the store binds
   its just-saved draft revision.
5. Return only the answer-free draft plus ETag. A failed source-binding write
   leaves an answer-free draft without flat staging and therefore cannot
   publish; return an actionable retryable/availability response. Do not try
   to delete arbitrary objects in the request error path.

The exact database/object sequence is not cross-system atomic. Its observable
invariant is: the database never points to absent/unverified source bytes, and
no candidate source becomes browser-deliverable. This matches the existing
QTI bytes-first candidate model (`qti_publication.rs:446-557`).

## Publication lifecycle

The dedicated publish handler should follow QTI's double-read/review pattern:

1. Authenticate publisher role and exact required `If-Match`; load the draft,
   verify it is native `flat_single_choice_v1`, validate server-owned
   capabilities and ordinary publication policy, and run public review when
   requested.
2. Re-read the same actor-visible workspace after review; require the same
   revision and re-run source/capability validation. Load
   `FlatQuestionStore::flat_question_source` and require that it binds the
   same revision/draft/family.
3. Read the private workspace source through the trusted object store; require
   exact `ObjectRecord` equality and checksum, parse it again, canonicalize it
   again, compile it again, and require its answer-free draft equals the
   stored draft and its public-binding checksum equals the staged metadata.
   This prevents a source-object substitution or a stale metadata bridge.
4. Only then mint `ProblemVersionRef`, copy the *canonical* bytes to a fresh
   `ObjectKey::ProblemSource`, and construct `PublishedSourceArtifact` with
   `QuestionBackend::Native`. Preserve source checksum/size/media type, set
   the published license from the answer-free draft, and use a server-owned
   provenance. Source keys remain non-signable by their typed key policy.
5. Build `FlatQuestionPublicationPromotion { source, grading }`, where
   `grading` contains the freshly compiled private canonical bytes. Submit one
   `PublishDraftCommand` with the matching native published source,
   `Some(source_artifact)`, and `Some(flat_question_promotion)`. The store is
   the atomic database boundary for immutable public payload, catalog grant,
   source artifact metadata, grader-only key material, and draft consumption.
6. Map stale/changed source or draft state to conflict, source/object absence
   to not-found, malformed/inconsistent persisted data to a stable
   unprocessable/invalid-publication response, and temporary database/object
   failure to service unavailable. Never include the source parser detail or
   private feedback in an HTTP response or log.

Candidate object writes may precede the database transaction; a losing
publication race leaves an unbound immutable candidate. It must not cause a
durable `problem_id` to be visible. The publication must not be folded into
`catalog::publish_problem`: that route is intentionally generic and does not
own an object-store dependency.

## Runtime grading injection

Extend `NativeBackend` rather than add a public source-object read to run
routes. Its public flat question is fully represented by the catalog
`QuestionDefinition`; issuance and reproduction need only the normal native
adapter and asset bindings. Only grade/submit need a private capability.

Use `NativeBackend<S, G>` (or an equivalent internal generic) with a distinct
`Arc<G>` where `G: FlatQuestionGradingStore + Send + Sync + 'static`. For the
flat family only:

1. Validate attempt/reference and reproduce the ordinary native attempt first,
   preserving the existing seed/provenance protections.
2. Call `G::flat_question_published_grading(context, reference)`. Treat no
   visible payload as invalid/unavailable grader binding, never as an
   ungraded answer.
3. Decode with `FlatQuestionPrivate::from_canonical_bytes`, require the
   payload's public binding equals the decoded binding, then call
   `private.evaluate(question, response)`. That method independently binds
   the key/feedback to the public immutable question.
4. `grade` returns only the `GradeOutcome`; `submit` builds a `GradeReceipt`
   from the returned result plus policy-redacted feedback. Do not route the
   private bytes into `QuestionEnvelope`, attempts, DTOs, or `Debug`.

All other native families retain the adapter's existing grade/submit path.
`CompositeBackend` needs no new dispatch kind because flat remains native; it
delegates native source as it already does at `composite_backend.rs:110-126`
and `242-300`.

Production composition must create a separate `PostgresGraderStore` for the
native backend, not hand `PostgresStore` application credentials the grading
trait. The existing QTI construction demonstrates the separate connection
point (`composition.rs:384-399`). Generalize its environment/configuration
from QTI-specific optional runtime to a required grader connection whenever
the flat runtime is enabled, pass it to `NativeBackend`, and retain it as the
only `FlatQuestionGradingStore` in production. Focused in-memory tests should
use `MemoryStore::with_flat_question_grader()` rather than implementing the
grader trait for the app store (`in_memory.rs:199-205`).

## Required integration edits

- `crates/server/src/lib.rs`: add the one flat publication module declaration.
- `crates/server/src/composition.rs`: add `FlatQuestionStore` to the concrete
  route/composition bounds; pass `objects` into the new route module; connect
  and inject the isolated grader handle into `NativeBackend`; merge the new
  router. The current composition builds native without a grader at `429-447`
  and merges only QTI source publication at `521-553`.
- `crates/server/src/native_backend.rs`: keep generic public/native behavior;
  isolate flat private evaluation in a short helper or child module if it
  approaches file-size pressure. The file is currently below the owner's
  1000-line ceiling but already contains a large focused test module.
- `crates/server/src/composite_backend.rs`: type parameters may need a
  mechanical update for `NativeBackend<S, G>`; do not make the composite or
  browser DTO aware of a grader store.
- Do not extend `workspace.rs` or `catalog.rs` with private source fields;
  their generic requests stay answer-free.

## Focused acceptance tests

Add server tests alongside the new module and native-backend tests for:

1. Save accepts only strict bounded JSON, canonicalizes it, saves an
   answer-free draft and a non-signable workspace source, returns ETag and
   `Cache-Control: no-store`; malformed/oversized source, path/workspace
   mismatch, missing/stale/malformed ETag, unauthenticated, and non-author
   requests are refused.
2. Save preserves revision lineage, never returns `correct`, choice feedback,
   `publicSha256`, object key, or private bytes, and a subsequent ordinary
   workspace update clears staging.
3. Publish success copies bytes to a fresh `ProblemSource`, supplies both
   artifact and promotion, persists an answer-free public model, consumes the
   workspace staging, and rejects signed URL issuance for both source keys.
4. Publish refuses missing/stale/source-substituted metadata, altered object
   bytes/checksum, changed source after review, a non-flat native draft,
   actor/tenant mismatch, and public-review denial. Failed candidate write
   does not mint a catalog record.
5. Native issue/reproduce responses are identical public envelopes for flat
   questions; a valid submit uses only the injected grader and returns
   allowed feedback. A missing/corrupt/noncanonical/mismatched private payload
   fails closed, foreign tenant receives no grading material, and no private
   payload appears in serialized response/debug logs.
6. PostgreSQL live gate: app credentials cannot select `answer_key`; grader
   credentials can read only a visible version under tenant context; a flat
   publish atomically creates public/version/grant/source artifact/key and a
   foreign tenant cannot grade it. Include the source-row update/promotion
   privilege path, not only unit mocks.

## Blocking concern

The current PostgreSQL implementation cannot yet be treated as the above
server contract: `upsert_flat_question` uses `ON CONFLICT ... DO UPDATE`
(`crates/learning-data-access/src/postgres/flat_question.rs:101-126`), while
the currently inspected migration grants the application role no `UPDATE` on
`workspace_flat_question_source`
(`schemas/migrations/2026080802_catalog_authoring.sql:743-749` and
`941-945`). The `SECURITY DEFINER` flat promotion function also locks
workspace rows (`2026080805_operations_analytics.sql:427-506`) and needs its
owner privileges/RLS semantics proven. Resolve these schema findings and pass
the live PostgreSQL gate before merging or exposing the HTTP route.

