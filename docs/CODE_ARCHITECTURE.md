# Code architecture

## Overview

Peptidyle Learning Engine (PLE) is a question-agnostic learning platform. A
SolidJS browser client renders server-issued presentations; a Rust API owns
identity, authorization, assessment state, grading, and durable records. The
repository separates question generation, answer-bearing grading, persistence,
object storage, and external-engine integration into focused Rust crates.

This is the high-level map. [FILE_STRUCTURE.md](FILE_STRUCTURE.md) maps paths,
[CONTRACTS.md](CONTRACTS.md) indexes durable rules, and
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) records their rationale. Current
release state belongs in [active_plans/](active_plans/), not in this document.

## System shape

```text
browser
  SolidJS application + answer-free domain WebAssembly
        |
        | same-origin HTTPS requests and browser-safe JSON
        v
CloudFront --> application load balancer --> Rust API --> PostgreSQL
                                              |          |
                                              |          `-> forced RLS tenant records and jobs
                                              v
                                         four object domains
                                              |
                       public-assets publisher outbox and dedicated publisher

Rust API --> private external question-engine broker --> reviewed provider or renderer
```

The browser and WebAssembly bridge are deliberately thin and key-free. They
format values, validate response shape, and display timers, but receive no
answer keys and cannot make authoritative timing, grading, feedback, or
completion decisions. The API is the trust-boundary owner for every
browser-visible assessment transition.

## Security boundaries and guarantees

- **Server-owned assessment authority.** Answer keys, correctness, timing
  verdicts, the assignment-owned five-field learner-disclosure policy,
  completion, and grade changes remain in Rust server code and durable storage.
  [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) and
  [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) define this
  boundary.
- **Compile-time answer separation.** `crates/grading/` is outside the
  dependency closure of `crates/wasm/`. The browser bridge therefore cannot
  import answer-bearing checkers. [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md)
  defines adapter responsibilities.
- **Actor-scoped learner access.** Learner routes use learner-scoped store
  operations. Each operation verifies the acting user and re-evaluates active
  student membership, assignment audience, and applicable groups before
  binding the stable learner identity to a retained enrollment, attempt, or
  run in the same store boundary; a route-level ownership check is not the
  sole control.
- **Database-enforced tenant isolation.** PostgreSQL forces row-level security
  on tenant-bearing records. API, worker, grader, and public-asset-publisher
  processes use distinct login profiles with closed capability-role contracts;
  startup attests the required login and role attributes before handling work.
  [DATABASE_TENANCY.md](DATABASE_TENANCY.md) and
  [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) define the record
  scope rules.
- **Physical object-domain isolation.** Typed keys select four object domains:
  `public-assets`, `private-content`, `student-records`, and
  `temp-processing`. The deployed configuration gives each its own S3 bucket
  and KMS key. A key cannot be supplied as an arbitrary object-store path.
- **Published public assets are a committed transition.** Catalog publication
  records pending public assets and a `publishPublicAssets` job transactionally.
  The dedicated publisher verifies bytes and checksums, writes only the final
  immutable public object, then conditionally activates its registry record.
  The ordinary API and worker do not own that public-write capability.
- **External side effects are fenced.** The external-tool broker owns launch,
  submission, and result normalization. It stores a pre-dispatch marker under
  an active activity lease before an effectful provider request; an uncertain
  outcome remains fenced rather than being retried as a potentially duplicate
  learner action.

## Major components

| Component                | Location                                                                                  | Responsibility                                                                                                                                                                                                                                                              |
| ------------------------ | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Question model           | [crates/question_model/](../crates/question_model/)                                       | Question taxonomy, typed public references, immutable public bylines, mandatory course-term values, canonical assignment revisions, reusable-curriculum meaning, bounded adoption previews, browser-safe presentations, learner-progress projections, and response schemas. |
| Domain                   | [crates/domain/](../crates/domain/)                                                       | Run state, pure effective-policy and learner-disclosure evaluation after entitlement, the two shipped course-grade evaluators, seeded generation, timing inputs, and validation without a database or wall clock.                                                           |
| Grading                  | [crates/grading/](../crates/grading/)                                                     | Answer-bearing checkers and correctness decisions; server-side only.                                                                                                                                                                                                        |
| Learning data access     | [crates/learning-data-access/](../crates/learning-data-access/)                           | Store contracts, concrete PostgreSQL production persistence, compiler-gated deterministic test adapters, current receipt/projection boundaries, migrations, forced RLS context, capability roles, and conformance tests.                                                    |
| Base Course installation | `crates/base-course-installation/`                                                        | Focused product crate for typed Base Course request/receipt, ordinary recipe, and deterministic orchestration. It has no HTTP route or server-start hook.                                                                                                                   |
| Object storage           | [crates/objects/](../crates/objects/)                                                     | Typed object keys, checksums, strict image ingress validation, and MinIO/S3-compatible implementations.                                                                                                                                                                     |
| Native adapter           | [crates/adapters/native/](../crates/adapters/native/)                                     | First-party generated questions and static flat-question compilation.                                                                                                                                                                                                       |
| External adapters        | [crates/adapters/](../crates/adapters/)                                                   | Bounded QTI, H5P, iMathAS, and WeBWorK integration behind declared capabilities.                                                                                                                                                                                            |
| Server                   | [crates/server/](../crates/server/)                                                       | Axum routes, passwordless auth, authorization, instructor assignment workspace and course-grade routes, learner aggregate and per-item redaction, adapter selection, API composition, ordinary worker, and public-asset publisher process.                              |
| Browser and WebAssembly  | [src/](../src/) and [crates/wasm/](../crates/wasm/)                                       | SolidJS interaction layer, strict browser decoder/editor boundary, shared answer-free assignment landing presentation, focused assignment workspace pages, and answer-free browser bridge to shared domain logic. |
| Export and project tools | [crates/export/](../crates/export/) and [crates/project-tools/](../crates/project-tools/) | Print/export generation plus repository-only code generation, migration, fixture, pilot-content validation, E2E seed commands, and the direct `base-course` CLI adapter.                                                                                                    |
| Deployment configuration | `deploy/opentofu/`                                                                        | AWS network, edge, compute, database, storage, IAM, observability, and WAF definitions.                                                                                                                                                                                     |

The Cargo dependency graph is a security control: `crates/domain/` depends only
on the question model, while `crates/wasm/` does not depend on `crates/grading/`.
The server composition root selects concrete stores, object backends, identity
providers, and adapters.

Reusable curriculum has two deliberate aggregate boundaries. `ReusableCurriculumStore` owns
personal Blueprint and shared Alpha CRUD. `CurriculumAdoptionStore` owns revision-bound fork,
instantiation, rollover, term-shift, import inspection, controlled-update, and receipt-led
reconciliation operations. The
question model keeps semantic baselines separate from immutable source provenance, resolves relative
schedules by target-term calendar days and IANA local time, bounds every browser witness, and derives
course-creation commands from the exact previewed source, title, term, and revision evidence.

An adoption preserves reusable meaning separately from teaching-owned state. Its normalized semantic
payload contains the ordered pins, pool behavior, scoring, reusable defaults, and relative schedule;
receipt-keyed immutable evidence records the exact source definition, observed revision, destination
binding, actor, time, and completed outcome. A separate current import projection is explicitly
repairable from that evidence; reconciliation changes only that projection and refuses when the
immutable evidence is incomplete. The Store reauthorizes every exact pin for the destination,
records the authorized pin, and keeps the original evidence inspectable. Rollover preserves the
source course's ordered module tree while creating an empty ordinary teaching course, and term shift
returns a typed rollover recovery when issued work makes in-place shifting ineligible.
Roster, learner activity, issued work, grades, accommodations, audience, retention, and delivered
evidence remain teaching-owned state.

The deterministic in-memory adapter is compiled only for crate tests or the explicit
`test-support` Cargo feature. The production and live-demo server composition has one concrete
`PostgresStore`; it has no runtime storage selector or in-memory fallback. This makes the adapter a
contract-test tool rather than an alternate application architecture. The B2 PostgreSQL migration,
broker/RLS boundary, routes, browser client, and connected workflow use that production composition.
The mutable package handoff and acceptance evidence remain in
[implementation_status.md](active_plans/implementation_status.md).

For Base Course installation, `learning-data-access` remains the sole SQL, PostgreSQL
lock, durable install-state, migration, and Store owner. `base_course_installation` orchestrates
only through LDA's public contracts, and `project-tools` calls it directly as a CLI adapter. The
installer is not a server component and does not gain an HTTP route or a server-start hook. Evidence
stays KISS: pure installer-crate tests cover typed request/receipt/recipe convergence; the existing
LDA PostgreSQL live oracle covers schema and locking; and
`tests/e2e/e2e_live_demo_baseline.py` covers the connected lifecycle.

Course-term ownership is deliberately vertical and singular. `question_model` validates exact
calendar dates, ordered inclusive bounds, and case-sensitive IANA membership; `CourseRecord` and
`CourseSummary` require that value. Memory and PostgreSQL Store implementations carry the same
value, the existing course routes serialize it, generated TypeScript owns the response shape, and
the Solid course form supplies it explicitly. PostgreSQL adds native date/date/text columns to the
existing course row and treats an invalid stored value as unavailable rather than inventing a
fallback. B2 stores reusable availability, due, and close defaults as relative calendar-day and
local-wall-time values. It resolves them in the selected target term's IANA zone, returns local and
absolute preview outcomes, and supplies typed field-specific corrections for DST gaps and
ambiguities. `CourseScheduleRevision` binds a whole-course preview and apply pair; course-term and
assignment base-schedule writers advance it atomically, while assignment-local edits retain their
own `AssignmentRevision` contract.

## Assessment and asset-publication flow

```text
authorized author publishes immutable version
  -> catalog transaction registers pending public assets + durable outbox job
  -> dedicated publisher verifies private source bytes and checksum
  -> publisher writes tagged immutable public asset and activates registry
  -> course assignment references immutable version
  -> active enrolled learner starts or resumes a run
  -> API issues one attempt-bound browser-safe presentation
  -> browser submits response identity and value
  -> API authorizes, validates, grades, and commits the result
  -> API projects only currently released score, per-item, feedback, and solution fields
