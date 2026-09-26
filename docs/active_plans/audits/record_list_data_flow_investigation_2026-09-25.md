# RecordList data flow and performance investigation

Date: 2026-09-25. Source investigation complete; implementation pending.

This supplements the [complete caller inventory](record_list_caller_investigation_2026-09-25.md)
and informs the [shared framework plan](../active/record_list_page_frame_standardization_plan.md).
Graphify located the Library, picker, Pool and Blueprint boundaries; current source and existing
test cases supplied the findings below. No product tests, database benchmark or browser performance
trace ran during this investigation. These are demonstrated work/ownership problems, not measured
latency claims.

## Architectural conclusion

Keep RecordList's DOM, reactivity, responsive layout, accessibility, focus, selection and interaction
in Solid/TypeScript. Make it fast primarily by giving it useful, bounded records and stable owners.
PostgreSQL/API own broad collection predicates, ordering, facets and result pages. The page/data layer
owns query state, continuation, selection and return state. Shared controls present sorting and paging.
The row component receives content and callbacks, without fetching or processing a catalog.

The existing Question Library browser requests **50 records** per page; its API default is also 50
and its ceiling is **100**. Pool and Blueprint discovery use the same default/ceiling through their
existing paging contracts. The user's subsequent page-size request supports a small shared choice:
**50 (default), 100 or 250 per page** on broad discovery surfaces. The human specifically identified
rapid Question scanning as the reason for 250; WP-P0 raises the discovery ceiling accordingly. The original
100-200 example is not a new bound. Size changes restart at page one, retain task selection, and travel
with current query/return state; no account preference, arbitrary numeric input or All option is needed.

## Investigated collection paths

| Surface | Current transfer and query ownership | Browser behavior | Plan disposition |
| --- | --- | --- | --- |
| Question Library Search/Browse | 50 requested; API permits 1-100. Store fetches every available entry; Rust resolves sources, filters, builds facets and sorts before taking the page. | Appends on continuation; scrolling near the end requests more. DOM windowing does not bound retained rows. | SQL predicate/order/page and full-query facet aggregates; hydrate only page summaries. Explicit page replacement and shared Previous/Next plus 50/100/250 choice. |
| Question picker: Library / Shared Library / My Questions | Same Question Library repository and server path, including authorship filter for My Questions. | Appends result pages; selected Questions already have separate task state. | Same bounded query path; replace discovery page while retaining the controlled selected tray and exact revision order. |
| Question picker: one Blueprint Assessment | Loads the selected Blueprint revision, extracts its Assessment content, filters locally, slices 100 records with an offset token. | Same picker session currently appends. | Task-local content remains local. Use the picker page controls; keep its existing source-specific bound and authored order. No database search over this saved document. |
| Question Pool Library | API default 50/max 100; SQL applies filters, aggregates and ID-ordered continuation before returning at most page size plus one. | Explicit Load more appends every result; DOM is not windowed. | Retain working SQL/API. Replace displayed page using the existing cursor; keep inspection and selected detail separate. |
| Blueprint Pool picker | Same bounded Pool endpoint; default 50. | Load more appends; selected summary/detail are separate signals. | Same page replacement and shared controls; selected Pool remains explicit and usable when its result page is absent. |
| Public Blueprint Search | SQL filters by authorization, literal name/classification and promotion, orders name/ID, and limits the page. 50 requested, max 100. | Appends; returning from detail replays and accumulates all previously visited pages. | Keep SQL ordering. Save current page cursor and visited cursor history; refetch the current page on return without replaying earlier result pages. |
| My Blueprint Courses | Same bounded endpoint, ordered by name/ID; request has no sort field. | Name/adoptions/students comparator sorts only accumulated results. A high-count Course on a later page cannot rank first. | Add the existing choices to the API/SQL contract; full-query order before page selection. Remove the local result comparator and replace displayed pages. |
| Live Assessment editor: available Questions | Course-authorized SQL joins the entire available Published Question catalog; API returns an unpaged array. | All candidates render in the available scan. This is catalog discovery despite appearing inside an Assessment task. | Reuse the existing QuestionPicker and bounded Library repository; retire the separate catalog-dump endpoint after its caller moves. |
| Live Assessment editor: available Pools | Calls the paged Pool API once and takes `items`; ignores continuation. | Only the first default page is reachable. | Reuse the existing Pool picker with page controls, preserving capacity, attestation and deferred Save. |
| Selected Assessment entries, selected Question trays, Student/Course task lists | Task-local projections and existing domain limits/paging; an Assessment can still have many entries under its current domain cap. | Native scan/Sequence and domain editing. | Retain task semantics and existing limits. Local ordering of a complete draft is valid. Profile a real rendered task before adding windowing. |
| Private Drafts and Sysadmin Instructor Accounts | Current endpoints return all records within the private workspace/admin scope; no list limit was found. | Ordinary local lists/forms. | Record as unbounded, not proven small. This investigation establishes no large-workload or latency requirement for them; keep them outside the broad discovery changes and revisit on actual volume evidence. |

