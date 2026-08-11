# Plan: Peptidyle Learning Engine platform build

## Status

[customer-spec.md](customer-spec.md) describes a
backend-agnostic assignment platform built around repeated attempts, algorithmic questions, and
question-level timing. M0 and M1 are complete, the main M2 through M4 learning paths are implemented,
retention R4.4, QTI profile import WP-QTI-12, course appearance WP-CA1 through WP-CA7/WP-RC1, and
production-seam closure WP-RC2 are accepted. The authoritative remaining package sequence, binary scope ledger, owners,
files, behavior, success conditions, and validation are in
[release_completion_plan.md](active/release_completion_plan.md). That release plan and
[implementation_status.md](implementation_status.md) own the current next package; this foundation
plan does not duplicate the changing handoff.

The separately active `docs/active_plans/peptidyle-walkthrough-plan.md` owns its corrected M8-M11
acceptance sequence. It proves visible instructor course, local roster, and corpus-backed assignment
setup followed by student keyboard take/score/repeat. Production email and canonical onboarding are
separate release work and do not gate that walkthrough.

ADAPT (`OTHER_REPOS/adapt/`) is the surface model and the source of the sharpest lessons, because its
weaknesses are visible in its own schema. Three review passes (`reviewer_commments.md`,
`reviewer_commments_2.md`, `reviewer_commments_3.md`) plus the owner's operating experience moved this
design. Six requirements shape it:

- **Answer-bearing content is a separate security class.** Answers, keys, and grading logic stay on
  the server, so grading is a server round trip.
- **Object storage is a core subsystem**, not an afterthought.
- **Identity must separate drafts from published problems.** ADAPT mints a durable official ID for
  every saved problem, so the owner's sandbox holds abandoned experiments carrying permanent catalog
  numbers.
- **The sharing boundary is educational content versus educational records.** Assignments are course
  artifacts, not shareable content.
- **Demand is met by adding replicas**, making in-process state and process clocks design errors.
- **Completion is not the end of activity.** The owner reports students voluntarily running a
  finished assignment 30 or more times to learn through algorithmic variation. This is the single
  largest change: Peptidyle is a high-volume attempt-event system, not an assignment submission
  system rather than an assignment submission system.

The intended outcome is ADAPT's surface and its best feature -- one published problem reusable by
thousands of instructors without copying -- without its three structural weaknesses: unbounded
payloads in operational tables, no content integrity, and identity granted before publication.

This plan records the platform architecture and accepted milestone history. Remaining implementation
uses dependency-ordered production packages with explicit owners and behavior gates; scaffolding or
mock-only wiring is never completion evidence.

## Decisions

The architecture decisions below remain authoritative. Every former scope uncertainty is now closed
in the in-scope and out-of-scope ledgers in
`docs/active_plans/active/release_completion_plan.md`. No implementer is expected to
choose a product default, storage rule, source format, authentication method, deployment tool, or
release boundary while writing code.

## Objectives

- Deliver a mastery loop whose perceived latency is dominated by local work: answer-format validation
  and the next question are already in the browser, and grading is a server round trip whose
  server-side processing time is measured and recorded as a baseline.
- Support unlimited post-completion practice runs as a first-class product behavior, with completion,
  grading, and variation as three independent policies.
- Guarantee no answer, key, or grading implementation is reachable from the browser, enforced by the
  crate dependency graph rather than reviewer discipline.
- Make every historical attempt reproducible from seed, generator version, and problem version, at a
  per-row cost small enough to survive hundreds of millions of rows.
- Separate draft identity from published identity so an abandoned experiment never occupies a durable
  catalog number.
- Keep published content shared and immutable while every educational record carries a tenant ID and
  is protected by database-enforced row-level security.
- Delete student records on a privacy-by-default schedule with configurable institutional retention,
  while anonymous question statistics survive so the library keeps improving.
- Keep binary and archival content out of PostgreSQL, with every artifact carrying checksum, size,
  media type, license, and provenance.
- Keep API containers stateless so demand is met by adding replicas.
- Freeze module contracts early enough that at least six lanes proceed in parallel without
  coordinating mid-flight.
- Land first-party algorithmic, WeBWorK, and iMathAS adapters, plus DOCX and PDF exam export.
- Demonstrate the local teaching loop end to end: instructor course/roster/assignment setup followed
  by student keyboard take, scoring, repeat practice, and instructor gradebook confirmation.

## Scope

- Preserve and close the accepted M0 through M4 architecture and behavior.
- Complete WP-RC1 through WP-RC12 from the release-completion plan in dependency order.
- Complete M8 through M11 from the corrected instructor-to-student walkthrough plan without an email
  or canonical-onboarding dependency.
- Treat all repository-owned Rust, TypeScript, SQL, container, OpenTofu, test, and documentation
  artifacts as in scope for the working-codebase release.
- Treat institutional credentials, legal certification, and named human-pilot participation as
  production-activation evidence with explicit external owners, not unfinished code.
- Require a real implementation, behavior gate, and independent review for every claimed capability.

## Design philosophy

Three organizing trade-offs.

**Secrecy over local speed.** Answers stay on the server, so responsiveness comes from moving
_non-secret_ work to the browser and hiding the round trip behind prefetch. Native H5P shows why this
matters: it ships answer evaluation to the browser, so any H5P question is inspectable by any student.
That is a property of the format, and it sets the adapter's honest capability declaration.

**Contracts before code.** Freezing interfaces in one milestone costs a serial stage and buys wide
parallelism afterward. It only works if the contracts are complete, so the contract-freeze milestone
ships executable reference implementations and conformance suites, not just type definitions.

**Logical tenancy over physical tenancy.** The owner's chosen boundary -- shared content, tenant-owned
records -- is preserved exactly. Its _implementation_ is one PostgreSQL cluster with a tenant ID on
every tenant-owned row and forced row-level security. Database-per-tenant at 1,000 instructors buys
operational overhead rather than safety, and RLS supplies the boundary physical separation was wanted
for: the database itself refuses a cross-tenant read, so correctness does not rest on every query
carrying the right filter. One cluster means one connection pool, one migration run, and one backup
policy, and the tenant ID column keeps a later physical split available without a schema change.

Cited from [REPO_STYLE.md](../REPO_STYLE.md):

- **Fix the design, not the symptom.** Grading lives in a crate the WASM build cannot depend on, so
  shipping a key to the browser is a compile error. Tenant isolation is a database policy, not a
  code-review habit.
- **Design for adaptability.** Every engine enters through one adapter trait publishing capabilities.
  Physical storage hides behind an object service. Catalog search hides behind a repository so a
  dedicated search service can replace PostgreSQL full-text without touching callers.
- **Atomic task decomposition.** The module catalog gives every module one owner, one contract, one
  independent verification.
- **Long-term over short-term.** Immutable published versions, globally unique external IDs, tenant
  IDs on every record, and cursor pagination land in the first schema, because all four are painful
  to retrofit and free to adopt now.
- **Perfect is the enemy of good.** No Kubernetes, Redis, Kafka, sharding, dedicated search index, or
  microservice fleet. M0 through M4 run on `podman compose` with MinIO.

Evidence strategy for uncertain methods:

- Cross-target determinism is settled by measurement in WP-C4: a committed seed table, hashed
  outputs, the same assertions under `cargo test` and `wasm-bindgen-test`. If parity fails the
  primitive is replaced before any dependent lane starts.
- The secret-free WASM claim is settled by WP-C5: an export allowlist plus a dependency-graph
  assertion. "We were careful" is not evidence.
- Tenant isolation is settled by a test that sets a foreign tenant context and proves the query
  returns zero rows, run in `tests/e2e/` on every gate.
- Performance gates assert correctness absolutely and speed relatively. A first run establishes a
  recorded baseline; later gates compare against that baseline rather than against a number chosen in
  advance. Grading latency is split into server-side processing time, which this project controls and
  can hold to a regression budget, and network round trip, which varies with the student's connection
  and is reported for context rather than gated.
- Exactness is required in one place and one place only: seeded generation must produce identical
  output on both targets, because the render cache and the reproducibility record are keyed on that
  equality. Everywhere else, tolerances and baselines are the right instrument.

## Detailed platform scope

- Create the Cargo workspace, Solid toolchain, container set, and object storage subsystem.
- Freeze every module contract with executable in-memory reference implementations and conformance
  suites before production backends consume it.
- Implement the question model, identity and lifecycle, attempt state machine, timing, scoring,
  capability validation, and audit events in Rust.
- Implement the enrollment, run, and attempt model with independent completion, grading, and
  variation policies, plus transactionally maintained summary rows.
- Implement the shared content catalog and tenant-owned records in one cluster with forced RLS.
- Implement the object store with immutable keys, checksums, and three-bucket separation.
- Implement the first-party algorithmic adapter, then WeBWorK, QTI, H5P, and iMathAS.
- Build the Solid student assignment interface and instructor assignment editor.
- Implement DOCX and PDF export, and the worker pool that produces them and drains render work.
- Implement LTI Advantage grade passback.
- Document architecture, contracts, question model, identity, storage, tenancy, and determinism.

## Non-goals

Phrased as the behavior to follow, per the **prompt positively** principle in
[REPO_STYLE.md](../REPO_STYLE.md). Each bullet
names what this plan does instead of the excluded alternative, so a subagent reading it acts on the
instruction directly.

- Serve native H5P as ungraded practice with `serverGrading: false`, and import supported types into
  the server-graded internal representation when grading is required.
- Keep the infrastructure to containers, one PostgreSQL cluster, object storage, and a worker pool.
  Kubernetes, an in-memory cache tier, a streaming bus, sharding, a dedicated search index, and
  multi-region deployment each have a documented threshold in the scale evaluation and arrive when
  measurement calls for them.
- Focus the product on assignments, problems, attempts, and grades. Discussions, clickers, LMS roster
  sync, external research exports, and generated question content stay outside this plan.
- Schedule learning trees as the first post-M6 candidate.
- Consume the WeBWorK renderer over HTTP as a separate service, using it as shipped.
- Store binary and archival content in object storage at every size.
- Derive every storage key from stable IDs and versions.
- Serve every asset through a stored object record.
- Read grades from `student_assignment_summary`.
- Paginate every list endpoint with a cursor.
- Model assignments as tenant-owned course artifacts that reference shared published versions.
- Use executable in-memory reference backends for fast tests and PostgreSQL for every environment
  holding durable student records.

## Current state summary

- The Rust workspace, Solid browser application, generated contract pipeline, PostgreSQL schema,
  object store, API, worker, local container stack, and repository gates are implemented.
- M0/M1 contracts, core author/publish/assign/run/grade/feedback/export/statistics paths, retention
  R4.4, QTI profile import WP-QTI-12, the database baseline, keyboard pass, local launcher, and course
  appearance WP-CA1 through WP-CA7/WP-RC1 have accepted evidence.
- `launch_local_stack.sh` builds, migrates, seeds, starts, waits for semantic health, and opens the
  local application. Its default profile intentionally excludes WeBWorK until WP-RC3 replaces the
  invented private dialect with the shipped upstream service contract.
- The first forward migration after the accepted six-file baseline is
  `schemas/migrations/2026080907_course_appearance.sql`; later filenames and owners are reserved in
  the release-completion plan.
- The remaining work is not an architecture discovery exercise. WP-RC2 through WP-RC12 name every
  production artifact, behavior, validation command, and release boundary.

### What ADAPT actually does with content

Established from migrations and controllers, because `reviewer_commments_2.md` asked for evidence
rather than characterization. The answer is **type-based hybrid storage with no size threshold**:

| Question asked                        | Finding                                                                      | Evidence                                                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Binary in MySQL?                      | No. Zero `blob`, `binary`, or `mediumBlob` columns in any migration          | `OTHER_REPOS/adapt/database/migrations/`                                                                 |
| Large XML or JSON moved out of MySQL? | No. QTI XML is a `text` column; parsed `qti_json` stays on the questions row | `2022_05_06_150939_create_qti_imports.php:21`, `2023_02_03_115902_update_qti_json_type_to_questions.php` |
| Small images also in S3?              | Yes. All media goes to S3 by type; no threshold exists                       | `2024_06_03_173537_create_question_media_uploads_table.php`                                              |
| Configured size threshold anywhere?   | None found                                                                   | upload and import controllers                                                                            |
| Original imported package preserved?  | No evidence. Only parsed XML plus `directory` and `filename`                 | `qti_imports` schema                                                                                     |
| H5P packages stored?                  | No. Referenced by remote `technology_id`                                     | `OTHER_REPOS/adapt/app/Question.php`                                                                     |

Three weaknesses neither review named, each becoming a requirement here:

- **No checksum column** on `question_media_uploads` (`id`, `question_id`, `original_filename`,
  `size`, `s3_key`, `transcript`, `status`). No checksum means no deduplication, no corruption
  detection, and no way to prove a historical attempt saw a given image.
- **Keys are random, not content-addressed, and filenames participate in identity.**
  `$s3_key = md5(uniqid('', true)) . '.html'`
  (`OTHER_REPOS/adapt/app/Http/Controllers/QuestionMediaController.php:242`), and `qti_imports`
  uniquely indexes `(user_id, directory, filename)`.
- **Signed URLs live seven days.** `temporaryUrl(..., Carbon::now()->addDays(7))`
  (`QuestionMediaController.php:279`). A leaked URL grants a week of access, which for a
  student-record artifact is a FERPA-relevant exposure. This plan uses minutes.

## Resolved decisions

