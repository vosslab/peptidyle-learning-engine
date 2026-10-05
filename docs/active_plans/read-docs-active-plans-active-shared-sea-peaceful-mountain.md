# Plan: reusable shared search page

## Current status

M1-M16 below record the original implementation sequence and historical acceptance. The
October 5 drift corrections passed the Rust, frontend, UI, Python, live Library, and connected
acceptance gates. The renamed screenshot workflows passed a full replay, and all 256 captures were then published
from a clean Demo after the naming review. The user deferred the broader Library Object design
and kept this task focused on the shared search interface and corresponding screenshots.
See [the audit and corrections](reports/shared_search_drift_audit_2026_10_05.md).

## Test-policy update after implementation

The October 5 user review supersedes the original permanent-test inventory below. Retain shared
session regressions, the existing real Library browser journey, and focused connected domain
invariants. The separate shared-search browser suite, accessibility harness, consumer matrices,
and repeated API/filter assertions were one-time implementation evidence and are removed.
Apply all six questions in [PYTEST_STYLE.md](../PYTEST_STYLE.md) before adding a permanent case;
new consumers do not automatically require duplicated tests. Screenshot capture remains a
requested repository workflow. Historical milestone receipts remain evidence of work performed.

## Context

[SHARED_SEARCH_PAGE_SCHEMATIC.md](active/SHARED_SEARCH_PAGE_SCHEMATIC.md)
describes one search page that many kinds of content can use:

```text
search definition for one content type -> shared search page -> that content type's API
```

Today Question Library, Public Blueprint Course search, Pool discovery, and the two pickers
each have their own copy of the search form, request handling, paging, display switching, and
loading and error states. The schematic asks for one shared version, proven first by Question
Library and Public Blueprint Course search.

Question Library is the harder of the two. HG and
[QUESTION_MODEL.md](../QUESTION_MODEL.md) require one search over Published Questions
and Question Pools, with filters for result kind and Pool membership, and a preferred default
of Questions in no Pool plus Pools. That needs a combined server query and some Pool data
fixes. That work supports the Library; the shared search page is the main goal.

## Objectives

- Build one shared search page that works for different content types, filters, sorts,
  result fields, actions, and APIs, with one copy of the request and display code.
- Prove it with two users of the shared code, Question Library and Public Blueprint Course
  search, each supplying only its search definition. Write a guide so the next search can be
  added quickly.
- Offer Compact, List, and Visual boxes on every search, with the same title, fields, and
  link in each, passing automated accessibility and keyboard checks.
- Return Published Questions and Pools from one Library query that filters, sorts, and pages
  them together, with result-kind and Pool-membership filters and the preferred default.
- Store each Pool's owner, Question Type, Backend, and license, keep each Question in a Pool
  at most once, and check these rules on every membership change.
- Use the same search code in the Question picker and the Assessment content picker.
- Finish the whole plan with the manager and subagents only, using automated tests and
  independent agent reviews as gates.

## Design philosophy

- **Design for adaptability** and **Fix the design, not the symptom**: shared code controls
  how a search behaves on screen; each search definition says what can be searched, shown,
  and opened; the server handles access, filtering, sorting all matches, and returning one page.
- Build the shared parts from what both existing searches already do (see Shared code
  evidence). Keep a feature inside its own search until a second real search needs it.
- Keep result kinds and Pool membership inside Question Library. The shared code uses a
  unique ID and one display description per row, which already handles mixed rows.
- Combine Questions and Pools on the server, so one sort and one page cover every match.
- Keep it simple: a search definition is a small TypeScript module with types, functions, and
  slots for custom controls.
- Adopt Blueprint before building the page shell. When Blueprint needs something new, add it
  to the shared code and re-run the Library checks.
- Run narrow checks at small milestones and broad checks, live tests, and independent reviews
  at integration points, so checks support delivery instead of dominating it.

## Scope

- Move shared search behavior into `src/features/search/`: request handling, controls, active
  filters, results, paging, sorting, selection, three display modes, loading, empty, error and
  retry states, link and leave behavior, and use inside a page or a dialog.
- Move Question Library (Search and Browse) and Public Blueprint search onto it.
- Fix Pool data: add owner, remove Pool author names and the manual Pool license, store one
  Question Type and one Backend, calculate the license from members, allow each Question once
  per Pool, enforce these rules, and repair content that breaks them.
- Extend the Library server query to return Questions and Pools together, with result-kind,
  Pool-membership, and Owner filters, and add an endpoint that tells whether an ID is a
  Question or a Pool.
- Show Pools at `/library/{id}`; remove the Pool discovery panel embedded in the Library page.
- Move the Question picker and the Assessment content picker onto the shared code.
- Run independent usability, accessibility, visual, and code reviews; fix their serious findings.
- Write `docs/SEARCH_PAGE_ARCHITECTURE.md`; update tests, screenshots, and docs.

## Non-goals

- Leave the schematic's "Pool validation handoff" (mismatch warnings, release blocking,
  classification re-checks, unordered member storage, sortable Pool member editor) to its own
  plan.
- Keep each search on its own server API.
- Leave images in Visual boxes, NC and ND licenses, AI Bloom assignment, and Stars and Watches
  to later work.

## Current state summary

Findings from the source code (2026-10-04):

- Shared pieces that already exist: `RecordList` (one `content(row)` drives compact, list, and
  poster layouts; links always open a new tab), `RecordPageControls`, `RecordSortControl`,
  `RecordCollectionStateView`, `SearchResultDisplay`, `SearchLeaveGuard`,
  `LibraryClassificationSearch`. Missing: shared request handling, controls, and page layout.