### Source anchors

- [question_library_repository.ts](../../../src/api/question_library_repository.ts):32,208-260
  requests 50 and forwards sort/cursor; [query.rs](../../../crates/server/src/question_library/query.rs)
  and [question_library.rs](../../../crates/server/src/question_library.rs):40,134-214 validate limits
  and execute the current full-catalog pipeline.
- [PostgreSQL Question Library store](../../../crates/learning-data-access/src/postgres/question_library.rs):64-92
  uses an unrestricted `fetch_all`; [summaries.rs](../../../crates/server/src/question_library/summaries.rs):32-40,114-180
  resolves, parses and compiles each native source sequentially before paging.
  [search_query.rs](../../../crates/server/src/question_library/search_query.rs) searches metadata,
  authors, tags, classification and type labels, not rendered prompts. Those predicates do not require
  compiling the whole native catalog.
- [paging.rs](../../../crates/server/src/question_library/paging.rs):41-111 already orders the full
  match with an ID tie-breaker; its cursor binds the normalized query/sort. This is globally correct
  service sorting with excessive upstream work, unlike My Blueprint's partial-result sort.
  [facets.rs](../../../crates/server/src/question_library/facets.rs) counts the full intersection
  and caps free-text facet lists. Preserve that meaning when moving aggregation into SQL.
- [Library session](../../../src/pages/library_page_model.ts):622-759 appends pages and retains a
  return snapshot; [Library rows](../../../src/pages/library_browse_rows.tsx):292-367 measures windows
  and requests another page on scroll. the former client window helper (removed after bounded server pages)
  builds layout arrays across all retained records. It bounds ordinary mounted DOM, not transfer,
  retained data or all layout calculation; a distant focused row can also expand the mounted slice.
- [Question picker model](../../../src/features/question_picker/question_picker_model.ts):202-218,233-366,423-453
  distinguishes Library from one-Assessment sources and appends their results.
- [Pool SQL](../../../schemas/base_schema/50_functions/question_pools.sql):499-636 and
  [Pool store](../../../crates/learning-data-access/src/postgres/question_pool_library.rs):62-128
  already implement bounded discovery and full-query Bloom facets. The store removes the lookahead
  row. Pool Library and picker append in
  [library_pool_discovery.tsx](../../../src/pages/library_pool_discovery.tsx):101-124 and
  [question_pool_picker.tsx](../../../src/features/question_pool_picker/question_pool_picker.tsx):54-78.
- [Blueprint list route](../../../crates/server/src/blueprint_course/list.rs):23-191,
  [store](../../../crates/learning-data-access/src/postgres/blueprint_course/search.rs), and
  [SQL](../../../schemas/base_schema/50_functions/blueprint_operations.sql):467-569 own filtered
  name/ID pages. [My Blueprint workspace](../../../src/features/blueprint_course/blueprint_courses_workspace.tsx):94-165
  applies the partial local comparator. [Public Search](../../../src/pages/blueprint_course_search_page.tsx):135-205
  replays earlier pages during restoration.
- [Assessment picker SQL](../../../schemas/base_schema/50_functions/assessment_operations.sql):72-113,
  [store](../../../crates/learning-data-access/src/postgres/assessment_release.rs):187-216 and
  [route](../../../crates/server/src/assessment_release.rs):220-243 return the full catalog.
  [Questions page](../../../src/pages/assessment_workspace/assessment_workspace_questions_page.tsx):138-146
  loads that array alongside only the first Pool page.
- [Private Draft SQL](../../../schemas/base_schema/50_functions/question_authoring_operations.sql):348-362
  and [Instructor Account SQL](../../../schemas/base_schema/50_functions/accounts.sql):425-438
  establish the two unbounded scoped cases above; no performance measurement is implied.

