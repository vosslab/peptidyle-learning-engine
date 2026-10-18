# Shared search page architecture

This guide records the implemented shared boundary for searchable collections. SearchResults
composes DataSheetControls and DataSheetBlock; Compact, List, Cards, and Visual Boxes share
record content and interaction owners. Final integrated gates remain in the active plan.

Product behavior comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) and
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md).
[modular_ui_architecture_completion_plan.md](active_plans/active/modular_ui_architecture_completion_plan.md) and its
[current status report](active_plans/reports/datasheet_collection_status.md) are the current
authorities for milestone status, proof, and remaining acceptance. The earlier
[modular_ui_completion_plan.md](archive/modular_ui_completion_plan.md) and
[modular_ui_completion_ledger.md](active_plans/audits/modular_ui_completion_ledger.md) preserve
historical acceptance of the earlier 32 milestones.
Source descriptions here explain ownership.

## Owners and composition

| Owner | Current contract |
| --- | --- |
| Page or dialog | Owns authentication, route values, domain criteria, authorized source, actions, selection workflows, and editors. Supplies the query definition or complete-source adapter and the record fields. |
| `PageFrame` | Supplies route content width, an optional actions area, and the content stack. Ribbon and breadcrumbs identify the page; `PageFrame` has no heading props or heading markup. Dialogs remain page-owned. |
| `SearchBlock` | Places supplied criteria and controls `above` or `beside` one required collection. It composes page-supplied criteria and controls with the required `SearchResults` collection. |
| `SearchResults` | Mounts `DataSheetBlock` and `DataSheetControls` even for an initial empty search. Maps loading and error states to the collection and shows numbered page controls after activation. |
| `DataSheetBlock` | Owns all four modes and shared sorting, filtering, selection, and record actions. Uses `DataSheetTable` for Compact/List and `RecordItem` in every mode. |
| `DataSheetTable` | Owns the Compact/List table presentation, column configuration, and interactions: resize, show/hide, rearrange, responsive hiding, and independent temporary widths, visibility, and order for each table mode. Contains `HeaderRow` and `RecordItem` instances. |
| `HeaderRow` | Presents headings within `DataSheetTable`; forwards sort/filter input to `DataSheetBlock` and column input to `DataSheetTable`, without an independent controller. |
| `DataSheetControls` | Supplies surrounding modes, query controls, pagination, counts, and optional bulk UI through shared state/callbacks. Delegates collection behavior to `DataSheetBlock` and column mechanics to `DataSheetTable`. |
| Page/domain query owner | Owns collection scope, criteria, query execution, creation, and domain commands. Remote sorting and filtering apply to the full matching collection before paging. |

The collection has Compact, List, Cards, and Visual Boxes. Compact and List use DataSheetTable
with independent page-instance column settings; Cards are text-oriented and Visual Boxes are
image-oriented. DataSheetTable and HeaderRow implement the shared tabular boundary.
Mode changes preserve query, ordering, page, and selected identities without a query request. Query and
filter changes clear selection; sorting preserves it. Selection remains distinct from editor
membership and single-choice selection. See the active plan for the complete behavior contract.

`RecordField` declares a stable ID, label, value, and optional sort and filter keys. Its
`defaultColumn` declares priority, width, and initial visibility. One identity field supplies
the semantic title column; rendered labels do not become query values. The compact presentation
supports column sorting, filters, resize, visibility, and order. Sort and filter controls are
available in the other presentations too. The approved plan uses an approximate 20 px desktop
row target for compact Library results; it is an inspection target, not a fixed assertion.

Current `RecordListQuery.kind` is `paged` when rows are one page of a larger result set. Use
`local-complete` only when the rows passed to `DataSheetBlock` already are the complete displayed
collection. The query owner, including the local complete adapter below, applies collection-wide
operations before handing a page to a `paged` DataSheetBlock. A page must not sort or filter only the
visible server page.

## Two request paths

### Server-paged search

Use a focused `SearchDefinition` with `createSearchState` for server-owned paging. The definition
owns the query shape, initial query, cleanup, eligibility, text mapping, API request, row ID,
record content, applied chips, and selection maximum. Its `fetchPage` returns
items, the resolved page number and size, total count, and facet counts for that exact query.
Decode API data at the transport boundary. `question_library_search_definition.ts` and
`blueprint_course_search_definition.ts` are examples.

`SearchSession` is framework-neutral and owns one numbered page, request ordering, page size,
selection by stable row ID, retry, and reset. An eligible replacement query starts at page one
and clears old rows, counts, and selection. An ineligible query stays in the initial state.
Paging and refresh retain the visible verified page while loading; errors retain that page and
offer retry of the failed request. A late response cannot replace a newer result. `createSearchState`
connects the session to Solid, disposes it with the view, and exposes explicit submit, apply,
open, clear, page navigation, refresh, and selection operations.

