# Changelog

## 2026-08-07

### Additions and New Features

- Completed MOD-CLIENT with a strict same-origin HTTP implementation of the
  browser `ApiClient`. Successful response bodies enter as `unknown` and pass
  exhaustive field-by-field decoders; malformed UUIDs, timestamps, numeric
  ranges, nested records, and discriminants fail before reaching application
  state without `any` or unchecked assertions.
- Added authenticated, bounded HTTP fallbacks for key-free response-format,
  timer, and assignment-capability evaluation. They delegate to the same pure
  Rust domain functions as WebAssembly while trusted publication, database-time
  enforcement, and correctness decisions remain on their authoritative server
  paths.
- Added exact course lookup and guarded run-screen composition across the run,
  active attempt, enrollment, assignment, course, and immutable question
  resources. The client rejects inconsistent identifiers, tenants, or repeated
  attempt cursors instead of combining independently valid but unrelated data.
- Restricted newly issued API attempt seeds to JavaScript's exact 53-bit
  integer range while retaining operating-system randomness and the internal
  generator's full `u64` contract.
- Completed MOD-API-ASSET with an immutable database registry that maps one
  route identifier to an exact typed `ObjectRecord`. Catalog assets reuse
  their logical `AssetId`; tenant-owned student-record artifacts reuse their
  `ObjectId`, preserving both model identities behind one stable route.
- Added public-CDN and protected asset delivery in
  `crates/server/src/asset.rs`. Public catalog requests bypass authentication
  and object-store signing, while protected content resolves the HttpOnly
  session, authorizes in the session-derived tenant, appends a URL-free audit
  event, and redirects to the exact stored key.
- Added the forced-RLS `asset_delivery` migration plus shared MemoryStore and
  PostgreSQL `AssetStore` implementations. Registration accepts only matching
  published `ProblemAsset` records or tenant-matching `StudentRecord` exports;
  source packages, render caches, and temporary-processing objects are not
  deliverable through the route.
- Completed MOD-API-RUN with authenticated start-or-resume, run history,
  question-attempt history, idempotent submission, enrollment, and grading
  summary routes. Students mutate only their own enrollment; course
  instructors and tenant administrators receive scoped read access.
- Added one-question-at-a-time issue and resume behavior. Each new attempt gets
  a fresh operating-system-random seed, the unresolved attempt retains its
  seed, repeated problem references remain distinct by assignment position,
  and the store lock prevents concurrent requests from starting two timers.
- Added atomic MemoryStore and PostgreSQL submission paths. Database-owned
  timestamps drive timer verdicts and completion; response, grade event, run,
  enrollment, and compact summary changes commit together, while exact retries
  return the immutable first result without grading or counting twice.
- Added the cursor-paged `listRuns` and `listAttempts` client contracts and
  aligned the run mock with the real `{ assignmentId }` start body and
  enrollment-owned route group.
- Completed MOD-GRD with the server-only
  `grading::grade(question, response, key)` contract. The generic checker
  supports all-or-nothing numeric, multiple-choice, short-text, and ordering
  questions after repeating shared response-format validation.
- Added explicit grader outcomes and errors for ungraded practice, missing or
  mismatched keys, invalid public parameters, backend-owned partial credit,
  and file uploads requiring manual review. The public result contains only
  correctness and points, never answer-bearing material.
- Completed MOD-API-COURSE with authenticated cursor-paged course and
  assignment reads plus server-owned course and assignment creation. Course
  access uses explicit course-local membership; tenant administrators use a
  separate derived authority path rather than a persisted membership value.
- Added Rust-owned `CourseId`, `CourseMembershipRole`, `CourseRole`,
  `CourseSummary`, and `AssignmentSummary` contracts. The generated TypeScript
  client now uses `CourseId` instead of an untyped string, and fixture schema 3
  records the signed-in student's course role.
- Added normalized `course` and `course_member` PostgreSQL tables, forced RLS,
  membership and assignment paging indexes, and hot `course_id` and title
  columns on assignments. Memory and PostgreSQL stores now share course-local
  visibility, assignment scoping, and validation behavior.
- Completed MOD-API-CAT with authenticated catalog browse, exact version,
  taxonomy, publication, deprecation, and archival routes. Every route derives
  its tenant from the shared session store and every list uses a bounded
  stable-key cursor.
- Expanded the initial identity contract to the plan's five one-way lifecycle
  states: draft, validated, published, deprecated, and archived. Added
  institution and public scopes, trusted backend summaries, nonempty author
  ownership, linear revision lineage, attributed forks, and database-owned
  publication timestamps.
- Added a dedicated `CatalogStore` contract with memory and PostgreSQL
  implementations. Publication locks and compares the exact validated draft,
  installs any institution visibility grant, writes hot metadata and immutable
  payload, and removes the draft in one transaction.
- Added the server-owned `BackendRegistry` and configurable
  `PublicReviewGate`. The publish route returns every capability violation,
  permits institution publishing for instructors, and restricts public
  publishing to publishers or administrators plus any configured review.
- Added Rust-owned `CatalogProblemSummary` browser types and fixture schema 2.
  Catalog and taxonomy mocks now match the cursor-paged route shapes while
  browse results omit prompts, responses, and private backend locators.
- Implemented the provider-neutral MOD-API-AUTH route group for login, current
  session, and logout. A typed `IdentityProvider` boundary permits later OIDC,
  institutional SSO, LTI, or explicit local-development integration without
  coupling credential verification to session mechanics.
- Added distinct Rust-owned `UserId` and multi-role `UserRole` contracts, with
  lower-camel serialized role names and regenerated browser types. Authenticated
  users are no longer incorrectly represented by assignment `StudentId`.
- Added a separate replica-safe `SessionStore` contract with memory and
  PostgreSQL implementations. Sessions use a 256-bit OS-random opaque cookie,
  persist only its SHA-256 hash, use backend-authoritative expiration and
  revocation, and derive tenant context only after resolving the session row.
- Added the forced-RLS `auth_session` migration and narrowly scoped `ple_auth`
  role. Route tests prove login on one replica, session use on another, and
  immediate cross-replica revocation; the PostgreSQL conformance gate reuses
  the same behavior when a dedicated test database is configured.
- Recorded permanent session-storage compliance guidance in
  `docs/HUMAN_GUIDANCE.md` and kept authentication state in an HttpOnly cookie
  rather than `localStorage`. HTTPS defaults to `Secure; SameSite=Lax`, LTI
  embedding has an explicit `SameSite=None; Secure` mode, and plain HTTP
  requires the explicit local development policy. The cookie now omits
  `Max-Age` and `Expires` so ordinary authentication ends with the browser
  session; persistent `remember me` behavior requires a separate consent and
  legal review.
- Completed MOD-SCHEMA and MOD-STO with an embedded, checksummed PostgreSQL
  migration runner and a full `PostgresStore` implementation of the frozen
  backend-neutral persistence contract.
- Added shared catalog tables, forced-RLS tenant tables, a 16-way hash split
  for problem-version payloads, and monthly partitions for question attempts,
  submissions, grade events, and audit events.
