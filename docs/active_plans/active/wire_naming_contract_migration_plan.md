# Plan: PLE wire naming contract migration

## Context

`WP-INST-WN1` is the corrective prerequisite for implemented, acceptance-open `WP-INST-G2`.
The current pre-WN1 material tree still carries lower-camel PLE transport in many closures. The
approved target is one direct `Foo` whose PLE data properties equal effective Rust Serde
`snake_case`, from route and model producers through generated TypeScript, strict browser readers,
Wasm/adapter PLE bridges, and current durable PLE data.

The system is stable enough for this contract migration: the independent WN1-A reviews identified
and closed allocation gaps rather than an unresolved product failure. Fresh v3 review accepted the
revised [implementation_status.md](../implementation_status.md) allocation on 2026-08-28. `WP-INST-WN1-B` is the
current implementation package.

## Objectives

- Make Rust Serde the sole source for snake PLE properties, query keys, and portable values.
- Generate one direct per-type `Foo` with exactly the effective Serde names.
- Give route-only DTOs one pure `crates/browser-api-contract` owner.
- Move each producer, reader, fixture, and durable record family in an atomic child package.
- Retire the legacy human manual-grading product surface before C3 while preserving automated
  grader-exception retry/recalculation and roster score export.
- Supply item-analysis wire/client/decoder ownership in C4, then deliver the visible Instructor
  workflow as `WP-INST-G3-IA1` after WN1-F, accepted G2, and D1.
- Preserve accepted migrations and historical evidence while rebuilding current live-stack data
  from canonical producers.

## Naming and architecture boundary

| Concern                            | Canonical owner                     | Required result                                                                                  |
| ---------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------ |
| PLE properties and portable values | Rust Serde                          | `snake_case`, strict mutating records, one effective spelling                                    |
| Generated TypeScript DTO           | `project-tools` `tsgen` modules     | One direct `Foo`; data properties and literals equal effective Serde                             |
| Route-only request/response DTO    | pure `browser-api-contract`         | No Axum, Store, server runtime, persistence, or application state dependency                     |
| Server boundary                    | owning route/projection module      | One mapping from domain/Store state into the direct DTO                                          |
| Browser boundary                   | owning feature client/decoder       | Direct strict snake DTO plus semantic, disclosure, range, relationship, and opaque-ID checks     |
| Durable PLE data                   | named producer/reader/version owner | Clean rebuild, named forward version/migration, or frozen history exactly as the ledger declares |
| External protocol                  | registered/upstream owner           | Existing HTTP, URL, DOM, wasm-bindgen, WebAuthn, QTI, H5P, LTI, IMathAS, or WeBWorK spelling     |

TypeScript functions, locals, signals, and components retain TypeScript conventions. For example,
a local `rosterId` may read the direct DTO property `row.roster_id`. `src/api/contracts.ts` may
compose browser-only values while generated serializable contracts remain direct and canonical.

## Dependency flow

```text
accepted WN1-A
  -> WN1-OPS1 root-script naming
  -> WN1-B1..B5 foundation
  -> WN1-GO1 orphaned generated-output retirement after B5
  -> WN1-MG manual-grading retirement
  -> C1, C2, C4, C5, C6 route children and dependency-ready QM children
  -> C3 after WN1-MG
  -> affected WA and D children
  -> WN1-OPS2..OPS10 PLE-owned shell-family closures before final acceptance
  -> WN1-SR6 product-document review and WN1-FD filename disposition
  -> WN1-F final material-tree acceptance
  -> G2 W5/W6 close-out
  -> G3-IA1 visible item-analysis workflow
```

Parallel execution is valid only for child rows whose ledger dependencies and source ownership are
disjoint. Every child has one owner; shared generator, model, and durable files remain serialized.

## Work packages

### WN1-A: allocation and review

- Owner: architect.
- Outcome: the ledger allocates every matrix-identified Axum producer, every public serializable
  question-model source closure, each PLE query family, Wasm/adapter PLE bridges, and every durable
  record/artifact family.
- Evidence: Graphify navigation followed by current-source verification, durable disposition
  review, naming-convention review, and independent acceptance. These are one-time receipts.
