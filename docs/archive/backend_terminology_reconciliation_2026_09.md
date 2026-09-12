# Plan: Reconcile the PLE backend with the current terminology contract

## Context

`docs/HUMAN_GUIDANCE.md` and `docs/TERMINOLOGY_CONTRACT.md` now define a simpler domain model: only published reusable Question and Blueprint content has Revisions. Mutable working state uses current records and subject-specific Edit Numbers where concurrency requires them. Student Work and operational evidence remain separate immutable records containing the exact facts used.

The backend still contains four retired revision families-Course Schedule Revision, Assignment Revision and its entry snapshots, Question Change Proposal Revision, and Course Retention Plan Revision-plus Rust contracts, SQL functions, API responses, fixtures, tests, and documentation built around them. This is a pre-production system without durable production data, so the correct response is a clean schema and implementation redesign rather than compatibility machinery.

The repository has substantial uncommitted Interface Cleanup work touching some of the same Assignment, Course, time-zone, API, and test surfaces. Execution must preserve and integrate those changes.

## Objectives

- Make Question Revision and Blueprint Revision the only backend Revision concepts.
- Represent Assignment, Course Instance, Blueprint Draft, availability, Question Change Proposal, retention, Account Time Zone, and other configuration as current state.
- Preserve exact Question Revision and Blueprint Revision provenance wherever published content or Student Work depends on it.
- Make Assignment release, Released-state editing, Attempt creation, and Unrelease conform to the new evidence model.
- Remove Course Time Zone and make Account Time Zone the sole local-time interpretation and display authority.
- Produce a fresh, internally consistent schema, Rust model, API, fixture corpus, and end-to-end test suite without compatibility layers.

## Design philosophy

Apply the repository's KISS and "Fix Things Right" principles: replace the invalid revision architecture with direct current-state and evidence records, rather than renaming snapshots or maintaining parallel old/new representations. The rejected alternative is a translation layer that preserves Assignment, schedule, proposal, or retention revisions beneath revised terminology.

Keep evidence explicit and domain-shaped. Attempts and Issued Questions copy the facts they need; they do not point to a generic Assignment snapshot or opaque policy blob.

- Evidence strategy for uncertain methods: use fresh-schema catalog checks, foreign-key inventory, representative mutation/Attempt scenarios, and exact before/after Student Work counts. If retained observation data cannot reproduce Question Statistics exactly, extend the immutable observation facts before enabling Unrelease.

## Scope

- Rewrite baseline migrations and downstream SQL so retired revision structures are never created.
- Replace retired Rust domain types, store contracts, routes, validation, generated transport types, fixtures, and tests.
- Make current Assignment content pin exact Question Revisions.
- Implement current-state Release, valid Released-state saves, Attempt evidence capture, Unrelease impact reporting, and atomic Unrelease deletion.
- Move Published Question and Blueprint Course Availability to their stable lineages.
- Introduce one current Blueprint Draft with a Blueprint Draft Edit Number and explicit publication.
- Convert Question Change Proposal and Course Retention Plan storage to current records with Edit Numbers.
- Remove Course Time Zone and expose authenticated Account Time Zone read/write behavior.
- Reconcile Course creation, Question selection, Blueprint selection, Course Origin, grading, presentation, and statistics with the new model.
- Update current architecture, database, contract, test-evidence, naming, API, instructor, active-plan, and changelog documentation.
- Run focused and integrated backend acceptance against a freshly created database.

## Non-goals

- Preserve existing disposable database contents or migration checksums.
- Add compatibility views, aliases, adapters, dual-write paths, or legacy serialized shapes.
- Redesign unrelated frontend workflows beyond the contract/client changes required to exercise the backend.
- Change the immutable semantics of Question Revision or Blueprint Revision.
- Add speculative Course Instance Edit Numbers or accommodation APIs where no concurrent mutable operation exists.
- Build unimplemented Blueprint copy/update workflows merely to retain their obsolete scaffolding.
- Change Close or Archive into destructive operations; only Unrelease removes Student Work.

## Current state summary

- `course_schedule_revision`, `assignment_revision` and four Assignment revision-entry tables, `question_change_proposal_revision`, and `course_retention_plan_revision` are the complete set of retired schema-level Revision concepts found by the terminology sweep.
- Assignment release currently manufactures an immutable Assignment Revision and points `released_assignment_revision_id` at it. Attempt creation, delivery, completion, gradebook, and presentation then reread that revision.
- Current Assignment authoring already has an Assignment Edit Number and current fixed-question selection, but selection stores only the stable Question ID and resolves the latest revision during release.
- Unrelease is absent, while Student Work spans Attempts, Issued Questions, Question Attempts, submissions, grading records, presentation evidence, backend sessions, and statistics observations.
- Published Question and Blueprint availability are currently revision-scoped instead of lineage-scoped.
- Blueprint PUT currently creates another immutable Blueprint Revision; no current Blueprint Draft record exists.
- Account Time Zone storage and IANA validation exist, but Course Term and schedule contracts still carry Course Time Zone.
- Question Change Proposal and Course Retention Plan revision structures are schema scaffolding without complete product routes.
- Unused Blueprint-operation and grading-operation contracts retain Assignment, schedule, and operation revision concepts even though no live server workflow consumes them.
- Active Interface Cleanup changes overlap Assignment, Course, time-zone, generated API, fixtures, and E2E files; destructive Git cleanup is prohibited.