| Decision             | Choice                                                                                                                  | Reason                                                                                                                                                                                                                 |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Server runtime       | Native Rust `axum`; shared crates also built for `wasm32`                                                               | Owner-selected. Native is fastest and keeps direct database access                                                                                                                                                     |
| Web server           | No Apache, nginx, or lighttpd in the request path                                                                       | See the LAMP mapping below; all reviews agree the load balancer replaces Apache                                                                                                                                        |
| Database             | **PostgreSQL on RDS**, one cluster                                                                                      | Owner-selected. JSONB with indexing, forced row-level security, mature `FOR UPDATE SKIP LOCKED`                                                                                                                        |
| Tenancy              | **Logical: tenant ID on every tenant-owned row, forced RLS. Not a database per tenant**                                 | Reviewer 3. Database-per-instructor at 1,000 instructors is operational overhead without safety gain; RLS is database-enforced, so a missing `WHERE` cannot cross tenants. The column preserves a later physical split |
| Grading location     | **Server only**                                                                                                         | Owner-selected. No answer, key, or grading code reaches the browser                                                                                                                                                    |
| H5P grading          | Native H5P is ungraded practice; `serverGrading: false`                                                                 | Owner's observation: H5P ships answer evaluation to the browser                                                                                                                                                        |
| WASM contents        | Parameter generation, answer-format validation, timer display, state transitions                                        | Non-secret work only, enforced by the dependency graph                                                                                                                                                                 |
| Sharing boundary     | **Shared educational content; tenant-owned records and course artifacts**                                               | Owner-selected, refined by reviewers. Assignments reference immutable published versions and are never copied                                                                                                          |
| Activity model       | `assignment_enrollment` / `assignment_run` / `question_attempt`                                                         | Owner's 30-runs observation. Completion is not terminal; practice continues with new variants                                                                                                                          |
| Grade computation    | Transactionally maintained summary rows; never scan attempt history                                                     | At 300 M+ attempt rows, scanning for a course page is not an option                                                                                                                                                    |
| Problem identity     | `workspace_id`, `problem_id`, `version_id`; external IDs are UUIDv7 or random                                           | Only publication mints a durable `problem_id`. Non-sequential IDs distribute S3 prefixes and index inserts                                                                                                             |
| Partitioning         | Monthly range partitions on the four highest-volume append-only tables only                                             | 300 M rows per term makes this non-speculative; everything else stays unpartitioned per reviewer 3                                                                                                                     |
| Pagination           | Cursor only; `OFFSET` banned by lint and review                                                                         | Large `OFFSET` scans are unusable at catalog and history scale                                                                                                                                                         |
| Content storage      | Split by role with a size backstop (below)                                                                              | Answers the owner's direct question                                                                                                                                                                                    |
| Flat question source | Versioned PLE flat-question JSON, compiled into separate public and grader-only values                                  | Keeps ordinary static authoring small and deterministic; QTI remains an import/export adapter instead of defining the internal model                                                                                   |
| Catalog table split  | `problem_version` metadata separate from hash-partitioned `problem_version_payload`                                     | 10 M payloads is ~100 GB; browse and search must run on the ~2 GB metadata table                                                                                                                                       |
| Object storage       | S3 with three buckets; MinIO locally                                                                                    | Different retention, access, and logging policies per bucket                                                                                                                                                           |
| Asset delivery       | CDN with long-lived immutable URLs for public content; authorized short-lived URLs for secure and student-record assets | Routing 50,000 students' image requests through the API is waste; immutable keys make CDN caching safe for non-records                                                                                                 |
| Rendered output      | Cached by `(version_id, seed)` in the `content` bucket and CDN                                                          | Rendering is deterministic, so a Perl fork becomes a CDN hit. Determinism pays for itself twice                                                                                                                        |
| Session storage      | Opaque session ID cookie, session row in the database                                                                   | Works across replicas and stays revocable                                                                                                                                                                              |
| Timer clock          | Timestamps from PostgreSQL, never a process clock                                                                       | Replica clock skew would otherwise change verdicts                                                                                                                                                                     |
| Background work      | `worker` container pool on a jobs table with `FOR UPDATE SKIP LOCKED`                                                   | Import, export, and renderer work leave the request path                                                                                                                                                               |
| Autoscaling          | Fargate target tracking: request count for `api`, queue depth for `worker`                                              | A class-start burst is a request spike; renderer load is a queue-depth signal                                                                                                                                          |
| Execution shape      | Contract freeze, then parallel module lanes                                                                             | Owner-requested modularization; see the module catalog                                                                                                                                                                 |
| Repo layout          | Reduced monorepo: `src/`, `crates/`, `pipeline/`, `containers/`, `schemas/`                                             | No `apps/` or `packages/` split until a second app exists                                                                                                                                                              |

### Recorded disagreement with reviewer 1

`reviewer_commments.md` recommends a TypeScript API server with Rust as a called library. The owner
chose the native Rust server with the trade-offs visible. The reviewer's velocity argument is real;
the counter-arguments are that two languages in the request path means serialization at every domain
call, and that `customer-spec.md` itself requires the TypeScript layer not to contain grading rules.
Recorded rather than reopened; raise it if velocity becomes the binding constraint.

### Answering the storage question directly

**A split, but a narrow and principled one. Not "just metadata."** The rule is by _role_, because role
is stable, with size only as a backstop:

| Category                    | Home                                        | Contents                                                                                                                        |
| --------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Operational content         | PostgreSQL                                  | The compact normalized question model the renderer and grader execute; grading rules; policies; all metadata and references     |
| Answer-bearing content      | PostgreSQL, separate tables, separate grant | Answer keys and checker configuration, readable only by the grading role, never joined into a student-facing query              |
| Archival and binary content | Object storage, always, at any size         | Original QTI ZIP, images, audio, video, H5P packages, DOCX and PDF exports, large source bundles                                |
| Derived artifacts           | Object storage, separate prefix             | Rendered output, sanitized HTML, extracted resources, thumbnails, student-specific exports. Regenerable, so different retention |
| Temporary                   | Container disk, then discarded              | Archive extraction, conversion, scanning                                                                                        |

Backstop: normalized operational payloads remain subject to their accepted schema and contract
ceilings, including the 256 KiB private grading ceiling where defined. An oversized source/private
write refuses before mutation; archival source and binary bytes use typed object storage. Version 1
does not silently replace a hot-path normalized model with an object reference, and the shipped
ceilings are not an unresolved profiling decision.

Why not metadata-only: a normalized question model is kilobytes and is read on **every attempt**.
Pushing it to object storage adds a network hop to the hottest path for no benefit, the exception
`reviewer_commments_2.md` names. Why not ADAPT's approach: unbounded payloads in operational tables
with no threshold and no checksum, which is the bloat the owner is right to worry about.

The rule that makes the split safe: **every artifact carries identity metadata regardless of which
side it lands on** -- `object_id`, `sha256`, `size_bytes`, `media_type`, `category`, `version_id`,
`license`, `provenance`. Text in PostgreSQL is checksummed exactly like a ZIP in S3.

### The modern LAMP equivalent

| LAMP letter | 1999                                                    | This project                                    |
| ----------- | ------------------------------------------------------- | ----------------------------------------------- |
| **L**inux   | Host OS, hand-configured                                | Container image, immutable                      |
| **A**pache  | HTTP server, mod_php process manager, static files, TLS | **Nothing.** The Rust binary is the HTTP server |
| **M**ySQL   | Same box as Apache                                      | PostgreSQL on RDS, plus S3 for files            |
| **P**HP     | Templates rendering HTML per request                    | TypeScript in the browser, Rust on the server   |

Apache is not obsolete, it is unemployed here. Its four historical jobs are gone or reassigned: there
is no CGI, so `axum` on `tokio` handles concurrency in-process with no interpreter pool to supervise;
`ServeDir` and CloudFront serve static files; the load balancer terminates TLS and rotates
certificates; routing is typed application code rather than a config file no test covers. Adding
Apache in front would buy a network hop, a second config surface, and a second thing to patch.

On the alternatives asked about: nginx is right when a reverse proxy is genuinely needed and the
operator knows it, but the ALB covers that role. lighttpd offers no 2026 advantage and a smaller
community. A Python server is wrong twice: `http.server` is not production, and gunicorn or uvicorn
would mean a Python backend, contradicting the Rust decision.

## Architecture boundaries and ownership

### Contributor-facing component names

Use plain capability names first. Cargo package and crate-directory names use
hyphens; Rust import and module names use underscores:

| Plain component name  | Physical path                                | Rust name              | Responsibility                                                                                        |
| --------------------- | -------------------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------- |
| Learning data access  | `crates/learning-data-access`                | `learning_data_access` | Typed persistence contracts, authorization-aware queries, PostgreSQL, migrations, and RLS             |
| In-memory data access | `crates/learning-data-access/src/in_memory*` | `in_memory`            | Database-free implementation used by conformance and server behavior tests                            |
| Project tools         | `crates/project-tools`                       | Cargo-only binary      | Repository-only generation, fixtures, database operations, and E2E seeding invoked with `cargo tools` |

These names were migrated atomically with all callers, manifests, tests,
scripts, and documentation. The former `cargo xtask` alias is retired.

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

### The sharing boundary

The boundary is **educational content versus educational records**, which is also the FERPA line and
the reuse line. Those turn out to be the same line, which is why the design works. Both live in one
cluster; the distinction is ownership and policy, not physical separation.

| Shared content (no tenant ID)                    | Tenant-owned (tenant ID on every row, RLS enforced) |
| ------------------------------------------------ | --------------------------------------------------- |
| Published problem catalog                        | Courses and sections                                |
| Immutable problem versions                       | Assignments                                         |
| QTI, H5P, WeBWorK, and iMathAS source references | Instructor workspaces                               |
| Shared media assets                              | Draft problems                                      |
| Tags, taxonomy, licensing                        | Enrollments, runs, attempts, submissions            |
| Backend capability definitions                   | Grades, summaries, timers                           |
| Anonymous question statistics                    | Per-student analytics and audit logs                |
| Public and community libraries                   | Student-record artifacts                            |

An assignment is **not** shareable content. It is a course artifact referencing published problems,
which is what lets one published problem serve thousands of instructors without copying:

```text
Published Problem Version (shared, immutable)
        |                         |
        v                         v
Assignment (Tenant A)     Assignment (Tenant B)
        |                         |
        v                         v
Runs and Attempts (A)     Runs and Attempts (B)
```

RLS is enforced, not advisory: every tenant-owned table declares `FORCE ROW LEVEL SECURITY`, the
application connects as a non-superuser role that cannot bypass it, and the tenant context comes from
a session variable set from the authenticated session -- never from a client-supplied parameter. A
test in `tests/e2e/` sets a foreign tenant context and asserts zero rows.

Buckets, separated so retention, access, and logging policies differ:

| Bucket            | Contents                                                      | Delivery                                                                          | Retention                        |
| ----------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------- | -------------------------------- |
| `content`         | Source packages, shared assets, cached renders                | CDN, immutable URLs for public content; authorized 60-min URLs for secure content | Indefinite, versioned            |
| `student-records` | Student-specific exports, uploaded responses, annotated exams | Authorized 5-min URLs, always logged                                              | Explicit expiration and deletion |
| `temp-processing` | Extraction and conversion workspaces                          | Never served                                                                      | Lifecycle rule, days             |

Crates and their forbidden dependencies:

Each crate's dependency list is exhaustive: the allowed set is the whole set, so a `Cargo.toml`
matching this column satisfies the boundary by construction.

| Crate                                      | Owns                                                                                                                    | Depends only on                                  |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `crates/question_model`                    | Question types, capabilities, identity, taxonomy, `ts-rs` derives                                                       | External crates                                  |
| `crates/domain`                            | Attempt state machine, run and completion rules, timing verdict, seeded generation, capability validation, audit events | `question_model`                                 |
| `crates/grading`                           | **Answer keys, checkers, correctness decisions**                                                                        | `question_model`, `domain`                       |
| `crates/objects`                           | `ObjectStore` trait, S3 and MinIO backends, key construction, checksums                                                 | `question_model`                                 |
| `crates/learning-data-access`              | `Store` trait, PostgreSQL backends, migrations, RLS context management                                                  | `question_model`, `domain`, `objects`            |
| `crates/adapters/{native,webwork,qti,h5p}` | Per-engine load, generate, grade delegation, capability declaration                                                     | `question_model`, `domain`, `grading`, `objects` |
| `crates/export`                            | Print model, DOCX and PDF writers                                                                                       | `question_model`, `objects`                      |
| `crates/wasm`                              | `wasm-bindgen` bridge, delegating every call to `domain`                                                                | `question_model`, `domain`                       |
| `crates/server`                            | axum routes, auth, worker mode, composition root                                                                        | Every crate above                                |

Two load-bearing properties follow from that table. `crates/domain` reaches only
`question_model`, so it has no clock and no database, which lets it run in a browser and makes the
seed-parity test meaningful; time and storage arrive as parameters. `crates/wasm` reaches only
`question_model` and `domain`, so the answer-bearing surface in `crates/grading` sits outside its
dependency closure and shipping an answer to the browser becomes a compile-time impossibility rather
than a code-review question.

## Activity model

The owner's observation that students voluntarily rerun completed assignments 30 or more times for
learning is the largest single change to this plan. Completion is not terminal, and the earlier
model -- assignment to attempts -- could not express it.

Three levels, per reviewer 3:

| Entity                  | Holds                                                                             | Cardinality                               |
| ----------------------- | --------------------------------------------------------------------------------- | ----------------------------------------- |
| `assignment_enrollment` | Completion status, first completion time, current and best grade pointers         | One per student per assignment            |
| `assignment_run`        | Run number, started and completed times, score, mode, variation policy applied    | Many per enrollment; 30 or more is normal |
| `question_attempt`      | Run ID, question version ID, seed, parameter hash, response, result, timer record | Several per question per run              |

Four policies, deliberately independent so an instructor can combine them freely:

| Policy                 | Options                                                    |
| ---------------------- | ---------------------------------------------------------- |
| Completion requirement | First time all required questions are correct              |
| Grade policy           | First, latest, highest, or instructor-defined              |
| Continued practice     | Unlimited new runs after completion, or capped             |
| Variation policy       | New seeds, selected problem variants, or full regeneration |

Two derivations that look contradictory and are not, so the distinction is stated once here:
**within-run completion is derived** from question states, never stored as a boolean, which keeps the
state machine honest. **Cross-run grade state is a maintained summary row**, updated transactionally
when a run changes, because scanning hundreds of millions of attempts to render a course page is not
an option. Different scopes, different mechanisms.

`student_assignment_summary` holds `best_score`, `latest_score`, `completed_run_count`,
`total_question_attempts`, and `last_activity_at`. Ordinary course and gradebook pages read only the
summary. Historical runs stay available for learning analysis through explicitly asynchronous
analytics, never through a synchronous page query.

## Data retention and deletion

ADAPT's retention practice -- notify the instructor 30 days after a course ends, automatically reset
at 100 days -- is privacy by default rather than privacy by policy, and it is worth adopting rather
than reinventing. It also validates the content-versus-records boundary: deleting every student record
in a course destroys no reusable content, because assignments reference shared problem versions
instead of owning copies. The boundary was drawn for sharing and turns out to be the same boundary
retention needs.

Lifecycle, with each stage a scheduled worker job:

```text
course ends
   |
   +30 days   notify instructor: archive, delete, or extend
   |
   +100 days  automatic archive of student records (configurable default)
   |
   +1 year    permanent deletion of student records (configurable)
```

What each stage touches:

