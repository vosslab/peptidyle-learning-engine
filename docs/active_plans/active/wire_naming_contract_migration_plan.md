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
- Move each producer, reader, fixture, and associated durable record in an atomic child package.
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
  -> WN1-OPS2..OPS10 PLE-owned shell naming closures before final acceptance
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
  question-model source closure, each PLE query boundary, Wasm/adapter PLE bridges, and every durable
  record or artifact type.
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

### WN1-OPS2 through WN1-OPS10: remaining PLE-owned shell naming closures

- Owner: one shell operations coder per child registered in the ledger.
- Depends on: accepted WN1-A; each child package is otherwise independent and completes before WN1-F.
- Outcome: convert script-private state to lowercase `snake_case` in the root aggregate, browser
  front doors, Rust front door, Wasm build, Python setup, Wasm runner setup, course-appearance E2E,
  database-baseline E2E, and named E2E orchestrators. Preserve exported environment spelling,
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
[implementation status registry](../implementation_status.md) under `WN1-SR3 exact Assignment Attempt and Store
register`, `WN1-SR4 exact browser register`, and `WN1-SR5 exact PostgreSQL register`. These tables
are part of WN1-A, not optional supporting material. Implementation changes every listed name in
its one closure; it adds neither aliases nor parallel role vocabulary.

- SR3 names every public and `_impl` Assignment Attempt Store method, `learner_submission_status`, Assignment Attempt screen
  model, Student-work/store type, module, server projection, and local/function target.
- SR4 names `LearnerAssignmentPresentationDelivery`, `LearnerAssignmentPresentationData`,
  `LearnerAssignmentPresentationProps`, `toLearnerAssignmentPresentationData`,
  `LearnerAssignmentPresentation`, `decodeLearnerDisclosurePolicy`,
  `decodeLearnerAssignmentSummary`, `decodeLearnerAssignmentDetail`,
  `decodeLearnerQuestionAttempt`, `decodeLearnerAssignmentProgress`,
  `decodeLearnerAssignmentPage`, and `decodeLearnerSubmissionStatus`, the class-statistics
  converter, and each browser client/runtime member target.
- SR5 maps every legacy Student-work protected database function to its exact successor target in the ledger,
  including assignment, attempt, audience, course, enrollment, group-member, group, member,
  prefetch, Assignment Attempt, and summary operations. It also names the fence and the three exact current
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

| Package | Atomic child packages                                                                                                                                                                                                    |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1      | Calculated Gradebook, Student/operation selection, submitted Assignment Attempt chooser, audited detail, roster, roster import, roster score CSV export                                                                  |
| C2      | Session/logout, passwordless/account/email/invitation, seeded selector, PLE WebAuthn wrappers                                                                                                                            |
| C3      | Assignment Attempt/Question Attempt/prefetch/submit/status/summary/feedback, iMathAS Question Backend PLE wrapper, author preview, three validation fallbacks                                                            |
| C4      | Question Library browse/search/resolve/detail/publication; Question authoring workspace CRUD/validation/diff; PLE Question JSON assets/source/publication; item analysis; Question Folder/Saved Question Search curation |
| C5      | Curriculum preview/apply/inspection/reconciliation and PLE QTI import/conversion/publication wrappers                                                                                                                    |
| C6      | Course/listing, grade scheme/totals/export, assignment workspace/delivery, grading operations, teaching authority/groups/preview                                                                                         |

The [implementation status registry](../implementation_status.md) is the current route-by-route
authority. No C7 is created.

### WN1-C6-GO1: Instructor Grading Operation Retry Token

- Owner: one expert coder for the atomic grading-operations route/Store/receipt closure.
- Depends on: accepted `WN1-B5`, accepted `WN1-MG`, and the applicable
  `WN1-QM-GRADING-OPS` source-type closure; it precedes `WN1-F`.
- Outcome: make the server own one durable binding of an Instructor Grading Operation Retry Token
  to the exact Instructor Grading Operation, `retry` or `recalculate` action, Request Checksum, and
  accepted Receipt. Project it once through a route-only `browser-api-contract` DTO with effective
  Serde `retry_token`, regenerate the direct TypeScript contract, and change the strict decoder,
  same-origin client, and assignment-workspace intent together. A new Instructor decision creates
  one opaque token; ambiguous retry preserves it. The registered `idempotency-key` HTTP header
  remains protocol framing rather than a PLE value name.
- Boundaries: this child owns no manual grading, generic cross-operation retry abstraction,
  enrollment or Student-submission idempotency terms, compatibility camel alias, or browser-grading
  authority. It adds any required forward persistence shape only with its Store producer/reader;
  accepted migrations and immutable historical receipts remain unchanged.
