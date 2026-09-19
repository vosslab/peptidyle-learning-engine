# Plan: Replace Blueprint Drafts with save-created Blueprint Revisions

## Context

Blueprint Course authoring currently uses two persisted version-like values: a mutable Blueprint
Draft Edit Number and an immutable Blueprint Revision Number. The schema stores parallel Draft and
Revision aggregates, the server exposes separate save and publish routes, and the browser asks the
Instructor to save a Draft and then publish a Revision. That model makes it unclear which value
identifies the reusable course content that a Course Instance receives.

Human Guidance now establishes one Blueprint content version sequence. The Instructor makes any
number of local edits, then one explicit Save creates one complete immutable Blueprint Revision.
The exact current Revision is also the save precondition. A Course Instance always pins one exact
Blueprint Revision and never reads mutable Blueprint working state.

Current evidence confirms that this is a direct pre-production redesign:

- `schemas/base_schema/50_functions/blueprints.sql` stores one mutable Draft aggregate beside the immutable
  Revision aggregate and implements separate create, save, and publish functions.
- `crates/server/src/blueprint_course.rs` exposes `/draft` and `/publish` routes with a Draft Edit
  Number ETag.
- `src/features/blueprint_course/blueprint_course_workspace.tsx` already keeps unsaved edits in
  browser state, but its Save and Publish actions establish two persistence boundaries.
- `schemas/base_schema/50_functions/course_operations.sql` already creates Course Instances from an exact
  caller-supplied Blueprint Revision.

## Objectives

- Give Blueprint Course content exactly one saved version sequence: Blueprint Revision Numbers.
- Make every semantically changed Blueprint Course Save create one complete immutable Revision;
  converge an unchanged Save on the current Revision without adding history.
- Replace the pre-production content-encoding history with one deterministic, unversioned canonical
  Blueprint-content encoding for checksums and no-op comparison.
- Use the exact current Blueprint Revision as the optimistic-concurrency precondition.
- Keep every Course Instance pinned to the exact Blueprint Revision selected at creation.
- Make the first accepted Blueprint creation produce a complete valid Revision 1 that is immediately
  visible and reusable by vetted Instructors.
- Treat one named Module containing one named Assignment with at least one Question-bearing entry as
  the intentional minimum server-saved Blueprint Course; every pin must resolve to a Published
  Question Revision.
- Protect unsaved browser edits from accidental navigation or window close.
- Replace the numeric Blueprint availability Edit Number with an opaque lineage metadata ETag so the
  Revision Number is the only Blueprint sequence exposed to Instructors.
- Remove the persisted Blueprint Draft domain object and its Draft Edit Number throughout the
  schema, Rust, generated browser contract, TypeScript client, interface, tests, and current docs.

## Design philosophy

Apply **fix the design, not the symptom** and **Keep It Simple, Stupid**. A saved reusable course
state is a Blueprint Revision; a local unsaved editor state is not another domain object. Network
retries converge on one save receipt. A changed Save creates one immutable Revision, while a
semantically unchanged Save returns the current Revision and keeps history meaningful.

- Evidence strategy for uncertain methods: use transaction-level concurrency tests and one focused
  built-browser journey to prove the save boundary, stale-editor behavior, and unsaved-change guard.

## Scope

- Replace persisted Blueprint Draft content with save-created Blueprint Revisions.
- Keep incomplete initial authoring local. Make the first valid Blueprint Course create request
  atomically create its stable lineage and complete Revision 1; because lineage availability
  defaults to Available, that accepted Revision is immediately visible and reusable by vetted
  Instructors. This plan deliberately favors a minimally reusable public Blueprint over server-saving
  an incomplete skeleton.
- Make Blueprint Course Save accept complete reusable structure and the exact current Revision,
  then create the next Revision only when that structure changed semantically.
- Replace `BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION`, its encoded `version` field, and every
  version-selection or compatibility path with one canonical Blueprint-content encoding. The
  pre-production base schema and disposable data have no compatibility obligation to old encodings.
- Move Blueprint short and long names out of revisioned content into lineage metadata. Rename and
  availability changes use an opaque metadata ETag and create no Blueprint Revision or visible Edit
  Number.
- Preserve stable Blueprint Module and Blueprint Assignment references across Revisions so future
  comparison and propagation can identify additions, removals, moves, and edits.
- Preserve the future propagation distinction: a new Blueprint Assignment is added to eligible
  Course Instances as Unreleased, while changes to an existing Assignment require the future
  explicit propagation-approval workflow.
- Keep Course Instance creation tied to the exact current Revision advertised for the selected
  Available Blueprint. Keep older Revision loading for existing provenance and audit; this plan does
  not add an older-Revision picker for new Course Instances.
- Replace Draft save/publish browser actions with one explicit Save action.
- Add dirty-state protection for Blueprint navigation, browser Back, reload, and window close.
- Align current authority and contract docs with the new lifecycle when implementation lands.
- Update installation data, connected fixtures, and focused tests that exercise Blueprint creation,
  authoring, reuse, and Course Instance provenance.

## Non-goals