## Architecture boundaries and ownership

- The PostgreSQL schema owns current-state/evidence separation, transaction boundaries, referential integrity, RLS, and destructive Unrelease authority.
- `question_model` owns canonical Edit Number, status, availability, schedule, and retained-evidence types.
- `learning-data-access` owns parameterized SQL and atomic store operations; routes must not reconstruct cross-table invariants.
- `server` owns strict DTOs, authentication, concealed authorization failures, ETags, status codes, and cache policy.
- Attempt and Issued Question records own all facts needed to interpret existing Student Work.
- Generated TypeScript and minimal frontend adapters mirror the final Rust wire contract but do not dictate the domain model.
- The integrator owns shared migration ordering, server composition, generated artifacts, live-demo fixtures, and conflict resolution with the dirty Interface Cleanup work.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M1 / WS-FS | PostgreSQL schema foundation | Fresh schema contains only canonical Revision concepts |
| M1 / WS-FD | Rust domain foundation | Retired types are absent and `question_model` passes |
| M2 / WS-PQ | Published Question availability | Stable-lineage availability and exact historical resolution |
| M2 / WS-PB | Blueprint Draft and availability | Draft saves and publication are distinct operations |
| M3 / WS-CF | Proposal and retention current state | Current records plus exact immutable events/jobs |
| M3 / WS-TZ | Account Time Zone and Course Term | No Course Time Zone remains |
| M4 / WS-AS | Assignment and Student evidence | Release/edit/Attempt/Unrelease transaction boundary |
| M5 / WS-IV | Integration and verification | Clients, fixtures, full gates, docs, and review |

### Public interfaces

| Area | Final contract |
| --- | --- |
| Assignment | Existing list, GET, PUT, preview, and release-validation routes remain. `POST .../release` requires `If-Match`, creates no Revision, and returns `200` with the updated Assignment and ETag. |
| Unrelease | Add `GET .../unrelease-impact` and `POST .../unrelease`. POST requires `If-Match` and strict `{ "assignmentTitle": "exact title" }`; it returns the updated Assignment plus authoritative deletion counts. |
| Published Question | Search and pickers expose only Available lineages. Add owner-only `POST /api/questions/by-id/{question_id}/archive` and `/restore` with strict confirmation and a current-view ETag. |
| Blueprint Course | Keep published list/GET. Replace revision-creating base PUT with GET/PUT `/api/course-blueprints/{reference}/draft`; add POST `/publish`, `/archive`, and `/restore`. |
| Account Time Zone | Add authenticated GET/PUT `/api/account/time-zone`; PUT accepts only `{ "timeZone": "<IANA identifier>" }` and requires `If-Match`. |
| Errors | Use `428` for missing preconditions, `412` for stale ETags, concealed `404` for foreign resources, `409` for invalid transitions or confirmation mismatch, and `422` for semantic validation failures. |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Canonical data foundation | Remove retired schema and domain concepts and establish current-state/evidence types | Make later vertical slices build on one authoritative model |
| M2 | Published reusable content | Move availability to stable lineages and separate Blueprint Draft saves from publication | Preserve exact revisions without revisioning mutable working state |
| M3 | Current configuration and time | Convert proposal, retention, Course Term, and Account Time Zone behavior | Remove remaining configuration and schedule revision assumptions |
| M4 | Assignment and evidence lifecycle | Rebuild release, Released editing, Attempt capture, and Unrelease | Make mutable Assignment state safe without losing historical Student evidence |
| M5 | Integration and close-out | Reconcile clients, fixtures, tests, docs, and independent review | Prove the new backend end to end and remove stale terminology |

### Milestone M1: Canonical data foundation

- Depends on: none - the governing documentation and retired-concept inventory are complete.
- Deliverables: rewritten fresh-schema structures, canonical Rust types, removed dead revision scaffolding, and behavior-based catalog checks.
- Workstreams: WS-FS, WS-FD.
- Entry criteria: preserve the current dirty-work inventory and record overlapping files for integration.
- Exit criteria:
  - A fresh database contains Revision structures only for Question and Blueprint content.
  - `question_model` contains no Assignment, Course Schedule, Proposal, Retention Plan, or generic operation Revision type.
  - Current-state tables expose required Edit Numbers and immutable evidence tables contain copied facts.
  - The narrow schema and domain gates pass.
  - Update `docs/CHANGELOG.md`.