- Permanent gate: focused Rust/Store behavior proves same-token same-request replay returns one
  equal accepted Receipt and no second side effect, while changed operation/action/checksum refuses
  the old binding; the named strict Node decoder/client and assignment-workspace model suites prove
  `retry_token` decoding, request/receipt equality, malformed/retired-field refusal, and ambiguous
  retry reuse. Run TypeScript compilation and the applicable generated-contract gate.
- Service gate: disposable PostgreSQL route/transaction evidence proves the unique durable binding,
  receipt replay, and authorization boundary. Browser mocks do not accept this service or
  persistence claim. Record `git diff --check`, direct generated/wire-shape inspection, and the
  package receipt before completing vocabulary rows 459-460.

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
  `QuestionRevisionReference` remain with their current owners.
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

### WN1-QM-PRESENTATION-COURSE-BANNER-INFORMATIVE-TEXT: Course Banner Informative Text

- Owner: one expert coder for the complete Course Appearance terminology closure.
- Depends on: accepted `WN1-B5`; it is independent of deferred Course Appearance persistence,
  service, and mounted-editor capability work, and precedes `WN1-F`.
- Outcome: replace the abbreviated validated-string type `CourseBannerAltText` with
  `CourseBannerInformativeText` across the Question Model, public facade, regenerated TypeScript,
  strict decoder, Course Banner renderer, focused fixtures/tests, and current documentation. Keep
  `CourseBannerAlternativeText` as the one closed Decorative-or-Informative accessibility-treatment
  type and retain JSON `alternativeText` as its existing canonical property name. Remove the old
  Rust export and generated `CourseBannerAltText.ts` directly; add no aliases, duplicate fields, or
  legacy decoder branch.
- Boundaries: this child changes no Course Banner object-address/storage ownership, schema,
  PostgreSQL migration, Store, route, authorization, or mounted editor. The absent Course Appearance
  revision/current-pointer capability remains separately allocated future work. The checklist row
  is completed only after this child's evidence is recorded.
- Permanent gate: `cargo test -p question_model course_appearance`; regenerate with `cargo tools
tsgen`; run `npx tsc --noEmit`; and run `node --import tsx --test
tests/test_course_theme_scope.mjs` after adding strict Decorative/Informative decoding and rendered
  `alt=""`/exact informative-text assertions. Run `cargo fmt --all --check` and `git diff --check`.
  A targeted active-owner search confirms that `CourseBannerAltText` is retired while
  `CourseBannerAlternativeText`, `CourseBannerInformativeText`, and `alternativeText` each retain
  their distinct canonical meanings. Generated-import inspection and the no-schema/no-route
  inventory are one-time evidence, not substitute service claims.

### WN1-QM-PRESENTATION-COURSE-APPEARANCE-VIEW: Course Appearance View

- Owner: one expert coder for the complete existing reader-boundary closure.
- Depends on: accepted `WN1-B5`, completed
  `WN1-QM-PRESENTATION-COURSE-BANNER-INFORMATIVE-TEXT`, and completed
  `WN1-QM-PRESENTATION-COURSE-BANNER-REFERENCE`; it precedes `WN1-F`.
- Outcome: replace the PLE-owned Course Appearance projection meaning with the direct reader
  object `CourseAppearanceView`: exactly `{ theme, revision, banner }`, where `theme` is
  `CourseTheme`, `revision` is `CourseAppearanceRevision`, and `banner` is absent or the
  independently closed `CourseBanner`. Complete that direct name through the existing Question
  Model/public facade, generated TypeScript declaration, strict browser decoder, route-reader/client
  contracts and consumers, focused fixtures/tests, and directly affected documentation. Retire the
  prior PLE-owned projection name directly; add no alias, dual DTO, or legacy decoder branch.
- Deferred boundary: this reader-only child creates no Course Appearance Store or retained record,
  schema/current-pointer relation, PostgreSQL migration, server route, authorization oracle, Course
  Banner Upload promotion/cleanup, or mounted editor. Low-level database/query projection remains
  distinct technical vocabulary. Those persistence and editor capabilities are deferred feature work,
  not evidence for this terminology closure.
- Permanent gate: `cargo test -p question_model course_appearance`; `cargo tools tsgen`; `npx tsc
--noEmit`; `node --import tsx --test tests/test_course_theme_scope.mjs`; `cargo fmt --all --check`;
  and `git diff --check`. Targeted active-owner and generated-import inspection are one-time
  evidence. Focused evidence and independent review are required before accepting the child and
  checking vocabulary row 469.

### WP-SD1-A-TERM-01-RQB1: accepted iMathAS backend terminology evidence

- **Accepted historical evidence (2026-09-01).** RQB1 established the `Question Backend` product
  category and its server-managed iMathAS Session, state, result, and launch/return boundary.
  RQB2 now directly replaces the remaining generic backend-session names with exact iMathAS ownership. iMathAS adapter-local
  records use iMathAS names; registered protocol claims, including `ple_launch_challenge`, retain
  their protocol spelling.
