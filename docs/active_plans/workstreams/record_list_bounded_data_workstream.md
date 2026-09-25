# RecordList bounded data workstream

This is a required workstream of the
[shared framework plan](../active/record_list_page_frame_standardization_plan.md), grounded in the
[data-flow investigation](../audits/record_list_data_flow_investigation_2026-09-25.md).
All packages are pending. MANAGER maintains status in the
[execution ledger](record_list_page_frame_standardization_ledger.md).
This adds M25-M32 without renumbering existing milestones; dependencies, not numeric order, control
dispatch. Every package below is part of the current plan's completion path.

## Fixed ownership and behavior

- RecordList remains Solid/TypeScript presentation. Query choices belong to pages; predicates,
  full-query ordering and broad collection paging belong to PostgreSQL/API. The shared family owns
  the sort select and Previous/Next presentation, with controlled callbacks and loading/disabled state.
- Keep the discovery default of 50. Broad discovery presents the shared
  choice of 50, 100 or 250, including the human's rapid-scanning use case. WP-P0 raises only the
  discovery API/store/SQL/decoder ceiling to 250, retaining the ordinary list limit of 100. Page size belongs to the query/return state and API
  request, with a first-page reset on change and retained selection. Keep it out of row props and
  account settings. The local one-Assessment source keeps its current authored-order slice behavior;
  it needs no page-size chooser. Shared choices are fixed; callers do not supply size menus.
- Retain one displayed result page. The owner keeps the input cursor for that page, its next cursor
  and the visited input-cursor stack for Previous. Fetch through the existing server contract when
  moving either way; replace rows on success. Existing loading/generation/error state handles pending
  transitions. Share a small cursor helper only where the repeated transition is actually identical.
- Keep selection as separate task state: Library stores selected IDs under its existing bulk limit;
  QuestionPicker retains selected exact revisions in its bounded tray; PoolPicker retains one selected
  summary/detail. Page changes preserve selection. Query/source changes follow the existing selection
  policy. Library's "Select this page" adds its page IDs up to remaining capacity, with a truthful
  count. It never needs other result pages to perform the bulk command.
- A transient page failure retains the previous authorized page and its current cursor; Retry requests
  the failed target. A successful transition commits the cursor/history and moves focus to the results
  heading or status. Query/sort/page-size changes reset history; stale responses cannot install rows. Session or
  authorization loss clears affected rows, selection and return data through the existing scope owner.
- Return state contains the current page/cursor/history and focused record, not accumulated results.
  Library may retain its existing one authorized in-memory page; mutations trigger its existing
  refresh behavior. Public Blueprint Search refetches only the saved current page to recheck access.
  Missing records or invalid continuation reset to the first current page with a useful status.
- ASVS 1.2.4, 2.2.1-2.2.3 and 8.3.1 apply to the changed query boundary: bind values, validate closed
  sort/filter/cursor input, and reapply authorization to both items and aggregates on every request.
  Cursor data describes position and query; it grants no access. Retain answer-free browser projections.

## Packages

### M25 / WP-P0: support the demonstrated discovery page sizes

- Owner: DISCOVERY-CONTRACTS. Depends on: WP-B1.
- Files: the owning Rust model/export for a fixed discovery maximum; `learning-data-access` pagination
  types and Question/Pool/Blueprint search signatures; their server validators, SQL limit checks and
  frontend page decoders, plus `library_page_model.ts`'s browse-page bound. Generated artifacts are
  produced by this owner before handing shared files to QUESTION-DATA, BLUEPRINT-DATA and LIBRARY.
- Implementation: add a fixed 250-record discovery bound, retaining ordinary `PageSize`/`PageRequest`
  at 100. Use a narrow validated discovery page-size/request type for Pool/Blueprint discovery and the
  new Question search operation; reuse existing cursor/page payloads. This separates actual task
  limits rather than adding a configurable maximum to every list. Keep default 50, and API validation
  for positive sizes up to 250; the UI exposes only the shared 50/100/250 choices.
- Update Question search's server maximum, Pool SQL's 100 guard and Blueprint SQL's 101 lookahead
  guard to the discovery bound (250 items, at most 251 internal lookahead). Update only the matching
  frontend result-array decoders. `MAX_QUESTION_SEARCH_PAGE_ITEMS` currently also caps static prompt
  blocks: separate that existing 100-block content limit instead of raising it accidentally. Ordinary
  cursor pages, facet caps, bulk-selection limits and task-content limits retain their own meanings.
