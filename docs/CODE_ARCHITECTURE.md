# Code architecture

## Overview

The Peptidyle Learning Engine is a modular monolith: one browser application,
one stateless Rust server composition, and focused Rust crates that enforce
security and ownership boundaries at compile time. The authoritative plan is
[active_plans/implementation_plan.md](active_plans/implementation_plan.md);
this document is the working map. The path-by-path contributor map is
[FILE_STRUCTURE.md](FILE_STRUCTURE.md).

The current code includes the question model, domain rules, server-only
grading, object storage, learning data access, API routes, Solid browser client,
WebAssembly bridge, question-engine adapters, export workers, manual grading,
course item analysis, retention, a six-file PostgreSQL baseline, and the first
forward course-appearance migration. WP-RC3 source artifacts also implement a
private, upstream WeBWorK `render_rpc` adapter, optional private local compose
overlay, immutable PGML pilot seed, and opt-in E2E/browser gates. The bounded
local integration is accepted; it is not a production deployment claim. The
remaining dependency order is recorded in
[active_plans/active/release_completion_plan.md](active_plans/active/release_completion_plan.md),
and the dated current snapshot is
[active_plans/reports/project_status_report_2026-08-10.md](active_plans/reports/project_status_report_2026-08-10.md).

## Durable documentation map

The active plans say what remains to be built. [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) is the
conceptual entrypoint for the settled choices below. The plan, [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[CONTRACTS.md](CONTRACTS.md), schemas, and named code owner remain the source authorities.

| Teaching question | Decision and contract maps | Primary code owner |
| ----------------- | -------------------------- | ------------------ |
| What is PLE optimizing for? | [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md), [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md), and [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) | `crates/question_model/`, `crates/domain/`, and course/run routes |
| What travels between browser, PLE, and a grading backend? | [API_CONTRACTS.md](API_CONTRACTS.md), [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md), [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md), and [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md) | `crates/server/src/run/`, `crates/adapters/`, and `src/features/attempt/` |
| Which identity and data may cross each boundary? | [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md), [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md), [DATA_CONTRACTS.md](DATA_CONTRACTS.md), [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md), and [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) | auth, course, `crates/learning-data-access/`, and server DTOs |
| How do replicas survive overlap and failure? | [CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md), [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md), and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md) | PostgreSQL transactions, objects, jobs, and composition |
| What evidence supports a claim? | [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md), [DEVELOPMENT.md](DEVELOPMENT.md), and [E2E_TESTS.md](E2E_TESTS.md) | owning behavior test, live oracle, or human review |

The established reference documents remain the detailed implementation maps:
[QUESTION_MODEL.md](QUESTION_MODEL.md), [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md),
[SECURITY_MODEL.md](SECURITY_MODEL.md), [DATABASE_TENANCY.md](DATABASE_TENANCY.md),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md), [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md), and
[FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md).

Use the dated
[active_plans/reports/project_status_report_2026-08-10.md](active_plans/reports/project_status_report_2026-08-10.md)
for current acceptance state. A design document explains a boundary; it does
not by itself prove that a future work package has shipped.

## Contributor vocabulary

Plain capability names lead. Cargo package and crate-directory names use
hyphens; Rust import and module names use underscores.

| Plain name             | Physical path                                | Rust name              | What it owns                                                                 |
| ---------------------- | -------------------------------------------- | ---------------------- | ---------------------------------------------------------------------------- |
| Learning data access   | `crates/learning-data-access`                | `learning_data_access` | Persistence contracts, queries, PostgreSQL, migrations, and tenant isolation |
| In-memory data access  | `crates/learning-data-access/src/in_memory*` | `in_memory`            | Database-free implementation used by behavior and conformance tests          |
| PostgreSQL data access | `crates/learning-data-access/src/postgres*`  | `postgres`             | SQLx transactions, schema verification, RLS context, and production queries  |
| Project tools          | `crates/project-tools`                       | Cargo-only binary      | Repository-only generation, fixtures, database operations, and E2E seeding   |

Use `cargo tools` for project-tool commands. The opaque `cargo xtask` alias is
retired.

## Current and target topology

The maintained local path is the browser bundle in `dist/`, the same-origin
gateway, the `axum` API and worker, PostgreSQL, and MinIO, composed by
[launch_local_stack.sh](../launch_local_stack.sh) and
[containers/compose.yaml](../containers/compose.yaml). The optional private
WeBWorK overlay is the accepted bounded WP-RC3 local integration. It is not a
production topology or a claim of broad upstream-problem compatibility.

The following is the planned production topology. It is owned by WP-RC10 and
WP-RC11; it is not a deployed configuration or release claim.

