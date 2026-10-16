# Plan: Complete PLE&#8217;s modular collection interface

## Context

The revised [Human Guidance](../HUMAN_GUIDANCE.md) defines three distinct responsibilities:

**PageFrame &#8594; SearchBlock &#8594; RecordList**

At the approved-plan baseline, the October modularization provided the foundation, but PageFrame still required headings, display controls sat in search controls, spreadsheet column interactions were missing, and ordinary searches warned on leave. The [ledger](../active_plans/audits/modular_ui_completion_ledger.md) records the later implementation and verification state.

This replacement plan follows the Blueprint planning template. It incorporates the completed source audit and independent plan review.

For current milestone status and evidence, see the [modular UI completion ledger](../active_plans/audits/modular_ui_completion_ledger.md). The ledger is the single status record; this plan preserves the approved M1-M32 scope and gates.

## Current acceptance checkpoint (2026-10-16)

The approved M1-M32 scope and gates remain unchanged. The [ledger](../active_plans/audits/modular_ui_completion_ledger.md) is the single current status record: M1-M32 are verified (32 verified, zero active or pending). The verified local build included the M26 proposal timestamp, M31 Student-retention, and M30 recovery DTO repairs. The Demo was stopped for final aggregate acceptance; the post-full receipt records zero containers.

M26 completed native proposal, ordering, stewardship, fork, and history proof. M30 completed strict Recover selection and recovery at HTTP 200. M31 completed its combined fresh specification and quality review: current Student query/partial-retry, Sysadmin Discipline-guard, Question Use, update-review, and privacy-safe five-Student statistics API/DOM proof from attempt `1792136066729026000`, plus the native inactive-retention proof from `1792136703798793000`. That retention proof uses canonical current-clock lifecycle transition for Course `CIJAAXY2YT`: Mary retains list and landing HTTP 200 while retention remains active; archived Course `CIM64DK7QF` is absent and its landing returns 404.

M32's technical gates passed. Current Search/Browse/My Questions native attempt `1792137381897594000` on bundle `723f7c8c` passed fresh specification, distinct quality, and three-image visual review; actual queries, DOM/shared controls, 20.5px Compact rows, and byte-exact overlay restoration are recorded under `tests/_temp/modular_ui/phase_c/m28/attempt_1792137381897594000`. The current canonical Templates capture and privacy/checkpoint receipt are preserved under `tests/_temp/modular_ui/phase_c/final_gates/evidence/`. Source architecture and removal of two duplicate sort producers passed fresh specification and quality review; RecordList owns sort controls and invokes domain query bindings.

Canonical fast attempt 3 and full checks exited 0; `tests/_temp/modular_ui/phase_c/final_gates/fast_receipt.json` and `full_receipt.json` record clean schema style, 410 generated contracts, Rust 41 default and 41 all-feature tests, Node 24, and Python 9,205. The full gate additionally passed the disposable PostgreSQL schema/authority/persistence oracle; native browser and visual acceptance remain their separate evidence. Cleanup removed exactly 75 approved one-time operative files (807,322 bytes); all 1,850 retained artifacts and 92 permanent test/support files matched their cleanup baselines. Reports, logs, images, JSON receipts, and 355 archived source snapshots remain. Fresh `/root/final_plan_integration_spec` and distinct `/root/final_plan_integration_quality` both passed with no findings. The manager accepted M32 and all 32 milestones on 2026-10-16. Independent final quality checks confirmed all 92 permanent baselines unchanged, all 75 approved operative paths absent, and all 1,848 immutable retained artifacts unchanged. The two `FINAL_REPORT.md` evidence indexes remain authorized mutable records; the 1,850-artifact hash match above describes the cleanup checkpoint.

The completed plan was archived on 2026-10-16 using the user-authorized `git mv`; this preserves the approved scope and gates below.

## Objectives