- Build Blueprint-to-Course-Instance propagation; this plan preserves the exact Revision and stable
  child-identity evidence needed for the future new-Assignment and existing-Assignment behaviors.
- Modify content inside an existing Course Instance Assignment without the future explicit
  propagation-approval workflow, or modify any disseminated Assignment through Blueprint propagation.
- Change the Draft Question authoring and publication lifecycle.
- Add background Blueprint persistence, local crash recovery, or network-failure recovery. This
  plan ends at dirty-navigation and window-close protection; recovery remains a separate future
  editor-reliability project.
- Add currently unimplemented fork or Course-Instance-to-Blueprint user interfaces; their future
  create paths must still create a new Blueprint lineage and Revision 1 without a Draft.
- Add compatibility tables, routes, aliases, dual writes, or migrations for the retired Draft
  model; PLE remains pre-production and the canonical base schema is editable.
- Rewrite dated reports or archived changelog entries that accurately describe earlier evidence.

## Current state summary

- `ple_data.blueprint_course` is the stable lineage and owns availability.
- The lineage currently exposes a numeric Availability Edit Number for archive/restore concurrency;
  the redesign replaces it with an opaque metadata token shared by name and availability actions.
- Four `blueprint_draft*` tables store mutable content, Question pins, Modules, and Assignments.
- Four `blueprint_*revision*` tables store the corresponding immutable aggregate.
- Draft create and save receipts are separate from the Blueprint publication event.
- Rust exposes `BlueprintDraft`, `BlueprintDraftView`, `BlueprintDraftEditNumber`, and separate
  Create, Save, and Publish commands and receipts.
- The browser view contains an optional owner-only Draft plus an optional latest published
  Revision. Non-owners read only published content.
- Course Instance creation already receives and stores an exact Blueprint Revision Number.
- The current Course Instance creation interface offers only the latest Revision carried by each
  Available Blueprint summary; it has no historical-Revision picker.
- The Blueprint editor already tracks a local dirty flag and preserves local input on a conflict;
  it needs the repository's navigation/window-close protection at the new Save boundary.
- Initial Blueprint authoring already stays local until the browser and server accept a complete
  tree with at least one Module, one Assignment per Module, and one Question-bearing entry per
  Assignment.
- Domain, SQL, browser validation, and a focused UI test all enforce that complete-tree creation
  boundary. The resolved model retains the established minimum as a deliberate public-reuse rule.
- Current save persistence already reports `changed` and leaves its version unchanged when canonical
  content and checksum are unchanged. The Revision-only Save should preserve that convergent no-op
  behavior.
- The current canonical-content encoder carries a development-history encoding version. Because PLE
  is pre-production and the base schema and fixtures are disposable, the cutover removes that
  version number, encoded field, and compatibility selector instead of preserving a false history.
- Current replacement normalization already preserves submitted retained Module and Assignment
  references and allocates new references only for members declared New. Publication copies those
  same references into revision child rows.
- The current replacement normalizer resolves a retained Assignment only within its prior Module,
  even though the schema makes the Assignment reference unique across the Blueprint Course. The
  Revision-only contract must settle that mismatch in favor of course-wide durable identity.
- The current Blueprint title lives in Draft/Revision content even though Human Guidance makes
  Blueprint short and long names lineage metadata. Draft removal must relocate that field rather
  than copy the old ownership into every new Revision.

### Graphify scope audit

The Sep 12, 2026 Graphify map confirms that Blueprint Draft removal crosses four connected layers,
then current-source verification supplies the generated and SQL edges that Graphify cannot infer:

| Boundary      | Mapped and verified owners                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Persistence   | `schemas/base_schema/50_functions/blueprints.sql`, Blueprint installation data, and exact Course Instance foreign keys in `schemas/base_schema/50_functions/course_operations.sql`                                                                                                                                                                                                                                                      |
| Rust/domain   | `crates/question_model/src/blueprint_course.rs`, `crates/question_model/src/blueprint_course/`, `crates/question_model/src/blueprint_operations.rs`, `crates/question_model/src/blueprint_operations/`, `crates/question_model/src/lib.rs`, Blueprint and Course Instance learning-data-access owners and PostgreSQL adapters, their server routes, and `crates/browser-api-contract/src/blueprint_course.rs` |
| Browser       | Blueprint API contract/client/decoder, `blueprint_course_model.ts`, `blueprint_course_creation.ts`, create dialog, workspace, feature exports, Blueprint live pages, Course list, Assignment source repository, and Course Instance Revision selection                                                                                                                                                        |
| Evidence/docs | Blueprint client/model/UI tests, Blueprint and Course Instance service journeys, affected browser journeys, installation-data oracle, and every current durable doc found by the scoped Blueprint Draft terminology sweep                                                                                                                                                                                     |

Graphify also shows the workspace importing the model's replacement-content conversion, the HTTP
client importing the Blueprint API contract, and live pages importing the workspace. Its reverse
dependency traversal does not bridge generated Rust/TypeScript DTOs, so generated contract output
and every current caller remain explicit inspection gates rather than inferred omissions.

## Architecture boundaries and ownership