```text
browser                     ALB          stateless replicas
+------------------------+           +----------------------------+
| Solid SPA (src/)       |  +-----+  | api x N (crates/server)    |
|  student/ instructor/  |->| ALB |->|   axum, native Rust        |
|  +------------------+  |  +-----+  |  +----------------------+  |
|  | domain.wasm      |  |           |  | domain + grading     |  |
|  | params, format   |  |           |  | authoritative        |  |
|  | validate, timer  |  |           |  +----------------------+  |
|  | NO answers       |  |           +----------------------------+
|  +------------------+  |             |            |         |
+------------------------+             v            v         v
        ^                       PostgreSQL      jobs queue   S3
        | public assets         one cluster:        |      3 buckets
   CloudFront (immutable)       shared content      v
        |                       + tenant rows   worker x N
   /api/assets (authorized)     + forced RLS    export, import,
                                                renderer fill
                                                     |
                                                     v
                                              renderer x N
                                              WeBWorK PG, private
```

## Two guarantees the structure enforces

Both are enforced by the shape of the code, not by review discipline.

- **Grading is server-only.** `crates/grading` holds answer keys and
  correctness decisions. It is not in the dependency closure of `crates/wasm`,
  so shipping an answer key to the browser is a compile-time impossibility
  rather than something a reviewer has to notice. Adding `grading` to
  `crates/wasm/Cargo.toml` is the single change that would break this.
- **Records are tenant-owned, content is shared.** Published problems and their
  versions carry no tenant ID and are immutable. Courses, assignments,
  enrollments, runs, attempts, and grades carry a tenant ID on every row under
  `FORCE ROW LEVEL SECURITY`, with the tenant context set from the
  authenticated session and never from a request parameter. Authentication
  uses one host-only opaque cookie rather than browser `localStorage`; the
  cookie is an essential service credential, not an analytics identifier. It
  is a browser-session cookie, while bounded expiration remains authoritative
  in shared backend storage.

## Crate boundaries

Each crate's dependency list is exhaustive: the allowed set is the whole set,
so a `Cargo.toml` matching this table satisfies the boundary by construction.

| Crate                         | Owns                                                                                                      | Depends only on                                     |
| ----------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| `crates/question_model`       | Question types, capabilities, identity, taxonomy                                                          | external crates                                     |
| `crates/domain`               | Attempt state machine, run and completion rules, timing verdict, seeded generation, capability validation | `question_model`                                    |
| `crates/grading`              | Answer keys, checkers, correctness decisions                                                              | `question_model`, `domain`                          |
| `crates/objects`              | `ObjectStore` trait, S3 and MinIO backends, key construction, checksums                                   | `question_model`                                    |
| `crates/learning-data-access` | Learning data-access contracts, PostgreSQL, migrations, RLS                                               | `question_model`, `domain`, `objects`               |
| `crates/adapters/native`      | First-party generated questions and static flat-question compilation                                      | `question_model`, `domain`, `grading`               |
| `crates/adapters/webwork`     | Private renderer projection and server-side grade delegation                                              | `question_model`, `domain`, `grading`, `objects`    |
| `crates/adapters/qti`         | Bounded package import, profile mapping, and private grade handoff                                        | `question_model`, `domain`, `grading`, `objects`    |
| `crates/adapters/h5p`         | Ungraded practice import and capability declaration                                                       | `question_model`                                    |
| `crates/adapters/imathas`     | Contracted/self-hosted scored embed boundary                                                              | `question_model`, `objects`, `learning-data-access` |
| `crates/export`               | Print model, DOCX and PDF writers                                                                         | `question_model`, `objects`                         |
| `crates/wasm`                 | `wasm-bindgen` bridge delegating to `domain`                                                              | `question_model`, `domain`                          |
| `crates/server`               | axum routes, auth, worker mode, composition root                                                          | every crate above                                   |

Two properties follow from that table. `crates/domain` reaches only
`question_model`, so it has no clock and no database: `chrono` is declared with
`default-features = false`, which drops the `clock` feature, and time arrives
as a parameter. That is what lets the same code run in a browser and makes the
seed-parity test meaningful. `crates/wasm` reaches only `question_model` and
`domain`, which is the grading guarantee above.

Two drivers are owned rather than shared. `sqlx` is declared only in the
learning data-access crate (`crates/learning-data-access`) and `aws-sdk-s3` only in
`crates/objects`; `crates/server`
enables them through features and names neither, so the database and object
clients stay replaceable behind their traits.

## Capability ownership inside crates

Crates enforce security and deployment boundaries. Focused modules inside each
crate give a contributor a smaller unit to understand and change.

WP-ARCH1 applies that rule across the repository while preserving stable public
facades:

- `crates/learning-data-access/src/contracts/`, `in_memory/`, and `postgres/`
  separate public Store contracts from backend capability implementations;
  their conformance and live tests use the same behavior-based division. The
  paired `in_memory/runs/attempt_issuance.rs` and `postgres/runs/attempt_issuance.rs`
  owners keep presentation binding, prefetch promotion, timing creation, and
  private WeBWorK replay persistence together without turning the broader run
  lifecycle modules into warehouses.
- `crates/server/src/run/` owns prefetch, queries, and submission, while the
  catalog, course, workspace, retention, composition, iMathAS, and publication
  parents remain small route or composition facades over focused behavior and
  test modules.