- Parallel-plan ready: yes - max parallel doers: 2, because SQL foundation and Rust-domain cleanup have separate ownership and generated artifacts remain integrator-owned.

### Milestone M2: Published reusable content

- Depends on: M1 - availability and draft implementations require the canonical roots and Edit Number types.
- Deliverables: lineage-level availability, immutable availability events, Blueprint Draft persistence, explicit Blueprint publication, archive/restore routes, and updated selection validation.
- Workstreams: WS-PQ, WS-PB.
- Entry criteria: fresh-schema and domain-foundation gates pass.
- Exit criteria:
  - Publishing does not reset availability.
  - Archive removes content from new-use paths without breaking exact historical pins.
  - Blueprint draft saves create no Blueprint Revision.
  - Each successful publish creates exactly one Blueprint Revision.
  - Course creation rejects an Archived Blueprint Course but preserves existing Course Origin resolution.
  - Focused Question Library and Blueprint Course E2E pass.
  - Update `docs/CHANGELOG.md`.
- Parallel-plan ready: yes - max parallel doers: 2, with Question and Blueprint routes/stores owned separately and shared composition handed to the integrator.

### Milestone M3: Current configuration and time

- Depends on: M1 - the current-state and evidence conventions must be established first.
- Deliverables: current Question Change Proposal and Course Retention Plan records, exact events/jobs, current Course Term dates, Account Time Zone route and Edit Number, and removal of Course Time Zone.
- Workstreams: WS-CF, WS-TZ.
- Entry criteria: canonical foundation merged.
- Exit criteria:
  - Proposal edits use CAS and events retain the exact acted-on proposal facts.
  - Retention jobs/events retain exact effective settings without referencing a mutable plan.
  - Course creation and Assignment schedule input use the acting Instructor's Account Time Zone.
  - Instructor and Student responses render stored instants in their respective Account Time Zones.
  - No schema or wire field owns a Course Time Zone.
  - Focused catalog, time-zone, DST, and Course Instance gates pass.
  - Update `docs/CHANGELOG.md`.
- Parallel-plan ready: yes - max parallel doers: 2, because configuration evidence and time-zone/Course Term components do not share route or domain ownership.

### Milestone M4: Assignment and evidence lifecycle

- Depends on: M2 and M3 - Assignment selection needs stable availability, while schedule capture needs the Account Time Zone model.
- Deliverables: current Assignment content, Released-state validation, revision-free Release, explicit Attempt/Issued Question evidence, Unrelease impact and action routes, deletion authority, and statistics correction.
- Workstreams: WS-AS.
- Entry criteria: Published Question availability and Account Time Zone gates pass.
- Exit criteria:
  - Current selections pin exact Question Revisions and never silently advance.
  - Valid Released saves succeed and invalid Released saves fail atomically.
  - Existing Attempts remain unchanged after Assignment edits; later Attempts use current state.
  - Release advances the Assignment Edit Number without creating a Revision.
  - Unrelease deletes the complete Student Work graph atomically and corrects affected statistics.
  - Close and Archive preserve Student Work.
  - Focused Assignment release, Attempt, delivery, completion, gradebook, and Unrelease E2E pass.
  - Update `docs/CHANGELOG.md`.
- Parallel-plan ready: no - the Assignment row, Attempt-start transaction, delivery functions, deletion graph, and statistics corrections form one tightly coupled consistency boundary.

### Milestone M5: Integration and close-out

- Depends on: M2, M3, and M4 - integration must validate the complete domain cutover.
- Deliverables: regenerated contracts, minimal client adjustments, reconciled fixtures, full test evidence, documentation, semantic audit, and independent review.
- Workstreams: WS-IV.
- Entry criteria: every focused backend gate passes.
- Exit criteria:
  - The clean-database migration and full backend E2E suites pass.
  - Generated Rust/TypeScript contracts agree.
  - Current documentation contains no retired architectural claims.
  - The final semantic scan finds only legitimate Question Revision and Blueprint Revision usage outside historical records.
  - An independent reviewer finds no unresolved correctness, security, terminology, or evidence-retention blocker.
  - Record final results and limitations in `docs/CHANGELOG.md`.
- Parallel-plan ready: no - generated artifacts, active dirty work, cross-suite fixtures, final documentation, and completion claims require one integration owner.

## Workstream breakdown

### Workstream WS-FS: Fresh-schema foundation