The stable `BlueprintCourse` lineage owns identity, ownership, names, availability, and its exact
current Blueprint Revision reference. The lineage contains no mutable reusable-content aggregate.
Names and availability remain lineage metadata because they control discovery and administration;
they do not change the reusable structure copied into a Course Instance. Any present or future field
needed to reconstruct or compare reusable Course Instance structure belongs in the Revision instead.
Lineage metadata uses purpose-specific opaque concurrency handling rather than a second user-facing
Blueprint content version number.

Creation accepts the lineage's short and long names beside the first reusable structure and writes
both ownership boundaries atomically. Later name or availability changes use a separate lineage
metadata command with one opaque metadata ETag. Short name, long name, and availability intentionally
share this boundary because they form the public listing metadata and archive confirmation validates
the current long name. A concurrent Rename and Archive therefore has one winner; the loser reloads
metadata without affecting reusable content. The token is an HTTP concurrency control, not a second
displayed version sequence; only Blueprint Revision Numbers identify saved reusable content. The
content editor's explicit Save handles reusable structure; a distinct Rename/Availability action
handles lineage metadata and cannot create a Blueprint Revision.

`BlueprintRevision` owns one complete immutable reusable structure: ordered Modules and Assignments,
stable child references, relative schedules, reusable defaults and policies, and exact Question
Revision pins. Revision 1 is created with the lineage only after the structure contains at least one
Module, one Assignment in every Module, and one Question-bearing entry in every Assignment; every
Question pin must resolve to a Published Question Revision. After all other content validation passes,
the new Available lineage immediately joins vetted-Instructor discovery and reuse.

A retained Module or Assignment reference means the same reusable child lineage across Revisions.
Reordering or moving that child retains its reference; deleting and later recreating similar content
creates a new reference, and a retired reference is never reused. Save validation resolves every
retained child anywhere in the expected head Revision and permits it exactly once. This intentional
identity contract supplies the future distinction between a new Assignment and a changed existing
Assignment.

A Save locks the lineage and verifies that `If-Match` names its exact current Blueprint Revision.
The server resolves retained/new child references and Question pins, then uses one deterministic,
unversioned canonical `BlueprintRevisionContent` encoding, `BlueprintContentChecksum`, and exact
stored aggregate comparison as the only change decision. The encoding contains no encoding-version
field, selector, fallback, or compatibility reader. When the canonical aggregate or checksum differs,
the transaction creates the next positive Revision Number,
inserts the complete child aggregate, records the adapted save receipt, and advances the lineage's
current Revision reference. When both are unchanged, it records or resolves the request receipt and
returns the current Revision with `changed: false`.

The HTTP representation returns the exact current `BlueprintRevisionReference` and a strong ETag
derived from that same Revision Number. The browser sends that ETag in `If-Match`; it does not send
a separate Edit Number in JSON. An exact idempotent retry returns the original result, including its
`changed` outcome. A distinct Save based on an older Revision fails without writing another Revision.
The representation also carries the opaque metadata ETag required only by Rename/Availability
actions; the interface never labels it as a Blueprint version or Edit Number.

Course Instance creation continues to accept an exact `BlueprintRevisionReference`. The interface
offers each Available Blueprint's advertised current Revision and submits that exact value; it does
not offer arbitrary historical Revisions. The server rechecks that the submitted Revision is still
the Blueprint head before creation, so a concurrent Blueprint Save yields a conflict instead of a
surprising older source. Persisted provenance remains exact, and multiple Course Instances may pin
the same Revision. Exact historical loading remains available for those existing provenance records.

Future propagation compares stable child references between the Instance-pinned Revision and a
newer Revision. A reference that appears for the first time represents a new Assignment and follows
the automatic Unreleased-Assignment rule for eligible Course Instances. A retained Assignment
reference whose content changed follows the future explicit propagation-approval workflow. Building
those transitions remains outside this plan; preserving enough immutable evidence to distinguish
them is inside it.

Fresh creation, future fork-from-Revision, and future creation from a Course Instance's reusable
structure all converge on one rule: create a new Blueprint Course lineage and its immutable
Revision 1 atomically. None creates a Blueprint Draft.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                              | Review boundary                                          |
| ---------------------- | -------------------------------------- | -------------------------------------------------------- |
| M1 / WS-D              | PostgreSQL Blueprint lifecycle         | Atomic lineage, Revision, receipt, and RLS contract      |
| M2 / WS-R              | Rust model, Store, and HTTP API        | Exact Revision CAS and Course Instance provenance        |
| M3 / WS-B              | TypeScript client and Blueprint editor | One Save action and protected local work                 |
| M4 / WS-V              | Documentation and closure              | Reconciled evidence, terminology, and aggregate closeout |

## Milestone plan

| M   | Title                     | Summary                                                      | Goal                                                    |
| --- | ------------------------- | ------------------------------------------------------------ | ------------------------------------------------------- |
| M1  | Revision-only persistence | Replace Draft persistence with atomic save-created Revisions | Establish one authoritative content version sequence    |
| M2  | Revision-based API        | Carry the exact current Revision through Rust and HTTP       | Make stale Saves explicit and Instance provenance exact |
| M3  | Explicit Save editor      | Replace Save-plus-Publish with one guarded Save              | Match the interface to the domain lifecycle             |
| M4  | Lifecycle closeout        | Reconcile docs and completed owner evidence                  | Remove current Draft terminology and close the redesign |