- adapter child modules own iMathAS caching, WeBWorK HTML projection, and
  adapter tests; `crates/project-tools/src/e2e_seed/` owns native, WebWork,
  timing, scoring, and record seeding separately.
- `src/api/decoders/`, `src/api/http_client/`, and `src/api/mock/handlers/`
  own browser decoding, transport, and mock behavior. `src/components/responses/`
  owns response-family controllers; `src/components/response_widget/` owns
  keyboard and external-tool extensions behind the previous import paths.
- `devel/bump_version.py` remains the command facade over the
  `devel/bump_version/` package.

The permanent tracked-source boundary is `tests/test_source_file_line_limit.py`.
Maintained code reaches at most 999 physical lines, and the exact override list
contains only frozen migrations and documentation or history artifacts. Module
names, symbol lists, and today's exact file layout are implementation evidence,
not permanent test contracts.

One learning data-access capability normally has four cooperating owners:

1. A backend-neutral contract under `crates/learning-data-access/src/`.
2. An in-memory implementation under `crates/learning-data-access/src/in_memory/`.
3. A PostgreSQL implementation under `crates/learning-data-access/src/postgres/`.
4. Behavior or conformance coverage under `crates/learning-data-access/tests/conformance/`.

For example, external-tool work is divided among
`crates/learning-data-access/src/external_tool.rs`,
[crates/learning-data-access/src/in_memory/external_tool.rs](../crates/learning-data-access/src/in_memory/external_tool.rs),
[crates/learning-data-access/src/postgres/external_tool.rs](../crates/learning-data-access/src/postgres/external_tool.rs), and
[crates/learning-data-access/tests/conformance/external_tool.rs](../crates/learning-data-access/tests/conformance/external_tool.rs).
The crate root declares and re-exports that contract; it does not own the
external-tool behavior.

Protected asset delivery follows the same four-file shape:
`crates/learning-data-access/src/asset_delivery.rs`,
`crates/learning-data-access/src/in_memory/assets.rs`,
`crates/learning-data-access/src/postgres/assets.rs`, and
[crates/learning-data-access/tests/conformance/assets.rs](../crates/learning-data-access/tests/conformance/assets.rs).
That component owns immutable logical-to-physical bindings, public-catalog
resolution, protected educational-record authorization, and access auditing.

Course appearance follows the same backend-neutral ownership shape:
`crates/learning-data-access/src/course_appearance.rs`,
`crates/learning-data-access/src/in_memory/course_appearance.rs`,
`crates/learning-data-access/src/postgres/course_appearance.rs`, and
`crates/learning-data-access/tests/conformance/course_appearance.rs`.
It owns default creation, revision compare-and-swap, persisted session authority, bytes-first banner
promotion, exact-current delivery, and bounded two-phase cleanup. The PostgreSQL implementation uses
a scoped security-definer actor resolver rather than granting `ple_app` direct access to
`auth_session`; a second trigger rejects any current pointer whose delivery kind, tenant, or course
does not match the appearance row. The disposable live oracle is
`crates/learning-data-access/tests/postgres_course_appearance_live.rs`.

The HTTP and image boundary remains in `server_core`:
`crates/server/src/course_appearance.rs` owns authenticated GET/PUT/candidate orchestration,
`crates/server/src/course_appearance/image.rs` owns bounded orientation and one 1200 by 328 WebP
normalization, and `crates/server/src/course_appearance/tests.rs` owns route recovery and refusal
behavior plus the combined PostgreSQL/MinIO cleanup oracle. Successful appearance traffic also runs
one bounded best-effort tenant claim/delete/complete sweep; missing objects are idempotent, object
failure never blocks a course read, and completion follows only successful exact-key deletion. The
route never discloses the future banner identity held by persistence. It verifies and
copies candidate bytes first, then asks the Store to apply one revision CAS. Existing
`crates/server/src/asset.rs` remains the single same-origin delivery route and delegates course
banners to exact-current-pointer authorization before object signing.

The browser owner is `src/features/course_appearance/`. `theme_catalog.ts` is the exhaustive
15-theme and derived-token registry; `course_theme_route.ts` classifies only course-owned paths;
`course_theme_scope.tsx` loads the safe projection before rendering and owns its CSS-variable
subtree; and `course_theme_context.ts` lets attempt, summary, and instructor pages reuse route data
without importing router mechanics. `course_appearance_model.ts` owns the pure editable draft and
atomic update, `course_appearance_repository.ts` owns the narrow client seam, and
`course_appearance_page.tsx` owns the native-control form and preserved-error recovery.
`course_entry_identity.tsx` renders the text title and optional current banner only from the existing
course-route projection. `crates/server/src/run/queries.rs` supplies the summary's authorized
course/appearance projection from the stored assignment and session, never from a browser-supplied
course ID. Global navigation, non-entry routes, and semantic status colors remain outside the banner
projection. The integrated PostgreSQL/MinIO owner is `tests/e2e/e2e_course_appearance.sh`; the
built-browser and generated visual owners are `tests/playwright/course_appearance*.ts` and
`tests/playwright/course_theme_scope.spec.ts`.

