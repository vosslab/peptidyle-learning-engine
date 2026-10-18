# Plan: Complete PLE's modular UI architecture

## Context

Approved by the user; saved 2026-10-16 before implementation. This is the complete follow-on
plan for the settled DataSheet architecture, collection behavior, editor composition, content-first
layout, and publication thumbnails. It can be executed without further human participation.

The October 4 consolidation established shared architecture. The completed
[modular_ui_completion_plan.md](../../archive/modular_ui_completion_plan.md) and its
[modular_ui_completion_ledger.md](../audits/modular_ui_completion_ledger.md) retain their accepted
32-milestone history. This plan builds on that work and completes the subsequently refined design.
It does not reopen the earlier acceptance or claim that the new requirements already passed.

The archived [DataSheetBlock_and_SearchBlock_Specification.md](../../archive/DataSheetBlock_and_SearchBlock_Specification.md)
supplies the four-mode component model. Later user decisions recorded here supersede its older
cache, uncropped WeBWorK preview, and selection wording. At the planning baseline,
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) contained earlier three-mode terminology; M01
reconciles it with approved requirements, preserving authorship and historical provenance.

The critical three-page architectural proof, called M18 in the original 32-milestone plan,
is M23 here. M18 here concerns Native previews. These milestone namespaces are separate.
Execution status and evidence live in
[datasheet_collection_status.md](../reports/datasheet_collection_status.md).

## Objectives

- Complete one shared PageFrame, SearchBlock, and DataSheet implementation across applicable pages.
- Make useful records and actions visible immediately after the Ribbon and breadcrumbs.
- Provide four distinct presentation modes with common query, column, pagination, and action behavior.
- Preserve working domain rules, accessibility, privacy, revision identity, and actual edit protection.
- Compose Pool and Assessment editors from independent shared collections and direct record actions.
- Produce seed-zero WeBWorK thumbnails at publication and revision changes, outside the API runtime.
- Complete and verify every milestone through manager decisions, independent agents, and automated checks.

## Design philosophy

Apply **Fix the design, not the symptom**, **Design for adaptability**, and **Atomic task
decomposition** from [REPO_STYLE.md](../../REPO_STYLE.md). Extract and connect working shared
behavior before writing replacements. Domain fields and commands vary; collection interaction
and presentation do not need page-specific implementations.

Use KISS aggressively. Shared column state is a small in-memory model, not a configuration
framework. Use complete local data in the browser and existing server query owners for remote
collections. Extend a backend only when its existing capabilities cannot satisfy the contract.

Every permanent test must protect intentionally stable, important behavior with a plausible
regression risk. Use one-time evidence for tunable dimensions, default column choices, and spatial
comparisons. Do not create tests that merely enforce component file structure.

Evidence strategy: use the completed code audit and actual renderer probe as the baseline.
Resolve implementation details through bounded agent experiments at the owning milestone,
record the result, implement the selected approach, and verify it before accepting the milestone.
An investigation is not a substitute for completing its required implementation.