- Exit: achieved 2026-08-28; fresh independent v3 review accepted the revised ledger.

### WN1-B: direct generator and pure contract foundation

- Owner: expert coder, with one named module owner per child.
- Depends on: accepted WN1-A.
- Outcome: create `browser-api-contract`; split the existing 969-line `tsgen.rs` into source-model,
  type-mapping, rendering, and orchestration modules; add two-root discovery and duplicate-name
  rejection; generate only direct `Foo` files.
- Sequence: `B1-contract-root -> B2-source-model -> B3-types -> B4-render -> B5-runner`.
- Permanent gate: focused project-tools and crate-boundary tests plus TypeScript compilation.
  Two-root regeneration, generated cleanup, and source-size review are one-time evidence.

### WN1-OPS1: root-script naming

- Owner: shell operations coder.
- Depends on: accepted WN1-A.
- Outcome: in `run_live_demo.sh`, convert `SCRIPT_DIRECTORY`, `COMMAND`, and `HEADLESS` to
  `script_directory`, `command`, and `headless`. In `build.sh`, convert `SCRIPT_DIRECTORY`,
  `PROFILE`, `STAGE_NAMES`, `STAGE_TIMES`, `BUILD_START`, `CARGO_PROFILE_FLAG`,
  `WASM_PROFILE_FLAG`, `BUILD_END`, and `TOTAL` to `script_directory`, `profile`, `stage_names`,
  `stage_times`, `build_start`, `cargo_profile_flag`, `wasm_profile_flag`, `build_end`, and
  `total`. Exported process configuration retains `SCREAMING_SNAKE_CASE`.
- Permanent gate: shell syntax plus each script's visible usage/build behavior. The root-script
  source inventory is one-time evidence.

### WN1-OPS2 through WN1-OPS10: remaining PLE-owned shell families

- Owner: one shell operations coder per child registered in the ledger.
- Depends on: accepted WN1-A; each family is otherwise independent and completes before WN1-F.
- Outcome: convert script-private state to lowercase `snake_case` in the root aggregate, browser
  front doors, Rust front door, Wasm build, Python setup, Wasm runner setup, course-appearance E2E,
  database-baseline E2E, and E2E orchestrator families. Preserve exported environment spelling,
  explicit `readonly` fixture constants, command behavior, and shared-template ownership.
- Permanent gate: each ledger row's existing shell syntax and visible behavior path. The uppercase
  private-name inventory is one-time evidence; no permanent source inventory is added.

### WN1-GO1: orphaned generated-output retirement

- Owner: generated-contract cleanup coder.
- Depends on: accepted WN1-B5.
- Outcome: remove the unconsumed legacy `ts-rs` pair under `crates/question_model/bindings/` and
  retain `crates/project-tools -> generated/api` as the sole active TypeScript contract owner.
  `QM-CAPABILITY` remains responsible for the separate capability-discriminant spelling change.
- Permanent gate: focused project-tools generator tests and TypeScript compilation. Regeneration,
  consumer search, and retired-path inventory are one-time evidence.

### WN1-A exact Student-role allocation

The binding current-name to target-name allocation is embedded in the
[implementation status registry](../implementation_status.md) under `WN1-SR3 exact run and Store
register`, `WN1-SR4 exact browser register`, and `WN1-SR5 exact PostgreSQL register`. These tables
are part of WN1-A, not optional supporting material. Implementation changes every listed name in
its one closure; it adds neither aliases nor parallel role vocabulary.

- SR3 names every public and `_impl` Store/RunStore method, `learner_submission_status`, run-screen
  model, Student-work/store type, module, server projection, and local/function target.
- SR4 names `LearnerAssignmentPresentationDelivery`, `LearnerAssignmentPresentationData`,
  `LearnerAssignmentPresentationProps`, `toLearnerAssignmentPresentationData`,
  `LearnerAssignmentPresentation`, `decodeLearnerDisclosurePolicy`,
  `decodeLearnerAssignmentSummary`, `decodeLearnerAssignmentDetail`,
  `decodeLearnerQuestionAttempt`, `decodeLearnerAssignmentProgress`,
  `decodeLearnerAssignmentPage`, and `decodeLearnerSubmissionStatus`, the class-statistics
  converter, and each browser client/runtime member target.
