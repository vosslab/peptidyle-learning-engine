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
  verdicts, feedback disclosure, completion, and grade changes remain in Rust
  server code and durable storage. [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md)
  and [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) define this
  boundary.
- **Compile-time answer separation.** `crates/grading/` is outside the
  dependency closure of `crates/wasm/`. The browser bridge therefore cannot
  import answer-bearing checkers. [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md)
  defines adapter responsibilities.
- **Actor-scoped learner access.** Learner routes use learner-scoped store
  operations. Each operation verifies the acting user, active student
  membership, enrollment, and attempt or run relationship in the same store
  boundary; a route-level ownership check is not the sole control.
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

| Component | Location | Responsibility |
| --- | --- | --- |
| Question model | [crates/question_model/](../crates/question_model/) | Question taxonomy, identifiers, capabilities, browser-safe presentations, and response schemas. |
| Domain | [crates/domain/](../crates/domain/) | Run state, policy evaluation, seeded generation, timing inputs, and validation without a database or wall clock. |
| Grading | [crates/grading/](../crates/grading/) | Answer-bearing checkers and correctness decisions; server-side only. |
| Learning data access | [crates/learning-data-access/](../crates/learning-data-access/) | Store contracts, in-memory and PostgreSQL implementations, migrations, forced RLS context, capability roles, and conformance tests. |
| Object storage | [crates/objects/](../crates/objects/) | Typed object keys, checksums, strict image ingress validation, and MinIO/S3-compatible implementations. |
| Native adapter | [crates/adapters/native/](../crates/adapters/native/) | First-party generated questions and static flat-question compilation. |
| External adapters | [crates/adapters/](../crates/adapters/) | Bounded QTI, H5P, iMathAS, and WeBWorK integration behind declared capabilities. |
| Server | [crates/server/](../crates/server/) | Axum routes, passwordless auth, authorization, adapter selection, API composition, ordinary worker, and public-asset publisher process. |
| Browser and WebAssembly | [src/](../src/) and [crates/wasm/](../crates/wasm/) | SolidJS interaction layer and answer-free browser bridge to shared domain logic. |
| Export and project tools | [crates/export/](../crates/export/) and [crates/project-tools/](../crates/project-tools/) | Print/export generation plus repository-only code generation, migration, fixture, pilot-content validation, and seed commands. |
| Deployment configuration | `deploy/opentofu/` | AWS network, edge, compute, database, storage, IAM, observability, and WAF definitions. |

The Cargo dependency graph is a security control: `crates/domain/` depends only
on the question model, while `crates/wasm/` does not depend on `crates/grading/`.
The server composition root selects concrete stores, object backends, identity
providers, and adapters.

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
  -> API returns only feedback allowed by assignment policy
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
accounts. Course membership and enrollment determine course authority.

The data-access Store contracts make authority explicit at the persistence
boundary. Learner operations accept an actor and require active learner access;
Instructor-history operations use their own contracts. PostgreSQL evaluates those
operations in transactions with tenant context and row-level security, while
the in-memory store supplies the same behavior for conformance testing.

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
project-scoped cleanup plans. It is deliberately a controller rather than a
second implementation of local-stack bootstrap: private `local_stack_control/launch.sh`
remains the authority for builds, secret bootstrap, migrations, seed
publication, renderer checks, and launcher-level readiness.

The controller discovers resources by Compose labels rather than generated
names. Read-only commands may inspect a named project. Default mutations are
restricted to the `containers` project. A separate closed disposable-owner
adapter (`python3 -m local_stack_control._consumer_cli`) forms
temporary E2E targets only from a private mode-0600 manifest and a runner-held
cleanup capability. The closed owners are `course-appearance`, `chapter-one-pilot`,
`database-baseline`, `chapter-one-browser`, and `replica-restart`; each fixes its
project namespace and Compose files before any action is formed. The adapter
allows scoped Compose actions, diagnostics, or the one owner-specific replica
outage action, while cleanup requires the private capability and label-derived
snapshot. It cannot accept a caller-selected target or generic removal command.
The canonical walkthrough imports the controller's discovery and cleanup
primitives while retaining its separate private inputs, fixed-port checks,
visible-action evidence, and report boundary.

`python3 local_stack.py acceptance` is the public aggregate acceptance
entry point. It delegates stack-conflict preflight and child-environment
sanitization to the controller, then invokes the internal ordered lane runner
under `tests/playwright/`. The lane runner keeps browser-test sequencing but
does not duplicate lifecycle policy.

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
  `python3 local_stack.py`; keep bootstrap and teaching-data initialization in
  private `local_stack_control/launch.sh`.
- Add a disposable E2E consumer by declaring a closed owner policy and private
  manifest contract in `local_stack_control/consumer.py`, rather than adding a
  general project or cleanup flag.

## Known gaps

- Verify the deployed AWS account's DNS, ACM certificates, Secrets Manager
  values, database login provisioning, backup recovery, alerting, and incident
  procedures before production use.
- Verify each enabled external provider and renderer image against its live
  protocol, egress rule, authentication, and operational recovery contract.