## Additional boundary findings for the 250 choice

250 is new product guidance, not a current supported API size. The ordinary
[PageSize](../../../crates/learning-data-access/src/pagination.rs) maximum is 100; the Question route
also has its own 100 ceiling. Pool SQL validates 1-100, and Blueprint SQL permits 101 only for the
internal lookahead. Result decoders cap Question, Pool and Blueprint arrays at 100, including a second
Library browse decoder. Changing the select alone would therefore fail.

The [shared decoder constants](../../../src/api/decoders/shared.ts):46,68 alias Question page size to
ordinary cursor size, and [question_library.ts](../../../src/api/decoders/question_library.ts):307,320
uses the Question limit for both prompt blocks and search results. WP-P0 separates those meanings and
raises only discovery. Page-only usage-statistics enrichment has no additional 100-ID limit in the
inspected store/SQL. The [HTTP response reader](../../../src/api/http_client/response.ts):63 retains
its existing bounded-response policy; realistic 250-row payloads belong in the contract proof.

## Implementation consequences

1. **Bound at the database boundary.** Push Question predicates and keyset page selection into the
   authenticated store. Reuse the current search grammar and exact revision identity. Aggregate facets
   over the complete authorized match independently of page position. SQL may still scan many rows to
   answer a filter/count; bounded output is not a claim of constant database work.
2. **Resolve only useful content.** Native source validation/compilation may still supply metadata
   fields of a returned summary that are absent from the database projection. Do that only for page
   items, retaining exact-source checks. Detail and preview remain separate on-demand routes. A new
   summary cache, snapshot service or persisted search index is unnecessary for this boundary repair.
3. **Order the full query.** Library's existing title/newest order moves to SQL. Blueprint's existing
   name/adoptions/students choices become trusted API choices with deterministic name/ID ties. A sort
   change resets continuation; the shared select only emits the requested choice. Complete task-local
   arrays, such as unsaved Assessment order, may still use local sorting.
4. **Bound browsing state.** Keep one result page plus independent task selection. Previous/Next
   refetch through saved query-bound cursor positions; cursor history does not retain prior row sets.
   Preserve current page, query and focus for detail return. Retain current safe rows during a failed
   same-query transition; access denial clears them. Query or page-size changes reset page history and use existing
   stale-response guards. Bulk selection spans pages through IDs under the existing bulk limit;
   "Select this page" adds the current results without claiming to select all query matches.
5. **Keep rendering simple.** Compare plain rendering of the default and requested 250-row Library page with its existing
   window helper during implementation. Prefer plain rendering when the populated task is responsive;
   retain the helper only for demonstrated rendering cost. Either result uses the shared row renderer.
   Current windowing is existing code, not proof that every RecordList needs virtualization.
6. **Keep WASM evidence driven.** There is no measured heavy browser computation here. A future
   demonstrated computation over data that genuinely must already be local could justify a worker or
   Rust/WASM data-processing boundary. The current redesign introduces neither, and keeps DOM and
   interaction ownership in Solid.

The [bounded data workstream](../workstreams/record_list_bounded_data_workstream.md) assigns these
required corrections to concrete packages and the existing caller owners. It supersedes the earlier
recommendation to preserve My Blueprint's local comparator and Library's accumulating page history.

## Evidence for execution

Use existing Library cursor tests for query binding, title/ID ties, newest ordering and full-query
Bloom facets; move relevant behavioral assertions to the new store boundary. Existing picker,
Library browser and Blueprint return-state cases are the first frontend checks. One small multi-page
fixture should put an important match or popular Course beyond the former first page and prove it
appears in the correct query-wide position. Exercise page replacement, selection, return and failure.

For a temporary representative catalog, record returned rows, native source resolutions, retained
browser rows and observed request/render work before and after. Capture PostgreSQL 17
`EXPLAIN (ANALYZE, BUFFERS)` for the actual inner page/facet queries. Treat timing as diagnostic
evidence, not a new arbitrary pass threshold; justify an index only from the measured plan.
PostgreSQL documents the need for deterministic page ordering in
[LIMIT/OFFSET](https://www.postgresql.org/docs/17/queries-limit.html) and explains how to inspect
actual query work in [Using EXPLAIN](https://www.postgresql.org/docs/17/using-explain.html).
Keep this evidence temporary after recording its conclusion; reuse durable behavior checks for the
small set of changed contracts.
