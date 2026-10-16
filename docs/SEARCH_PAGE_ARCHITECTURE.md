# Shared search page architecture

This guide describes the implemented composition for searchable collections.

Product behavior comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) and
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md).
[modular_ui_completion_ledger.md](active_plans/audits/modular_ui_completion_ledger.md) is the sole
authority for milestone status, proof, and remaining acceptance.
[modular_ui_completion_plan.md](archive/modular_ui_completion_plan.md) retains
historical decisions and gates. The ledger records current native acceptance and canonical gates;
source descriptions here explain ownership.

## Owners and composition

| Owner | Current contract |
| --- | --- |
| Page or dialog | Owns authentication, route values, domain criteria, authorized source, actions, selection workflows, and editors. Supplies the query definition or complete-source adapter and the record fields. |
| `PageFrame` | Supplies route content width, an optional actions area, and the content stack. Ribbon and breadcrumbs identify the page; `PageFrame` has no heading props or heading markup. Dialogs remain page-owned. |
| `SearchBlock` | Places supplied controls `above` or `beside` one required `SearchResults` collection. Layout comes from the consumer, independent of request state. Embedded pickers can use it without `PageFrame`. |
| `SearchResults` | Mounts `RecordList` even for an initial empty search. Maps loading and error states to the collection and shows page controls after activation. |
| `RecordList` | Renders fields, row content and actions, selection controls, and Compact, List, and Visual boxes presentations. It owns temporary mode, column visibility, order, and resize settings. A mode change does not fetch another result set. |
| Query owner | Applies text, filters, and sort to the whole authorized matching collection before paging. It supplies column filter choices and sort bindings to `RecordList`. |

`RecordField` declares a stable ID, label, value, and optional sort and filter keys. Its
`defaultColumn` declares priority, width, and initial visibility. One identity field supplies
the semantic title column; rendered labels do not become query values. The compact presentation
supports column sorting, filters, resize, visibility, and order. Sort and filter controls are
available in the other presentations too. The approved plan uses an approximate 20 px desktop
row target for compact Library results; it is an inspection target, not a fixed assertion.

`RecordListQuery.kind` is `paged` when rows are one page of a larger result set. Use
`local-complete` only when the rows passed to `RecordList` already are the complete displayed
collection. The query owner, including the local complete adapter below, applies collection-wide
operations before handing a page to a `paged` RecordList. A page must not sort or filter only the
visible server page.

## Two request paths

### Server-paged search

Use a focused `SearchDefinition` with `createSearchState` for server-owned paging. The definition
owns the query shape, initial query, cleanup, eligibility, text mapping, API request, row ID,
record content, applied chips, and selection maximum. Its `fetchPage` returns
items, the next cursor, and counts for that exact query. Decode API data at the transport
boundary. `question_library_search_definition.ts` and
`blueprint_course_search_definition.ts` are examples.

`SearchSession` is framework-neutral and owns one cursor sequence, request ordering, page size,
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

### Existing complete resource

Use `createLocalCompleteSearch` when a page already has one complete, authorized
`createResource` result. The adapter in `local_complete_search_state.ts` uses
`local_complete_search.ts` to match keywords, apply OR within each field and AND across fields,
sort with a stable ID tie-break, derive whole-result facet choices, and then take a page. It
exposes the same `SearchStateController` and a `paged` `RecordListQuery`. The caller supplies
fields, stable ID, keyword text, content, and any fixed domain scope or special comparator.

The adapter projects source loading and error over the session's retained rows and counts.
Retry refetches a failed source; a session request failure retries through the session. The
first successful source load opens the initial query. Later successful reloads refresh the
current query, remove selected IDs missing from the full authorized source, and step back when
a deletion leaves the last page empty. Query, filter, sort, and page changes use the cached
resource; a mutation or explicit refresh refetches the existing source. Consumers such as My
Drafts and Course collections keep their own data lifecycle and domain actions.

## SearchBlock and controls

`SearchBlock` requires `layout`, supplied controls, a state controller, and a collection with
an accessible label, empty state, fields, and query binding. The collection may also supply an
initial empty state, row body, selection, and a disabled state for a content action. The block
has no alternate results slot. `SearchControls` renders keyword submission, Clear all, applied
chips, and content controls. `RecordList` owns every sort control and invokes the supplied query
binding; the query owner applies whole-result sorting. A page with another local text field
can use `onSubmit` to compose it into one query; `hasDraft` and `onClear` concern that control's
local draft only.

Question Library Search uses controls above a full-width collection. Browse places its
classification navigator beside the same collection and stacks it above results at narrow
widths. My Questions uses the same Library component with a predefined authorship scope and
loads on entry. The Library definition requires a keyword or filter for an ordinary Search
request; the empty initial RecordList stays mounted with its fields and controls. A predefined
collection scope can load automatically. Browse and My Questions use the same RecordList field
model and result actions as Search.

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

1. Decide whether the API owns a cursor-paged query or the page already holds the complete
   authorized source. Reuse the corresponding request path above.
2. Supply stable record IDs, semantic fields, record content and actions, query-wide sort and
   filter bindings, and an accurate accessible label. Keep domain scope and authorization at
   the page/API boundary.
3. Mount `SearchControls` and `SearchBlock` with an explicit `above` or `beside` layout. Let
   `SearchResults` keep the initial RecordList and page controls in one place.
4. Keep dialogs, workflows, and dirty checks with their owning page or editor. Protect only
   meaningful unsaved work before an operation removes it.
5. Verify the actual consumer through its owner checks and browser or rendered evidence as
   needed. Put one-time proof in `tests/_temp/`; retain a permanent test only when its stable
   assertion earns its fixture cost under [PYTEST_STYLE.md](PYTEST_STYLE.md). Record acceptance in
   the completion ledger.
