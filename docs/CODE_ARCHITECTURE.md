# Code architecture

## Overview

Peptidyle Learning Engine (PLE) is a question-agnostic learning platform. A
SolidJS browser client renders answer-free server projections. Rust owns
identity, authorization, course and assignment state, grading, and durable
records. PostgreSQL is the production persistence authority; the in-memory
Store is a deterministic contract-test adapter, not a runtime fallback.

The course model has one reusable source aggregate and one delivery aggregate:

```text
BlueprintCourse (reusable, revisioned, answer-free source)
  +- ordered BlueprintModule
     \`- ordered BlueprintAssignment
        \`- exact published QuestionRevisionReference pins

CourseInstance (private teaching aggregate)
  +- immutable BlueprintCourse parent and applied revision
  +- copied definitions and resolved delivery settings
  \`- Students, releases, deadlines, accommodations, grades, and activity
```

`BlueprintCourse` has no Students, live deadlines, releases, accommodations,
grades, or delivery settings. Drafts are private to their owner and authorized
workspace collaborators. An explicitly published projection is visible and
reusable by every vetted Instructor. `CourseInstance` is private to its
current equal Teaching Team Members and enrolled Students. Every instance has exactly
one immutable Blueprint parent and records the applied Blueprint revision.

The former product-level Alpha/Blueprint split is not part of this architecture.
The former names and routes appear only as SD1 migration inputs where a cleanup
needs to identify legacy source, generated output, or immutable historical SQL.

## System shape

```text
browser
  SolidJS application + answer-free Rust WebAssembly
        |
        | same-origin HTTPS and strict browser-safe JSON
        v
gateway/load balancer -> Rust API -> PostgreSQL
                          |          |
                          |          \`-> forced RLS and capability brokers
                          v
                     workers and typed object domains
                          |
                          \`-> private question engines and public-asset publisher
```

The browser and Wasm facade format values, validate non-secret response shape,
and display server decisions. They never receive answer keys, grading payloads,
private source, or authority to create a course record. The API authenticates
and preflights the active session before decoding protected path, query, or body
values. PostgreSQL rechecks Account, workspace, course, Student, and worker lease
scope under forced row-level security.

## Course boundaries

### BlueprintCourse

`crates/question_model/src/blueprint_course.rs` owns the reusable meaning.
The aggregate contains a title, reviewed Question Authorship/publication state, one strong
revision, ordered modules, ordered Blueprint Assignments, policy defaults,
relative schedule defaults, evidence context, and public Question ID members.
The Store resolves each public Question ID to an exact immutable publication pin
before committing a whole-definition replacement.

Relative availability, due, and close values retain calendar-day and local-wall-
clock meaning. They are defaults, not live deadlines. A semantic no-op keeps the
revision; a stale or invalid replacement changes nothing. Publishing and later
controlled parent updates are explicit actions and never silently tether a
CourseInstance to a moving source.

### CourseInstance

`CourseInstance` is created or updated through the separate Blueprint-operation boundary.
`crates/question_model/src/blueprint_operations.rs` owns source observations,
target-term schedule resolution, previews, commands, provenance, receipts, and
Apply Blueprint Update semantics. Create Course from Blueprint copies reusable
definitions, policy/theme defaults, reviewed offsets, and normalized manifests
into an exact destination `CourseId`; it never copies Students, invitations,
Course Memberships, accommodations, Assignment Attempts, Student Responses, grades, or issued
evidence.

New assignments added to a BlueprintCourse propagate to daughter instances as
unreleased definitions. An Instructor explicitly releases each new delivery
assignment through the CourseInstance boundary. Delivery edits remain private
to the instance. Untouched imports may fast-forward before issued work;
divergent assignments require explicit selected copying or a new source-derived
assignment. Copy Course for New Term and Shift Course Dates are separate Course Instance operations.

## Major components