- **Superseded terminology evidence:** `ETLS1`, `ETLC1`, `ETGC1`, and `ETPRT1` retain their
  implementation receipts only as evidence pending this cutover; their checked terminology targets
  are not canonical. `ETRGR1` is absorbed here and cannot complete independently. RQB1 reopens and
  directly replaces their source, schema, API, generated-artifact, documentation, and validation
  language, including the row-535 result-to-grading lineage.
- **Owned model and inheritance:** `Question Revision -> Question Backend -> iMathAS Question
Backend Binding (Deployment Reference, Item Reference, pinned Profile) -> iMathAS Question Backend
Session -> Challenge, Authentication, Grading Context, encrypted State, Result Token, and Result
Exchange -> immutable iMathAS Result -> marker Question Submission -> Question
Submission Grading -> Job -> Grading Result -> Automated Grading Receipt`. Question Model owns the
  typed binding, LDA persists it as a Session fact, and the iMathAS adapter owns Launch State, cache,
  and protocol behavior. The Result Exchange inherits authority
  through its Session and introduces no duplicate learner, Course, Assignment, Question Attempt,
  Question Revision, Seed, or Question Grading Rule columns. The marker carries no backend bytes,
  token, or score.
- **Direct cutover:** RQB2 renames the Question Model response control/marker and generated browser
  contract to `ImathasQuestionBackend`; it renames the LDA aggregate, Store, iMathAS adapter boundary,
  SQL tables/functions/policies, service-oracle names, fixtures, and active documentation together.
  Use the shared typed iMathAS binding at every Question Model, LDA, adapter, and SQL boundary; name
  iMathAS-only transport records `Imathas*`. Remove the orphaned `lti_grade_return` table, trigger,
  function,
  policies, grants, and staged-database assertions. `LTI` stays only where it identifies a current
  registered protocol boundary; the current product has none. No alias, compatibility DTO, old wire
  value, old SQL procedure, transitional schema view, or successor migration is introduced.
- **Migration allocation:** rewrite both unshipped migrations directly. `2026082927` owns the
  iMathAS Question Backend cache/session/exchange schema and removes the orphaned LTI schema;
  `2026090102` owns the Session/Exchange lifecycle, encrypted-state fields, result-token checksum,
  marker Submission, pending ordinary-grading Job, worker-leased idempotent Grading Result and
  Automated Grading Receipt lineage. Neither migration is shared with a superseded package.
- **Dependency order:** (1) terminology contract/checklist allocation, (2) Question Model marker
  and generated contracts, (3) LDA and iMathAS adapter aggregate cutover, (4) both migrations and
  PostgreSQL Store procedures, (5) existing service oracle/staged schema checks, (6) active docs
  and one-time retirement review, then (7) focused gates, full aggregate, and independent review.
- **Permanent gates:** retain and rename the deterministic LDA and iMathAS behavior tests already
  present in the absorbed ETLS/ETRGR working tree for
  bounded/redacted state, protocol verification, exact Context/Authentication binding, one-use
  stage/replay, result-to-grading derivation, receipt checksum, and Memory lease behavior. They use
  production-generated capabilities with outcome-independent assertions. Use only those existing
  behavior tests as permanent evidence; keep source inventories and connected browser work in their
  one-time or service lanes.
- **Disposable and one-time gates:** rename and run the existing ignored PostgreSQL Store test,
  its explicit E2E runner, and the existing staged-database acceptance as service evidence; retain
  no real service connection in default Rust or pytest lanes. Use the vocabulary-count script and
  focused contextual searches as one-time closure evidence, not permanent tests. Run generated
  contracts, format, strict Clippy, documentation links/source-size gates, `git diff --check`, and
  `source source_me.sh && ./all_test.sh` on the final tree. Check the affected vocabulary rows only
  after those required gates and independent review accept the direct replacement.

### WN1-WA: Wasm and adapter-local PLE JSON

- Owner: one expert coder for each `WASM`, `PLE-QUESTION-JSON`, `QTI`, `H5P`, and `PROVIDER-CACHE` child.
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
  - Canonical JSON; documented semantic Digests used as identities, fingerprints, cache discriminators,
    or deduplication values; Request Checksums; receipts; PLE Question JSON/QTI/H5P records; object metadata,
    replay schemas; and seed vectors retain their canonical named identity and receive an explicit
    forward version only for new records.
  - Pilot sources/provenance, accepted screenshot generations, external archive/source bytes, and
    accepted migration files remain frozen historical evidence.

- Permanent gate: owning producer/reader, migration-conformance, deterministic encoding, request-checksum, replay, and
  deterministic fixture behavior. Migration review, clean-volume rebuild, regeneration, digest
  comparison, connected service runs, and visual inspection are one-time evidence.

The durable ledger gives each migration one named producer and reader. `1879` creates the bounded
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