- SR5 maps every legacy student-work broker function to its exact successor target in the ledger,
  including assignment, attempt, audience, course, enrollment, group-member, group, member,
  prefetch, run, and summary operations. It also names the fence and the three exact current
  function names and targets in the ledger.

Fresh WN1-A review checked those embedded tables against current material source before WN1-B and
accepted them on 2026-08-28. The allocation receipt makes no implementation or test-acceptance claim.

### WN1-MG: legacy manual-grading retirement

- Owner: expert coder for the manual-evaluation route/Store/backend closure.
- Depends on: accepted WN1-A. C3 and the affected activity/grading/durable children depend on it.
- Outcome: retire the human-credit route and product closure atomically; preserve the existing
  answer-free `GradingOperationStore` retry, assignment recalculation, ordinary worker publication,
  current Gradebook, and roster score CSV export. A source-proven historical reader, if retained,
  receives a frozen-reader owner and no product mutation path.
- Permanent gate: exact automated retry route plus Store conformance and route-policy absence.
  Source retirement inventory is one-time evidence.

### WN1-C1 through C6: Axum route closures

- Owner: one coder per atomic child row in the ledger.
- Depends on: WN1-B5 and the row's listed QM dependencies; C3 also depends on WN1-MG.
- Outcome: project route-only types through `browser-api-contract`; change the producer, direct
  generated DTO, query parser/builder, browser client/decoder, fixture, and narrow Rust/Node test
  together. Each migrated PLE boundary accepts snake and rejects retired camel/unknown input.
- Fixed package scope:

| Package | Atomic child families                                                                                                                                                                                       |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1      | Calculated Gradebook, Student/operation selection, submitted-run chooser, audited detail, roster, roster import, roster score CSV export                                                                    |
| C2      | Session/logout, passwordless/account/email/invitation, seeded selector, PLE WebAuthn wrappers                                                                                                               |
| C3      | Run/attempt/prefetch/submit/status/summary/feedback, external-tool PLE wrapper, author preview, three validation fallbacks                                                                                  |
| C4      | Question Library browse/search/resolve/detail/publication; Question authoring workspace CRUD/validation/diff; flat assets/source/publication; item analysis; Question Folder/Saved Question Search curation |
| C5      | Curriculum preview/apply/inspection/reconciliation and PLE QTI import/conversion/publication wrappers                                                                                                       |
| C6      | Course/listing, grade scheme/totals/export, assignment workspace/delivery, grading operations, teaching authority/groups/preview                                                                            |

The [implementation status registry](../implementation_status.md) is the current route-by-route
authority. No C7 is created.

For C6, `course/routing.rs` retains shared nonserializing topology, state, and body-limit support.
`C6-CR1` owns `course/pagination.rs` with direct `CoursePageQuery`, plus
`course/course_listing.rs` for course list/create/get and term-failure projection. `C6-AS1` owns
`course/assignments/listing.rs` and `course/assignments/strict_request.rs`; it consumes the shared
`CoursePageQuery` and therefore depends on C6-CR1. This makes each PLE query and assignment request
one direct contract rather than leaving serialization in a shared routing module.

### WN1-C4-IA1 and `WP-INST-G3-IA1`

- WN1 owner: C4 item-analysis route-contract coder.
- WN1 outcome: add direct `CourseItemAnalysisResponse`, `AssignmentItemAnalysisResponse`, and
  `ItemAnalysisResponseDistribution` types to the pure route-contract crate; map the current
  server report once; add a strict decoder, same-origin/no-store client, API capability, and
  `tests/test_item_analysis_client.mjs`.
- Boundary: existing report production, worker, Store, SQL, publication generation, and private
  `QuestionVersionReference` remain with their current owners.
- Visible successor: `WP-INST-G3-IA1`, after WN1-F, accepted G2, and D1, adds the Instructor-only
  assignment workspace Analysis task. It joins aggregate item IDs to existing assignment titles
  and public Question IDs and links to audited Gradebook inspection, Library/source context, and
  Questions replacement. G3-IA1 adds no report SQL, score mutation, or G4 decision record.