Typing changes only the draft text. `submit()` commits it; `apply(query)` commits a composed
filter, sort, or chip change. `open(query)` handles an automatic route or host starting query.
`clear()` returns to the definition's initial query and runs it only when eligible. A page's
`beforeQueryChange` hook may protect content-owned work before a query replacement.

### Existing complete source

Use `createLocalCompleteSearch` when a page already has one complete, authorized reactive
collection. Pass a `rows` accessor; `undefined` means undelivered and an empty array means a
delivered empty collection. Supply optional `loading`, `error`, and `refetchSource` callbacks
when the source has that lifecycle. A synchronous derived collection needs only `rows`.
The adapter in `local_complete_search_state.ts` uses
`local_complete_search.ts` to match keywords, apply OR within each field and AND across fields,
sort with a stable ID tie-break, derive whole-result facet choices, and then take a page. It
exposes the same `SearchStateController` and a `paged` `RecordListQuery`. The caller supplies
fields, stable ID, keyword text, content, and any fixed domain scope or special comparator.

The adapter projects source loading and error over the session's retained rows and counts.
Retry refetches a failed source when `refetchSource` is supplied; otherwise it retries through
the session. The first delivered source opens the initial query. Later source updates refresh the
current query, remove selected IDs missing from the full authorized source, and step back when
a deletion leaves the last page empty. Query, filter, sort, and page changes use the current
source; explicit refresh calls `refetchSource` when supplied, otherwise it refreshes the session.
The page owns mutation-driven source updates or refetches. Consumers such as My
Drafts and Course collections keep their own data lifecycle and domain actions.

## SearchBlock and controls

Current `SearchBlock` requires `layout`, supplied controls, a state controller, and a collection with
an accessible label, empty state, fields, and query binding. The collection may also supply an
initial empty state, row body, selection, and a disabled state for a content action. The block
has no alternate results slot. SearchResults composes DataSheetControls and the required
DataSheetBlock within SearchBlock. `SearchControls` renders keyword
submission, Clear all, applied chips, and content controls. DataSheetControls invokes
DataSheetBlock's shared sorting/filtering state and callbacks; DataSheetTable owns column mechanics
and contains HeaderRow. The query owner applies whole-result sorting/filtering before pagination:
PostgreSQL `WHERE` and `ORDER BY` for database-backed collections, equivalent shared processing for
complete local sources. A page with another local text field can use `onSubmit` to compose it into
one query; `hasDraft` and `onClear` concern that control's
local draft only.

Question Library Search uses controls above a full-width collection. Browse places its
classification navigator beside the same collection and stacks it above results at narrow
widths. My Questions uses the same Library component with a predefined authorship scope and
loads on entry. The Library definition requires a keyword or filter for an ordinary Search
request; the empty initial DataSheetBlock stays mounted with its fields and controls. A predefined
collection scope can load automatically. In current code, Browse and My Questions use the same
RecordField model and result actions as Search.

The Library API owns complete-result matching, facets, deterministic sort, and global paging
for mixed Question and Pool rows. `LibrarySearchRow` carries the object kind; page actions and
bulk editors interpret that kind. Question-only callers explicitly select Questions before
decoding the Question row contract. The Question picker retains its ordered selection tray as
domain state, separate from search selection. The Assessment content picker uses mixed Library
discovery and applies its own single-Pool or multiple-Question selection rule.

## Protecting actual edits

Search text, result state, and display settings are temporary discovery state. `SearchBlock`
does not guard navigation for them. Pool creation and Question or Pool metadata editors use
`UnsavedChangesGuard` with editor-owned meaningful dirty checks. The shared guard handles route
navigation and browser unload; `manualLeave` covers Cancel, close, selection, or query changes
that would discard local editor work. Save proceeds only after confirmed success; a failed or
uncertain write leaves the editor open. `SearchStateOptions.beforeQueryChange` lets a Library
query transition ask those editors before removing their work.

## Adding a consumer

1. Decide whether the API owns a numbered-page query or the page already holds the complete
   authorized source. Reuse the corresponding request path above.
2. Supply stable record IDs, semantic fields, record content and actions, query-wide sort and
   filter bindings, and an accurate accessible label. Keep domain scope and authorization at
   the page/API boundary.
3. Mount `SearchControls` and `SearchBlock` with an explicit `above` or `beside` layout. Let
   `SearchResults` keep the initial DataSheetBlock and page controls in one place.
4. Keep dialogs, workflows, and dirty checks with their owning page or editor. Search snapshots
   stay unchanged after cross-tab edits until a new query or explicit refresh. Protect only
   meaningful unsaved work before an operation removes it.
5. Verify the actual consumer through its owner checks and browser or rendered evidence as
   needed. Put one-time proof in `tests/_temp/`; retain a permanent test only when its stable
   assertion earns its fixture cost under [PYTEST_STYLE.md](PYTEST_STYLE.md). Record acceptance in
   the completion ledger.