- Added an opt-in PostgreSQL execution of the reusable store conformance suite.
  It also proves an unfiltered foreign-tenant query returns zero rows and the
  student role receives PostgreSQL permission error `42501` on `answer_key`.
- Recorded adaptability as an owner design priority in
  `docs/HUMAN_GUIDANCE.md`: the system must evolve with changing requirements
  and insights while remaining functional and relevant.
- Completed MOD-OBJ with one replica-safe `S3ObjectStore` implementation shared
  by AWS S3 and MinIO, explicit three-bucket name mapping, and no SDK types in
  the backend-neutral trait.
- Added an opt-in MinIO execution of the reusable object-store conformance
  suite. The normal offline gate continues to run the same behavior against
  `MemoryObjectStore` without requiring services or credentials.
- Completed MOD-CAP with `domain::policy::validate_assignment_config`, returning
  every missing immutable-question/backend capability pair in deterministic
  question and capability order.
- Added `crates/domain/tests/capability_violation_cases.json`, a reviewed
  behavior table covering all eight capabilities, plus a key-free WebAssembly
  export, typed browser facade, API fallback contract, and server-free mock.
- Completed MOD-SCORE with `domain::scoring::score`, a pure completed-run grade
  selector, and moved the existing incremental summary projection into its
  canonical scoring owner. The frozen `domain::run` compatibility path remains
  available.
- Added typed completed-run and grade-selection values plus explicit errors for
  invalid scores, zero or duplicate run numbers, duplicate run IDs, and invalid
  instructor selections.
- Completed MOD-TIME with the clock-free `domain::timing::timer_verdict`
  contract. It evaluates untimed, open, grace-period, submitted-on-time,
  submitted-within-grace, and timed-out states from server-owned inputs.
- Added typed timer evaluation to the reviewed WebAssembly boundary, browser
  facade, API fallback contract, and server-free mock. The browser wire uses
  lower camel case while Rust items retain idiomatic snake case and upper camel
  case.
- Completed MOD-STATE with `domain::attempt::apply`, one pure state machine for
  not-started, active, submitted, correct, incorrect, retry-available,
  exhausted, timed-out, and abandoned question states.
- Implemented `crates/domain/src/completion.rs` as the owner of within-run
  completion derivation. Empty runs remain in progress; answer-all,
  all-correct, and inclusive score-threshold behavior are explicit and invalid
  fractions remain errors.
- Completed WP-C8 with `docs/CONTRACTS.md`, covering all 36 modules in the
  active-plan catalog. Every row names its contract source, one owning role,
  every direct consumer, and the allowed dependency stub.
- Classified contract sources as frozen, stubbed, or reserved so the register
  distinguishes callable interfaces from compiling boundaries and future
  source ownership without presenting later milestone behavior as complete.
- Completed WP-C9, freezing the frontend architecture in
  `docs/FRONTEND_ARCHITECTURE.md` and the Solid reactivity contract in
  `docs/SOLID_MODEL.md`. The executable contract defines all 11 product routes
  plus a separate not-found route.
- Added a browser-safe `ApiClient`, router-owned cached query runtime, and
  server-free typed mock client. The course, assignment, and active-run screens
  all use that injected boundary rather than importing fixtures directly.
- Added the single browser WebAssembly facade and a fully controlled
  multiple-choice reference widget. Number keys select and move focus, Enter
  submits, Escape returns to the assignment, and a live region announces only
  key-free response-format status.
- Added a persistent app shell with route and question error boundaries,
  explicit pending and empty states, responsive 56-pixel response targets, and
  an honest contract surface for each later M3 route.
- Added `docs/PALETTE_CONTRAST_AUDIT.md`, generated from source by the
  color-accessibility tooling. All seven source palette colors pass the 5.5:1
  repository target against white, from 6.56:1 to 16.27:1.
- Completed WP-C7 with a Rust-owned fixture generator, a reviewed published
  problem corpus, and a dependency-free mock API. The corpus contains two
  checksummed SVG assets, one draft, one published problem version, one
  assignment reference, one enrollment, three completed runs, and one
  in-progress run.
- Added full attempt reproducibility records to all four fixture runs, including
  adapter, generator, source artifact, asset object, grading implementation,
  parameter hash, rendered-question hash, and a distinct fresh seed for every
  issued instance.
- Added mock handlers for all five planned API route groups: authentication,
  catalog, course and assignment, run and grading, and authorized assets. The
  fetch replacement has no network fallback, so missing mocks fail locally.
- Completed WP-C6, establishing the browser/server grading boundary.
  `grading::AnswerKey` now holds every answer-bearing value, while
  `domain::validation::validate_response_format` provides key-free structural
  checks callable through the WebAssembly bridge.
- Added an exact processed-WebAssembly export allowlist and a conservative
  workspace dependency-closure test. The shipped `wasm_bridge` closure is
  exactly `wasm_bridge`, `domain`, and `question_model`; it contains no
  `grading` crate.
- Added `docs/SECURITY_MODEL.md`, documenting code placement, the key-free
  browser surface, the answer-bearing server surface, and the evidence required
  before a new WebAssembly export is accepted.
- Completed WP-C5, implementing the deterministic seeded parameter generator
  and a shared native/browser parity harness. The committed corpus covers all
  current integer, decimal, choice, and fixed branches with 65 seeds for
  `parameter-map@1`, including the full `u64` seed endpoint.
- Added `GeneratorReference`, which keeps a generator's stable ID and additive
  version together in published randomization definitions, generated variants,
  and attempt provenance.
- Added `docs/DETERMINISM_CONTRACT.md`, a maintained seed-vector regeneration
  example, a version-matched local `wasm-bindgen-test` setup script, and a real
  headless-Chromium parity runner under `tests/playwright/`.
- Completed WP-C4, freezing the backend-neutral `Store` and `ObjectStore`
  contracts. `Store` covers drafts, immutable published versions, assignments,
  enrollments, runs, question attempts, and compact summaries; every list
  operation uses a bounded cursor request.
- Added reusable store and object-store conformance suites, initially run
  against `MemoryStore` and `MemoryObjectStore`. Later PostgreSQL, MinIO, and S3
  implementations must pass these same suites.
- Added typed object buckets and semantic keys derived only from problem,
  version, asset, tenant, seed, and object identities. Object writes compute
  SHA-256 metadata, reads verify the bytes, and signed URLs apply the documented
  60-minute content, 5-minute student-record, and never-served temporary rules.
- Added `docs/HUMAN_GUIDANCE.md` and linked it from `AGENTS.md`, preserving owner
  decisions about plan status, Codex versus Claude guidance, local Podman use,
  repeated practice, tenant/content boundaries, server-only grading, and
  measured Rust/WebAssembly performance choices.
- Completed WP-C3, the enrollment, run, question-attempt, policy, and compact
  summary contract (MOD-RUN). Every educational-record type carries a direct
  `TenantId`, enrollment completion derives from its first-completion record,
  and assignment runs deliberately carry no stored completion boolean.
