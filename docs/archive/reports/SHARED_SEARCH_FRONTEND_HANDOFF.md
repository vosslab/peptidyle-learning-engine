# Shared search frontend handoff: M11 and M12

Historical preparation for the now-verified M11-M12 implementation. Source paths and proposed
changes below describe the starting seams. Current behavior is documented in
[the architecture guide](../../SEARCH_PAGE_ARCHITECTURE.md); completed gates are in
[the implementation ledger](SHARED_SEARCH_IMPLEMENTATION.md).

Prepared from the current frontend on 2026-10-05. This is an implementation map, not a new API design. M10 owns the combined-query row and kind-lookup contracts; update the frontend only after those generated/client boundaries are available.

## Current route and detail seam (M11)

`/library/:questionId` is already the single public-object route in the route contract. Its route ID is still `questionDetail`, its surface says `Published question detail`, and `routes.ts` mounts `QuestionDetailPage`. `route_params.ts` validates the `questionId` with the existing canonical public-ID parser, so the route parameter can stay unchanged when it becomes a mixed Library-object lookup.

`QuestionDetailPage` currently performs the Question-only resolution at `src/pages/question_detail_page.tsx:447-461`: it validates `params.questionId`, then calls `resolveQuestion` and `queries.questionDetails`. The M11 replacement seam is a small `LibraryObjectDetailPage` route component that owns the existing parameter validation, calls M10's kind lookup, and dispatches to the existing Question detail presentation or a new Pool detail component. Keep `QuestionDetailPage` as the Question branch until the wrapper is in place; do not make the route parameter encode kind.

The existing Pool detail presentation is embedded in `LibraryPoolDiscovery`:

- Local detail request and error handling: `library_pool_discovery.tsx:347-356`.
- Detail content and member list: `:659-756`.
- Instructor-only Pool support/Bloom controls: `:702-735`.
- Instructor-only Star and Watch controls: `:736-739`.

Move the display component from those lines into `question_pool_detail.tsx`; give it the existing Pool client, classification/support capabilities, and role booleans it already receives. The new route wrapper should own loading/error/retry for its public ID. Pool detail keeps its current `getQuestionPool` read and does not need a second generic detail abstraction.

The route composition currently supplies Pool APIs through `LibraryRoutePage` into `LibraryPage`. M11 needs a route-level composition for the new detail wrapper from `useApplicationApi()` instead; that lets M12 delete the Pool-only optional props from `LibraryPage`.

Pool Watch inbox links are the second direct route consumer. `targetHref()` in `library_watch_notifications_page.tsx:33-38` sends Questions to `/library/{id}` but sends Pools to `/library?pool={id}`. Switch the Pool branch to the same canonical detail path once M11 accepts either kind.

## Current Pool-panel removal boundary (M12)

The old discovery surface is entirely rooted in `LibraryPage`:

- Pool client and capability imports/optional prop: `library_page.tsx:8-16, 45-55`.
- `?pool=` deep-link state: `:68, 87` via `hasCanonicalPoolDeepLink()` in `library_page_helpers.tsx:41-50`.
- `<details>` wrapper and lazy `LibraryPoolDiscovery` mount: `:210-230`.

After M11 routes Pool detail, remove this block, `poolLibraryClient` from `LibraryPageProps`, and its route-composition arguments. Delete `library_pool_discovery.tsx`, then remove the obsolete `hasCanonicalPoolDeepLink` helper and `?pool=` handling. The remaining Pool creation dialog is independent and stays in the `SearchPage.extra` slot.

`library_pool_discovery.tsx` has three responsibilities that must not be dropped with the panel:

1. **Pool detail** moves to `question_pool_detail.tsx` as described above.
2. **Pool-owned bulk metadata and Bloom correction** are separate from Question bulk editing. M12 must decide their explicit destination using the M10 mixed-row contract; do not pass Pool IDs through the current Question bulk controller.
3. **Pool list/filter/paging state** is replaced by the M10 combined Library query. Its custom Pool-only form, Bloom facet report, and pagination do not migrate independently.

## Mixed-row seams after M10 (M12)

The existing Library model and definition are Question-only:

- `QuestionLibraryBrowseRow` carries a mandatory Question revision tuple, authors, Question format, evidence, and Question license (`library_page_model.ts:38-60`).
- `questionLibraryContent()` assumes every row is a Published Question, builds Question facts, and links using `questionLink()` (`question_library_search_definition.ts:21-65`).
- The repository maps only `searchQuestionLibrary()` Question summaries in `question_library_repository.ts`.