- Question Library: `src/pages/library_page.tsx` (775 lines), `library_page_model.ts` (682).
  Searches on every keystroke (`:370`). Its own request class
  `QuestionLibraryBrowseSession` (model:498-682). About 220 lines of copied filter dropdowns
  (`:417-641`). Unused scroll-window props (`:694-695`). `/library?search=foo` fills the box
  but does not search.
- Public Blueprint search: `src/pages/blueprint_course_search_page.tsx` (403). The leave
  warning stays on after Clear; it shows "0 shown" before any search. No UI tests.
- Pool discovery: `src/pages/library_pool_discovery.tsx` (759), inside a Library
  `<details>` panel, no URL of its own, no sort, its own loading and error markup, Pool details
  open in place. No UI tests. Lists every Pool row, including Assessment forks
  (`question_pools.sql:745-786`).
- Pickers: `src/features/question_picker/` (623 + 570 lines), `question_pool_picker.tsx` (328).
- Library server query (`question_library_operations.sql:158-430`): Questions and filter
  counts; pages by (title, id) or (published time, id); no owner column; no membership filter.
- Pool table (`20_tables/question_pool.sql`): has classification, Tags, Bloom, `created_at`,
  `source_question_pool_id`; no owner; still has `question_pool_authorship` and an optional
  manual license in `question_pool_provenance` (CC0-1.0, CC-BY-4.0, CC-BY-SA-4.0;
  `question_pools.sql:517`); no check that member licenses fit together; Backend checked only
  for production support. Members are unique per Question and Revision (`:73`), so two
  Revisions of one Question can share a Pool. Index on members by Question exists
  (`40_indexes.sql:450`).
- `published_question.backend` and `question_type` are `NOT NULL`; one `question_type` list
  is shared by every Backend (`10_types.sql:51`).
- Question and Pool IDs cannot collide: both reserve their ID in
  `ple_private.public_id_reservation`, whose primary key covers both
  (`20_tables/account.sql:8-15`, triggers at `question_pools.sql:5`, `question_lineages.sql:17`).
- Pools are created only through the signed-in creation and fork operations
  (`crates/server/src/question_pool_creation.rs`, fork functions in `question_pools.sql`).
  Test fixtures insert Pools directly
  (`crates/learning-data-access/tests/blueprint_course_postgres/`,
  `tests/e2e/assessment_saved_response/03_course_pool_forks.sql`).
- The HTTP client cannot cancel requests (`src/api/http_client/request.ts:23-27`).
- `@axe-core/playwright` is installed (`package.json:23`).

## Shared code evidence

What the existing searches already do, and where each feature will live.

| Feature | Question Library | Blueprint search | Pool discovery | Pickers | Where it lives |
| --- | --- | --- | --- | --- | --- |
| Text search | every keystroke | on submit | on submit | none | Shared (on submit) |
| Classification and other filters | Library controls + counts | Blueprint control | Library control | preset | Shared slot; each search draws its own |
| Sort | 2 server sorts | 6 server sorts | none | none | Shared, optional |
| Paging, 50/100/250 rows | yes | yes | yes | yes | Shared |
| Ignore late, outdated responses | yes | yes | yes | yes | Shared |
| Three display modes | yes | yes | none | none | Shared, every search |
| Loading, empty, error, retry | RecordList | RecordList | own markup | own | Shared |
| "Applied search" line | none | yes | yes | none | Shared |
| Results open in a new tab | yes | yes | in place | n/a | Shared (via `RecordList`) |
| Leave warning | yes | yes | from host page | n/a | Shared, pages only |
| Selection | checkboxes, max 1000 | none | checkboxes | radio or checkbox | Shared selection and bar; actions belong to each search |
| Filter counts | yes | none | Bloom counts | none | Shared clears them on a new search; each search draws them |
| Search when opened | Browse | no | yes | yes | Shared option |
| Starting values from the URL | yes | no | `?pool=` | no | Each search (passes a starting query) |
| Result kinds and membership | Questions + Pools | one | one | Assessment picker: both | Each search |

## Resolved decisions

Shared search page:

- **Text search runs on submit** (from the human-interact-expert review): Enter or Search runs
  it; an empty search shows everything. Filters, sort, and page size apply at once and return
  to page 1. Reasons: free text with quotes, minus, and `field:value` passes through broken
  half-typed states (Shneiderman 15.2.2 and 15.3); a server search per keystroke cannot answer
  within the 0.1 s users need to link cause and effect, and users need to see the terms they
  searched (*Designing with the Mind in Mind* ch. 12 and ch. 7); screen readers should hear one
  result count per search (WCAG 4.1.3). A typing delay (debounce) still sends half-typed queries.
- **Late responses**: each request gets a number; only the newest request's response is shown.
  A new search clears the old rows and the old filter counts, because counts depend on the
  query; selected filter values stay visible without counts. A failed Next or Previous keeps
  the current page and its counts.
- **Selection**: tracked by row ID; a new search, filter, or sort clears it; paging and page
  size keep it; each search sets its own maximum.
- **Display**: three modes on every search; switching redraws the current rows from page memory.
- **Search state**: lives in page memory for the open page and ends when the page closes.
- **Links and leaving**: results open in a new tab; the leave warning appears after a search
  has run or while text is typed; Clear turns it off.

Question Library (HG, `docs/QUESTION_MODEL.md`, schematic):