Assess runtime stability before accepting dependent work. ROOT accepted M07 and M18-M23 VERIFIED on
2026-10-17 after fresh whole-specification and distinct quality reviews passed for each milestone.
M07's independent SQL oracle and actual picker/keyword-classification proof, plus Native math/image
rendering, seed-zero capture and painting, immutable revision artifacts, publication, backfill,
zero-capture browsing, and outage recovery are recorded in the current status checkpoint. Earlier
failures remain historical evidence; the accepted source route for backfill and temporary browser
proof is documented under M22 and the canonical execution routes below. ROOT accepted M10 VERIFIED
on 2026-10-17 after fresh whole-milestone SPEC and distinct QUALITY reviews passed, following its
13-oracle owned SQL run and canonical actual attempt 3. ROOT accepted M28 and M29 VERIFIED on
2026-10-17 after fresh whole-milestone SPEC and distinct QUALITY reviews plus their canonical actuals
and focused positive Watch Open/inactive-route supplements passed; see the current status checkpoint.
All 37 milestones are verified. M36 passed the canonical fast and full gates, current spatial and
integrated acceptance, and independent reviews; M37 reconciled the documentation and retired the
one-time executables while retaining evidence. ROOT accepted M32 VERIFIED on 2026-10-17
after fresh `m32_combined_spec` and distinct `m32_combined_quality` PASS. Its facade, controls/style,
numbered-request size, and dead result-cursor client cleanup is recorded in the current status
checkpoint; live native/provider/facet cursors remain. Focused TypeScript, lint, formatting, and
request-probe checks passed; final runtime acceptance is separate. ROOT accepted M31 after fresh
whole-milestone `m31_complete_spec` and distinct `m31_complete_quality` passed, with canonical native actual 14,
AdminStars actual 7, inherited consumer evidence, and C62 actual 3 (exit 0, 4.1s) closing its
required authoring-interaction gap. C57 avatar and C64 BlueprintWatch runtime evidence remains
limited as documented; accepted architecture/backend evidence applies without adding per-consumer
gates. M30 passed whole-milestone
SPEC, distinct QUALITY, and final documentation review; its accepted evidence and limits are in the
status report. M25 passed
whole-completion SPEC and distinct QUALITY review,
panel/C16/Browse actual acceptance, and ROOT acceptance; remaining gates are recorded in the status
report.
The shared local-complete query repair passed fresh SPEC and QUALITY, and C16 passed corrected
source reviews; both retain runtime/actual gates recorded in the status report.
The final fast/full acceptance results and their distinct evidence scopes are recorded in the
[status checkpoint](../reports/datasheet_collection_status.md#current-recovery-checkpoint---2026-10-17).
Shared browser/build execution remains ROOT-serialized. ROOT accepted M33 and M34 VERIFIED after
the final spatial visual aggregate, current Student, Admin and Instructor captures, desktop/theme/
zoom review, and the phone Add/Remove/reorder/save/reload walkthrough. ROOT accepted M35 VERIFIED
after reconciling the actual M07-M32 journeys with current-build native Select/Recover across four
modes, HTTP 200/no-store responses and exact revision pins. The guarded archive's elapsed scheduler
is outside this accepted recovery replay. Final integration requirement and distinct quality reviews
found no material current source ownership or quality defect. M36's canonical full gate subsequently
passed with exit 0.

## Scope

- Reconcile current guidance, architecture documents, and the consumer inventory.
- Preserve the heading-free PageFrame and remove remaining redundant page scaffolding.
- Extract DataSheetControls, DataSheetBlock, HeaderRow, and RecordItem from existing shared behavior.
- Complete the shared Compact/List DataSheetTable boundary from existing RecordTable in M15.
- Complete Compact, List, Cards, and Visual Boxes across applicable collections.
- Implement consistent selection, explicit refresh, numbered pagination, sorting, and filtering.
- Integrate parallel shared collections into Pool, Assessment, and Blueprint Assessment editors.
- Generate and serve immutable revision-specific WeBWorK thumbnails during publication.
- Audit and finish Instructor, Student, administrative, embedded-picker, and other collection consumers.
- Complete local runtime verification, independent review, documentation, and evidence cleanup.

## Non-goals

- Introduce another parallel collection-display system or page-specific column implementation.
- Add cell editing, a general spreadsheet engine, persistent display preferences, or speculative options.
- Synchronize search tabs automatically after records are edited elsewhere.
- Make multi-selection a prerequisite for ordinary record actions or editor membership changes.
- Generate WeBWorK images during Library browsing or add preview caches, queues, or background sync.
- Put Chromium in the API container, use WeasyPrint, or add client-side publication capture.
- Change the externally maintained WeBWorK renderer or Native question ownership to accommodate PLE.
- Perform Git/index/staging/commit/push operations or deploy production services.

## Current state summary

### Baseline capabilities and gaps

The baseline below predates the accepted source and runtime slices recorded in the status report.

- PageFrame heading, short-label, and introductory-text inputs have already been removed.
  Preserve that contract; verify current output rather than reintroducing suppression flags.
- SearchBlock currently wraps caller controls and SearchResults, which delegates to RecordList.
  RecordList already owns reusable fields, sorting, filtering, resizing, hiding, and rearrangement.
- Current presentation is three modes. List is not yet the specified wrapping table, and controls
  and presentation still need the approved ownership separation.
- Search state and column state already exist. Repair their transitions instead of creating
  separate state machines in each page.
- Existing server result owners use cursor pagination. Facet-choice cursors are a distinct concern.
  Some picker eligibility filtering occurs after a fetched page; move it before pagination.
- The local-complete adapter can reactively refresh/prune selection when source resources change.
  Separate explicit search snapshots from editor-owned draft updates.
- Publication already renders WeBWorK with seed zero for validation, then discards the returned HTML.
  There is no production Playwright capture service or registered revision-thumbnail artifact.
- The API image is a minimal Rust runtime. Test Playwright tooling proves a capture technique,
  not an application service that can be reused unchanged.

### Completed thumbnail investigation

The actual renderer probe used the unmodified
`/Users/vosslab/nsh/PROBLEMS/biology-problems/problems/biochemistry-problems/enzymes/enzyme_simulation.pgml`,
plus an ordinary biochemistry question and a genetics matching question. Evidence is retained
locally under `tests/_temp/thumbnail_capture_probe_20261016/`, especially
`receipt.json`, `browser_results_ready.json`, and `top_crop_result.json`.
These are standalone feasibility results, not integrated publication acceptance.

The enzyme question rendered to 912 x 1351 pixels. The historical probe scaled it to width 256,
giving 256 x 379; the selected top crop produced a 256 x 256 WebP at quality 80, 7,696 bytes.
This measurement records feasibility; its width-scaling method is superseded by the source-resolution
and cropping contract below.
Thumbnails provide a sense of the question; their text need not be readable.

The current preview CSP blocks the source's two inline scripts, leaving both canvases blank.
The probe retained sandbox isolation and allowed the exact script hashes; both canvases then
painted with zero browser errors or failed resources. M19 repairs this at shared document
preparation, rather than adding a simulation-specific renderer.

The private renderer network cannot reach the gateway directly. Capture must package PLE
embed assets and use the API's constrained renderer-asset proxy. Current publication cleanup
also has an early image-preparation failure gap after source prewrites; M21 covers the touched
failure paths. Caddy's current 30-second upstream timeout must be reconciled with renderer,
capture, storage, and transaction deadlines before accepting publication integration.

## Architecture boundaries and ownership

| Owner | Responsibility |
| --- | --- |
| PageFrame | Layout, spacing, page actions, and content; Ribbon/breadcrumbs supply identity |
| Page/domain | Collection scope, criteria, query execution, creation actions, and domain commands |
| SearchBlock | Compose page-supplied criteria and controls, DataSheetControls, and the required DataSheetBlock |
| DataSheetControls | Surrounding modes, pagination, counts, optional bulk UI, and appropriate sort/filter interfaces; delegate collection behavior to DataSheetBlock and column mechanics to DataSheetTable through shared state/callbacks |
| DataSheetBlock | All four display modes and shared collection sorting, filtering, selection, and record actions |
| DataSheetTable | Reusable Compact/List tabular presentation; column configuration and resize/show/hide/rearrange/responsive-hide interactions; independent temporary widths, visibility, and order per table mode; contains HeaderRow and RecordItem |
| HeaderRow | Heading interface within DataSheetTable; forwards sort/filter input to DataSheetBlock and column input to DataSheetTable without an independent controller |
| RecordItem | One record's identity, presentation, selection indicator, preview, and direct actions in every mode |
| Editor | Membership, saved order, drafts, revision pins, scoring, save/reload, and conflicts |
| API publication | Exact source/revision binding, capture request, validation, transaction, and object storage |
| Thumbnail service | Bounded isolated JavaScript rendering and WebP encoding; no persistence authority |

Physical placement does not transfer ownership. SearchBlock may arrange page-supplied criteria;
creation remains a page command. DataSheetControls sits outside DataSheetBlock and invokes its
shared state/callbacks rather than maintaining another query or presentation controller; column
controls invoke DataSheetTable's column mechanics. RecordList's working implementation is extracted
into these owners and its obsolete facade is retired. M13's verified extraction retains the existing
RecordTable. M15 completes and names that shared table as DataSheetTable, preserving M14's Compact
work and avoiding a parallel collection system. This target refinement does not reopen M13 acceptance.

### Mapping (milestones -> components)

| Milestone | Component | Review boundary |
| --- | --- | --- |
| M01-M03 | Guidance, inventory, spatial rubric | Complete requirements and evidence baseline |
| M04-M05 | Shared state and edit guards | Search snapshots versus genuine unsaved edits |
| M06-M12 | Pagination and domain query owners | Full-result predicates, totals, order, permissions |
| M13-M18 | Shared collection components and Native previews | One behavior owner across four modes |
| M19-M22 | WeBWorK document, capture, publication, artifacts | Exact revision and bounded private runtime |
| M23 | Search Library, Browse Library, My Questions | End-to-end shared architecture proof |
| M24-M27 | Pool and Assessment editors | Membership/order remain editor-owned |
| M28-M31 | Remaining collection consumers | Complete inventory, consistent integration |
| M32-M34 | Obsolete code removal and spatial cleanup | Shared defaults and all-role accessibility |
| M35-M37 | Integration, verification, documentation | Evidence-backed complete delivery |

### File scope

The inventory refines this expected boundary; generated files follow their canonical sources.

| File or directory | Intended change | Canonical source, for generated output |
| --- | --- | --- |
| `src/components/page_frame/`, `src/ribbon/` | Preserve shell contract; complete navigation integration | Existing shared sources |
| `src/components/data_sheet/` | Extract shared DataSheet components/state; retire obsolete facade | Existing field/query/action models |
| `src/components/search_block/`, `src/pages/` | Shared composition, query bindings, consumer integration | Domain query owners |
| Pool, picker, Assessment, Blueprint editor modules under `src/` | Parallel collections and direct draft commands | Existing editor models and save contracts |
| `crates/question_model/`, `crates/server/`, `crates/learning-data-access/` | Pagination, exact query counts, preview projections, publication | Rust domain and transport definitions |
| `schemas/base_schema/` | Query functions and revision-thumbnail relations | Initial SQL schema |
| `crates/objects/` | Typed thumbnail object and WebP verification | Existing object store and integrity model |
| `crates/adapters/webwork/` | Reuse validated seed-zero document | Existing renderer adapter |
| `question_thumbnail_renderer/` | Separate private Playwright/cwebp runtime process | Repository-root `package.json` and `package-lock.json` |
| `containers/`, `local_stack_control/` | Build, private network, health, lifecycle, deadlines | Existing local deployment owners |
| `src/styles/ple_embed.css`, `src/public/ple_bridge.js`, `src/assets/fonts/` | Package canonical embed resources for capture | These existing sources; no independent copies to maintain |
| `ple_import/`, `devel/` | Import verification and explicit missing-artifact backfill | Existing publication endpoint and revision identity |
| `tests/_temp/`, existing justified behavior tests | Temporary proof and selective stable regression protection | Actual behavior contracts |
| Architecture, contracts, guidance, schema docs, plan/status, changelog | Reconcile authority and record evidence | Code, SQL, approved decisions, canonical generators |

### Collection behavior contract

- Four modes: Compact, List, Cards, Visual Boxes. Every collection exposes all four. Domain rules
  can restrict an operation, such as display sorting for saved Assessment contents, without
  removing a presentation mode. Single-choice selection remains distinct from optional bulk selection.
- Compact uses aligned, single-line, truncated rows, subtle alternating backgrounds, and essential
  actions. Roughly 20 CSS pixels per desktop row is a practical target, not a rigid acceptance gate.
- List is a wrapping table with fewer, wider columns and variable-height rows. It reveals richer
  descriptions and metadata. Its difference from Compact must be obvious on the same records.
- Compact and List use the same reusable DataSheetTable, with independent transient
  width/order/visibility settings. DataSheetTable owns column configuration and resize, show/hide,
  rearrangement, and responsive hiding. Shared field definitions supply defaults. Settings last for
  the page instance, without browser/server storage.
- Cards are stacked text-oriented panels. Visual Boxes are responsive image-oriented tiles.
  Native and existing images retain their aspect ratio; the generated WeBWorK crop is specified below.
- Library defaults to Compact; Blueprint Course Search defaults to Cards. Other collections default
  to Compact except an existing visual task whose purpose warrants Visual Boxes.
- HeaderRow sits within DataSheetTable and forwards sorting/filtering input to DataSheetBlock;
  DataSheetTable handles column mechanics. Header clicks toggle ascending/descending ordering, with
  semantic headings and `aria-sort`. Column menus provide filters/configuration; keyboard
  alternatives support resize and rearrangement. HeaderRow has no independent controller.
  Cards/Visual Boxes expose the same query operations through DataSheetControls, which invokes shared
  DataSheetBlock state/callbacks and delegates column controls to DataSheetTable.
- DataSheetBlock owns sorting, filtering, selection, and record actions across all four modes.
  RecordItem provides content and direct actions in every mode, inside DataSheetTable for Compact/List.
- Sorting and filtering apply to the complete collection before pagination. Database-backed
  collections use PostgreSQL `WHERE` and `ORDER BY` across the complete matching result set before
  pagination; existing stable domain ordering gets a stable identity tie-breaker. Page-only sorting of remote
  results is invalid. Complete local sources use equivalent shared browser processing. Use existing
  query capabilities before extending SQL/API contracts.
- Numbered result pages use `pageNumber`, `pageSize`, `totalCount`, and `items`. Reuse page-size
  choices and existing numbered-control behavior: Previous/Next, page numbers, ellipses, current page,
  and total pages. Facet-choice cursors remain independent and are retained.
- Count and data predicates include identical scope, permissions, criteria, and picker eligibility.
  Clamp invalid/out-of-range pages through shared logic, without loops or fabricated totals.
- Keywords/filters clear selection and reset to page 1. Sorting resets to page 1 but preserves
  selection. Pagination, page-size changes, and modes preserve selection by stable record identity.
  Page-size changes retain a valid page through the common pagination transition.
- Mode changes preserve query, ordering, page, and selection and cause no search request.
  Visible-page selection selects only that page; bulk commands act on exact selected IDs.
- Question Library Search initially displays empty headings and a concise prompt. Require a keyword
  or filter to load results; clearing all criteria restores the initial state.
- My Questions is the same collection interface with predefined personal scope.
- Search places criteria above full-width results. Browse places classification beside results
  and above them on narrow screens. Both use identical DataSheet capabilities.
- Question Library and Blueprint search records open details/editors in a new tab or window.
  The original query/results/page/selection/scroll stay unchanged until explicit refresh or query.
  Current-interface commands update directly affected content; they do not synchronize other tabs.
- UnsavedChangesGuard protects meaningful authored changes only. Search activity and opening an
  editor are not unsaved work. Failed editor saves retain the draft.
- Selection is optional, unobtrusive, and present only with meaningful bulk commands. Preserve bulk
  metadata editing; direct actions and dedicated editors remain primary.
- Keep direct record actions keyboard/touch reachable. Preserve focus, visible selection, meaningful
  accessible names, and horizontal overflow where needed rather than silently losing actions.

### Editor behavior contract

Pool and Assessment editors each compose two independent SearchBlocks: current contents on the
left, ordinary Library results on the right. Narrow screens stack them in that order. Queries,
pagination, modes, and column configuration are independent; editor membership links the panels.

- Add/Remove buttons are primary. Drag-and-drop calls the same editor-owned draft operations.
  Both paths retain exact revision pins and validation; neither writes immediately behind Save.
- The right panel includes already-added questions if and only if they match its current search.
  Show Included and disable Add when that domain forbids duplicates. Do not inject included rows
  into nonmatching searches or hide them from matching results.
- Pool sorting/filtering affects display only. Preserve existing membership limits, eligibility,
  issued-member restrictions, conflict handling, and explicit Save/Reload.
- Multiple starting Questions for Pool creation must share Discipline and Subject. Retain entered
  work and explain actual incompatibilities when creation cannot proceed.
- Assessment contents preserve saved order during search/filtering and show original positions.
  Disable display sorting specifically for Assessment contents. Reordering is an explicit editor
  operation using stable entry IDs and full saved positions, never filtered-array indexes.
- Preserve Assessment scoring, exact Question/Pool references, revisions, and save concurrency.
  Blueprint Assessment duplicate rules remain domain-specific; never deduplicate all Question IDs.
- Optional bulk commands supplement direct operations. Avatar and archived-Attempt single-choice
  controls do not imply bulk-selection semantics.

### Publication thumbnail contract

Use a PLE-owned private synchronous Playwright service on `renderer_private`. Keep Chromium out
of the API container and out of the upstream WeBWorK renderer. The service has Node, matching
Playwright/Chromium headless shell, and cwebp; it has no database, object-store, or Account credentials,
public port, durable queue, or persistent cache.

The API keeps source/revision authority. Its existing validation renders seed zero exactly once;
retain that HTML and send it to the private capture endpoint. The service returns WebP bytes or a
bounded capture failure. It does not choose a revision, publish a Question, or supply trusted
object identity. UI and automated imports continue through the same publication endpoints.

- Share document preparation with isolated previews: preserve the existing sandbox and exact known
  parent-telemetry removal; allow required executable inline scripts by exact SHA-256 hashes.
  Use proper existing HTML parsing facilities. Do not add unrestricted inline execution,
  same-origin sandbox privilege, or simulation-specific scripts.
- Apply response-level CSP to the captured document; `page.setContent` alone is insufficient.
  Fresh contexts have no session cookies/storage and are destroyed after success or failure.
- Package embed CSS, bridge, and fonts from their canonical sources. Fulfill only those approved
  local asset paths. Forward renderer asset paths through the existing API proxy at its fixed
  internal origin. Reject unapproved network requests and preserve existing path/media/size checks.
  Do not depend on gateway reachability, external browser-origin DNS, or disabled TLS verification.
- Capture the actual question region at normal layout, excluding surrounding PLE navigation.
  Await required scripts, MathJax startup when present, fonts, image decoding, and initial paint.
  Use a bounded deadline and two animation frames; do not wait for animations to finish.
  Do not operate Run/buttons/sliders or otherwise change the initial question state.
- Preserve the existing rendered question layout. Output is always 256 x 256 pixels. Capture at
  sufficient source resolution to fill the thumbnail without raster upscaling, preserving the
  rendered content's proportions. Crop excess width symmetrically from both sides and excess
  height from the bottom, keeping the content horizontally centered and vertically top-aligned.
  Encode WebP quality 80 once.
- Validate resulting WebP bytes and dimensions with the existing image verifier. Preserve lossy
  encoded bytes instead of routing them through the existing lossless WebP reencoder.
- Store a typed immutable QuestionThumbnail artifact associated with exact published revision/source,
  checksum, byte length, media type, and dimensions. Keep adapter render-cache objects separate.
- Include the thumbnail reference in existing summary projections; avoid one detail request per
  record. Reuse existing Library access and object-delivery boundaries, with no new user auth flow.
- Capture and verify before source/image object prewrites. Register source, revision, and thumbnail
  together in the existing publication transaction and CAS boundary.
- Failed capture/storage/registration prevents publication and leaves the Draft editable.
  Clean up exact candidate objects on conclusive pretransaction failures/rollback; retain objects
  on ambiguous commit status under the existing recovery policy. Never delete a committed artifact.
- Generate on Draft publication and new published revisions, including forks through normal publish.
  Library browsing only reads the stored artifact. Existing published revisions use an explicit
  idempotent backfill, run by agents during M22, never a browsing-triggered job.
- Start with one capture at a time, bounded admission and a recoverable busy failure. Reuse the
  browser process with fresh contexts; recover browser failure without adding a durable worker queue.
- M21 owns the measured total publication deadline. Measure representative success and timeout
  failures, set the API work deadline below gateway/client timeouts, and keep the supported path
  within the existing import client's 120-second budget. Update the publication route's timeout
  owner where needed; do not assume today's 30-second gateway window is sufficient.
  Record concrete values and verify cancellation/cleanup before M21 can pass.

### Content-first rubric

For every element between breadcrumbs and primary content, including SearchBlock's controls, record
its owner, purpose, necessity, frequency, redundancy, footprint, and reason to precede the data.

| Criterion | Acceptance question |
| --- | --- |
| Necessity | Does the task require this element before the records or work? |
| Redundancy | Is its information already present in Ribbon, breadcrumbs, or nearby content? |
| Frequency | Are common operations direct and occasional configuration available on demand? |
| Priority | Is this element important enough to displace primary content? |
| Density | Can it use less space or share a row without harming interaction? |
| Context | Does it appear where its information is needed? |
| Responsiveness | Does it preserve function and useful content at narrow widths and zoom? |

Remove redundant headings, labels, explanations, and instructions by default. Retain explanation
only for a specific demonstrable usability need. Evaluate visible useful content, interface overhead,
and common-task interaction cost together. A higher first record achieved by hiding necessary actions
does not pass. Apply recurring repairs at shared component owners.

## Milestone plan

All milestones are required. The manager owns dependency coordination and acceptance; each named
owner below is the implementation responsibility to assign to an agent, not a new application layer.
Planning and architecture decisions remain with the manager and appropriately capable reviewers.
Luna agents may implement bounded, decision-complete tasks.

Common entry: dependencies accepted and current source/authority checked. Common exit: deliverables,
listed behavior proof, fresh specification review, then distinct quality review pass, and evidence is
recorded in the status ledger. A failed gate returns to its owner for correction and fresh re-review;
independent ready work continues. No gate waits for a human walkthrough or approval.
Use normal tool approval and the built-in entrypoints directly. Independent SPEC, distinct QUALITY,
and manager acceptance are review decisions recorded in the ledger, not a custom approval-file
protocol required to start the Demo. Leave the denied temporary `runtime_review.json` operation and
its historical driver unchanged; neither is part of the execution path below.

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M01 | Reconcile authorities | Preserve settled design and attribution | One current contract |
| M02 | Inventory consumers | Map every collection and behavior owner | Complete integration scope |
| M03 | Capture spatial baseline | Apply rubric to representative roles/pages | Comparable layout evidence |
| M04 | Repair selection transitions | Centralize identity-based rules | Optional consistent bulk behavior |
| M05 | Repair refresh and guards | Separate snapshots from editor drafts | Predictable browsing and safe edits |
| M06 | Define numbered pagination | Shared result/state contract | Consistent full-result navigation |
| M07 | Migrate Library pagination | Exact counts and eligible complete queries | Correct search and picker pages |
| M08 | Migrate Blueprint pagination | Shared Course search queries | Public Blueprint page navigation |
| M09 | Migrate proposals/history | Numbered scoped queries | Stable history and proposal results |
| M10 | Migrate Watch pagination | Numbered private activity | Correct private counts and pages |
| M11 | Migrate Admin pagination | Preserve restricted query boundaries | Correct administrative collections |
| M12 | Migrate retained-work pagination | Preserve retention and Student scope | Correct Student/recovery pages |
| M13 | Extract DataSheet owners | Reuse RecordList behavior | One modular implementation |
| M14 | Complete Compact | Dense accessible spreadsheet | Fast record scanning |
| M15 | Complete List and DataSheetTable | Refine/name existing RecordTable; wrapping independent table layout | Shared Compact/List owner and readable record details |
| M16 | Complete Cards/Visual Boxes | Shared panel and tile modes | Distinct useful alternatives |
| M17 | Complete shared controls | Compose controls and data | No page-level behavior duplication |
| M18 | Connect Native previews | Reuse prompt/media rendering | Real identifying record content |
| M19 | Build private capture service | Playwright, assets, CSP, crop | Reliable JavaScript thumbnail |
| M20 | Register revision artifacts | Schema, objects, summary/delivery | Immutable exact-revision images |
| M21 | Integrate publication | Capture, transaction, cleanup, deadlines | Atomic publish with thumbnail |
| M22 | Verify thumbnail lifecycle | UI/import, backfill, runtime recovery | Complete operational path |
| M23 | Prove three shared pages | Search, Browse, My Questions | Critical architecture acceptance |
| M24 | Integrate Pool panels | Independent current/available searches | Direct membership editing |
| M25 | Integrate Assessment panels | Saved order and direct actions | Searchable ordered contents |
| M26 | Integrate Blueprint editor | Preserve exact pins and duplicates | Shared authoring behavior |
| M27 | Connect transfer and native reorder | Shared editor commands and ordered-record reordering | Equivalent transfers and exact saved order |
| M28 | Integrate personal collections | Pools, Drafts, Starred, Watched | Consistent Instructor collections |
| M29 | Integrate Course collections | Blueprint, active, inactive, proposals | Public shared Course interface |
| M30 | Integrate Assessment/Student views | Templates, work, scores, history | Shared role-appropriate collections |
| M31 | Integrate remaining consumers | Admin, nested views, pickers | Inventory closure |
| M32 | Retire obsolete code | Remove superseded facades/contracts/styles | Single ownership |
| M33 | Apply Instructor rubric | Shared spatial corrections | Useful content dominates |
| M34 | Apply other-role rubric | Responsive accessible corrections | Consistent all-role layout |
| M35 | Verify integrated journeys | Cross-boundary behavior and failure recovery | Complete functional acceptance |
| M36 | Run final gates/reviews | Canonical checks and independent integration | Verified combined delivery |
| M37 | Close documentation/evidence | Reconcile status, docs, temporary probes | Complete handoff |

### Authority reconciliation

**M01. Owner: manager and documentation agent. Depends on: none.**
- Deliver: align Human Guidance, architecture/terminology/design decisions with the component,
  four-mode, selection, explicit-refresh, editor, thumbnail, and rubric contracts above.
  Attribute human instructions versus approved technical decisions correctly; preserve archived sources.
- Validate/exit: review conflicts with current three-mode and cached/uncropped preview wording;
  every settled requirement has a current owner and milestone. PageFrame identity stays removed.
- Parallel-plan ready: no; establish the contract before dependent implementation.

### Consumer inventory

**M02. Owner: architecture inventory agent. Depends on: M01.**
- Deliver: inventory distinct collections across all roles, direct/nested SearchBlock and RecordList
  consumers, custom tables, pickers, ordered editors, local-complete and server-backed sources.
  Record existing capability, disconnected capability, repair, domain query owner, and assigned milestone.
- Validate/exit: reconcile code call sites with routes/workflows; every collection is assigned or has
  a functional exception and alternative verification. Render-call count is not collection count.
- Parallel-plan ready: yes, with M03; read-only ownership mapping.

### Spatial baseline

**M03. Owner: UI assessment agent. Depends on: M01.**
- Deliver: fresh representative Instructor/Student/Admin desktop and narrow captures; rubric table
  for the content before records, including page actions, SearchBlock, and collection controls.
- Validate/exit: record viewport/theme, first useful content, visible record count, overhead, and
  common-task cost; identify shared causes without freezing arbitrary pixel thresholds.
- Parallel-plan ready: yes, with inventory; serialize access to the canonical runtime lease.

### Selection transitions

**M04. Owner: shared collection state agent. Depends on: M02.**
- Deliver: shared stable-ID selection transitions; clear on keyword/filter changes; preserve across
  sort, paging, page size, and mode. Support unobtrusive opt-in bulk operations and page-local select-all.
- Validate/exit: temporary behavior probe selects on multiple pages, changes each state dimension,
  and verifies exact selected IDs and direct actions without selection.
- Parallel-plan ready: yes, with unrelated domain work; serialize shared-state edits with M05/M06.

### Refresh and edit protection

**M05. Owner: search lifecycle and guard agent. Depends on: M02, M04.**
- Deliver: explicit snapshot refresh for searches, new-tab result opening, directly affected local
  action updates, editor draft updates, and actual-unsaved-work-only guarding.
- Validate/exit: another-tab edit leaves original results unchanged until refresh; Search/Browse
  transitions do not warn; dirty editor Stay/Discard and failed-save recovery retain correct data.
- Parallel-plan ready: yes, outside shared lifecycle files; no concurrent M04 edits.

### Numbered pagination contract

**M06. Owner: shared query contract agent. Depends on: M02, M04.**
- Deliver: numbered result DTO, shared state/control transitions, deterministic order/count contract,
  complete-local adapter behavior, and domain migration map. Retain facet cursor contracts.
- Validate/exit: direct page jumps, first/middle/last/empty results, out-of-range recovery, and local
  sort/filter-before-page behavior pass. Freeze shared signatures before domain migrations.
- Parallel-plan ready: no for shared signatures; M07-M12 become independent after acceptance.

### Library query migration

**M07. Owner: Question Library query agent. Depends on: M06.**
- Deliver: numbered Library SQL/store/API/client path with counts from identical predicates;
  reuse existing sorts/directions. Move picker eligibility/exclusion before count and pagination.
- Validate/exit: real multi-page keyword/classification/column-filter query matches an independent
  complete-result oracle in both sort directions; picker pages and totals contain only eligible records.
- Accepted 2026-10-17: ROOT accepted M07 VERIFIED after fresh
  `m07_whole_actual_completion_spec` and distinct `m07_whole_actual_completion_quality` PASS. The
  independent raw-fixture SQL oracle passed 12/12 in owned run `1792197850796352000`; actual picker
  and keyword-classification API/DOM proofs are recorded in the
  [current status checkpoint](../reports/datasheet_collection_status.md#current-recovery-checkpoint---2026-10-17).
  M23's final layout and visual acceptance are recorded in the current status checkpoint.
- Parallel-plan ready: yes, with other domain queries after common DTO generation is coordinated.

### Blueprint query migration

**M08. Owner: Blueprint Course query agent. Depends on: M06.**
- Deliver: numbered public Blueprint search and personal scope bindings with complete-query ordering.
- Validate/exit: first/middle/last pages, scope/classification changes, stable ties, archive options,
  and authorized visibility match the full expected collection.
- Accepted 2026-10-17: ROOT accepted M08 VERIFIED after fresh
  `m08_m11_m12_completion_spec` and distinct `m08_m11_m12_completion_quality` passed. The corrected
  owned SQL run `1792197850796352000` passed its Blueprint oracle; no additional API/browser gate is
  required by this milestone's accepted exit criteria. M09 History/Proposals API consumer work
  remains separate.
- Parallel-plan ready: yes, within isolated Course query files.

### Proposal and history migration

**M09. Owner: proposal/history query agent. Depends on: M06.**
- Deliver: numbered proposal, revision, metadata-history and related scoped result contracts found
  in M02; reuse existing state totals only where predicates match the result query.
- Validate/exit: total/page consistency, state filters, chronological direction, empty and restricted
  cases pass through API and shared consumer binding.
- Accepted 2026-10-17: ROOT accepted M09 VERIFIED after fresh
  `m09_whole_completion_spec` and distinct `Solm09_whole_completion_quality` both passed. The
  corrected owned SQL run `1792197850796352000` passed its 12 oracles, including rollback, with
  receipt `2ec3...`, history `41423cae`, and proposal `ff533732`. Canonical actual attempt 5 passed
  on client `8054`: all history, proposal API/UI, filters, private Mary 404 concealment, route
  access denial, and 8,054-context coverage; log SHA-256
  `fba1c336f22a1a58e01383587290e37ae91fb28a460c7bfeaf8107e5c0d1e4b0`, main source SHA-256
  `f53631833d109dda738c7a5adb67a4379f22a30c91856dc00e45aa0b534e7edd`, exit 0. Temporary attempts
  1-4 retain their failed selector, scope, and 404 corrections as history; diagnosis confirmed
  Priya's two proposer-only proposals and Elena's zero without a product query defect. M09 does not
  claim all modes, 51-row UI coverage, full-suite, or visual acceptance. See the
  [current status checkpoint](../reports/datasheet_collection_status.md#current-recovery-checkpoint---2026-10-17).
- Parallel-plan ready: yes, within isolated proposal/history owners.

### Watch activity migration

**M10. Owner: Watch query agent. Depends on: M06.**
- Deliver: numbered Watch activity query/store/client with private scope and stable ordering.
- Validate/exit: multi-page activity and filters match totals; another Account cannot read the
  private collection; current action feedback remains correct.
- Parallel-plan ready: yes, within Watch query ownership.
- Accepted 2026-10-17: ROOT accepted M10 VERIFIED after fresh `m10_whole_completion_spec` and
  distinct `m10_whole_completion_quality` both passed. The owned SQL run passed all 13 oracles;
  canonical actual attempt 3 passed the required Watch and shared-consumer checks. Exact evidence
  and limits are recorded in the current status checkpoint.

### Administrative query migration

**M11. Owner: administrative query agent. Depends on: M06.**
- Deliver: numbered Instructor Account, Course inspection, and other server-paged Admin collections
  from M02, preserving POST query, privacy, and access restrictions.
- Validate/exit: query totals/order/pages and role denial pass with existing canonical fixtures;
  state counts are reused only when they describe the same complete query.
- Accepted 2026-10-17: ROOT accepted M11 VERIFIED after fresh
  `m08_m11_m12_completion_spec` and distinct `m08_m11_m12_completion_quality` passed. Accounts
  (including emitted-order proof) and Courses passed in corrected owned SQL run
  `1792197850796352000`; the accepted exit criteria require no additional API/browser gate.
- Parallel-plan ready: yes, isolated query files; serialize shared fixture/runtime access.

### Retained-work query migration

**M12. Owner: Student/retention query agent. Depends on: M06.**
- Deliver: numbered server-owned Attempt/history/recovery collections; keep complete local Student
  collections local and preserve retained-course access and partial-load retry.
- Validate/exit: authorized retained versus unavailable work, direct page jumps, total counts, and
  unaffected rows during partial recovery pass. Recovery single-choice remains separate from bulk.
- Accepted 2026-10-17: ROOT accepted M12 VERIFIED after fresh
  `m08_m11_m12_completion_spec` and distinct `m08_m11_m12_completion_quality` passed. Corrected
  owned SQL run `1792197850796352000` passed all 12/12 oracles with terminal receipt and cleanup
  verified. The accepted real-component race/partial-retry counts and choices/single-choice proof
  are recorded in [datasheet_collection_status.md](../reports/datasheet_collection_status.md#m12); component SHA-256 is
  `c0d29598dfd5ecc55dc2fa812ff06d9f779631c198634da12dddd5f2c541ba04`.
- Parallel-plan ready: yes, with isolated query owners and coordinated fixtures.

### DataSheet component extraction

**M13. Owner: shared UI architecture agent. Depends on: M04, M06.**
- Deliver: extract DataSheetControls, DataSheetBlock, HeaderRow, RecordItem and shared state from
  RecordList; retain stable field/action/query boundaries and one behavior owner during migration.
  This verified extraction retains RecordTable; M15 owns its DataSheetTable completion and naming.
- Validate/exit: representative current consumers retain working sort/filter/column/actions through
  the extraction; no competing controller or second presentation implementation is introduced.
- Parallel-plan ready: no for core files; establish ownership before mode changes.

### Compact presentation

**M14. Owner: shared table agent. Depends on: M13.**
- Deliver: single-line aligned cells, truncation, subtle stripes, dense row actions and shared
  heading sorting/filtering/resize/visibility/rearrangement with keyboard alternatives.
  Preserve this Compact work when M15 refines the existing RecordTable into DataSheetTable.
- Validate/exit: real Question rows approach practical desktop density; resize/hide/reorder and
  narrow overflow preserve identity, focus, selection, and reachable actions.
- Parallel-plan ready: no alongside edits to the same table/state/styles.

### List and DataSheetTable presentation

**M15. Owner: shared table agent. Depends on: M14.**
- Deliver: refine and name existing RecordTable as the reusable DataSheetTable for Compact/List,
  preserving M14's Compact implementation. Complete the wrapping variable-height List table,
  fewer/wider default columns, richer details, and independent temporary Compact/List widths,
  visibility, and order. DataSheetTable owns column configuration, resizing, showing/hiding,
  rearrangement, and responsive hiding. It contains HeaderRow and RecordItem instances; HeaderRow
  forwards sort/filter input to DataSheetBlock and column input to DataSheetTable without an
  independent controller. Reuse the existing collection implementation.
- Validate/exit: switching the same records produces an obvious functional distinction; settings
  round-trip independently; shared sorts/filters and keyboard interactions still work. Verify the
  Block/Table/HeaderRow ownership boundary and preserved Compact behavior without a second table
  or collection controller.
- Parallel-plan ready: no while shared table changes overlap M14.

### Cards and Visual Boxes

**M16. Owner: shared record presentation agent. Depends on: M15.**
- Deliver: text-oriented stacked Cards and responsive preview-oriented Visual Boxes, reusing
  RecordItem identity, direct actions, selection and content rendering. DataSheetBlock retains
  shared sorting/filtering/selection/action behavior across table, panel, and tile modes.
- Validate/exit: four-mode round trip preserves data/order/page/selection and performs no fetch;
  meaningful text fallback and narrow layouts work without generic placeholder-only records.
- Parallel-plan ready: no for shared mode integration; isolated preview preparation may proceed later.

### Shared surrounding controls

**M17. Owner: SearchBlock composition agent. Depends on: M05, M16.**
- Deliver: DataSheetControls outside the data surface, common modes/paging/column/bulk controls,
  headerless-mode sort/filter access, and above/beside responsive composition with page-supplied
  criteria and controls. Delegate collection interactions to DataSheetBlock and column mechanics to
  DataSheetTable through their shared state/callbacks.
- Validate/exit: controls operate one shared state; no duplicate sorting/filtering controllers or
  stacked redundant toolbars; page creation actions remain page-owned.
- Parallel-plan ready: yes, with independent backend work.

### Native record previews

**M18. Owner: Native presentation agent. Depends on: M13, M16.**
- Deliver: reuse compiled prompt blocks, question renderer and exact-revision image bindings for
  embedded real-content previews; other records use meaningful available content/media.
- Validate/exit: text, mathematics and image questions identify themselves without answer leakage,
  hover dependency, per-row detail waterfalls, or server screenshot generation for Native content.
- Accepted 2026-10-17: ROOT accepted VERIFIED after fresh `m18_native_whole_actual_completion_spec`
  and distinct `m18_native_whole_actual_completion_quality` passed. The rebuilt canonical run
  `tests/_temp/canonical_m23_runtime/actualNative_20261017T154333Z/` passed its single Native math
  image test (4.5s); the original self-authored CC0 MathML 3/7 image rendered as a 1730-byte WebP
  with matching checksum and immediate public registration/search/binding proof. No heavy native
  detail or metadata waterfalls occurred. The exact fixture title remains for M23 layout proof;
  acceptance does not promote M23.
- Parallel-plan ready: yes, with thumbnail backend work after shared preview contract is coordinated.

### Private thumbnail runtime

**M19. Owner: capture runtime agent. Depends on: M01.**
- Deliver: a separate PLE-owned runtime process using the repository-root `package.json` and
  `package-lock.json` (no nested npm package), with Playwright+cwebp, fresh contexts, shared
  document preparation, approved asset mapping, bounded admission/readiness, and centered/top-aligned
  square crop.
- Validate/exit: repeat ordinary/matching/enzyme seed-zero probes through the service; canvases and
  math paint, assets resolve, image dimensions/crop are correct, forbidden requests fail, and
  timeout/browser failure releases resources. Preserve the API's browser-free image.
- Accepted 2026-10-17: ROOT accepted VERIFIED after fresh `Solthumbnail_whole_actual_completion_spec`
  and distinct `Solthumbnail_whole_actual_completion_quality` passed. The unchanged private original
  titration Draft rendered through the actual canonical runtime at
  `tests/_temp/canonical_m22_runtime/math_paint/actual_20261017T153831Z/`: seed 0, four SVGs,
  30 glyphs, graph HTTP 200, natural 520x420. Full-page offscreen capture was blank; the normal
  scroll-to-visible viewport recapture at `math_visible_viewport/2026-10-17T15-42-37-364Z/` passed
  in 1s, with both PNGs reviewed independently and all four pK math values and graph clear. No
  product paint, CSP, CSS, source, or metadata changes were needed.
- Parallel-plan ready: yes, with collection UI; coordinate document and artifact interfaces first.

### Revision thumbnail artifacts

**M20. Owner: publication storage agent. Depends on: M19 capture contract.**
- Deliver: typed QuestionThumbnail object, initial schema and publication relation, verification,
  existing-access delivery, and batched summary reference projection.
- Validate/exit: stored lossy bytes round-trip with checksum and correct exact revision; older
  revisions remain immutable; invalid images fail; reads use existing permissions.
- Accepted 2026-10-17: ROOT accepted VERIFIED after the fresh whole M20 specification and distinct
  quality reviews passed. Actual capture generated and validated WebP before the normal API returned
  HTTP 422 for the private original's missing license; no artifact bytes were returned or published.
  The same-Draft/source/metadata checks passed.
- Parallel-plan ready: yes, once capture output and object identity contracts are fixed.

### Publication integration

**M21. Owner: authoring publication agent. Depends on: M19, M20.**
- Deliver: retain seed-zero validation HTML; capture/verify before object writes; extend initial and
  revision publication transaction/CAS and exact-candidate cleanup; align measured end-to-end deadlines.
- Validate/exit: success binds the actual source/revision; render/capture/encoding/storage/CAS failure
  leaves Draft unpublished; conclusive rollback cleans candidates and ambiguous commit retains them.
  Prove no second PG render, document concrete timeout budgets, and exercise cancellation.
- Accepted 2026-10-17: ROOT accepted VERIFIED after the fresh whole M21 specification and distinct
  quality reviews passed. Actual ordinary publication/correction, immutable revision bytes, failure
  handling, and retained Draft recovery are accepted within the milestone evidence recorded in the
  status checkpoint.
- Parallel-plan ready: no across shared publication transaction and schema edits.

### Thumbnail lifecycle acceptance

**M22. Owner: runtime/integration agent. Depends on: M21.**
- Deliver: local build/start/health/stop wiring, explicit idempotent backfill of missing existing
  revision artifacts, and ordinary UI plus automated-import publication integration.
- Validate/exit: real publish and publish-revision store/display images; rerun backfill skips complete
  revisions; service failure prevents publish then recovers; Library browsing causes zero capture calls.
  Record exact failed revisions until repaired; do not silently skip them.
- Execution: use `launchers/run_live_demo.sh` for start/stop and the controller's existing diagnostics.
  The runtime owner reuses `local_stack_control/lifecycle_provision.py`'s fixed migrator action
  `question-thumbnails backfill`. The accepted public command is
  `source source_me.sh && python3 local_stack.py backfill-question-thumbnails`, implemented in
  `local_stack_control/cli.py` and `local_stack_control/commands.py`. It resolves the existing Demo
  and reuses validated target ownership plus mutation-capability/profile checks from
  `local_stack_control/disposable_stack_adapter.py`; the fixed executor remains in
  `local_stack_control/lifecycle_provision.py`, with the private adapter in
  `local_stack_control/disposable_stack_command.py` as its reference. Absent workspace, foreign
  ownership, or invalid capability reject before mutation. A valid owned target can attempt the
  one-shot action while stopped or empty and reports an actual dependency failure. Retain resource
  ownership and runtime serialization. Use fixed argv without new selectors, configuration, custom
  binary, or temporary approval file.
- Command proof: source specification and distinct quality reviews accepted target validation,
  fixed argv, and failure propagation. Actual preparation and repair used the existing project-tools
  migrator. Two new historical Numeric PG Revision 1 fixtures exercised absent original bytes from
  inception (`9ZSF-HY9G`) and verified stored originals (`AYTW-B2AB`). The public command exited 2,
  reported the first revision's source failure, and created the independent revision (1 created,
  266 already complete, 1 failed). After repairing only missing original bytes, rerun exited 0
  (1/267/0); a third invocation exited 0 (0/268/0). Capture admission/completion counts were
  266 -> 267 -> 268 -> 268. Fresh actual SPEC and distinct QUALITY accepted this bounded proof.
- Accepted backfill scope: actual import/backfill delivery, repair, idempotence, and exact immutable
  revision bytes passed the whole M22 review. Additional SQL-registration probes and private Web
  prelay are not completion gates. M37 retired the one-time fixture module, command arms, Python
  fixture actions/capabilities, and build step after their proof was complete.
- Runtime checkpoint: ROOT accepted M22 VERIFIED after fresh whole-specification and distinct
  quality reviews. Actual UI publication/correction, immutable 256-square WebP browser delivery,
  import/backfill recovery, zero-capture browsing, private enzyme negative-publication, and outage
  recovery passed. See the current status report for exact runs and limitations.
- Accepted 2026-10-17: ROOT accepted VERIFIED after fresh `Solthumbnail_whole_actual_completion_spec`
  and distinct `Solthumbnail_whole_actual_completion_quality` passed for the complete M19-M22
  lifecycle, with each milestone separately reviewed. See the status checkpoint for runtime receipts,
  startup recovery, image limits, and the exact scope of accepted behavior.
- Failure action: preserve redacted startup/service logs and existing drafts/results; correct the
  demonstrated failure at its owner and rerun the affected lane. Readiness, publication, backfill,
  and M23 are separate outcomes; a successful start alone does not verify this milestone.
- Parallel-plan ready: yes, with isolated UI work; serialize canonical runtime lease.

### Three-page architectural proof

**M23. Owner: independent architecture and browser agents. Depends on: M03-M07, M13-M18, M22.**
- Deliver: current Search Question Library, Browse Question Library, and My Questions all consume
  the same SearchBlock/DataSheetControls/DataSheetBlock/DataSheetTable/HeaderRow/RecordItem owners.
  Compact/List share DataSheetTable; HeaderRow is inside it and RecordItem is used in every mode.
- Validate/exit: source ownership plus actual API/DOM evidence proves all four modes, distinct table
  layouts, query-wide sorting/filtering, numbered jumps, selection transitions, no-fetch mode changes,
  real previews, initial empty search, ordinary navigation, new-tab snapshots and narrow/keyboard use.
  Independent visual review applies the rubric; no three page-specific substitutes pass.
  Preserve the earlier proof's Native publication cases with no self author and explicit self author,
  authored-but-nonowned Questions safely appearing in My Questions, feedback loading, and guarded
  source edits. Authorship never silently grants ownership or edit permission.
- Evidence scope: exercise shared numbered paging and query-wide ordering on the existing multi-page
  Library dataset. Prove My Questions uses actual authorship scope and the same query/UI with small
  fixtures. Do not require more than 50 authored copies, a hard fixture-size threshold, or an
  exhaustive per-page matrix. Actual three-page run `actual_20261017T133548Z` passed; the separate
  authorship case also passed. Final layout evidence is `current_layout_actual/selected_capture/attempt5/`
  (built-in selected capture, all five staged checks passed in 31.1s on client `8054`, with no
  container replacement) and `layout_proof/attempt6/` (fresh layout specification passed in 47.8s;
  all five PNGs, metrics, private origin, and raw logs retained). The final viewport probe measured
  Search/My Questions first-row positions at 389.64px versus the earlier 493.81px, Browse at
  260.14px with 32 rows, and narrow 393x852 at 20px root size with `scrollY=0`, first row 692.16px,
  and seven visible rows versus 919.53px and none before. Native classification summary,
  Find/focus/keyboard behavior, native Search filters default-closed/on-demand, callback with no Pool,
  chips, value preservation, and no-fetch baseline all passed. The initial narrow open state was
  horizontally clipped; only post-focus keyboard reachability was tested and accepted. No numeric
  post-focus viewport claim is made. My Pools showed 228 records across five pages; this is
  populated-state evidence, not an empty-data test. Fresh `m23_whole_current_completion_spec` and
  distinct `m23_whole_current_completion_quality` both passed; ROOT accepted M23 VERIFIED. See the
  current status checkpoint for limits and historical failed temporary probes. M07, M18, and M22
  are verified there; M05 is verified from actual Discard proof.
- Parallel-plan ready: no for acceptance; this is the architectural gate before broad integration.

### Question Pool panels

**M24. Owner: `/root/m24_pool_panels_implementation`. Depends on: M23.**
- Deliver: parallel independent current/available SearchBlocks with direct Add/Remove, Included state,
  all modes, compatible starting Questions, and existing membership draft/save/reload behavior.
- Validate/exit: search changes do not mutate membership; matching included records stay visible;
  invalid additions and conflicts retain work; exact revisions and existing member restrictions persist.
- Earlier attempts recorded an unrelated TypeScript error, later corrected by its source owner, and a
  browser failure because the TEMP request helper omitted the fourth page-size argument and received
  default 50. These are historical failures. M24 is now verified after canonical actual attempt 3 and
  fresh whole-completion SPEC, distinct QUALITY, and ROOT acceptance; see the
  [current status checkpoint](../reports/datasheet_collection_status.md#current-recovery-checkpoint---2026-10-17).
- Parallel-plan ready: yes, with Assessment owners when shared interfaces remain unchanged.

### Assessment panels

**M25. Owner: `/root/m25_assessment_panels_implementation`. Depends on: M23.**
- Deliver: parallel panels, direct actions, searchable/filterable saved-order contents with original
  position numbers, disabled left display sorting, and explicit entry-ID-based reorder. Complete the
  C16 final RecordList consumer on the same SearchBlock/DataSheet architecture, including aggregate
  points edits across filtered projections, exact pins/order/Pool settings, keyed invalid drafts and
  saves, the unsaved-change guard, and compare-and-swap conflict/Reload behavior.
- Validate/exit: filtered reorder changes the intended full-order entries; save/reload preserves pins,
  scoring and sequence; right Library search remains independent and respects duplicate policy. C16
  preserves full positions, keyed drafts, and aggregate saves, while Compact retains direct ordinary
  details with unique input identities and saved order without display sorting.
- Corrected panel source passed SPEC and distinct QUALITY after the review identified staged Pool Add
  and disabled actions while dirty. The correction appends directly with defaults and per-entry count
  points, scoring, duplicate-label and capacity handling. M25 is verified after all ten panel actual
  stages, corrected retained-tail actual attempt 3, C16 canonical actual attempt 5, shared Search
  CSS/Browse actual attempt 3, fresh `m25_whole_completion_current_spec` and distinct
  `m25_whole_completion_current_quality` PASS, and ROOT acceptance. The status report records exact
  runtime evidence and limits.
- Parallel-plan ready: yes, with Pool work; coordinate any shared Assessment model with M26.

### Blueprint Assessment editor

**M26. Owner: Blueprint Assessment editor agent. Depends on: M25.**
- VERIFIED 2026-10-17 after fresh `/root/m26_whole_completed_spec`, distinct
  `/root/m26_whole_completed_quality`, canonical actual attempt 4, and ROOT acceptance. The run
  passed the non-first Assessment in all four modes, independent facets/search/pagination, exact
  repeated-entry and Pool editing, save/reload, stale-revision recovery, and narrow keyboard stages.
  See the status checkpoint for receipt hashes, historical failures, and the actual4 asset-identity
  limit.
- Deliver: reuse shared Assessment composition in Blueprint authoring, retaining its exact references,
  scoring, permitted duplicate entries, revision/concurrency semantics and page-owned commands.
- Validate/exit: repeated permitted Question entries remain distinct by entry ID; search/filter and
  mode changes preserve order; save/reload and stale revision recovery pass.
- Parallel-plan ready: yes, outside shared Assessment model edits.

### Drag-and-drop transfers

**M27. Owner: shared record interaction/editor agent. Depends on: M24-M26.**
- ACTIVE 2026-10-17; released to `/root/m27_shared_native_interactions` after M24-M26 were verified
  and C52 source frozen. Its early checkpoint is pending. ROOT-serialized actual work follows the
  fresh SPEC and distinct QUALITY reviews.
- Deliver: restore native ordered-record drag reordering in the shared DataSheet/RecordItem owner,
  using existing reorder utilities, and add drag/drop transfers between editor panels through the
  same Add/Remove command paths, with clear valid targets, duplicate/eligibility rejection, and
  direct keyboard-button equivalents. Keep both interactions in the shared owner.
- Reuse the existing `ControlledRecordReorderProvider` with an optional binding that supplies
  `orderForRecord(recordId)` as the full current authored sibling ID order, `onMove(sourceId,
  targetId)`, `recordLabel`, and optional `isDisabled`. Ordinary records return their full order;
  C52 Assessment records return their own Module's full sibling order; a missing ID returns an
  empty order, and cross-Module placement uses its existing explicit command. Keep a transfer local
  to one editor, keyed by panel and stable record ID, and apply it through the live editor command.
- Keep drag handles in shared `RecordItem` across all four modes and DataSheetCollection, including
  an explicit empty-panel target. Add/Remove buttons and dragging use the same domain command. Clear
  pending gesture, transfer, and reorder sessions before executing an editor command so the same drop
  cannot execute twice. Preserve both panels' search sessions and query state. Use existing
  SearchSession state-object identity to invalidate a pending gesture when either panel changes
  results, loading, error, or unmount; a replacement revision or page-generation prop does not
  replace that lifecycle. HeaderRow column dragging remains separate.
- Implementation ownership proceeds from the shared interaction owner to consumer integration,
  then fresh SPEC, distinct QUALITY, and ROOT actual acceptance. M27 is active after its dependency
  and source-freeze release; do not claim actual acceptance before the ROOT runtime gate.
- VERIFIED 2026-10-17 after fresh `m27_whole_final_spec` and distinct
  `m27_whole_final_quality` PASS, plus ROOT acceptance of native actual 15, nested actual 14, and
  the existing component-lifecycle supplement. Shared native reorder, keyboard operations, and
  direct command equivalents are complete; detailed bounded evidence and current limits are in the
  [status report](../reports/datasheet_collection_status.md#current-recovery-checkpoint---2026-10-17).
- Validate/exit: reorder preserves the exact record order; drop/buttons produce equivalent drafts
  and validation, no unexpected save, and no transfer to the wrong editor or entry when
  results/filtering change. Validate that both source and target are currently present and enabled
  through the live validator for keyboard, button, and native drag paths. Preserve native reorder
  and verify keyboard equivalents without a page-specific drag controller.
- Parallel-plan ready: no with edits to the same record interaction and editor command files.

### Personal Question collections

**M28. Owner: Instructor collection agent. Depends on: M10, M23.**
- Deliver: My Pools, My Drafts, Starred and Watched inherit shared modes/state;
  correct Ribbon links and route reachability; preserve private scope and bulk metadata operations.
- Validate/exit: each inventory route exercises modes, direct actions, query changes and role checks;
  drafts retain edit protection and bulk operations remain optional.
- Accepted VERIFIED 2026-10-17 after fresh whole-milestone SPEC and distinct QUALITY PASS, canonical
  actual attempt 4, and positive Watch Open supplement actual attempt 2; details are in the current
  status checkpoint.
- Parallel-plan ready: yes, with isolated Course/Student consumers; coordinate Ribbon edits.

### Course collections

**M29. Owner: Course UI agent. Depends on: M08, M09, M23.**
- Deliver: Search Blueprint Courses, My Blueprint Courses, active/inactive Course collections and
  My Change Proposals share components; public Blueprint semantics and Cards search default;
  remove duplicate names/empty tags. My Change Proposals belongs in the Courses Level-2 ribbon.
- Validate/exit: route/Ribbon/breadcrumb coherence, all modes, Course create/open actions, full-result
  sort/filter and narrow layout pass while preserving dates, adoption and revision meaning.
  Verify the Courses ribbon proposal link and absence of a redundant My Change Proposals link in
  the My Blueprint Courses page body, including when the proposal count is zero.
- Accepted VERIFIED 2026-10-17 after fresh whole-milestone SPEC and distinct QUALITY PASS, canonical
  actual attempt 5, and inactive-route supplement actual attempt 2; details are in the current
  status checkpoint.
- Parallel-plan ready: yes, with isolated consumers and one Ribbon owner.

### Assessment and Student collections

**M30. Owner: Assessment/Student UI agent. Depends on: M12, M23.**
- Deliver: shared Templates/Assessment, Coursework, scores/progress/history and retained-work views
  from the inventory, preserving task-primary Start/Resume, export and recovery actions.
- Shared optional initial mode flows through SearchBlock, Results, DataSheetBlock, and state, defaulting
  to Compact. Cards preserves the consumer body. M29's released source defaults public Blueprint
  Course callers to Cards and personal callers to Compact; M29 is VERIFIED after its canonical and
  inactive-route actuals and whole-milestone reviews passed.
- Validate/exit: all applicable modes/query behavior work with privacy, partial retry, retained access,
  single-choice recovery and meaningful task context intact.
- Parallel-plan ready: yes, with disjoint Instructor/Admin integration.

### Remaining consumers

**M31. Owner: inventory completion agent. Depends on: M09, M11, M12, M23.**
- Deliver: migrate every remaining Admin, Account, history, statistics, embedded picker, avatar,
  nested collection and other M02 row; implement shared behavior wherever applicable.
- Validate/exit: inventory has no unassigned consumer; every exception states a fundamental workflow
  reason and verified interaction, not historic styling or component identity.
- Parallel-plan ready: yes, dispatch disjoint consumers after resolving common contracts.

### Obsolete implementation removal

**M32. Owner: shared architecture cleanup agent. Depends on: M24-M31.**
- Deliver: remove replaced RecordList facade, old mode/column controllers, result cursor interfaces,
  redundant page scaffolding and unused styles/inputs; keep genuine facet cursors and domain logic.
- Validate/exit: code search and dependency/type checks find one owner for each shared capability,
  no migrated consumer still uses removed APIs, and no compatibility suppression layer remains.
- Parallel-plan ready: no until all dependent consumers have migrated.

### Instructor spatial acceptance

**M33. Owner: UI/HCI assessment and shared layout agents. Depends on: M32.**
- Deliver: compare current Instructor pages with M03; remove unjustified preludes at shared owners,
  consolidate common controls, preserve frequently needed actions and meaningful domain context.
- Validate/exit: fresh desktop/narrow/theme/zoom captures plus task walkthrough automation and
  independent visual review show improved content visibility without increased common-task friction.
- Parallel-plan ready: yes for assessment, serialize shared layout fixes with M34.

### Other-role spatial acceptance

**M34. Owner: UI/accessibility and shared layout agents. Depends on: M32.**
- Deliver: same rubric across Student, Admin and other applicable roles, including nested workflows.
- Validate/exit: breadcrumbs lead into useful work; focus, labels, zoom, overflow, theme, actions and
  selection remain usable; all retained pre-content text has a specific task justification.
- Parallel-plan ready: yes for assessment; shared changes use one owner with M33.

### Integrated behavior acceptance

**M35. Owner: independent integration agent. Depends on: M27-M34.**
- Deliver: full journeys across search, personal collections, editors, publication, Course use,
  Student access and administrative recovery; reconcile every inventory/requirement evidence row.
- Validate/exit: real UI/API/storage outcomes and recoverable failures pass together, including
  publication thumbnails, complete-result sorting, exact pins, editor conflicts and role boundaries.
  Component tests and static audit alone cannot satisfy this gate.
- Parallel-plan ready: no for combined runtime acceptance; use the canonical fixture/lifecycle owner.

### Final verification and review

**M36. Owner: manager, verification agent, fresh specification and quality/integration reviewers.
Depends on: M35.**
- Deliver: current canonical fast/full/screenshot gates, regenerated schema/transport docs where
  changed, and independent review of architecture, behavior, test value and complete plan coverage.
- Validate/exit: record exact commands/exits and evidence provenance; repair meaningful failures and
  rerun affected gates. Fresh specification passes before a distinct quality/integration reviewer.
  Do not label focused checks, builds or old captures as fresh full/runtime acceptance.
- Parallel-plan ready: no for stack-resetting gates; read-only reviews may inspect frozen evidence.

### Documentation and evidence closure

**M37. Owner: manager and documentation agent. Depends on: M36.**
- Deliver: reconcile architecture, contracts, terminology, guidance, runtime instructions, schema docs,
  changelog, status and current-work index; retain concise evidence receipts and meaningful captures.
- Validate/exit: all 37 milestones are verified with review/behavior evidence; no required unfinished
  item is hidden in follow-ups. Remove one-time executable probes after retaining evidence, promoting
  only justified stable tests. Report completion, residual risks and local runtime state.
  Keep concise commands/results, relevant failure diagnostics, build identity and useful captures;
  old whole-source copies and hash matrices are not required evidence. Before temporary cleanup,
  identify anything still needed for active M22/M23 reproduction and preserve the current workspace.
- Parallel-plan ready: no; closure follows integrated acceptance. Archival Git moves are not required.

## Test and verification strategy

Read [PYTEST_STYLE.md](../../PYTEST_STYLE.md) and
[PLAYWRIGHT_TEST_STYLE.md](../../PLAYWRIGHT_TEST_STYLE.md) before adding or changing tests.
Use existing behavior coverage where it protects the new contract; remove obsolete expectations
rather than adding production compatibility solely to satisfy a test.

Permanent candidates require an explicit regression-value rationale: whole-query paging/sorting,
selection transition semantics, dirty-editor protection, exact-revision atomic publication, and
existing privacy boundaries. Prefer extending a focused existing test. No test-per-component rule,
fixed column defaults, precise row-height snapshots, animated-pixel equality, or permanent probe corpus.

Temporary experiments and browser scripts stay under ignored `tests/_temp/`. Record exact fixture,
bundle/source identity, viewport/theme, commands, exit codes, logs and captures. Preserve failed
attempt receipts and distinguish fixture failure from product failure. Agent-operated browser
automation and independent screenshot assessment replace human walkthrough gates.

Use exact record/revision IDs and source/object checksums where identity, immutable storage, CAS,
or byte-preserving image delivery requires them. Record which build served a browser run; invalidate
only evidence affected by later changes. A blanket all-source hash freeze, pixel-byte comparison,
or every-consumer-by-every-theme/mode matrix is not a completion requirement. Verify each inventory
consumer's actual integration and permissions, then exercise shared behavior on representative
records, including a non-first result/page and genuine failure recovery. Use desktop, narrow, zoom,
and theme evidence where it can expose distinct layout or access failures. Roughly 20px Compact
rows remain a practical density target; historical 20.5px measurements are observations. The fixed
seed-zero, 256 x 256, quality-80 thumbnail and crop contracts remain required.

Before creating a custom harness, test overlay, or build variant, inspect the existing launcher,
API/query, fixture producer, and browser/screenshot routes for the needed behavior. Extend the
owning implementation only when an actual requirement is missing. One-time checks protect useful
results, drafts, permissions, atomicity, and recovery; they do not require unrelated production
changes. If a temporary overlay is needed, retain and verify restoration of every touched original
file and report failed restoration immediately. Never weaken a product requirement merely because
its first proof was cumbersome.

Canonical execution and the failure response:

| Outcome | Existing entrypoint and meaningful evidence | Owner and failure action |
| --- | --- | --- |
| Demo readiness | `launchers/run_live_demo.sh --headless`; controller readiness URL/status and redacted service logs | Runtime owner preserves the failing stage/cause, repairs it, and retries only affected startup work; unresolved startup blocks connected claims |
| Connected browser behavior | `devel/run_playwright_tests.sh` for existing scenarios; its existing Python owner also accepts `--running --scenario ID` for focused registered work; remaining plan behavior uses focused actual API/browser proof | Owning milestone agent corrects the failing behavior or demonstrated fixture defect; retain valid results/drafts and replay the affected journey |
| Spatial evidence | `devel/capture_screenshots.sh --only IDS` uses current manifest IDs and stages selected captures without publishing; independent visual review applies the rubric | UI owner repairs concrete obstruction, overflow, contrast, or access failures and recaptures affected views |
| Missing-artifact backfill | M22's fixed migrator action through the canonical lifecycle owner, followed by revision/object/delivery checks | Runtime owner completes any missing supported invocation/fixture, records per-revision failures, and reruns after repair |
| Offline compliance | `launchers/run_fast_checks.sh` | Owning code/schema agent corrects a real failure, regenerates canonical outputs when needed, and repeats affected checks |
| Final aggregate | `launchers/all_test.sh`; its final `local_stack.py acceptance` is the disposable PostgreSQL baseline | Verification owner retains failure evidence and repairs/replays the affected gate; browser and thumbnail acceptance remain separately required |
| Runtime closeout | `launchers/run_live_demo.sh stop` when the serialized runtime work ends | Runtime owner records cleanup result and remaining resources; diagnose incomplete cleanup through the controller |

Run these commands through `source ./source_me.sh`. The screenshot publisher's unqualified command
below updates the canonical corpus; use its selected staging mode during focused proof. Its
`--fresh` option resets the owned Demo, so serialize that with fixture-dependent work. Do not add a
new latency, version, or configuration gate. Publication deadlines come from the actual render/API/
gateway/client budgets specified in M21, and failure must cancel/release resources and preserve the
editable Draft. Tool approval remains the normal execution boundary; a rejected built-in action is
reported with its reason rather than retried through a different route.

For a registered browser scenario on the running Demo, use
`python3 tests/e2e/e2e_live_demo_production_browser.py --running --scenario ID`, selecting an existing
ID from its registry. This reuses the private runtime inputs and production-browser owner. Finish
fixture-dependent browser/storage evidence and stop the Demo before `launchers/all_test.sh`:
`local_stack.py acceptance` correctly refuses conflicting default/fixed-Demo containers.

For an accepted one-time browser proof, use the same owner and existing private schema-2 runtime,
origin, persona, and CA inputs with:

```bash
source source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --running --spec tests/_temp/<proof>.spec.ts
```

The proof file must be one contained regular file and use the exact accepted matcher. Keep the default
scenario registry unchanged; this path adds no permanent test or approval file. The accepted seeded
persona-2 declaration uses the same temporary-proof boundary. Actual M22 publication/backfill and M23
runs now exist; their bounded acceptances and open requirements are recorded in the status report.
M23 whole-milestone SPEC and distinct QUALITY passed; ROOT accepted M23 VERIFIED.
M01-M37 are verified. The status report preserves bounded proof limits and historical failures;
the historical driver is retired.

Canonical verification, run by agents with the existing lifecycle and resource lease:

```bash
source ./source_me.sh && ./launchers/run_fast_checks.sh
source ./source_me.sh && ./launchers/all_test.sh
source ./source_me.sh && ./devel/capture_screenshots.sh
```

For schema changes, regenerate and verify at the schema owner:

```bash
source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py
```

Use the existing canonical transport generator (`cargo tsgen`) when Rust transport types change.
Generated files are outputs, not separately edited authorities. Run narrow gates per milestone,
then the final combined canonical gates. Repeat broad checks after new changes or unresolved failures,
not as an idle ritual. No new permanent tests are needed merely to save this plan.

## Migration and compatibility policy

PLE is pre-production. Update initial SQL and generated contracts directly and remove obsolete
result-cursor APIs once all consumers migrate. Preserve facet pagination where it still serves the
distinct choice-search purpose. No dual pagination API or persistent compatibility facade is required.

Introduce shared extraction before migrating consumers; any temporary facade has M32 as its explicit
removal milestone. Reuse working implementation during transition rather than retaining two owners.

Use canonical local schema/runtime refresh procedures for disposable fixtures, preserve evidence
before refresh, and recreate required fixtures through approved existing producers. Backfill
missing immutable thumbnail artifacts explicitly and idempotently. Do not reinterpret a failed
capture as a successful publication or invalidate existing published Question sources.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Responsibility drift | Another incomplete abstraction | Page-specific state/controls emerge | Shared architecture owner | Repair owner boundary; reject duplicate controller |
| Page-only query behavior | Wrong order/count or missing candidates | Filtering after remote pagination | Domain query owner | Identical full predicates and independent query oracle |
| Stale source/artifact association | Wrong published preview | Concurrent edits or CAS failure | Publication owner | Capture bound source; exact final transaction; failure injection |
| Incomplete object cleanup | Orphans or deleted committed content | Early prep error or ambiguous commit | Publication owner | Track candidates; clean only conclusive failures |
| Missing JS/assets or sandbox loss | Blank/incorrect capture or isolation regression | Canvas zero, CSP/network errors | Capture owner | Shared preparation, strict asset mapping, actual complex probe |
| Publication deadline mismatch | Client fails despite continued backend work | Old 30-second gateway expires | Runtime/publication owner | Measured full budget, nested deadlines and cleanup proof |
| Excessive chrome | Content still displaced | Toolbars replace removed headings | Shared UI owner | Rubric and before/after task/capture assessment |
| Saved-order corruption | Assessment semantics change | Filter-index reorder | Editor owner | Stable entry IDs, full-order mapping and save/reload proof |
| Fixture interference | Invalid evidence and flaky recovery | Concurrent canonical lifecycle use | Manager | One lease; serialize destructive runtime gates |
| Test/configuration expansion | Long-term maintenance burden | Mirrored implementation tests/options | Manager/quality reviewer | Require demonstrated need; prefer temporary checks |
| Plan drift or hidden unfinished work | False completion | Missing consumer or skipped gate | Manager/integration reviewer | Ledger and requirement/inventory reconciliation |

## Rollout and release checklist

- [x] Save and independently review this complete plan before production edits.
- [x] Reconcile current authorities and inventory; retain earlier accepted plan history.
- [x] Establish shared contracts and implementations before broad page migration.
- [x] Pass M23 on the three real collection pages.
- [x] Complete every applicable consumer and editor, including drag/drop and direct actions.
- [x] Verify private capture deployment, UI/import publication, failure recovery and explicit backfill.
- [x] Remove obsolete collection interfaces (M32).
- [x] Complete the content-first rubric across roles (M33-M34).
- [x] Pass current integrated/canonical gates and independent reviews.
- [x] Close evidence and documentation; report genuine limitations without marking them complete.

All checklist operations are local and agent-executable. Production deployment, credentials supplied
by a sleeping human, human screenshots, and Git/index actions are not prerequisites for acceptance.
If an actual environment dependency cannot be recovered through existing local tooling, record its
specific cause and affected milestones, continue independent work, and report the genuine blocker.

## Documentation close-out requirements

- Maintain this plan and [datasheet_collection_status.md](../reports/datasheet_collection_status.md)
  as the requirement and execution record. Assign actual agent names at dispatch.
- Update [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md),
  [TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md), [CONTRACTS.md](../../CONTRACTS.md),
  [CODE_ARCHITECTURE.md](../../CODE_ARCHITECTURE.md) and relevant search/runtime guides at their owners.
- Keep [current_work.md](current_work.md) accurate and add bounded entries to
  [CHANGELOG.md](../../CHANGELOG.md) after their narrow gates.
- Retain the renderer findings in durable completion evidence; temporary local files may not survive
  cleanup, so future readers must not require them to understand the accepted contract.
- Close temporary executables separately from evidence receipts; retain no new permanent test
  without a behavior-level rationale. Record current local stack/resource state.
- Preserve archived acceptance history; this plan's verification never follows merely from the old
  plan's completion. No Git move is necessary to finish the documentation.

## Open questions and decisions needed

No product decision or human approval is required to start or complete this approved scope.

The manager and appropriately capable architecture reviewers own planning precision and any
cross-component changes. Luna agents receive bounded coding tasks after decisions are complete.
Fresh reviewers independently assess implementation; an implementer does not certify its own work.

Implementation investigations are owned deliverables: M02 resolves consumer classification from
source/workflow evidence; M19 validates bounded runtime/capture settings; M21 measures and sets
concrete publication deadlines. Each ends in an implemented, reviewed decision before dependents
pass. Choose the smallest existing pattern satisfying the contracts. Escalate a genuine blocker
with evidence; do not add a human gate for routine implementation choices.
