# Plan: Course appearance and banners

## Status

Planning state: implementation-complete on 2026-08-09. WP-CA1 through WP-CA7 and release package
WP-RC1 are accepted. Candidate expiry and delivery timing are fixed below, no course-appearance
scope question remains, and WP-RC3 is the next dependency.

## Decisions

- Keep the accepted 15-theme catalog and Roosevelt-inspired `grass` default.
- Store one authoritative tenant-owned appearance revision per course.
- Normalize every banner to one 1200 by 328 pixel WebP center crop; browsers scale that exact
  derivative without recropping.
- Expire an unconsumed candidate 60 minutes after creation. Limit any protected object delivery grant
  to 60 minutes and recheck the exact current course pointer on every delivery request.
- Show one centered banner only at course entry; theme every course-owned route; preserve global and
  semantic-status colors.

## Objectives

- Let an instructor or tenant administrator choose one reviewed course theme and save it with
  compare-and-swap conflict protection.
- Let an instructor upload, preview, replace, or remove one centered course-entry banner without
  exposing object keys or leaving two banners current.
- Apply the selected three-color palette to every learner and instructor page in that course while
  keeping global PLE pages and semantic status colors stable.
- Preserve tenant isolation, retention ownership, accessibility, answer secrecy, and the existing
  grading and Wasm boundaries.
- Keep the implementation compartmentalized into small contract, object, persistence, API, and
  browser owners that can be implemented and reviewed independently.

## Scope

WP-CA6 and WP-CA7 completed the in-scope version 1 capability: the real instructor settings page,
production client/route integration, keyboard/recovery/responsive behavior, disposable
PostgreSQL/MinIO evidence, visual artifacts, durable documentation, and independent review.

## Background

Blackboard Original let an instructor give a course its own visual identity through a course theme
and a centered banner at the course entry point. PLE needs the useful part of that idea without
arbitrary instructor CSS, inaccessible color combinations, orphaned files, or theme state leaking
between courses.

The durable owner guidance is intentionally small: one preconfigured three-color biome or habitat
theme across every page inside a course, plus one small centered banner at the course entry page.
This plan translates that guidance into a bounded, tenant-owned course capability. It does not make
the instructor a graphic designer and does not recolor authored scientific content or semantic
grading states.

## Design philosophy

Use a closed, measured design-token catalog and one course-root projection instead of arbitrary CSS
or page-specific styling. Use one atomic appearance revision instead of independent theme and banner
updates. The banner follows the low-noise Blackboard entry-point model: fixed centered cropping and
two responsive previews, not a general-purpose image editor. The server emits one exact 1200 by 328
pixel WebP, preserving the proportions of the centered 1546 by 423 pixel YouTube banner safe region
at the existing 1200-pixel PLE width cap; the browser only scales that derivative down.

- Evidence strategy for uncertain methods: measure every rendered foreground/background pair, emit
  an all-theme contact sheet, and compare role-matched palettes in OKLab before keeping or merging a
  theme. Validate object, RLS, and revision behavior against disposable PostgreSQL and MinIO rather
  than relying on mock wiring alone.

## Detailed capability scope

- Define a closed `CourseThemeId`, revisioned browser-safe appearance projection, banner alternative
  text choice, and strict update command.
- Persist tenant-owned course appearance separately from course membership replacement.
- Add a protected course-banner object and delivery class with one current course pointer.
- Add authenticated read, candidate upload, atomic save, removal, and current-banner delivery paths.
- Add one instructor appearance page and one course-scoped Solid theme provider.
- Apply the theme to course entry, assignment overview, run attempt, run summary, assignment editor,
  gradebook, and the new course appearance settings route.
- Add permanent contract, conformance, API, TypeScript, accessibility, visual, and security gates.

## Non-goals

- Do not allow arbitrary colors, CSS, fonts, menu layouts, background images, or per-page themes.
- Do not recolor authored question media, scientific diagrams, print/export output, success, danger,
  warning, correctness, or grading feedback semantics.
- Do not repeat the banner on working pages after the course entry page.
- Do not store appearance in `localStorage` or treat it as a browser preference.
- Do not place course banners in student-record storage or delete them during student-record purge.
- Do not build manual pan, zoom, filters, illustration tools, or a general media library in this
  package.

## Current state summary