- **Result filters** (final labels not fixed):
  - Show: Both / Questions only / Pools only.
  - Pool membership, for Question rows: Questions in no Pool / All Questions (includes Pool
    members). Disabled when Pools only is chosen.
  - Default: Both with Questions in no Pool, which shows Questions in no Pool plus Pools.
    HG says "probably", so the default lives in one place (the Library definition's starting
    query) and the active default shows as a removable chip.
  - "In a Pool" means the Question (any Revision) is a member of any Pool row, including
    Assessment forks.
  - Pool rows match on Pool-own values; a match found in a member shows as that member's own
    row when All Questions is chosen.
  - Counts per filter choice: Questions in no Pool, Questions in a Pool, Pools.
- **Which Pools appear**: every Pool, including Assessment forks (today's behavior; HG says
  forks remain Pools and Pools enter the Library immediately).
- **How filters treat Pool rows**:

| Filter | Question rows | Pool rows |
| --- | --- | --- |
| Text and `field:value` | Question fields | Pool title, description, classification, Tags only |
| Discipline, Subject | Question values | Pool values |
| Topic, Subtopic, Tags, Bloom | Question values | The Pool's own values only |
| Question Type, Backend | Question values | The Pool's one Type and Backend |
| License | Question license | Calculated Pool license |
| Owner | Question owner | Pool owner |
| Author | Question authors | Applies to Questions only (Pools have an owner) |
| Used in my courses, capability, authored by me | Question values | Applies to Questions only |

- **Automatic metadata**: PLE fills in metadata where it can; publishing requires the rest of
  the required fields. Pool owner, Type, Backend, and license are required (`NOT NULL`) and
  filled in automatically. Bloom may stay empty while waiting for AI assignment; Bloom filters
  and counts use assigned values, and rows show Bloom once it is assigned. Optional fields,
  such as Pool Topic and Subtopic, keep their current rules.
- **Pool owner**: the account that created or forked the Pool (Course adoption fork: the
  Instructor adopting the course). Test fixtures set it directly.
- **Pool Type and Backend**: taken from the first member when the Pool is created, the same way
  Discipline and Subject are today; every later member must match.
- **One copy per Question**: a Pool holds each Published Question at most once (unique on Pool
  and Question), still pinned to one exact Revision.
- **Pool license table** (an independent reviewer checks it before M9 enforces it):

| Member licenses present | Pool license | Source |
| --- | --- | --- |
| CC0 only | CC0-1.0 | HG: one license that fits every member |
| CC BY, with or without CC0 | CC-BY-4.0 | Same HG rule; a collection of unchanged works |
| CC BY-SA, with any of CC0 or CC BY | CC-BY-SA-4.0 | Interview decision record: CC BY + CC BY-SA gives CC BY-SA |

  Every current combination yields a Pool license. The table can gain "rejected" rows when NC
  licenses return.
- **Paging mixed results**: sort by title or newest, then by public ID. Public IDs are unique
  across Questions and Pools (see Current state), so this order is complete and stable.
- **Pickers**: the Pool member picker uses All Questions and shows Published Questions that fit
  the Pool (same Discipline, Subject, Type, Backend, and a license that fits), including
  Questions already in other Pools. The Assessment picker shows both kinds with the Library
  default and the same membership filter; choosing a Pool runs the existing fork import. The
  Assessment picker replaces `QuestionPoolPicker`.

## Repairing content that breaks the new rules

The audit in M8 lists every Pool with mixed Types, mixed Backends, or two Revisions of one
Question, plus every Assessment entry that uses it (selection count, points per item).

- Test fixtures: fix each one on purpose so it still tests what it was written to test (for
  example, make the members share a Type, or drop the extra Revision). The milestone reviewer
  reads each fixture change against its test.
- Live Demo and teaching content: keep each affected Assessment's Questions per Attempt and
  total points the same, and keep each Pool focused on its original learning target. Duplicate
  Revisions: keep the Revision that Assessments currently deliver. Mixed Type or Backend: move
  the off-rule members into a new Pool with the same Discipline and Subject; if the original
  Pool then has too few members for an Assessment's selection count, split that Assessment
  entry into one entry per Pool with selection counts that add up to the original and the same
  points per item. List every moved or removed member and every changed entry in the audit
  report.
- A before-and-after query proves Questions per Attempt and total points stayed the same. An
  independent reviewer checks the report, the totals, and that each Pool still targets the same
  learning. Changes in selection chances caused by a split are noted in the report.

## How agents finish the plan

- The manager runs every check. Independent reviewer subagents review at integration points
  (end of M5, M10, M12, M14, M16) and at any milestone that changes `src/features/search/`
  after M4. Serious findings are fixed before moving on and noted under
  `### Developer Tests and Notes` in `docs/CHANGELOG.md`.
- Agents run the Live Demo with `local_stack.py` and screenshots with
  `./devel/capture_screenshots.sh`.
- An `image_evaluator` agent reviews screenshots. A new agent using `human-interact-expert`
  walks through the main tasks and lists problems by severity. Axe and keyboard tests check
  accessibility.
- When a product question comes up, check HG, then `docs/QUESTION_MODEL.md`, then the
  interview decision record, then this plan. If none answers it, the manager picks the
  simplest option that fits HG, records it in `docs/DESIGN_DECISIONS.md`, lists it in the
  final report, and continues.

## Architecture boundaries and ownership