- Done: 250-item Question/Pool/Blueprint pages survive the API/store/SQL/frontend path, and 251 as a
  requested result size is rejected. Smaller choices still work. Existing page-only statistics accepts
  the selected IDs; discovery hydration remains limited to the page after WP-P3. Preserve the HTTP
  client's existing bounded-response policy and check realistic long descriptions/authors against it.
- Validation: extend the existing page-boundary behavior checks at the changed owners; use one
  populated 250-item response per distinct decoder path and reject over-bound input. Inspect generated
  output and current source consumers; avoid a permanent source-inventory test or blanket limit change.
- Parallel-plan ready: yes with CORE/NAVIGATION. One owner performs this shared contract handoff;
  QUESTION-DATA and BLUEPRINT-DATA then proceed independently. Shared generation runs serially.

### M26 / WP-P1: database-owned Question result pages

- Owner: QUESTION-DATA. Depends on: WP-P0.
- Files: Question Library store trait/records in `crates/learning-data-access/src/question_library.rs`,
  PostgreSQL implementation and a focused search module beside it; Question Library functions/grants
  under `schemas/base_schema/50_functions/` and `70_grants/`; their existing database checks.
- Implementation: add a typed store search operation over the existing accepted metadata projection.
  Carry normalized exact-ID/text terms, classification/Bloom, authorship, Course-use, authors/tags,
  backend/type/license/capability filters and the closed sort/cursor position. Use the established
  field/phrase/exclusion grammar with literal matching and current AND/OR semantics. Page by
  title ascending plus Question ID, or publication descending plus Question ID. Match ordering and
  continuation collation explicitly. Select at most the requested page plus one lookahead row.
- Keep existing adapter capability declarations authoritative: translate requested capabilities to
  the supported backend predicates from those declarations at the service boundary. The database
  filters metadata; it does not execute Question source or learn a second configurable backend model.
- Done: the store returns the correct bounded page across ties and filters; query execution never
  transfers the entire catalog to Rust. Page and cursor keys describe the same order.
- Validation: reuse existing search grammar/cursor cases as behavioral inputs; prove a match beyond
  the old first page and tied title/date continuation in the existing database oracle. Capture the
  inner SQL plan on a representative fixture. Reuse existing indexes first.
- Parallel-plan ready: yes. Independent of CORE, NAVIGATION and WP-P4; one QUESTION-DATA writer.

### M27 / WP-P2: database-owned Question facets

- Owner: QUESTION-DATA. Depends on: WP-P1; continues the same file lease.
- Implementation: aggregate the complete authorized predicate intersection before applying the
  cursor/page limit, reusing the predicate authority from WP-P1. Return bounded facet groups with
  existing normalized labels, per-Question deduplication, truncation flags and zero-count Bloom
  categories. Keep empty-page facets meaningful. Backend counts can be folded into capability counts
  using the same existing declarations in Rust; this folds bounded groups rather than catalog rows.
- Execute page and aggregate reads against one consistent database snapshot, either in one statement
  or a read transaction with the required snapshot semantics. Public facets describe the same query
  as its items; this is not a new persistent snapshot store.
- Done: counts include matching Questions beyond the returned page, exclude unauthorized/nonmatching
  records and preserve current facet limits. Neither aggregation nor cursor position requires native
  Question source reads.
- Validation: move/reuse the existing beyond-first-page Bloom facet case and existing text-facet
  normalization cases at this boundary. Inspect the aggregate SQL plan; add an index only for an
  observed query need. Keep SQL measurements temporary.
- Parallel-plan ready: no within QUESTION-DATA; shares WP-P1's query authority and files.

### M28 / WP-P3: route bounded Question discovery through the store

- Owner: QUESTION-DATA. Depends on: WP-P2.
- Files: `crates/server/src/question_library.rs`, its `query`, `search_query`, `paging`, `facets` and
  `summaries` modules as affected; corresponding store callers/tests. Keep exact detail/preview routes.
- Implementation: normalize/parse the request, decode its query-bound cursor, and call the bounded
  store. Resolve native source only for returned page items when needed to preserve the existing
  summary fields and exact-source validation. Keep page-only usage-statistics enrichment. Use SQL
  aggregates for the current response shape; retain 50/default and the WP-P0 discovery ceiling of 250.
