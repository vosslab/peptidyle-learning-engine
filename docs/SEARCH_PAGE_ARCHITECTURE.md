# Shared search page architecture

This guide records the implemented shared boundary for searchable collections. SearchBlock adapts
the search controller and requires one DataSheetBlock; DataSheetBlock mounts DataSheetControls
before the record region. Compact, List, Cards, and Visual Boxes share record content and
interaction owners. The active plan records current implementation review and remaining gates.

Product behavior comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) and
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md).
[COLLECTION_ARCHITECTURE_SIMPLIFICATION_PLAN.md](active_plans/active/COLLECTION_ARCHITECTURE_SIMPLIFICATION_PLAN.md)
tracks current milestone status, review, and remaining acceptance. The earlier
[modular_ui_architecture_completion_plan.md](active_plans/active/modular_ui_architecture_completion_plan.md) and its
[status report](active_plans/reports/datasheet_collection_status.md) record broader collection
implementation evidence. The earlier
[modular_ui_completion_plan.md](archive/modular_ui_completion_plan.md) and
[modular_ui_completion_ledger.md](active_plans/audits/modular_ui_completion_ledger.md) preserve
historical acceptance of the earlier 32 milestones.
Source descriptions here explain ownership.

## Owners and composition

| Owner | Current contract |
| --- | --- |
| Page or dialog | Owns authentication, route values, domain criteria, authorized source, actions, selection workflows, and editors. Supplies the query definition or complete-source adapter and the record fields. |
| `PageFrame` | Page route compositions own and render the route's PageFrame, which supplies content width, an optional actions area, and the content stack. Ribbon and breadcrumbs identify the page; `PageFrame` has no heading props or heading markup. Dialogs remain page-owned. |
| `SearchBlock` | Places supplied criteria and controls `above` or `beside` one required collection. Adapts `SearchStateController` lifecycle state and passes rows, empty state, and numbered pagination to `DataSheetBlock`. |
| `DataSheetBlock` | Owns all four modes, standard `DataSheetControls`, and shared sorting, filtering, selection, and record actions. It mounts controls before the record region, uses `DataSheetTable` for Compact/List, and uses `RecordItem` in every mode. |
| `DataSheetTable` | Owns the Compact/List table presentation, column configuration, and interactions: resize, show/hide, rearrange, responsive hiding, and independent temporary widths, visibility, and order for each table mode. Contains `HeaderRow` and `RecordItem` instances. |
| `HeaderRow` | Presents headings within `DataSheetTable`; forwards sort/filter input to `DataSheetBlock` and column input to `DataSheetTable`, without an independent controller. |
| `DataSheetControls` | Supplies surrounding modes, query controls, pagination, counts, and optional bulk UI from the presentation and table state owned by `DataSheetBlock`. |
| Page/domain query owner | Owns collection scope, criteria, query execution, creation, and domain commands. Remote sorting and filtering apply to the full matching collection before paging. |

The collection has Compact, List, Cards, and Visual Boxes. Compact and List use DataSheetTable
with independent page-instance column settings; Cards are text-oriented and Visual Boxes are
image-oriented. DataSheetTable and HeaderRow implement the shared tabular boundary.
Mode changes preserve query, ordering, page, and selected identities without a query request. Query and
filter changes clear selection; sorting preserves it. Selection remains distinct from editor
membership and single-choice selection. Reorder bindings identify source and target records by stable
IDs; the shared provider keeps temporary display positions inside its interaction implementation.
See the active plan for the complete behavior contract.

`RecordField` declares a stable ID, label, value, and optional sort and filter keys. Its
`defaultColumn` declares priority, width, and initial visibility. One identity field supplies
the semantic title column; rendered labels do not become query values. The compact presentation
supports column sorting, filters, resize, visibility, and order. Sort and filter controls are
available in the other presentations too. The approved plan uses an approximate 20 px desktop
row target for compact Library results; it is an inspection target, not a fixed assertion.

`DataSheetQuery` carries optional sort and filter bindings; it does not describe paging mode. The
query owner applies collection-wide operations before handing a page to `DataSheetBlock`. A page
must not sort or filter only the visible server page.