| Part | Responsible for | Location |
| --- | --- | --- |
| Search definition type | Query type, cleanup, text get and set, `fetchPage`, row ID, `content(row)`, optional sort, applied-filter labels and clear actions | `src/features/search/search_session.ts` |
| Search session | Current query, page stack, page size, request numbering, retry, selection, clearing query-dependent data on a new search | `search_session.ts` (plain TypeScript) |
| Search state hook | Typed text, submit, filter update, sort, page size, paging, clear, display mode, touched flag, validation | `search_state.ts` |
| Search controls | Text box, applied-filter chips and Clear all, sort, display buttons, result count, filter slot | `search_controls.tsx` |
| Search results | `RecordList` in three modes, loading/empty/error states, page controls | `search_results.tsx` |
| Selection bar | Count, Select loaded, Clear, slot for actions | `search_selection_bar.tsx` |
| Search page | Page layout, toolbar and extra-content slots, leave warning | `search_page.tsx` |
| Search definitions | Content rules, filter controls, actions, API calls | `src/pages/question_library_search_definition.ts`, `blueprint_course_search_definition.ts`, picker definitions |
| Library extras | Filter list, kind and membership filters, bulk edit by kind, URL starting values, `/library/{id}` routing, Pool detail panel | `src/pages/library_search_filters.tsx`, `library_bulk_actions.tsx`, `library_search_parameters.ts`, `library_object_detail_page.tsx`, `question_pool_detail.tsx` |
| Library data and API | Pool owner, Type, Backend, license, one copy per Question, rule checks, combined query, kind lookup | SQL schema, `crates/learning-data-access`, `crates/server/src/question_library/`, `crates/server/src/question_pool_library.rs`, `generated/api` |

### Mapping (milestones -> components)

| Milestone | Part | What the reviewer checks |
| --- | --- | --- |
| M1 | Search session, definition type | Plain TypeScript behavior |
| M2 | Library uses the session; text on submit | Only text-submit behavior changed |
| M3 | State hook, controls, results, selection bar | Library Search on shared parts |
| M4 | Blueprint definition | Second search uses shared code only |
| M5 | Search page layout; Library Browse | Page layout shared by both searches (integration point) |
| M6 | Pool owner | Schema, operations, API |
| M7 | Pool author names removed | Schema, API, Pool editor |
| M8 | Pool membership rules | Audit, repair, one copy per Question, Type and Backend checks |
| M9 | Pool license | Table, calculation, manual license removed, checks |
| M10 | Library server query | Combined query, filters, kind lookup, speed (integration point) |
| M11 | Library links | `/library/{id}` routing, Pool detail page |
| M12 | Library shows both kinds | Library UI; old Pool panel removed (integration point) |
| M13 | Question picker | Dialog use; Pool member rules |
| M14 | Assessment content picker | Both kinds; fork import (integration point) |
| M15 | Independent reviews | Usability, accessibility, visuals, speed |
| M16 | Close-out | Docs, cleanup, full checks (integration point) |
| M17 (optional) | Third user | Works for a plain list |

### File scope

| File or directory | Change | Generated from |
| --- | --- | --- |
| `src/features/search/` (new) | Session, state, controls, results, selection bar, page | |
| `src/pages/library_page.tsx`, `library_page_model.ts`, `library_browse_controls.tsx` | Keep only Library wiring; remove the old request class | |
| `src/pages/library_browse_rows.tsx` | Delete | |
| `src/pages/question_library_search_definition.ts`, `library_search_filters.tsx`, `library_bulk_actions.tsx` (new) | Library definition, filter list, kind and membership filters, bulk editors | |
| `src/pages/library_search_parameters.ts` | Search runs for any URL starting value; adds kind and membership | |
| `src/pages/blueprint_course_search_page.tsx`, `blueprint_course_search_definition.ts` (new), `blueprint_course_search_return_state.ts` | Thin page, definition, move the snapshot type | |
| `src/pages/library_pool_discovery.tsx`, `library_page_helpers.tsx` | Delete the Pool panel and `?pool=` handling | |
| `src/pages/library_object_detail_page.tsx`, `question_pool_detail.tsx` (new), `src/routes.ts`, `src/route_contract.ts` | `/library/:questionId` shows a Question or a Pool | |
| `src/pages/library_watch_notifications_page.tsx` | Pool links go to `/library/{id}` | |
| `src/features/question_picker/`, `src/features/question_pool_picker/`, picker callers | Pickers on shared code; Pool picker removed | |
| `src/api/question_library_repository.ts`, `question_search_query.ts`, decoders | Kind, membership, owner, mixed rows, kind lookup | |
| `schemas/base_schema/20_tables/question_pool.sql`, `50_functions/question_pools.sql`, `question_library_operations.sql`, grants | Pool data, rule checks, combined query, kind lookup | |
| `crates/learning-data-access/src/**`, `crates/server/src/question_pool_*.rs`, `crates/server/src/question_library/` | Store and server code | |
| `generated/api/*.ts` | Regenerate | Rust ts-rs types |
| Schema tables doc | Regenerate | `devel/generate_schema_tables_doc.py` |
| Test fixtures and Live Demo content that create Pools or Pool entries | Set owner; repair rule breaks as described above | |
| `tests/test_search_session.mjs`, existing live Library browser journey | Focused permanent shared-search contracts | |
| One-time accessibility and keyboard checks | Review evidence; no permanent application harness | |
| Existing tests (`test_library_classification_search.mjs`, `test_frontend_contract.mjs`, `test_question_watch_client.mjs`, `test_question_picker.mjs`, `test_question_pool_source_binding.mjs`, `test_question_pool_discovery_client.mjs`, `test_blueprint_course_client.mjs`) | Update to the new code and paths | |
| `crates/server/src/question_library/tests.rs`, Pool store tests | Combined query rules, Pool rule checks | |
| `tests/playwright/e2e_live_demo_question_library_browser.mjs`, `screenshot_corpus/scenarios_*.ts` | Press Enter to search; add Blueprint, both-kind, membership, display-mode, picker scenes | |
| `docs/SEARCH_PAGE_ARCHITECTURE.md` (new) | Guide for adding a search | |