| Component           | Canonical owner                                         | Responsibility                                                                                                                                    |
| ------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Question model      | `crates/question_model/`                                | BlueprintCourse tree, typed references, exact question identities, assignment meaning, adoption commands, previews, and browser-safe projections. |
| Domain              | `crates/domain/`                                        | Pure timing, policy, disclosure, run, scoring, generation, and validation behavior without database or wall-clock reads.                          |
| Grading             | `crates/grading/`                                       | Answer-bearing checkers and correctness decisions; server-only and outside the Wasm dependency closure.                                           |
| Store contracts     | `crates/learning-data-access/src/contracts/`            | `BlueprintCourseStore` for BlueprintCourse and `CurriculumAdoptionStore` for source-to-instance operations.                                       |
| Retired Memory seam | `crates/learning-data-access/src/in_memory/`            | Unmounted legacy source being removed during the direct PostgreSQL cutover; it is not a Store implementation or a production selection path.      |
| PostgreSQL Store    | `crates/learning-data-access/src/postgres/`             | Production persistence, transaction locks, source re-resolution, broker calls, and RLS-backed projections.                                        |
| Server              | `crates/server/src/`                                    | Authentication, preflight, route binding, HTTP policy, Store composition, worker composition, and answer-free response assembly.                  |
| Generated contracts | `crates/project-tools/src/tsgen.rs` -> `generated/api/` | Derivative TypeScript DTOs generated from Rust contract roots; generated files are not hand-edited.                                               |
| Browser             | `src/`                                                  | Strict decoding, route/page state, BlueprintCourse editing and discovery, adoption previews, and visible CourseInstance decisions.                |
| Object storage      | `crates/objects/`                                       | Typed keys, checksums, image ingress, and the `public-assets`, `private-content`, `student-records`, and `temp-processing` domains.               |
| Adapters            | `crates/adapters/`                                      | Bounded PLE, QTI, H5P, iMathAS, and WeBWorK Question Backends behind declared capabilities.                                                       |

The server composition root is `crates/server/src/composition/`. It selects
PostgreSQL, object storage, identity, adapters, worker capabilities, and the
public-asset publisher. It does not select a product-level alternate curriculum
implementation.

## Persistence ownership

`crates/learning-data-access/src/contracts/blueprint_course.rs` exposes one
BlueprintCourse capability: list, get, replace, publish/lifecycle projection,
and delete where the lifecycle permits it. `crates/learning-data-access/src/in_memory/blueprint_course.rs`
implements the same aggregate for conformance. The production implementation
is `crates/learning-data-access/src/postgres/blueprint_course.rs`, with SQL
decoding, complete-tree validation, cursor paging, and authorization-aware
projections.

The current Question Model owns the Blueprint-operation transport contracts;
there is no mounted learning-data-access Store implementation yet. A future
Store must preserve immutable operation receipts separately from repairable
current projections. Reconciliation may repair only the derived projection.

The fresh SD1 migration epoch is owned by the allocation in
[implementation_status.md](active_plans/implementation_status.md). The
course/curriculum capability range is planned within `2026082913` through
`2026082916`; broker, forced-RLS, grants, and acceptance helpers are within
`2026082929` through `2026082932`. The exact active migration ledger remains
status-owned. The immutable
`2026081837_blueprint_alpha_curriculum.sql` and accepted successor migrations
are historical implementation evidence and are not edited to disguise the old
split.

## End-to-end data flow

```text
Instructor creates or edits BlueprintCourse
  -> question_model validates ordered tree and relative defaults
  -> Store resolves public Question IDs to exact published pins
  -> PostgreSQL broker authorizes workspace or vetted-Instructor projection
  -> strict server DTO and generated TypeScript projection
  -> browser edits a local draft and sends a strong-revision command
  -> CurriculumAdoptionStore previews target CourseInstance materialization
  -> server resolves term/zone and DST corrections
  -> atomic apply binds one Blueprint revision to one CourseId
  -> CourseInstance owns delivery, student records, release, and grading state
```

The picker selects a nested Blueprint assignment by typed reference plus module
and assignment positions. Assignment instantiation targets an existing exact
CourseInstance; Create Course from Blueprint creates a new CourseInstance. Forking
creates an independent BlueprintCourse with immutable source lineage. None of
these operations grants private CourseInstance or FERPA authority to a public
Blueprint projection.

## Assessment and delivery boundary

Published Questions remain shared Question Library content. Course assignments, Student
records, runs, attempts, responses, grades, accommodations, and issued evidence
are exact CourseInstance records. The normal assessment flow is:

```text
published immutable question
  -> CourseInstance assignment and entitlement
  -> server-issued answer-free attempt presentation
  -> accepted immutable Student response
  -> sealed worker grading and receipt
  -> current policy-controlled score and feedback projection
```