- Goal: make the database instantiate only the current terminology model.
- Owner: expert_coder with PostgreSQL responsibility.
- Work packages: WP-FS1, WP-FS2.
- Interfaces:
  - Needs: governing terminology and the retired-concept inventory.
  - Provides: canonical tables, keys, functions, RLS boundaries, and migration acceptance for every later workstream.
- Review boundary, when modifying the repository: migrations and schema catalog tests only.

### Workstream WS-FD: Rust domain foundation

- Goal: make canonical types express current state, exact published revisions, and retained evidence.
- Owner: expert_coder with Rust responsibility.
- Work packages: WP-FD1.
- Interfaces:
  - Needs: final field mapping in this plan.
  - Provides: Edit Number, availability, draft, schedule, and evidence types to stores and routes.
- Review boundary, when modifying the repository: `question_model` and contract-generation source only.

### Workstream WS-PQ: Published Question availability

- Goal: attach current availability to the stable Published Question lineage.
- Owner: coder.
- Work packages: WP-PQ1.
- Interfaces:
  - Needs: current availability schema and exact Question Revision types.
  - Provides: browse/new-selection rules and exact archived-content resolution to Assignment and Blueprint work.
- Review boundary, when modifying the repository: Question Library schema/store/routes and focused tests.

### Workstream WS-PB: Blueprint Draft and availability

- Goal: distinguish mutable Blueprint Draft edits from immutable publication.
- Owner: expert_coder.
- Work packages: WP-PB1, WP-PB2.
- Interfaces:
  - Needs: stable Blueprint Course root and current availability conventions.
  - Provides: exact Blueprint Revisions and availability checks to Course creation.
- Review boundary, when modifying the repository: Blueprint schema/store/routes and focused tests.

### Workstream WS-CF: Current configuration evidence

- Goal: replace proposal and retention revisions with current rows and exact event/job facts.
- Owner: coder.
- Work packages: WP-CF1.
- Interfaces:
  - Needs: Edit Number conventions from M1.
  - Provides: schema contracts and tests; no speculative product routes.
- Review boundary, when modifying the repository: stewardship, retention, job, and audit schema.

### Workstream WS-TZ: Account Time Zone and Course Term

- Goal: remove Course Time Zone and use each authenticated Account's zone.
- Owner: expert_coder.
- Work packages: WP-TZ1.
- Interfaces:
  - Needs: existing Account Time Zone validation and account defaults.
  - Provides: local-time interpretation and display contracts to Course, Blueprint, and Assignment operations.
- Review boundary, when modifying the repository: Account, Course, schedule, store, route, and focused time tests.

### Workstream WS-AS: Assignment and Student evidence

- Goal: implement the complete current Assignment lifecycle and destructive Unrelease contract.
- Owner: expert_coder.
- Work packages: WP-AS1, WP-AS2, WP-AS3.
- Interfaces:
  - Needs: stable availability, Account Time Zone resolution, and canonical evidence schema.
  - Provides: exact Attempt and Issued Question evidence to presentation, grading, and analytics.
- Review boundary, when modifying the repository: Assignment-to-Student-Work transaction boundary.

### Workstream WS-IV: Integration and verification

- Goal: merge all vertical slices without overwriting active work and prove the final behavior.
- Owner: integrator.
- Work packages: WP-IV1, WP-IV2, WP-IV3.
- Interfaces:
  - Needs: all focused implementations and their evidence.
  - Provides: generated contracts, complete fixtures, final test receipts, documentation, and closure report.
- Review boundary, when modifying the repository: shared composition, generated artifacts, E2E corpus, and current docs.

## Work packages

### Work package WP-FS1: Rewrite retired schema structures

- Owner: expert_coder.
- Touch points: baseline migrations for Course delivery, Assignment attempts, stewardship, retention, jobs, and downstream SQL functions.
- Depends on: none.
- Acceptance criteria:
  - Fresh schema never creates retired revision tables or foreign keys.
  - Question Revision and Blueprint Revision structures remain unchanged except for stable-lineage availability integration.
  - No compatibility object recreates the retired model.
- Evidence or review, when useful:
  - Fresh-volume migration plus catalog assertions and a constrained schema terminology scan.
- Obvious follow-ons:
  - Replace downstream stored functions and indexes before declaring the schema package complete.

### Work package WP-FS2: Define current-state and retained-evidence records

- Owner: expert_coder.
- Touch points: Assignment content, Attempt, Issued Question, Blueprint Draft, availability, proposal, retention, Account Time Zone, jobs, and audit tables.
- Depends on: WP-FS1.
- Acceptance criteria:
  - Mutable records carry subject-specific Edit Numbers only where CAS is exercised.
  - Attempt/Issued Question/event/job records copy exact interpretation facts.
  - Student Work ownership supports complete trusted Unrelease deletion without granting ordinary DELETE access.