## Milestone plan

Narrow checks run at every milestone; broad checks run at integration points.

- Narrow frontend check: `source ./source_me.sh && ./check_codebase.sh` plus the touched node
  test files (`node --import tsx --test tests/<file>.mjs`).
- Narrow backend check: `source ./source_me.sh && devel/generate_schema_tables_doc.py &&
  schema_style/check_schema_style.py` when SQL changes, plus `cargo test` filtered to the
  touched crate and tests.
- Broad check G: `source ./source_me.sh && ./check_codebase.sh && ./launchers/run_fast_ui_checks.sh`.
- Broad check B: `source ./source_me.sh && ./check_rust.sh && ./check_codebase.sh` plus the
  schema checks.
- Integration points (M5, M10, M12, M14, M16) also run the live Library test and an
  independent review. After M4, any change to `src/features/search/` runs G, including the
  Library and Blueprint cases.

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Search session | Definition type and request handling with unit tests | Shared request handling works on its own |
| M2 | Library uses the session | Library on the session; text runs on submit | Text runs on submit; layout stays the same |
| M3 | Shared controls and results | State hook, controls, results, selection bar in Library Search | Display code shared |
| M4 | Second search | Blueprint on the session, controls, and results | Shared code proven by two searches |
| M5 | Shared page layout | Page layout and leave warning for both; Library Browse | Page behavior shared |
| M6 | Pool owner | Owner column filled by operations; API | Owner filter possible |
| M7 | Pool author names removed | Remove Pool author names and attribution text | Pool credit matches HG |
| M8 | Pool membership rules | Audit and repair; one copy per Question; one Type and Backend | Members form a proper set with shared Type and Backend |
| M9 | Pool license | Table, calculation, manual license removed, check | License filter fits whole Pools |
| M10 | Library server query | Combined query; kind, membership, Owner filters; kind lookup; speed check | Both kinds filtered, sorted, and paged together |
| M11 | Library links | `/library/{id}` routing; Pool detail page | Every result opens in a new tab |
| M12 | Library shows both kinds | Library definition with kind and membership filters and default; old panel removed | Combined Library search done |
| M13 | Question picker | Shared parts in the picker; Pool member rules | Shared code works in a dialog |
| M14 | Assessment content picker | Both kinds; Pool choice forks | One Assessment picker |
| M15 | Independent reviews | Usability walkthrough, axe, screenshot review, speed report | Quality checked by independent agents |
| M16 | Close-out | Guide, cleanup, docs, full checks | Shared search work documented and passing |
| M17 | Third user (optional) | My Blueprint Courses list on the session and results | Works beyond searches |

### Milestone: M1 search session

- Depends on: none.
- Deliverables: `search_session.ts` with `SearchDefinition` and `SearchSession` (`apply`,
  `setPageSize`, `next`, `previous`, `refresh`, `retry`, `reset`, `dispose`, `select`,
  `selectLoaded`, `clearSelection`); `tests/test_search_session.mjs`.
- Entry criteria: none.
- Exit criteria (narrow frontend check): tests show an outdated response is ignored, including
  its filter counts; a new search clears rows and filter counts while loading; a failed next
  page keeps rows and counts and retry resends the same page; a new filter or sort returns to
  page 1 and clears selection; page-size change keeps selection; the maximum is enforced; late
  responses after `dispose` are ignored.
- Parallel-plan ready: yes; independent of M6-M9.

### Milestone: M2 Library uses the session

- Depends on: M1.
- Deliverables: Library uses `SearchSession`; typed text runs on Enter or Search;
  `QuestionLibraryBrowseSession` removed; `library_browse_controls.tsx` and test harnesses read
  the new snapshot; exports the pickers use are kept.
- Entry criteria: M1 done.
- Exit criteria (G): `test_library_classification_search.mjs:425-474` rewritten for the
  session; Enter added after typing in `e2e_live_demo_question_library_browser.mjs:49-65` and
  in screenshot scenes (`scenarios_instructor.ts:205`, `scenarios_personal_theme.ts:141`,
  `scenarios_webwork.ts:145`).
- Parallel-plan ready: no; one frontend owner does M2-M5 so the shared code stays consistent.

### Milestone: M3 shared controls and results

- Depends on: M2.
- Deliverables: `search_state.ts`, `search_controls.tsx`, `search_results.tsx`,
  `search_selection_bar.tsx`; Library Search uses them; `library_search_filters.tsx` replaces
  the copied dropdowns; `library_bulk_actions.tsx` holds bulk-edit state; applied-filter chips
  and Clear all; `library_browse_rows.tsx` and unused scroll-window code removed.
- Entry criteria: M2 done.
- Exit criteria (G): browser verification (Enter sends
  exactly one request, typing alone sends zero, switching display sends zero, links open a new
  tab with `noopener`, removing a chip runs the search once) added to `run_fast_ui_checks.sh`;
  `test_question_pool_task_isolation`, `test_bloom_classification_workflow`, and the
  server-render archive test pass; a one-time check confirms `src/features/search/` imports
  only shared components, the API layer, and its own files.
- Parallel-plan ready: no; same owner.

### Milestone: M4 second search

- Depends on: M3.
- Deliverables: `blueprint_course_search_definition.ts`; the Blueprint page uses the session,
  controls, and results; before the first search it shows the search form and filters; three
  display modes kept; new needs go into shared code.