```

`crates/server/src/run/` coordinates issue, prefetch, submission, manual
grading, and external-tool paths. `crates/learning-data-access/` commits run,
submission, score, summary, and outbox transitions. The browser decodes the
response schema and selects a response widget; it does not select a grading
backend or derive a correct response.

Published content is immutable and shared. Courses, memberships, enrollments,
runs, attempts, grades, and student artifacts are tenant-owned. This keeps
course-record deletion separate from reusable published content.

## Assignment workspace flow

The Instructor assignment workspace is a course-bound, revisioned aggregate
with four focused pages. A linked assignment title opens the assignment home;
the home provides status, publication readiness, and local navigation to
Questions, Policies, and Student view. New assignment creation persists an
empty Draft before the Questions page opens. Draft and Archived definitions may
remain empty; a transition to Published requires publication readiness, including
an active deliverable position and supported question capabilities.

```text
Instructor course assignments
  -> linked title
  -> assignment home
  -> Questions | Policies | Student view
  -> focused revision-checked save or no-store read
```

`crates/question_model/src/assignment_workspace.rs` owns the closed create,
content, policy, audience, publication-readiness, and issued-work-conflict
contracts. `crates/learning-data-access/src/contracts/assignment_editing.rs`
defines the Store boundary; the in-memory adapter is in
`crates/learning-data-access/src/in_memory/assignment_workspace.rs`, and the
PostgreSQL implementation is in
`crates/learning-data-access/src/postgres/course_assignments.rs`. The forward
capability boundary is
[`2026081848_assignment_workspace_drafts.sql`](../schemas/migrations/2026081848_assignment_workspace_drafts.sql).

The server's nested routes live in
`crates/server/src/course/assignments/workspace.rs`. `POST .../drafts` creates
the empty draft. `PUT .../content` replaces only the Questions-owned title and
ordered definition, while `PUT .../policies` atomically replaces the
Policies-owned audience, disclosure, run policy, instructions, schedule,
limits, and lifecycle. Both updates require the current `If-Match` assignment
revision and return a complete authoritative workspace detail with a new
`ETag`; stale writes preserve the browser's local draft. Structural content
changes after learner work is issued return the closed generated
`generated/api/AssignmentContentIssuedWorkConflict.ts` contract, which the
Questions page maps to durable create-a-new-assignment guidance.

`assignment_landing_presentation` in
`crates/server/src/course/assignments/learner.rs` builds one answer-free
landing projection from the current assignment definition and course time
zone. The ordinary learner detail adds learner-authorized delivery and
progress, while the Instructor Student-view route adds course-wide base
delivery and keeps the Instructor identity. Both render through
[`src/components/learner_assignment_presentation.tsx`](../src/components/learner_assignment_presentation.tsx).
Student view is a `no-store` read and creates no enrollment, run, attempt,
submission, receipt, grade, or preview record.

The former combined assignment editor and its teaching-settings route are
retired. Assignment authoring now belongs to `src/pages/assignment_workspace/`;
the existing `assignment_editor_*` modules that remain are focused Questions
helpers (picker, content list, pool, model, repository, and styles), not a
combined assignment route. The independent `/workspace/:workspaceRef` editor
remains the private question-draft editor and is unrelated to course assignment
workspace routing.

## Identity and authorization

`crates/server/src/auth/` implements passwordless email and passkey flows,
session issuance, logout, challenge binding, request-origin checks, and
rate-limiting inputs. Opaque user identifiers, not email addresses, identify
accounts. Canonical course-membership episodes and the derived entitlement
evaluator determine current course/assignment authority; an enrollment is
retained educational evidence, never an access grant.

The data-access Store contracts make authority explicit at the persistence
boundary. Learner operations accept an actor and require active learner access;
Instructor-history operations use their own contracts. PostgreSQL evaluates those
operations in transactions with tenant context and row-level security. The compiler-gated
in-memory adapter supplies deterministic contract behavior for unit and conformance testing.

The effective-policy resolver is a separate pure domain boundary. It consumes the entitlement
decision and evaluator-approved scopes supplied by S5, then applies lifecycle, entitlement, and
authorization gates in order before resolving the base policy, approved group modifiers, and an
individual exception. Both Store backends construct that same grant-filtered input for current
resolution and action paths. PostgreSQL alone persists the sealed per-attempt receipt and normalized
per-field provenance as immutable historical attempt evidence. S4 disclosure
projections instead use the current S3-resolved effective-policy verdict with
the current assignment-owned disclosure policy; no attempt-level receipt is a
learner-disclosure authority.

Assignment teaching intent remains one revisioned aggregate:
`AssignmentTeachingSettings` contains the closed lifecycle, validated plain-text
instructions, and the S3 base policy. The Policies workspace transports
`InstructorAssignmentTeachingSettingsLocal`; the server alone converts its
millisecond-precise wall-clock fields through the course's stored IANA zone and
inclusive term. Memory and PostgreSQL validate the same aggregate, apply it
atomically with the assignment revision, and re-resolve active attempts. Only
stored `Published` lifecycle opens G1; Draft is not implicitly published,
Closed and Archived are unavailable for learner starts, and Archived cannot
reopen. Workspace reads return `teachingSettings` alongside a closed
`currentState` union computed from backend-authoritative time and the same
schedule boundaries.

The learner does not receive that intent record. The dedicated learner-detail
route runs current S5 entitlement and the current S3 resolver first, then emits
only instructions and resolved delivery facts such as the course zone,
availability/due/close instants, limits, deadline behavior, and neutral late
status. Draft, closed, attempt-limited, or otherwise denied assignments share a
non-enumerating result. `ScoringStatus` is independent of disclosure and
activity: Recalculating or Failed suppresses learner aggregate scores, run
scores, attempt results, and feedback point values until the maintained summary
is Current.

## Course-grade flow

WP-PROF-S6 adds one isolated `CourseGradebookStore` capability. The pure domain evaluator supports
only the two shipped modes: total points and weighted categories with optional drop-lowest. A
completion-based mode is deferred to a later package and is absent from the model, migration, and
HTTP selector. Memory and PostgreSQL implementors share the same validation and conformance cases.

```text
Instructor GET /grade-scheme
  -> one course scheme snapshot + current server-owned assignment titles