- Evidence or review, when useful:
  - Foreign-key graph review and schema tests for atomic ownership, immutability, and RLS.
- Obvious follow-ons:
  - Add every aggregate input required for exact Question Statistics recomputation.

### Work package WP-FD1: Remove retired Rust revision architecture

- Owner: expert_coder.
- Touch points: Assignment, Course, Blueprint operations, student work, preview, grading operations, library exports, and generated-contract inputs.
- Depends on: none.
- Acceptance criteria:
  - Remove retired Revision numbers, references, successor conflicts, and dead operation contracts.
  - Keep only Question and Blueprint Revision references.
  - `AssignmentAttempt` and `IssuedQuestion` expose explicit retained evidence.
  - `CourseTerm` contains dates only; schedule resolution accepts an Account Time Zone explicitly.
- Evidence or review, when useful:
  - `cargo test -p question_model` and a semantic type-name scan.
- Obvious follow-ons:
  - Leave generated TypeScript regeneration to WP-IV1.

### Work package WP-PQ1: Implement stable Published Question Availability

- Owner: coder.
- Touch points: publication schema, Question Library store/routes, picker queries, validation, fixtures, and focused tests.
- Depends on: WP-FS2, WP-FD1.
- Acceptance criteria:
  - Initial publication starts Available.
  - Archive/restore use owner authority, explicit confirmation, concurrency, audit, and non-cacheable responses.
  - Search and new selections require Available; existing exact pins and authorized historical resolution survive archive.
  - Publishing a new Question Revision preserves current availability.
- Evidence or review, when useful:
  - Question Library E2E covering archive, restore, stale ETag, foreign actor, publishing while archived, and exact resolution.
- Obvious follow-ons:
  - Provide the stable availability query consumed by Assignment and Blueprint validation.

### Work package WP-PB1: Implement current Blueprint Draft storage

- Owner: expert_coder.
- Touch points: Blueprint Course schema, model, store, and draft GET/PUT routes.
- Depends on: WP-FS2, WP-FD1.
- Acceptance criteria:
  - Each Blueprint Course owns one current draft with a Draft Edit Number.
  - Draft saves use CAS and create no Blueprint Revision.
  - Collaborator events target the stable draft and record the exact Draft Edit Number.
  - Newly introduced question pins require Available lineages; retained archived pins remain valid.
- Evidence or review, when useful:
  - Focused store/route tests for valid save, stale save, archived retained pin, and archived new pin.
- Obvious follow-ons:
  - Feed only validated saved draft content into WP-PB2.

### Work package WP-PB2: Implement Blueprint publication and availability

- Owner: expert_coder.
- Touch points: Blueprint publication, list/detail, archive/restore, Course creation validation, and Course Origin.
- Depends on: WP-PB1, WP-PQ1.
- Acceptance criteria:
  - Publish requires the current Draft ETag and a draft edit newer than the last published edit.
  - One publish creates exactly one Blueprint Revision and one publication event.
  - Initial create atomically establishes the lineage, draft, and Revision 1.
  - Availability persists across publication.
  - Archived Blueprint Courses cannot seed a new Course, while existing Course Origins retain exact Blueprint Revision resolution.
- Evidence or review, when useful:
  - Blueprint and Course Instance E2E, including duplicate publish and concurrent archive/publish cases.
- Obvious follow-ons:
  - Supply final DTOs to WP-IV1.

### Work package WP-CF1: Convert proposal and retention state

- Owner: coder.
- Touch points: stewardship, correction, retention, job, and audit schema and catalog tests.
- Depends on: WP-FS2, WP-FD1.
- Acceptance criteria:
  - A Question Change Proposal is one current row with an Edit Number.
  - Its immutable event copies the exact acted-on edit, base Question Revision, payload/checksum, validation, impact, and result.
  - A Course Retention Plan is one current row with an Edit Number.
  - Jobs and cleanup events copy exact effective values and never depend on later plan edits.
  - No product route is invented for workflows that remain unimplemented.
- Evidence or review, when useful:
  - CAS and exact-event catalog/transaction tests.
- Obvious follow-ons:
  - Remove obsolete job constraints and compatibility-oriented fixtures.

### Work package WP-TZ1: Make Account Time Zone authoritative

- Owner: expert_coder.
- Touch points: Account time-zone storage/store/routes, Course Instance schema/routes, Blueprint schedule resolution, Assignment DTOs, and time fixtures.
- Depends on: WP-FS2, WP-FD1.
- Acceptance criteria:
  - Course Term stores current dates directly and no Course Time Zone.
  - Account Time Zone GET/PUT supports both Instructor and Student accounts with validation and CAS.
  - Student defaulting from an Instructor occurs only at Student-account creation.
  - Ambiguous/nonexistent local times are rejected; stored instants are stable across preference changes.
  - Responses render instants in the requesting Account's zone.