- Entry criteria: M3 done.
- Exit criteria (G): one-time Blueprint browser verification; a one-time check
  confirms the Blueprint page holds only its definition and page wiring; the screenshot scene
  at `scenarios_instructor.ts:343` still captures.
- Parallel-plan ready: no; same owner.

### Milestone: M5 shared page layout

- Depends on: M4, so both searches shape the page layout.
- Deliverables: `search_page.tsx` used by Library Search, Library Browse, and Blueprint; leave
  warning rules (on after a search the user runs or while text is typed; off after Clear and
  after a search that runs on open); Browse searches on open and shows rows after an exact filter is chosen; any
  URL starting value runs a search; the Pool creation dialog sits beside a hidden wrapper so
  the search survives it.
- Entry criteria: M4 done.
- Exit criteria (integration point): G with leave-warning cases (untouched page leaves
  directly, used page asks first, Clear turns it off) for Library and Blueprint;
  `tests/e2e/e2e_live_demo_question_library.sh` passes; independent review clean.
- Parallel-plan ready: no; same owner.

### Milestone: M6 Pool owner

- Depends on: none (backend owner; runs alongside M1-M5).
- Deliverables: required owner column on `question_pool`; creation, fork, and Course adoption
  fork fill it from the signed-in account; fixtures set it directly; Pool API and generated
  types include it; Question owner (from `question_ownership_event`) available to search.
- Entry criteria: none.
- Exit criteria (narrow backend check): store tests show the owner set by creation and by each
  fork path; generated types regenerated and `./check_codebase.sh` passes.
- Parallel-plan ready: yes; backend only.

### Milestone: M7 Pool author names removed

- Depends on: M6 (same table; one change at a time).
- Deliverables: `question_pool_authorship` table and the attribution text in
  `question_pool_provenance` removed; source-Pool link and Pool hints and feedback kept; the
  manual license stays until M9 replaces it; API, generated types, and the Pool credit editor
  updated to match.
- Entry criteria: M6 done.
- Exit criteria (narrow backend and frontend checks): API types carry the Pool owner and
  source-Pool link as the Pool's credit fields.
- Parallel-plan ready: yes alongside frontend work; one backend change at a time.

### Milestone: M8 Pool membership rules

- Depends on: M7.
- Deliverables: one-time audit script (`tests/_temp/`) listing Live Demo and fixture Pools
  with mixed Types, mixed Backends, or two Revisions of one Question, plus the Assessment
  entries that use them; repairs as described in "Repairing content that breaks the new rules";
  unique constraint on Pool and Question; required Pool Type and Backend columns filled from
  the first member; checks in every membership change (create, member edit, fork, Course
  adoption fork).
- Entry criteria: M7 done; audit report written.
- Exit criteria (B): store tests reject a second Revision of a member Question, a member of the
  wrong Type, and one of the wrong Backend; Live Demo rebuilds; before-and-after query shows
  unchanged Questions per Attempt and total points for every affected Assessment; independent
  review of the audit report and repairs is clean.
- Parallel-plan ready: yes alongside frontend work.

### Milestone: M9 Pool license

- Depends on: M8.
- Deliverables: one step that replaces the manual license with the calculated one: the
  license table in SQL; calculation on creation, membership change, and fork; required Pool
  license column filled automatically by PLE's calculation; the manual license
  field and the now-empty `question_pool_provenance` table removed; API, generated types, and
  Pool views show the Pool license while each member keeps its own license.
- Entry criteria: an independent reviewer checks the license table against HG,
  `docs/QUESTION_MODEL.md`, the interview decision record, and Creative Commons guidance on
  collections.
- Exit criteria (narrow backend check, then B): store tests for every table row, including
  CC BY + CC BY-SA giving CC BY-SA; the schema and API carry only the calculated Pool license.
- Parallel-plan ready: yes alongside frontend work.

### Milestone: M10 Library server query

- Depends on: M6-M9 (the Pool fields the query filters on).
- Deliverables: Library rows come from Questions plus Pools (`UNION ALL`); kind and owner
  columns; a kind parameter (both, questions, pools); a membership parameter (in no Pool, all)
  using an `EXISTS` check on Pool members across every Pool, including Assessment forks; an
  Owner filter; filter rules from the table above; counts for Questions in no Pool, Questions
  in a Pool, and Pools; Pool "newest" uses `created_at`; paging by sort value then public ID
  with both new parameters bound into the page cursor; an endpoint that returns the kind for
  an ID, with Library read access; Rust types, generated types, and a client decoder for mixed
  rows. The Library page still asks for Questions only.
- Entry criteria: M9 done; speed baseline recorded (method below).
- Exit criteria (integration point, B): `question_library/tests.rs` cases: kind filter; "in no
  Pool" leaves out a Question that is only in an Assessment fork; "all" includes it; a text or
  Tag match found only in a member returns the member's own row under "all" and leaves its
  Pool out; Author returns Questions only; Owner matches both kinds; a Bloom filter returns
  only rows with that assigned value; mixed paging
  returns every row once, in order, for both sorts and both membership choices, including tied
  sort values. Speed results recorded; independent review clean.
- Parallel-plan ready: yes; frontend M2-M5 continue meanwhile.

### Milestone: M11 Library links

- Depends on: M10 (kind lookup) and M5 (frontend owner free).
- Deliverables: `library_object_detail_page.tsx` shows `QuestionDetailPage` or
  `question_pool_detail.tsx` (panel moved from `library_pool_discovery.tsx:347-357` and
  `:659-756`; Star and Watch markup and `mayWatchPools` kept); watch inbox links go to
  `/library/{id}`.