Authentication sessions are similarly isolated in
[crates/learning-data-access/src/session.rs](../crates/learning-data-access/src/session.rs),
`crates/learning-data-access/src/in_memory/sessions.rs`,
`crates/learning-data-access/src/postgres/sessions.rs`, and
`crates/learning-data-access/tests/conformance/sessions.rs`. The component owns opaque token
hashes, the backend-authoritative clock, revocation, and replica-safe lookup;
it does not own HTTP cookies or educational records.

Static flat-question authoring is owned by
[crates/adapters/native/src/flat_question.rs](../crates/adapters/native/src/flat_question.rs).
The facade preserves the exact version 1 single-choice contract and delegates
the closed all-family version 2 shapes to `flat_question/v2.rs`. Together they
parse MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT source, compile the
browser-safe draft, bind separate grader-only answers and feedback to that
exact public content, and evaluate responses through the shared grading crate.
The version 2 semantics follow QTI Package Maker's reviewed item model, with a
bounded PLE hotspot extension. The native adapter facade exposes the
capability; the question-model crate remains answer-free and unaware of the
authoring syntax. Publication persistence and the instructor editor are
separate vertical packages, so the codec does not absorb data-access or HTTP
concerns.

Learner controls for the response families live in
`src/components/responses/`. Multi-blank uses labeled text fields, matching
uses native radio groups, ordering uses a keyboard-operable list, and hotspot
uses candidate-region controls as its primary no-mouse path. A future pointer
overlay may enhance hotspot interaction without replacing the accessible list
or moving correctness into the browser.

QTI import and its answer-bearing grader boundary are owned by
[crates/learning-data-access/src/qti.rs](../crates/learning-data-access/src/qti.rs),
[crates/learning-data-access/src/in_memory/qti.rs](../crates/learning-data-access/src/in_memory/qti.rs),
[crates/learning-data-access/src/postgres/qti.rs](../crates/learning-data-access/src/postgres/qti.rs), and
[crates/learning-data-access/tests/conformance/qti.rs](../crates/learning-data-access/tests/conformance/qti.rs).
The ordinary data-access handle can write validated private staging metadata,
but only the separately injected grader handle can read opaque grading
material. The QTI adapter keeps its public facade in
[crates/adapters/qti/src/lib.rs](../crates/adapters/qti/src/lib.rs), its bounded archive/parser
implementation in
`crates/adapters/qti/src/parser.rs`, and its reviewed
profile mapping under
[crates/adapters/qti/src/profiles/](../crates/adapters/qti/src/profiles/). Unsafe archives fail as a
whole; safe archives retain accepted items and record every semantic rejection.
Normalized checksums report exact and likely duplicates within the batch
without exposing the grading binding.

The H5P adapter exposes its key-free practice importer through
`crates/adapters/h5p/src/import.rs`.
The WeBWorK adapter exposes its server-only renderer request, response,
identity, failure, render, and grade boundary through
`crates/adapters/webwork/src/renderer_contract.rs`. Its
`http_renderer/client.rs` joins only the configured application base and
`render_rpc`; `protocol.rs`, `response_shape.rs`, `html_projection.rs`, and
`grade.rs` own its focused protocol, validation, projection, and grade work.
The client rejects redirects and unbounded/wrong-type responses, and projects the approved single-radio PG
shape. It uses `html5ever` tokenization to extract the exact radio group,
then discards upstream field names, hidden fields, session material, and source
bytes before forming a PLE question envelope. `shipped_render_rpc.rs` fixes the
upstream path and form media type. The adapter cache stores only the safe
rendered projection and emits `ple.webwork.cache` `renderer_call` and
`cache_hit` events. `crates/server/src/webwork_backend.rs` resolves immutable
catalog source bytes under the attempt tenant before calling that adapter.
The optional private upstream deployment is owned by the WP-RC3 workstream;
it is not a browser-facing service.

The profile-to-native author path keeps each ownership transition explicit:

1. [crates/server/src/qti_profile_import.rs](../crates/server/src/qti_profile_import.rs) authorizes
   the workspace before consuming the opaque archive, persists it in protected storage, queues the
   worker, and returns only no-store safe projections.
2. [crates/server/src/qti_import.rs](../crates/server/src/qti_import.rs) runs the Canvas/Blackboard
   profile parser and commits the mixed accepted/rejected report with exact archive and result
   digests.
3. [crates/server/src/qti_profile_conversion.rs](../crates/server/src/qti_profile_conversion.rs)
   re-reads the immutable archive and repeats the full report binding. It sends one acknowledged
   accepted item through
   [crates/server/src/qti_profile_flat_bridge.rs](../crates/server/src/qti_profile_flat_bridge.rs)
   to the native flat compiler, then one Store command advances the draft revision while committing
   canonical source, opaque grading material, provenance, and current origin atomically.
4. [src/features/qti_profile_import/](../src/features/qti_profile_import/) owns upload, bounded
   polling, answer-free review, acknowledgement, conversion, conflict recovery, and editor handoff.