- Added `crates/domain/src/run.rs` with pure within-run completion,
  continued-practice eligibility, and transactional summary projection rules.
  Added `crates/domain/tests/run_31.rs`, which drives 31 completed runs and
  compares the final projection with a hand-computed 31-run, 93-attempt value.
- Added 19 Rust-owned activity definitions to `generated/api/`, including
  `AssignmentEnrollment`, `AssignmentRun`, `QuestionAttempt`, and
  `StudentAssignmentSummary`. `AttemptProvenance` captures the full plan-defined
  implementation, object, and checksum record needed to reproduce an attempt.
- Added `docs/ACTIVITY_MODEL.md`, documenting the three-level record model,
  tenant ownership, independent policies, derived completion, summary
  projection, and WP-C3 gates.

### Behavior or Interface Changes

- Public asset redirects now carry an immutable one-year cache policy and the
  object's SHA-256 ETag. Protected redirects are `no-store`, suppress the
  referrer, and are independently rejected if the object backend exceeds the
  60-minute `content` or 5-minute `student-records` lifetime.
- Missing, cross-tenant, and unauthorized protected assets share the same
  not-found response. Signed URLs remain response headers only; audit payloads,
  browser storage, persisted markup, and JSON responses never contain them.
- Enrollment authorization now uses an explicit `UserId` distinct from the
  institution's `StudentId`. `QuestionAttempt` now carries a zero-based
  `assignmentPosition`, allowing repeated problem/version references and their
  retries to remain separate on the lower-camel browser wire.
- Run submission responses reveal correctness and points only when the
  question's feedback policy permits them. Deferred results remain hidden
  until run completion and release-gated results remain hidden; answer keys,
  expected values, private rubrics, and checker state remain server-only.
- The run API issues every never-attempted assignment position once before
  selecting an allowed retry. At most one unresolved attempt exists in a run,
  and a correct position cannot be retried.
- The grading contract now takes the exact published question definition
  rather than the compact persisted attempt. The run service resolves that
  definition from the attempt's immutable problem/version reference before
  grading, so tolerance and points are not duplicated into activity rows.
- Generic grading deliberately refuses partial credit without a capable
  backend or explicit private rubric, and refuses to fabricate a score for a
  file upload requiring review. Ungraded practice produces an explicit
  ungraded outcome rather than a false correctness result.
- Assignments now belong to exactly one `CourseId`, require a nonempty bounded
  title and at least one exact published version, and are listed only through
  their parent course. Assignment writes reject missing courses and catalog
  versions that are hidden or not assignable; they never copy a question
  payload into a tenant record.
- Coarse instructor status no longer implies access to every tenant course.
  Direct students may read their course assignments, direct instructors may
  manage them, tenant administrators may access every tenant course, and
  nonmembers receive the same not-found response as an absent course.
- Removed the generic direct-publication store method. All publication writes
  now pass through the atomic draft-to-catalog transition, so callers cannot
  bypass draft comparison, visibility grants, ownership, or lineage rules.
- Context-free catalog reads now expose public content only. Institution
  versions require the session-derived `TenantContext` and an exact
  tenant/problem/version grant under forced PostgreSQL row-level security.
- Deprecation now follows the active plan's soft-withdrawal contract: the
  version disappears from browse but remains assignable by an exact reference.
  Archival additionally blocks new assignments; both states remain exactly
  resolvable for historical records.
- The PostgreSQL application role can insert catalog records and update only
  lifecycle fields. It cannot update or delete immutable problem identity,
  payload, scope, backend, capabilities, metadata, authorship, or lineage.
- Taxonomy paging now uses collision-free hex-encoded scheme/code cursor keys,
  so slashes inside either controlled-vocabulary field cannot merge distinct
  terms.
- PostgreSQL store operations now assume the non-superuser, non-bypass
  `ple_app` role inside each transaction. Tenant operations set
  `ple.tenant_id` with transaction-local scope so pooled connections cannot
  retain another request's context.
- Activity writes lock their run, enrollment, and summary records as needed and
  commit the immutable activity plus compact grade projection atomically.
  Stable-key cursor queries use no positional offset.
- Complete persistence records now serialize as lower-camel JSONB with a
  SHA-256 checksum while normalized identity, relationship, timestamp, and
  paging columns retain database constraints and indexes.
- S3-compatible object writes now use `If-None-Match: *`, so concurrent or
  repeated puts cannot overwrite immutable content. Reads reconstruct the full
  object record from encoded metadata and reject semantic-key, bucket,
  category, version, media-type, size, or SHA-256 mismatches.
- S3-compatible signed URLs now confirm that the exact object record exists and
  use the server-supplied timestamp as the signing start time. Content remains
  valid for 60 minutes, student records for 5 minutes, and temporary objects
  remain unsignable.
- Seeded generation, graded work, partial credit, immediate hint feedback, and
  per-question timing now imply their backend requirements directly from each
  question definition. Client rendering, print export, and offline preview are
  explicit assignment-wide requirements applied to every selected question.
- Duplicate requirements collapse to one violation. The assignment editor and
  future publish route receive the same lower-camel question/capability result
  from one Rust implementation.
- First and latest grading now select by one-based run number rather than input
  order. Highest-score ties keep the earlier run, and instructor-selected
  grading remains empty until an instructor names a completed run.
- `store` now consumes the MOD-SCORE projection owner directly; WP-C3 callers
  may continue using the compatibility re-export from `domain::run`.
- Timed deadlines and grace boundaries are inclusive. Authorized pause time is
  reconstructed from server audit events and extends the base deadline before
  grace is applied; browser time remains display-only and cannot change the
  verdict.
- Malformed timer records now fail explicitly for policy/deadline mismatch,
  negative pause duration, timestamp ordering errors, and arithmetic overflow.
- Grading may no longer skip the submitted state, retry policy must resolve an
  incorrect response to retry-available or exhausted, and terminal attempt
  states refuse later events. Starting a retry authorizes a new attempt with a
  fresh seed and never mutates the earlier attempt record.
- Within-run completion now lives at
  `domain::completion::derive_within_run_completion`; `domain::run` retains a
  compatibility re-export for existing WP-C3 consumers.
- Frozen contract changes must now update the register, owning source, every
  named consumer and stub, generated projections, affected evidence, and the
  changelog atomically. Producer-first changes with later consumer repair are a
  blocking contract-gate failure.
- The built mock-backed client now supports the complete first-success path:
  course list, assignment list, assignment overview, resume the active run,
  validate one response in Rust/WebAssembly, and submit through the typed API
  boundary without a running backend.
- `src/wasm/index.ts` explicitly initializes the generated module from
  `ple_bridge_bg.wasm`. If initialization fails, the same facade uses the typed
  server-format fallback and displays one persistent degraded-mode notice; it
  never falls back to browser grading.
- Browser-facing generated data remains lower camel case while Rust modules,
  fields, and raw wasm-bindgen exports remain snake case behind their adapters.
  Components call only the lower-camel `validateResponseFormat` facade.
- JavaScript and CSS now receive independent content hashes in the built HTML,
  so a stylesheet-only change cannot reuse a stale JavaScript-derived cache
  key.
