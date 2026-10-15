# Shared search page architecture

This guide describes the shared search code used by the mixed Question Library, Public Blueprint
Course search, Question picker, and Assessment content picker. It describes source behavior;
active-plan evidence and final visual review remain separate from this guide. Intended interface
behavior belongs to [UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md), following
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Current wiring below is implementation evidence and does
not override those requirements.

## Intended shared interface and current gaps

PageFrame owns page structure; RecordList owns shared collection presentation and interaction.
Individual pages supply domain content and workflows. Search adds keywords above full-width
RecordList results; Browse adds a compact classification navigator beside the same component.
See [UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md#question-library-discovery) for initial states,
column behavior, and navigation. Question data and query contracts remain in
[README.md](QUESTION_SPECS/README.md).

Source inspection identifies these differences from that intended behavior:

| Current source behavior | Required behavior |
| --- | --- |
| `search_results.tsx` withholds RecordList in the initial state; `library_page.tsx` also withholds Search filters. | Show an empty spreadsheet with usable column filters before a keyword or filter is applied. |
| `search_state.ts` submits and applies queries without a minimum-criteria check; the sort control calls `apply`. | Search requires a keyword or filter; sorting alone does not initiate an unfiltered query. |
| `search_page.tsx` guards leaving after search activity or typed search text. | Navigate directly between search and browsing; protect meaningful unsaved edits. |

This documentation update does not implement those changes or establish browser acceptance.
Column resizing, hiding, arrangement, and shared display behavior also require implementation
verification against the UI guide; their specification is not evidence of runtime completion.

## Search and Browse source audit

Source review on October 15, 2026 found two `SearchPage` callers: the Question Library page
(both Search and Browse) and Blueprint Course search. Pickers reuse search controls and results
without mounting `SearchPage`.

### Layout findings

- Both Library routes declare full-width content in `src/route_contract.ts`. PageFrame is not
  what narrows Search results.
- `src/features/search/search_page.tsx` provides toolbar, results, and extra slots. It chooses
  an initial-state CSS class from request state, with no Search/Browse layout setting.
- At widths of at least `75rem`, `src/features/search/search.css` gives every noninitial page
  a `16rem` to `22rem` toolbar column beside results. Initial pages instead use one column
  limited to `42rem`. Below that breakpoint, controls and results stack. Loading a search
  therefore changes its layout, even though the discovery mode has not changed.
- `src/pages/library_page.tsx` already supplies the same `SearchResults` and RecordList for both
  modes. Bulk actions and row selection are also shared. Separate result implementations are
  unnecessary.
- Sort and display buttons currently live in `SearchControls`, which Library mounts only in
  Search mode. Browse therefore does not receive those controls. Search filters also live in
  separate page controls rather than RecordList column headings.

### Leave guard findings

`src/components/search_leave_guard.tsx` is mounted only by SearchPage. It intercepts router
navigation and browser unload, including reload or closing the tab. Its router dialog says that
search results will be discarded. It does not save anything.

SearchPage activates it for `searchUsed()`, `hasTypedDraft()`, or its optional `hasDraft` callback.
The only caller supplying that callback is Blueprint Course search, where it checks the Tag
search field. None of these inputs describes an unsaved record edit. Automatic `open()` does not
set the user-search flag, so merely loading Browse does not activate the guard.

There is nevertheless real editable content on the Library page. Pool creation keeps a Title,
description, and member selection in local state. Question and Pool metadata editors keep pending
classification and Tag changes. Inspection of `question_pool_create_dialog.tsx`,
`question_bulk_metadata_editor.tsx`, `question_pool_search_metadata_editor.tsx`, and
`library_bulk_actions.tsx` found no connection from those edits to either leave guard. Busy flags
disable some controls but do not provide navigation protection.

The search guard can incidentally block navigation while one of those editors is open after a
search. It does not reliably protect those edits: the same editor can be reached without prior
user search activity. Removing the search guard alone would remove that incidental protection.

### Recommended changes

1. Give SearchPage one explicit layout choice: controls above results or a sidebar beside results.
   Choose it from the page's purpose, not its request state. Library Search uses controls above;
   Browse uses the sidebar. Stack the Browse sidebar above results when width is limited.
   Make Blueprint Course search's choice explicit when updating the shared caller.
2. Require RecordList inside SearchBlock and retain the existing shared search state. Put shared sorting,
   column filtering, display controls, and record actions with the shared collection presentation
   so both modes expose them. Keywords and classification navigation supply criteria to that
   same result set. Do not copy RecordList behavior into either layout.
3. Keep the empty Search spreadsheet and its column controls mounted. Enforce the keyword-or-filter
   requirement before requesting Search results, including sorting and clearing the last criterion.
4. Replace incidental search-based protection with editor-owned checks for meaningful unsaved
   changes in Pool creation and metadata editing. Reuse `src/components/unsaved_changes_guard.tsx`
   where its Save/Discard behavior fits the editor. Check pending saves as part of that work.
5. Remove SearchLeaveGuard from SearchPage and remove its search-only `hasDraft` input. Keep
   `SearchControls.hasDraft`, which separately makes Clear available for an unsubmitted local
   search field. Ordinary Search/Browse navigation, reload, and tab close should not warn about
   temporary search state.

These are recommendations for implementation, not completed code changes. Check both layouts
before and after queries at wide and narrow widths, shared controls in both modes, direct
navigation with search state alone, and protection for actual unsaved edits reached both with
and without prior search activity. This audit inspected source; it did not run browser checks.

## Unsaved editor protection review

Source review on October 15, 2026 covered the shared guard and its current callers. This section
recommends implementation work; application code remains unchanged.

### Existing shared behavior

`src/components/unsaved_changes_guard.tsx` accepts an editor-owned `dirty` check, a save function
returning success or failure, and dialog wording. It intercepts router navigation while dirty.
Its standard dialog offers Stay, Save, and Discard. Save continues only when the callback returns
true; exceptions and false results leave the user on the page. Discard resumes the requested
navigation. Buttons are disabled while the guard's own save is running.

The guard also registers `beforeunload` while mounted and requests the browser's warning when
dirty. This path does not run its save callback or show the application's three-button dialog.
`manualLeave` lets a caller protect an action that removes local work without changing routes.
The caller must request that prompt only when there are unsaved changes; the guard does not apply
its dirty check to a manual request.

| Current callers | Protection style |
| --- | --- |
| Question Draft editor, Native JSON editor, Student Assessment attempt | Try saving on router navigation; show the dialog if saving fails. |
| Blueprint Course detail, Course classification, fixed Question points, Assessment Question Sequence | Ask before leaving unsaved edits. The Question Sequence wrapper supplies wording to the same shared guard. |
| Assessment Templates | Also protects replacing the current editor through `manualLeave`. |
| Blueprint Course creation | Also protects closing the creation dialog. Its required save callback always returns false and labels the save button Keep editing. |

The Blueprint creation callback exposes a small gap in the shared component: it requires a Save
action even when a task should only offer Stay or Discard. Copying that callback would duplicate
an awkward workaround. The guard also knows only about saves it starts itself; it does not
automatically wait for an editor's already-running request.

### Library editor gaps

- Pool creation owns local Title, description, and member choices. Its Create operation validates
  those values and sends the creation request. It has no unsaved-changes guard.
- Question metadata editing owns Keep/Replace/Clear choices, classification values, and Tag text.
  It has an explicit update operation and handles stale or uncertain outcomes by refreshing
  metadata. It has no unsaved-changes guard.
- Pool metadata editing owns similar local choices and an explicit update operation. It also
  lacks the guard.
- `library_bulk_actions.tsx` removes editors when selection or query changes, as well as on Cancel
  and successful completion. A route-only guard would miss these ways of losing edits.
- Search activity is not an editor dirty check. The current SearchLeaveGuard sometimes blocks
  these actions' surrounding navigation, but provides no reliable protection for their edits.

### Simplest consistent approach

Keep one shared UnsavedChangesGuard. Each editor supplies whether it has meaningful unsaved work;
the shared guard handles navigation and the confirmation dialog. Keep this responsibility out of
RecordList, SearchPage, and search state.

1. Allow the shared guard's Save action to be omitted. Pool creation should then offer Stay and
   Discard, while its existing Create button remains the explicit creation action. Apply the same
   option to Blueprint creation instead of its always-failing save callback. No automatic creation
   or new persistent draft store is needed.
2. Metadata editors should offer Stay, Save, and Discard using their existing update operation.
   Make the save callback report confirmed success without needing a form event. Validation,
   stale data, and failed or uncertain writes must keep the editor open. Preserve existing
   metadata refresh and error handling rather than treating any completed request as success.
3. Compare local work with the starting or last successfully saved values. Opening an editor,
   loading records, or receiving a source Question does not itself make it dirty. Pool text and
   user changes to member choices do; metadata changes include pending replacement text and
   Keep/Replace/Clear choices, even if incomplete. Reverting all changes clears the dirty state.
   Keep picker discovery terms separate from member choices retained for Pool creation.
4. Route Cancel, close, and any query or selection change that would remove an editor through the
   same leave decision. Resume the requested action only after Save succeeds or Discard is chosen.
   Changing picker views while retaining the work does not require confirmation. Reuse
   `manualLeave`; do not introduce separate dialogs for each way of leaving.
5. Use the editor's actual saving state to prevent duplicate writes or premature navigation.
   If a write is already running, wait for its outcome before continuing. Discard drops local
   edits; it cannot cancel or undo a request already sent. Browser unload remains a browser
   warning, not a promise to finish saving.
6. Wire these editor checks before removing SearchLeaveGuard. Retain direct Search/Browse
   navigation whenever only temporary search state exists. Remove search-only guard inputs while
   retaining SearchControls' separate Clear-button handling.

Check untouched, edited, reverted, incomplete, successfully saved, failed, and already-saving
states. Cover router navigation, reload/tab close, editor Cancel, and parent query/selection
changes, both with and without prior search activity. These checks are proposed; this review did
not run browser tests.

The layout recommendation remains unchanged: Search supplies keywords above full-width results;
Browse supplies classification navigation beside results, stacking on narrow screens. Choose by
Search or Browse mode. One shared RecordList owns sorting, column filtering, display modes, and
record actions in both layouts.

## Intended SearchBlock boundary

SearchBlock is the intended replacement for the current SearchPage composition, not an existing
component. The complete `search_page.tsx` review found four responsibilities: wrapping PageFrame
and forwarding its title, short label, introduction, actions, and other props; arranging toolbar
and results slots; keeping extra content mounted alongside them; and mounting SearchLeaveGuard.
It also reads search state to choose the initial-layout class and activate the guard. It does not
fetch records or own query, paging, sorting, or display-mode state.

The simplest change is to move PageFrame composition to the page and require RecordList inside
SearchBlock, with one explicit above-or-beside layout choice for supplied controls. Pages connect
their controls to the existing shared search state. The block needs neither a domain-specific
Search/Browse mode nor search-activity checks. Search and Browse pages choose the appropriate
layout; Blueprint Course search supplies its own controls. Keep one shared RecordList.

SearchBlock is a required part of displaying searchable collections, not an optional wrapper
around arbitrary result content. PageFrame can display ordinary content directly, but search
results go through SearchBlock and its RecordList. My Questions, My Pools, My Drafts, Starred,
Watched, Search Question Library, and Browse Question Library follow this same intended structure,
with page-owned criteria and controls. My Questions supplies a predefined filter. Preserve each
collection's access and lifecycle rules; this composition does not make Drafts Published Library
Objects or apply Search Question Library's empty initial state to every collection. These pages'
current implementations have not been audited for this expanded requirement.

Page-owned dialogs currently passed through `extra` can sit alongside SearchBlock inside
PageFrame, remaining mounted when the discovery block is hidden for Pool creation. Preserve
that behavior during the change. Editor-owned unsaved protection also stays outside SearchBlock.
Remove the old PageFrame heading props in line with current Human Guidance; a rename alone
would leave both the page-wrapper responsibility and the unwanted heading features in place.

The earlier numbered recommendations describe changes to current SearchPage. Apply their layout
recommendation to SearchBlock under this intended split. Keep source descriptions below named
SearchPage until application code actually changes.

## Current responsibility boundary

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

## Current Clear and leave behavior

`searchUsed()` is true after a user-directed search operation. `hasTypedDraft()` is true only for
nonempty shared text entered on this open page. `SearchPage` shows its leave guard when either is
true, or when its optional `hasDraft` callback reports a content-owned unsent draft. An automatic
`open` leaves directly. This search-activity guard conflicts with current Human Guidance. Search
terms and existing results alone should not trigger confirmation; meaningful unsaved content
edits remain the reason to protect navigation.

`Clear all` always calls the shared `state.clear()` first. It returns the controller to idle,
empties shared typed text and results, clears selection, and sends no request. An optional
`SearchControls.onClear` callback runs afterward for content-owned draft UI. The current
`hasDraft` hooks also expose Clear before first submit. A local search field is not, by itself,
a reason to activate the navigation guard.

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
3. Supply domain filter meanings to the shared controls and RecordList, following the UI guide's
   column-filter behavior. Use one composed `onSubmit` for additional local free text.
4. Render `SearchResults` with an accurate accessible label and empty state. Add
   `SearchSelectionBar` only when the definition declares selection.
5. Wrap the surface in `SearchPage`. Keep Clear state separate from protection for meaningful
   unsaved content edits; do not propagate the current search-activity leave guard.
6. Keep the page responsible for authentication, route parsing, domain actions, and its API
   clients. Keep server-side authorization and filtering in the API.

## Focused tests

The former `test_search_session.mjs` and live Library browser journey protected late responses,
reset, retry, paging, page size, selection, explicit submit, display changes, protected detail
navigation, and the leave warning. They were retired during test pruning, so this is historical
evidence rather than current Search coverage. The database-baseline lane does not replace that
browser coverage.

Use the permanent-test checklist in [PYTEST_STYLE.md](PYTEST_STYLE.md) before adding coverage.
A new consumer does not automatically need another harness or a copy of the shared tests.
Use temporary browser checks for integration, accessibility, and rendered review; retain a new
case only for an important regression not already protected by these owners. Screenshot scenes
remain the repository's repeatable capture workflow.

## Mixed Library boundary

`LibrarySearchRow` is a discriminated Question/Pool row union. The Library definition begins
with Both and Questions in no Pool. Its no-Pool chip is visible even while search is idle;
removal broadens membership to All Questions. Unrestricted choices have no removal chip.
These are current query defaults, not an instruction to populate Search on entry. The default
object-kind policy remains tentative, separately from the settled empty initial Search display.
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