The browser never receives archive bytes, object keys, private choice maps, correct-choice material,
or unreleased feedback. The production grading path can resolve the opaque payload only through the
separately injected PostgreSQL grader handle. The live oracle in
[crates/server/src/qti_profile_postgres_live.rs](../crates/server/src/qti_profile_postgres_live.rs),
invoked by [tests/e2e/e2e_database_baseline.sh](../tests/e2e/e2e_database_baseline.sh), proves the
full upload/worker/convert/edit/publish/correct-and-incorrect-grade path against PostgreSQL 17 with
real application, student, grader, and foreign-tenant role denials, immutable checksum-bound
provenance, retention, and exact disposable cleanup.

The QTI publication HTTP capability is owned by
[crates/server/src/qti_publication.rs](../crates/server/src/qti_publication.rs),
with its private route-contract tests in
`crates/server/src/qti_publication/tests.rs`.
The production owner keeps committed-staging validation, strong workspace
revision checks, exact source-byte copying, review authorization, and the one
visible-version promotion boundary together; its test owner exercises those
boundaries without enlarging the route module.

The QTI runtime backend is owned by
[crates/server/src/qti_backend.rs](../crates/server/src/qti_backend.rs).
Its shared private fixtures live in `qti_backend/tests/mod.rs`, direct
private-grading and asset-integrity cases in
`qti_backend/tests/private_grading.rs`, and the learner HTTP/replay proof in
`qti_backend/tests/run_lifecycle.rs`. This keeps answer-bearing lookups,
tenant/provenance refusal, asset binding, and idempotent run behavior visible
as separate review surfaces without duplicating setup.

The same rule applies to server routes and browser features: group code by the
capability a contributor changes, preserve stable facade paths, and keep each
source file below 1000 lines. Split by ownership rather than arbitrary line
ranges.

## Database storage

The table-by-table revision, authentication, assignment, FERPA isolation, pilot-volume, and
ten-million-question growth map lives in [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md). It marks
implemented relations separately from the proposed production identity and passkey tables, so
future authentication planning does not get confused with accepted schema state.

The first six ordered SQLx migrations under `schemas/migrations/` are the accepted pre-data baseline
for principals, catalog/authoring, courses/assignments, activity/feedback, operations/analytics, and
retention. The forward `schemas/migrations/2026080907_course_appearance.sql` migration adds
revisioned appearance, banner candidates, exact-current delivery policy, and forced RLS without
rewriting that baseline. `cargo tools database status`, `migrate`, and `verify` expose the exact
ledger/checksum boundary; application startup is
verify-only and never mutates a deployed schema. The baseline creates shared immutable
catalog tables separately from tenant-owned educational records. Compact
problem-version metadata remains unpartitioned for browsing, while JSONB
payloads use 16 hash partitions. Question attempts, submissions, protected-record access logs,
and audit events use monthly range partitions plus a default partition so an
unexpected timestamp cannot make an otherwise valid write disappear.

Every tenant-owned table enables and forces row-level security. A data-access
transaction first assumes the non-superuser, non-bypass `ple_app` role and then
sets `ple.tenant_id` with transaction-local scope. Returning the pooled
connection clears both settings. The separate `ple_student` role can read only
the browser-safe tables and has no grant on `answer_key`; `ple_grader` owns that
answer-bearing access. Before the tenant is known, the narrowly privileged
`ple_auth` role can access only `auth_session`, and forced RLS restricts that
access to the SHA-256 hash of the single presented opaque credential. The raw
credential is never stored in PostgreSQL.

Schema migration and application access are distinct privileges. The
connection used by `learning_data_access::postgres::apply_migrations` must be able to create
roles and schema objects. A production runtime login needs membership that
permits `SET ROLE ple_app` and `SET ROLE ple_auth`, but must not itself be a
superuser or have `BYPASSRLS`. The local development superuser may perform both
jobs; that convenience is not the production role model.

The PostgreSQL data-access implementation (`PostgresStore`) serializes complete
contract records as checksummed JSONB while
retaining normalized identity, relationship, timestamp, and pagination columns
for constraints and indexed queries. Activity rows and their compact summary
projection commit in one transaction. The backend-neutral data-access traits
keep
SQLx and schema details out of callers so the persistence implementation can
evolve without changing domain or API contracts.

`asset_delivery` is the immutable registry between database metadata and
object bytes. A public catalog entry uses its logical `AssetId` as the route
identifier; a student-record artifact uses its `ObjectId`. The row contains
the exact checksummed `ObjectRecord` and typed key, so `/api/assets/{id}` never
constructs a key from request text or discovers content with a bucket listing.
Forced RLS protects student-record rows and institution catalog visibility.

Catalog browsing reads only normalized hot metadata from `problem_version`;
exact version resolution joins the separately hash-partitioned immutable
payload. Institution publications receive an exact tenant/version grant under
forced RLS, while public versions are visible without a tenant-specific grant.
Context-free data-access catalog reads are explicitly public-only;
tenant-visible reads go through `CatalogStore` with a session-derived
`TenantContext`.