- `./build.sh` and `./check_codebase.sh` now verify the tracked fixture bytes
  against Rust types before regenerating their ignored TypeScript projection.
  The projection uses `satisfies MockFixtureCorpus`, so MOD-QM wire changes fail
  TypeScript compilation instead of silently drifting into mock data.
- Prettier validation now explicitly uses `.prettierignore`, keeping ignored
  `generated/api/` and `generated/fixtures/` inside the formatting gate while
  excluding only disposable `generated/wasm-export-check/` glue.
- Response-format violations now cross the Rust/JavaScript boundary in lower
  camel case. An exact serialization test covers both the tagged variant
  (`textTooLong`) and its data fields (`maxLength` and `actualLength`), and the
  generated Node bridge exercises the same JSON contract end to end.
- Moved executable response-shape validation out of `question_model` and into
  the key-free `domain` crate. The question model remains declarative;
  correctness checks and private rubrics remain server-only in `grading`.
- Seeded generation now uses `rand_chacha::ChaCha20Rng`, expands each `u64`
  seed through a domain-separated SHA-256, iterates output-bearing maps in
  `BTreeMap` order, samples inclusive ranges without modulo bias, and formats
  decimals as exact fixed-precision strings.
- Fresh practice takes priority over replay: every newly issued parameterized
  question instance receives a fresh server-owned seed, while resume and
  re-render of that same attempt retain its recorded seed.
- Moved reproducible TypeScript API definitions out of tracked `src/` and into
  ignored root `generated/api/`. Both `./build.sh` and `./check_codebase.sh`
  regenerate them from the tracked Rust model before consumption; TypeScript,
  ESLint, and Prettier still validate the resulting files. The generator
  removes stale files only when its ownership header is present.
- Added an explicit, non-defaultable `TenantContext` to every tenant-owned
  persistence operation. The memory backend keys each educational record by
  tenant and returns no rows when queried through another tenant's context.
- Activity writes and their summary projection are atomic in the store
  contract. Completing a run now updates its enrollment's first-completion,
  current-grade, and best-grade pointers in the same operation; subsequent
  policy-permitted practice runs remain startable.
- Object metadata, store errors, and object-store errors expose no PostgreSQL,
  SQLx, AWS SDK, or S3 implementation types.
- Aligned the run-policy wire contract with the active plan. Continued practice
  is unlimited, capped, or closed; grade policy is first, latest, highest, or
  instructor-selected; and variation uses new seeds, selected problem variants,
  or full regeneration.
- Aligned feedback disclosure with the four server-evaluated modes in the plan:
  immediate full teaching feedback, immediate correctness without an answer,
  deferred feedback, and instructor release.
- `StudentAssignmentSummary` now maintains current, best, and latest scores,
  completed-run count, total question attempts, and last activity without a
  synchronous scan of historical attempts. Out-of-order activity cannot move
  the last-activity timestamp backward.

### Fixes and Maintenance

- Repeated key-free response-format validation at the run server boundary
  before invoking an adapter, and added store-side rejection of nonfinite,
  negative, over-awarded, or zero-possible backend point results.
- Restricted the submission-idempotency table to application `SELECT` and
  `INSERT` privileges. The first receipt is immutable, changed keys or
  responses conflict, and unauthorized attempt owners receive a nonrevealing
  not-found result.
- Removed the vulnerable `rustls-webpki` 0.101.7 dependency path reported by
  three GitHub security alerts. The S3 dependency now selects only the modern
  HTTP 1.x TLS client, and the all-features graph resolves `rustls-webpki`
  0.103.13 with no legacy Rustls 0.21 client.
- Refreshed direct Rust and TypeScript dependencies to current stable releases
  with open minimum-version requirements. Major migrations include axum 0.8,
  rand_core/rand_chacha 0.10, SHA-2 0.11, syn 3, and SQLx 0.9; TypeScript uses
  the compatible open range `>=6.0.3 <7`.
- Moved the Rust toolchain to the floating stable channel and made the
  wasm-bindgen test setup derive its matching runner version from Cargo rather
  than carrying a static version.
- Synchronized all Cargo workspace packages to the repository's `26.08`
  CalVer release, represented as SemVer `26.8.0` in Cargo and npm metadata.
- Updated current container configuration to floating Rust, Debian, and
  PostgreSQL images. The PostgreSQL volume now mounts `/var/lib/postgresql`,
  the path required by PostgreSQL 18 and later official images, and SQLx uses
  its current Rustls/AWS-LC TLS backend.
- Added a lockfile boundary test requiring every resolved `rustls-webpki`
  release to include all three reported fixes.
- Moved the pure route table into `src/route_contract.ts` so Node contract tests
  do not import Solid Router's client-only implementation. `src/routes.ts`
  derives the executable browser definitions from that single table.
- Changed mock submission acknowledgement to return the in-progress attempt
  with no result, avoiding disclosure of an unrelated historical correctness
  result in the reference flow.
- Fixed number-key navigation to move focus with selection, prevented the
  narrow header from clipping its third navigation item, distinguished neutral
  idle validation from success, and applied the focus outline to the whole
  choice target.
- Excluded disposable `generated/wasm-export-check/` bindgen glue from ESLint
  and Prettier while retaining typecheck, lint, and formatting coverage for the
  regenerated `generated/api/` contract.
- Fixed the TypeScript generator's union wrapping to account for the actual
  declared type name and Prettier's intermediate line-break form. The longer
  `FeedbackDisclosure` union now passes Prettier unchanged, with a focused
  generator regression test.

### Decisions and Failures

- Schema migration and application access remain separate privilege domains.
  The migration connection creates roles and schema objects; a production
  runtime login may assume `ple_app` but must not be superuser or bypass RLS.
- The PostgreSQL conformance test requires an explicit
  `PLE_POSTGRES_TEST_URL` pointing to a dedicated empty database. It is compiled
  in the normal all-features gate but remains ignored until the later local
  service pass; no `psql` client or port 5432 listener was available in this
  work session, and deployment debugging remains out of this lane.
- SQLx 0.9's public meta-crate records optional MySQL and SQLite macro-driver
  packages in `Cargo.lock` even with default features disabled. The compiled
  all-features workspace graph contains only `sqlx-core` and `sqlx-postgres`;
  using semver-exempt internal SQLx crates would require the exact pins the
  owner has prohibited.
- The frozen `ObjectStore::get` contract returns the full `ObjectRecord`, so
  the S3-compatible backend stores an encoded record with the object instead of
  keeping replica-local metadata. Base64 0.23.1 is optional with the `s3`
  feature and uses an open `>=` manifest requirement.
- A bounded live MinIO attempt stopped after `podman info` succeeded but the
  compose provider could not resolve its selected machine connection. No
  container deployment debugging followed; the compiled opt-in conformance
  test remains the exact live gate for the later deployment pass.
- npm initially selected TypeScript 7 because it is the registry's latest
  release, but the current frontend toolchain requires TypeScript 6. The
  manifest now permits every compatible 6.x release instead of pinning 6.0.3.
