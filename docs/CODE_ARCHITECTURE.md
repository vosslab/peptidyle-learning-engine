# Code architecture

How the Peptidyle Learning Engine is put together and why the boundaries sit
where they do. The authoritative plan is
[active_plans/implementation_plan.md](active_plans/implementation_plan.md);
this document is the working map.

Current state: M2 core lanes in progress. The M1 contracts are frozen, the
shared domain and grading boundaries compile natively and for WebAssembly, and
the real object and PostgreSQL backends implement the same contracts as their
memory test doubles. Authentication has a provider-injected route group, and
the catalog route group implements scoped publication, browsing, taxonomy, and
one-way lifecycle changes. The course route group now implements course-local
membership, course-scoped assignment creation and browsing, and exact immutable
problem/version references. The run route group implements owned run issue,
attempt history, idempotent submissions, server-side grading, and summary
reads. Asset routes and the first owner-selected native question family remain
later M2 work.

## Shape of the system

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

| Crate                                      | Owns                                                                                                      | Depends only on                                  |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `crates/question_model`                    | Question types, capabilities, identity, taxonomy                                                          | external crates                                  |
| `crates/domain`                            | Attempt state machine, run and completion rules, timing verdict, seeded generation, capability validation | `question_model`                                 |
| `crates/grading`                           | Answer keys, checkers, correctness decisions                                                              | `question_model`, `domain`                       |
| `crates/objects`                           | `ObjectStore` trait, S3 and MinIO backends, key construction, checksums                                   | `question_model`                                 |
| `crates/store`                             | `Store` trait, PostgreSQL backend, migrations, RLS context                                                | `question_model`, `domain`, `objects`            |
| `crates/adapters/{native,webwork,qti,h5p}` | Per-engine load, generate, grade delegation, capability declaration                                       | `question_model`, `domain`, `grading`, `objects` |
| `crates/export`                            | Print model, DOCX and PDF writers                                                                         | `question_model`, `objects`                      |
| `crates/wasm`                              | `wasm-bindgen` bridge delegating to `domain`                                                              | `question_model`, `domain`                       |
| `crates/server`                            | axum routes, auth, worker mode, composition root                                                          | every crate above                                |

Two properties follow from that table. `crates/domain` reaches only
`question_model`, so it has no clock and no database: `chrono` is declared with
`default-features = false`, which drops the `clock` feature, and time arrives
as a parameter. That is what lets the same code run in a browser and makes the
seed-parity test meaningful. `crates/wasm` reaches only `question_model` and
`domain`, which is the grading guarantee above.

Two drivers are owned rather than shared. `sqlx` is declared only in
`crates/store` and `aws-sdk-s3` only in `crates/objects`; `crates/server`
enables them through features and names neither, so the database and object
clients stay replaceable behind their traits.

## Database storage

The initial migration under `schemas/migrations/` creates shared immutable
catalog tables separately from tenant-owned educational records. Compact
problem-version metadata remains unpartitioned for browsing, while JSONB
payloads use 16 hash partitions. Question attempts, submissions, grade events,
and audit events use monthly range partitions plus a default partition so an
unexpected timestamp cannot make an otherwise valid write disappear.

Every tenant-owned table enables and forces row-level security. A store
transaction first assumes the non-superuser, non-bypass `ple_app` role and then
sets `ple.tenant_id` with transaction-local scope. Returning the pooled
connection clears both settings. The separate `ple_student` role can read only
the browser-safe tables and has no grant on `answer_key`; `ple_grader` owns that
answer-bearing access. Before the tenant is known, the narrowly privileged
`ple_auth` role can access only `auth_session`, and forced RLS restricts that
access to the SHA-256 hash of the single presented opaque credential. The raw
credential is never stored in PostgreSQL.

Schema migration and application access are distinct privileges. The
connection used by `store::postgres::apply_migrations` must be able to create
roles and schema objects. A production runtime login needs membership that
permits `SET ROLE ple_app` and `SET ROLE ple_auth`, but must not itself be a
superuser or have `BYPASSRLS`. The local development superuser may perform both
jobs; that convenience is not the production role model.

`PostgresStore` serializes complete contract records as checksummed JSONB while
retaining normalized identity, relationship, timestamp, and pagination columns
for constraints and indexed queries. Activity rows and their compact summary
projection commit in one transaction. The backend-neutral `Store` trait keeps
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
The context-free `Store` catalog reads are explicitly public-only;
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
mechanics. An `IdentityProvider` establishes a validated `SessionSubject`; the
same route and store contracts can therefore accept a future institutional
OIDC, SSO, LTI, or explicit local-development provider without changing cookie
handling or tenant derivation. The production provider remains an owner
selection rather than an inferred implementation choice.

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

`crates/server/src/catalog.rs` authenticates every catalog request and accepts
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

`crates/server/src/course.rs` authenticates every course request and uses the
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

`crates/server/src/run.rs` resolves the authenticated enrollment owner before
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

| Service         | Image                                     | Purpose                                 |
| --------------- | ----------------------------------------- | --------------------------------------- |
| `api`           | built from `containers/Containerfile.api` | axum API server                         |
| `postgres`      | `postgres:latest`                         | shared content and tenant-owned records |
| `minio`         | `quay.io/minio/minio`                     | S3-compatible object storage            |
| `createbuckets` | `quay.io/minio/mc`                        | one-shot bucket creation                |

Details and commands are in [CONTAINER.md](CONTAINER.md); the macOS virtual
machine setup is in [MACOS_PODMAN.md](MACOS_PODMAN.md).

`/health` returns 200 only after a real `SELECT 1` and a real `HeadBucket`
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

## Gates

`./check_codebase.sh` runs both toolchains: TypeScript typecheck, wider
typecheck, ESLint at zero warnings, Prettier, Node unit tests, then
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo test --workspace`. A missing `cargo` emits a loud SKIP rather than
passing silently.

`pytest tests/` runs the repository hygiene suite. Browser tests live in
`tests/playwright/`, and slower whole-system checks in `tests/e2e/`, both
outside the fast lane.