Course and assignment browse columns are normalized as well: `course` carries
its title, `course_member` maps authenticated `UserId` values to course-local
student or instructor authority, and `assignment` carries `course_id` plus its
title. Assignment payloads retain the ordered policy and exact-version
contract, while `(tenant_id, course_id, assignment_id)` supports cursor paging
without loading or filtering JSON.

The application database role may insert published content but cannot update
or delete its identity, payload, scope, backend, capabilities, metadata,
authorship, or lineage. Its only catalog update privilege covers `lifecycle`
and `lifecycle_reason`. The partial unique index on
`(problem_id, previous_version_id)` makes each version chain linear even under
concurrent publication.

## Authentication boundary

`crates/server/src/auth.rs` separates credential verification from session
mechanics. An `IdentityProvider` establishes a validated account presentation;
the same route and store contracts can therefore accept the production
passwordless provider, optional institutional SSO, LTI, or the explicit
local-development provider without changing cookie handling. WP-RC8 now implements
PLE-managed email/passkey accounts and invite-by-email enrollment with acceptance open. The provider
abstraction and opaque session remain stable; tenant context is derived from an
authorized course/tenant relationship rather than email or browser authority.

Successful login generates 256 bits from the operating-system random source.
Only its SHA-256 hash enters `SessionStore`; the raw value appears solely in an
HttpOnly cookie. The normal HTTPS policy is `Secure; SameSite=Lax`; embedded
LTI HTTPS can explicitly select `SameSite=None`, and insecure cookies require
the explicit local-HTTP transport setting. The cookie has no `Max-Age` or
`Expires` attribute; PostgreSQL still decides bounded creation, expiration,
and revocation time, so a request may log in on one replica and continue or
sign out on another.

The `/api/auth/login`, `/api/auth/session`, and `/api/auth/logout` router is
provider-injected. Session resolution constructs `TenantContext` only after an
active database row is found. Missing, malformed, unknown, expired, and revoked
credentials share the same unauthenticated response, and dependency details
are not returned to the browser.

## Catalog boundary

`crates/server/src/catalog/routes.rs` authenticates every catalog request and accepts
no tenant identifier from a URL, query, or body. Browse and taxonomy routes use
bounded stable-key cursors and return hot `CatalogProblemSummary` values rather
than prompt, response, or source-locator payloads. Exact lookup loads one
visible immutable `QuestionDefinition`, including deprecated and archived
versions needed by historical references.

Publication resolves adapter capabilities through the server-owned
`BackendRegistry`, runs the same complete `validate_assignment_config` result
used by the editor, and passes the exact validated draft into one store
transaction. The store locks and compares that draft, inserts metadata and
payload, installs an institution grant when needed, and deletes the draft
atomically. New works and forks receive a fresh server-generated `ProblemId`;
owned revisions retain their existing problem and form one linear chain.

Institution publication accepts instructor, publisher, or administrator
roles. Public publication accepts publisher or administrator roles and also
passes the institution-configurable `PublicReviewGate`, whose default is off.
Only an author can deprecate or archive a version. Both states disappear from
browse while exact references remain resolvable; archival additionally blocks
new assignment references.

## Course boundary

`crates/server/src/course/routing.rs` authenticates every course request and uses the
session tenant rather than accepting one from the URL or body. Coarse login
roles only control course creation and tenant-administrator access. Ordinary
course visibility and assignment management come from explicit
`course_member` rows, so being an instructor in one course grants no access to
another.

`GET /api/courses` is cursor-paged and returns the signed-in user's effective
course role. Assignment lists require access to the parent course and are
structurally scoped by `CourseId`. `POST /api/courses/{course}/assignments`
accepts an ordered list of `ProblemVersionRef` values; the store verifies each
exact catalog version is visible and assignable, then stores those IDs rather
than copying question content. Tenant administrators receive derived global
authority in the response, but that authority cannot be persisted as a course
membership.

## Run and grading boundary

`crates/server/src/run/routes.rs` resolves the authenticated enrollment owner before
starting, issuing, or submitting work. Students can read only their own run;
the course instructor and tenant administrator may inspect enrollment history
and summaries. A nonowner receives the same not-found response as an absent
record.

The injected `RunBackend` is the server-only adapter boundary for question
generation and grading. A fresh operating-system-random seed in JavaScript's
exact 53-bit integer range is proposed for each new question instance, while
`Store::issue_or_resume_question_attempt` returns an existing unresolved
instance unchanged. The Rust generator still accepts the full `u64` domain.
The store locks the run and permits only one unresolved question at a time,
preventing concurrent requests from starting multiple question timers.

The route repeats browser-safe response-format validation before invoking the
backend. PostgreSQL then supplies the submission timestamp, applies the timing
verdict, persists submission and grade events, derives completion, and updates
the run, enrollment, and compact summary in one transaction. A compact
hash-partitioned idempotency table owns the immutable first result so an exact
browser retry does not grade or count twice.

Responses contain browser-safe question-attempt projections only. Feedback
policy may hide correctness and points, and no response contains an answer key,
expected value, private rubric, or checker implementation.