- SQLx 0.9 removed the combined `runtime-tokio-native-tls` feature; selecting
  its separate Tokio and Rustls/AWS-LC features restored the intended runtime
  and removed the old native-TLS dependency path.
- rand_core 0.10 renamed the generator's imported trait to `Rng`, and SHA-2
  0.11 removed hexadecimal formatting from its output array. Focused API
  migrations preserved the existing seed vectors and exact lowercase fixture
  checksums.
- MOD-DEPLOY is an implicit whole-system consumer in the register rather than
  being repeated in every consumer cell. Reserved paths for schema, worker,
  statistics, retention, LTI, and production deployment are explicit ownership
  commitments, not claims that those later modules are implemented.
- The first frontend Node test imported `src/routes.ts` and correctly failed
  because Solid Router called a client-only API without a DOM. Separating pure
  route data from browser assembly restored a real Node boundary instead of
  emulating a browser in unit tests.
- The first Chromium response-flow run exposed a degraded fallback: the current
  wasm-bindgen loader does not infer its `.wasm` URL when called without an
  argument. Passing the same-origin URL explicitly made Chromium request both
  bridge files and restored local Rust validation.
- A later Chromium launch was denied by the macOS command sandbox before any
  page opened. The identical repository runner passed with browser permission;
  this was an environment denial, not an application correction.
- Independent desktop and 320-pixel visual review used ignored artifacts under
  `generated/ui/`, keeping derivative screenshots out of Git while preserving
  local evidence.
- The committed JSON and SVG fixture corpus is intentional, reviewed work
  evidence. Its TypeScript projection is fully derivative and remains ignored
  under root `generated/`, matching the repository owner's artifact rule.
- Focused WP-C7 validation caught three issues before the complete gate: SVG
  colors required a wider Rust raw-string delimiter; a `satisfies` expression
  made the asset-body map too narrow for URL lookup; and ESLint rejected an
  `async` mock function with no `await`. Each was corrected at its owning
  boundary without weakening strict checks.
- Prettier's default Git-ignore behavior showed that ignored generated API
  files were not actually being formatted despite the earlier gate comment.
  Selecting `.prettierignore` explicitly made the stated policy executable.
- The first WP-C6 complete gate reached ESLint after the export test generated
  bindgen inspection glue, then correctly failed on two unused variables in
  that derivative file. Narrowly excluding only the disposable export-check
  directory made the focused lint check and the otherwise unchanged full gate
  pass; generated API definitions remain validated.
- The seed table remains Git-tracked as a deliberately reviewed compatibility
  baseline and work record; ordinary reproducible build output remains ignored
  under root `generated/`.
- The first Chromium launch was denied by the macOS command sandbox before any
  test ran; the identical local runner passed with browser permission. The
  first full pytest run then identified its Playwright import under
  `tests/e2e/`; moving it to the required `tests/playwright/` tier made the
  targeted and complete gates pass.
- The first complete gate run stopped in `cargo:clippy` with `E0463` while
  loading `aws-sdk-s3` dependencies. The manifest, lockfile, and a clean
  temporary build were correct, isolating stale metadata in `target/`; a
  package-scoped rebuild of `aws-sdk-s3`, `arc-swap`, and `rustversion` repaired
  the cache, and the unmodified gate then passed.

### Developer Tests and Notes

- MOD-CLIENT passes five focused HTTP behavior tests, strict TypeScript
  compilation, two authenticated validation-route tests, the JSON-safe fresh
  seed test, focused warning-free Rust Clippy, and diff hygiene. The complete
  `./check_codebase.sh` gate passes all 11 stages with 20 Node tests and 20
  server tests; live PostgreSQL and MinIO checks remain deferred to the later
  deployment pass.
- `cargo test -p store --all-features --test conformance` passes five memory
  tests with the opt-in PostgreSQL case ignored. The new reusable asset suite
  covers public, institution, student-record, foreign-tenant, duplicate, and
  forbidden temporary deliveries, and the memory audit proves only authorized
  protected requests append events.
- `cargo test -p server_core asset --all-features` passes three asset-route
  tests covering CDN bypass, session authorization, nonrevealing denial,
  auditing, cache controls, and exact bucket lifetimes.
- `./check_codebase.sh` passes all 11 stages after MOD-API-ASSET, including
  TypeScript generation and checking, Node contracts, crate boundaries, Rust
  formatting, warning-free all-target Clippy, workspace tests, and doctests.
  All 812 repository pytest checks and the five-stage debug build pass as the
  complete task gate; live PostgreSQL and MinIO remain explicit opt-in service
  checks for the later deployment pass.
- MOD-API-RUN passes 14 server behavior tests, the reusable memory/PostgreSQL-
  compiled store conformance suite, 14 typed client/mock tests, focused
  all-feature Clippy with warnings denied, and generated fixture/type checks.
  The route tests cover authorization, format refusal before grading, exact
  replay, changed-request conflicts, fresh seeds, one-active-question advance,
  feedback redaction, and cursor-paged run history.
- The complete MOD-API-RUN gate passes all 11 `check_codebase.sh` stages, all
  812 repository pytest checks, and the five-stage debug build including native
  Rust, WebAssembly, generated types, fixture verification, and the Solid
  client bundle. Live PostgreSQL execution remains the explicit opt-in service
  gate for the later deployment pass.
- MOD-GRD passes six focused behavior tests covering all four numeric
  tolerance modes, choice-set comparison, text normalization, ordering,
  malformed response/key combinations, invalid public parameters, ungraded
  practice, backend-owned partial credit, and manual review. Package-scoped
  all-target, all-feature Clippy passes with warnings denied.
- The complete MOD-GRD per-patch gate passes all 11 `check_codebase.sh`
  stages, all 812 repository pytest checks, and the five-stage debug build.
  The compiled browser closure and processed WebAssembly allowlist remain
  answer-key-free.
- MOD-API-COURSE passes the memory/PostgreSQL-compiled store conformance suite,
  12 server library tests, 32 question-model tests plus two doc tests, eight
  generator tests, eight focused TypeScript/mock behavior tests, and focused
  all-feature, all-target Clippy with warnings denied. The live PostgreSQL
  execution remains an explicit opt-in service gate.
- The complete MOD-API-COURSE per-patch gate passes all 11
  `check_codebase.sh` stages, all 812 repository pytest checks, and the
  five-stage debug build including native Rust, WebAssembly, generated types,
  fixture verification, and the Solid client bundle.
- MOD-API-CAT focused validation passes 31 question-model unit tests and two
  doc tests, the memory catalog conformance suite, all 11 server library tests,
  eight generator tests, 11 TypeScript/mock behavior tests, and 251 Markdown
  link and ASCII checks. Focused all-feature Clippy passes with warnings denied;
  the PostgreSQL execution remains an explicit opt-in service gate.
- The complete MOD-API-CAT per-patch gate passes all 11
  `check_codebase.sh` stages, all 812 repository pytest checks, and the
  five-stage debug build including native Rust, WebAssembly, generated types,
  fixture verification, and the Solid client bundle.
- MOD-STO passes the reusable memory conformance suite and compiles its ignored
  PostgreSQL conformance, forced-RLS, and answer-key grant gate. Focused
  all-feature, all-target Clippy passes with warnings denied.