- Complete the shared architecture and prove it end to end through Search Question Library, Browse Question Library, and My Questions.
- Provide shared Compact, List, and Visual boxes modes with sorting, filtering, column management, selection, and actions.
- Extend the completed behavior to every applicable collection.
- Remove redundant page content and search-based navigation warnings while protecting meaningful editor changes.
- Finish through automated checks and independent agent assessments, without human approval or visual-review gates.

## Design philosophy

Apply **KISS** and **fix the design that causes the problem**, as specified in REPO_STYLE.

Extend the existing record and search components at their owning boundaries. Pages describe domain criteria, fields, and actions. They inherit presentation and interaction behavior.

Use focused experiments when implementation details remain uncertain. The manager selects the smallest implementation that passes the stated behavior checks; uncertainty does not become a deferred requirement.

## Scope

- Complete PageFrame, SearchBlock, RecordList, and their integration.
- Repair query contracts, facets, and cursor handling where required for complete-result sorting and filtering.
- Update personal collections, Library discovery, Courses, Assessments, Student collections, and administration.
- Add My Pools, update Ribbon terminology, and support multiple starting Questions for Pool creation.
- Complete editor protection, accessibility, responsive behavior, documentation, and verification.

## Non-goals

- Replace the existing component family or search engine.
- Persist search history or column preferences.
- Change grading, authorship, ownership, publication, or authorization semantics.
- Require production deployment, external accounts, participant recruitment, or human sign-off.

## Approved baseline summary

| Requirement | Current support | Required work |
|---|---|---|
| Content-first PageFrame | Shared layout and spacing exist; heading markup is mandatory. | Remove heading-related features and clean up duplicated consumer scaffolding. |
| SearchBlock | SearchSession, controls, results, and SearchPage exist. | Separate search composition from PageFrame; require RecordList. |
| My Questions as a predefined search | Authenticated authorship filtering already works. | Replace its separate paging/presentation composition. |
| Three display modes | Rendering and selector exist; integration is concentrated in search. | Move shared controls and presentation ownership into RecordList. |
| Spreadsheet columns | RecordTable supplies semantic headers and cells. | Add sortable/filterable columns, resizing, visibility, rearrangement, and dense styling. |
| Multi-value filters | Several API fields and whole-query aggregates already support them; UI adapters often expose single values. | Connect existing capabilities; extend classification and Bloom contracts where necessary. |
| Initial empty Search spreadsheet | SearchSession has an initial state. | Keep headers and filters mounted; prevent criteria-free result requests. |
| Search/Browse layout | Both use the same Library results. | Choose layout explicitly rather than changing it after searching. |
| Unsaved-edit protection | Shared guard exists. | Remove search activity protection and connect actual editor changes, including manual selection changes. |
| My Pools | Mixed Library search supports Pool owner filtering. | Add route, Ribbon entry, breadcrumb, capability mapping, and shared search composition. |
| Multiple starting Questions | Server creation accepts multiple members and validates compatibility. | Extend the singular starting-Question UI contract and selection action. |
| Public Blueprint Courses | Availability is already `public` or `archived`; no private visibility mode exists. | Update terminology while preserving archive lifecycle behavior. |

This table records the audited baseline that motivated the approved plan. Use the ledger for current source status and remaining gates.

## Architecture boundaries and ownership

**PageFrame**

Own layout, spacing, actions, and content. Remove `title`, `eyebrow`, `lede`, heading IDs/tabindex, and their markup and accessibility references. Do not retain a hidden PageFrame heading.

ApplicationShell retains main-content focus and derives its accessible name from the terminal breadcrumb, including dynamic record labels. Genuine workflow headings and status messages remain with their content owners.

**SearchBlock**

Replace SearchPage&#8217;s composition with a block inside PageFrame. Its interface requires:

- An explicit `above` or `beside` controls layout.
- Page-supplied controls connected to the existing search state.
- A typed collection definition rendered through RecordList.

SearchBlock cannot accept arbitrary replacement results. RecordList owns selection controls and bulk actions. Actual editors and dialogs remain siblings inside PageFrame, with editor-owned dirty state.