- WN1 permanent gate: server projection tests plus the offline item-analysis client test. G3-IA1
  connected browser evidence and human visual review remain in the G3 package.

### WN1-QM: source/type closures

- Owner: one expert coder per ledger row.
- Depends on: WN1-B5 and the route projections that currently serialize the closure raw.
- Outcome: move the complete source/type graph to snake Serde and regenerate the same direct names.
  The fixed topics are `IDENTITY`, `CAPABILITY`, `LIFECYCLE`, `CATALOG`, `COURSE`, `CONTENT`,
  `ACTIVITY`, `STATS`, `PRESENTATION`, `ASSIGNMENT`, `CURRICULUM`, `TEACHING`, and
  `GRADING-OPS`.
- Permanent gate: focused Serde/semantic tests and the named strict direct-decoder tests in the
  ledger. Public-Serde inventory, Graphify, generated import review, and fixture counts are
  one-time evidence.

### WN1-WA: Wasm and adapter-local PLE JSON

- Owner: one expert coder for each `WASM`, `NATIVE`, `QTI`, `H5P`, and `PROVIDER-CACHE` child.
- Depends on: affected QM and route closures.
- Outcome: parse and stringify direct snake PLE values while retaining raw wasm-bindgen ABI and
  registered provider/package spelling.
- Permanent gate: Rust/Wasm parity or the adapter's deterministic parser/cache/replay/refusal
  tests. Connected providers and rendered review remain separate evidence.

### WN1-D: durable formats and clean rebuild

- Owner: one expert coder per durable ledger child; release integrator owns migration allocation.
- Depends on: the affected producer and reader closure.
- Outcome:

  - Current live-stack database/object rows rebuild on a clean volume from canonical Store, seed,
    publisher, cache, or export producers.
  - Accepted migrations `2026080801` through `2026081878` and checksums remain frozen.
  - Consecutive migrations `2026081879` through `2026081888` provide atomic forward SQL ownership:
    course-authority broker ownership; authority-function argument rebinding; Student-role schema
    vocabulary; Student-work broker vocabulary; automated-only scoring; Student-work payloads;
    canonical receipt V2; Question Library/Question authoring workspace payloads; Blueprint Course payloads; and operational payloads.
    They run on a clean volume with zero row backfill.
  - Canonical JSON, semantic/request digests, receipts, native/QTI/H5P records, object metadata,
    replay schemas, and seed vectors retain their existing named identity and receive an explicit
    forward version only for new records.
  - Pilot sources/provenance, accepted screenshot generations, external archive/source bytes, and
    accepted migration files remain frozen historical evidence.

- Permanent gate: owning producer/reader, migration-conformance, canonical-digest, replay, and
  deterministic fixture behavior. Migration review, clean-volume rebuild, regeneration, digest
  comparison, connected service runs, and visual inspection are one-time evidence.

The durable ledger gives each migration one producer/reader family. `1879` creates the bounded
`ple_course_authority_broker`; `1880` rebinds the four effective public authority functions and
their exact dependents while preserving current authorization behavior, explicit owners, ACLs,
`SECURITY DEFINER`, and `FORCE ROW LEVEL SECURITY`. Accepted migrations and immutable historical
records remain frozen; changed immutable PLE records receive a named forward version rather than a
byte rewrite.

### WN1-F: final acceptance and G2 handback

- Owner: integrator.
- Depends on: WN1-OPS1 through WN1-OPS10, WN1-GO1, WN1-SR6, WN1-FD, every C child, WN1-MG, and
  every applicable QM/WA/D child.
- Outcome: independent final ledger review, exact `docs/NAMING_CONVENTIONS.md` compliance review,
  final material-tree receipt, and handback to G2 W5/W6.
- Required final gate:

```bash
source source_me.sh && ./all_test.sh
```

Required unrun, skipped, or failed gates keep WN1 and G2 acceptance-open.

### WN1-SR6: live product documentation