- The combined MOD-OBJ/MOD-SCHEMA/MOD-STO/MOD-API-AUTH offline gate passes all
  11 `check_codebase.sh` stages, all 812 repository pytest checks, focused auth
  and session tests, and the five-stage build. The live PostgreSQL and MinIO
  gates remain intentionally deferred to the local-service pass.
- MOD-OBJ passes `cargo test -p objects --all-features` with four unit tests,
  memory conformance, and the compiled opt-in MinIO test; focused all-target
  Clippy also passes with warnings denied.
- The MOD-OBJ complete offline gate passes all 11 `check_codebase.sh` stages,
  all 804 pytest checks, and the five-stage build. The live MinIO conformance
  test is intentionally pending the later container-connection pass.
- The dependency refresh passes all 11 `check_codebase.sh` stages, including
  TypeScript checks, ESLint, Prettier, Node tests, WebAssembly dependency
  closure, rustfmt, Clippy with warnings denied, all workspace tests, and doc
  tests. `npm audit` reports zero vulnerabilities, and static compose expansion
  confirms the current PostgreSQL volume path and required run-time secrets.
- MOD-CAP's committed table covers all eight capability variants, no-requirement
  behavior, multiple simultaneous gaps, deterministic ordering, and duplicate
  suppression. The published fixture returns exactly `algorithmicGeneration`
  and `serverGrading` through the built Node WebAssembly bridge.
- The MOD-CAP complete gate passed all 11 `check_codebase.sh` checks, all 798
  pytest checks, all 33 domain unit tests, the processed WebAssembly allowlist,
  TypeScript strict checking, ESLint, and the mock fallback behavior test.
- MOD-SCORE's hand-computed 0.4, 0.9, 0.7 fixture selects 0.4 for first, 0.7
  for latest, and 0.9 for highest through both batch reconciliation and
  incremental projection. Tie, instructor-selection, malformed-input, and
  monotonic-activity cases also pass.
- The MOD-SCORE complete gate passed all 11 `check_codebase.sh` checks, all 798
  pytest checks, and all 30 domain unit tests plus both domain integration
  tests.
- MOD-TIME table-driven tests cover untimed, open, grace, late, pause-adjusted,
  malformed, overflow, and inclusive-boundary cases. Native Rust, the mock
  fallback, and the built Node WebAssembly bridge all returned the same
  lower-camel verdicts; the reviewed export allowlist includes `timer_verdict`.
- The MOD-TIME complete gate passed all 11 `check_codebase.sh` checks, all 715
  pytest checks, the 314-check documentation gate, and the five-stage build.
- `cargo test -p domain`: 26 unit tests plus the 31-run and seed-vector
  integration tests pass. The state table covers all 10 legal transitions,
  illegal grading, terminal-state refusal, and exact lower-camel serde values.
- The contract register and active-plan catalog each contain exactly 36 module
  rows. The focused ASCII, Markdown-link, and whitespace gate passes 314 tests.
- `node --import tsx --test tests/test_frontend_contract.mjs`: 5 contract tests
  pass for the exact route table, server-free run screen, key-free format and
  timer fallbacks, and answer-bearing-type exclusion.
- `./run_playwright_tests.sh tests/playwright/frontend_contract.spec.ts`: 3
  Chromium tests pass for all 11 product routes, course-to-submission keyboard
  flow through the real WebAssembly module, focus movement, 56-pixel targets,
  and the 320-pixel no-overflow baseline.
- `./build.sh`: the complete five-stage build passes in 4.01 seconds observed;
  the built document independently fingerprints `main.js` and `style.css`.
- The color-accessibility audit measures all seven source colors as PASS at the
  5.5:1 target against white; the narrowest ratio is success green at 6.56:1.
- `cargo test -p domain --test test_determinism -- --nocapture`: all 65
  committed vectors pass natively, with failures designed to name the first
  divergent seed.
- `node tests/playwright/e2e_wasm_determinism.mjs`: the same 65 vectors pass in
  headless Chromium through `wasm-bindgen-test` (1 passed, 0 failed).
- The `wasm32-unknown-unknown` no-run gate compiles the browser determinism test.
- Package-scoped Clippy passes for the question model, domain, WASM bridge, and
  store with all targets, all features, and warnings denied.
- `cargo test -p objects -p store`: both memory conformance suites, checksum
  tamper detection, and bounded-pagination validation pass.
- `cargo clippy -p objects -p store --all-targets --all-features -- -D warnings`:
  passes.
- `cargo test -p question_model`: 30 unit tests and 2 doc tests pass after
  moving executable format behavior to `domain`.
- `cargo test -p domain`: 13 unit tests plus the dedicated 31-run and seed-vector
  integration tests pass.
- `./pipeline/build_wasm.sh && node tests/e2e/e2e_wasm_bridge.mjs`: the Node
  bridge returns the Rust-owned version and the expected camel-case validation
  report.
- `node --test tests/test_wasm_export_allowlist.mjs`: the processed module's
  complete export list matches the reviewed allowlist.
- `source source_me.sh && python3 -m pytest -q tests/test_crate_boundaries.py`:
  the exact browser workspace closure passes and excludes `grading`.
- `cargo test -p xtask`: 8 generator tests pass, including fixture-contract
  shape, answer-key exclusion, and safe stale-output cleanup.
- `node --import tsx --test tests/test_mock_handlers.mjs`: 4 mock behavior tests
  pass with no API server running.
- `./build.sh`: the five-stage debug build passes, including fixture checking
  and projection generation; total observed time was 6.25 seconds.
- `./check_codebase.sh`: all 11 gate steps pass, including fixture drift, mock
  behavior, WebAssembly export allowlist, and crate-closure boundaries.
- `source source_me.sh && python3 -m pytest tests/`: 715 passed in 0.94 seconds.

## 2026-08-06

### Additions and New Features

- Brought the M0 foundation to a compiling, gated state across both toolchains:
  the 13-crate Cargo workspace, the Solid browser client, the WebAssembly build
  path, the container stack, and the extended check gate.
- Added `crates/server/src/health.rs`: readiness is a pure function over probe
  results, plus `probe_over_http`, which lets the API binary health-check
  itself so the runtime image needs no `curl` or `wget`.
- Added real dependency probes behind `/health`: `store::postgres::ping` issues
  `SELECT 1`, and `objects::minio::probe_bucket` issues `HeadBucket`. The
  endpoint returns 200 only when both answer and 503 naming the failing
  dependency otherwise.
- Added `containers/Containerfile.api` (two-stage build, `--locked`, non-root
  runtime user) and `containers/compose.yaml` (PostgreSQL 17, MinIO, named
  volumes, one-shot creation of the `content`, `student-records`, and
  `temp-processing` buckets), with `containers/env.example` as the credential
  template.
- Added `pipeline/build_wasm.sh`, which compiles `crates/wasm` for
  `wasm32-unknown-unknown` and emits both `wasm-bindgen` flavors into the
  gitignored `dist_wasm/` staging directory: `web` for the browser client and
  `node` for the WP-F2 gate.