### Milestone 1: Revision-only persistence

- Depends on: none -- the resolved model and authoritative contracts settle the domain model.
- Deliverables: revised base-schema tables, functions, RLS, grants, immutability, events, and
  idempotency receipts.
- Workstreams: WS-D.
- Entry criteria: current Blueprint baseline and Course Instance provenance tests are green.
- Exit criteria:
  - Complete valid creation produces an Available lineage plus Revision 1 atomically; incomplete
    creation writes no lineage. Valid means at least one Module, one Assignment per Module, and one
    Question-bearing entry per Assignment, with every pin resolving to a Published Question Revision.
  - Course Instance creation accepts a selected exact Revision only while it is the Blueprint's
    current head; historical Revisions remain readable for existing provenance.
  - One changed Save based on Revision N produces exactly Revision N+1.
  - One semantically unchanged Save returns Revision N with `changed: false` and writes no Revision.
  - A stale or unauthorized Save writes no Revision.
  - An exact transport replay returns its existing receipt, Revision, and `changed` outcome.
- Parallel-plan ready: no -- one owner must preserve a single transactional schema contract.

### Milestone 2: Revision-based API

- Depends on: M1 -- Rust and HTTP contracts must target the accepted database transaction.
- Deliverables: revised domain types, Store trait and PostgreSQL adapter, browser API contract,
  generated TypeScript types, routes, response ETags, and focused Rust tests.
- Workstreams: WS-R.
- Entry criteria: M1 transaction and authorization checks pass.
- Exit criteria:
  - No current Rust or generated API type exposes Blueprint Draft, Draft Edit Number, or Blueprint
    Availability Edit Number.
  - `POST /api/course-blueprints` returns complete Revision 1 and its Revision ETag.
  - `PUT /api/course-blueprints/{reference}` returns the next Revision for changed content and the
    current Revision with `changed: false` for semantically unchanged content.
  - Course Instance creation still requires and returns its exact pinned Revision and rejects a
    submitted Revision that is no longer the selected Blueprint's current head.
- Parallel-plan ready: no -- generated contracts depend on the settled Rust boundary.

### Milestone 3: Explicit Save editor

- Depends on: M2 -- the browser client needs the accepted Revision-based API.
- Deliverables: revised decoder/client/model, one Blueprint Save action, current Revision display,
  dirty-state navigation guard, and focused Node/component tests.
- Workstreams: WS-B.
- Entry criteria: generated browser contract and HTTP route tests pass.
- Exit criteria:
  - Multiple local edits produce one request and one Revision when Save is selected.
  - The Save control is unavailable when editable fields match the loaded current Revision.
  - Successful Save clears dirty state and displays the resulting Revision Number.
  - Rename changes lineage names through the metadata action without creating a Revision.
  - A stale Save preserves local input and explains that a newer Revision exists.
  - Navigation, browser Back, reload, and window close protect unsaved changes.
- Parallel-plan ready: no -- editor behavior and client error handling share one local state owner.

### Milestone 4: Lifecycle closeout

- Depends on: M1, M2, M3 -- final docs and aggregate evidence must describe accepted behavior.
- Deliverables: current contract docs, changelog, reconciled owner evidence, forced Graphify refresh,
  and aggregate validation.
- Workstreams: WS-V.
- Entry criteria: all focused persistence, API, and browser checks pass.
- Exit criteria:
  - Automated persistence, API, and browser evidence covers Live Demo provisioning, exact
    historical Revision resolution, later Saves, and archive transitions.
  - Current source and durable docs contain no Blueprint Draft lifecycle outside explicit history.
  - The aggregate repository gate passes.
- Parallel-plan ready: no -- closeout reconciles shared fixtures and authorities after the cutover.

## Workstream breakdown

### Workstream WS-D: PostgreSQL lifecycle

- Goal: make Blueprint Revision insertion the only reusable-content persistence boundary.
- Owner: PostgreSQL implementation owner.
- Work packages: WP-D1, WP-D2.
- Needs: the resolved model and current base-schema evidence.
- Provides: atomic functions and row shapes for the Rust Store.
- Review boundary, when modifying the repository: `schemas/base_schema/50_functions/blueprints.sql`, Course
  Instance foreign keys, and Blueprint installation-data ownership.

### Workstream WS-R: Rust and HTTP contract

- Goal: express creation, Save, read, archive/restore, and Instance creation with exact Revisions.
- Owner: Rust implementation owner.
- Work packages: WP-R1, WP-R2.
- Needs: WS-D row/function contract.
- Provides: generated browser types and stable HTTP semantics for WS-B.
- Review boundary, when modifying the repository: question model, learning-data-access, server, and
  browser-api-contract crates.

### Workstream WS-B: Browser editor

