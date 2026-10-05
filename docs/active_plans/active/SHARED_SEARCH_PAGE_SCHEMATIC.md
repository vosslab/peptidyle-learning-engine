# Plan: Shared search page schematic

## Context

Status: design proposal, not an implementation or completion claim.

The September 25 work remains in place: shared record displays, page layout, sorting and paging
controls, and database queries that return one page of results. Individual search pages still
assemble their filters and manage requests separately. Neil accepts that arrangement but sees
value in sharing more of the page.

This proposal extends that work. Product behavior follows
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). The earlier implementation is recorded in
[record_list_page_frame_standardization_ledger.md](../workstreams/record_list_page_frame_standardization_ledger.md).

## Objectives

- Build Question, Question Pool, and Blueprint Course searches from one shared search page.
- Let each content type supply its own fields, supported filters, sorting choices, and actions.
- Show the same search through compact spreadsheet rows, expanded lists, or visual boxes.
- Keep database searches bounded and search state temporary.

## Design philosophy

Apply **KISS** and **Design for adaptability**: share repeated interaction and presentation,
while keeping content rules with the code that already understands them. A page describes what
can be searched and displayed; shared code handles how the search works on screen.

Prove this boundary with Questions and Blueprint Courses before generalizing it further.
If the second page needs its own copy of the search controls or request handling, revise the
shared design. Differences in fields and server queries are expected.

## Scope

- Extract shared search controls, request handling, and result-page composition.
- Reuse existing record, table, classification, sorting, paging, and leave-confirmation components.
- Connect each search to its existing typed API and database queries.
- Keep domain actions, such as adding a Question or forking a Pool, with their existing owners.

## Non-goals

- Rebuild the database or combine all searches into one generic SQL engine.
- Add spreadsheet formulas, arbitrary cell editing, or a general page-building system.
- Add messaging features or change the decision about Stars and Watches.
- Store search terms or results in the database, browser storage, or a restoration service.

## Schematic

All three search definitions feed the same browser component. Proposed component names are
working names; this does not require a new service.

```text
 Questions definition    Pool definition    Blueprint Course definition
          |                    |                         |
          +--------------------+-------------------------+
                               |
                               v
                  SHARED SEARCH PAGE (browser)
          +------------------------------------------------+
          | Search text + filters + visible active choices |
          | Sort + display mode + page controls            |
          | Loading / empty / error / retry                |
          |                                                |
          | Compact rows | Expanded list | Visual boxes    |
          |      Same fields, result IDs, and actions      |
          +------------------------------------------------+
                               |
                    Shared request handling
                 current query + one result page
                               |
                  Content-specific API function
                               |
                  Existing server and PostgreSQL
                 validate access, filter, sort, page
                               |
                   One page of permitted results
                    returns to the shared page
```

The user-facing arrangement could be:

```text
 Search: [ordinary words                         ] [Search]
 Filters: [Discipline] [Subject] [More filters]
 Active:  Biology [x]  Genetics [x]

 Display: [Compact] [List] [Visual boxes]   Sort: [Title]

 Title                 Author / Owner       Subject        ...
 [available filter]    [available filter]   [available filter]
 -------------------------------------------------------------
 Matching result       Instructor name      Genetics       ...
 Matching result       Instructor name      Genetics       ...

 [Previous] [Next]                         Rows: [50 / 100 / 250]
```

This is a layout example, not a final screen design. A field offers a filter or sort only when
the server supports it. The column control and the top filter controls update the same query;
they do not maintain competing copies. List and visual-box modes keep filters available above
the results. Narrow screens preserve access to fields and controls through shared reflow or
table scrolling.

## Responsibilities

| Part | Owns | Example |
| --- | --- | --- |
| Search definition | Field labels, value display, available filters and sorts, stable result IDs, destinations, and API connection | Questions offer Question Type; Pools offer their existing classification filters |
| Shared search page | Search form, active filters, display choice, result composition, and paging controls | All three searches use the same clear-filter interaction |
| Shared search state | Draft input, applied query, current page, selection, loading, errors, and request ordering | A late response from an older query cannot replace newer results |
| Existing display components | Table cells, record details, images, actions, keyboard access, and responsive layout | A result uses the same title and destination in every display mode |
| Content-specific API code | Convert the typed query to its existing request and validate the returned data | Pool queries keep their existing Pool API |
| Server and database | Access checks, supported predicates, full-result sorting, and bounded pages | Sorting operates on every match, not only the 50 rows in the browser |

