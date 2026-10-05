# Human Guidance search and navigation compliance audit — 2026-10-04

## Scope and evidence

This compares the current Question Library and Public Blueprint Course search implementation with
the settled search and navigation requirements in [`HUMAN_GUIDANCE.md`](../../HUMAN_GUIDANCE.md).
It makes no production or generated-artifact changes.

Findings use **static** source evidence or **test** evidence run in this audit. A live browser
observation was unavailable; the reason is recorded under Validation.

## Priority findings

### P1 — HG-SRCH-01: list-result navigation replaces the search and restores old results

HG requires every list item, including a search result and an Assessment in an Assessment list,
to open in a new browser tab or window. Ribbon navigation stays in the current tab. It also says
to confirm before discarding an existing search, to leave empty search pages directly, and not to
store old search results for restoration ([HG:340-346](../../HUMAN_GUIDANCE.md#L340)). Public
Blueprint results have the same new-tab rule ([HG:537-538](../../HUMAN_GUIDANCE.md#L537-L538)),
as do Question list items ([HG:600-603](../../HUMAN_GUIDANCE.md#L600-L603)).

- **Static:** The shared `RecordActionControl` emits ordinary anchors without `target="_blank"`
  ([`record_list.tsx:185-197`](../../../src/components/record_list/record_list.tsx#L185-L197)).
- **Static:** Question results save a return token before ordinary same-tab navigation
  ([`library_browse_rows.tsx:99-129`](../../../src/pages/library_browse_rows.tsx#L99-L129)). Their
  return state retains query, loaded rows, aggregate facets, cursors, and scroll position
  ([`library_page_model.ts:189-301`](../../../src/pages/library_page_model.ts#L189-L301)).
- **Static:** Blueprint results do the same
  ([`blueprint_course_search_result.ts:35-45`](../../../src/pages/blueprint_course_search_result.ts#L35-L45),
  [`blueprint_course_search_page.tsx:273-310`](../../../src/pages/blueprint_course_search_page.tsx#L273-L310)),
  and persist a snapshot in `history.state` for reload/Back
  ([`blueprint_course_search_return_state.ts:56-118`](../../../src/pages/blueprint_course_search_return_state.ts#L56-L118)).
- **Test:** Current tests protect that restoration:
  [`test_blueprint_search_return.mjs:34-93`](../../../tests/test_blueprint_search_return.mjs#L34-L93),
  [`test_library_classification_search.mjs:427-483`](../../../tests/test_library_classification_search.mjs#L427-L483),
  and [`e2e_live_demo_question_library_browser.mjs:79-103`](../../../tests/playwright/e2e_live_demo_question_library_browser.mjs#L79-L103).

Question Library state is document-local and neither flow creates a database search record. They
still restore old search results, which conflicts with HG. Change list-result links to open a
separate tab/window, remove the Question and Blueprint return-snapshot flows and their tests, and
verify that the original search tab remains unchanged. Preserve current-tab Ribbon navigation.

### P1 — HG-SRCH-02: Public Blueprint sorting omits required choices

HG requires sorting by Stars, Watches, Adoptions, number of students having taken the course, and
most recent edit ([HG:532-534](../../HUMAN_GUIDANCE.md#L532-L534)). The current API type, UI, SQL,
and cursor logic provide only `name`, `adoptions`, and `students`:

- [`blueprint_course.ts:40-42`](../../../src/api/blueprint_course.ts#L40-L42)
- [`blueprint_course_search_page.tsx:412-422`](../../../src/pages/blueprint_course_search_page.tsx#L412-L422)
- [`blueprint_operations.sql:494-503`](../../../schemas/base_schema/50_functions/blueprint_operations.sql#L494-L503)
  and [`598-612`](../../../schemas/base_schema/50_functions/blueprint_operations.sql#L598-L612)
- [`search.rs:149-168`](../../../crates/learning-data-access/src/postgres/blueprint_course/search.rs#L149-L168)

Stars, Watches, and most-recent-edit are absent from the returned summary. Add those fields,
their sort keys, cursor forms, UI controls, and focused tests.

### P1 — HG-SRCH-03: the assembled Question search workflow lacks the required shared interface and modes

HG says both exploration and simple search begin in one spreadsheet-style interface; simple
search returns those same results; the interface supports compact, list, and movie-poster-style
boxes ([HG:586-603](../../HUMAN_GUIDANCE.md#L586-L603)). It also names Reddit's multiple display
modes and OER Commons' simple search/image-focused boxes as references
([HG:606-610](../../HUMAN_GUIDANCE.md#L606-L610)).

- **Static:** Question Library starts from a separate simple-search entry, then displays
  `RecordList` rows after input
  ([`library_page.tsx:425-717`](../../../src/pages/library_page.tsx#L425-L717),
  [`library_browse_rows.tsx:227-264`](../../../src/pages/library_browse_rows.tsx#L227-L264)).
- **Static:** The currently assembled workflow has no compact/list/movie-poster mode selection.
  `RecordListImageBrowser` offers only Gallery and List, and is not wired to Question search
  ([`record_list_image_browser.tsx:22-69`](../../../src/components/record_list/record_list_image_browser.tsx#L22-L69)).

The simple input feeds the same page-level query state and exposes filters after search, so
ordinary-word search and later refinement are partly present
([`library_page.tsx:108-161`](../../../src/pages/library_page.tsx#L108-L161)). This finding is
about the missing assembled user workflow and display modes; HG does not require a particular
component or backend shape.

### P1 — HG-SRCH-04: Ribbon navigation silently discards a nonempty search

HG requires confirmation before navigation discards an existing search and its results, while an
empty search page may leave directly ([HG:342-346](../../HUMAN_GUIDANCE.md#L342-L346)).

- **Static:** Neither search page installs `UnsavedChangesGuard`; Public Blueprint cleanup only
  cancels requests and frames
  ([`blueprint_course_search_page.tsx:312-334`](../../../src/pages/blueprint_course_search_page.tsx#L312-L334)).
- **Static:** `UnsavedChangesGuard` has route and unload interception, but this audit found no
  search-page use ([`unsaved_changes_guard.tsx:33-145`](../../../src/components/unsaved_changes_guard.tsx#L33-L145)).

Ribbon navigation already remains current-tab. Add a confirmation only when a search or its
results exist; bypass it for the empty initial page.

### P1 — HG-SRCH-05: Public Blueprint result metadata is incomplete

HG requires Course name, classification, author, institution, and useful usage or stewardship
signals directly in the result list ([HG:529-531](../../HUMAN_GUIDANCE.md#L529-L531)). The returned
summary has no institution, Stars, or Watches
([`blueprint_course.rs:155-180`](../../../crates/question_model/src/blueprint_course.rs#L155-L180)).
The implementation explicitly keeps Institution out
([`blueprint_course_search_result.ts:10-45`](../../../src/pages/blueprint_course_search_result.ts#L10-L45)),
and its client test requires no Institution row
([`test_blueprint_course_client.mjs:31-55`](../../../tests/test_blueprint_course_client.mjs#L31-L55)).

The owner display name is correctly supplied and shown as Author. Add the required Institution
field and useful stewardship signals to the summary, list row, and focused test.

## Confirmed current student count; remaining edge cases are not settled

HG calls for an aggregate count, rather than identifiable Student records
([HG:532-534](../../HUMAN_GUIDANCE.md#L532-L534)). The current `total_students_ever_enrolled`
counts historical Student memberships per adopted Course Instance
([`blueprint_course.rs:175-180`](../../../crates/question_model/src/blueprint_course.rs#L175-L180));
the SQL returns an aggregate count, not Student records
([`blueprint_operations.sql:548-566`](../../../schemas/base_schema/50_functions/blueprint_operations.sql#L548-L566)).

This implements the settled aggregate and privacy boundary. Exact treatment of withdrawals and
repeated enrollment is not settled by HG, so this audit records no compliance gap or completion
model requirement.

## Verified alignment

- **Static:** Public Blueprint search has text, promoted, hierarchy, cross-discipline, and Tag
  filters. Its submitted search/filter/sort summary remains visible
  ([`blueprint_course_search_page.tsx:343-422`](../../../src/pages/blueprint_course_search_page.tsx#L343-L422)).
- **Static:** Question Library uses one query state for ordinary-word search, filters, sorting,
  and bulk work ([`library_page.tsx:108-161`](../../../src/pages/library_page.tsx#L108-L161)).
- **Static:** No search-scoped hover-preview implementation was found. The Question Open action
  has a short native title; keyboard-focus tooltip behavior was not verified
  ([`library_browse_rows.tsx:107-127`](../../../src/pages/library_browse_rows.tsx#L107-L127)).
- **Static:** Blueprint student counts are numeric aggregates, with no Student records returned
  ([`blueprint_operations.sql:548-566`](../../../schemas/base_schema/50_functions/blueprint_operations.sql#L548-L566)).

## Validation

Passed:

```text
source source_me.sh && node --import tsx --test \
  tests/test_blueprint_search_return.mjs \
  tests/test_blueprint_course_client.mjs \
  tests/test_library_classification_search.mjs
# 34 passed

source source_me.sh && node --import tsx --test tests/test_blueprint_course_ui.mjs
# 2 passed
```

The passing tests include result restoration, which conflicts with HG-SRCH-01. No live browser
observation was possible. `local_stack_state/live_demo_browser/developer-result.json` records a
completed prior supervisor, but the required
`local_stack_state/live_demo_browser/developer-control.json` receipt is absent; its
`supervisor.log` records a prior image-build launch rather than a usable current browser target.
This audit did not start or rebuild the expensive local stack.

## Recommended implementation order

1. Correct list-result navigation and remove result restoration; add browser proof that the
   original search tab stays unchanged.
2. Add missing Public Blueprint summary metadata and sort choices end to end.
3. Build the shared spreadsheet-style Question workflow with its three display modes.
4. Add confirmation before Ribbon navigation discards a nonempty search.
