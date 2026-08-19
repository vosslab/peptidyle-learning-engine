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

| Component                | Location                                                                                  | Responsibility                                                                                                                                                                                    |
| ------------------------ | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Question model           | [crates/question_model/](../crates/question_model/)                                       | Question taxonomy, typed public references, immutable public bylines, mandatory course-term values, capabilities, browser-safe presentations, learner-progress projections, and response schemas. |
| Domain                   | [crates/domain/](../crates/domain/)                                                       | Run state, pure effective-policy and learner-disclosure evaluation after entitlement, seeded generation, timing inputs, and validation without a database or wall clock.                          |
| Grading                  | [crates/grading/](../crates/grading/)                                                     | Answer-bearing checkers and correctness decisions; server-side only.                                                                                                                              |
| Learning data access     | [crates/learning-data-access/](../crates/learning-data-access/)                           | Store contracts, in-memory and PostgreSQL implementations, current receipt/projection boundaries, migrations, forced RLS context, capability roles, and conformance tests.                        |
| Object storage           | [crates/objects/](../crates/objects/)                                                     | Typed object keys, checksums, strict image ingress validation, and MinIO/S3-compatible implementations.                                                                                           |
| Native adapter           | [crates/adapters/native/](../crates/adapters/native/)                                     | First-party generated questions and static flat-question compilation.                                                                                                                             |
| External adapters        | [crates/adapters/](../crates/adapters/)                                                   | Bounded QTI, H5P, iMathAS, and WeBWorK integration behind declared capabilities.                                                                                                                  |
| Server                   | [crates/server/](../crates/server/)                                                       | Axum routes, passwordless auth, authorization, learner aggregate and per-item redaction, adapter selection, API composition, ordinary worker, and public-asset publisher process.                 |
| Browser and WebAssembly  | [src/](../src/) and [crates/wasm/](../crates/wasm/)                                       | SolidJS interaction layer, strict browser decoder/editor boundary, and answer-free browser bridge to shared domain logic.                                                                         |
| Export and project tools | [crates/export/](../crates/export/) and [crates/project-tools/](../crates/project-tools/) | Print/export generation plus repository-only code generation, migration, fixture, pilot-content validation, and seed commands.                                                                    |
| Deployment configuration | `deploy/opentofu/`                                                                        | AWS network, edge, compute, database, storage, IAM, observability, and WAF definitions.                                                                                                           |

The Cargo dependency graph is a security control: `crates/domain/` depends only
on the question model, while `crates/wasm/` does not depend on `crates/grading/`.
The server composition root selects concrete stores, object backends, identity
providers, and adapters.

Course-term ownership is deliberately vertical and singular. `question_model` validates exact
calendar dates, ordered inclusive bounds, and case-sensitive IANA membership; `CourseRecord` and
`CourseSummary` require that value. Memory and PostgreSQL Store implementations carry the same
value, the existing course routes serialize it, generated TypeScript owns the response shape, and
the Solid course form supplies it explicitly. PostgreSQL adds native date/date/text columns to the
existing course row and treats an invalid stored value as unavailable rather than inventing a
fallback. Assignment dates remain absolute instants associated with a term-bearing course; this
slice does not resolve local wall times, daylight-saving transitions, or schedule shifts.

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
operations in transactions with tenant context and row-level security, while
the in-memory store supplies the same behavior for conformance testing.

The effective-policy resolver is a separate pure domain boundary. It consumes the entitlement
decision and evaluator-approved scopes supplied by S5, then applies lifecycle, entitlement, and
authorization gates in order before resolving the base policy, approved group modifiers, and an
individual exception. Both Store backends construct that same grant-filtered input for current
resolution and action paths. PostgreSQL alone persists the sealed per-attempt receipt and normalized
per-field provenance as immutable historical attempt evidence. S4 disclosure
projections instead use the current S3-resolved effective-policy verdict with
the current assignment-owned disclosure policy; no attempt-level receipt is a
learner-disclosure authority.

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
`assignment_policy.ts` under
[src/api/decoders/](../src/api/decoders/) strictly decodes the five-field
policy for authoring, while
[src/pages/assignment_editor_policy_panel.tsx](../src/pages/assignment_editor_policy_panel.tsx)
owns the instructor's native five-select editor controls.

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

[containers/compose.yaml](../containers/compose.yaml) defines the local stack:
gateway, API, ordinary worker, PostgreSQL, MinIO, a private renderer, and
one-shot setup services. The gateway is the browser entry point. Local
PostgreSQL and MinIO retain named-volume state; application services are
replaceable. [LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) document its topology
and operation.

`python3 local_stack.py` is the repository-anchored operator front door for inspection,
start, stop, restart, reset, validation, logs, and the aggregate live browser
acceptance handoff. Its `local_stack_control/` package owns typed Compose
provider selection, environment-file metadata and inherited-environment
sanitization, label-based Podman discovery, semantic service status, and
project-scoped cleanup plans. Focused Python modules own lifecycle sequencing:
`lifecycle.py` coordinates typed start, validation, and restart requests;
`local_environment.py`, `local_identity.py`, and `private_files.py` own default-only
private state; `private_state.py` owns mode-0700 repository-target run directories,
replacement-resistant descriptor access, and cross-process cleanup receipts for remote Podman
bind sources; `renderer.py` owns selected-renderer OCI configuration-ID provenance;
and lifecycle validation, waits, and diagnostics retain semantic readiness and safe failures.

`local_stack_control/chapter_one.py` owns the Chapter 1 subprocess boundary and protected,
same-directory atomic manifest publication used by both canonical Python E2E runners. The typed
lifecycle owner delegates this publication without reimplementing replay or temporary-file state.

The controller discovers resources by Compose labels rather than generated
names. Read-only commands may inspect a named project. Default mutations are
restricted to the `containers` project. A separate closed disposable-owner
adapter (`python3 -m local_stack_control._consumer_cli`) forms
temporary E2E targets only from a private mode-0600 manifest and a runner-held
cleanup capability. The closed owners are `course-appearance`, `chapter-one-pilot`,
`database-baseline`, `chapter-one-browser`, `webwork-browser`, `wp-r2-host-seed-renderer`, and `replica-restart`; each fixes its
project namespace and Compose files before any action is formed. The adapter
allows scoped Compose actions, diagnostics, or the policy-declared outage action,
while cleanup requires the private capability and label-derived
snapshot. It cannot accept a caller-selected target or generic removal command.
The canonical walkthrough imports the controller's discovery and cleanup
primitives while retaining its separate private inputs, fixed-port checks,
visible-action evidence, and report boundary.

`python3 local_stack.py acceptance` is the public aggregate acceptance
entry point. It delegates stack-conflict preflight and child-environment
sanitization to the controller, then invokes `local_stack_control/acceptance_lanes.py`. That Python
module keeps the fixed fail-fast browser and real-stack sequence but does not duplicate lifecycle
policy. The retained shell validation-lane entry point is only a compatibility `exec` facade.

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
- [tests/playwright/](../tests/playwright/) exercises built browser behavior and
  accessibility over HTTP. Its normal suite is distinct from the opt-in live
  aggregate and lane runner.
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