The current route still returns a broader learner attempt projection and uses
a tagged `StudentResponse`. Before WP-RC5, the accepted
[secure grading payload plan](active_plans/decisions/secure_question_grading_payload_plan.md)
replaces that wire atomically with an attempt-bound minimal descriptor,
CRC16-derived presentation-scoped rendered-item IDs, a SHA-256 presentation
consistency digest, server-selected response decoding, and server-only partial
credit. The CRC and digest are consistency checks; authenticated session,
forced RLS, attempt state, and idempotency remain the security authority.
The durable `docs/ASSESSMENT_PAYLOAD_DESIGN.md` explains the current and target
render, response, result, caching, prefetch, ADAPT comparison, and private
WeBWorK boundaries.

## Browser-safe validation fallback

`crates/server/src/validation.rs` authenticates and bounds three HTTP fallbacks
for response-format, timer, and assignment-capability evaluation. Each route
delegates directly to the same key-free `domain` function exported through
WebAssembly and returns `no-store` JSON. The results support a degraded browser
mode only: trusted publication still resolves stored definitions and backend
capabilities, authoritative timing still uses database timestamps, and grading
remains server-only.

## Object storage

| Bucket            | Holds                                          | Serving                                                                                 | Retention                        |
| ----------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------- |
| `content`         | source packages, shared assets, cached renders | CDN and immutable URLs for public content, 60-minute authorized URLs for secure content | indefinite, versioned            |
| `student-records` | exports, uploaded responses, annotated exams   | 5-minute authorized URLs, always logged                                                 | explicit expiration and deletion |
| `temp-processing` | extraction and conversion workspaces           | never served                                                                            | lifecycle rule, days             |

Keys are built only from identity types and versions, never from a
caller-supplied string. Checksums are computed on write and verified on read.
The real `S3ObjectStore` implementation is shared by MinIO and AWS. It uses a
conditional `If-None-Match: *` write so immutable keys cannot be overwritten,
and stores an encoded copy of the `ObjectRecord` in object metadata so any API
replica can reconstruct and verify the record without process-local state.
Reads reject a mismatched semantic key, bucket, category, version, media type,
size, or SHA-256 digest before returning bytes. Signed URLs use the
server-supplied timestamp as their signing start time.

`crates/server/src/asset.rs` first asks the registry for a globally public
catalog asset. Those requests need no session and redirect to the configured
CDN path with immutable caching; the object backend does not sign them.
Everything else resolves the HttpOnly session, authorizes in its RLS tenant,
and appends an audit event containing no credential or URL before requesting
the exact stored key. The server independently caps `content` URLs at 60
minutes and `student-records` URLs at 5 minutes, marks protected redirects
`no-store`, and refuses `temp-processing` delivery.

## Containers

The root [launch_local_stack.sh](../launch_local_stack.sh) is the maintained local-test front door.
Its default local path creates ignored high-entropy credentials, selects an available gateway port,
builds the host browser bundle, starts backing services, applies and verifies migrations,
provisions the distinct grader login, seeds a bounded demonstration course, and then starts API,
worker, and gateway. The gateway mounts `dist/` read-only, serves browser navigation, and proxies
only `/api`, `/api/*`, and `/health` to the API replicas; API-shaped paths can never fall through to
the single-page application. Normal shutdown preserves the named PostgreSQL and MinIO volumes. Passing
`--with-webwork` adds `containers/compose.webwork.yaml`,
which gives only the API a renderer endpoint and a read-only runtime password
file. The overlay starts source-pinned upstream WeBWorK and a dedicated MariaDB
on private networks; neither service publishes a host port or joins PLE
PostgreSQL, MinIO, gateway, or worker networks. The image, course initializer,
and semantic probe are under `containers/webwork/`.
This source-level integration still requires its explicit live acceptance gate.
Replica discovery, state ownership, worker scaling, network boundaries, and the
separate planned production topology are documented in
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md).

| Service         | Image                                     | Purpose                                 |
| --------------- | ----------------------------------------- | --------------------------------------- |
| `api`           | built from `containers/Containerfile.api` | axum API server                         |
| `postgres`      | digest-pinned official PostgreSQL 17      | shared content and tenant-owned records |
| `minio`         | digest-pinned official MinIO              | S3-compatible object storage            |
| `createbuckets` | digest-pinned official MinIO Client       | one-shot bucket creation                |
| `worker`        | built from `containers/Containerfile.api` | durable background work                 |
| `gateway`       | pinned official Caddy derivative          | browser and same-origin API             |

Details and commands are in [CONTAINER.md](CONTAINER.md); the macOS virtual
machine setup is in [MACOS_PODMAN.md](MACOS_PODMAN.md).

`/health` returns 200 only after exact PostgreSQL schema compatibility verification and a real `HeadBucket`
request, and 503 naming the failing dependency otherwise. A health check that
reports on process liveness rather than dependency reachability tells an
orchestrator nothing.

## Browser client