- Entry criteria: M10 and M5 done.
- Exit criteria (G): `test_frontend_contract.mjs:529-549` and
  `test_question_watch_client.mjs:134-145` point at the new file; an automated `tests/_temp/`
  Playwright check opens a Pool ID and a Question ID at `/library/{id}`.
- Parallel-plan ready: no; frontend owner.

### Milestone: M12 Library shows both kinds

- Depends on: M11.
- Deliverables: Library definition asks for both kinds with the default (Both, Questions in
  no Pool) set in its starting query; Show and Pool membership filters in
  `library_search_filters.tsx`, membership disabled for Pools only; the default appears as a
  removable chip; Owner field and filter; Pool rows show a Question Pool label, owner, member
  count, Type, Backend, license, classification, Tags, and Bloom when assigned; choosing Pools
  only clears and disables Question-only filters; bulk edit by kind with one 1000-row maximum;
  URL starting values include kind and membership; `library_pool_discovery.tsx`, the Library
  `<details>` panel, and `?pool=` handling deleted.
- Entry criteria: M11 done.
- Exit criteria (integration point): G with a Pool row and a Pool member Question in the fast UI
  Library test data, and cases for the default (member hidden, Pool shown), All Questions
  (member shown), Pools only, and Pool rows in all three display modes; the live Library test
  passes, including a mixed result that opens a Pool in a new tab; independent review clean.
- Parallel-plan ready: no; frontend owner.

### Milestone: M13 Question picker

- Depends on: M12 and M8-M9 (member rules).
- Deliverables: the picker uses the session, controls, and results inside its existing dialog;
  the Pool member picker and Pool creation use All Questions and ask only for Questions that
  fit the Pool (same Discipline, Subject, Type, Backend, and a fitting license); M8-M9 server
  checks back this up.
- Entry criteria: M12 done.
- Exit criteria (G): `test_question_picker.mjs` and `test_question_pool_source_binding.mjs`
  updated; an automated check shows the member picker offers only fitting Questions, including
  a Question already in another Pool.
- Parallel-plan ready: yes with M14 if a second frontend owner takes one; shared code frozen.

### Milestone: M14 Assessment content picker

- Depends on: M12.
- Deliverables: one Assessment content picker for Blueprint and Course Assessment editors
  showing both kinds with the Library default and membership filter; choosing a Pool runs the
  existing fork import with its current confirmation; the Assessment picker replaces
  `question_pool_picker/`, which is deleted.
- Entry criteria: M12 done.
- Exit criteria (integration point): G; automated `tests/_temp/` Playwright checks add a
  Question and a Pool to a Blueprint Assessment and a Course Assessment and confirm the fork
  entry; independent review clean.
- Parallel-plan ready: yes with M13 (see above).

### Milestone: M15 independent reviews

- Depends on: M12-M14.
- Deliverables: one-time accessibility check (axe on Library,
  Blueprint, Pool detail, and both pickers; keyboard path through text, filters, results, and
  paging); usability walkthrough report in `docs/active_plans/reports/` covering five tasks
  (find a known Question with `field:value`; explore by Discipline then Subject; find and open
  a Pool; find a Question that sits inside a Pool; sort Blueprints by Stars) with findings
  ranked by severity; `image_evaluator` review of screenshots (1280x800 density, three modes,
  Questions and Pools easy to tell apart, active default chip visible, Instructor layouts
  reviewed at the Human Guidance target of 1280x800); speed report; fixes for every high and medium finding.
- Entry criteria: M14 done.
- Exit criteria: zero serious or critical axe violations; every high and medium usability and
  visual finding fixed; G passes.
- Parallel-plan ready: yes; four independent reviews.

### Milestone: M16 close-out

- Depends on: M15.
- Deliverables: `docs/SEARCH_PAGE_ARCHITECTURE.md` (what shared code handles, definition
  checklist, slots, test pattern, steps to add a search); duplicate constants removed
  (`QUESTION_LIBRARY_PAGE_SIZES`); screenshot scenes for Library (default, All Questions, Pools
  only), three display modes, Blueprint, Pool detail, and pickers; doc updates below;
  `tests/_temp/` emptied after deciding which checks become permanent.
- Entry criteria: M15 done.
- Exit criteria (integration point): `source ./source_me.sh && ./launchers/all_test.sh`;
  `./devel/capture_screenshots.sh --verify`; `tests/e2e/e2e_live_demo_question_library.sh`;
  `ui_backbone_parity`; a final independent review of everything the plan delivered.
- Parallel-plan ready: yes; docs and screenshots split cleanly.

### Milestone: M17 third user (optional)

- Depends on: M16.
- Deliverables: My Blueprint Courses list (`blueprint_courses_workspace.tsx:101-180`, which
  repeats the same paging code) moves to the session and results using only
  `docs/SEARCH_PAGE_ARCHITECTURE.md`.
- Entry criteria: M16 done.
- Exit criteria: G passes; any shared-code change made here is also written into the guide.
- Parallel-plan ready: yes.

## Test and verification strategy

- Checks: narrow checks per milestone; G, B, live tests, and independent reviews at
  integration points; at the end `run_fast_checks.sh`, `all_test.sh`, and screenshots.
- Permanent coverage after the user-requested policy review:
  - Shared session tests own stale responses/facets, query reset, retry, cursor recovery, and
    bounded selection; failure means search state can mislead users or lose their selection.
  - The existing live Library browser journey owns submit/display behavior, mixed membership,
    new-tab detail navigation, and leave confirmation; failure means a user journey regressed.
  - Connected database tests own authorization, mixed global paging/membership, immutable Pool
    shape, exact pins, fork ownership, and calculated licenses; failure means integrity or
    visibility regressed. Keep each invariant at its closest meaningful boundary.
  - Existing picker tests retain exact Revision pins and the distinct Pool membership edit token.
