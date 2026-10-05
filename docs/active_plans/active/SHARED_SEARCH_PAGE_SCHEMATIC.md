# Plan: Shared search page schematic

## Context

Status: shared search implementation complete through M16. Milestone gates and current evidence
are tracked in [SHARED_SEARCH_IMPLEMENTATION.md](../reports/SHARED_SEARCH_IMPLEMENTATION.md).
M1-M5 are verified: shared session, controls, responsive page layout, and leave-warning rules
serve Library Search, Library Browse, and Blueprint search, with live and rendered evidence.
M6 Pool ownership is verified across creation and fork paths with connected PostgreSQL evidence.
M7 removes separate Pool Author storage and attribution; owner and source-Pool links remain.
M8 enforces one Question per Pool and one immutable Type/Backend pair, verified through the
rebuilt Live Demo, connected mutation/fork tests, and an independent audit.
M9 calculates the required Pool license from exact member Revisions, removes manual provenance,
and preserves member licenses, with full backend gates and independent review.
M10 verifies the combined Library API, global paging, kind lookup, filter rules, and speed on a
rebuilt Live Demo. M11 verifies shared Question/Pool detail links and both lookup/detail retries.
M12 verifies mixed Library defaults, filters, displays, selection, live journeys, and speed.
M13 verifies the shared Question picker and Pool eligibility. M14 verifies unified Assessment
content selection and real Blueprint/Course import journeys. M15 accessibility, usability,
visual, and speed reviews pass, including the Compact description correction. M16 full checks,
real-service acceptance, screenshot publication/verification, and independent review passed
before the subsequent test pruning and screenshot-scope correction. Focused follow-up gates
passed; the aggregate component gates subsequently passed in separate invocations. The
[independent drift audit](../reports/shared_search_drift_audit_2026_10_05.md) identified Pool
owner-role, obsolete discovery, and shared fixture issues. Follow-up corrections are implemented
and validated. The user deferred the broader Library Object result-contract
redesign; this task remains focused on shared search and its screenshots.
The separate Pool validation handoff remains open: mismatch warnings, release blocking,
classification re-checks, unordered member storage, and the sortable member editor.

The September 25 work remains in place: shared record displays, page layout, sorting and paging
controls, and database queries that return one page of results. At this plan's start, individual
search pages assembled their filters and managed requests separately. The verified shared-page
milestones above now replace that duplication for Library and Public Blueprint search.

This proposal extends that work. Product behavior follows
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). The earlier implementation is recorded in
[record_list_page_frame_standardization_ledger.md](../workstreams/record_list_page_frame_standardization_ledger.md).

Committed HG already describes the Library as one global collection of Published Questions
and Question Pools (statement present since September 16, commit `eecc03ac`). Combined Library
search is existing scope. Earlier wording here that allowed independent Question and Pool
searches failed to carry that requirement through; the combined-query work below corrects it.

## Objectives

- Build one combined Published Question and Question Pool search with result-kind and
  Pool-membership filters. Reuse the same page components for Blueprint Course search.
- Let each content type supply its own fields, supported filters, sorting choices, and actions.
- Show the same search through compact spreadsheet rows, expanded lists, or visual boxes.
- Keep database searches bounded and search state temporary.

## Design philosophy

Apply **KISS** and **Design for adaptability**: share repeated interaction and presentation,
while keeping content rules with the code that already understands them. A page describes what
can be searched and displayed; shared code handles how the search works on screen.

Prove shared component reuse with Question Library and Blueprint Course search. Complete the
combined Question-and-Pool query before declaring Question Library search complete.
If the second page needs its own copy of the search controls or request handling, revise the
shared design. Differences in fields and server queries are expected.

## Scope

- Extract shared search controls, request handling, and result-page composition.
- Reuse existing record, table, classification, sorting, paging, and leave-confirmation components.
- Extend the Question Library API/query to combine Questions and Pools before sorting and paging.
- Reuse existing content-specific queries and the separate Blueprint Course API where appropriate.
- Keep domain actions, such as adding a Question or forking a Pool, with their existing owners.

## Non-goals

- Rebuild the database or combine all searches into one generic SQL engine.
- Add spreadsheet formulas, arbitrary cell editing, or a general page-building system.
- Add messaging features or change the decision about Stars and Watches.
- Store search terms or results in the database, browser storage, or a restoration service.

## Schematic

Two search definitions feed the same browser component: the combined Question Library and
Blueprint Courses. Proposed component names are working names; this does not require a new service.