`src/` is a TypeScript and SolidJS single-page application.
[pipeline/build.mjs](../pipeline/build.mjs) builds it with the esbuild
JavaScript API plus `esbuild-plugin-solid`, because Solid compiles JSX to
direct DOM operations through a Babel preset that the esbuild CLI cannot load.

That was measured rather than assumed. Running the CLI against this source with
`--jsx=automatic` fails with three errors of the form
`No matching export in "node_modules/solid-js/dist/solid.js" for import "jsx"`,
so the wrong path fails loudly at build time rather than shipping a bundle that
breaks in the browser.

The client shares generation, format validation, and timer logic with the
server through the WebAssembly bridge that
[pipeline/build_wasm.sh](../pipeline/build_wasm.sh) produces, so the two cannot
drift apart. Build output lands in `dist/`, with the bridge under `dist/wasm/`.

## Testing and verification

`./check_codebase.sh` runs both toolchains: TypeScript typecheck, wider
typecheck, ESLint at zero warnings, Prettier, Node unit tests, then
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo test --workspace`. A missing `cargo` emits a loud SKIP rather than
passing silently.

`pytest tests/` runs the repository hygiene suite. Browser tests live in
`tests/playwright/`, and slower whole-system checks in `tests/e2e/`, both
outside the fast lane.

The opt-in WP-RC3 checks are `tests/test_webwork_renderer_container.py`,
`tests/e2e/e2e_webwork_render_rpc.sh`, and
`tests/playwright/webwork_run.spec.ts`.
They are intended to verify the private compose topology, one immutable PGML
source through PLE, render-cache evidence, correct/incorrect scoring, and the
browser's same-origin boundary. They do not run as part of the default fast
gate and are not recorded here as completed acceptance.

The in-memory and PostgreSQL data-access implementations share capability
conformance tests. Disposable PostgreSQL acceptance runs prove migration
checksums, real-role tenant isolation, serialization retry, family-filtered
concurrent worker claims, monthly partition pruning over 260,000 synthetic
attempts, bounded current-summary gradebook plans, manual grading, and
generation-fenced course item analysis. A separate one-time production-worker
rehearsal proved physical student-record deletion while preserving shared
catalog content, instructor drafts, course structure, and anonymous statistics.
A second one-time rehearsal restored encrypted role and database artifacts into
a separate empty PostgreSQL 17 cluster, matched the logical source fingerprint,
and re-exercised owners, grants, forced RLS, tenant access, application writes,
and broker calls. None of these gates touch the developer's long-lived database
container.

The maintained whole-system runner then composes those boundaries: it calls the
browser-safe Wasm bridge, runs the complete disposable database suite, and
starts PostgreSQL, MinIO, a non-root zero-capability gateway, and two stateless
API replicas. Its learner fixture logs in, starts a run, stops the replica that
issued the question, reproduces the exact envelope on the survivor, submits the
visible response twice with one idempotency key, and verifies the durable row
set. Every generated project and volume is removed afterward.

## Extension points

- Add question-engine behavior behind the adapter contracts in
  [crates/adapters/](../crates/adapters/); answer-bearing grading remains
  outside the WebAssembly dependency closure.
- Extend the shipped WeBWorK projection in `http_renderer/html_projection.rs`
  and `http_renderer/tests.rs` only after defining a safe PLE response shape;
  RC3 accepts one
  single-choice radio group, while matching has a separately owned release
  package.
- Add a learning data-access capability as a focused contract plus in-memory,
  PostgreSQL, and conformance modules. Keep
  [crates/learning-data-access/src/lib.rs](../crates/learning-data-access/src/lib.rs) as a facade.
- Add HTTP behavior to the owning route module under
  [crates/server/src/](../crates/server/src/), not to the composition root.
- Add browser behavior to an owning feature, page, component, or API module
  under [src/](../src/); use the generated client contract rather than a new
  request shape.
- Add repository automation to the project tools under
  [crates/project-tools/src/](../crates/project-tools/src/) and expose it through
  `cargo tools`.

## Known gaps

- WP-ARCH1 is accepted: its dated 26-file baseline has zero maintained-code
  size violations, and independent PostgreSQL, security, provider,
  TypeScript/HCI, test, size-policy, and final architecture reviews found no
  unresolved P0/P1 issue. WP-RC4's eight-family PLE flat JSON v2 implementation
  is present and awaits independent closeout; the secure payload cutover and
  WP-RC5 authoring/integrated-content work remain next dependencies.
- Deployed managed point-in-time recovery and object-store recovery drills are
  not complete. Local clean-cluster logical restore, the production-worker
  tenant-purge rehearsal, and the local whole-system replica gate have passed.
  Reserved Render and generic Import queue variants
  also remain intentionally unclaimed until each has a complete handler and
  atomic committer. See
  [active_plans/active/release_completion_plan.md](active_plans/active/release_completion_plan.md).
- WP-RC3's exact pinned upstream image, private semantic E2E, and PLE-owned
  keyboard browser path passed locally on Podman 6 and are accepted for the
  single immutable RadioButtons fixture. Broad OPL compatibility and MATCH
  remain separately owned by WP-RC5.