- Goal: make explicit Save the only persisted Blueprint content action and protect local work.
- Owner: SolidJS/TypeScript implementation owner.
- Work packages: WP-B1, WP-B2.
- Needs: WS-R generated contract and route semantics.
- Provides: user-facing behavior and its focused browser evidence for WS-V.
- Review boundary, when modifying the repository: Blueprint API client and decoder; Blueprint model,
  creation helper, create dialog, workspace, and exports; live pages, Course list, Assignment source
  repository, Course Instance Revision selection, and affected browser journeys.

### Workstream WS-V: Documentation and closure

- Goal: reconcile completed owner evidence and document the whole Revision-only lifecycle.
- Owner: documentation/integration owner.
- Work packages: WP-V1.
- Needs: WS-D, WS-R, and WS-B completed behavior.
- Provides: release-grade evidence and current durable documentation.
- Review boundary, when modifying the repository: current durable docs, changelog, final gate record,
  and generated architecture map.

## Work packages

### Work package WP-D1: Replace Draft tables with Revision-only storage

- Owner: PostgreSQL implementation owner.
- Touch points: `schemas/base_schema/50_functions/blueprints.sql`, `schemas/base_schema/50_functions/course_operations.sql`.
- Depends on: none.
- Acceptance criteria:
  - The lineage points to its exact current Revision and owns Blueprint short name, long name,
    availability, and their opaque metadata concurrency token, but no mutable reusable structure.
  - Revision rows and their Module, Assignment, and Question-pin children remain immutable.
  - Course Instance foreign keys continue to target exact Blueprint Revisions.
  - Module and Assignment references are course-wide durable child identities across Revisions;
    reorder and relocation preserve them, while delete-and-recreate allocates new references.
  - The Course Instance creation transaction verifies that its exact submitted Blueprint Revision is
    still the lineage's current head before it writes the pinned provenance.
  - Before Draft tables are removed, a focused automated characterization proves that publication
    currently copies the same stable Module and Assignment references into Revision child rows.
- Automated evidence: focused child-reference characterization, then disposable PostgreSQL baseline
  plus catalog/RLS assertions.
- Obvious follow-ons: remove every Draft table, policy, grant, comment, and helper in the module.

### Work package WP-D2: Implement replay-safe create and Save transactions

- Owner: PostgreSQL implementation owner.
- Touch points: Blueprint create/save functions, existing Draft create/save receipt and publication
  event ownership, `schemas/installation_data/live_demo.sql`, and its oracle.
- Depends on: WP-D1 -- functions target the final row model.
- Acceptance criteria:
  - Create validates the complete reusable structure, writes lineage metadata and Revision 1 as one
    result, and leaves invalid initial work unpersisted.
  - A changed Save checks exact current Revision and inserts exactly the next Revision; an unchanged
    Save returns the current Revision with `changed: false`.
  - Changed/no-op classification uses one deterministic, unversioned canonical
    `BlueprintRevisionContent` encoding, its `BlueprintContentChecksum`, and exact aggregate equality.
  - No canonical-content encoding version constant, encoded `version` field, version column,
    version-selection function, compatibility reader, or legacy checksum fixture remains.
  - Exact retries converge; competing changed Saves from one base Revision have one winner.
  - The implementation renames and adapts the existing receipt/event machinery for Create, Save,
    and Rename/Availability; it adds no general event framework or new receipt abstraction.
  - Rename/Availability checks the opaque metadata ETag and cannot insert a Blueprint Revision.
  - Concurrent Rename and Archive operations sharing one metadata ETag have one winner; the loser
    receives the current metadata needed to retry safely.
  - Live Demo provisioning creates Revision 1 through the ordinary accepted transaction.
- Automated evidence: focused PostgreSQL concurrency and replay tests, including a fresh-base
  checksum/no-op proof with the sole canonical encoding.
- Obvious follow-ons: keep the operation lock order and SQLSTATE mapping explicit for WS-R.

### Work package WP-R1: Replace Draft domain and Store contracts

- Owner: Rust implementation owner.
- Touch points: `crates/question_model/src/blueprint_operations/`,
  `crates/question_model/src/blueprint_operations.rs`, `crates/question_model/src/blueprint_course.rs`,
  `crates/question_model/src/blueprint_course/`, `crates/question_model/src/lib.rs`,
  `crates/learning-data-access/src/blueprint_course.rs`, and its PostgreSQL adapter.
- Depends on: WP-D2 -- types and methods follow the accepted database contract.
- Acceptance criteria:
  - Create and Save receipts return exact Blueprint Revision References.
  - Save accepts an expected Blueprint Revision and complete replacement content.
  - Retained child references resolve course-wide against the expected head and appear at most once;
    moving an Assignment between Modules keeps its reference.
  - Create separates lineage names from revisioned reusable structure; Save content excludes names
    and availability.
  - `BlueprintAvailabilityEditNumber` is replaced by an opaque metadata ETag type that cannot be
    presented as another Blueprint sequence.
  - A field required to instantiate or compare Course Instance structure is owned by the Revision;
    discovery and administrative metadata is owned by the lineage.
  - Draft types, optional Draft views, and publication commands are absent.