- Added `pipeline/build.mjs`, the esbuild JavaScript-API build for the Solid
  client, with content-hash cachebusting on the script and stylesheet URLs.
- Added `rust-toolchain.toml` pinning the compiler to 1.96.0 with the
  `wasm32-unknown-unknown` target, and a repo-root `Brewfile` for `podman`.
- Added `crates/xtask`, the workspace-local `wasm-bindgen` glue generator. It
  is build tooling, outside the product boundary, and no shipped crate depends
  on it.
- Added `tests/e2e/e2e_run_all.sh` as the non-browser E2E runner. It carries
  the `e2e_` prefix because `tests/test_test_naming_conventions.py` requires it
  for shell files under `tests/e2e/`, which takes precedence over the
  `run_all.sh` name suggested in `docs/E2E_TESTS.md`.
- Added `src/log.ts` as the single logging surface for `src/`, `src/app.tsx` as
  the app shell, and base styling in `src/style.css`.
- Added `tests/e2e/e2e_wasm_bridge.mjs`, which loads the generated bridge in
  Node and asserts a value that crossed the Rust boundary.
- Added `docs/CODE_ARCHITECTURE.md`, `docs/CONTAINER.md`, and
  `docs/MACOS_PODMAN.md`.
- Added `docs/active_plans/m0-results.md`, the evidence record for M0: which
  plan assumptions held, which were disproved, what only running the steps
  revealed, the first build and artifact measurements, and the claims that
  remain untested.
- Added `build.sh`, the single master build. It runs rust, wasm, tsgen, and
  client in dependency order and reports per-stage timing, because the useful
  question is where the whole build spends its time and cargo only sees its own
  stage.
- Added `crates/xtask/src/tsgen.rs`, the repo's own TypeScript generator. It
  parses the question model with `syn` and emits Prettier-shaped TypeScript
  into `src/api/generated/`.
- Completed WP-C1, the question model and taxonomy (MOD-QM). `question_model`
  now carries identity (`WorkspaceId`, `ProblemId`, `VersionId`, `AssetId` as
  distinct newtypes), `Capability` and `BackendCapabilities`, answer shapes
  (tolerance, text matching, selection cardinality), `ResponseDefinition` and
  `StudentResponse`, `ContentBlock` and `QuestionEnvelope`, generation specs,
  the question and run policies, and `QuestionDefinition` itself. 34 types, 23
  unit tests, 2 doc tests.
- Completed WP-C2, identity and lifecycle (MOD-ID). `Lifecycle` has three
  states (draft, published, withdrawn), and every change passes through one
  fallible `apply` function. Publishing is the transition that assigns a
  `ProblemId`; republishing a published version is refused, because changing
  published content means publishing a new version. Withdrawal keeps existing
  assignments working and stops new ones.
- Added `docs/PROBLEM_IDENTITY.md`, the WP-C2 document: the four identifiers,
  the draft rule, the transition table, and why immutability pays for itself.
- Added `docs/QUESTION_MODEL.md`, the WP-C1 contract document: what belongs in
  the model versus in `grading`, the wire format, and the Rust-to-TypeScript
  mapping table.
- Generated TypeScript for all 34 boundary types into `src/api/generated/`,
  passing `tsc --noEmit`, ESLint, and `prettier --check` unchanged.
- Added `.cargo/config.toml` with a `cargo tsgen` alias. Committed, not
  ignored: it is project configuration, and cargo's caches live in `~/.cargo`.

### Behavior or Interface Changes

- `check_codebase.sh` now runs three Rust steps after the TypeScript steps:
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace`. A missing `cargo` emits a loud SKIP, matching
  the script's existing honesty convention.
- `build_github_pages.sh` now delegates to `node pipeline/build.mjs`, because
  the esbuild CLI cannot load `esbuild-plugin-solid`.
- The workspace moved to edition 2024 (`rust-version` 1.85 already allowed it)
  and every member now inherits `version`, `edition`, and `rust-version` from
  `[workspace.package]`.
- `crates/domain` declares `chrono` with `default-features = false`, dropping
  the `clock` feature. The plan's load-bearing property is that `domain` has no
  clock; with default features it had one.
- `sqlx` moved from `crates/server` to `crates/store`, and `aws-sdk-s3` is
  reached only through `crates/objects`. Both are feature-gated, and
  `crates/server` names neither: it uses the `store::postgres::Pool` and
  `objects::minio::S3Client` aliases instead.
- `package.json` placeholders are filled: name `peptidyle-learning-engine` and
  version `26.8.0`. `clean` now points at `devel/clean_build.sh` and a new
  `clean:dist` points at `devel/dist_clean.sh`.
- `solid-js` and `@solidjs/router` moved from `devDependencies` to
  `dependencies` with `>=` pins, matching the policy in
  `docs/TYPESCRIPT_STYLE.md`.
- `tsconfig.json` gained `"jsx": "preserve"`, `"jsxImportSource": "solid-js"`,
  an `exclude` list covering `OTHER_REPOS`, `target`, `dist`, and `dist_wasm`,
  and an `include` list that covers `.tsx`.
- `src/index.html` loads `main.js` and `style.css` by relative path. A leading
  slash resolves to the user-site root on a GitHub Pages project site and 404s.

### Fixes and Maintenance

- Fixed five defects that stopped the workspace from compiling at all: a single
  `/` used as a comment in `crates/objects/src/lib.rs`, Python `#` comments in
  `crates/store/src/lib.rs`, unmarked prose after `pub mod docx;` in
  `crates/export/src/lib.rs`, a literal `\n` escape pasted into
  `crates/server/src/main.rs`, and a call to `anyhow::Result` with `anyhow`
  undeclared.
- Fixed `src/main.tsx`, which called `createRoot(element, <App />)`.
  `solid-js/web` exports `render`, and it takes a component function; the old
  call would have rendered a static snapshot outside a reactive root.
- Renamed `src/App.jsx` to `src/app.tsx`. As a `.jsx` file it matched neither
  the ESLint nor the Prettier glob, so the only component in the app sat
  outside every gate.
- Fixed `tsconfig.lint.json`, whose `include` list matched no files and made
  `tsc` exit 2 with TS18003, which in turn broke typed linting of
  `playwright.config.ts`.
- Stripped trailing whitespace from the six `.toml` files that carried it, and
  reformatted every `.rs` file with rustfmt.
- Added `containers/env.local` and `.env` to `.gitignore`.

### Removals and Deprecations

- Removed three temporary files from the index that were never meant to be
  tracked: `crates/adapters/webwork/CARGOFILEtmp.txt`,
  `crates/adapters/webwork/Cargo.tmp`, and `pipeline/build.mjs.tmp`.
- Reconciled ten paths that were staged as added while deleted from the working
  tree, so a commit could not resurrect files that had been removed.
- Removed 16 empty directories accidentally created at the repository root
  (`dy/`, `e/`, `ea/`, `g-e/`, `gi/`, `i/`, `in/`, `LE/`, `le-l/`, `MS/`, `n/`,
  `ne/`, `pt/`, `rc/`, `rn/`, `ROB/`). Git cannot see empty directories, so
  they would never have appeared in `git status`.