- Validate continuation before expensive source resolution. Remove the search route's full-catalog
  load/compile/filter/sort pipeline and superseded aggregate/matching code once callers are migrated.
  Keep parser and detail helpers that still serve real callers; close obsolete store/functions/grants
  after a caller search establishes their replacement. Regenerate affected bindings with the existing
  generator only if the public contract actually changes.
- Done: each search resolves at most its returned native Questions; filters/order/facets apply to the
  complete query. Existing Library and picker adapters continue to receive the established page shape.
- Validation: existing API/browser behavior plus a temporary source-read counter on a multi-page
  fixture. A non-page native source is not read during discovery. Retain exact detail validation and
  existing authorization/invalid-cursor outcomes. No arbitrary latency target.
- Parallel-plan ready: yes with presentation/navigation; serialize QUESTION-DATA work and shared
  generator/runtime operations through MANAGER.

### M29 / WP-P4: full-query Blueprint sort

- Owner: BLUEPRINT-DATA. Depends on: WP-P0.
- Files: Blueprint list request in `crates/learning-data-access/src/blueprint_course.rs`, PostgreSQL
  `blueprint_course/search.rs`, Blueprint SQL function/grants, `crates/server/src/blueprint_course/list.rs`,
  owning model/API type, `src/api/blueprint_course.ts`, `src/api/http_client/blueprint_course.ts`,
  generated bindings and existing client/database behavior cases. BLUEPRINT owns the later page edit.
- Implementation: add only the existing My Blueprint choices: name, adoptions and students. Default
  to name so Public Search keeps its current task. Compute the current adoption/student expressions
  over matching authorized Courses before page selection. Order counts descending, then name and ID;
  use name/ID for the name choice. Extend typed cursor positions and bind the selected sort along with
  existing visibility/query/classification/page-size fields. Keep literal SQL and bound values.
- Done: a Course outside the former first page appears first when its count earns that rank. Adjacent
  pages share deterministic order; changing sort starts a new cursor sequence. The API requires no
  fetching-all workaround or browser comparator. Preserve existing name-order Public Search.
- Validation: existing Blueprint client/list checks with a multi-page fixture covering the existing
  choices and tied values; reject a cursor reused under a different sort. Inspect actual SQL work
  before adding an index, cached total or other optimization.
- Parallel-plan ready: yes with QUESTION-DATA, CORE and NAVIGATION; distinct source files after the shared WP-P0 contract handoff.
  MANAGER leases the shared generator and database runtime one operation at a time.

### M30 / WP-C9: shared paging controls

- Owner: CORE. Depends on: WP-C1, WP-P0.
- Files: a focused `RecordPageControls` module and family styles/export; existing family harness.
- Implementation: native Previous and Next buttons, with a 50/100/250 records-per-page select for broad discovery and fixed shared placement,
  focus treatment and loading/disabled state. The owner supplies availability and callbacks; the
  control neither fetches nor stores cursors. Its optional controlled page-size value/callback serves
  broad discovery; local lists omit it. The three fixed choices stay in shared code. Existing endpoints provide continuation rather than
  exact totals, so the control presents truthful navigation without total-page or jump-to-last claims.
- Done: the same control supports Library, Question/Pool pickers and Blueprint discovery. Ordinary
  task-local RecordLists need no paging props or extra wrapper.
- Validation: extend the existing harness only for keyboard activation, names and boundary/busy
  behavior and page-size change emission; real page transitions are proven by the caller tasks below.
- Parallel-plan ready: no within CORE's shared file lease; independent of data packages once WP-C1
  is accepted. Release each ready caller without waiting for unrelated data work.

### M31 / WP-P5: Assessment discovery uses the bounded pickers

- Owner: INSTRUCTOR. Depends on: WP-L1.question_picker, WP-BP1.question_pool_picker, WP-C7, WP-P3.
- Files: `assessment_workspace_questions_page.tsx` and `assessment_workspace_questions_view.tsx`,
  their affected model/helpers, and the obsolete Course picker route/store/client/generated contract
  and `list_assessment_question_picker` SQL/grants. MANAGER leases any shared API file after DATA handoff.
  This package completes the ledger's existing view task and its adjacent page task together.
- Implementation: open the existing QuestionPicker with the bounded Library repository, using
  remaining capacity and exact selected revisions. Reuse the Pool picker moved to a common feature
  by its BLUEPRINT owner. Add confirmed choices to the existing unsaved Assessment draft. Keep the
  Sequence, Bloom-sort command, attestation, dirty guard, conflicts and explicit Save behavior.