- Automated evidence: focused Rust unit and PostgreSQL Store tests, including the sole canonical
  encoding and no legacy encoding-selection path.
- Obvious follow-ons: regenerate API types only after the Rust contract compiles.

### Work package WP-R2: Expose the Revision-based HTTP lifecycle

- Owner: Rust implementation owner.
- Touch points: `crates/server/src/blueprint_course.rs`,
  `crates/browser-api-contract/src/blueprint_course.rs`, generated contracts, Course Instance
  request/receipt types, `crates/learning-data-access/src/course_instance.rs` and its PostgreSQL
  adapter, `crates/server/src/course_instance.rs`, and Blueprint/Course Instance service journeys.
- Depends on: WP-R1 -- routes expose the Store contract.
- Acceptance criteria:
  - Create and Save responses include the exact current Revision and matching strong ETag.
  - Blueprint views carry a separate opaque metadata ETag for Rename/Availability without exposing
    another numeric Blueprint version.
  - A changed Save returns `changed: true` with Revision N+1; an unchanged Save returns
    `changed: false` with Revision N and the same ETag.
  - Missing/malformed preconditions fail; stale Saves return the repository's conflict response.
  - `/draft` and `/publish` routes are removed.
  - Course Instance creation receives one exact current Blueprint Revision Reference and rejects a
    historical or concurrently superseded Revision for a new Instance.
- Automated evidence: focused route status, ETag, authorization, and idempotency tests.
- Obvious follow-ons: regenerate and inspect browser types before WS-B begins.

### Work package WP-B1: Replace Draft client state with Revision-based Save

- Owner: SolidJS/TypeScript implementation owner.
- Touch points: `src/api/blueprint_course.ts`, `src/api/http_client/blueprint_course.ts`,
  `src/api/decoders/blueprint_course.ts`, generated API types, Blueprint feature exports and live
  pages, `src/pages/course_list_page.tsx`, `src/pages/assignment_editor_repository.ts`, and Course
  Instance Revision selection callers.
- Depends on: WP-R2 -- the generated transport shape is authoritative.
- Acceptance criteria:
  - Loaded Blueprint state has one current Revision and one matching ETag.
  - Save submits complete local reusable structure with that ETag and handles either the next
    Revision or the unchanged current Revision.
  - Rename/Availability uses only the opaque metadata ETag and leaves content dirty state and the
    current Revision unchanged.
  - Course Instance creation offers the current Revision advertised for each Available Blueprint and
    carries that exact value through submission; historical Revision loading remains read-only
    provenance support.
  - No client method or decoder names Blueprint Draft or Blueprint publication.
- Automated evidence: focused deterministic Node client/decoder tests.
- Obvious follow-ons: update every caller before removing transitional source names.

### Work package WP-B2: Make explicit Save protect one batch of local edits

- Owner: SolidJS/TypeScript implementation owner.
- Touch points: `src/features/blueprint_course/blueprint_course_model.ts`,
  `src/features/blueprint_course/blueprint_course_creation.ts`, Assignment content editor, create
  dialog, workspace, feature exports, route/live pages, styles, shared unsaved-changes guard,
  Blueprint model/UI tests, and affected browser journeys.
- Depends on: WP-B1 -- the editor needs the final Save result and conflict type.
- Acceptance criteria:
  - One or many local changes remain browser-only until explicit Save.
  - Complete initial local authoring creates Revision 1 and becomes available to vetted Instructors;
    incomplete initial authoring remains local.
  - Creation validation deliberately requires one Module, one Assignment per Module, and one
    Question-bearing entry per Assignment, with every pin resolving to a Published Question Revision.
  - The creation interface states that Create makes the complete Blueprint reusable to vetted
    Instructors immediately.
  - The Save control is unavailable when editable local fields match the loaded current Revision;
    server-side canonical comparison remains authoritative for no-op classification.
  - A changed Save creates one Revision, reports its number, and clears dirty state.
  - Reordering or relocating a retained child preserves its stable reference in the Save request.
  - Rename changes short/long lineage names without creating a Revision.
  - Accidental navigation or window close prompts before discarding dirty work.
  - A conflict keeps typed work available for deliberate reconciliation.
- Automated evidence: focused model/UI tests plus deterministic built-browser Blueprint and Course
  Instance journeys.
- Obvious follow-ons: remove Publish controls, copy, state, and styling completely.

### Work package WP-V1: Reconcile documentation and close accepted evidence

- Owner: documentation/integration owner.
- Touch points: `docs/TERMINOLOGY_CONTRACT.md`, `docs/DESIGN_DECISIONS.md`, `docs/CONTRACTS.md`,
  `docs/API_CONTRACTS.md`, `docs/CONCURRENCY_CONTRACTS.md`, `docs/IDENTITY_CONTRACTS.md`,
  `docs/AUTHORIZATION_CONTRACTS.md`, `docs/CODE_ARCHITECTURE.md`,
  `docs/DATABASE_AUTHORIZATION.md`, `docs/DATABASE_STRUCTURE.md`, `docs/FILE_STRUCTURE.md`,
  `docs/FAQ.md`, `docs/LOCAL_STACK_OPERATIONS.md`, `docs/MULTI_SERVER_SETUP.md`,
  `docs/NAMING_CONVENTIONS.md`, `docs/ROADMAP.md`, `docs/TEST_EVIDENCE_MODEL.md`, `docs/TODO.md`,
  `docs/CHANGELOG.md`, and any other current authority found by the final scoped terminology sweep.