- Accessibility scans, keyboard walkthroughs, exhaustive filter surveys, and consumer-by-consumer
  display checks are implementation review evidence unless a specific uncovered regression
  earns permanent protection.
- One-time checks (`tests/_temp/`, removed at close-out, results noted in the changelog):
  import-direction and shared-code-only checks; Pool audit and before-and-after Assessment
  totals; double submit on a slow network; localStorage, sessionStorage, and IndexedDB stay
  empty after searching and switching modes; after a sort change, the first result can come
  from a later server page; link and picker flows.
- Speed check: a `tests/_temp/` script calls the Library search endpoint on Live Demo for four
  queries (default, one text word, one Discipline filter, All Questions) at 50 rows: three
  warm-up runs, then 15 timed runs each; record median and 90th percentile before M10 and
  after M10 and M12. If the median after M10 is above the baseline's 90th percentile, run
  EXPLAIN ANALYZE and fix clear query-plan problems (missing index, repeated subquery) before
  M11, then record the result.
- When an existing test fails: decide whether the behavior changed on purpose (update the test
  in the same milestone) or broke (fix it before the milestone ends).

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Shared code picks up Library-only ideas | Other searches need workarounds | One-time check finds Library imports, or Blueprint copies code | Frontend owner | Blueprint before page layout; kinds and membership stay in Library; fix shared code |
| Moving code breaks bulk edit, Pool creation, or Browse | Instructor tasks break | Guard tests or live test fail | Frontend owner | Small steps M2, M3, M5; guard tests in G |
| Content repair changes what an Assessment delivers | Wrong Question counts or points | Before-and-after totals differ | Backend owner | Repair rules keep totals; independent review of the report |
| Default hides member-only matches | Instructors miss a Question | Walkthrough task "find a Question inside a Pool" fails | Frontend owner | Visible default chip; All Questions one click away; fix per walkthrough findings |
| Combined query is slower | Library feels slow | Median above baseline 90th percentile | Backend owner | EXPLAIN ANALYZE; add index or trim columns before M11 |
| New Assessment picker breaks fork import | Assessment editing breaks | M14 checks fail | Frontend owner | Call the existing import unchanged; keep its confirmation |
| Generated API types fall behind Rust | Type errors | Rust change without regenerating | Backend owner | Regenerate in the same milestone; `check_codebase.sh` catches it |
| Pool validation work enters this plan | Delivery slows | Mismatch or release-block work appears | Manager | Route it to the schematic's Pool validation handoff |
| Plan and schematic disagree | Docs mislead | A milestone changes the design | Manager | Update the schematic status each milestone |

## Rollout and release checklist

- [x] Live Demo rebuilt from the base schema after M6-M10; fixtures and repaired content load.
- [x] Screenshots refreshed after M12 and M16; `image_evaluator` review recorded.
- [x] `docs/CHANGELOG.md` updated after every milestone.
- [x] Final report lists any product decisions the manager made during the work.

## Documentation close-out requirements

- Active plan / progress tracker: update the schematic's status each milestone. At the end,
  mark the shared search page, combined Library search, and the Pool fields this plan delivered
  as done with evidence, and keep the schematic in `docs/active_plans/active/` with its "Pool
  validation handoff" items listed as still open (mismatch warnings, release blocking,
  classification re-checks, unordered member storage, sortable member editor). Update evidence
  rows in `docs/active_plans/audits/human_guidance_implementation_checklist.md`.
- docs/CHANGELOG.md entry: one per milestone under that day's headings.
- `docs/SEARCH_PAGE_ARCHITECTURE.md`: new guide for adding a search.
- `docs/QUESTION_MODEL.md`: update its implementation-alignment paragraph (one copy per
  Question and Pool Author removal are done; member positions and classification re-checks
  remain).
- `docs/DESIGN_DECISIONS.md`: shared search code lives in `src/features/search/`; text runs on
  submit and why; result kinds and membership belong to each search and are combined on the
  server; Pool owner, Type, Backend, one copy per Question, calculated license.
- `docs/TERMINOLOGY_CONTRACT.md`: Questions in no Pool, result-kind choices, Pool owner,
  calculated Pool license.
- `docs/HUMAN_GUIDANCE.md`: add the user's rule that every search offers three display sizes,
  in their words.
- `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`: search code, definitions,
  `/library/{id}` routing.
- Temporary checks: make permanent or delete everything in `tests/_temp/`.

## Open questions and decisions needed

- How the manager and subagents decide:
  - Who decides: frontend owner for the shared code's shape; backend owner for Library data;
    manager for scope and conflicts.
  - Rule: HG, then `docs/QUESTION_MODEL.md`, then the interview decision record, then this
    plan; if none answers, the simplest option that fits HG, recorded in
    `docs/DESIGN_DECISIONS.md`.
- Later follow-ups (none block this plan):
  - Pool validation handoff in the schematic: mismatch warnings, release blocking,
    classification re-checks, unordered member storage, sortable Pool member editor.
  - More searches after M17: My Questions, Starred, Assessment Templates.
  - Whether "Used in my courses" should someday match a Pool through its Assessment forks
    (this plan keeps Pools out of that filter).
  - Whether the "probably" default should change after the usability walkthrough (one-line
    change in the Library definition's starting query).