- Removed the untracked root symlink `implementation_plan.md`, which duplicated
  `docs/active_plans/implementation_plan.md`.

### Decisions and Failures

- The prior M0 state was reported as complete but had never passed a gate: the
  Rust workspace did not compile, the Solid entry point called an API that does
  not exist, and `pipeline/build.mjs` was an empty file. Recorded here because
  the lesson is procedural rather than technical: run the gate before reporting
  the milestone.
- `npm run clean` points at `devel/clean_build.sh`, not `devel/dist_clean.sh`
  as WP-F3 specifies. `dist_clean.sh` is the deep reset that also removes
  `node_modules` and `target/`, and its own header documents `clean_build.sh`
  as the everyday cleaner wired to `npm run clean`. The plan's underlying
  defect (the script path did not exist) is fixed, and `clean:dist` exposes the
  deep reset under its own name.
- `package.json` carries version `26.8.0` rather than the `26.08` in `VERSION`.
  npm rejects `26.08` as invalid semver, so the CalVer value is expressed in
  the nearest valid form.
- Build timings reported by `build.sh` are information: they show where time
  went so a stage that grows becomes visible. Correctness lives in
  `./check_codebase.sh`, and the timings are most useful read as a trend across
  runs on one machine.
- `ts-rs` and `xml-rs` were both dropped in favor of our own implementations,
  at the owner's direction. Evidence that prompted the first: ts-rs output is
  not `prettier --check` clean, and M1 requires generated TypeScript to pass
  Prettier unchanged, which a third-party generator gives no way to control.
  The QTI parser is now ours to write, which raises the stakes of the M4
  hostile-corpus gate; that is the accepted cost.
- `uuid` refuses to compile for `wasm32-unknown-unknown` without an explicit
  randomness source. Rather than granting the browser an RNG, identifier
  generation moved behind a `question_model/generate` feature that the server
  enables and the WASM bridge does not. The browser never mints identifiers: a
  `ProblemId` is constructed on the publish transition. A compile error became
  a boundary.
- `build_github_pages.sh` was removed along with a stray `build_rust.sh`, and
  `./build.sh` replaces both. This repository ships a server platform, so the
  GitHub Pages framing was template inertia; `dist/` is a client bundle the API
  serves, not a static site. `deploy-pages.yml` was removed for the same
  reason.
- The `wasm-bindgen` glue is generated by a workspace crate (`crates/xtask`,
  depending on `wasm-bindgen-cli-support`) rather than an externally installed
  `wasm-bindgen` binary. The generator and the `wasm-bindgen` crate compiled
  into the module must be the same version; taking the generator from the
  workspace makes `Cargo.lock` pin both, so they cannot drift. The first
  attempt used a Homebrew CLI plus a version comparison in the build script,
  which was a check for a problem the structure can prevent outright.
- The generated Node flavor failed to load: it is CommonJS, and the repo-root
  `package.json` sets `"type": "module"`, so Node parsed the `exports.`
  assignments as ESM. `pipeline/build_wasm.sh` now writes
  `{"type":"commonjs"}` into `dist_wasm/node/`, which scopes the module system
  to that directory instead of renaming generated files after the fact.
- The container health check is declared in `containers/compose.yaml` as well
  as in the Containerfile: podman compose did not carry the image-level
  `HEALTHCHECK` onto the running container, leaving `State.Health` unset.
- Two Swift build scripts for an unrelated application, `build_debug.sh` and
  `build_release.sh`, are staged at the repository root. They reference
  `swift build` and an `AppCloser` binary and appear to have arrived by
  accident. Left in place pending the owner's decision.

### Developer Tests and Notes

- WP-C1 tests caught two defects that would have shipped a wrong wire format.
  First, serde's `rename_all` renames enum _variants_ and leaves the fields
  inside them alone, so `graceSeconds` was serializing as `grace_seconds` while
  sibling structs used camelCase; every tagged enum now also declares
  `rename_all_fields`. Second, the generator's serde-attribute reader stopped
  at the first entry it did not recognize, so a `rename_all` written after
  `tag` was invisible to it and the TypeScript disagreed with the JSON.
- A third defect came from a brittle assertion rather than the code: a test
  searched the serialized JSON for `mass` and matched inside `molar_mass`. The
  assertion now searches for the quoted key.
- Identifier types were originally declared inside a `macro_rules!` body, which
  made them invisible both to the TypeScript generator, which reads source, and
  to a reader skimming for the contract. The four structs are now written out,
  with `impl_identifier!` supplying the shared behavior.

- `./check_codebase.sh`: 8 checks pass, 1 SKIP (`test:node`, no
  `tests/test_*.mjs` present yet). The Rust steps are `cargo:fmt`,
  `cargo:clippy`, and `cargo:test`.
- `cargo test --workspace`: 3 unit tests and 3 doc tests pass, 0 failures.
- `pytest tests/`: 645 passed in 1.09s.
- `podman compose -f containers/compose.yaml --env-file containers/env.local up -d`
  brings up `api`, `postgres`, `minio`, and the one-shot `createbuckets`.
  `mc ls local` lists `content/`, `student-records/`, and `temp-processing/`.
- The WP-F4 health gate was verified in both directions, which is the part that
  matters: `/health` returns 200 `{"status":"ready"}` with both services up;
  503 `{"failing":["postgres"],...}` with PostgreSQL stopped; 503
  `{"failing":["postgres","object-store"],...}` with both stopped; and 200
  again after both restart.
- The built bundle was checked for Solid rather than React output: zero
  occurrences of `react` or `jsx-runtime` in `dist/main.js`, with Solid's
  `insert` calls present.
- Tested the esbuild CLI claim rather than repeating it. `npx esbuild
src/main.tsx --bundle --jsx=automatic` fails with three errors of the form
  `No matching export in "node_modules/solid-js/dist/solid.js" for import
"jsx"`. The CLI path is unusable, as the plan says, but the failure mode is a
  hard build error, not the silent React-flavored bundle described in the first
  draft of these comments. The build script, `pipeline/build.mjs`, and
  `docs/CODE_ARCHITECTURE.md` were corrected to state the measured behavior.
- Learned during WP-F2 that `wasm32-unknown-unknown` was not installed, and
  that listing it under `targets` in `rust-toolchain.toml` makes rustup fetch
  it automatically on the next cargo invocation; no manual `rustup target add`
  was needed.
- `./pipeline/build_wasm.sh` produces `dist_wasm/web/` (19 KB `.wasm` plus 5 KB
  of glue) and `dist_wasm/node/`.
- `bash tests/e2e/e2e_run_all.sh`: 1 passed, 0 failed. The bridge returns
  `0.1.0` to Node, read from the Rust side's `CARGO_PKG_VERSION`.
- `./build_github_pages.sh` produces `dist/` with `index.html`, `main.js`
  (11.5 KB), `main.js.map`, `style.css`, `wasm/`, and `.nojekyll`, and the
  script URL carries the bundle content hash (`main.js?v=352b012a`).