- Blackboard's official [course-style documentation](https://help.anthology.com/blackboard/instructor/en/original-course-view/set-up-courses/customize-your-course/course-style-options.html)
  places a centered banner at the course entry point and its
  [customization overview](https://help.anthology.com/blackboard/instructor/en/original-course-view/set-up-courses/customize-your-course.html)
  treats themes/colors as course-scoped, institution-controlled choices. PLE adopts those two
  boundaries, not Blackboard's broader menu/structure styling.
- `CourseRecord` and `CourseSummary` carry course identity, title, role, and memberships, but no
  appearance.
- PostgreSQL has tenant-leading forced-RLS `course` and `course_member` rows, but no appearance row
  or revision.
- The object contract has no tenant-owned `CourseBanner` class. Current signed delivery knows only
  catalog and student-record assets.
- The browser has a fail-closed route-scoped appearance provider across all seven course-owned
  surfaces and a working instructor settings page with keyboard, conflict, responsive, and
  forced-color evidence.
- The frozen `/instructor/courses/:courseId/appearance` route, production client, mock, decoder,
  documentation, and tests changed atomically.
- WP-QTI-8 through WP-QTI-12 and WP-CA1 through WP-CA7 are accepted. The independently assigned QTI
  Package Maker WP-FQ-0 contract can proceed without contending for these closed client, route,
  Store, and documentation seams.

## Architecture boundaries and ownership

`question_model` owns only browser-safe course appearance types. `objects` owns physical object keys
and signing classification. `learning-data-access` owns a separate `CourseAppearanceStore` contract
and memory/PostgreSQL implementations. Every mutation accepts the authenticated actor and revalidates
persisted instructor/administrator authority inside the same backend operation; server route checks
remain an earlier non-enumerating guard, not the only authorization. `server_core` owns image decoding,
normalization, candidate orchestration, and HTTP. The Solid feature owns palette literals, route
scope, the instructor form, and visual acceptance.

The browser projection contains the theme ID, strong appearance revision, and optional current
banner presentation. It never contains a bucket/key, checksum, filename, source bytes, upload
metadata, signed URL, grading data, or answer-bearing content. The banner presentation contains a
same-origin asset ID/route and either explicit informative text or the decorative state.

One `course_appearance` row is keyed by `(tenant_id, course_id)` and contains the stable theme ID,
optional current banner delivery ID, banner alternative-text state, revision, and update time. It is
created transactionally with a new course. Every mutation uses the current strong ETag and advances
one revision. PostgreSQL creates the default row through an `AFTER INSERT` course trigger; the Memory
`upsert_course` path creates it under the same write lock. WP-CA3 alone owns these narrow existing
course-creation seams.

A separate private `course_banner_candidate` relation owns candidate object identity, normalized
checksum and dimensions, creator, expiry, deterministic promoted-object identity, and consumed/
cleanup state. It is never a course summary, asset-delivery row, or browser DTO. Its only readers and
writers are the narrow appearance promotion/cleanup capabilities under the same tenant/course and
persisted-role checks.

Banner replacement uses a two-step internal lifecycle:

1. A bounded author-only upload writes normalized bytes to a non-signable
   `CourseBannerCandidate` key and persists a tenant/course/actor-bound candidate row with checksum,
   expiry, and a deterministic future current-object identity.
2. One JSON appearance save locks that candidate and revision, copies/verifies bytes first into the
   protected immutable `CourseBanner` key, then atomically selects the theme, registers delivery,
   swaps the current pointer, and marks the candidate consumed under `If-Match`.
3. A stale CAS keeps the candidate retryable and records the copied current object as candidate-owned,
   never as an untracked orphan. One bounded, idempotent cleanup worker deletes expired candidates and
   any unreferenced copied bytes only after rechecking that no appearance points at them.

A stale save leaves the existing appearance current and the candidate eligible for bounded cleanup.
No candidate is learner-deliverable. A superseded current banner becomes immediately
non-deliverable because authorization verifies the course's current pointer before issuing access.
Managers may read retained course branding; students may read it only while
`ple_course_records_accessible` is true. Mutations require persisted instructor/administrator
authority. Outsiders and foreign tenants receive concealed 404 behavior. Banner delivery enforces
the same split rather than relying on tenant-only RLS.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                                | Review boundary                                                             |
| ---------------------- | ---------------------------------------- | --------------------------------------------------------------------------- |
| CA1 / WS-A             | `question_model`, route and DTO contract | Closed IDs, revision, safe projection, generated type boundary              |
| CA2 / WS-B             | `objects`                                | Typed candidate/current keys and exact classification/signing               |
| CA2 / WS-C             | schema, asset delivery, appearance Store | Forced RLS, CAS, cleanup, current-pointer and backend parity                |
| CA3 / WS-D             | `server_core::course_appearance`         | Authorization, image normalization, candidate and atomic-save HTTP behavior |
| CA4 / WS-E             | `src/features/course_appearance/`        | Exhaustive theme registry and route-scope cleanup                           |
| CA4 / WS-F             | instructor appearance page               | Keyboard flow, preview, conflict recovery, save/remove behavior             |
| CA5 / WS-G             | integration and acceptance               | Live database/object/browser evidence and independent review                |

## Milestone plan

| M   | Title                       | Summary                                                          | Goal                                                            |
| --- | --------------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------- |
| CA1 | Freeze the contract         | Lock types, palettes, route shape, image policy, and defaults    | Every later owner implements one unambiguous boundary           |
| CA2 | Build secure storage        | Add banner object/delivery and revisioned Store/schema behavior  | Tenant-safe CAS and object lifecycle pass backend parity        |
| CA3 | Add the server capability   | Normalize uploads and expose protected appearance operations     | One atomic, recoverable instructor save works over HTTP         |
| CA4 | Build the course experience | Add the route-scoped theme and instructor settings surface       | Theme follows every course page and the banner stays entry-only |
| CA5 | Prove the package           | Run permanent and live security, accessibility, and visual gates | Independent review reports no P0/P1 finding                     |

### Milestone CA1: Freeze the contract

- Depends on: WP-QTI-12 plus the existing course, auth, object, and frontend contracts.
- Deliverables: closed theme IDs and palette table; safe Rust/TypeScript appearance shapes; API and
  route contract; upload/normalization policy; exact default and error semantics.
- Workstreams: WS-A.
- Entry criteria: this plan is linked from the authoritative implementation plan and WP-QTI-12 has
  independently passed.
- Exit criteria: generated types compile; every accepted ID has exactly one palette; unknown IDs
  refuse; no storage/object data enters the DTO; architecture and route docs agree.
- Parallel-plan ready: no. This contract is the shared input to every later owner.

### Milestone CA2: Build secure storage

- Depends on: CA1.
- Deliverables: typed course-banner object and candidate/current delivery shapes; forward migration;
  forced RLS; memory/PostgreSQL appearance capability; shared conformance.
- Workstreams: WS-B, then WS-C.
- Entry criteria: CA1 contract gate green.
- Exit criteria: exact object classification and signing tests pass; Store conformance proves CAS,
  one current banner, current-pointer authorization, and foreign-tenant non-enumeration; fresh and
  no-op migration gates pass.
- Parallel-plan ready: no. WS-C consumes WP-CA2's frozen object identity and owns all asset-delivery,
  schema, Store, cleanup, and shared re-export seams so those persistence owners cannot race.

### Milestone CA3: Add the server capability

- Depends on: CA2.
- Deliverables: focused course-appearance router; image validator/normalizer; candidate upload;
  current banner delivery; atomic save/remove; focused route tests.
- Workstreams: WS-D.
- Entry criteria: both CA2 backend contracts green.
- Exit criteria: manager success, student refusal, outsider 404, stale 412, invalid image/theme
  refusal, one-current-banner, and no-store/ETag behavior pass; failures preserve the old appearance.
- Parallel-plan ready: no. HTTP behavior composes the two CA2 capabilities.

### Milestone CA4: Build the course experience

- Depends on: CA1 for mock-backed work and CA3 for live integration.
- Deliverables: exhaustive palette registry; course appearance scope; instructor route and form;
  centered banner preview; strict client/decoder/runtime updates; all seven course-owned routes
  wired.
- Workstreams: WS-E and WS-F, followed by one route/client integration owner.
- Entry criteria: CA1 for pure UI work; CA3 for production transport.
- Exit criteria: keyboard selection/upload/save/remove/conflict recovery pass; route scope clears when
  leaving a course; no appearance state enters browser storage; all course routes inherit the theme;
  global pages and semantic status colors do not.
- Parallel-plan ready: yes -- max parallel doers: 2. Theme-scope and settings-form owners use separate
  files; the integration owner alone edits shared route/client files.

### Milestone CA5: Prove the package

- Depends on: CA3 and CA4.
- Deliverables: disposable PostgreSQL and MinIO oracle; built-browser Playwright flow; contact sheet
  and palette metrics; cognitive walkthrough; durable docs; independent P0/P1 review.
- Workstreams: WS-G.
- Entry criteria: focused Rust and browser gates green.
- Exit criteria: full task gate green; visual artifacts inspected; all contrast and dedup metrics
  recorded; independent review PASS; plan/status/changelog/architecture docs updated.
- Parallel-plan ready: no. This milestone exists to find cross-layer behavior hidden by focused
  tests.

## Workstream breakdown

### Workstream WS-A: Contract and theme catalog

- Goal: freeze stable identifiers, safe projections, exact palettes, default, routes, and errors.
- Owner: `architect` with `color-accessibility-expert` review.
- Work packages: WP-CA1.
- Needs: current course/frontend contracts and measured palette evidence.
- Provides: types and acceptance fixtures for every later owner.
- Review boundary, when modifying the repository: no persistence or HTTP mutation in this slice.

### Workstream WS-B: Banner object contract

- Goal: add typed candidate/current banner keys and signing classification without changing existing
  source, catalog, or student-record semantics.
- Owner: `expert_coder`.
- Work packages: WP-CA2.
- Needs: WP-CA1 identifiers and image policy.
- Provides: exact object identity/classification contract to persistence and server owners.
- Review boundary, when modifying the repository: `objects` only.

### Workstream WS-C: Appearance and banner-delivery persistence

- Goal: persist one revisioned appearance, protected candidate/current delivery, and bounded cleanup
  state under forced RLS with backend parity.
- Owner: one `expert_coder`; a separate `postgresql-expert` reviews the SQL/RLS/lock boundary.
- Work packages: WP-CA3.
- Needs: WP-CA1 and the WS-B delivery identity.
- Provides: actor-authorized CAS, candidate promotion/cleanup, and current-pointer delivery to the
  server.
- Review boundary, when modifying the repository: schema, asset delivery, focused
  memory/PostgreSQL modules, course-creation seams, and conformance only.

### Workstream WS-D: Server API and image normalization

- Goal: expose one authenticated, bounded, recoverable appearance workflow.
- Owner: `rust-code-expert`.
- Work packages: WP-CA4.
- Needs: WP-CA2 and WP-CA3.
- Provides: safe HTTP/client shapes and normalized image evidence.
- Review boundary, when modifying the repository: focused router/normalizer and route tests.

### Workstream WS-E: Theme scope

- Goal: project three audited colors through one Solid course subtree without global bleed,
  including the new appearance page itself.
- Owner: `solid-js-expert` with `color-accessibility-expert` review.
- Work packages: WP-CA5.
- Needs: WP-CA1.
- Provides: one theme provider consumed by course surfaces.
- Review boundary, when modifying the repository: theme registry/provider and focused Node tests.

### Workstream WS-F: Instructor settings

- Goal: provide a short, keyboard-complete theme/banner task with preview and recovery.
- Owner: `ui-ux-engineer` with `human-interact-expert` review.
- Work packages: WP-CA6.
- Needs: WP-CA1 for mocks and WP-CA4 for live transport.
- Provides: appearance page and behavior tests.
- Review boundary, when modifying the repository: feature-local form/model/components only.

### Workstream WS-G: Integration and acceptance

- Goal: prove the whole capability across real roles, storage, routing, rendering, and accessibility.
- Owner: `integrator`, followed by an independent `reviewer`.
- Work packages: WP-CA7.
- Needs: WP-CA2 through WP-CA6.
- Provides: release evidence and documentation closure.
- Review boundary, when modifying the repository: shared seams, live tests, Playwright, and docs.

## Work packages

### Work package WP-CA1: Freeze course appearance contracts

- Owner: `architect`.
- Status: complete on 2026-08-09. The Rust owner, generated TypeScript, executable route/reference
  implementation, and
  contract documents passed focused and full repository gates. WP-CA2 through WP-CA5 subsequently
  passed; WP-CA6 and WP-CA7 subsequently passed. During WP-CA5 the owner changed the pre-data
  default and Grass anchors;
  the coordinated Rust, SQL, generated-TypeScript, and browser contracts now reflect that decision.
- Touch points: `crates/question_model/src/course_appearance.rs`, generated API types,
  `src/route_contract.ts`, `docs/FRONTEND_ARCHITECTURE.md`, `docs/CONTRACTS.md`.
- Depends on: WP-QTI-12.
- Acceptance criteria:
  - Define `CourseThemeId`, `CourseAppearanceRevision`, safe banner presentation, strict mutation,
    and exactly one default (`grass`).
  - Keep all storage identity and answer-bearing types out of generated/browser contracts.
  - Add `/instructor/courses/:courseId/appearance` atomically to route docs, executable route
    contract, mocks, and route tests.
  - Freeze exactly the 15 IDs and measured source colors below.

#### Frozen theme palette

The redundancy oracle compares only role-matched `secondary` and `accent` anchors in OKLab and
reports Euclidean distance times 100 as `DeltaE_OK`. Merge a pair only when its mean distance is
below 8 and its maximum role distance is below 10. Forest/Woodland measures 0/0. The closest retained
pair, Coral reef/Salt marsh, has mean 8.2 with role distances 1.8 and 14.6, so it remains distinct.

| Theme ID      | Canvas    | Secondary | Accent    | On colors C/S/A    | Minimum contrast |
| ------------- | --------- | --------- | --------- | ------------------ | ---------------- |
| `tundra`      | `#e3e1da` | `#725e72` | `#485b3c` | black/white/white  | 5.90:1           |
| `forest`      | `#e4ebdd` | `#166747` | `#aa831a` | black/white/black  | 5.97:1           |
| `desert`      | `#f3e2bd` | `#c07a3b` | `#68402a` | black/black/white  | 6.09:1           |
| `grass`       | `#bddeb1` | `#73c167` | `#008852` | ink/ink/decorative | 5.51:1 derived   |
| `arctic`      | `#e5f5f8` | `#7cbed1` | `#1f5d78` | black/black/white  | 7.25:1           |
| `ocean`       | `#ddeff5` | `#0b6c88` | `#123c69` | black/white/white  | 5.97:1           |
| `tropical`    | `#e4f2d6` | `#1b7646` | `#8a1976` | black/white/white  | 5.64:1           |
| `coral-reef`  | `#e8f6f1` | `#006d68` | `#b52d3d` | black/white/white  | 6.16:1           |
| `swamp`       | `#e8e5c9` | `#4e5f23` | `#4b3426` | black/white/white  | 7.03:1           |
| `underground` | `#e6e0d8` | `#59504a` | `#c9732c` | black/white/black  | 5.97:1           |
| `salt-marsh`  | `#e8f0df` | `#1e6a6d` | `#76511f` | black/white/white  | 6.29:1           |
| `wetland`     | `#e4eee7` | `#466f59` | `#3b648c` | black/white/white  | 5.71:1           |
| `sea-floor`   | `#dee8ed` | `#344e62` | `#086a72` | black/white/white  | 6.33:1           |
| `magma`       | `#f5e0cf` | `#a92720` | `#3b2928` | black/white/white  | 7.00:1           |
| `beach`       | `#f3e7c9` | `#56a8b0` | `#8a3d24` | black/black/white  | 7.57:1           |

Grass is Roosevelt-inspired, not an official institutional theme. The two vivid greens come from
the [Roosevelt Lakers brand guide](https://rooseveltlakers.com/documents/2025/2/7/RooseveltLakers_BrandGuidelines_2024.pdf),
while `#BDDEB1` is a pale fill observed in the public
[Roosevelt University logo SVG](https://upload.wikimedia.org/wikipedia/commons/a/ad/Roosevelt_University_Logo.svg).
Raw `#008852` remains a decorative anchor because it cannot meet the house 5.5:1 text target with
black or white. The browser derives `#006B40` for white-on-action controls and `#005C38` for links on
the Grass canvas; those projections are design-system outputs, not stored theme colors.

`woodland` is consolidated into `forest` before persistence; the UI does not create two
indistinguishable choices. Re-run the OKLab oracle whenever any anchor changes.

- Evidence or review, when useful: generated-type gate, palette contrast/OKLab report, and an
  independent contract review.
- Next dependency: WP-CA2, WP-CA3, and mock-backed WP-CA5/WP-CA6 consume this accepted contract.

### Work package WP-CA2: Add protected banner objects

- Owner: `expert_coder`.
- Status: complete on 2026-08-09. Typed candidate/current identities, classification, signing
  refusal/permission, memory conformance, S3-feature compilation, and full repository gates passed.
  WP-CA3 subsequently passed; object identity remains owned here rather than by its persistence.
- Touch points: typed object keys and object conformance tests only.
- Depends on: WP-CA1.
- Acceptance criteria:
  - Add tenant/course-bound candidate and current `CourseBanner` keys with no caller-supplied path.
  - Classify current banners as protected tenant course content, never source or student records.
  - Make candidates non-signable. Permit current banner signing at the typed-object layer while
    requiring WP-CA3 current-pointer authorization before any delivery record is usable.
  - Preserve every existing source-signing refusal and student-record retention rule.
- Evidence or review, when useful: Memory/MinIO object conformance and exact signing/refusal tests.
- Next dependency: WP-CA3 consumes the frozen identities; WP-CA2 does not edit asset-delivery persistence.

### Work package WP-CA3: Persist revisioned course appearance

- Owner: one `expert_coder`; a separate `postgresql-expert` reviews the SQL/RLS/lock boundary.
- Status: complete on 2026-08-09. Memory and PostgreSQL now create the default appearance with the
  course, enforce revision CAS and persisted session authority, persist bytes-first banner promotion,
  authorize only the exact current pointer, and perform bounded two-phase cleanup. The new forward
  migration and disposable PostgreSQL 17 oracle passed the full seven-migration database gate;
  WP-CA4 through WP-CA7 subsequently passed.
- Touch points: one forward migration, asset-delivery contract/backends/conformance, focused
  `course_appearance` contract/backends/conformance, initial course-creation seams, and bounded
  cleanup worker/capability.
- Depends on: WP-CA1 and the WP-CA2 delivery identity.
- Acceptance criteria:
  - Create appearance with the course through one PostgreSQL `AFTER INSERT` trigger and the same
    atomic Memory `upsert_course` write lock; use tenant-leading keys and `FORCE ROW LEVEL SECURITY`,
    and constrain the closed theme ID and positive revision.
  - Make every mutation accept the authenticated actor and transactionally revalidate persisted
    instructor/administrator authority. Managers may read retained branding; students may read only
    while `ple_course_records_accessible` is true; outsiders/foreign tenants are non-enumerating.
  - Persist candidate checksum/expiry/future-current identity, perform bytes-first idempotent
    promotion, enforce one exact current banner pointer, and provide race-safe cleanup that rechecks
    references before deleting expired candidate or unreferenced promoted bytes.
  - Prove CAS success/conflict, student mutation refusal, membership-change behavior, candidate and
    superseded delivery refusal, and current delivery success.
  - Preserve appearance and banner through student-record archive/delete; remove them only through a
    future explicit hard course deletion or bounded replacement cleanup.
- Evidence or review, when useful: unchanged memory/PostgreSQL conformance plus one disposable
  real-role/RLS/CAS oracle.
- Next dependency: WP-CA4 consumes only this narrow capability.

### Work package WP-CA4: Implement atomic appearance HTTP behavior

- Owner: `rust-code-expert`.
- Status: complete on 2026-08-09. The production router now exposes bounded author candidate upload,
  current appearance GET, atomic strong-ETag PUT, and current-only same-origin banner delivery.
  Server-owned normalization accepts only JPEG/PNG/WebP, rejects animation/malformed/oversized or
  undersized images, applies orientation, strips metadata, and emits one exact 1200 by 328 WebP.
  Focused route/image tests, strict Clippy, the complete disposable PostgreSQL/RLS gate, and all 11
  repository checks passed. WP-CA5 through WP-CA7 subsequently passed.
- Touch points: `crates/server/src/course_appearance.rs`, composition, focused tests, image codec
  dependency owned only by the server.
- Depends on: WP-CA2 and WP-CA3.
- Acceptance criteria:
  - Accept JPEG, PNG, or WebP input at no more than 2 MiB and 20 decoded megapixels; reject SVG,
    animation, malformed bytes, and images that cannot supply a 1200 by 328 pixel center crop after
    orientation.
  - Correct orientation, strip metadata, center-crop without stretching, and emit exactly one 1200
    by 328 pixel WebP derivative. Never upscale and never create device-specific derivatives.
  - Require explicit decorative state or 1-160 character informative alternative text; never infer
    it from filename or course title.
  - Provide author-only candidate upload, strict atomic JSON save, current banner read, and no-store
    responses with strong ETags.
  - Return 412 for stale `If-Match`, 413 for size, 415 for unsupported media, 422 for decoded/image/
    theme/alt validation, 403 for a student mutation, and concealed 404 for outsiders/foreign
    tenants. Preserve the old appearance on every failure.
- Evidence or review, when useful: hostile image corpus, focused route tests, and bytes-first object
  failure tests.
- Next dependency: the generated-contract owner updates strict client contracts before WP-CA6.

### Work package WP-CA5: Implement the course theme scope

- Owner: `solid-js-expert`.
- Status: complete on 2026-08-09. One course-root loader now supplies course and appearance data,
  run attempts reuse `RunScreenData.course`, and authorized run summaries carry a server-derived
  safe course projection. The exhaustive 15-theme registry rejects unknown IDs, scopes variables
  below the persistent shell, clears them across course/global navigation, and leaves semantic
  statuses unchanged. Focused Node/Rust checks, the complete 56-case built Playwright suite, and all
  11 repository checks passed. WP-CA6 and WP-CA7 subsequently passed.
- Touch points: `src/features/course_appearance/theme_catalog.ts`, provider/styles/tests, course route
  composition.
- Depends on: WP-CA1; live transport integration depends on WP-CA4.
- Acceptance criteria:
  - Exhaustively map every generated theme ID to exactly `canvas`, `secondary`, and `accent` anchors
    plus audited derived foreground/focus/surface tokens.
  - Apply variables to one course route subtree and clear them on cross-course/global navigation.
  - Theme course entry, assignment, run, summary, editor, gradebook, and appearance settings without
    a second flash-producing learner fetch; leave Library, Workspace, global navigation, and
    semantic statuses unchanged.
  - Use one course-root loader for course-ID routes, reuse `RunScreenData.course` for run attempts,
    and add an authorized safe course-appearance projection to run-summary data. Do not accept a
    browser-supplied course identity or issue a theme-only learner fetch after render.
  - Reject an unknown ID as a contract error rather than silently substituting a theme.
- Evidence or review, when useful: Node token/route-scope tests and rendered pair measurements.
- Next dependency: WP-CA6 uses this same provider for its previews.

### Work package WP-CA6: Build instructor appearance settings

- Owner: `ui-ux-engineer` with HCI review.
- Status: complete on 2026-08-09. The real settings route uses native radio cards, exact wide/narrow
  previews, decorative/informative alternative text, one save action, explicit stale-revision
  reload, replacement/removal, and locally preserved recovery state. Focused Node and built-browser
  behavior, keyboard, axe, forced-color, and 320/480-pixel checks passed.
- Touch points: `src/features/course_appearance/` model/client/components/page and focused tests.
- Depends on: WP-CA1 for mocks, WP-CA4 for live transport, and WP-CA5 for preview.
- Acceptance criteria:
  - Present labeled native radio cards with the theme name and three decorative swatches; preserve
    logical keyboard order and never rely on color alone.
  - Show exact centered wide and narrow banner previews; keep the course title as text outside the
    image; require decorative/informative alt choice. Both previews preserve the exact 1200 by 328
    derivative ratio and differ only in CSS display width.
  - Use one `Save appearance` action; disable duplicate saves; preserve local theme/file/alt state on
    validation, network, auth, permission, and stale-revision errors; offer explicit conflict reload.
  - Make remove/cancel and remove/save visibly different; no empty banner frame appears to students.
  - Meet forced-colors, reduced-motion, 200 percent zoom, 320/480-pixel no-overflow, and touch/focus
    target requirements.
- Evidence or review, when useful: behavior-focused Node tests and a keyboard cognitive walkthrough.
- Next dependency: WP-CA7 exercises the real page without direct test-state injection.

### Work package WP-CA7: Run integrated appearance acceptance

- Owner: `integrator`, then independent `reviewer`.
- Status: complete on 2026-08-09. All seven course surfaces, entry-only learner banner, production
  request shapes, real-role PostgreSQL/RLS/CAS, database-enforced current-pointer ownership,
  combined PostgreSQL-to-MinIO idempotent cleanup, current delivery, hostile input, visual metrics,
  and responsive artifacts passed. Three read-only reviewers reported no P0/P1/P2.
- Touch points: route/client seams, disposable PostgreSQL/MinIO oracle, Playwright, visual artifacts,
  architecture/status/changelog docs.
- Depends on: WP-CA2 through WP-CA6.
- Acceptance criteria:
  - Prove instructor select/upload/save/reload/replace/remove/conflict and student read-only behavior
    through visible controls and real same-origin request shapes.
  - Prove all seven course routes inherit one appearance, global pages do not, banner stays
    entry-only, and route changes cannot retain the previous course's variables.
  - Prove contrast, focus, alt behavior, forced colors, reduced motion, responsive layout, object
    lifecycle, RLS, current-pointer authorization, and retention ownership.
  - The accepted 2026-08-09 run emitted a 15-theme contact sheet plus
    320/480/768/1920/forced-color screenshots. Future refreshes use the canonical 1280 by 800
    instructor canvas plus a separate narrow compatibility guard and retain `palette_metrics.json`
    containing rendered contrast plus the OKLab dedup table.
  - Keep grading/Wasm/generated answer secrecy and learner network-trace gates green.
- Evidence or review, when useful: full commands and exact artifact paths in the implementation
  handoff; independent PASS with no P0/P1 finding.
- Next dependency: WP-RC3 shipped upstream WeBWorK consumes the accepted package sequence.

## Acceptance criteria and gates

- Per-patch gate: focused owner tests, formatting, strict lint/Clippy, generated types when touched,
  Markdown/ASCII checks for docs, and `git diff --check`.
- Contract gate: Rust enum, generated TS union, database constraint, browser registry, route contract,
  mock, and decoder change together; no intermediate accepted ID exists in only one layer.
- Accessibility gate: all rendered normal-text pairs meet the 5.5:1 house target; large text,
  controls, icons, selected outlines, and two-color focus indicators meet 3:1; axe reports no
  critical/serious findings.
- Tenant/object gate: forced RLS and current-pointer authorization hide foreign, outsider,
  candidate, and superseded banners; no object key/checksum/source URL enters a DTO or log.
- Integration gate: focused Rust/Node/Playwright suites plus disposable PostgreSQL and MinIO oracles
  pass before `./check_codebase.sh` and the built Playwright suite.
- Independent review gate: a reviewer who changed none of the package files audits the contracts,
  live evidence, screenshots, and metrics and reports no P0/P1 finding.

## Test and verification strategy

Permanent Rust unit/conformance tests own closed IDs, appearance validation, CAS semantics, one
current pointer, object classification/signing, membership-sensitive delivery, and memory/PostgreSQL
parity. Focused HTTP tests own authorization, no-store/ETag, hostile inputs, atomicity, and error
classes. Node tests own strict decoding, exhaustive registry mapping, state recovery, and scope
cleanup. Playwright owns visible instructor/student behavior, keyboard and accessibility states,
course/global scope, and responsive rendering.

Disposable PostgreSQL verifies fresh/replay migrations, grants, forced RLS, concurrent CAS, pointer
constraints, and retention. Disposable MinIO verifies bytes-first write, checksum, candidate/current/
superseded delivery, and idempotent cleanup. Visual acceptance uses the built browser artifact; DOM
assertions alone do not accept palette quality.

Do not add permanent tests for exact configurable production byte/pixel maxima beyond their bounded
hard ceilings, generated UUID values, signed URL contents, object cleanup timing, screenshot bytes,
theme-card counts, or exact error prose. Assert stable behavior and named IDs instead.

## Migration and compatibility policy

- Add a forward migration; do not rewrite the accepted six-file baseline after its audited boundary.
- Persist only the 15 closed IDs. `woodland` is consolidated into `forest` before first persistence,
  so no compatibility alias is required unless prototype data is discovered during implementation.
- New courses receive `grass` transactionally. Existing pre-data courses receive `grass` in the
  forward migration and the same positive initial revision in both backends.
- Adding, renaming, or removing a theme is a coordinated Rust/SQL/generated-TS/browser-registry
  migration. Palette value changes retain the stable ID but require renewed contrast, dedup, contact-
  sheet, and visual review evidence.
- A missing current banner object is a recoverable banner-delivery failure; it does not block course
  assignments or mutate the appearance record silently.

## Risk register

| Risk                                                      | Impact                                             | Trigger                                                        | Owner          | Mitigation                                                                              |
| --------------------------------------------------------- | -------------------------------------------------- | -------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------- |
| Theme variables bleed into another course/global page     | Users lose course identity and status colors drift | Navigation retains prior CSS variables                         | WS-E           | Route-subtree ownership and cleanup tests across two courses/global routes              |
| Palette passes source math but fails rendered states      | Unreadable controls or focus                       | Hover/disabled/visited/overlay pair misses threshold           | WS-E/WS-G      | Measure computed styles and inspect contact sheet, not anchors alone                    |
| Banner upload exhausts memory or processes active content | Availability/security failure                      | Oversized decoded image, SVG, animation, malformed codec input | WS-D           | Pre-read byte cap, decoded pixel cap, raster allowlist, bounded decoder, hostile corpus |
| Replacement exposes two/current stale banners             | Incorrect branding or leaked inactive object       | Old asset route still delivers after CAS                       | WS-B/WS-C      | Authorize against current pointer on every delivery; superseded refusal test            |
| Student-record retention deletes branding                 | Instructor loses reusable course identity          | Banner classified as student record                            | WS-B/WS-C      | Typed `CourseBanner` content class and archive/delete rehearsal                         |
| Multi-tab save silently overwrites another instructor     | Lost work                                          | Stale revision accepted                                        | WS-C/WS-D/WS-F | Strong ETag, 412, preserved local state, explicit review-current flow                   |
| Theme ID contracts drift across languages                 | Unknown theme or silent fallback                   | One layer accepts an unmapped ID                               | WS-A           | Coordinated contract gate and fail-closed decoder/provider                              |
| Feature files enlarge existing oversized parents          | Harder maintenance and merge conflicts             | New behavior lands in `course.rs` or global CSS/client parents | Integrator     | Focused modules plus one-owner shared-seam patches                                      |

## Rollout and release checklist

- [x] Freeze the 15-ID catalog, exact palettes, Grass default, image policy, and route/DTO contract.
- [x] Pass CA2 object, Store, schema, RLS, and conformance gates.
- [x] Pass CA3 upload/normalization/API behavior and hostile-image gates.
- [x] Pass WP-CA5 course scope, cross-course cleanup, fail-closed decoding, and rendered contrast.
- [x] Pass WP-CA6 settings, keyboard, recovery, and responsive gates.
- [x] Inspect the contact sheet, responsive screenshots, and palette metrics.
- [x] Pass disposable PostgreSQL and MinIO live oracles, including their combined cleanup lifecycle.
- [x] Pass `./check_codebase.sh` and built Playwright acceptance.
- [x] Obtain independent no-P0/P1 review.
- [x] Update plan/status/changelog/architecture/file-structure/contracts docs.

## Documentation close-out requirements

- Active plan/progress tracker: update `docs/active_plans/implementation_plan.md` and
  `docs/active_plans/partial_commit_status.md` at every CA milestone exit.
- `docs/CHANGELOG.md`: record contract, persistence/object, server, browser, and acceptance slices
  only after their own gates pass.
- Durable architecture: update `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`,
  `docs/FRONTEND_ARCHITECTURE.md`, `docs/SOLID_MODEL.md`, `docs/CONTRACTS.md`, and object/retention
  documentation where behavior changes.
- Closure: add `docs/active_plans/workstreams/course_appearance_implementation.md` with exact gates,
  artifact paths, accepted tradeoffs, and independent review result.

## Patch plan and reporting format

- Patch 1: WP-CA1 closed contract, theme catalog, generated route/DTO seams.
- Patch 2: WP-CA2 pure object identities/classification.
- Patch 3: WP-CA3 asset delivery, schema/Store, course-creation seam, candidate promotion/cleanup,
  and backend conformance.
- Patch 4: WP-CA4 server API and image normalization.
- Patch 5A and 5B in parallel: WP-CA5 theme scope and WP-CA6 settings/model against mocks; one later
  integration owner wires production route/client files.
- Patch 6: WP-CA7 live/browser acceptance, docs, and independent review.

Each report names work package, exact files, focused and full commands, artifact paths, deviations,
and index state. Each atomic task has one owner, one outcome, and at least one independently readable
verification artifact.

## Resolved decisions

- Keep 15 visible themes and consolidate `woodland` into `forest`; no other measured pair meets the
  redundancy rule.
- Use exactly three stored design colors: canvas, secondary, and accent. Derived text, border,
  surface, focus, and semantic status tokens remain design-system outputs rather than extra theme
  choices.
- Default new and migrated courses to Roosevelt-inspired `grass` at the owner's direction. Keep
  `#008852` decorative and use measured derived action/link projections where text contrast applies.
- Show one centered banner only on the course entry page; theme every course-owned route.
- Use one exact 1200 by 328 pixel server-owned center crop with ratio-preserving browser previews
  instead of manual crop controls, client recropping, or multiple responsive derivatives.
- Treat appearance as authoritative tenant-owned server state with strong revisions, never a browser
  preference.
- Expire an unconsumed candidate after 60 minutes. Limit a protected course-banner object grant to
  60 minutes and retain the current-pointer authorization check as the actual access boundary.

## Decision completeness

No course-appearance scope or implementation decision remains open. Deployment may shorten the
delivery grant, but it may not lengthen either accepted 60-minute ceiling without a reviewed contract
change and renewed object/security evidence.