```text
 Question Library definition          Blueprint Course definition
 (Published Questions + Pools)                    |
                |                                 |
                +--------------+------------------+
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
 Show: [Questions + Pools]  [Include Pool members: off]
 Filters: [Discipline] [Subject] [More filters]
 Active:  Biology [x]  Genetics [x]

 Display: [Compact] [List] [Visual boxes]   Sort: [Title]

 Title                 Owner             Authors          Subject
 [available filters use the field shown in each column]
 ----------------------------------------------------------------
 Question result       Instructor name   Author names     Genetics
 Pool result           Instructor name   --               Genetics

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
| Search definition | Field labels, value display, available filters and sorts, stable result IDs, destinations, and API connection | Questions and Pools expose Type and Backend; Pool text and metadata filters use the Pool's own values |
| Shared search page | Search form, active filters, display choice, result composition, and paging controls | Library and Blueprint searches use the same clear-filter interaction |
| Shared search state | Draft input, applied query, current page, selection, loading, errors, and request ordering | A late response from an older query cannot replace newer results |
| Existing display components | Table cells, record details, images, actions, keyboard access, and responsive layout | A result uses the same title and destination in every display mode |
| Content-specific API code | Convert the typed query to its existing request and validate the returned data | The Library request includes result-kind and Pool-membership filters |
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

## Pool decisions for implementation

The interview decisions below refine this schematic; enforcement is not yet verified.
The current decision record is
[HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md](../decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md).

| Property | Pool rule |
| --- | --- |
| Owner and source | Pool owner and source-Pool link; no Pool Author field. Members keep their own owners and authors |
| Membership | Unordered set of distinct Published Questions pinned to exact Revisions; selection is random |
| Question Type and Backend | Every member shares one Type and one Backend |
| Discipline and Subject | All members match the Pool's values |
| Pool mismatch | Pool no longer satisfies current requirements; show specific causes and block affected Assessment release until resolved; already-released Assessments continue as-is |
| Selection count | Preserve enough members for the Assessment selection count, including during member removal |
| Topic/Subtopic, Tags, Bloom dimensions | The Pool has its own values; members keep theirs and may differ |
| Filtering those metadata fields | Match Pool values only, never member values |
| Text search | Search Pool text and metadata only; do not match through member Questions |
| License | Automatically calculate one compatible license from members; reject incompatible combinations; retain original member licenses |
| Current license scope | CC0, CC BY, and CC BY-SA; NC and ND content are deferred |
| Forking | Pools are intended to be forked frequently; member Question ownership is preserved |

Required fields, including Question Type, must not become nullable merely to combine result kinds.
Pools have a common Type and Backend from their members. Bloom may remain NULL awaiting AI
assignment, with no enforced deadline; initial AI assignment remains deferred. Keep Topic/Subtopic
and optional support content optional. See
[QUESTION_MODEL.md](../../QUESTION_MODEL.md#required-and-assigned-metadata).

The metadata and combined-search decisions are settled for this work. Neil emphasized that
combining Published Questions and Pools was the original reason for the work. Result-kind and
Pool-membership filters operate on that combined search. Neil's current preferred default is
Questions in no Pool plus Pools; including individual Pool members remains available. Preserve
his "probably" qualifier for the default, rather than recording it as an inflexible decision.
The schematic controls illustrate that behavior; final control labels are not fixed.

"Questions in no Pool" means no membership in any Pool, not merely no membership in Pools that
match the current query. Pool-only text and metadata matching remains in effect: a term found
only in a member does not make its Pool match. Include member Questions to find those individual
matches. This is an explicit search tradeoff, not a guarantee that every member-only match appears
under the preferred default. In a Pool member picker, existing membership in another Pool does
not make a Question ineligible; the Library display default is not a membership constraint.
He rejected the proposed Pool ranking preference as bikeshedding; do not add it.
Validate the new Type/Backend membership requirements before relying on them as guaranteed
Pool properties. Keep the current metadata fields and allowed filters visible in each definition;
API limitations are implementation gaps, not permission to discard settled requirements.

## Combined results and pickers

The Library server query applies filters and sorting to the combined matching Questions and
Pools before taking a page. The browser must not concatenate two independently paged lists.
Use the result kind and public ID together for row identity and selection. Paging must remain
stable across tied sort values and must retain the chosen result-kind and Pool-membership filters.
A field missing on one result kind must not be filled with a different concept: Pool owner is
not Pool author, and member authors are not Pool authors.

Existing HG settles the main picker boundaries:

- Adding members to a Pool offers Published Questions only; Pools cannot contain Pools.
  Restrict candidates to the Pool's Discipline, Subject, Type, Backend, and compatible licenses.
- Adding Assessment content can offer Published Questions and Pools. Selecting a Pool invokes
  the existing Assessment-owned fork operation; it does not attach a shared mutable source Pool.
- A Pool remains an independently reusable Library object. A fork retains its own Pool identity
  and source link, while the member Questions retain their identities, owners, and authors.

No new selection workflow is implied by these rules. Preserve existing confirmation and editing
behavior while reusing the shared search controls.

### Pool validation handoff

Pools enter the Library immediately. Use that existing lifecycle; an unpublished-Pool category
has not been approved. Assessment forks remain Pools, with independent membership and source links.

Membership is an unordered set. The Pool editor presents member Questions with spreadsheet-style
sorting. Changing the display sort does not change membership, advance the membership Edit Number,
or change random selection. Enforce uniqueness by Published Question identity while retaining each
member's exact Revision. Review existing member-position storage and ordering controls against this
distinction; internal or display ordering does not establish an ordered Pool membership model.

Neil rejected overlap between separate Assessment entries as a reason for new restrictions.
An Instructor may add a Question directly even when a Pool in that Assessment also contains it.
Do not add Assessment-wide overlap bans or automatic deduplication from this interview.

The selection-count rule is settled. Locate the checks in the trusted membership-edit and
Assessment paths so removing members cannot leave an issued Assessment with too few candidates.
Choose the check locations during implementation; this is not another product decision.

Use **Pool mismatch** for a Pool that no longer satisfies its current requirements. Report the
specific causes: too few members, Discipline/Subject mismatch, duplicate Questions, or another
existing Pool constraint violation. This replaces the provisional "stale" label. For insufficient
members, evaluate the requirement against the affected Assessment's selection count; that count
is not a global minimum for every use of the Pool. Retain trusted checks that prevent invalid
writes; the mismatch state is not permission to save duplicate members or unusable selections.
A later Discipline/Subject mismatch must flag the affected Pool and block release until resolved.
Current admission-only validation is insufficient for this rule.
Implement the block through the existing Assessment release flow; do not invent a separate Pool
publication workflow. This decision does not authorize replacing Questions in existing Attempts
or silently removing incompatible members. If a mismatch develops after an Assessment is already
released, allow that Assessment to continue as-is. Preserve the existing post-issue limits and
checks against destructive membership edits; this exception does not authorize creating too few
members for future Attempts.

## Approach

Implementation is future work. Use this serial sequence so the shared contract is tested
before all callers depend on it; no parallel implementation is needed for the initial extraction.

| Step | Owner and files | Depends on | Done when |
| --- | --- | --- | --- |
| S1: Extract shared search | One frontend owner; proposed `src/features/search/`, existing Question Library page/model and browse controls | None | Question search uses the shared page and retains its fields, selection, actions, and behavior |
| S2: Prove reuse | Same shared-component owner; Blueprint Course search page and its API connection | S1 | Blueprint search uses the same form, state handling, and display controls without copying them |
| S3: Combine Library queries | One API/database owner; Library search requests, result contracts, database search, and Pool membership fields | S2 | Result-kind and Pool-membership filters operate before one global sort and page; Pool fields meet the settled rules |
| S4: Connect combined results | One frontend owner; Library page, Pool result display, and picker connections | S3 | One Library search shows both result kinds in each mode; navigation and content actions retain their behavior |
| S5: Close out | Integration owner; affected tests, reports, checklist evidence, and changelog | S4 | Duplicate search machinery is removed and integrated browser checks pass |

Each step ends with its focused behavior check before the next begins. Existing API and database
owners keep their current contracts unless a demonstrated search requirement needs a specific
addition. S3 must resolve the missing Pool-owner identity and new membership enforcement in
the data model; do not invent an owner from author or attestation fields. Stars/Watches scope
is separate from this search extraction.

## Verification

- Reuse existing tests for search, filters, sorting, paging, selection, and navigation. Add
  permanent coverage only for a shared behavior that is not already protected.
- Check a mixed Question/Pool result set across page boundaries for complete, stable ordering.
  A member-only metadata or text match must not cause its Pool to match.
- Check one delayed-request case: an older response must not overwrite the latest search.
- Check that a later database page can contain the first result after changing sort; browser
  sorting of only the loaded page fails this check.
- In the running application, check Library result-kind and Pool-membership filters and Blueprint search,
  supported display modes, simple-search
  entry, result links, leave confirmation, keyboard use, and a narrow viewport.
- Confirm that changing display mode sends no new search request and creates no saved-search
  record or browser-storage entry. Use one-time inspection for this implementation evidence.
- Run the repository's fast and complete checks, then capture the affected UI with the existing
  screenshot workflow. Mark only behavior demonstrated by those checks as complete.

This schematic is complete when responsibilities and the reuse path are clear. Implementation
completion requires the behavior checks above; the existence of shared components alone is
not sufficient evidence.