Reuse SearchSession for request ordering, paging, retries, and selection. Complete-array collections use a shared local adapter; paginated collections retain server-owned queries.

**RecordList**

Own record presentation, mode controls, column interactions, filtering controls, sorting controls, selection, and record actions.

Extend the existing model with stable field IDs, labels, typed values/renderers, sorting/filter bindings, and default column configuration. Do not infer columns from rendered text or whichever records happen to load first.

Use RecordTable for Compact. Retain the existing semantic siblings for authored sequences, hierarchies, response controls, and comparisons.

**Domain query owners**

Own matching semantics, authorization, whole-result filtering/sorting, facet values/counts, and pagination. RecordList initiates these operations through typed bindings; it does not sort a server page and present that as collection-wide ordering.

**UnsavedChangesGuard**

Remain the shared protection mechanism. Editors report actual changed values and save outcomes. SearchBlock and RecordList do not classify search activity as unsaved work.

### Mapping (milestones -> components)

| Milestone | Component | Review boundary |
|---|---|---|
| M1&#8211;M3 | Planning evidence, PageFrame, ApplicationShell, consumers | Hierarchy, accessibility, retained workflow context |
| M4&#8211;M11, M14 | RecordList family and shared search composition | Shared contracts, interaction, responsive behavior |
| M12&#8211;M15 | Library query, classification, facets, sorting | Complete-result correctness and authorization |
| M16&#8211;M18 | UnsavedChangesGuard and three proof pages | End-to-end interaction and preservation of work |
| M19&#8211;M31 | Domain collections and workflows | Consistent adoption without duplicate controls |
| M32 | Integrated application and documentation | Complete requirements coverage and independent acceptance |

### File scope

| File or directory | Intended change | Canonical source, for generated output |
|---|---|---|
| `src/components/page_frame.*`, ApplicationShell and Ribbon modules | Remove heading features; preserve identity, focus, and navigation | Source-owned |
| `src/components/record_list/`, `src/features/search/` | Complete shared collection behavior and SearchBlock | Source-owned |
| `src/pages/`, domain features and API adapters | Supply definitions, connect workflows, remove duplicated presentation | Source-owned |
| Rust query/domain/store modules and `schemas/base_schema/` | Required sorting, filtering, facets, and cursor repairs | Rust contracts and SQL |
| `generated/api/` | Updated transport types | Regenerate from Rust; do not hand-edit |
| Existing tests, temporary proof, and screenshot scenarios | Behavior coverage and staged visual evidence | Canonical test/capture tooling; `tests/_temp/` for one-time proof |
| Active plan, audit ledger, UI/search documentation, CHANGELOG | Track requirements, evidence, and final architecture | Current HG remains authoritative |

## Milestone plan

Every milestone below is required. Each summary identifies its owner and dependencies; its goal is the exit check. Entry requires the named dependencies to pass. A failed check returns to the owning component for repair and focused re-verification.