| Deleted with student records         | Retained indefinitely                                        |
| ------------------------------------ | ------------------------------------------------------------ |
| Enrollments                          | Published problems and immutable versions                    |
| Runs, question attempts, submissions | Shared problem catalog, taxonomy, licensing                  |
| Grades and summary rows              | Instructor question drafts and workspaces                    |
| Timer events and render traces       | Assignment definitions (instructor's choice at archive time) |
| Per-student analytics                | Backend capability metadata                                  |
| Student-record bucket artifacts      | Anonymous question statistics (below)                        |

Wording matters in the notification, because "reset" sounds like breakage. The instructor-facing copy
is: _This course ended 30 days ago. Student records are still available. If they are no longer needed,
archive or delete the course now. Student records will be automatically removed after 100 days unless
the course is archived or the retention period is extended by an administrator._

Retention is configurable per institution because institutional policy varies -- 100 days, one year,
five years, or never-automatic for a research institution -- with the privacy-preserving default
applying when nothing is configured. The software encourages the short default without forcing it.

### How backups interact with deletion

Stated plainly because institutions will ask, and because the honest answer is a constraint rather
than a feature. Deletion removes student records from live systems immediately and irreversibly.
Encrypted backups and point-in-time recovery snapshots taken before the deletion still contain those
records until they age out under the backup window's own retention.

Selective purge from a point-in-time recovery window is not possible with managed snapshots, so the
documented guarantee is: _deleted student records are immediately unrecoverable through the
application, and expire from encrypted backups within the configured backup window._
`docs/RETENTION_POLICY.md` states the current window numerically. An institution requiring a shorter
total exposure must shorten its backup window, which is a deliberate durability-versus-privacy
trade-off for that institution to make rather than one this platform makes for them.

### Anonymous question statistics survive deletion

The feature that makes deletion sustainable: the question library should keep improving after the
records that taught it are gone. Aggregate statistics live in **shared content**, carry no tenant or
student identifiers, and survive record deletion:

```text
Question 123 (version_id ...)
  attempts_mean 2.7
  time_median_s 58
  difficulty_index 0.71
  discrimination_index 0.43
  cohort_size 214
```

Three design consequences, because this cannot be bolted on afterward:

- Statistics are **aggregated incrementally or by scheduled rollup while records still exist**, never
  computed on demand from attempt history. Deleting the attempts must not delete the knowledge.
- Discrimination index needs per-student total scores, so it is computed before deletion and stored as
  an aggregate. A post-deletion recomputation is impossible by design, which is the point.
- Aggregates are suppressed below a minimum cohort size. With one student, "average attempts: 3" _is_
  that student's record, so a k-anonymity threshold (default 5) gates publication. The reviewer did
  not raise this; it is the difference between an anonymous statistic and a re-identifiable one.

## Problem identity and lifecycle

The fix for a sandbox full of junk carrying official numbers. Three IDs with distinct jobs:

| ID             | Scope                                          | Mutability                      | Visibility                                 |
| -------------- | ---------------------------------------------- | ------------------------------- | ------------------------------------------ |
| `workspace_id` | One instructor's sandbox item, tenant-owned    | Freely editable and deletable   | Private to owner and invited collaborators |
| `problem_id`   | A reusable published problem in shared content | Stable forever once minted      | Globally discoverable and citable          |
| `version_id`   | One immutable revision                         | Never changes after publication | Referenced by assignments and attempts     |

Lifecycle: `draft -> validated -> published -> deprecated -> archived`.

- A draft gets an internal UUID immediately so it can be referenced and collaborated on, but that
  UUID is never presented as a problem number.
- Only the publish transition mints a `problem_id`, and only after validation passes.
- Editing a published problem creates a new `version_id`; it never mutates an existing one.
- Replacing an image creates a new asset object with a new checksum and key; the old object stays so
  historical attempts remain reproducible.
- Assignments reference `(problem_id, version_id)`. Publishing a new version changes no existing
  assignment; adopting it is an explicit instructor action.
- `deprecated` hides a version from search while keeping it resolvable for existing assignments;
  `archived` additionally blocks new references. Deprecation carries a stated reason, which is how an
  author signals "this version contains an error."

### Publication governance

Who may publish into a shared catalog is a product decision that shapes the data model, so it is
settled here rather than discovered during M4.

Three publication scopes, each a different visibility contract:

| Scope         | Visible to                                     | Requires                                                                      |
| ------------- | ---------------------------------------------- | ----------------------------------------------------------------------------- |
| `private`     | The owning workspace and invited collaborators | Nothing                                                                       |
| `institution` | Every course in the tenant                     | Validation passing                                                            |
| `public`      | The shared catalog, every tenant               | Publisher role, validation passing, and an optional institutional review gate |

The review gate is configurable per institution and off by default, because a two-instructor
deployment does not need editorial process and a large institution may require it.

Conflicting revisions from multiple authors are prevented structurally rather than merged. A published
problem has an owning author or author set and a **linear version chain**; a third party cannot publish
a new version into someone else's `problem_id`. Instead they **fork**: a new `problem_id` recording
`derived_from` the source `version_id`, preserving attribution and license lineage. That keeps every
version chain single-writer, which is why no merge semantics are needed anywhere in the model. The
`derived_from` field is the reason this decision touches the schema.

### Telling institutions a better version exists

Immutability plus manual adoption means a defect fix could sit unnoticed, which would make a shared
catalog untrustworthy. Publishing a new version therefore emits a **version-available notification** to
every tenant holding an assignment that references an earlier version, carrying a severity:

- `correction` -- the earlier version is wrong. Surfaced prominently, with a diff, and recorded in the
  instructor's action list.
- `improvement` -- wording, clarity, or added variants. Surfaced quietly.

Adoption stays a manual instructor action in both cases. Silent upgrades are refused on purpose:
changing a published version mid-term would change what students see and would break the
reproducibility record for attempts already taken.

## Object storage and content identity

Keys are immutable and derived from IDs and versions, never from user-supplied filenames:

```text
content/problems/{problem_id}/versions/{version_id}/source/qti-package.zip
content/problems/{problem_id}/versions/{version_id}/assets/{asset_id}.png
content/problems/{problem_id}/versions/{version_id}/renders/{seed}.html
student-records/exports/{tenant_id}/{export_id}/exam.pdf
temp-processing/imports/{import_id}/
```

Every object record carries `object_id`, `bucket`, `key`, `sha256`, `size_bytes`, `media_type`,
`category` (`source` / `asset` / `render` / `export`), `license`, `provenance`, `created_at`.

Requests resolve assets from a known object record and read pre-parsed models, so bucket listings and
archive parsing stay in the worker at import time. Public content assets are served from the CDN by
immutable URL; secure and student-record assets go through `/api/assets/{id}`, which authorizes, logs,
and redirects to a short-lived signed URL.

The `renders/{seed}` prefix is what makes the WeBWorK renderer affordable: rendering is deterministic
given `(version_id, seed)`, so the first render fills the cache and every later student with that seed
gets a CDN hit instead of a Perl fork.

Authoritative-versus-derived roles, settled per backend:

| Backend            | Authoritative artifact                                     | Derived                                           |
| ------------------ | ---------------------------------------------------------- | ------------------------------------------------- |
| Native algorithmic | Generator id and version; parameters derived from the seed | Rendered output                                   |
| Native static      | Canonical versioned PLE flat-question JSON source          | Public question model and private grader material |
| WeBWorK            | PG source reference and version                            | Rendered HTML, images, cached renders             |
| QTI                | Original ZIP in object storage                             | Parsed model in shared content, extracted assets  |
| H5P                | Remote package reference                                   | Any imported internal representation              |

QTI import runs in the worker, never a request: store the original ZIP unchanged; validate structure;
reject unsafe paths, symlinks, and unexpected entries; enforce maximum archive size, maximum expanded
size, and file-count limits; extract into an isolated `temp-processing` workspace; parse the manifest;
store each referenced asset as its own checksummed object; rewrite content references to internal
asset IDs; convert supported content into the internal model; record unsupported features explicitly
so they survive as data; preserve the original package so a later parser improvement can re-import.
Determine every media type by sniffing the stored bytes, treating any supplied type as a hint to
verify.

PLE flat-question JSON follows the same source-preservation principle without
copying QTI's interchange model. The bounded answer-bearing JSON is private in
the workspace, is canonicalized and checksummed by the native adapter, and is
promoted to an immutable non-signable `ProblemSource` object at publication.
The compiler writes an answer-free operational JSONB projection and separate
grader-only key/feedback JSONB. Student rendering and grading read those compact
database projections rather than fetching or reparsing the source object.

The 2026-08-09 flat-question package implements that transition: a typed
compare-and-swap save atomically advances the workspace draft and source
metadata; publication binds the copied canonical source, answer-free model, and
typed private payload in one catalog transition; and native runtime grading
uses an independently injected grader-only capability. The completed instructor
editor uses the protected author-only canonical-source route as its narrow
answer-bearing browser exception; learner and public contracts remain
answer-free. The bounded Canvas and Blackboard QTI profile mappings, Q3 pure
native flat bridge, Q4 provenance contract, Q5/WP-QTI-7 schema/RLS/object-
binding implementation, WP-QTI-8 Memory/PostgreSQL conversion boundary, and WP-QTI-9 server routes
are complete. WP-QTI-8
closes staged profile evidence, revalidates exact accepted-result `itemId` binding, and atomically
commits the CAS revision, draft, canonical source, current private grading, and current origin under
the frozen lock order. Ordinary saves stage current grading, publication promotes only the stored
grading value after origin promotion, and PostgreSQL reaches private provenance and grading only
through forced-RLS broker capabilities. Strict lowercase 64-hex `Sha256Digest` serialization keeps
the evidence boundary exact. WP-QTI-9 adds deterministic private archive/job ingress, strict worker
evidence, answer-free report review, strong-ETag atomic conversion, deterministic published archive
copy, and a prepared-import draft-deletion fence with Memory/PostgreSQL parity. The route, worker,
and independent P0/P1 evidence passed. WP-QTI-10 author UI is also complete: it uses a feature-local
answer-free QTI client and the existing workspace route/editor, keeps the selected ZIP and safe report
only in component memory, requires an acknowledged report plus the displayed clean strong revision,
and locks the stale editor through conversion/refetch recovery. Real-route Chromium and offline
evidence passed. WP-QTI-11 live PostgreSQL/RLS/profile-to-native acceptance is complete: a fresh
PostgreSQL 17 database exercised the real upload worker, mixed accepted/rejected report, native
conversion and publication, correct/incorrect grading, role denials, provenance, and exact cleanup.
WP-QTI-12 independent review and documentation close-out are also complete: six separate passes
reported no remaining P0/P1 issue after stale README and ownership-map findings were corrected and
re-reviewed. PLE flat JSON v2 now implements MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT
source/runtime semantics based on the reviewed QTI Package Maker item models, while exact v1 remains
preserved. Remaining acceptance is recorded in
`docs/active_plans/active/flat_question_family_evolution_plan.md`; external QTI-JSONL is a separate
future adapter concern. The course appearance package is accepted through WP-CA7/WP-RC1 and production-seam closure
WP-RC2 is accepted; WP-RC3 shipped upstream WeBWorK is the current implementation package. The dependency order and exact
profiles, refusal semantics, provenance
boundary, author workflow, and acceptance gates are frozen in
[qti_profile_mapping_plan.md](decisions/qti_profile_mapping_plan.md).

Deduplication is designed for but not built: the logical `asset_id` is stable and the physical key is
chosen inside MOD-OBJ, so a later move to `objects/sha256/ab/cd/...` changes no caller.

## Reproducibility record

Every question attempt persists: `problem_id`, `version_id`, source artifact `object_id` and `sha256`,
adapter id and version, renderer version where one applies, generator id and version, seed,
**parameter hash**, asset `object_id` list, grading implementation version, and rendered-question hash.

The record stores a parameter _hash_ rather than the parameters, because parameters are reproducible
from seed plus generator version by construction and WP-C4 proves regeneration is exact. At 300 M
attempts per term, storing both would add hundreds of gigabytes to restate what the seed already
determines. The hash still detects a regeneration mismatch.

This is the requirement that forces immutability upstream: a mutable question row or an overwritten
image makes the record a lie.

## Source-of-truth and compatibility policies

Four ownership questions that look like implementation details and are actually architecture. Each is
settled here because discovering the answer during recovery or a decade-later migration is expensive.

### Which artifact defines a native question

The other backends have obvious authoritative artifacts; the native backend
needed separate rulings for generated and static questions.

For an algorithmic question, **the pinned generator identifier, generator
version, and parameter specification are authoritative.** The normalized
question model in `problem_version_payload` is a derived, cached projection for
rendering and search, regenerable from the pinned generator at any time.

The consequence that matters: **a generator evolving leaves every existing published version intact.**
Generator version is part of the published version's identity, so improving a generator produces a
_new_ `version_id` and every existing assignment and attempt keeps resolving to what it already had.
Generator versions are therefore additive-only; removing one is a breaking change requiring explicit
deprecation of every version that pins it.

For a static flat question, **the canonical, versioned PLE flat-question JSON
source is authoritative.** Publication preserves it as a private immutable
source object and compiles it into an answer-free public model plus separately
granted grader-only material. The two compiled values carry checksums and a
public/private binding, but neither replaces the preserved source for recovery
or future re-import. QTI remains an adapter into and out of supported internal
semantics; Canvas or Blackboard XML never becomes the PLE source contract.

| Backend            | Authoritative                                           | Derived and regenerable                             |
| ------------------ | ------------------------------------------------------- | --------------------------------------------------- |
| Native algorithmic | Pinned generator id, generator version, parameter spec  | Normalized model, rendered output                   |
| Native static      | Canonical versioned PLE flat-question JSON source       | Public model and private grader material            |
| WeBWorK            | PG source reference and version                         | Normalized model, rendered HTML, cached renders     |
| QTI                | Original ZIP in object storage                          | Parsed model, extracted assets                      |
| H5P                | Remote package reference                                | Any imported internal representation                |
| iMathAS            | Checksum-pinned source snapshot and integration profile | Safe render envelope and deterministic render cache |

### Reading a version 1 payload with version 5 software

Immutability creates a long-lived compatibility obligation. Every stored payload carries a
`model_schema_version`, and readers **upcast on read** into the current in-memory model, leaving the
immutable row as written.

The mechanism that keeps this honest is a committed corpus holding one payload per historical schema
version, with a test asserting every one still loads into the current model. A schema change that
cannot upcast an existing corpus entry is rejected at the gate. Dropping support for a historical
schema version is an explicit breaking change requiring a documented batch re-publication path, never
a silent read failure.

### Database or object store: who owns existence

The reconciliation job implies these can diverge, so the asymmetry is stated rather than discovered:

- **The database is authoritative for object existence.** An object record with no corresponding
  bucket object is a _broken reference_: a defect, alerted on, never auto-repaired by deleting the
  record.
- **The object store is authoritative for bytes.** A bucket object with no record is an _orphan_:
  garbage, collectable after a quarantine window.

Write ordering follows from that: **bytes first, record second.** A crash between the two leaves an
orphan, which is harmless and collectable. The reverse ordering would leave a broken reference, which
is harmful. Checksums are verified on read so a silently corrupted object surfaces as an error rather
than as wrong content shown to a student.

### Grading version compatibility and regrading

**Grading implementations are additive and permanently executable.** A grading version is never
removed while any attempt references it, because a grade is a record and being unable to explain how it
was produced is not acceptable.

Regrading is supported and explicit: it creates a **new grade event** referencing the new grading
version, never overwriting the old one. The history therefore shows both results and the reason for the
change, which is what makes "why did my grade change" answerable. A test asserts every grading version
referenced by the fixture corpus is still callable.

## Scale evaluation

Target: 10,000,000 problems, 1,000 instructors, 50,000 students. Start: 500 problems, 2 instructors,
100 students. The architecture holds at both ends, and the deployment size changes while the
application model does not. The arithmetic, and the seven decisions it forces, follow.

### Activity volume is the dominant concern

With the owner's practice behavior included, per term:

| Quantity                                           | Value                 |
| -------------------------------------------------- | --------------------- |
| Students x assignments x questions x complete runs | 50,000 x 10 x 20 x 30 |
| Question instances per term                        | ~300 M                |
| Plus incorrect attempts within runs                | 500 M+ rows over time |
| Peak submission rate (due-date evening)            | ~300-500 / s          |
| Database writes per submission                     | ~4                    |
| Peak write rate                                    | ~2,000 writes / s     |

Two thousand small writes per second is routine for a provisioned RDS PostgreSQL instance, so the
request path is fine. The row count is what drives design:

- The four highest-volume tables (`question_attempt`, `submission`, `grade_event`, `audit_event`) use
  monthly range partitions from the first migration. At 300 M rows per term this is not speculative,
  and partitions cost nothing at 100 students. Reviewer 3 cautions against partitioning everything
  from the prototype, which this follows: nothing else is partitioned, and every other table carries
  the IDs and timestamps that permit partitioning later.
- Grades come from `student_assignment_summary`, never from scanning attempts.
- Attempt rows stay compact: seed plus parameter hash, not parameters.
- Verbose render traces and temporary artifacts get retention rules, not indefinite storage.

### 10 million problems

Ten million rows is unremarkable for PostgreSQL. Ten million _payloads_ is not: at roughly 10 KB
each that is ~100 GB in operational tables. Most are cold, so the schema splits hot metadata from
cold payload:

| Table                     | Contents                                                     | Size at 10 M              | Access                       |
| ------------------------- | ------------------------------------------------------------ | ------------------------- | ---------------------------- |
| `problem_version`         | Identity, lifecycle, capability and taxonomy refs, checksums | ~2 GB                     | Hot; browse, search, resolve |
| `problem_version_payload` | Normalized question model                                    | ~100 GB, hash-partitioned | Cold; read on attempt issue  |

Browse and search then run against ~2 GB with useful indexes. Tags at roughly five per problem is
50 M join rows, ordinary for PostgreSQL. Full-text and trigram search over 10 M rows with a GIN index
is fine; faceted search with live counts is the documented replacement point, and immutable versions
are what make adding a search index safe later.

### 1,000 instructors

Small, as reviewer 3 says -- but only because tenancy is logical. Database-per-instructor would mean
1,000 catalogs, 1,000-step migrations, and constant connection-pool eviction, each rebuild paying a
10-50 ms TLS handshake on a request path where that cost is pure overhead. One cluster with forced RLS
removes the tenant pool
registry, per-tenant migrations, and the RDS Proxy requirement entirely. This is the plan's largest
simplification and it came from reviewer 3.

Simultaneous students are the real number: 1,000 to 10,000 concurrent from 50,000 registered, met by
adding stateless replicas.

### WeBWorK is the likely bottleneck

Rendering and grading are CPU-heavy Perl at roughly 100-500 ms per render. Mitigations, in order of
leverage: deterministic render caching by `(version_id, seed)` turns repeat renders into CDN hits;
question prefetch hides first-render latency; a worker pool autoscales on queue depth, grading
latency, CPU, and timeout rate. Submitted answers are still graded server-side regardless of caching.

### Honest replacement thresholds

| Component                          | Holds until                                              | Then                                                              |
| ---------------------------------- | -------------------------------------------------------- | ----------------------------------------------------------------- |
| `FOR UPDATE SKIP LOCKED` job queue | ~5,000 jobs / min                                        | Move to SQS                                                       |
| PostgreSQL faceted search          | ~10 M problems with live counts                          | Add a search index fed from immutable versions                    |
| Single writer                      | ~2,000 writes / s sustained                              | Read replicas for catalog and reporting first, then shard tenants |
| One cluster, logical tenancy       | Regulatory or contractual demand for physical separation | Split by tenant ID, which every record already carries            |

### Deployment at each end

|            | Start                          | Target                                 |
| ---------- | ------------------------------ | -------------------------------------- |
| `api`      | 1-2 Fargate tasks              | Autoscaled on request count            |
| `worker`   | 1 task                         | Autoscaled on queue depth              |
| `renderer` | 1 task                         | Autoscaled pool                        |
| Database   | One modest RDS instance        | Larger primary plus read replicas      |
| Objects    | One MinIO, then one bucket set | CDN-backed, lifecycle rules            |
| Search     | PostgreSQL full-text           | Dedicated index if faceting demands it |

## Browser interface design

The student-facing surface is where the platform is judged, so it gets the same treatment as the
domain. Two repo documents already govern it and are treated as requirements rather than suggestions:
[PLAYFUL_TRAINING_GAME_STYLE.md](../PLAYFUL_TRAINING_GAME_STYLE.md)
targets learners aged 15-30 building a real skill, which is exactly this audience, and
[COLOR_CONTRAST_ACCESSIBILITY.md](../COLOR_CONTRAST_ACCESSIBILITY.md)
governs palette contrast.

### Route map

| Route                                                          | Surface                                              | Notes                                                 |
| -------------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------- |
| `/`                                                            | Course list for the signed-in role                   | Student and instructor views diverge below this       |
| `/courses/:courseId`                                           | Assignment list with progress and run counts         | Reads summary rows                                    |
| `/courses/:courseId/assignments/:assignmentId`                 | Assignment overview, run history, start or resume    | Entry point for a new run                             |
| `/runs/:runId`                                                 | The attempt loop, one question at a time             | The screen that must feel instant                     |
| `/runs/:runId/summary`                                         | Run result, per-question outcomes, start another run | Where practice re-entry lives                         |
| `/library`                                                     | Problem browser over the shared catalog              | Virtualized, faceted, cursor-paged                    |
| `/library/:problemId/versions/:versionId`                      | Problem detail, statistics, version chain            | Shows anonymous statistics and `derived_from` lineage |
| `/workspace`                                                   | Instructor drafts                                    | Private, pre-publication                              |
| `/workspace/:workspaceId`                                      | Draft editor with validation and preview             | Preview renders through WASM generation               |
| `/instructor/courses/:courseId/assignments/:assignmentId/edit` | Assignment editor with policy configuration          | Capability gating surfaces inline                     |
| `/instructor/courses/:courseId/gradebook`                      | Gradebook                                            | Reads summary rows only                               |

### Reactivity contract

`docs/SOLID_MODEL.md` records this and is the file a reviewer checks a component against. Solid's
fine-grained model is the reason a timer ticking four times a second costs one text-node update rather
than a component re-render.

| State                               | Primitive                                | Owner               | Rationale                                                                                        |
| ----------------------------------- | ---------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------ |
| Session and role                    | Context over a store                     | App shell           | Read everywhere, written rarely                                                                  |
| Current run and per-question status | Store with granular reads                | Run route           | Nested and partially updated; a store avoids replacing the whole object on one question's change |
| Remaining time                      | Signal holding an integer of deciseconds | Timer component     | Scalar, high-frequency, one subscriber                                                           |
| Submission in flight                | Signal holding a discriminated union     | Attempt component   | `idle`, `validating`, `submitted`, `graded`, `failed`                                            |
| Question content                    | Resource keyed on question attempt id    | Question component  | Async, suspendable, cache-friendly                                                               |
| Prefetched next question            | Store keyed by question index            | Prefetch controller | Written by prefetch, read by navigation                                                          |
| Catalog browse results              | Resource plus cursor signal              | Library route       | Cursor pagination, never an offset                                                               |

Conventions the review checklist enforces positively: read props at the use site so reactivity is
preserved; place teardown in `onCleanup`; render dynamic lists with `<For>` when identity matters and
`<Index>` when position matters; derive values with `createMemo` rather than writing them from an
effect.

### Question rendering and sanitization

Rendering a backend-neutral question is the most security-sensitive part of the frontend, because two
adapters return markup produced elsewhere.

The pipeline: the API returns a **render envelope** holding prompt blocks, a response definition, and
asset references. The renderer maps each block to a component, and each response definition to a
response widget. Two block kinds carry supplied markup -- WeBWorK rendered HTML and QTI converted
content -- and both pass through a **server-side allowlist sanitizer** before ever reaching the
envelope. Sanitization happens on the server, in the worker at render time, so the sanitized form is
what gets cached and what every client receives; the browser trusts the envelope because the server
already validated it.

The allowlist covers structural markup, math, tables, and images whose `src` resolves to an internal
asset ID. Script, style, event-handler attributes, iframes, and external URLs are dropped at
sanitization time and the drop is recorded on the render record, so an adapter producing unexpected
markup is visible rather than silent.

Response widgets, one per response type in `question_model`, are the reusable core of the student UI:
numeric entry with unit display, formula entry with live format validation, single and multiple
selection, ordering, matching, short text, and file upload. Each widget calls the WASM
format-validator on input and shows a local, immediate hint when the shape is wrong -- the one place
the browser gives a real-time answer-adjacent response, and it is safe because format validity carries
no information about correctness.

### Feedback disclosure is a fifth policy

`docs/PLAYFUL_TRAINING_GAME_STYLE.md` makes the wrong-answer screen the highest-value screen in the
product and requires three parts in order: what the learner chose, the correct answer, and one sentence
of why. That is pedagogically right for mastery and practice, and wrong for a quiz or exam where
revealing the answer defeats the assessment.

So feedback disclosure joins the activity model's policy set as a fifth independent policy:

| Disclosure              | Shows                                         | Default for         |
| ----------------------- | --------------------------------------------- | ------------------- |
| `immediate_full`        | Chose, correct answer, why                    | Practice, mastery   |
| `immediate_correctness` | Correct or not, with a hint, no answer        | Quiz with retries   |
| `deferred`              | Nothing until the run is submitted            | Quiz single-attempt |
| `on_release`            | Nothing until the instructor releases results | Exam                |

The disclosure policy is evaluated on the server, and the response envelope carries only what the
policy permits. A client asking for more receives no more, which keeps the answer-secrecy guarantee
independent of UI correctness.

### Timer design

The browser timer is display; the server owns the verdict. A signal decrements from the server-supplied
expiry, and the component reconciles against the server's remaining-time value on every response so
drift self-corrects rather than accumulating. At expiry the client submits whatever exists and the
server rules on whether it arrived in time. A clock moved forward on the client shortens only that
student's own display, and the server's verdict is unaffected -- verified by a test.

Presentation follows the training-game guidance: the timer is legible at a glance, calm rather than
alarming, and it never becomes the loudest element on screen. A student who runs out of time sees a
teaching screen, not a failure screen.

### Prefetch and perceived latency

Perceived speed comes from three mechanisms, in order of contribution:

1. **Next-question prefetch.** While a student works on question N, the client requests question N+1's
   envelope and warms its assets. Navigation after a graded answer is then a store read.
2. **Local format validation.** Malformed input is caught in WASM with no request.
3. **Explicit pending state.** The submit button enters a `submitted` state immediately with the
   student's answer echoed back, so the round trip is visible progress rather than a frozen UI. No
   correctness is implied or guessed before the server answers.

Next-question prefetch uses a durable, key-free reservation rather than creating an early attempt.
The reservation binds the current unresolved attempt, the first unattempted assignment position, the
server-owned seed, parameter hash, and complete backend provenance. Submitting question N promotes
that reservation into the one real N+1 attempt and timer, then records an immutable successor link in
N's idempotent receipt. A replica can heal a committed-but-unlinked successor from the sole pending
receipt, but it never scans newer run state to rewrite a replay.

The browser keeps the prefetched envelope only in memory. It advances without another run-screen fetch
only when the receipt's minimal `nextIssued` descriptor exactly matches predecessor, run, position,
version, seed, and backend-owned rendered hash. Mismatch, outage, teardown, or a late response clears
the speculative value and uses the ordinary authoritative screen path. Asset warming is limited to 12
deduplicated same-origin logical asset routes.

### Failure states

An assessment tool is judged on what happens when the network drops mid-question, so these are
designed rather than discovered:

| Situation                      | Behavior                                                                                                                            |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| Submission request fails       | Retry with the same idempotency key, backing off; the answer stays visible and editable; the timer keeps its server-anchored expiry |
| Repeated failure               | A persistent banner states the answer is saved locally and will be submitted, with a manual retry control                           |
| Session expires mid-run        | Re-authentication returns the student to the same question with the run intact                                                      |
| Question content fails to load | The question shows a retry affordance and the run remains resumable                                                                 |
| Renderer unavailable           | Only WeBWorK-sourced questions show a degraded state; the rest of the run proceeds                                                  |

### Accessibility

An assessment platform carries institutional accessibility obligations, so this is a gate rather than
a polish pass. `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md` owns the complete student interaction
contract. Every response widget has a visible platform path: Tab and Shift+Tab move focus, Space
selects choices and activates buttons, and native links retain Enter activation. The primary
acceptance journey reaches the explicit Submit answer button without requiring an Arrow key, digit,
response-input Enter, or Escape. Enter-to-submit, Arrows, visible-choice digits 1-9, and Escape are
widget extensions with separate scenarios so their failures do not hide a platform-path regression.
Every widget carries a programmatic label and
announces its validation state through a live region, so a screen-reader user learns that an entry is
malformed at the same moment a sighted user sees it. Timers announce at meaningful intervals rather
than on every tick. Touch targets meet the 56 px floor, which also serves benchtop tablet use. Contrast
is verified against `docs/PALETTE_CONTRAST_AUDIT.md` with measured values, and color never carries
meaning alone: correct and incorrect states pair their color with an icon and text. MATCH, FIB,
MULTI-FIB, and HOTSPOT must pass both their platform path and separately scoped extension behavior
before WP-RC5 acceptance.

### Client architecture

| Concern               | Choice                                                                                                                                                         |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Framework shape       | Solid SPA, static bundle, one Rust backend                                                                                                                     |
| Router                | `@solidjs/router`                                                                                                                                              |
| Query and cache layer | `query` plus `createAsync` from `@solidjs/router`, already present with the router; keyed on attempt, run, and cursor so revalidation is explicit              |
| API access            | The generated typed client only; every call goes through it so the boundary stays one file deep                                                                |
| WASM loading          | One module instantiated once at app start, awaited behind a splash state, shared by every consumer                                                             |
| WASM fallback         | When instantiation fails, format validation falls back to a server call and the app continues with a round trip per validation, reporting the degradation once |
| Server authority      | The server owns grading, timer verdicts, completion, and grade state; the client owns navigation, display, and input buffering                                 |

Browser persistence is deliberately narrow, since anything stored is data at rest on a shared machine:

| Store            | Contents                                                                            | Cleared                                      |
| ---------------- | ----------------------------------------------------------------------------------- | -------------------------------------------- |
| `localStorage`   | UI preferences only: theme, sound, reduced motion                                   | On explicit reset                            |
| `sessionStorage` | In-progress response text keyed by question attempt, for crash and refresh recovery | On successful submit, run exit, and sign-out |
| Memory only      | One exact key-free next-question envelope and its receipt-binding descriptor        | On advance, mismatch, run exit, or unmount   |
| Nothing          | Session tokens, keys, grades, and any answer-bearing value                          | n/a                                          |

Session identity lives in an `HttpOnly` cookie the page cannot read, which is what keeps it out of the
table above.

### Frontend security rules

- **Answer-bearing types stay out of the generated client.** Type generation runs over
  `crates/question_model` only, and `crates/grading` is never a generation input. A test asserts the
  generated surface contains no answer-key type, mirroring the WASM export allowlist so both halves of
  the secrecy boundary are checked the same way.
- **Supplied markup is sanitized server-side** before it enters a render envelope, so the sanitized
  form is what gets cached and delivered.
- **Content Security Policy** ships with the app: script sources limited to the bundle's own origin,
  `wasm-unsafe-eval` present because WebAssembly instantiation requires it, `object-src` empty, and
  frame ancestors limited to the LMS origins configured for LTI launch. The esbuild bundle contains no
  inline script, so no inline allowance is needed.
- **Asset URLs** are internal asset IDs resolved through the client, so a bucket URL never appears in
  markup.
- **Logging** carries identifiers and error codes; response text, grades, and student names stay out of
  the browser console and any telemetry payload.

### Forms, errors, and focus

- Response widgets are controlled inputs with validation state as data, so a widget renders its own
  error text and a run-level summary can list every outstanding issue from the same source.
- An error boundary wraps each route and, separately, the question renderer, so a failure in one
  question's content leaves the run shell and timer intact and offers a retry.
- Focus moves deliberately between attempt phases: to the feedback panel when a result arrives, so a
  screen-reader user hears the teaching content immediately, then to the advance control once feedback
  has been announced. Focus returns to the first response widget on the next question.
- Every asset carries required accessibility text on its object record, and the renderer surfaces it as
  alt text or an extended description. Math renders as MathML with a text alternative; structure and
  sequence figures require a description before a problem version may be published, which is a
  validation rule rather than an author's good intention.

### Frontend validation strategy

| Layer                    | Covers                                                                                                                                                                 |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Node component tests     | Response widget behavior, validation state transitions, envelope-to-component mapping                                                                                  |
| Playwright functional    | Mastery loop, post-completion practice run, give-up flow, resume after refresh, retry after a failed submission, timer expiry, publish refusal                         |
| Playwright accessibility | Keyboard-only completion of a full run, focus order across attempt phases, live-region announcements, contrast measured against `docs/COLOR_CONTRAST_ACCESSIBILITY.md` |
| Playwright network       | Offline submit and recovery, slow renderer, expired session mid-run, WASM instantiation failure falling back to server validation                                      |
| Playwright visual        | Feedback panel states and timer states, where a rendering regression is easier to see than to assert                                                                   |
| Interaction latency      | Measured and recorded per interaction as a baseline, compared for regression rather than against a chosen number                                                       |

### Instructor surfaces

The instructor side is the larger build and its hard problem is scale, not styling.

**Problem browser over ten million rows.** A virtualized list backed by cursor-paged queries, with
facets over taxonomy, capability, license, and statistics. Facet counts come from the catalog's own
aggregates so the UI never triggers a full scan. Search is a single input over full-text and trigram
matching, and the component boundary keeps the query behind a repository call so a dedicated search
service can replace it without a UI change.

**Assignment editor.** Question selection uses exact immutable versions from the catalog; a workspace
draft must be published before it can be selected. The editor exposes the four assignment-level
`RunPolicies` controls with their current values visible. Timing and attempt policies remain immutable
properties of each published question rather than assignment overrides. The browser submits only the
ordered version references and policy choices. The server resolves every version through tenant-visible
catalog state, uses the persisted immutable capability declaration, and returns the complete deterministic
`validate_assignment_config` violation list. Capability failures render beside the affected selections so
the instructor sees every violation at once rather than one per submission.

**Draft editor and preview.** A draft renders through the same question components as the student view,
generating parameters in WASM so an author sees a real variant per seed without a server round trip.
The preview shows the student view and the answer-key view side by side, since an author needs both.

**Publish flow.** Validation results, the target scope, and a diff against the previous version when
one exists. Publishing states plainly that the version becomes immutable and shareable.

**Course appearance.** One focused instructor surface selects a closed, measured three-color biome
or habitat theme and optionally uploads one small centered banner. The theme is authoritative
tenant-owned course state and applies through one route scope to every learner and instructor page
inside that course; it never recolors global PLE pages, authored scientific content, or semantic
success/danger/correctness states. The banner appears only on the course entry page. Theme and
banner changes share one strong appearance revision so a stale instructor tab preserves its local
choices rather than overwriting another instructor. The exact palette, object, RLS, image, API,
accessibility, and atomic work-package contracts are frozen in
`docs/active_plans/decisions/course_appearance_plan.md`.

**Gradebook.** Reads summary rows only, showing best and latest scores, completed run count, and
last activity. A student's run history is a drill-down that loads on demand, so the default view stays
a summary query regardless of how many practice runs a class has accumulated.

## Module catalog

The unit of owned work. Every module has one owner, one contract, the contracts it consumes, an
executable reference/test implementation where isolation is useful, and one independent
verification. A reference implementation is behavior-bearing test infrastructure, not release
substitution for a required production path.

| ID               | Module                                                                   | Exposes                                                                       | Consumes                                                              | Reference/test implementation        | Independent verification                                                                                                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MOD-QM           | `question_model`                                                         | Types, capabilities, identity, taxonomy                                       | none                                                                  | n/a (root contract)                  | `cargo test`; `ts-rs` output compiles                                                                                                                                                                                               |
| MOD-ID           | Identity and lifecycle                                                   | Draft workspace identity, published `ProblemId`/`VersionId`, lifecycle        | MOD-QM                                                                | n/a                                  | Lifecycle tests; no published identity construction outside publish                                                                                                                                                                 |
| MOD-RUN          | Enrollment, run, attempt model and policies                              | Run lifecycle, four policy types                                              | MOD-QM                                                                | n/a                                  | 30-run scenario; policy combinations                                                                                                                                                                                                |
| MOD-STATE        | Attempt state machine                                                    | `apply(state, event)`, within-run completion                                  | MOD-QM, MOD-RUN                                                       | n/a                                  | Every legal transition plus a rejected illegal one                                                                                                                                                                                  |
| MOD-TIME         | Timing rules                                                             | `timer_verdict(...)` pure fn                                                  | MOD-QM                                                                | n/a                                  | Table-driven grace and pause cases                                                                                                                                                                                                  |
| MOD-SCORE        | Scoring and grade policies                                               | `score(...)`, summary projection                                              | MOD-QM, MOD-RUN                                                       | n/a                                  | First/latest/highest agree with a hand-computed fixture                                                                                                                                                                             |
| MOD-CAP          | Capability validation                                                    | `validate_assignment_config -> Vec<Violation>`                                | MOD-QM                                                                | n/a                                  | Committed violation table                                                                                                                                                                                                           |
| MOD-GEN          | Seeded generation                                                        | `generate(seed, spec)`                                                        | MOD-QM                                                                | n/a                                  | Seed-vector parity (WP-C4)                                                                                                                                                                                                          |
| MOD-GRD          | Grading (server-only)                                                    | `grade(question, response, key)` and typed flat private integrity             | MOD-QM, MOD-STATE                                                     | n/a                                  | Checker behavior tests; MOD-STO's opaque typed integrity use is server-only; absent from the `wasm32` closure (WP-C5)                                                                                                               |
| MOD-OBJ          | Object store                                                             | `ObjectStore` trait                                                           | MOD-ID                                                                | `MemoryObjectStore`                  | Conformance suite on memory, MinIO, S3                                                                                                                                                                                              |
| MOD-STO          | Persistence and RLS context                                              | `Store` trait                                                                 | MOD-QM, MOD-ID, MOD-RUN, MOD-GRD (opaque flat private integrity only) | `MemoryStore`                        | Conformance suite on memory and PostgreSQL; cursor pagination only; no private material enters Wasm                                                                                                                                 |
| MOD-SCHEMA       | Migrations, RLS policies, partitions                                     | Shared and tenant schema                                                      | MOD-ID, MOD-RUN                                                       | n/a                                  | Fresh apply; foreign tenant context returns zero rows                                                                                                                                                                               |
| MOD-ADP-NAT      | Native adapter                                                           | Algorithmic families and strict PLE flat-question compiler                    | MOD-QM, MOD-GEN, MOD-GRD                                              | n/a                                  | End-to-end generated family; flat JSON public/private split and reproducible hash                                                                                                                                                   |
| MOD-ADP-WW       | WeBWorK adapter                                                          | Adapter impl, renderer client, render cache                                   | MOD-QM, MOD-OBJ                                                       | Recorded renderer fixtures           | Exact immutable authored `which_hydrophobic-simple.pgml` RadioButtons fixture renders and grades; repeat seed cache hit; private topology, timeout, PLE API, and browser gates pass; broad OPL corpus compatibility is out of scope |
| MOD-ADP-QTI      | QTI adapter                                                              | Import pipeline, export                                                       | MOD-QM, MOD-OBJ                                                       | `MemoryObjectStore`                  | Hostile-ZIP corpus rejected; unsupported features recorded                                                                                                                                                                          |
| MOD-ADP-H5P      | H5P adapter                                                              | Adapter impl, `serverGrading: false`                                          | MOD-QM                                                                | n/a                                  | Capability honesty test; import path to internal model                                                                                                                                                                              |
| MOD-ADP-IMATHAS  | iMathAS adapter                                                          | Immutable snapshot, verified remote broker, render cache, capabilities        | MOD-QM, MOD-OBJ, MOD-STO, MOD-API-RUN                                 | Recorded, redacted provider fixtures | Pinned seeded item renders and grades; replay, cache, outage, disclosure, and isolation gates                                                                                                                                       |
| MOD-EXPORT       | Print model and writers                                                  | DOCX and PDF                                                                  | MOD-QM                                                                | Fixture version                      | Four artifacts; unexportable flagged pre-build                                                                                                                                                                                      |
| MOD-WASM         | WASM bridge                                                              | Typed exports                                                                 | MOD-QM, MOD-STATE, MOD-TIME, MOD-GEN, MOD-CAP                         | n/a                                  | Export allowlist; no `grading` in closure                                                                                                                                                                                           |
| MOD-API-AUTH     | Auth and sessions                                                        | `/auth`                                                                       | MOD-STO                                                               | `MemoryStore`                        | Login on one replica, proceed on another                                                                                                                                                                                            |
| MOD-API-CAT      | Catalog routes                                                           | `/problems`, taxonomy, publish                                                | MOD-STO, MOD-ID, MOD-CAP                                              | `MemoryStore`                        | Publish refuses on violations; drafts hold no `problem_id`; cursor paging                                                                                                                                                           |
| MOD-API-COURSE   | Course routes                                                            | `/courses`, `/assignments`                                                    | MOD-STO                                                               | `MemoryStore`                        | Assignments store `(problem_id, version_id)`                                                                                                                                                                                        |
| MOD-API-RUN      | Run and attempt routes                                                   | `/runs`, `/attempts`, `/submissions`, `/grading`                              | MOD-STO, MOD-RUN, MOD-STATE, MOD-TIME, MOD-GRD                        | `MemoryStore`                        | DB timestamps; idempotent replay; summary updated transactionally; no key in any response                                                                                                                                           |
| MOD-API-ASSET    | Asset delivery                                                           | `/assets/{id}`                                                                | MOD-OBJ, MOD-STO                                                      | `MemoryObjectStore`                  | Authorizes, logs, short-lived URL; public assets bypass to CDN                                                                                                                                                                      |
| MOD-WORKER       | Jobs queue and worker pool                                               | Enqueue and drain                                                             | MOD-STO                                                               | `MemoryStore`                        | Two workers never claim one job; scales on queue depth                                                                                                                                                                              |
| MOD-STATS        | Anonymous question statistics                                            | Incremental aggregation, k-anonymity gate                                     | MOD-RUN, MOD-STO                                                      | `MemoryStore`                        | Aggregates match a hand-computed fixture; below-threshold cohorts suppressed; aggregates survive record deletion                                                                                                                    |
| MOD-RETENTION    | Retention lifecycle                                                      | Scheduled notify, archive, delete; per-institution config                     | MOD-STO, MOD-OBJ, MOD-STATS                                           | `MemoryStore`                        | 30/100/365-day stages fire; deletion removes records and bucket artifacts and leaves catalog content and statistics intact                                                                                                          |
| MOD-CLIENT       | Typed API client                                                         | TS client from generated types                                                | Generated types                                                       | Mock handler set                     | Type tests; no `any`, no unchecked `as`                                                                                                                                                                                             |
| MOD-UI-SHELL     | App shell, routing, session context, error boundaries, focus conventions | Route tree, boundaries, layout                                                | MOD-CLIENT, WP-C9                                                     | Mock handlers                        | Every route resolves; a thrown render error leaves the shell usable                                                                                                                                                                 |
| MOD-UI-COURSE    | Course shell and appearance settings                                     | Course-scoped three-color theme, entry banner, instructor appearance workflow | MOD-UI-SHELL, MOD-CLIENT, MOD-API-COURSE, MOD-OBJ                     | Appearance mock repository           | Theme follows all course routes without global bleed; keyboard save/conflict flow; contrast and visual artifact gates                                                                                                               |
| MOD-UI-WIDGETS   | Response widget set                                                      | One component per response type, with local format validation                 | MOD-WASM, WP-C9                                                       | Reference widget                     | Each widget satisfies `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`, is label-announced, and flags invalid shape without issuing a request                                                                                              |
| MOD-UI-RENDER    | Question renderer                                                        | Envelope-to-component mapping, asset resolution, math and figure alternatives | MOD-UI-WIDGETS                                                        | Fixture envelopes                    | Every block kind renders; a sanitized-markup fixture renders without script execution; missing accessibility text surfaces as an authoring error                                                                                    |
| MOD-UI-ATTEMPT   | Attempt loop                                                             | Submit, pending state, feedback disclosure, timer, prefetch, retry            | MOD-UI-RENDER, MOD-CLIENT                                             | Mock handlers                        | Full mastery run; 31st run; timer expiry; offline submit recovers; disclosure policy respected per mode                                                                                                                             |
| MOD-UI-BROWSE    | Catalog browser                                                          | Virtualized cursor-paged list, facets, problem detail                         | MOD-CLIENT                                                            | Mock handlers                        | Ten-thousand-row synthetic list scrolls without a full fetch; facet counts come from aggregates                                                                                                                                     |
| MOD-UI-EDITOR    | Draft and assignment editors                                             | Draft editing, WASM preview, policy controls, capability gating, publish flow | MOD-UI-RENDER, MOD-WASM                                               | Mock handlers                        | Preview generates a real variant per seed offline; a policy a backend cannot support marks the question and names the capability; publish shows the version diff                                                                    |
| MOD-UI-GRADEBOOK | Gradebook                                                                | Summary-row views, run-history drill-down                                     | MOD-CLIENT                                                            | Mock handlers                        | Default view issues one summary query regardless of run count                                                                                                                                                                       |
| MOD-LTI          | LTI Advantage                                                            | Launch and grade passback                                                     | MOD-STO, MOD-API-AUTH                                                 | Sandbox fixtures                     | Passback verified against an LMS sandbox                                                                                                                                                                                            |
| MOD-DEPLOY       | Containers and AWS                                                       | Compose, images, Fargate, RDS, buckets, CDN                                   | all                                                                   | n/a                                  | Burst load test scales out with no failed submissions                                                                                                                                                                               |

Shared artifacts with exactly one owning module, so lanes never contend:

| Artifact                                   | Owner      |
| ------------------------------------------ | ---------- |
| `crates/domain/tests/seed_vectors.json`    | MOD-GEN    |
| `tests/fixtures/published_problem/` corpus | MOD-QM     |
| `schemas/migrations/**`                    | MOD-SCHEMA |
| WASM export allowlist                      | MOD-WASM   |
| Mock API handler set                       | MOD-CLIENT |
| `containers/compose.yaml`                  | MOD-DEPLOY |

## Milestone plan

| M   | Title                    | Summary                                                               | Goal                                                  |
| --- | ------------------------ | --------------------------------------------------------------------- | ----------------------------------------------------- |
| M0  | Foundation and toolchain | Workspace, Solid build, containers, gates                             | Both toolchains green on a hello-path                 |
| M1  | Contract freeze          | Every contract, reference backend, and conformance suite              | Six or more lanes start without coordinating          |
| M2  | Core lanes               | Domain, runs, grading boundary, storage, objects, native adapter, API | Parity, secrecy, and tenant-isolation gates green     |
| M3  | Experience lanes         | Student and instructor UIs, worker pool, export                       | Latency baseline recorded; a 31st run works           |
| M4  | Adapter lanes            | WeBWorK with render cache, QTI, H5P                                   | Three adapters live with zero diff in `crates/domain` |
| M5  | Integration hardening    | Cross-cutting E2E, isolation, hostile inputs, retention               | Every gate green together, not just per lane          |
| M6  | Platform and deploy      | LTI, analytics, AWS, autoscaling                                      | Passback verified; burst load scales out              |

### Milestone: M0 foundation and toolchain

- Depends on: none.
- Deliverables: Cargo workspace with every crate present and compiling; Solid app rendering one route;
  `pipeline/build.mjs` producing `dist/main.js` plus the `.wasm` asset; compose bringing up `api`,
  `postgres`, and `minio`; template defects fixed; `check_codebase.sh` extended with Rust gates.
- Entry criteria: none.
- Exit criteria: `./check_codebase.sh`, `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo test`, and `pytest tests/` green; `podman compose up` yields `/health` 200 backed by a real
  `SELECT 1` and a MinIO bucket probe.
- Parallel-plan ready: no. Bootstrapping is serial; the workspace must compile before any lane starts.

### Milestone: M1 contract freeze

- Depends on: M0.
- Deliverables: MOD-QM types with generated TypeScript; MOD-ID identity; MOD-RUN run and policy model;
  trait signatures for MOD-OBJ, MOD-STO, and the adapter boundary; `MemoryStore` and
  `MemoryObjectStore`; both conformance suites; the fixture corpus; the mock handler set; the WASM
  export allowlist; the frontend architecture contract with `docs/SOLID_MODEL.md`,
  `docs/FRONTEND_ARCHITECTURE.md`, and one reference response widget; `docs/CONTRACTS.md`.
- Entry criteria: M0 exit criteria met.
- Exit criteria: every catalog module compiles against its stated contracts and executable reference
  backends with no missing module; conformance suites pass against in-memory backends; generated TypeScript passes
  `tsc --noEmit`, ESLint, and Prettier unchanged and contains no answer-bearing type; a UI lane builds a
  screen against the WASM facade, the generated client, and the mock handlers with no backend running; a
  reviewer walks every catalog row and confirms no contract gap.
- Parallel-plan ready: partly. MOD-QM is serial and first; MOD-ID, MOD-RUN, the reference-backend
  packages, and the frontend contract then run as lanes.

The frontend contract lands here rather than in M3 for the same reason the backend contracts do: the
two UI lanes are only independent once the route map, state model, facade signatures, and one reference
widget exist to build against.

### Milestone: M2 core lanes

- Depends on: M1.
- Deliverables: domain modules, run and scoring model, grading boundary with both gates, real object
  storage, PostgreSQL store with RLS and partitions, native adapter including the PLE flat-question
  compiler and split persistence path, API route groups.
- Lanes: (1) MOD-STATE, MOD-TIME, MOD-SCORE, MOD-CAP; (2) MOD-GEN, MOD-ADP-NAT; (3) MOD-GRD,
  MOD-WASM; (4) MOD-OBJ; (5) MOD-SCHEMA, MOD-STO; (6) the five API modules; (7) MOD-CLIENT.
- Entry criteria: M1 exit criteria met.
- Exit criteria: seed parity green on both targets; WASM allowlist and dependency assertion green;
  conformance suites green against PostgreSQL and MinIO; a foreign tenant context returns zero rows;
  the student-facing role cannot read any answer-key table; an in-progress run resumes across restart
  and across replicas; a replayed submission returns the first result; every list endpoint uses a
  cursor; flat JSON publication preserves the non-signable canonical source and binds grader-only
  material to the exact answer-free public model.
- Parallel-plan ready: yes. Seven lanes.

### Milestone: M3 experience lanes

- Depends on: M2 for live behavior; UI lanes may start after M1 against mocks.
- Deliverables: app shell and routing; the response widget set; the question renderer; the attempt loop
  with prefetch, timer, and feedback disclosure; catalog browser; draft and assignment editors;
  gradebook; course appearance theme/banner capability and instructor settings; worker pool and jobs
  queue; print model with DOCX and PDF writers.
- Lanes: (1) MOD-UI-SHELL, then MOD-UI-WIDGETS and MOD-UI-RENDER; (2) MOD-UI-ATTEMPT;
  (3) MOD-UI-BROWSE, MOD-UI-EDITOR, MOD-UI-GRADEBOOK; (4) MOD-UI-COURSE after its closed
  object/Store/API contracts; (5) MOD-WORKER; (6) MOD-EXPORT.
- Entry criteria: M1 exit, including the frontend architecture contract, for mock-backed UI work;
  M2 exit for live integration.
- Exit criteria: server-side grading processing time recorded at p50, p95, and p99 over 500 synthetic
  submissions, with the numbers written to the tracker as the baseline later runs compare against;
  end-to-end round trip recorded alongside it for context; a browser network trace confirmed free of
  any answer or key; answer-format validation confirmed to resolve locally with no request issued; a
  student completes an assignment and starts a 31st run with fresh variants and a correct summary row;
  publish refusal names the question and capability; a draft carries no catalog number; two workers
  each claim distinct jobs; four export artifacts produced from one fixture; one instructor changes
  a course theme/banner under CAS and a student sees the theme across every course route without a
  banner outside the entry page or palette bleed into global pages.
- Parallel-plan ready: yes. Six lanes. The course appearance package supplies its own two-owner
  parallel limits and one-owner integration seams.

### Milestone: M4 adapter lanes

- Depends on: M1 for the adapter contract; M2 for MOD-OBJ and MOD-STO; the atomic
  draft-identity contract amendment below before its lane starts.
- Deliverables: WeBWorK adapter, renderer container, and deterministic render cache; QTI import
  pipeline and export; H5P adapter with honest capabilities and an import path; iMathAS adapter
  (with MyOpenMath as a provider) using immutable source snapshots, an honest capability profile,
  and a verified server-to-server grade broker.
- Lanes: (1) MOD-ADP-WW; (2) MOD-ADP-QTI; (3) MOD-ADP-H5P; (4) MOD-ADP-IMATHAS after the
  draft-identity contract amendment. These lanes do not modify `crates/domain`.
- Entry criteria: M2 exit criteria met.
- Exit criteria: the exact immutable licensed authored
  `content/pilot/webwork/which_hydrophobic-simple.pgml` RadioButtons fixture renders and grades
  through the shared model; a repeat `(version_id, seed)` is served from cache without touching the
  renderer; the renderer has no public endpoint, no PLE database access, enforced CPU, memory, and
  request-time limits, no SQL database or persistent renderer volume; its timeout degrades only
  WeBWorK questions; PLE
  API and browser-network gates prove no protected material crosses the boundary. Broad OPL corpus
  compatibility is outside this bounded fixture acceptance. The hostile-ZIP corpus is fully rejected
  with actionable errors; unsupported QTI features are recorded rather than dropped; the original
  package is re-importable; H5P declares `serverGrading: false`; an iMathAS sandbox preview remains
  unversioned and private, while publication archives a checksum-pinned snapshot and profile before
  minting a durable version; iMathAS grades only through an authenticated, idempotent
  server-to-server exchange; a browser message is presentation/readiness only; and an iMathAS
  outage affects only that attempt. **Zero diff inside `crates/domain` across all adapters** is the
  real test of the boundary.
- Parallel-plan ready: yes. Three independent lanes immediately; iMathAS begins after its atomic
  draft-identity contract amendment and then proceeds independently.

#### Draft identity prerequisite and iMathAS lane

The persisted backend wire value is `imathas`; the product label is **iMathAS**. MyOpenMath is an
iMathAS provider, not a second backend. Before MOD-ADP-IMATHAS begins, land the following atomic
draft-identity refactor. It is a prerequisite for every adapter, not an iMathAS-lane repair, and no
adapter may consume an intermediate form:

- MOD-QM defines a tenant-owned `DraftQuestionDefinition` with workspace-only identity and no
  `ProblemId` or `VersionId`; published-only definitions and references require both IDs.
- MOD-ID makes the lifecycle transition from validated draft to published content mint the full
  `ProblemVersionRef` only after all publication validation succeeds. A revision retains its
  published `ProblemId` and mints a new `VersionId`; a fork mints both. A failed publication mints
  neither ID.
- MOD-STO and MOD-SCHEMA update the memory and PostgreSQL stores, migration and JSON payload
  boundaries so drafts store only their workspace identity and published rows store the immutable
  reference. Catalog and API publication paths use that transition rather than a draft-held version.
- MOD-CLIENT updates generated TypeScript and all direct browser/API consumers. MOD-QM regenerates
  the generated clients and published/draft fixtures through their owners, and conformance tests
  prove a sandbox draft is private and unversioned until successful publication.

This same patch must update the lifecycle, store conformance, catalog/API fixtures, generated clients,
and published-problem corpus; it follows the frozen-contract change rule and blocks
MOD-ADP-IMATHAS until complete. The adapter continues to make **zero diff inside `crates/domain`**.

After that prerequisite:

- Add `QuestionSource::Imathas` and `QuestionBackend::Imathas`. A draft sandbox preview may retain
  only an unversioned, private provider/question reference. Publishing first fetches and validates a
  supported profile, then stores immutable source bytes with an `ObjectId` and SHA-256 and pins the
  integration profile. The prerequisite publication transition mints IDs only after that work
  succeeds; any source or profile change publishes a new version.
- Keep provider endpoint, credentials, accepted origin, and egress policy deployment configuration
  keyed by an opaque provider name. Never persist or serialize an arbitrary iframe URL, launch JWT,
  answer, solution, remote session state, or credential as question source.
- Keep `ExternalToolLaunch` separate from serializable question source. Its `launchUrl` is a
  non-secret, same-origin, session-authenticated attempt route or handle, never an opaque provider
  launch capability. It contains no provider bearer, token, credential, source locator, or
  correctness material, and none may enter URLs, browser history, logs, traces, or serializable
  source. The server keeps the provider launch and correlation server-held or in a server-only
  HttpOnly session, and rechecks enrollment and attempt ownership on every route use. Browser
  messages may communicate validated presentation/readiness state only; they never grade.
- The server alone launches and grades: it checks attempt/enrollment ownership, holds correlation and
  idempotency keys, verifies provider authentication, expiry, nonce, and attempt/version/seed
  correlation, then persists the first verified grade. A browser score or callback is never a
  fallback. Unsupported verification or deterministic seeding makes the requested graded feature
  unavailable rather than pretending capability.
- Source snapshots are answer-bearing objects, not CDN assets. Student launches, responses, and
  provider transcripts are tenant-owned records under RLS and retention; immutable shared content
  stays outside tenant records. Cache keys include immutable version and seed, never tenant data.
- iMathAS failures show a question-local retry/degraded state with saved editable response. They do
  not become incorrect grades, block adjacent questions, or weaken disclosure policy. Solutions are
  requested, sanitized, and returned only after server policy permits them.

Permanent behavior gates use recorded, redacted protocol fixtures and a local mock provider: sandbox
publication refusal without a snapshot or durable IDs; immutable replay after provider mutation;
deterministic render cache; forged or stale browser/provider messages leaving attempts ungraded;
idempotent grade replay and tenant isolation; copied launch URLs and disclosure traces free of source,
provider launch material, secrets, answer, and unauthorized score; outage isolation; and capability
refusal for unsupported profiles. A dedicated
non-production iMathAS/MyOpenMath compatibility probe is a one-time release-readiness check, never a
permanent credentialed or network test.

### Milestone: M5 integration hardening

- Depends on: M3, M4.
- Deliverables: cross-cutting `tests/e2e/` suite; orphaned-object reconciliation; MOD-STATS
  incremental aggregation with the k-anonymity gate; MOD-RETENTION lifecycle with per-institution
  configuration; asynchronous analytics; `docs/SECURITY_MODEL.md`, `docs/RETENTION_POLICY.md`.
- Lanes: (1) MOD-STATS; (2) MOD-RETENTION; (3) cross-cutting E2E, owned by `integrator`.
- Entry criteria: M3 and M4 exit criteria met.
- Exit criteria: every gate green in one run; clock-skew invariance, tenant isolation, answer-key
  grants, object round trip, partition pruning on a large synthetic attempt table, and
  renderer-outage degradation all proven together; a course deletion test proves student records and
  `student-records` bucket artifacts are gone while catalog content, instructor drafts, and anonymous
  statistics remain; a below-threshold cohort's statistics are proven suppressed.
- Parallel-plan ready: no. This milestone exists to find interactions that per-lane green results
  hide.

### Milestone: M6 platform and deploy

- Depends on: M5.
- Deliverables: LTI Advantage passback; server-side aggregate views reading summaries and anonymous
  statistics with no client analytics; OpenTofu AWS
  deployment (Fargate target tracking, RDS PostgreSQL, three buckets, ALB, CloudFront, Secrets
  Manager, CloudWatch); backup and retention policy; burst load test; FERPA control checklist with
  evidence.
- Lanes: (1) MOD-LTI; (2) aggregate observability; (3) MOD-DEPLOY.
- Entry criteria: M5 exit criteria met.
- Exit criteria: passback verified against an LMS sandbox; encryption at rest and in transit
  demonstrated; restore-from-backup rehearsed and timed; a synthetic class-start burst triggers
  scale-out with no failed submissions, replica count and p99 recorded.
- Parallel-plan ready: yes. Three lanes.

## Work packages

M0 and M1 remain below as accepted bootstrap history. The complete remaining M2-M6 closure packages,
including owners, files, behavior, success conditions, and validation, are WP-RC1 through WP-RC12 in
`docs/active_plans/active/release_completion_plan.md`; no milestone-entry expansion
or implementer-authored specification is required.

### M0 packages

| ID    | Title                                 | Owner          | Depends on   | Acceptance                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----- | ------------------------------------- | -------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WP-F1 | Create the Cargo workspace            | `expert_coder` | none         | Every crate in the boundary table exists and compiles empty; current edition; `Cargo.lock` committed; the forbidden-dependency column encoded as real absences, not comments                                                                                                                                                                                                                                                                                                              |
| WP-F2 | Add the WASM build path               | `expert_coder` | WP-F1        | `wasm-bindgen` output into a gitignored staging dir; a trivial export callable from Node; current stable toolchain and version-matched runner                                                                                                                                                                                                                                                                                                                                             |
| WP-F3 | Stand up Solid and the build pipeline | `expert_coder` | WP-F2        | `build_github_pages.sh` delegates to `node pipeline/build.mjs` with `esbuild-plugin-solid` and copies the `.wasm`; `tsconfig.json` gains `"jsx": "preserve"`, `"jsxImportSource": "solid-js"`, and an `exclude` for `OTHER_REPOS` and `target`; `src/log.ts` exists so no `console` appears in `src/`; placeholders filled matching `VERSION`; `clean` points at `devel/dist_clean.sh`                                                                                                    |
| WP-F4 | Containerize api, postgres, minio     | `expert_coder` | WP-F1        | `containers/Containerfile.api` builds a multi-stage slim image under `podman build`; `containers/compose.yaml` brings up current stable PostgreSQL and MinIO with named volumes and creates three buckets; `/health` returns 200 only after exact migration/checksum compatibility verification and a bucket probe; credentials arrive at run time from the environment; `docs/LOCAL_STACK_OPERATIONS.md` records the commands and `docs/MACOS_PODMAN.md` records the macOS machine setup |
| WP-F5 | Extend the check gate with Rust steps | `coder`        | WP-F1        | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` through the existing `step_run` helper; loud `SKIP` when `cargo` is absent, matching the script's honesty convention                                                                                                                                                                                                                                                                                         |
| WP-F6 | Foundation documentation              | `coder`        | WP-F3, WP-F4 | README first paragraph pure prose under 250 chars passing `tests/test_readme_first_paragraph.py`; `docs/CODE_ARCHITECTURE.md` carries the container, boundary, bucket, and crate tables; `pytest tests/` green                                                                                                                                                                                                                                                                            |

### M1 contract-freeze packages

#### Work package: WP-C1 define the question model and taxonomy

- Owner: `architect`. Module: MOD-QM. Depends on: WP-F1.
- Touch points: `crates/question_model/src/`, `docs/QUESTION_MODEL.md`.
- Acceptance criteria: covers the spec's `QuestionDefinition` fields; `BackendCapabilities` carries all
  eight flags; response and grading shapes are enums whose invalid combinations do not compile; tags,
  taxonomy, and licensing types included as shared-content data; **no answer-bearing type defined
  here**; every public item documented per `docs/RUST_STYLE.md` section 13; `ts-rs` derives on every
  boundary type; external IDs are UUIDv7 or random, never sequential.
- Evidence or review: `reviewer` confirms no capability is a bare `bool` that two call sites must
  re-check, per `docs/RUST_STYLE.md` section 9.
- Next dependency: WP-C2 and WP-C3 consume this accepted package.

#### Work package: WP-C2 define identity and lifecycle

- Owner: `architect`. Module: MOD-ID. Depends on: WP-C1.
- Touch points: `crates/question_model/src/identity.rs`, `docs/PROBLEM_IDENTITY.md`.
- Acceptance criteria: `WorkspaceId`, `ProblemId`, and `VersionId` are distinct branded types that
  cannot substitute for one another; the lifecycle is an enum with transitions through one fallible
  function; `ProblemId` is constructible only on the publish transition; the document states the
  draft-versus-published rule in one sentence a maintainer can apply.
- Next dependency: WP-C3 and WP-C4 consume this accepted package.

#### Work package: WP-C3 define the run, policy, and summary model

- Owner: `architect`. Module: MOD-RUN. Depends on: WP-C2.
- Touch points: `crates/question_model/src/activity.rs`, `crates/domain/src/run.rs`,
  `docs/ACTIVITY_MODEL.md`.
- Acceptance criteria: enrollment, run, and attempt as distinct types; the four policies
  (completion requirement, grade policy, continued practice, variation policy) are independent enums
  that compose freely; within-run completion is a derivation with no stored boolean; the summary
  projection is a pure function of a run transition so the store can apply it transactionally; a test
  drives 31 runs and asserts the summary matches a hand-computed expectation.
- Evidence or review: the 31-run test is the artifact a reviewer reads, because it encodes the owner's
  observed student behavior as a requirement.
- Next dependency: WP-C4 and WP-C5 consume this accepted package.

#### Work package: WP-C4 freeze the store and object contracts with reference backends

- Owner: `expert_coder`. Modules: MOD-STO, MOD-OBJ (contract portion). Depends on: WP-C3.
- Touch points: `crates/learning-data-access/src/{lib,in_memory}.rs`, `crates/objects/src/{lib,in_memory}.rs`, both
  conformance suites.
- Acceptance criteria: `Store` covers every entity, exposes cursor pagination only with no `OFFSET`
  parameter anywhere in the trait, and carries an explicit tenant context type that cannot be
  defaulted; `ObjectStore` exposes `put`, `get`, `delete`, `signed_url` with keys built only from IDs
  and versions and no caller-supplied key; checksums computed on write and verified on read; both
  memory backends pass conformance suites the PostgreSQL and S3 backends will later run unchanged; no
  SQL or AWS type leaks through either trait.
- Evidence or review: the conformance suites are the deliverable, because they are the contract every
  later lane is held to.
- Next dependency: WP-C5 and M2 lanes 4 and 5 consume this accepted package.

#### Work package: WP-C5 commit the seed vector table and parity harness

- Owner: `tester`. Module: MOD-GEN (verification). Depends on: WP-C1.
- Touch points: `crates/domain/tests/seed_vectors.json`, `crates/domain/tests/test_determinism.rs`,
  `crates/wasm/tests/test_determinism_wasm.rs`, `docs/DETERMINISM_CONTRACT.md`.
- Acceptance criteria: the corpus covers every generator and every branch of its parameter space, with
  a floor of 50 seeds per generator so coverage rather than a round number sets the size; each entry
  records its expected output hash; the same assertions run under `cargo test` natively and
  `wasm-bindgen-test` in headless Chromium; a failure names the first divergent seed; the contract
  states that `rand_chacha::ChaCha20Rng` is used because its algorithm carries a stability guarantee
  that `StdRng` does not, that `BTreeMap` is used wherever iteration order reaches output, and that
  exact equality is the requirement here because the render cache and reproducibility record are keyed
  on it.
- Evidence or review: both command outputs in the tracker. This gate blocks every generation-dependent
  lane and underwrites both the parameter-hash storage decision and the render cache.
- Next dependency: WP-C6 consumes this accepted package.

#### Work package: WP-C6 establish and prove the grading boundary

- Owner: `expert_coder`. Modules: MOD-GRD, MOD-WASM (boundary). Depends on: WP-C1.
- Touch points: `crates/grading/src/lib.rs`, `crates/wasm/src/lib.rs`,
  `tests/test_wasm_export_allowlist.mjs`, `tests/test_crate_boundaries.py`, `docs/SECURITY_MODEL.md`.
- Acceptance criteria: `crates/grading` holds the answer-bearing surface; answer _format_ validation,
  needing no key, stays in `crates/domain` so the browser can call it; the `.wasm` export list is
  compared against a committed allowlist so any new export fails the gate until deliberately added; a
  second check asserts `crates/grading` is absent from the `wasm32` dependency closure; both run in
  `check_codebase.sh`; the document states which side new code belongs on.
- Evidence or review: the allowlist diff is what a reviewer reads. This makes "answers never reach the
  browser" checkable rather than aspirational.
- Next dependency: M2 lane 3 consumes this accepted package.

#### Work package: WP-C7 build the fixture corpus and mock handler set

- Owner: `coder`. Modules: MOD-QM (fixtures), MOD-CLIENT (mocks). Depends on: WP-C3.
- Touch points: `tests/fixtures/published_problem/`, `src/api/mock/`.
- Acceptance criteria: one published problem version with two assets, one draft, one assignment
  reference, one enrollment with three completed runs, and one in-progress run including full
  reproducibility records; mock handlers cover every route group so both UI lanes start before any
  route exists; fixtures generated from MOD-QM types so they cannot drift.
- Evidence or review: a UI lane renders the mastery loop against mocks with the API absent, the
  condition that unblocks M3 early.
- Next dependency: M3 lanes 1 and 2 consume this accepted package.

#### Work package: WP-C9 freeze the frontend architecture contract

- Owner: `architect`, with `solid-js-expert` and `ui-ux-engineer` guidance. Depends on: WP-C1, WP-C7.
- Touch points: `docs/SOLID_MODEL.md`, `docs/FRONTEND_ARCHITECTURE.md`, `src/routes.ts`,
  `src/wasm/index.ts` facade signature, `src/api/` client shape, one reference response widget.
- Acceptance criteria: the route map, reactivity contract, client architecture table, persistence
  boundaries, security rules, focus and error conventions, and the validation strategy from the browser
  interface design section are recorded as the frontend's frozen contract; the WASM facade and generated
  client signatures exist so both UI lanes compile against them; one response widget is implemented end
  to end as the pattern the remaining widgets follow; the accessibility baseline is stated as testable
  conditions rather than aspirations; a test asserts the generated client surface contains no
  answer-bearing type.
- Evidence or review: a UI lane can build a screen against the facade, the client, and the mock handler
  set with no backend running. The reference widget plus this document is what makes the two UI lanes
  independent instead of merely concurrent.
- Next dependency: M3 UI lanes consume this accepted package.

#### Work package: WP-C8 write the contract register

- Owner: `architect`. Depends on: WP-C1 through WP-C7, WP-C9.
- Touch points: `docs/CONTRACTS.md`.
- Acceptance criteria: one row per catalog module naming its contract file, owner, consumers, and
  executable reference/test implementation;
  a stated rule that changing a frozen contract requires updating this file and every consumer lane in
  the same patch.
- Next dependency: M2 dispatch consumes the accepted contract register across seven lanes.

### M3 course appearance package

#### Work package: WP-M3-COURSE-APPEARANCE add course themes and banners

- Owner: `architect` coordinates the seven atomic owners defined in
  `docs/active_plans/decisions/course_appearance_plan.md`. Depends on: WP-QTI-12 plus the
  existing course, auth, object, Store, schema, client, and frontend contracts. WP-QTI-12 and
  WP-CA1 through WP-CA7/WP-RC1 are accepted. The current owner decision uses PLE flat JSON v2 for
  native all-family source; external QTI-JSONL is a separate future adapter concern.
- Touch points: focused `course_appearance` modules in `question_model`, `learning-data-access`,
  `server_core`, and Solid; typed course-banner object/delivery owners; one forward migration; route,
  generated client, Playwright, and durable documentation owners.
- Progress: WP-CA1 through WP-CA7/WP-RC1 passed on 2026-08-09. The closed browser-safe
  Rust/generated-TypeScript contract, exact 15 IDs and Grass default, strong revision and banner
  mutation shapes, executable instructor route, and tenant/course-bound candidate/current banner
  object identities are accepted. Candidates are temporary and non-signable; current banners are
  protected course content whose delivery now requires the exact persisted current pointer. Memory
  and PostgreSQL create and retain the revisioned appearance, revalidate persisted manager authority,
  preserve bytes-first copied-object ownership across stale CAS, and run bounded two-phase cleanup.
  Production HTTP now supplies bounded JPEG/PNG/WebP candidate normalization, strong-ETag no-store
  appearance GET/PUT, bytes-first promotion, and current-only protected delivery. The Solid course
  scope now consumes one authorized course/appearance projection, reuses run data, clears across
  course/global navigation, fails closed on unknown IDs, and passes rendered contrast across all 15
  themes. The real instructor settings workflow, exact entry-only banner, combined PostgreSQL/MinIO
  cleanup lifecycle, all-seven-route browser traversal, visual artifact set, durable documentation,
  and independent no-P0/P1/P2 review are accepted.
- Acceptance criteria: keep 15 measured themes, consolidating `woodland` into `forest`; store only a
  closed theme ID and one revisioned current banner relation; apply exactly canvas, secondary, and
  accent theme colors through one course scope; show the centered banner only on the course entry
  page; preserve global and semantic status colors; prove CAS recovery, forced RLS, current-pointer
  authorization, object lifecycle, 5.5:1 text contrast, keyboard behavior, responsive rendering, and
  absence of grading/answer/object-key data in browser contracts.
- Evidence or review: focused Rust/Node/Playwright gates, disposable PostgreSQL and MinIO oracles, a
  15-theme contact sheet plus palette metrics, and independent PASS with no P0/P1 finding.
- Next dependency: WP-RC3 consumes the accepted production-seam module catalog, active status,
  architecture, file structure, contracts, frontend/route docs, retention/object docs, and changelog
  boundary.

### M3 flat-question family evolution package

#### Work package: WP-M3-FLAT-FAMILIES complete all flat families

- Owner: `architect` coordinates the family closeout in
  `docs/active_plans/active/flat_question_family_evolution_plan.md`.
  Depends on: accepted WP-M3-COURSE-APPEARANCE, the secure learner-payload package, and
  the existing native flat, grading, object, Store, schema, server, client, and frontend contracts.
- Touch points: v1 compatibility; closed PLE flat JSON v2 source/compiler; public/private
  compilation; family response/checker types; source-to-object bindings; persistence, author
  editors, learner widgets, live evidence, and durable documentation.
- Current implementation: the version 2 source/runtime core covers MC, MA, FIB, MULTI-FIB, NUM,
  MATCH, ORDER, and HOTSPOT, while version 1 single choice remains compatible.
- Acceptance criteria: keep answers and optional feedback protected; complete family-specific visual
  authoring and the Memory/PostgreSQL/object-store paths; prove accessible author/learner flows,
  immutable publication, forced RLS, asset lifecycle, correct/incorrect grading, cleanup,
  historical-version compatibility, and no browser/Wasm answer association.
- Evidence or review: focused Rust/Node/Playwright gates, disposable PostgreSQL/object-store oracles,
  the full repository gate, and independent PASS with no remaining P0/P1 finding.
- Next dependency: WP-RC5 publishes the exact Chapter 1 content after MATCH and completes the other
  family work packages in their frozen dependency order.

## Acceptance criteria and gates

- Per-patch gate: `./check_codebase.sh` green (typecheck, wider typecheck, ESLint at zero warnings,
  Prettier, Node tests, Rust fmt/clippy/test, WASM export allowlist); `pytest tests/` green;
  `docs/CHANGELOG.md` updated in the same patch.
- Contract gate: changing a frozen contract requires the same patch to update `docs/CONTRACTS.md`,
  every production consumer, and every executable reference/test implementation. A contract change
  landing without its consumers is a blocking finding.
- Determinism gate: WP-C5 parity green on both targets. Blocks every generation-dependent lane.
- Secrecy gate: WP-C6 allowlist and dependency assertions green, plus the M3 network trace. A red
  secrecy gate is a release blocker with no workaround.
- Tenant isolation gate: from M2, a foreign tenant context returns zero rows and the student-facing
  role cannot read any answer-key table.
- Scale gate: from M2, no `OFFSET` in any query path and no unbounded list endpoint.
- Integration gate: `tests/e2e/` and `./run_playwright_tests.sh` green, all in one run at M5.
- Course appearance gate: the frozen WP-M3-COURSE-APPEARANCE contract, real-role RLS/current-pointer
  oracle, built-browser workflow, computed contrast metrics, and contact sheet are green before the
  appearance capability may be called complete.
- Performance gate: server-side processing time for grading, attempt issue, and catalog browse compared
  against the baseline recorded at M3. A regression beyond 25 percent of baseline opens an
  investigation; the gate exists to catch a change in behavior, so the baseline is re-recorded whenever
  a deliberate change moves it, with the reason noted in the tracker.
- Independent review gate: each lane reviewed by a `reviewer` that did not write it, using
  `audit-code-reviewer` before milestone exit.

## Test and verification strategy

- `cargo test --workspace`: domain rules, transitions, run and policy combinations, the 31-run summary
  scenario, timing tables, scoring, identity lifecycle, and both conformance suites against in-memory
  backends. Fast, no container.
- `pytest tests/`: repo hygiene, the nondeterminism guard, the crate-boundary assertion, and an
  `OFFSET` grep guard. Thin and cross-ecosystem; no assertions on collection sizes, dates, or tunable
  constants.
- `node --import tsx --test tests/test_*.mjs`: generated-type freshness, WASM export allowlist, API
  client and mock-handler shapes.
- `tests/playwright/`: mastery loop, a post-completion practice run, timer behavior, latency
  measurement, publish refusal, and the network trace proving no answer crosses the wire.
- `tests/e2e/`: container-dependent checks -- restart durability, replica independence, clock-skew
  invariance, submission replay, migration application, RLS foreign-tenant isolation, answer-key
  grants, object round trip, partition pruning on a synthetic large attempt table, render cache hit,
  hostile-ZIP corpus, worker queue concurrency, renderer-outage degradation. Excluded from pytest by
  the existing `collect_ignore`.

Failure semantics: a red per-patch gate blocks the patch. A red determinism, secrecy, isolation,
contract, or scale gate blocks the milestone and triggers design review rather than a workaround.

## Risk register

| Risk                                               | Impact                                                                       | Trigger                                                                           | Owner            | Mitigation                                                                                                                                                                                                       |
| -------------------------------------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| An answer or key reaches the browser               | Assessment integrity lost, silent until exploited                            | A new WASM export, or grading code moved into `domain`                            | `expert_coder`   | `grading` absent from the `wasm32` closure; export allowlist gate; M3 network trace (WP-C6)                                                                                                                      |
| RLS is bypassed or unset                           | Cross-tenant exposure of educational records                                 | Application connects as a bypassing role, or tenant context set from client input | `expert_coder`   | `FORCE ROW LEVEL SECURITY`; non-superuser role; context from the authenticated session only; foreign-context test on every gate                                                                                  |
| A frozen contract turns out incomplete             | Parallel lanes stall or diverge                                              | A lane finds a missing trait method mid-flight                                    | `architect`      | Conformance suites shipped with contracts in M1; contract gate requires consumers updated in the same patch; M1 exit walks every catalog row                                                                     |
| Native and wasm32 generation diverge               | Historical attempts not reproducible; render cache serves wrong content      | Parity mismatch                                                                   | `tester`         | Ban known causes up front; measure before dependent lanes start; replace the primitive rather than special-case the platform (WP-C5)                                                                             |
| Attempt tables outgrow the design                  | Slow gradebook, painful migrations                                           | Practice volume beyond 300 M rows per term                                        | `expert_coder`   | Monthly partitions on the four append-only tables from the first migration; summary rows for all grade reads; compact attempt rows; partition-pruning test                                                       |
| Grade computed by scanning history                 | Course pages time out at scale                                               | A convenient aggregate query in a page path                                       | `expert_coder`   | Summary row is the only grade source; review rejects any aggregate over `question_attempt` in a request path                                                                                                     |
| Database bloat from payloads in operational tables | Slow backups, restores, replication                                          | A large payload committed to a table                                              | `expert_coder`   | Role-based split; strict 256 KiB flat source/private cap and other bounded payload constraints; oversized writes refuse; hot/cold table split                                                                    |
| WeBWorK renderer saturates                         | Timed questions fail to load under burst                                     | Many students on WeBWorK questions at once                                        | `expert_coder`   | Deterministic render cache; prefetch; worker pool autoscaled on queue depth, latency, CPU, and timeout rate                                                                                                      |
| External-tool callback accepted as a grade         | Assessment integrity or tenant isolation lost                                | Browser message, stale launch, or unverifiable provider response reaches grading  | `expert_coder`   | iMathAS browser messages are presentation-only; same-origin attempt launch; server-held correlation and idempotency; authenticated server-to-server verification; recorded forged-message and cross-tenant gates |
| Malicious archive during QTI import                | Remote code execution or disk exhaustion                                     | A crafted ZIP uploaded                                                            | `expert_coder`   | Import in the worker; size, expanded-size, and file-count limits; path and symlink rejection; media sniffing; never serve from an extracted path; hostile corpus test                                            |
| Course banner exhausts image processing            | Availability failure or active-content exposure                              | Oversized decoded raster, SVG, animation, malformed codec input                   | `expert_coder`   | Pre-read byte cap; decoded-pixel cap; JPEG/PNG/WebP raster allowlist; metadata-stripping normalization; hostile-image tests                                                                                      |
| Course theme bleeds across route scope             | Wrong course identity or unreadable global/status UI                         | Prior course variables remain after navigation                                    | `ui-ux-engineer` | One course-subtree provider; cross-course/global cleanup tests; computed rendered-pair contrast and contact-sheet review                                                                                         |
| Orphaned objects accumulate                        | Storage cost and retention drift                                             | Deleted records leaving objects behind                                            | `expert_coder`   | Reconciliation job comparing object records to bucket inventory; lifecycle rules; M5 deliverable                                                                                                                 |
| Small-cohort statistics re-identify a student      | Privacy failure disguised as an anonymous aggregate                          | A question attempted by one or two students publishes its statistics              | `architect`      | k-anonymity threshold (default 5) gates publication; suppression test in M5 exit                                                                                                                                 |
| Statistics lost when records are deleted           | The library stops learning, and deletion becomes something instructors avoid | Statistics computed on demand from attempt history                                | `expert_coder`   | Incremental or scheduled aggregation while records exist; discrimination index computed before deletion; MOD-STATS ordered before MOD-RETENTION                                                                  |
| Retention deletes reusable content                 | Instructors lose authored work and stop trusting the system                  | A deletion path following assignment references into shared content               | `expert_coder`   | Deletion is scoped to tenant-owned records by construction; the M5 deletion test asserts catalog content and drafts survive                                                                                      |
| Signed URL leakage                                 | Educational records exposed                                                  | A URL shared or logged                                                            | `expert_coder`   | Minutes-long TTL, not ADAPT's seven days; `student-records` at 5 minutes; access logged per request                                                                                                              |
| Draft problems leak into shared content            | The exact ADAPT failure this design exists to avoid                          | A code path minting `ProblemId` outside publish                                   | `architect`      | `ProblemId` constructible only on the publish transition (WP-C2); test asserting no other construction site                                                                                                      |
| Parallel lanes collide on a shared artifact        | Merge conflicts and lost work                                                | Two lanes editing migrations or the seed table                                    | `integrator`     | One owning module per shared artifact, tabulated in the catalog                                                                                                                                                  |
| Scope creep toward ADAPT parity                    | Version 1 never ships                                                        | Requests for rubrics, learning trees, discussions                                 | `architect`      | Binary out-of-scope ledger in the release-completion plan                                                                                                                                                        |
| Plan drifts from implementation                    | Reviews check the wrong thing                                                | Package work outpacing the tracker                                                | `architect`      | Release-completion tracker updated at every WP-RC exit                                                                                                                                                           |

## Rollout and release checklist

- [ ] M0 through M4 run only on `podman compose` with MinIO; no cloud resources provisioned.
- [x] `api` scaled to two replicas in the maintained Compose E2E from M2, so replica assumptions are
      exercised continuously rather than discovered in production.
- [ ] RDS PostgreSQL with KMS encryption at rest, automated backups, and point-in-time recovery.
- [ ] PostgreSQL in private subnets; TLS with certificate verification on every hop.
- [ ] Application role is non-superuser and cannot bypass RLS; verified in the deployed environment,
      not only in tests.
- [ ] Three private buckets with server-side encryption, per-bucket lifecycle rules, no public access
      except the CDN origin path for public content.
- [ ] Secrets in Secrets Manager; no credential in any image layer or in git.
- [ ] Fargate autoscaling: `api` on request count with minimum two tasks; `worker` and `renderer` on
      queue depth.
- [ ] Renderer reachable only on the private network, with CPU, memory, and request-time limits.
- [ ] Class-start burst load test run; replica count and p99 recorded.
- [ ] Restore-from-backup rehearsed and timed.
- [ ] FERPA control checklist completed with evidence per control; retention and deletion implemented
      for `student-records` and render traces.
- [ ] Retention default configured to the privacy-preserving value, with the per-institution override
      documented and one non-default institution exercised.
- [ ] A real course deletion rehearsed end to end: records and bucket artifacts gone, catalog content
      and anonymous statistics intact, and the result recorded.
- [ ] Course appearance acceptance completed: real-role RLS/current-pointer oracle, centered
      entry-banner lifecycle, all-route theme scope, 15-theme contact sheet, and measured contrast.
- [ ] `devel/make_release.py` run for the first tagged release after WP-RC12 is green.

## Documentation close-out requirements

- Active plan and tracker: update
  `docs/active_plans/active/release_completion_plan.md` at every WP-RC exit and move completed
  companion plans to `docs/archive/` with `git mv` only after their acceptance gates pass.
- `docs/CHANGELOG.md`: one entry per patch under the canonical section headings, recording key
  implementation choices and failures so the log stays a learning record.
- New durable docs, each owned by the work package creating it: `docs/CONTRACTS.md`,
  `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`, `docs/INSTALL.md`, `docs/USAGE.md`,
  `docs/LOCAL_STACK_OPERATIONS.md`, `docs/MACOS_PODMAN.md`, `docs/QUESTION_MODEL.md`,
  `docs/ACTIVITY_MODEL.md`,
  `docs/PROBLEM_IDENTITY.md`, `docs/OBJECT_STORAGE.md`, `docs/DATABASE_TENANCY.md`,
  `docs/DETERMINISM_CONTRACT.md`, `docs/ADAPTER_DEVELOPMENT.md`, `docs/SECURITY_MODEL.md`,
  `docs/RETENTION_POLICY.md`, `docs/SOLID_MODEL.md`, `docs/FRONTEND_ARCHITECTURE.md`,
  `docs/DEVELOPMENT.md`.
- Closure notes: record measured latency, determinism parity evidence, the WASM export allowlist, and
  the partition-pruning result in the tracker before archiving, so the architecture's central claims
  stay auditable.

## Patch plan and reporting format

- Patch 1: WP-F1, WP-F2 (workspace and WASM path).
- Patch 2: WP-F3 (Solid app, build pipeline, template defect fixes).
- Patch 3: WP-F4, WP-F5 (containers and extended gate).
- Patch 4: WP-F6 (foundation documentation).
- Patch 5: WP-C1 (question model and taxonomy).
- Patch 6: WP-C2 (identity and lifecycle).
- Patch 7: WP-C3 (run, policy, and summary model, including the 31-run test).
- Patch 8: WP-C4 (store and object contracts with reference backends and conformance suites).
- Patch 9: WP-C5 (seed vectors and parity harness) -- its own patch because it is a gate.
- Patch 10: WP-C6 (grading boundary) -- its own patch because it is a gate.
- Patch 11: WP-C7 (fixtures and mock handlers).
- Patch 12: WP-C9 (frontend architecture contract, reactivity model, reference widget) -- its own patch
  because the UI lanes' independence rests on it.
- Patch 13: WP-C8 (contract register).
- Patches 14 onward are accepted implementation history. Remaining patches follow WP-RC2 through
  WP-RC12 in `docs/active_plans/active/release_completion_plan.md`, one integrated
  package at a time with only the explicitly owned subpatch splits.

Report each patch as: module ID and work package ID, files touched, gate commands with their exact
output lines, and any skipped check with a one-line scope note.

## Decision completeness

There are no unresolved implementation or scope decisions in this plan. The former questions are
settled as follows and expanded into dispatchable packages in
`docs/active_plans/active/release_completion_plan.md`:

- MC is the accepted first family and MATCH is next.
- New assignments default to `highest`; new practice runs use `newSeeds` while resumed attempts keep
  their issued seed.
- Retention defaults are notify at 30 days, archive at 100 days, learner-record deletion at 365 days,
  and aggregate publication at k >= 5.
- Course deletion retains assignment definitions by default.
- Existing normalized-payload hard ceilings remain strict refusal boundaries; oversized archival and
  binary source uses typed object storage.
- Content-addressed deduplication is out of scope because it is an optimization, not an integrity or
  lifecycle requirement.
- Published WeBWorK PG source is an immutable PLE object with license/provenance, and the adapter
  calls the private external standalone `/render-api` service directly.
- Native Rust `axum` remains the server runtime; the TypeScript-server alternative is closed.