Use the M10 returned discriminant directly in the Library row union. The shared `SearchDefinition` only needs stable `rowId()` and `content(row)`, so the Library definition is the correct place for a discriminated `Question`/`Question Pool` record-content branch. Both branches should link to `/library/{publicId}`. Pool content should provide the M12-required Pool label, owner, member count, Type, Backend, license, classification, Tags, and assigned Bloom using the fields M10 exposes. Keep this domain rendering in `question_library_search_definition.ts`; shared search components remain domain-free.

The query boundary is `QuestionLibraryBrowseQuery` and its normalization/strict page decoder in `library_page_model.ts`, then `questionSearchRequest()` in `question_library_repository.ts`. Add M10's kind, membership, and owner fields consistently in all four places:

- initial query/default;
- URL parse, recovery, and serialization in `library_search_parameters.ts`;
- normalization and request adaptation;
- applied-filter labels and clear hierarchy in `question_library_search_definition.ts`.

`LibrarySearchFilters` owns the scalar/filter JSX and should receive the new Show and Pool-membership controls there. It is already wired only through `changeQuery()` in `LibraryPage`; when Pools-only is selected, clear Question-only filters in that change path or filter callback before applying the query. The shared controls stay unchanged.

The Browse exact-filter policy is deliberately Library-local in `LibraryPage.displayedRows()` (`library_page.tsx:129-132`). Revisit its predicate only if M12 changes which filters are exact; retain the row override so a broad automatic Browse request does not make hidden rows selectable.

## Current bulk-action limitation

`LibraryBulkActions` is explicitly Question-only:

- `questionLibraryBulkSelectionRequest()` validates `PublishedQuestionId` values and says its command is Question-only (`question_library_repository.ts:63-101`).
- It fetches `getCurrentQuestionSharedMetadata(questionIds)` and renders `QuestionBulkMetadataEditor` (`library_bulk_actions.tsx:124-145, 214-224`).
- Its title cache and selection method assume `row.questionTitle` and every selected row can be a Question (`:67-77, 90-102`).

For M12, preserve one shared selection maximum of 1000 at the `SearchState` level, then partition selected mixed rows by kind in the Library bulk controller. Continue the existing Question editor only for selected Question rows. Route selected Pool rows to the already-existing Pool metadata editor/client path from the deleted discovery panel, with Pool-specific metadata restrictions preserved. Do not widen Question bulk API types to accept Pool IDs.

## URL and direct-link inventory

Current Question Library URL keys parsed by `searchHandoffQuery()` are classification hierarchy, `search`, repeated `subjects`/`topics`, Bloom dimensions, `authorName`, `backend`, `tag`, `questionType`, `capability`, `questionLicense`, `usedInMyCourses`, and `sort` (`library_search_parameters.ts`). M12 adds M10's kind, membership, and owner keys there, in `recoverLibrarySearch()`, and in `searchWithinResultsPath()`.

`?pool=` is currently only a Pool-panel deep link: parsed by `hasCanonicalPoolDeepLink`, mounted by the Library `<details>`, and emitted by Watch inbox links. Remove it rather than translating it silently; canonical Pool links become `/library/{id}`.

## Existing test points

- Route contract: `tests/test_frontend_contract.mjs` already asserts `/library/{id}` matches `questionDetail`; update its M11 source assertions from `library_pool_discovery.tsx` to the new Pool detail file.
- Watch client/page: `tests/test_library_watch_notification_client.mjs` covers the mixed notification payload; add or update the page-link assertion for a Pool target.
- After the user-requested test review, the existing live Library browser journey owns default membership, Pools-only, display behavior, and Pool-link navigation. The separate shared-search fixture suite was removed as duplicate implementation proof.
- Add the planned temporary Playwright check for direct `/library/{questionId}` and `/library/{poolId}` rendering during M11; it proves the runtime routing and kind dispatch, not only the route table.
- The panel and its standalone discovery client are removed. Combined Library search owns Pool
  discovery; the old discovery-client tests are deleted. Pool detail and mutation clients remain.

## Order of work

1. Wait for M10 generated/client contract and inspect its actual discriminated row and kind-lookup shapes.
2. M11: add the route wrapper, extract Pool detail intact, redirect Watch links, and prove direct Question and Pool URLs.
3. M12: move the combined row/query contract through Library model/repository/definition/filters, then partition bulk actions by kind.
4. Delete the Pool panel and `?pool=` path only after the combined results and direct Pool route work.
