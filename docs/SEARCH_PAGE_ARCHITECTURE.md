# Shared search page architecture

This guide describes the shared search code used by the mixed Question Library, Public Blueprint
Course search, Question picker, and Assessment content picker. It describes source behavior;
active-plan evidence and final visual review remain separate from this guide.

## Responsibility boundary

| Part | Responsibility | Source |
| --- | --- | --- |
| Definition | Owns the query shape, cleanup, API request, row ID, row content, sort, chips, and selection limit. | `src/pages/*_search_definition.ts` or the feature model |
| Session | Owns one cursor-page sequence, request ordering, retry, selection, and reset. It has no Solid dependency. | `src/features/search/search_session.ts` |
| State controller | Connects a session to Solid signals. It owns typed text, explicit submit, display mode, page controls, and disposal. | `src/features/search/search_state.ts` |
| Controls | Renders the text field, Search, Clear all, applied chips, optional sort, display buttons, and a content-control slot. | `src/features/search/search_controls.tsx` |
| Results | Renders `RecordList`, empty/loading/error states, and cursor-page controls. | `src/features/search/search_results.tsx` |
| Selection bar | Renders loaded and selected counts plus common selection commands. Its action slot remains content-owned. | `src/features/search/search_selection_bar.tsx` |
| Page shell | Provides `PageFrame`, responsive search columns, optional toolbar/results/extra slots, and the leave guard. | `src/features/search/search_page.tsx` |

Use `SearchSession` for framework-neutral request behavior. Use `createSearchState` once for each
open Solid page or dialog surface. Do not create another request, cursor, retry, or display-mode
state alongside it.

## Definition checklist

Create a focused definition beside the page or feature model, following
`src/pages/blueprint_course_search_definition.ts` or
`src/pages/question_library_search_definition.ts`.

- Define one immutable query type and one function that returns its starting values.
- Provide `initialQuery`, `cleanup`, `getText`, and `setText`.
- Make `cleanup` normalize values before the request; for example, trim submitted text.
- Implement `fetchPage(query, cursor, pageSize)` with the content API and return items, the next
  server cursor, and content-owned filter counts.
- Decode transport data before it reaches the definition when the API client needs decoding.
- Provide a stable `rowId` and `content` for `RecordList`. Result links and their target behavior
  belong in that content description.
- Add `sort` only when the server accepts an order. Its `get` and `set` functions work only on the
  query object.
- Add `appliedFilters` for every committed filter that a user can remove. Each chip returns a new
  query through `clear`; the shared control applies it once.
- Add `selection.maximum` only when the search supports selection.

Keep domain names, filter labels, API calls, and row presentation in the definition or its nearby
content helpers. Keep generic paging and request coordination in `src/features/search/`.

## State and request contract

`SearchSession` starts idle. `apply` starts a replacement request at page one, clears selection,
and clears rows and filter counts while the replacement is loading. A response carries a request
number, so a late response cannot replace a newer one. Failed page navigation keeps its current
page and filter counts; `retry` repeats the failed request. `reset` invalidates an in-flight
request and returns to the definition's initial query without a request.

The state controller exposes two ways to run a query:

- `submit()` commits the shared text field with the current committed query.
- `apply(query)` commits a content-owned filter, sort, chip removal, or composed submit query.

Typing updates only `typedText`; it does not request results. Display changes redraw the saved
page locally and do not request results. Page size, next, previous, and retry use the existing
query and cursor sequence.

Use `open(query)` for an automatic opening from a URL or Browse route. It loads the query without
marking the page as user-searched. `SearchStateOptions.initialQuery` supplies a starting query;
`runOnMount` calls `open` for that query. The Library route uses `open` when its URL supplies
search values or when Browse opens.

## User, Clear, and leave contracts

`searchUsed()` is true after a user-directed search operation. `hasTypedDraft()` is true only for
nonempty shared text entered on this open page. `SearchPage` shows its leave guard when either is
true, or when its optional `hasDraft` callback reports a content-owned unsent draft. An automatic
`open` leaves directly.

`Clear all` always calls the shared `state.clear()` first. It returns the controller to idle,
empties shared typed text and results, clears selection, and sends no request. An optional
`SearchControls.onClear` callback runs afterward for content-owned draft UI. If a page adds a
local text field, pass `hasDraft` to both `SearchControls` and `SearchPage` so Clear is visible
and leaving is protected before first submit.