| M | Title | Summary | Goal |
|---|---|---|---|
| M1 | Requirements and baseline | **Manager; no dependency.** Record every requirement and audited consumer, current status, owning milestone, and validation evidence. Establish local seeded runtime and baseline checks. | Every requirement has a completion owner; no hidden human or production dependency. |
| M2 | PageFrame contract | **Shell owner; M1.** Remove heading features and update callers atomically. Name main content from breadcrumbs and preserve focus. | Type checks and route/keyboard tests pass without PageFrame heading markup or props. |
| M3 | Page-content cleanup | **Page consumers; M2.** Remove repeated introductions, labels, section titles, instructions, duplicate names, and empty metadata. Preserve real status/task context. | Independent capture review confirms useful content follows breadcrumbs across roles. |
| M4 | Shared field model | **RecordList owner; M1.** Add stable field definitions and typed sort/filter bindings, preserving keyed record and editor state. | One definition supplies fields/actions across all presentations without label parsing. |
| M5 | SearchBlock composition | **Search owner; M2, M4.** Replace SearchPage with explicit above/beside layout and required RecordList; move editors outside the block. | Shared composition works in a page and an embedded picker; no replacement-results escape hatch. |
| M6 | Search activation and initial states | **Search owner; M5.** Add definition-owned eligibility and initial-state handling; cancel obsolete requests when criteria disappear. | Empty Search retains headings/filters; sort alone makes no result request; Clear restores the initial state. |
| M7 | Dense Compact rendering | **RecordList owner; M4.** Use semantic aligned columns, concise actions, alternating backgrounds, and essential fields. | Library rows approach the approximately 20-pixel desktop target without clipping or inaccessible controls. |
| M8 | Column resizing | **RecordList owner; M7.** Add edge dragging and keyboard-accessible width adjustment with sensible minimum widths. | Resizing changes the intended column, preserves other state, and does not trigger sorting. |
| M9 | Visibility and responsive columns | **RecordList owner; M7.** Add shared column visibility controls and priority-based responsive hiding. | Narrow layouts retain identity/actions; hidden-column filters remain usable; explicit show choices remain reachable through overflow. |
| M10 | Column rearrangement | **RecordList owner; M7.** Add heading dragging and keyboard move controls using stable field IDs. | Moving columns preserves values, filters, widths, sorting, and record order. |
| M11 | Shared display controls | **RecordList owner; M4, M7.** Consolidate existing mode controls/state, including image-browser duplication. | Compact/List/Visual boxes preserve query, sort, page, selection, actions, and editor drafts without refetching. |
| M12 | Multi-value query contracts | **Library query owner; M4.** Connect existing array filters; extend scalar classification/Bloom selections through Rust, generated types, URL decoding, normalization, SQL, and cursor binding. | Multiple values and combined fields filter the complete collection with valid hierarchy and unchanged authorization. |
| M13 | Filter-choice access | **Library query owner; M12.** Add a read-only facet-choice path within the existing search domain; reuse predicates and vocabulary services. Support bounded searchable choices and continuation. | Filters work before the first result query; choices/counts come from the relevant whole result set, never the loaded page. |
| M14 | Shared column-filter interface | **RecordList owner; M9, M12, M13.** Add heading value pickers, combined active filters, Clear, and access through the shared controls when a column is hidden. | The same filter works in all modes and layouts, including an initially empty spreadsheet. |
| M15 | Library sorting | **Library query and RecordList owners; M4, M12.** Connect header sort state and implement missing server directions/orders with matching keyset cursors. | Both directions remain correct across multiple pages and tied values; stale-query cursors are rejected. |
| M16 | Guard completion | **UnsavedChangesGuard owner; M1.** Support optional Save, actual in-flight save completion, dirty-checked manual leave, and coordination of multiple dirty editors. | Untouched/reverted editors leave directly; failed or pending saves cannot silently lose work or duplicate writes. |
| M17 | Library editor protection | **Library/Pool editor owners; M16.** Protect metadata changes and Pool creation on navigation, close, query changes, and selection changes; then remove SearchLeaveGuard and its bookkeeping. | Search alone never warns; meaningful edits remain protected with and without prior search activity. |
| M18 | Three-page integration proof | **Search/Library owners; M3, M5&#8211;M15, M17.** Complete Search Question Library, Browse Question Library, and My Questions using the same structure and field model. Repair native publication identity and Review guard behavior needed for an authored, nonowned Question to appear safely in My Questions. | Automated journeys and independent architecture/visual review pass every shared capability on all three pages; each page inherits the same implementations, with no page-specific workarounds. Include native publication with no self author and an explicit self author, an authored nonowned Question, feedback loading, and guarded source edits. |
| M19 | My Pools | **Question collection/Ribbon owners; M18.** Add route, catalog, capability, and breadcrumb entries; use Pools plus current-account ownership criteria. | My Pools loads automatically, clears only user refinements, and exposes all shared collection features. |
| M20 | My Drafts | **Draft collection owner; M18.** Rename consistently and adapt the complete private Draft list to shared search, filtering, and sorting. | Search and mode changes preserve private access, creation/import/delete actions, and Draft lifecycle. |
| M21 | Starred | **Stewardship query/UI owners; M18.** Feed the existing Instructor-only `GET /api/questions/stewardship/stars` through the common Library matching, results, facets, and keyset owner using a fixed internal current-Starred scope. | Private whole-result discovery, stable Starred-date keysets, archived/retired inclusion, and Instructor-only star actions work through shared components. |
| M22 | Watched | **Watch query/UI owners; M18.** Make the existing private activity collection searchable and cursor-paged; remove the capped-first-100 limitation from collection operations. | Search/filter/sort reach the complete authorized collection and preserve existing activity links. |
| M23 | Multiple starting Questions | **Pool creation owner; M17, M18.** Accept selected starting revision tuples, show their shared classification, and reuse the existing creation/picker workflow. | Compatible selections create a Pool containing every starting Question; incompatible selections fail clearly without losing entered work. |
| M24 | Blueprint collections | **Blueprint query/UI owners; M18.** Integrate My Blueprint Courses and Search Blueprint Courses; complete required sorting/filtering and update terminology. | Both inherit shared behavior; archive lifecycle, inspection, adoption, and creation remain functional. |
| M25 | Course collections | **Course UI owner; M18.** Integrate active/inactive Courses and Course assessment lists, retaining inline row models. | Searching, column changes, sorting, and mode changes preserve inline drafts, save/conflict behavior, and Course identity. |
| M26 | Proposals and Blueprint history | **Blueprint workflow/query owners; M24.** Add My Change Proposals to the Ribbon and integrate proposal, history, fork, and stewardship collections, extending paginated queries where needed. | Empty collections remain reachable; complete-result operations work without altering review or authored structure. |
| M27 | Assessment collections | **Assessment UI owner; M18.** Integrate templates, due-soon lists, and applicable assessment/attempt collections. | Shared controls coexist with creation, editing, release state, due dates, and existing actions. |
| M28 | Pickers and Pool members | **Picker/Pool owners; M18, M23.** Use SearchBlock for candidate searches, supply the missing Question-picker sort binding, and connect existing Pool comparators to shared headers. | Selection survives presentation changes; inspection works; sorting Pool members changes neither membership nor authored Assessment order. |
| M29 | Student collections | **Student UI/query owners; M18.** Integrate Courses, global Coursework and Grades collections, progress, Student Course invitations, and global Attempt History with appropriate query adapters. Remove Account Pending Invitations source that has no server route; retain working Student Course invitation and Course workflows. | Shared behavior works across enrolled Courses while retaining Course identity, permissions, and current Student workflows. |
| M30 | Administration collections | **Administration query/UI owners; M18.** Integrate Instructor accounts, Course inspection, roster, gradebook, and recovery selections. Human Guidance 282-284 confirmation/audit applies only to Sysadmin FERPA lookup; Instructor recovery has no extra confirmation/audit. | Full-result operations, selection, privacy constraints, and table relationships remain correct. |
| M31 | Remaining consumers and obsolete code | **RecordList/integration owners; M19&#8211;M30.** Reconcile avatars, statistics, vocabulary records, and every remaining audited consumer; remove abandoned helpers and duplicate composition. QuestionUse uses SD data-owner global exact fixed revision counts, without a Course-RLS-filtered join; API owner rows and the complete authorized ownCourses array are returned, removing the 100/truncated stub contract. No StudentWork joins are added. This owner repair starts only after the fifth database snapshot completes. | Every ledger entry has passing integration evidence or a specific semantic justification; no unexplained bypass remains. |
| M32 | Independent acceptance and closeout | **Manager and fresh review/test agents; M1&#8211;M31.** Run integrated checks, staged captures, architecture review, documentation updates, and temporary-test cleanup. | Every required requirement is verified; no unfinished implementation is represented as complete. |