- Depends on: WP-D2, WP-R2, WP-B2 -- durable docs describe accepted current behavior and owner
  evidence is complete before closure.
- Acceptance criteria:
  - Current docs use Save-created Blueprint Revision terminology consistently.
  - Current docs preserve the new-Assignment-as-Unreleased behavior separately from the future
    explicit propagation-approval workflow for changes to existing Assignments.
  - Durable contracts record the deliberate creation minimum, current-only new-Instance source,
    shared lineage metadata boundary, course-wide retained child identity, and canonical no-op rule.
  - Automated evidence proves two Course Instances can pin one current Revision, a later changed
    Save leaves both unchanged, and a new Instance pins the newer current Revision.
  - Historical reports and changelog archives remain intact.
- Automated evidence: Markdown-link/style checks, forced Graphify regeneration, and an independent
  subagent source-and-test review against this plan's acceptance criteria.
- Obvious follow-ons: archive this plan only after every completion condition passes.

## Acceptance criteria and gates

- Per-patch gate: the affected language's focused format, compile, lint, and behavior checks pass.
- Canonical-encoding gate: a fresh disposable baseline has one deterministic Blueprint-content
  encoding with no encoding-version constant, encoded field, database column, selector, fallback,
  or old-encoding fixture; equivalent validated content has one checksum and a changed canonical
  aggregate has a different checksum.
- Persistence gate: complete valid create yields Revision 1 and invalid create writes no lineage;
  changed Save from Revision 1 yields Revision 2; an unchanged Save returns Revision 2 without
  Revision 3; a competing stale Save writes nothing; exact retry returns its original outcome.
- Instance gate: existing Course Instances retain their pinned Revision after later Blueprint Saves;
  a new Instance stores the exact advertised current Revision; a historical or concurrently
  superseded Revision is rejected for new creation while remaining resolvable for provenance.
- Browser gate: deterministic Playwright journeys use synthetic authenticated Instructor sessions
  and a disposable database to prove that several local edits produce one Save and one Revision;
  navigation, browser Back, reload, and page-close events protect dirty work; conflict recovery
  retains unsaved work; incomplete first-authoring stays local; valid creation becomes discoverable
  to a second synthetic vetted-Instructor session; and Rename updates names without changing the
  current Revision.
- Terminology gate: current source and durable docs contain no `BlueprintDraft`, `blueprint_draft`,
  `Blueprint Draft`, `Draft Edit Number`, `BlueprintAvailabilityEditNumber`, numeric availability
  Edit Number, or separate Blueprint publish action outside explicit history.
- Integration gate: `source source_me.sh && ./launchers/all_test.sh` exits zero.
- Independent review gate: a subagent inspects the final diff, Graphify map, focused evidence, and
  aggregate results against every acceptance criterion; any uncovered criterion returns to its owner
  with a reproducible failure command or missing-evidence description.

## Test and verification strategy

- Update permanent Rust, Node, and PostgreSQL tests that protect the save-created Revision,
  meaningful no-op, idempotency, authorization, stale-write, immutability, stable child identity,
  public Revision 1 visibility, shared metadata concurrency, and exact Instance provenance
  contracts.
- Add a permanent deterministic canonical-encoding test that starts from a fresh base schema,
  asserts the single encoding has no version metadata or selection API, and proves equivalent
  validated content converges on one checksum while a structural change does not.
- Remove tests that preserve the retired Draft/publish structure or inventory implementation names.
- Use `tests/_temp/` for one deterministic real-stack cutover proof driven by a disposable database,
  synthetic authenticated sessions, and Playwright: create Blueprint Revision 1; make multiple
  browser edits; Save Revision 2; submit a canonical no-op without creating Revision 3; prove stale
  conflict; retain one Assignment reference while moving it between Modules; create two Instances
  from current Revision 1 before the Save; reject Revision 1 for new creation after the Save; create
  one Instance from current Revision 2; resolve both exact historical Revisions; archive and resolve
  again.
- Exercise the dirty-state guard by automated browser navigation, Back, reload, and `page.close()`;
  capture DOM-state assertions, response receipts, and database rows as the deterministic evidence.
- Remove the temporary proof at closeout unless a behavior meets the permanent-test checklist.
- Run the aggregate only after focused gates pass; a fresh failure remains related until evidence
  shows otherwise.

## Migration and compatibility policy

PLE is pre-production. Change `schemas/base_schema/50_functions/blueprints.sql` and other owning base modules
directly. Add no forward migration, compatibility view, legacy route, fallback reader, dual write,
or create-then-drop transition. Rebuild disposable databases and installation data from the
canonical base.

## Risk register