`crates/grading/` remains server-only. `crates/wasm/` may validate response
format and timing inputs, but it cannot grade or authorize. External engines
are brokered server-side and their output is treated as untrusted input.

## Browser and runtime topology

`src/api/` decodes every response from `unknown`; `src/features/` and
`src/pages/` own visible behavior; `crates/wasm/` provides the one browser-safe
Rust bridge. The production `dist/` artifact is served through the same-origin
gateway in the disposable live-demo owner. PostgreSQL, MinIO, API, worker,
gateway, and private renderer compose the local production-shaped topology.

`local_stack_control/` owns typed lifecycle, readiness, leases, scoped cleanup,
and acceptance composition. It is operational infrastructure, not a second
application architecture. `deploy/opentofu/` describes the AWS target but does
not prove that production activation has occurred.

## Testing and verification

Validation follows [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md):

- Permanent Rust and Node tests protect BlueprintCourse normalization, ordering,
  exact pins, authority, strict DTO decoding, adoption previews, CourseInstance
  exclusions, unreleased propagation, and answer-free projections.
- Memory conformance proves the same reusable and adoption behavior without a
  runtime storage selector.
- Disposable PostgreSQL/RLS oracles prove broker-only authority, forced RLS,
  vetted-Instructor published reads, workspace and CourseInstance privacy,
  exact source revision and CourseId binding, rollback, and idempotency.
- Production HTTPS Playwright proves visible authoring, nested picker reuse,
  Fork Blueprint Course, Copy Assignment from Blueprint, Create Course from Blueprint, DST correction, explicit
  release of propagated assignments, Apply Blueprint Update, and divergence recovery.
- Graphify, source inventories, migration registration, generated-file
  regeneration, and screenshot publication are one-time implementation
  evidence. They are not recurring pytest or Node gates.
- Rendered screenshots and independent visual/accessibility review remain
  separate human evidence; artifact counts and pixel identity are not behavior
  gates.

A focused documentation check may validate Markdown links, ASCII compliance,
and whitespace. Documentation success does not close an unrun required runtime,
database, browser, or human-review gate.

## Dependency order

The single-installation plan is the dependency authority:

```text
SD1-A decisions and inventory
  -> SD1-B Rust domain and authorization contracts
     -> SD1-C fresh PostgreSQL schema, RLS, and brokers
        -> SD1-D Memory and PostgreSQL Store implementations
           -> SD1-E server, workers, objects, and adapters
              -> SD1-F API, generated TypeScript, browser, and live demo
                 -> SD1-G connected evidence and release closure
```

Within the BlueprintCourse work, freeze the Rust question-model contracts and
typed source/destination operations first. Then implement Memory conformance,
the status-allocated SQL epoch, PostgreSQL parity, server routes and policy,
generated TypeScript, strict browser decoders and clients, and finally the
visible workspace/adoption flow. Run focused permanent tests at each boundary;
connected PostgreSQL, Playwright, screenshot, and human-review evidence comes
after the owning implementation is coherent.

## Extension points

- Add reusable course meaning in the Rust question-model BlueprintCourse
  contracts, then update both Store implementations and conformance behavior.
- Add a delivery operation in `blueprint_operations` with an explicit source,
  destination, revision, authorization, preview, apply, and receipt contract.
- Add PostgreSQL schema only through the status-owned forward migration
  allocation; preserve applied migrations as history.
- Add server routes under the owning capability module and register them through
  the composition root and route policy.
- Regenerate `generated/api/` from Rust after contract changes, then update
  strict decoders and typed browser clients.
- Add browser behavior in the owning API client, feature, page, or component;
  keep CourseInstance delivery decisions server-authoritative.
- Add external question behavior through an adapter with a safe public
  projection and a server-only grading handoff.

## Known gaps

- The current working source still contains paired legacy reusable-curriculum
  symbols and routes. Their removal is an SD1 dependency-ordered migration
  across question model, Store, SQL, server, generated output, and browser.
- The fresh SD1 migration epoch and its exact per-file ledger remain owned by
  `implementation_status.md`; this document does not allocate migrations.
- Production AWS activation, external provider attestation, institutional
  FERPA/legal sign-off, and human pilot acceptance remain separate release
  evidence classes.