A local text field can join the shared explicit submit boundary with `SearchControls.onSubmit`.
The callback composes the local draft and `state.typedText()` into one `state.apply` call. The
Public Blueprint Tag field is the current example. Do not request once per keystroke for a local
free-text filter.

## Page slots

`SearchControls` accepts content-specific filter controls as children. Filter changes that should
run immediately call `state.apply` with a complete next query. The controls component owns the
universal form controls and applies chips and sort changes itself.

`SearchResults` accepts an accessible list label and content-owned empty state. Its optional
`selection` supplies row checkboxes; its optional `disabled` holds page navigation during a
content action; its optional `rows` lets a page with a deliberate discovery policy withhold rows.

`SearchSelectionBar` accepts callbacks for selecting loaded rows and clearing selection, plus a
children slot for content actions. `SearchPage` accepts toolbar, results, and extra slots. Use
`extra` for a dialog that must remain mounted while a task hides the main surface.

## Add a search

1. Define the query, starting values, API-to-`SearchPage` adapter, `rowId`, and `content` in a
   new page-local definition module.
2. Construct `createSearchState(definition)` in the authenticated page or dialog composition.
   Use `initialQuery` and `open` only for a real route or host starting value.
3. Place domain filters in `SearchControls`; use one composed `onSubmit` only when the page has
   additional local free text.
4. Render `SearchResults` with an accurate accessible label and empty state. Add
   `SearchSelectionBar` only when the definition declares selection.
5. Wrap the surface in `SearchPage`, passing any local-draft callback needed for Clear and the
   leave guard.
6. Keep the page responsible for authentication, route parsing, domain actions, and its API
   clients. Keep server-side authorization and filtering in the API.

## Focused tests

`tests/test_search_session.mjs` protects the framework-neutral contract: late responses, reset,
retry, paging, page size, and selection. Extend it when session behavior changes.

The existing live Library browser journey in
`tests/playwright/e2e_live_demo_question_library_browser.mjs` protects explicit submit,
local display changes with visible descriptions, mixed membership, protected new-tab detail
navigation, and the leave warning. The connected database tests own filtering and global paging.
A failure means one of these contracts changed; repair the behavior or confirm a deliberate
product change before updating its assertion.

Use the permanent-test checklist in [PYTEST_STYLE.md](PYTEST_STYLE.md) before adding coverage.
A new consumer does not automatically need another harness or a copy of the shared tests.
Use temporary browser checks for integration, accessibility, and rendered review; retain a new
case only for an important regression not already protected by these owners. Screenshot scenes
remain the repository's repeatable capture workflow.

## Mixed Library boundary

`LibrarySearchRow` is a discriminated Question/Pool row union. The Library definition begins
with Both and Questions in no Pool. Its no-Pool chip is visible even while search is idle;
removal broadens membership to All Questions. Unrestricted choices have no removal chip.
The combined server query owns filtering, facets, deterministic order, and global paging.

Pool-only queries clear structured Question-only predicates. Search text stays opaque to the
browser; the server owns field grammar. Pool text matches title, description, classification,
and Tags. Pool Type is available through the structured Type filter, not text Type matching.
The definition emits owner and Pool metadata separately
from Question authorship. `/library/{id}` resolves authorized object kind before mounting its
Question or Pool detail view, and RecordList links preserve the search in a separate tab.

Library bulk actions keep selected kind and title alongside the generic selected-ID set. They
partition the one capped selection into existing Question and Pool metadata editors. Completing
one kind removes only those IDs through `deselectIds`; the other kind remains selected through
refresh. Generic shared code knows neither kind nor metadata payload.

Question-only callers explicitly request Questions and All membership before decoding the
Question-only row contract. This includes My Questions and source-bound Pool creation. They do
not inherit the Library's mixed discovery default.

## Picker consumers

The Question picker remains Question-only. Its ordered selection tray is domain state, separate
from shared search selection: search Clear resets discovery but leaves the tray intact, while
**Clear selection** clears the ordered tray. Changing picker source also resets that tray.
Pool-source eligibility remains in the picker model rather than shared search.

The Assessment content picker uses the mixed Library definition. It accepts either one Pool or one
or more Questions within its configured Question maximum, never a mixed Question-and-Pool choice.
It converts retained selected rows into the existing Assessment Question or Pool import inputs.
It replaces the former Question Pool picker rather than creating a parallel Pool-picker surface.

## Remaining Pool validation handoff

This search work does not close the separate Pool validation handoff: mismatch warnings,
release-blocking behavior, classification re-checks, unordered member storage, and a sortable
member editor remain open product and implementation work.