**Parallel-plan ready: yes.** Shell, record-model, query-contract, and guard work have separate initial boundaries. Shared-component edits proceed in dependency order. After M18, independent domain integrations can run concurrently where file ownership does not overlap. The manager assigns implementation and fresh review agents during execution.

For M18, the publication repair accepts an explicit 0-based `selfAuthorPosition` (wire field; Rust `self_author_position`) or None from Review. The server links only that chosen name from validated native source to the authenticated AccountId using the canonical U-prefixed `ple_data.account_id` domain; client text never asserts identity. Forks and revisions preserve existing author links. This completes the existing authorship search scope without changing ownership or authorization. Existing general-feedback GET supplies the parent tuple needed by Review; it does not load the full Draft list. The shared Review guard retains the baseline and dirty values across source edits and autosave. Shared filter updates preserve parent and child intent after the guard resolves. Record acceptance evidence in [modular_ui_completion_ledger.md](../active_plans/audits/modular_ui_completion_ledger.md).

For M21, keep the existing Instructor-only `GET /api/questions/stewardship/stars` transport. The server fixes the internal current-Starred scope and derives the actor from the session; the browser sends neither actor ID nor scope. Reuse common Library matching, results, facets, and keyset ownership, deriving private matches from the authenticated actor while retaining archived and unavailable Published Questions (including retired classification) and excluding Pools. Preserve default `starred_at DESC` with `question_id ASC` ties; Starred-date sorting also supports ASC through the same keyset owner. Keep public Library sort vocabulary, availability, and admin reading unchanged, with no private Starred parameter on its endpoint. Reuse SearchBlock, RecordList, shared typed Question fields, and private Open/Unstar actions; do not add Library bulk editing. Remove the old title-only list, paging, and model only after replacement. Query owns SQL, sorting, facets, and cursors; server owns fixed routing, authorization, and transport; UI owns repository wiring, fields, actions, and composition. Validate multiple pages and ties, both Starred-date directions, archived and retired stars, two-Instructor isolation, Student/admin denial, full facets, and star/unstar. This clarifies existing M21 scope and adds no gate.