A search definition should stay small: labels, typed field access, supported controls, and
callbacks. Use ordinary TypeScript modules. Keep classification dependencies in the existing
classification component. Keep specialized editors and actions outside the generic search
state; existing selection and bulk-edit behavior must survive the move.

Display fields should be described once and reused by the table, list, and visual boxes.
Start with fields already available in each API. A missing Pool-owner identity is a separate
data problem; do not relabel an author or another recorded Instructor as the owner.

## Search behavior

1. Simple search supplies initial text and filters to this same page. Instructors can also
   begin directly in the full interface, including exploring without a text query.
2. Submitting search text or applying a filter creates an applied query and resets paging.
   Changing sort or page size also resets paging. Preserve current submit behavior; do not
   introduce an expensive search on every keystroke.
3. The API searches the database and returns one page. The shared state accepts a response
   only if it belongs to the current request. During replacement, loading and error states
   must not present old rows as matches for the newly applied query.
4. Changing display mode redraws the current results without rerunning the database search.
   Selection follows stable result IDs, not row positions. Preserve existing bulk-selection
   limits and query-change rules through the content-specific connection.
5. Opening a result opens its destination in a new tab or window. Buttons such as Search,
   Next, and Clear remain controls on this page. Opening a Pool for inspection follows the
   same result-link rule; forking remains an explicit existing content action.
6. Ribbon navigation stays in the current tab. Confirm before discarding an existing search;
   leave an untouched empty search page directly. Reuse the existing leave guard.

Browser memory holds only the currently open page's working state, including its bounded
result page and paging tokens. This is not saved-search storage. Closing or leaving the page
discards that state. An open tab can become stale: opening a result reads the destination
again, and changing content always uses current server checks. This design promises no
long-lived snapshot and adds no background refresh service.

## Approach

Implementation is future work. Use this serial sequence so the shared contract is tested
before all callers depend on it; no parallel implementation is needed for the initial extraction.

| Step | Owner and files | Depends on | Done when |
| --- | --- | --- | --- |
| S1: Extract shared search | One frontend owner; proposed `src/features/search/`, existing Question Library page/model and browse controls | None | Question search uses the shared page and retains its fields, selection, actions, and behavior |
| S2: Prove reuse | Same shared-component owner; Blueprint Course search page and its API connection | S1 | Blueprint search uses the same form, state handling, and display controls without copying them |
| S3: Move Pool search | One frontend owner; Pool discovery and its API connection | S2 | Pools use the same controls and display modes; result links open separately; existing Pool actions still work |
| S4: Close out | Integration owner; affected tests, reports, checklist evidence, and changelog | S3 | Duplicate search machinery is removed and integrated browser checks pass |

Each step ends with its focused behavior check before the next begins. Existing API and database
owners keep their current contracts unless a demonstrated search requirement needs a specific
addition. Pool ownership and Stars/Watches decisions do not block extracting shared controls.

## Verification

- Reuse existing tests for search, filters, sorting, paging, selection, and navigation. Add
  permanent coverage only for a shared behavior that is not already protected.
- Check one delayed-request case: an older response must not overwrite the latest search.
- Check that a later database page can contain the first result after changing sort; browser
  sorting of only the loaded page fails this check.
- In the running application, check all three searches, supported display modes, simple-search
  entry, result links, leave confirmation, keyboard use, and a narrow viewport.
- Confirm that changing display mode sends no new search request and creates no saved-search
  record or browser-storage entry. Use one-time inspection for this implementation evidence.
- Run the repository's fast and complete checks, then capture the affected UI with the existing
  screenshot workflow. Mark only behavior demonstrated by those checks as complete.

This schematic is complete when responsibilities and the reuse path are clear. Implementation
completion requires the behavior checks above; the existence of shared components alone is
not sufficient evidence.