- Evidence or review, when useful:
  - Unit and E2E coverage for two accounts in different zones and DST boundary cases.
- Obvious follow-ons:
  - Update every fixture and decoder still expecting `courseTimeZone`.

### Work package WP-AS1: Replace Assignment revisions with current content

- Owner: expert_coder.
- Touch points: Assignment tables, domain/store contracts, picker/save/release validation, and release route.
- Depends on: WP-PQ1, WP-TZ1.
- Acceptance criteria:
  - Current Assignment content stores exact Question Revision references.
  - Reusing an existing pin does not advance it; newly introduced pins require Available content.
  - All mutable Assignment changes, including status, advance one Assignment Edit Number.
  - Released saves succeed only when the resulting current state passes release validation.
  - Release performs `Unreleased -> Released`, creates no revision, and returns the updated ETag.
- Evidence or review, when useful:
  - Assignment create/save/release API and PostgreSQL E2E.
- Obvious follow-ons:
  - Remove all release-response revision fields from consumers.

### Work package WP-AS2: Capture exact Attempt and Issued Question evidence

- Owner: expert_coder.
- Touch points: Attempt-start transaction, delivery, presentation, completion, submissions, grading, progress, and analytics.
- Depends on: WP-AS1.
- Acceptance criteria:
  - Attempt start locks and validates the current Released Assignment.
  - Attempt records contain the exact Assignment-derived schedule and policy facts used.
  - Issued Questions contain exact Question Revision, seed, order, points, scoring/statistics behavior, pool provenance, and limits.
  - Resume, completion, grading, and display read retained evidence rather than mutable Assignment state.
- Evidence or review, when useful:
  - E2E comparison showing an old Attempt unchanged after Assignment edits and a new Attempt using the edits.
- Obvious follow-ons:
  - Delete all remaining joins from Student delivery to retired Assignment revision structures.

### Work package WP-AS3: Implement atomic Unrelease

- Owner: expert_coder.
- Touch points: Assignment store/routes, Student Work ownership, grading/evidence deletion, statistics receipts, and audit.
- Depends on: WP-AS2.
- Acceptance criteria:
  - Impact GET returns current title, Edit Number, status, and typed advisory counts.
  - POST rechecks authority, status, ETag, exact title, and actual counts under one Assignment lock.
  - Student Work and student-identifying derived evidence are completely removed.
  - Affected Question Statistics are recomputed from remaining normalized observations.
  - The status transition and deletion either commit together or leave all state unchanged.
  - The remaining audit event is redacted and aggregate-only.
- Evidence or review, when useful:
  - Focused concurrent E2E plus independent authorization/data-deletion review.
- Obvious follow-ons:
  - Re-release and create fresh Student Work in the same acceptance scenario.

### Work package WP-IV1: Reconcile shared contracts and fixtures

- Owner: integrator.
- Touch points: server composition, generated TypeScript, decoders, minimal UI consumers, live-demo seeding, SQL fixtures, and changed active Interface Cleanup files.
- Depends on: WP-PB2, WP-CF1, WP-TZ1, WP-AS3.
- Acceptance criteria:
  - Rust and TypeScript wire shapes agree.
  - No client expects Assignment Revision, Course Time Zone, or implicit Blueprint publication.
  - Fixtures use current records and exact evidence.
  - Existing unrelated dirty changes remain intact.
- Evidence or review, when useful:
  - Generated-output diff review, TypeScript build/tests, and focused fixture replay.
- Obvious follow-ons:
  - Remove compatibility-named tests or rewrite them as current-contract behavior tests.

### Work package WP-IV2: Run integrated acceptance and semantic audit

- Owner: tester.
- Touch points: migration acceptance, Rust/TypeScript/Python suites, backend E2E, and terminology scan.
- Depends on: WP-IV1.
- Acceptance criteria:
  - All required automated and connected backend gates pass.
  - Exact completed, skipped, and failed gates are recorded honestly.
  - The semantic scan has no unexplained retired Revision usage.
- Evidence or review, when useful:
  - Command receipts and an independent reviewer's finding report.
- Obvious follow-ons:
  - Route any failure to its owning work package; do not weaken assertions to close the plan.

### Work package WP-IV3: Close documentation and reporting

- Owner: integrator.
- Touch points: active plan, changelog, architecture/database/contracts/test-evidence/naming/API/instructor docs, generated repository indexes.
- Depends on: WP-IV2.
- Acceptance criteria:
  - Current documentation describes only the implemented current-state/evidence model.
  - Authority documents remain unchanged unless execution discovers a real contradiction.
  - Changelog claims name the exact gates completed and any gate not run.
  - Deleted symbols do not survive in generated repository maps.