For M20, put whole-authorized-array matching in one pure `src/features/search/local_complete_search.ts`: keyword matching, OR within each filter and AND across filters, facet choices computed with their own field excluded while preserving selected values, stable ID tie-breaks, then paging. A thin shared Solid adapter uses the page's existing `createResource` cache and builds standard `SearchDefinition`, `SearchStateController`, and `RecordListQuery` bindings; pages provide fields, ID, keyword and domain scope, default order, and actions. The adapter composes the existing controller state accessor to project source loading/error over retained session rows, counts, and cursor, and retries through `refetchSource` on source error. First source success opens the query once; later successful reloads refresh the cached-array query, prune selections absent from the entire authorized source, and retreat only when deletion empties the current page. Query changes and Clear retain existing selection clearing; display/source refresh preserves surviving selection and editors. The effect tracks source transitions rather than query changes. Query, mode, filter, sort, and page changes use the cache without HTTP; mutations and explicit Refresh refetch the existing resource. Course `assessmentRows` memo/keyed inline models remain source-driven. Keep the central `RecordListQuery.kind` rename from `server-paged` to `paged` in its four existing bindings and retain `local-complete` for whole displayed arrays. All implementation remains gated by M18.

### Resolved implementation decisions

- **Search:** Keep the spreadsheet and column controls mounted initially. A normalized keyword or explicit filter activates results. Default object-kind settings, sorting, resizing, and display changes do not. Clearing the last criterion cancels pending results and returns to the initial prompt.
- **Browse:** Initially load navigation choices/counts. A classification selection or explicit column filter loads records. Keep the sidebar beside results on wide screens and above them on narrow screens.
- **Personal collections:** Load automatically using their predefined scope. Clear removes user refinements while retaining that scope. My Questions uses authorship; My Pools uses ownership.
- **Column state:** Keep widths, order, and visibility in the current RecordList instance. Preserve them through paging and mode changes; reopening restores defaults. Responsive hiding uses field priorities. Explicitly shown columns can overflow rather than disappearing.
- **Compact:** Keep descriptions and secondary metadata in List/details. Preserve full accessible titles when visually truncated. Use native buttons and semantic table relationships rather than introducing an ARIA grid keyboard model.
- **Sorting:** First activation sorts ascending; repeated activation reverses direction. Preserve established initial domain ordering. Compact uses headings; other modes expose the same sort through shared controls.
- **Filtering:** Use OR within a selected-value field and AND across fields, retaining existing domain-specific semantics such as capabilities requiring all selected values. Validate classification relationships, prune incompatible descendants when parents change, and preserve explicit cross-Discipline Subject behavior.
- **Facet choices:** Use the same normalized effective query and authorization as results, excluding only the field being enumerated when offering additional values. Keep selected values accessible and distinguish facet counts from displayed-row counts.
- **State preservation:** Presentation and column changes never alter query, selection, pagination, or editor drafts. Query changes retain existing selection policy, with actual pending editor changes protected before destructive transitions.
- **Pool creation:** Preserve exact starting revisions. Recheck shared Discipline, Subject, Type, and Backend and use the canonical license-compatibility rules. Do not require identical licenses or automatically create a Pool during navigation.
- **Exceptions:** Preserve genuine ordered editors, hierarchies, response controls, comparisons, and single-record details. A historical layout difference or missing integration is not an exception.
- **M28 selection and inspection:** Selection persists through presentation changes. Idempotent actions are keyed to exact revision tuples and preserve order; bulk add includes only newly selected members. Shared inspection services are wired by each of the three consumers.
- **M31 QuestionUse owner contract:** The implemented repair returns global exact fixed revision counts from the SD data owner, API-owner rows, and the full authorized ownCourses array, with no Course-RLS-filtered join, 100-row truncation, or StudentWork joins. The sixth owned-database oracle passed; HTTP and browser behavior remain M31 gates.