| Risk                                     | Impact | Trigger                                                                                                   | Owner                  | Mitigation                                                                                                             |
| ---------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| A retry creates duplicate Revisions      | High   | One idempotency key yields two Revision rows                                                              | PostgreSQL owner       | Lock the lineage and resolve the durable receipt before the stale-base check                                           |
| A no-op pollutes history                 | Medium | Unchanged canonical structure creates a new Revision                                                      | PostgreSQL owner       | Use the sole canonical content/checksum comparison and return the current Revision with `changed: false`               |
| Encoding-history machinery survives      | Medium | A content-encoding version constant, payload field, selector, column, fallback, or legacy fixture remains | PostgreSQL/Rust owners | Remove it in the pre-production cutover and prove the fresh baseline has exactly one canonical encoding                |
| Stable child identity is lost            | High   | Reordered or relocated Blueprint Assignments receive new references                                       | Domain owner           | Resolve retained references course-wide, preserve them across moves, and allocate new references only for New children |
| Course Instance provenance drifts        | High   | A stale picker creates a new Instance from a superseded Revision                                          | API owner              | Submit the advertised exact Revision, verify it is still current, and persist that exact provenance                    |
| Unsaved work is lost                     | High   | Navigation discards dirty editor state without warning                                                    | Browser owner          | Reuse the shared guard and prove each exit path with automated browser events and state assertions                     |
| Current docs describe old behavior       | Medium | Draft terminology remains in a durable authority                                                          | Documentation owner    | Run a scoped terminology sweep and update all current contract owners together                                         |
| Concurrent unrelated work is overwritten | High   | A touched file has active user or agent edits                                                             | Integration owner      | Inspect each diff, preserve unrelated hunks, and keep patches within named Blueprint owners                            |

## Documentation close-out requirements

- Active plan / progress tracker updates: keep this named plan current during execution; archive it
  under `docs/archive/` only after all gates pass.
- `docs/CHANGELOG.md` entry: record the behavior change, Draft/publish removal, settled one-sequence
  decision, unversioned pre-production canonical encoding, and final automated verification evidence
  under the current date.
- Archive / closure notes: preserve historical Draft-era reports as dated evidence; update only
  current authorities and generated architecture evidence.

## Patch plan and reporting format

- Patch 1: PostgreSQL lineage, immutable Revision, adapted receipts, installation data, and replay-safe
  changed/no-op Save transaction.
- Patch 2: Rust model, Store, HTTP, generated browser contract, and service-level lifecycle evidence.
- Patch 3: TypeScript client, creation flow, explicit Save editor, unsaved-change protection, and
  owning browser evidence.
- Patch 4: durable docs, evidence reconciliation, forced Graphify regeneration, and aggregate closeout.

## Resolved decisions

- Blueprint Course content uses Blueprint Revision Numbers as its only saved version sequence.
- Blueprint Course content has no persisted Draft and no content Edit Number.
- Blueprint lineage metadata uses an opaque ETag; no Blueprint availability Edit Number remains.
- Multiple local edits before explicit Save become one immutable Blueprint Revision.
- A semantically unchanged Save returns the current Revision with `changed: false` and creates no
  historical entry.
- The exact current Blueprint Revision is the Save concurrency precondition.
- Incomplete initial authoring stays local. Complete valid creation creates an Available Blueprint
  lineage and Revision 1 atomically, making it immediately reusable by vetted Instructors. The
  intentional minimum is one Module, one Assignment per Module, and one Question-bearing entry per
  Assignment, with every pin resolving to a Published Question Revision.
- Course Instances pin exact Blueprint Revisions. Content changes inside retained Assignments require
  the future explicit propagation-approval workflow; newly added Assignments follow the defined
  automatic Unreleased behavior.
- Future propagation uses stable child references: newly added Blueprint Assignments are added to
  eligible Course Instances as Unreleased, while changes inside retained Assignments require the
  future explicit propagation-approval workflow.
- Blueprint names and availability are lineage metadata. Revisions contain every field required to
  instantiate or compare reusable Course Instance structure.
- Short name, long name, and availability share one opaque lineage metadata ETag because they define
  public listing state and Archive confirmation depends on the current long name.
- Retained Module and Assignment references are durable child lineage identity across Revisions;
  moves preserve them, while delete-and-recreate produces a new identity.
- New Course Instance creation uses the selected Blueprint's advertised current Revision. Historical
  Revisions remain resolvable for existing provenance and are not offered as new-Instance sources.
- Save no-op detection uses one deterministic unversioned Blueprint content encoding, checksum, and
  exact stored aggregate comparison. PLE has no content-encoding version number, payload field,
  selector, column, fallback reader, or legacy fixture before its first production baseline.
- Create and Save adapt the repository's existing request/receipt semantics; this redesign adds no
  broader event or receipt framework.
- Draft Questions retain their separate private authoring and publication lifecycle.

## Open questions and decisions needed

None for dispatch. Browser crash, tab crash, and network-failure recovery are a separate future
editor-reliability project. Blueprint-to-Course-Instance propagation is also a separate future
project whose new-versus-retained Assignment behavior is fixed above; this plan delivers the exact
Revision and stable-reference evidence that project will consume.