- Evidence or review, when useful:
  - Documentation link/style checks and final semantic scan.
- Obvious follow-ons:
  - Mark the active plan complete and close the revision-concerns note with links to final evidence.

## Acceptance criteria and gates

- Per-patch gate: run the narrowest deterministic schema, Rust, API, or fixture tests owned by that patch; review `git diff --check`; update `docs/CHANGELOG.md` before beginning another bounded patch.
- Integration gate:
  - Fresh database migrates successfully and a second migration pass is a no-op.
  - Rust formatting, workspace check, clippy, and workspace tests pass.
  - Generated contracts are current and TypeScript build/tests pass.
  - Python tests run through `source source_me.sh && python3`.
  - Focused Question Library, Blueprint Course, Course Instance, Account Time Zone, Assignment release, Attempt, delivery, completion, gradebook, and Unrelease E2E pass.
  - The full backend E2E runner passes against the rebuilt stack.
- Independent review gate:
  - A reviewer verifies the terminology boundary, exact-provenance retention, released-save behavior, and absence of generic snapshots.
  - A security-focused reviewer verifies Unrelease authority, atomic deletion, FERPA/course isolation, redacted audit, strict DTOs, and safe failure behavior.
  - Any blocker or high-severity finding returns to its owning work package and is rerun through focused and integrated gates.

## Test and verification strategy

- Replace source-inventory tests that enshrine retired structures with behavior-focused tests.
- Keep permanent tests deterministic, offline, fast, and contract-oriented; use catalog tests only for durable schema invariants.
- Classify clean-stack migration and connected HTTP/PostgreSQL workflows as explicit E2E evidence.
- Required Assignment scenarios:
  - release without creating a revision;
  - valid and invalid Released saves;
  - retained exact Question pin despite a newer publication;
  - archived retained pin versus archived new selection;
  - old Attempt unchanged and new Attempt updated;
  - Unrelease impact, stale ETag, wrong title, wrong status, foreign Course, concurrent Attempt start, rollback, complete deletion, statistics correction, re-release, and new work.
- Required published-content scenarios:
  - archive, restore, stale ETag, foreign actor, browse filtering, exact historical resolution, availability persistence across publication, and Course Origin preservation.
- Required Blueprint scenarios:
  - draft save without publication, stale draft save, one-revision publication, duplicate-publish refusal, retained archived pins, and Archived Blueprint Course creation refusal.
- Required time scenarios:
  - Instructor and Student in different IANA zones, DST gap, DST ambiguity, Account Time Zone update, unchanged stored instants, and absent Course Time Zone fields.
- Required proposal/retention scenarios:
  - stale Edit Number refusal and immutable event/job reconstruction after later current-state edits.
- Finish with a one-time semantic audit across active code, schema, fixtures, tests, generated contracts, and current docs. Historical changelog/archive prose may retain old terms when clearly historical.

## Migration and compatibility policy

- Rewrite the earliest applicable baseline migrations and all downstream references so a fresh schema never creates retired concepts.
- Do not append create-then-drop migrations, compatibility views, aliases, fallback readers, dual writes, serialized-version branches, or ledger edits.
- Treat migration checksum mismatch in an existing disposable stack as the expected signal to recreate that stack using the repository's bounded local-stack workflow.
- Do not delete broad directories or volumes with unresolved targets. Resolve the exact disposable database/volume first.
- Preserve changed-checksum detection as an acceptance feature even though the working database is rebuilt.
- Publish one current PLE wire shape; update all in-repository consumers in the same patch sequence.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Incomplete Unrelease deletion | High | Any Student-identifying row remains or a foreign-key failure occurs | WS-AS owner | Inventory the full ownership graph, test seeded records in every branch, use one trusted transaction, and independently review |
| Incorrect Question Statistics after deletion | High | Aggregates differ from remaining observation facts | WS-AS owner | Normalize every required observation input first, then recompute affected revisions inside the Unrelease transaction |
| Lost or overwritten Interface Cleanup work | High | Diff unexpectedly removes unrelated changes | Integrator | Record overlapping files, inspect current hunks before edits, avoid reset/checkout, and resolve conflicts deliberately |
| Silent Question Revision advancement | High | Existing Assignment or Blueprint pins change after publication | WS-PQ/WS-AS owners | Carry exact references through DTOs and compare retained versus newly introduced pins transactionally |
| Time-zone change alters deadlines | High | Stored timestamp changes after Account preference update | WS-TZ owner | Store instants independently, test before/after equality, and confine zone use to local input/output |
| Blueprint save still publishes implicitly | Medium | A draft PUT creates a Blueprint Revision | WS-PB owner | Separate draft and publish store operations and assert revision counts |
| Schema cutover leaves stale consumers | Medium | Compile, generated-contract, or fixture failures mention retired fields | Integrator | Centralize generated artifacts and fixture reconciliation in WP-IV1 |
| Scope expands into absent product features | Medium | Work starts adding accommodation or copy/update routes | Manager | Apply the Scope/Non-goals boundary and delete unused scaffolding without inventing replacement workflows |
| Documentation overstates connected evidence | Medium | Close-out claims a gate that lacks a receipt | Tester/integrator | Record exact completed and skipped gates and require independent review before closure |