- Owner: product-documentation coder.
- Depends on: the source closures whose role-bound terminology it describes.
- Outcome: convert person/role uses to Student in `docs/LIVE_DEMO_SPEC.md`,
  `docs/STUDENT_GUIDE.md`, `docs/INSTRUCTOR_GUIDE.md`, `docs/COOKBOOK.md`, `docs/FAQ.md`,
  `docs/API_CONTRACTS.md`, `docs/ENROLLMENT_DESIGN.md`, `docs/CODE_ARCHITECTURE.md`,
  `docs/FILE_STRUCTURE.md`, `docs/FRONTEND_ARCHITECTURE.md`,
  `docs/MASTERY_ASSIGNMENT_DESIGN.md`, `docs/DATA_CLASSIFICATION.md`,
  `docs/RETENTION_POLICY.md`, `docs/CACHING_AND_PREFETCH.md`,
  `docs/INSTRUCTOR_PAGE_VISUALS.md`, `docs/STUDENT_PAGE_VISUALS.md`,
  `docs/UI_DESIGN_REVIEW.md`, and `docs/LOCAL_STACK_OPERATIONS.md`. Retain generic learning and
  teaching language, `learning-*` system terms, registered external names, frozen historical
  material, and clearly labeled current-pre-WN1 evidence.
- Evidence: one-time independent documentation/material-tree review. No permanent text-inventory
  test is added.

### WN1-FD: active filename disposition

- Owner: documentation/files owner.
- Depends on: WN1-SR6.
- Outcome: register frozen historical filenames and allocate the active naming-document-audit
  exceptions: `docs/how-to-reduce-impact-of-bot-traffic.md`, `docs/QTI-JSON_OBJECT_FORMAT.md`,
  `docs/active_plans/Rust_SQLx_and_PostgreSQL_implementation.md`, `customer-spec.md`,
  `m0-results.md`, `m0-review.md`, `peptidyle-security-audit.md`,
  `peptidyle-walkthrough-plan.md`, and dated reports under `docs/active_plans/reports/`. The
  implementation owner applies each recorded canonical rename, archive/history move, or frozen
  registration under the repository move policy and updates in-tree links when applicable.
- Evidence: material-tree link validation is one-time. No broad filename inventory test is added.

## Naming-convention acceptance gate

The WN1-F reviewer verifies these exact boundaries:

1. PLE data-object/JSON properties, PLE query keys, and portable discriminants use `snake_case`.
2. Direct generated type names equal Rust type names; their properties equal effective Serde.
3. TypeScript functions, locals, signals, and components retain TypeScript conventions.
4. DOM/framework/dependency and registered protocol names retain owner conventions.
5. Migrated PLE inputs reject retired camel aliases and unknown properties.
6. Route-only DTOs remain pure `browser-api-contract` values and server projection occurs once.

This is an explicit one-time acceptance review. Permanent coverage remains deterministic offline
behavior rather than source inventories, generated-file counts, or style snapshots.

## Risk register

| Risk                        | Trigger                                                          | Owner           | Control                                                                      |
| --------------------------- | ---------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------- |
| Shared-model drift          | A route still serializes raw shared data while its model changes | C and QM owners | Land the route projection first, then move the complete source/type closure. |
| Generator instability       | New behavior is added to the 969-line monolith                   | WN1-B           | Complete the responsibility split and module-level gates first.              |
| Digest/history break        | Current spelling change alters retained bytes                    | D owner         | Keep the old named version and add a forward producer/reader version.        |
| External protocol drift     | Snake conversion reaches provider/package values                 | C/WA owner      | Project only the PLE wrapper and retain the registered seam.                 |
| Manual surface resurrection | C3 adds a codec/client for human grading                         | WN1-MG/C3       | Complete retirement first and keep automated retry as the positive route.    |
| Premature acceptance        | Focused or connected evidence is treated as final                | WN1-F           | Require all named lanes and the final material-tree command.                 |

## Documentation and evidence close-out

Each completed child updates the changelog after its narrow gate. The ledger and
`implementation_status.md` stay synchronized as the package/status authority. Inventories,
regeneration, independent review, clean-stack work, connected services, and visual inspection are
recorded as one-time evidence; deterministic offline behavior is retained as permanent coverage.