## Test and verification strategy

Use existing automated checks, temporary behavior proof, and independent agent walkthroughs of realistic tasks: find a Question, refine classification, compare records, manage a personal collection, create a Pool, and leave an edited record safely. The following are behaviors to verify, not a list of permanent tests to create or retain:

- Initial search eligibility and cancellation of late results.
- Whole-result filtering/sorting, deterministic pagination, and authorization.
- Mode/column changes preserving state.
- Keyboard sorting, filtering, resizing, and rearrangement.
- Genuine dirty-state detection, save failures, pending saves, and manual leave.
- Multi-Question Pool creation and compatibility validation.

For M1-M32, reuse the canonical owner suite or an existing assertion when it already proves the behavior. Put new one-time tests, reproductions, measurements, and browser journeys in ignored `tests/_temp/` while implementation is active. Add or retain a permanent case only when its *actual assertion* protects important, intentionally stable behavior with a plausible regression, existing coverage does not suffice, and its value earns the entire fixture and harness cost. When in doubt, delete the permanent case. There is no test quota per milestone, feature, bug, or state. At closeout, retain ignored reports, logs, receipts, and captures as evidence; delete one-time executable probes unless independently justified for promotion.

M13 and M14 use temporary new proof only. M18 must run automated real-application journeys for Search, Browse, and My Questions through the canonical existing owner harness, with independent architecture and visual review; those journeys may be temporary. M32 reruns the existing fast and full gates and verifies the integrated application without building a new mass component-test registry. Passing earlier component receipts remain historical evidence even where their temporary or pruned harness code no longer exists.

Use temporary measurements and staged captures for density, default column choices, visual hierarchy, and responsive composition. Validate at 1280x800 and the repository&#8217;s narrower Student viewports, with enlarged text and both appearance modes. The approximately 20-pixel row target is an inspection target, not a brittle universal pixel assertion.

Run the repository fast checks after bounded changes and the full suite at integration:

- `source ./source_me.sh && ./launchers/run_fast_checks.sh`
- `source ./source_me.sh && ./launchers/all_test.sh`
- Use `devel/capture_screenshots.sh --only &#8230;` for staged scenario evidence and the applicable screenshot verification path.
- Check changed documentation for whitespace, encoding, and Markdown-link integrity with the repository's existing non-Git checks.

Update or remove obsolete tests that demand visible descriptions in every mode or search-discard dialogs. Preserve any important, stable behavior through existing coverage or temporary proof before considering a new permanent assertion.

A failed gate produces an owned repair task and repeat of the affected checks. Independent agents assess captures and architecture; no milestone waits for human visual acceptance.

## Migration and compatibility policy

Update shared interfaces and their consumers coherently. Regenerate API types from Rust, update query/URL decoders and cursor bindings together, and use the pre-production database workflow for SQL changes.

Preserve working routes where renaming visible labels is sufficient. Add the missing My Pools route. Preserve archive behavior and all access controls.

Retain existing record IDs, selected revision identities, retries, request ordering, and editor models. Remove superseded components after their consumers migrate; do not leave parallel old and new presentation systems.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
|---|---|---|---|---|
| Sorting/filtering only loaded rows | Incorrect results | A bounded endpoint is treated as complete data | Domain query owner | Implement server predicates/order and multi-page tests before integration acceptance |
| Column operations reset work | Lost selection or drafts | Rendering branches recreate state | RecordList owner | Keep stable models outside presentation branches and test transitions |
| Guard bypass loses another editor | Lost edits | Multiple dirty editors or an in-flight save | Guard owner | Coordinate the leave decision and wait for confirmed outcomes |
| Responsive hiding removes functionality | Unreachable actions or filters | Narrow width or forced column visibility | RecordList owner | Shared hidden-column controls, reachable actions, keyboard checks, and overflow |
| Another partial consolidation | Persistent custom implementations | Proof pages pass but remaining consumers are deferred | Manager | Keep every consumer assigned to a required milestone and block closure on unfinished entries |

## Rollout and release checklist

- [x] Complete the three-page proof before expanding consumer integrations. (M18 attempt 12; doc checkpoint and ledger 57/87-91.)
- [x] Complete every subsequent required collection milestone. (M19-M31 verified.)
- [x] Pass independent architecture, behavior, accessibility, and capture reviews at the owner gates.
- [x] Pass applicable aggregate checks and record any baseline failures with their resolution. (Current canonical fast and full receipts; prior failures and repairs preserved.)
- [x] Verify local seeded workflows without production credentials.
- [x] Leave production deployment outside this plan&#8217;s completion criteria.
- [x] Pass final integrated specification and distinct quality reviews, then record manager acceptance of M32. (`/root/final_plan_integration_spec` and `/root/final_plan_integration_quality` PASS; manager accepted M1-M32 on 2026-10-16.)

## Documentation close-out requirements

- Maintain `docs/active_plans/active/modular_ui_completion_plan.md` and a requirement/consumer ledger under `docs/active_plans/audits/`.
- Record each milestone as pending, active, failed, or verified, with evidence and remaining defects.
- Update SEARCH_PAGE_ARCHITECTURE and UI_DESIGN_GUIDE to describe the implemented responsibilities.
- Reconcile overlapping Ribbon and HG plans so superseded requirements cannot reintroduce old names or heading behavior.
- Correct remaining singular Pool-creation wording in supporting specifications.
- Update CHANGELOG after each completed bounded task.
- Retain justified behavior tests; remove temporary probes after recording evidence.
- Archive the completed plan only after all required milestones pass.

## Open questions and decisions needed

No human decision is required to begin or finish this plan.

The manager and an independent architecture or UI reviewer resolve implementation details using current HG, source evidence, behavior tests, and staged captures. Default column widths and visibility are selected through those checks and remain tunable within the shared definitions.

Implementation, technical verification, fresh final integrated reviews, and manager acceptance are complete. All approved M1-M32 requirements and gates are verified.