## Rollout and release checklist

- [ ] Record the current dirty-work inventory and ownership boundaries.
- [ ] Complete M1 and pass fresh-schema/domain gates.
- [ ] Complete M2 and M3 through their independent focused gates.
- [ ] Complete the serial M4 Assignment/evidence cutover.
- [ ] Reconcile generated contracts, fixtures, and active Interface Cleanup consumers.
- [ ] Resolve the exact disposable database target and recreate it through the repository workflow.
- [ ] Run clean migration, second-pass no-op, focused E2E, and full backend acceptance.
- [ ] Run semantic, correctness, and security reviews.
- [ ] Update current documentation and regenerate repository indexes.
- [ ] Record final evidence and limitations in the changelog.
- [ ] Close the active plan only when every required gate has an observable result.

## Documentation close-out requirements

- Active plan / progress tracker updates: publish this plan under `docs/active_plans/active/`, record work-package status and evidence as execution proceeds, and coordinate explicitly with the active Interface Cleanup plan.
- `docs/CHANGELOG.md` entry: add one bounded entry after each completed patch and a final integrated entry naming the fresh-schema, Rust, TypeScript, Python, and E2E evidence.
- Current documentation: reconcile contract, architecture, database, test-evidence, naming, API, and instructor guidance with the implemented model.
- Authority preservation: do not edit `docs/HUMAN_GUIDANCE.md` or `docs/TERMINOLOGY_CONTRACT.md` unless implementation uncovers a genuine contradiction requiring user resolution.
- Archive / closure notes: close `docs/active_plans/revision_concerns.txt` with links to the implemented patches and final test evidence; regenerate Graphify output so deleted nodes do not remain visible.

## Patch plan and reporting format

- Patch 1: schema - remove retired revision structures and establish current-state/evidence tables.
- Patch 2: domain - remove retired Rust contracts and define canonical Edit Number/evidence types.
- Patch 3: Published Question - stable availability, archive/restore, and selection rules.
- Patch 4: Blueprint Course - current draft, explicit publication, availability, and Course creation rules.
- Patch 5: configuration and time - proposal, retention, Account Time Zone, and current Course Term.
- Patch 6: Assignment - current content, exact Question pins, Released saves, and revision-free release.
- Patch 7: Student evidence - Attempt capture, delivery consumers, atomic Unrelease, and statistics correction.
- Patch 8: integration - generated contracts, minimal frontend consumers, fixtures, and full acceptance.
- Patch 9: documentation - current authorities, changelog evidence, active-plan closure, and regenerated indexes.

Each patch report must state: owned behavior, files/components touched, narrow gates run, exact outcomes, unresolved findings, and the next dependency unlocked.

## Resolved decisions

- Rewrite baseline migrations directly and recreate disposable databases.
- Preserve only Question Revision and Blueprint Revision.
- Use explicit Attempt and Issued Question fields rather than a generic snapshot or policy JSON.
- Make Assignment status part of current state governed by the Assignment Edit Number.
- Allow valid Released-state edits to affect only later Attempts.
- Make Unrelease the sole destructive Assignment status action.
- Store Published Question and Blueprint Course Availability on their stable lineages.
- Keep initial Blueprint creation as an explicit create-and-publish transaction, then require separate draft-save and publish operations.
- Allow publishing an Archived Blueprint Course without restoring it; continue to prohibit new Course creation from it.
- Remove unused Blueprint copy/update and grading-operation revision scaffolding without constructing replacement APIs.
- Add an authenticated Account Time Zone mutation route and remove Course Time Zone entirely.
- Preserve Course Origin's exact Blueprint Revision provenance.

## Open questions and decisions needed

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: architect for terminology classification; expert_coder plus reviewer for Student Work deletion completeness.
  - Evidence and decision rule: for every newly discovered `Revision` term, preserve it only if it identifies immutable published Question/Blueprint content or an exact reference to that content. Convert genuine mutable concurrency to a subject-specific Edit Number; delete unused revision-only architecture. For Unrelease, treat the foreign-key inventory and seeded deletion oracle as authoritative; if an aggregate cannot be reconstructed exactly, extend its immutable input facts before deletion is enabled.
- Non-blocking follow-up: future draft-only Blueprint creation, accommodation management, and Blueprint copy/update product workflows may be planned separately after this backend contract is complete.