Instructor PUT /grade-scheme with If-Match
  -> title-free whole-scheme replacement under a positive revision CAS
Instructor GET /gradebook-totals
  -> server evaluator + maintained compact assignment summaries
  -> compact no-store totals and explicit unavailable reasons
Instructor POST /grade-export.csv (empty body)
  -> bounded synchronous rows + PII-free durable audit metadata
```

Totals use one scheme snapshot and never ask the browser to recompute a score. The compact totals
response omits email and raw learner-summary data. Export rows may carry ephemeral roster email and
display name for the direct instructor, while `course_total_export_audit` stores only course, actor,
revision, mode, rounding, row count, and timestamps. Connected evidence runs
under the fixed `ple-live-demo-browser` owner: one canonical production-browser
invocation is followed serially by the distinct WebWork renderer and
two-API/one-PostgreSQL replica service oracles. This is the accepted S6
capability boundary.

## Learner disclosure and progress projection

Each assignment owns five independent learner-disclosure timings: score,
per-item correctness, feedback text, solution, and class statistics. The
closed timing vocabulary lives in
[crates/question_model/src/run_policy.rs](../crates/question_model/src/run_policy.rs).
The pure evaluator in
[crates/domain/src/disclosure_policy.rs](../crates/domain/src/disclosure_policy.rs)
consumes only the S3-resolved effective-policy verdict, S5-authorized inputs,
an authoritative supplied time, and evidence that the learner submitted. It
does not reconstruct entitlement, consult a browser clock, or treat a feedback
release record as an unlock authority.

[crates/learning-data-access/src/feedback.rs](../crates/learning-data-access/src/feedback.rs)
forms the private current disclosure/projection boundary: after S5 entitlement,
it combines the current S3-resolved effective-policy verdict, assignment policy,
authoritative evaluation time, and submitted fact before server serialization.
Both Store backends rebuild that input for current reads; PostgreSQL decodes the
five normalized columns through `assignment_records/learner_disclosure.rs` under
[crates/learning-data-access/src/postgres/](../crates/learning-data-access/src/postgres/).

The same learner route evaluates the independently timed class-statistics
field before it reads `CourseItemAnalysisStore`. That Store rechecks current S5
entitlement and returns only `LearnerClassStatistics`: metric-free
`insufficientEvidence` or `available` with a completed-learner cohort size and
normalized assignment average. It uses the latest completed run per enrollment
from the current course-local report and suppresses metrics below the default
five-learner floor, during incomplete manual grading or recent rescoring, and
when the average is absent or invalid. The browser receives the server result
only; it never evaluates policy, timing, or aggregate evidence.

Forward migration
[schemas/migrations/2026081805_assignment_learner_disclosure_policy.sql](../schemas/migrations/2026081805_assignment_learner_disclosure_policy.sql)
creates that one column-level authority and removes the retired coarse
disclosure columns.

The server uses the resulting decision to redact per-item run fields and to
project aggregate scores into the browser-safe `LearnerAssignmentProgress`
contract. `noActivity`, `withheld`, and `available` distinguish a learner with
no submitted response from a learner whose current policy withholds score totals. A started run may
still supply the safe last-activity timestamp while its score state remains `noActivity`.
The browser receives neither the policy, the authoritative clock, nor tenant
or enrollment identifiers needed to infer or bypass that decision.
`assignment_policy.ts` and `assignment_policy_validation.ts` under
[src/api/decoders/](../src/api/decoders/) strictly decode assignment policy
and focused Policies corrections. The assignment workspace owns native
controls in
`src/pages/assignment_workspace/assignment_workspace_policy_panel.tsx`; the
Questions page uses the focused picker and content/pool helpers that remain in
`src/pages/assignment_editor_*`.

## Catalog discovery and statistics disclosure

Catalog discovery is a Store capability shared by the in-memory and PostgreSQL
implementations. Its opaque continuation is a versioned, query-bound,
server-secret HMAC capability that also retains the first-page publication and
statistics-disclosure event boundary. Both Store implementations reevaluate
current lifecycle and tenant visibility on every request, while page rows and
facets describe the same retained ranked snapshot. The database-independent
ranked-search admission and fixed-point scoring helpers live in
`crates/learning-data-access/src/in_memory/catalog_search.rs`;
the in-memory catalog module owns its state projection and snapshot assembly.

PostgreSQL owns canonical full-text and word-similarity discovery in
`crates/learning-data-access/src/postgres/catalog/search.rs`.
It runs page and facet queries from one ranked CTE in a tenant snapshot, pins
the word-similarity threshold, and uses the migration-provided normalized
search projection and indexes. Migration
`schemas/migrations/2026081401_ranked_catalog_discovery.sql`
adds the shared monotonic publication/disclosure event sequence, the
security-invoker catalog view, and forced-RLS, broker-owned disclosure recording.
It is the database authority for disclosure events; application readers see
them only through catalog visibility.

Typed public references live in `crates/question_model/src/public_route.rs` and resolve through one
authorized server navigation result; they never become authorization inputs. Immutable
`PublicByline` attribution lives beside the published question model, while private author-account
relationships remain outside browser-safe catalog projections. PostgreSQL stores and validates the
ordered byline on the immutable version and projects it through the security-invoker catalog view.

## External question engines

Adapters normalize supported source formats into PLE question and grading
contracts. PLE continues to own assignment, attempt, authorization, record,
and feedback policy. The external-tool broker treats provider data as
untrusted input and isolates effectful requests behind activity leases,
request identities, and indeterminate-outcome fences.

The WeBWorK integration calls a separate private `webwork-pg-renderer` service
for PG/PGML rendering and grading. It is not a second assignment platform. The
adapter projects reviewed response shapes for the browser and keeps source and
grading calls server-side. [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md)
documents that boundary.

`OTHER_REPOS/` contains reference snapshots only. It is not a source import
path, container build context, or runtime dependency.

## Storage and background work

PostgreSQL stores policy-bearing relationships, passwordless sessions,
attempts, submissions, summaries, jobs, and audit events. Object storage keeps
the bytes that do not belong in relational rows. The four typed domains are:

- `public-assets`: immutable, published browser-visible problem assets.
- `private-content`: source, restricted problem material, and private course
  assets.
- `student-records`: tenant-owned exports and learner record artifacts.
- `temp-processing`: bounded temporary ingress and processing bytes; it is not
  signable for browser delivery.

The normal worker claims the supported educational job families. The
public-asset publisher is a separate process (`--public-asset-publisher`) with
its own database login, queue filter, and object-store authority. A restart
does not erase a job or educational result because the state transitions remain
in shared storage.

## Browser and local/deployed topology

The browser application in [src/](../src/) decodes API contracts before they
enter Solid components. `crates/wasm/` exposes answer-free helpers for
deterministic formatting, response validation, and timer support. The server
remains authoritative whenever browser and server could disagree.

[containers/compose.yaml](../containers/compose.yaml) defines the common
topology: gateway, API, worker, PostgreSQL, MinIO, a private renderer, and
one-shot setup services. The fixed developer/browser session adds
`tests/e2e/compose.live-demo-browser.yaml`
for production authentication, disposable storage, and the TLS gateway. The
gateway is the browser entry point. The owner lease serializes developer and
browser sessions and verifies exact cleanup.
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) document its topology
and operation.

`python3 local_stack.py` is the repository-anchored operator front door for inspection,
start, stop, restart, reset, validation, logs, and the aggregate live browser
acceptance handoff. Its `local_stack_control/` package owns typed Compose
provider selection, environment-file metadata and inherited-environment
sanitization, label-based Podman discovery, semantic service status, and
project-scoped cleanup plans. Focused Python modules own lifecycle sequencing:
`lifecycle.py` coordinates typed start, validation, and restart requests;
`local_environment.py` and `private_files.py` own default-only private state;
`browser_suite_developer.py`, `browser_suite_lease.py`, and `browser_suite_reset.py`
own the fixed developer/browser lifecycle; `private_state.py` owns mode-0700
repository-target run directories,
replacement-resistant descriptor access, and cross-process cleanup receipts for remote Podman
bind sources; `renderer.py` owns selected-renderer OCI configuration-ID provenance;
and lifecycle validation, waits, and diagnostics retain semantic readiness and safe failures.

`local_stack_control/chapter_one.py` owns Chapter One subprocess and protected,
same-directory atomic manifest publication inside the selected lifecycle. The
fixed live-demo owner delegates this baseline installation without creating a
separate stack or browser route.

The controller discovers resources by Compose labels rather than generated
names. Read-only commands may inspect a named project. Default mutations are
restricted to fixed owner policies. A separate closed disposable-owner
adapter (`python3 -m local_stack_control._consumer_cli`) forms temporary service
targets only from a private mode-0600 manifest and a runner-held cleanup
capability. The retained narrow release/service owners are `course-appearance`,
`live-demo-baseline`, `wp-r2-postgres-rls`, and `wp-rc8-postgres-outbox`.
Browser, WebWork, replica, and `DATABASE_BASELINE` profiles share the fixed
`live-demo-browser` owner; each profile fixes its Compose files and capabilities
before any action is formed. `live-demo-browser` is the
owner-locked, disposable HTTPS production-auth E2E and developer session; it is neither a
caller-selected local target nor a public production deployment. The adapter
allows scoped Compose actions, diagnostics, or the policy-declared outage action,
while cleanup requires the private capability and label-derived
snapshot. It cannot accept a caller-selected target or generic removal command.
The browser scenario, screenshot, and service-oracle owners import the controller's
discovery and cleanup primitives while retaining their private inputs, fixed-port
checks, visible-action evidence, and report boundaries. All three use the fixed
`ple-live-demo-browser` lifecycle and seeded production authentication.

The `DATABASE_BASELINE` profile is a browser-free PostgreSQL oracle under that
same fixed `ple-live-demo-browser` lease and project. It runs serially and
resets the owner afterward; it is not an independently named stack.

### Assignment delivery preview

The preview plane keeps delivery-policy authority on the server. Its route
authenticates the direct instructor and binds the requested course before it
decodes a request. A synthetic or derived subject is identity-free. The server
then owns the ordered S5 authorization, S3 effective-policy resolution, and S4
disclosure projection. A successful derived preview records exactly one
PII-minimal audit; synthetic and denied requests do not add a derived audit.
The browser uses strict relative same-origin `no-store` transport and renders
the returned result. The canonical real-stack scenario creates and changes the
needed assignment, group, and policy state through visible PLE controls before
asserting the instructor result, persistence, revision recovery, and access
boundaries.

`python3 local_stack.py acceptance` is the public aggregate acceptance entry
point. It delegates stack-conflict preflight and child-environment
sanitization to the controller, then invokes
`local_stack_control/acceptance_lanes.py`. That Python module runs exactly one
canonical production-browser invocation, followed serially by the distinct
browser-free WebWork renderer and two-API/one-PostgreSQL replica service
oracles under the fixed `ple-live-demo-browser` owner; it does not duplicate
lifecycle policy. The retained shell validation-lane entry point is only a
compatibility `exec` facade.

`deploy/opentofu/` identifies the production deployment target:
CloudFront and WAF at the edge, a CloudFront-restricted application load
balancer, private ECS tasks, private PostgreSQL, S3 VPC access, separate task
roles, and four encrypted object buckets. It is deployment configuration, not
evidence that an AWS account has been provisioned or operated correctly.

## Testing and verification

- [check_codebase.sh](../check_codebase.sh) runs the repository's TypeScript,
  Rust formatting, lint, and test gates.
- [tests/](../tests/) contains repository-policy and deterministic Node checks.
- Rust unit and integration tests live beside their crates; data-access
  conformance tests exercise matching in-memory and PostgreSQL behavior.
- The ignored PostgreSQL Store, disclosure, and plan suites in
  [crates/learning-data-access/tests/](../crates/learning-data-access/tests/)
  require the disposable acceptance database. Their exact selectors run from
  [tests/e2e/e2e_database_baseline.sh](../tests/e2e/e2e_database_baseline.sh),
  which is the database-baseline runner rather than a fast offline gate.
- [tests/playwright/](../tests/playwright/) contains production-browser
  scenarios and accessibility checks. Every Playwright E2E selection runs only
  through [run_playwright_tests.sh](../run_playwright_tests.sh) and its fixed
  owner; aggregate acceptance invokes the canonical scenario catalog once,
  then runs its distinct browser-free service oracles serially.
- [tests/e2e/](../tests/e2e/) contains disposable PostgreSQL, replica,
  WebAssembly, local-stack, and publication evidence.
- `tests/test_local_stack_control.py` is an offline behavior suite for typed
  controller contracts. It supplies in-memory command results rather than
  starting Podman, Compose, a browser, or a network service.
- `deploy/opentofu/tests/policy.tftest.hcl` asserts deployment-policy invariants
  in the infrastructure configuration.

[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) distinguishes permanent tests,
one-time acceptance evidence, and human review. A passing document or plan is
not evidence that an unrun deployment path works.

## Extension points

- Add question behavior behind an adapter and declare its capabilities in the
  question model.
- Add persistence behavior through a Store contract, both implementations, and
  conformance coverage when behavior is shared.
- Add HTTP behavior in its owning server module; keep composition focused on
  dependency assembly.
- Add browser behavior in its owning API, feature, page, or component module;
  preserve the decoded contract boundary.
- Add an external-engine response family only after defining its safe browser
  projection, grading handoff, authorization, and side-effect behavior.
- Add forward-only schema changes under [schemas/migrations/](../schemas/migrations/).
- Add a normal local-stack operation in `local_stack_control/` and expose it through
  `python3 local_stack.py`; keep lifecycle state in focused typed Python modules.
- Add a disposable E2E consumer by declaring a closed owner policy and private
  manifest contract in `local_stack_control/consumer.py`, rather than adding a
  general project or cleanup flag.

## Known gaps

- Verify the deployed AWS account's DNS, ACM certificates, Secrets Manager
  values, database login provisioning, backup recovery, alerting, and incident
  procedures before production use.
- Verify each enabled external provider and renderer image against its live
  protocol, egress rule, authentication, and operational recovery contract.