- Retain Course authorization in workspace reads and mutations; Library discovery uses its existing
  authorized Instructor boundary. Remove the old full-catalog availability scan, first-page-only Pool
  select and their unused transport/SQL path after source inventory confirms no remaining consumers.
- Done: an Instructor can find and add a Question or Pool from a later result page without loading
  the whole catalog; existing entries and unsaved edits survive picker cancellation/failure.
- Validation: reuse existing picker and Assessment edit/save behavior, with a later-page Question/Pool
  and the existing capacity/conflict case. Regenerate the affected API surface through its normal owner.
- Parallel-plan ready: yes after named handoffs; independent of Student and other Instructor files.

### M32 / WP-P6: accept bounded work and remove unnecessary machinery

- Owner: INTEGRATOR. Depends on: WP-X1, WP-P3, WP-P4, WP-P5.
- Implementation: exercise the actual query-to-record path using a temporary representative catalog
  and existing browser harness. Record returned rows, native source resolutions, retained rows and
  observed render work; inspect page/facet query plans. Compare plain rendering of the default page and the user-requested 250-row page with
  existing Library windowing. Choose plain rendering when it serves the populated task responsively;
  retain windowing only when this comparison identifies a real cost it solves.
- Route required source corrections to their named owner. LIBRARY removes unused window observers,
  spacer/measurement/scroll machinery when plain rendering is chosen, while retaining needed return
  focus/position behavior. CORE removes the shared helper only if its caller inventory is empty.
  Both choices preserve the same row renderer and page bound; ordinary lists gain no new window API.
- Done: global sorting and all-page reachability work, discovery retains one current page, independent
  selection still works, and the chosen rendering path has recorded evidence. Earlier pages are not
  replayed just to return to the current page. Every superseded data/presentation path is removed.
- Validation: existing permanent tests first, a few changed-contract cases only where needed. Keep
  profiling, counters, seeded volume and comparison variants temporary; record conclusions and remove
  that machinery. WP-V1 then runs complete repository acceptance, including changed database/API code.
- Parallel-plan ready: yes with FRAME/NAVIGATION after WP-X1; one owner leases capture/database runs.

## Caller handoffs

| Existing owner/task | Additional prerequisites | Concrete completion |
| --- | --- | --- |
| LIBRARY: `library_page`, `library_browse_rows` and `library_page_model` | WP-C9, WP-P3, plus existing presentation prerequisites | Replace auto-append scrolling with current-page navigation and 50/100/250 request choice; preserve query-wide sort/facets, current-page return, scoped selection and shared collection notices. |
| LIBRARY: `question_picker` and its model | WP-C9, WP-P3, plus WP-C3/WP-C7 | Replace discovery rows per page; forward the Library page-size choice and keep selected tray separate. Its local Blueprint Assessment source keeps local authored-order semantics. |
| LIBRARY: `library_pool_discovery` | WP-C9 plus existing base/Sequence proof | Use existing SQL-paged endpoint and shared size choice; preserve selected inspection and focus while replacing results. |
| BLUEPRINT: `question_pool_picker` | WP-C9 plus WP-C3/WP-C6 | Page results with the shared size choice and retain selection; move the reusable picker/CSS into `src/features/question_pool_picker/`, updating its current Blueprint caller. Release to WP-P5. |
| BLUEPRINT: `blueprint_courses_workspace` | WP-C9, WP-P4 plus base/WP-C8 | Send sort/size to API, reset page on sort/filter/size, remove local comparator and accumulated rows. |
| BLUEPRINT: `blueprint_course_search_page` and return-state helper | WP-C9 plus base proof | Use existing server order and selected page size; restore by current input cursor, replacing prior page-replay logic. |
| INSTRUCTOR: Questions workspace page/view | WP-P5 package handoff | Complete existing Sequence migration and bounded picker integration together; retire available catalog row API. |

Other caller and Frame/Nav dependencies stay as recorded. DATA work can start with WP-B1 while CORE
proves rendering. Only the listed callers wait for data/control handoffs. Plan completion requires
WP-P6; the workstream is not deferred optimization. Its scope is removing demonstrated unnecessary
work and incorrect partial sorting, with no WASM port, generic query engine, background cache or
configurable internal performance policy beyond the demonstrated page-size choice.