## Two request paths

### Server-paged search

Use a focused `SearchDefinition` with `createSearchState` for server-owned paging. The definition
owns the query shape, initial query, cleanup, eligibility, text mapping, API request, row ID,
record content, applied chips, and selection maximum. Its `fetchPage` returns
items, the resolved page number and size, total count, and facet counts for that exact query.
Decode API data at the transport boundary. The Library definition is in
`features/question_library/question_library_search_definition.ts`; the Blueprint definition is in
`features/blueprint_course/blueprint_course_search_definition.ts`.

`createSearchState` is the sole reactive controller and lifecycle authority. It owns the query,
numbered page, request ordering, page size, selection by stable row ID, retry, and reset. The
`search_definition.ts` module contains shared search definitions and types; it has no competing
session/controller.
An eligible replacement query starts at page one
and clears old rows, counts, and selection. An ineligible query stays in the initial state.
Paging and refresh retain the visible verified page while loading; errors retain that page and
offer retry of the failed request. A late response cannot replace a newer result. The controller
disposes with the view and exposes explicit submit, apply, open, clear, page navigation, refresh,
and selection operations.

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
returns `{ state, collection: { fields, query } }`, where `state` is the standard
`SearchStateController` and `collection` is the binding consumed by `DataSheetBlock`. Declare
fields once in the helper options. The caller supplies stable ID, keyword text, content, and any
fixed domain scope or special comparator.

The adapter projects source loading and error over the controller's retained rows and counts.
Retry refetches a failed source when `refetchSource` is supplied; otherwise it retries through
the controller. The first delivered source opens the initial query. Later source updates refresh the
current query, remove selected IDs missing from the full authorized source, and step back when
a deletion leaves the last page empty. Query, filter, sort, and page changes use the current
source; explicit refresh calls `refetchSource` when supplied, otherwise it refreshes the session.
The page owns mutation-driven source updates or refetches. Consumers such as My
Drafts and Course collections keep their own data lifecycle and domain actions.

## SearchBlock and controls

Current `SearchBlock` requires `layout`, supplied controls, a state controller, and a collection with
an accessible label, empty state, fields, and `DataSheetQuery` binding. The collection may also
supply an initial empty state, row body, selection, and a disabled state for a content action. The block
has no alternate results slot. SearchBlock adapts loading, error, initial-empty, and numbered
pagination state into the collection props; DataSheetBlock mounts DataSheetControls before its
record region. `SearchControls` renders keyword submission, Clear all, applied chips, and content
controls. DataSheetTable owns column mechanics and contains HeaderRow. The query owner applies
whole-result sorting/filtering before pagination:
PostgreSQL `WHERE` and `ORDER BY` for database-backed collections, equivalent shared processing for
complete local sources. A page with another local text field can use `onSubmit` to compose it into
one query; `hasDraft` and `onClear` concern that control's
local draft only.

`createQuestionLibraryCollection` assembles the Library definition, controller, fields, and query
binding as `{ state, collection: { fields, query } }`. LibraryWorkspace, the Assessment content picker,
and QuestionPoolAvailableQuestions use this same assembly while retaining their own scope,
eligibility, permissions, actions, edit guards, and classification callbacks. Starred Questions
keeps its separate ordering adapter.

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
3. Declare fields and the `DataSheetQuery` once in the search binding. Mount `SearchControls` and
   `SearchBlock` with an explicit `above` or `beside` layout; `DataSheetBlock` owns controls,
   presentation, and numbered pagination.
4. Keep dialogs, workflows, and dirty checks with their owning page or editor. Search snapshots
   stay unchanged after cross-tab edits until a new query or explicit refresh. Protect only
   meaningful unsaved work before an operation removes it.
5. Verify the actual consumer through its owner checks and browser or rendered evidence as
   needed. Put one-time proof in `tests/_temp/`; retain a permanent test only when its stable
   assertion earns its fixture cost under [PYTEST_STYLE.md](PYTEST_STYLE.md). Record acceptance in
   the completion ledger.
