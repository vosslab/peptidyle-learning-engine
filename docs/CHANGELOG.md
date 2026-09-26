# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 18 entries are archived in [CHANGELOG-2026-09n.md](CHANGELOG-2026-09n.md).

## 2026-09-25

### Fixes and Maintenance

- The Question ID contract generator creates its output directory before writing. Clean checkouts
  now build without a missing `generated/api/` failure during Live Demo screenshot startup.
  Clean-directory generation, freshness checking, and the complete debug build passed.
- Screenshot capture installs missing npm dependencies before browser setup or static verification,
  installs its required Chromium browser, and streams Live Demo startup progress. Dependency ordering
  and eight existing driver checks passed.
  The fast gate passed Rust and browser-source checks; pytest passed 9,489 checks with one existing
  broken `region_spec.ts` link in the September 25 record presentation audit.
  Live capture passed host build and database initialization, then stalled at MinIO startup.
  A direct pull of the configured Quay digest returned `unauthorized`, including with empty registry
  credentials; the complete capture remains unverified.

### Developer Tests and Notes

- Public Blueprint search shows the classification already returned with each result and can sort the whole query by name, adoptions, or students. Returning still restores that search, including scroll position. Stars, Watches, and last edit are not sort keys until the discovery row carries them.

- Removed the unused Question Library virtual-row measurement. Browse and search render the current server page.

- Removed the unused Blueprint Course page-append helper. My Blueprint Courses and public search replace each server page instead of accumulating rows.

- Opening a Blueprint from public search keeps the return to that result page, including when the viewer owns the Blueprint. The breadcrumb parent still follows read access. Without a search token, owners return to My Blueprint Courses.

- Closed the shared record and page presentation work. `./launchers/all_test.sh` passed, including live installation-data acceptance. The screenshot corpus completed, and the Blueprint detail checkpoint now opens the detail route so that capture stamps `blueprintCourseDetail`.

- Removed the unused RecordList client window helper. Question and Blueprint discovery keep one server page of 50, 100, or 250 records. The browser contract no longer exercises windowing; status, actions, selection, media, body, refresh, reorder, paging, and notices stay.

- PageFrame now owns the ordinary content stack. PageSection carries an in-page heading, optional helper and actions, and the section body. Student Courses puts invitations in page actions, and Grades groups each Course with PageSection. Repeated content-root gaps moved into that stack. `contentClass` remains only where an editor, table, library task, or failure card still needs an inner hook.

- Student Coursework and Gradebook browser proofs now assert the visible Assessment title, action link, and score text. They no longer look for removed record-region nodes.

- Removed the RecordRegion grid from RecordList and RecordSequence. Ordered Assessment Entries now use semantic identity plus a bounded Pool body, and shared movement stays on the Sequence.

- Instructor screenshot scenarios add Assessment Questions through Choose published Questions, search, one checkbox per seeded title, and Add selected Questions. The retired Add Question and Question-ID controls are no longer driven.

- Assessment Questions now adds published Questions and Pools through the bounded pickers. The catalog-dump route, store method and SQL function are removed. Save, capacity and conflict behavior remain.

- Question Library search now asks the store for one bounded page, then reads native source only for those returned items. An invalid continuation is rejected before that read. The browser page shape is unchanged.

- Question Pool discovery now shows one SQL page at a time through shared Previous/Next and 50/100/250 controls. Pool rows use semantic title, description, classification, Bloom, identity and Inspect; ordered members use a read-only Sequence. Inspection still returns focus and scroll to the results trigger.

- Public Blueprint Course search keeps one server-ordered page, the shared 50/100/250 page size, and Previous/Next. Returning restores the current cursor and focus without replaying earlier pages.

- Migrated the fixed Question point-value editor to semantic RecordList rows. Each row keeps the
  exact Question revision as its title and the editable point value in the bounded body; aggregate
  Save/Cancel, validation, conflicts and draft retention stay on the editor.

- Continued the RecordList caller migration across Student, Instructor, Library and Blueprint pages.
  Student Course, Course invitation and Assessment Overview records now expose their stable task
  content through semantic rows; pending invitations retain the authenticated route, confirmation,
  busy, reload and focus behavior. Proposal records preserve state, native creation time and paging;
  Blueprint stewardship and starred-Instructor names retain their existing chronology and privacy.
  Student Coursework now uses the shared Type and native due-time facts, and the Due Soon caller keeps
  the distinct Course and Assessment destinations. Obsolete invitation, draft and due-soon row CSS
  was removed where shared presentation replaced it. Independent review found and closed concrete
  score wording, fixture validity, Course action evidence and stale due-time selector gaps. Focused
  browser, existing workflow and source checks passed for the accepted packages. Focused populated
  Scores and Draft contracts now protect Course grouping, released-score withholding, and the
  confirmation, concurrency and recovery behavior for Draft deletion. A populated Due Soon proof
  caught a formatter that captured UTC before the response loaded; the caller now tracks the loaded
  Account zone, and Chromium confirms the resulting Chicago display time. The fast UI rerun also
  found an incomplete Pending Invitations App-shell stub that kept the route in session loading;
  the browser test now supplies the shell's null-avatar response and owns its invitation data inline.

- Completed shared Sequence movement, sort and page controls, plus the WP-N2 loaded-name and parent
  navigation publishers. Focused browser contracts and independent reviews passed; the full stack-free
  UI gate passed on the integrated current tree. Native shared `RecordFact` time now retains ISO
  `datetime` and visible formatted text for Watch and other time callers.

- Implemented the first shared `RecordList` content contract: typed semantic facts, media, native
  actions, bounded domain bodies, same-ID reactive updates, and collection notices that retain loaded
  rows. Converted the fixed-points editor and Instructor Accounts to the existing Detail presentation,
  and made Assessment workspace breadcrumb publication follow the current route/session generation.
  The existing component browser contract and new fixed-points and A-to-B-to-A publisher browser
  checks passed in the fast UI launcher; focused TypeScript, lint, format, and independent review
  checks passed. The launcher required the host permissions Chromium needs on this macOS environment.

- Migrated Student Course Progress and Attempt History to the shared semantic content model and
  removed their row-specific and phone-specific presentation. Course grouping, score/activity states,
  dates, links and per-Course cursor pagination remain in the caller data. Existing focused behavior
  checks, type/lint/format checks and independent source review passed. The review restored Progress'
  parent spacing and readable disclosure treatment while leaving row layout to RecordList; populated
  wide/narrow screenshots remain part of final integrated validation.

- Extended the shared semantic record content and bounded editor-body renderer to native ordered
  `RecordSequence` items. Same-ID refresh retains body input state and action identity while metadata
  updates; loading/error notices retain ordered records, and content reflows at the narrow viewport.
  Compile-time mode checks, independent review, and the complete fast UI launcher passed. Remaining
  callers and movement controls stay scheduled for their dependent work packages.

- Migrated the Pool creation preview, Assessment Blueprint update review, and Blueprint History to
  shared semantic list/sequence content. Exact Question revisions and Pool edits, current/proposed
  labels, history order, Inspect action, Outline, and classification details remain visible. Added
  a typed Course classification fact to the shared component and verified current/retired labels,
  tags, and narrow layout in its browser contract. Focused caller tests, independent review, and the
  integrated fast UI launcher passed.

- Completed four editable Sequence migrations: Blueprint Assessment content, Blueprint Pool members,
  Blueprint fork apply, and Assessment Pool entries. Shared movement/actions replace caller-owned
  controls while bounded editors, destination selection, attestation, count restrictions, and deferred
  saves remain with their owners. Independent reviews corrected fork removal semantics and verified
  the Assessment Pool refresh/announcement path; its temporary populated caller proof was deleted
  after acceptance. No caller-specific permanent browser tests were added.

- Completed WP-N3 Blueprint local context. The selected Assessment shows its loaded Blueprint and
  Module ancestry, preserves edits when returning to the outline, and restores focus to the originating
  Assessment action. Independent source review and an ignored populated Chromium proof passed; the
  temporary proof was removed and no caller-specific permanent test was retained.

- Completed WP-P0 discovery bounds for Questions, Pools, and Blueprints. The canonical disposable
  PostgreSQL baseline returned all 250 matching Pool rows without a continuation cursor and verified
  Blueprint pagination across 250 results plus one lookahead row; the second page returned that final
  row only. Generated contracts, zero/over-bound rejection, focused tests, and independent API/SQL
  review also passed.

- Completed WP-P1 and WP-P4 PostgreSQL query contracts. Question search passed typed metadata/text
  filters, literal wildcard and exclusion behavior, and tied-title/tied-date keyset pages (50 + 1)
  in canonical Question ID order. The Rust store now calls the 23-argument SQL contract, and the
  private search uses private classification readers under its owner role; independent review
  confirmed authorization and results remain equivalent. Blueprint sorting passed query-wide
  ordering across 250 rows plus one lookahead. Populated plan samples returned 51 Question rows in
  570.45 ms and 251 Blueprint rows in 14.945 ms, with zero disk reads; the ignored plans remain at
  `tests/_temp/record_list_p1_p4_populated_plans.json` for WP-P6's scale review.

- Accepted WP-C2P after an ignored Chromium composition proof of the production Course Progress page.
  At 1280x800 and 393x852, released 7/8 and withheld-score states, shared Type labels, and semantic
  RecordList rows remained visible without overflow or browser/console errors. The inline API data
  and proof artifacts were removed; no caller-specific permanent test was added.

- Accepted WP-C2H after an ignored Chromium composition proof of the production Attempt History page.
  At 1280x900 and 393x852, Course groups, disclosed and withheld submitted scores, native Started and
  Submitted times, per-Course 50+1 pagination, and flat shared boundaries remained intact without
  browser errors. The Live Demo was unavailable, so the proof used inline API data; temporary scripts
  and screenshots were removed after acceptance, with no caller-specific permanent test added.

- Clarified the Question discovery handoff: WP-P1 owns the store-only typed predicate and cursor-position
  contract, while WP-P3 retains the existing server parser and opaque query-bound cursor codec and
  adapts once into the store. This keeps SQL from parsing browser text or owning cursor tokens and
  avoids a duplicate parser across the two sequential data leases.

- Applied the follow-up external review to the
  [RecordList plan](active_plans/active/record_list_page_frame_standardization_plan.md) and ledger.
  Clarified domain bodies versus shared record presentation, including form-local controls, and made
  same-ID refresh with retained input focus/draft explicit in the existing CORE and Course editor
  browser proofs. Retained milestone numbering, dependencies and the scoped discovery contract.
  One-time document checks passed for 124 local links, 32 milestones/packages, 85 file tasks and
  the acyclic completion graph; the 38-caller table remains complete. These are planning edits;
  implementation and product testing remain pending.

- Investigated RecordList's real query-to-render paths and added the
  [data-flow audit](active_plans/audits/record_list_data_flow_investigation_2026-09-25.md) and required
  [bounded data workstream](active_plans/workstreams/record_list_bounded_data_workstream.md).
  The plan keeps presentation in Solid/TypeScript, moves Question search/facets/paging to PostgreSQL,
  resolves only returned native Questions, corrects Blueprint partial sorting, and replaces the
  Assessment editor's full-catalog/first-page-only discovery with reusable paged pickers.
  Recorded current defaults of 50/max 100, then incorporated the human's rapid-scanning requirement
  as shared 50/100/250 choices, default 50, with a scoped API/SQL/decoder limit change.
  The shared decoder also used its search-page bound for prompt blocks; the plan separates those meanings.
  Added eight bounded milestones and updated caller dependencies; windowing earns retention through
  an implementation-time comparison. One-time document checks passed for seven planning artifacts,
  248 local links, 32 milestones/packages, 85 file tasks and an acyclic dependency graph whose tasks
  all reach closure. The complete 38-caller table remains present. Planning only: no production code
  changed, no product tests or runtime performance measurements ran.

- Deepened the RecordList caller investigation and added the
  [complete caller table](active_plans/audits/record_list_caller_investigation_2026-09-25.md).
  Revised the plan around expressive shared content: descriptions, linked facts, image media,
  copyable references, native pressed/disclosure commands, focus refs and bounded editor bodies,
  with presentation owned by the shared family. Stable record/action identity and reactive body ownership address refresh/focus risks.
  Sequence now reuses that foundation; separate milestones centralize movement and
  result-sort controls. The plan consolidates Library's duplicate metadata layouts and loaded-state
  notices while preserving genuine avatar List/Gallery. Documented existing native/backend Question
  preview paths and the missing production snapshot capability without making it a migration gate.
  A fresh source inventory matched all 38 direct caller files to the audit table; plan/ledger checks
  passed for 24 milestones, 84 file tasks, local links and the complete dependency graph.
  This is planning only; no production code changed and no
  product tests ran.
- Completed the initial caller investigation for the
  [record_list_page_frame_standardization_plan.md](active_plans/active/record_list_page_frame_standardization_plan.md).
  All 38 RecordList/Sequence consumer files now have source-based content dispositions and individual
  prerequisites in the ledger. The base contract is title, always-visible semantic details and native
  actions, with shared Assessment Type icon/label rendering required by Human Guidance. Selection,
  Question Preview and avatar Gallery have concrete consumers. Detail and Sequence
  work can start independently, and frame tasks wait only for their actual navigation overlaps.
  Replaced Git inspection gates with source/document checks and made shared defaults and test restraint
  explicit in each dispatch. One-time document checks passed for local links, all 83 ledger tasks
  and the complete acyclic dependency graph. This was planning/source investigation; implementation
  remains pending, and no product tests were run.
- Prepared the autonomous
  [record_list_page_frame_standardization_plan.md](active_plans/active/record_list_page_frame_standardization_plan.md)
  and its file-level execution ledger and manager goal. The plan proves a minimal flat scan first,
  uses one shared reflow treatment, admits only demonstrated shared capabilities, and completes the
  caller/API/CSS cleanup. It includes bounded ownership, independent breadcrumb work, agent-operated
  verification, and explicit removal of staging and temporary evidence. This is planning work;
  production implementation remains pending.
- Audited RecordList, PageFrame, row styling, and breadcrumb hierarchy against the requirement that
  shared components enforce presentation standards. Current-source browser evidence confirmed
  hidden Student scores at phone width and touching rounded Attempt History rows. Recorded the
  proposed component contract, spacing ownership, ancestry fixes, and validation limits in
  [record_list_page_frame_standardization_audit_2026-09-25.md](active_plans/audits/record_list_page_frame_standardization_audit_2026-09-25.md).
  TypeScript and the focused RecordList browser contracts passed; this entry records an audit,
  with implementation recommendations still open.
- Completed the shared record-presentation components and all ledgered page migrations. The clean
  screenshot corpus, `source ./source_me.sh && ./launchers/run_fast_ui_checks.sh`, and
  `source ./source_me.sh && ./launchers/all_test.sh` passed. Full acceptance included database
  persistence, installation-data replay, and the PostgreSQL/MinIO Course-appearance oracle.
- Ran Live Demo and Chromium outside the sandbox. A fresh populated Blueprint capture confirmed
  Module-to-Assessment nesting and selection return; a populated phone roster capture confirmed
  Student/State/Action headers and horizontally reachable actions at a 393 px phone viewport,
  with a 334 px visible table scrollport.
  Temporary evidence is under `/private/tmp/record-presentation-evidence/`.
- Markdown-link validation passed (325 tests) and `git diff --check` passed.
- Corrected Known Blueprint forks to use the compact `RecordList` scan specified by the ledger;
  the expanded comparison remains a separate review. All 56 ledger sources now match their named
  component. The fast offline gate passed (9,439 Python tests), and the outside-sandbox Chromium UI
  lane plus a temporary Known Forks page fixture passed.
- Temporary current-source Chromium fixtures now verify populated Blueprint history and fork
  destination relationships, destination move/remove/restore/cancel behavior, paired fork
  differences, Student Response Stats, and a three-Question Attempt review with long feedback and a
  response table. Laptop and phone renders had no horizontal page overflow or browser errors.
  Captures and the ignored one-time fixture are under `/private/tmp/record-presentation-evidence/`
- Additional populated phone fixtures verified recovery Attempt selection and Question evidence,
  bulk metadata current values, nested discussion posts and active/cancelled Impact notices, and
  editable Content Disciplines records. Each retained content and actions without horizontal page
  overflow or browser errors.
- After the Known Forks correction, `source ./source_me.sh && ./launchers/all_test.sh` passed with
  9,439 Python tests and all three real-service oracles. The isolated Chromium UI lane passed, and
  `source ./source_me.sh && ./devel/capture_screenshots.sh --fresh --verify` passed manifest
  closure, privacy, and artifact-integrity checks. It retained 149 byte-different replay PNGs;
  visual review of migration-related pages found seeded ID/time and persona-accent changes with the
  same layouts.
- The 56-row Record presentation migration is complete; its documentation retains the migration
  plan and the dated candidate inventory that grounded its scope.
- Consolidated Gradebook and Course roster styling in the shared table family. Replaced invalid
  `minmax()` column widths with percentages and added logical column alignment. Browser contracts
  verify full-width laptop tables, column edges and headers, roster actions, and narrow horizontal
  scrolling without page overflow.
- After the final table changes, `source ./source_me.sh && ./launchers/all_test.sh` passed, including
  all 9,439 Python tests and the database persistence, installation-data replay, and PostgreSQL/
  MinIO Course-appearance oracles. The Chromium UI lane passed outside the sandbox.
- Published the fresh complete screenshot corpus and atlas outside the sandbox, and visually
  reviewed the updated Gradebook and roster captures. Populated current-source laptop and phone
  captures confirmed readable tables, internal horizontal scrolling, and no page overflow.
- The independent KISS/test-liability audit removed source-inspection tests, brittle table geometry
  assertions, duplicated sibling state checks, and the one-time migration fixture. Plan gates now
  follow responsive risk, and its archive closure is recorded as pending the required writable Git
  index.

## 2026-09-24

### Decisions and Failures

- Neil approved the hybrid Student Course-context model: Coursework and Grades stay across enrolled
  Courses, Courses opens explicit Course-specific content, and no persistent Course pin filters the
  global views. This supersedes the earlier unresolved-pin note below.
- Updated the Student Tier 2 grouping: Active Attempt is under Coursework and Response Stats is
  under Grades. The current labels and order are recorded in Design Decisions.
- Moved Response Stats under Grades in the route and Ribbon contracts, matching its role as an
  outcome summary. Recorded the current Student Tier 2 grouping in Design Decisions.
- Recorded the Student Tier 1 order as Coursework, Grades, Courses and the fixed per-area choices in
  Design Decisions.
- Updated the Student navigation contract so Coursework and Grades span all enrolled Courses, while
  Courses Tier 2 lists the enrolled Course short names in stable order.
- Made the Student Tier 2 layout a first-class role-and-Tier-1 schema and moved Student Course,
  Active Attempt, and Latest Feedback lookup rules into a small Ribbon helper. The Application Shell
  refreshes that small lookup on Student route changes and keeps the last result while it reloads.
- Confirmed Attempt History is grouped by Course, with each Course section newest first and
  independently paginated. Course identity and the existing Course-scoped cursor API match the
  Student view; the page does not promise a single chronology across Courses.
- Added a Course-authorized Active Attempt read that selects the latest-activity unsubmitted Attempt
  with an unexpired deadline and drives the fixed Coursework shortcut's disabled/enabled state.

### Fixes and Maintenance

- Kept successful per-Course Active Attempt lookups when another Course lookup fails and preserved
  Instructor-authored Course long names unchanged in breadcrumbs.
- Removed redundant route-sampling tests and the standalone CSS-geometry probe; the fixed Tier 2
  and route-stability contracts remain.
- Kept Active Attempt as a direct Coursework shortcut to a timed Attempt with a running clock; it stays in place and is disabled otherwise.
- Kept submitted Attempt reviews in Grades so Latest Feedback and the Grades Tier 2 row remain in context.
- Fixed the signed-in non-phone Ribbon identity and Profile geometry across roles and pointer types; only Tier 1 links respond to mid-width or touch compaction.
- Changed matching-response columns to wrap according to the control's available width, so intermediate page widths do not leave narrow columns.
- Removed the response controls' obsolete restore/reset-to-initial actions; saved responses still load when an Attempt resumes.
- Removed the redundant Tier 2 caption across roles. The named navigation
  landmark and destination links retain their accessible names; the caption
  was plain text, not a semantic group label. Screenshots are refreshed below.
- Kept the Student phone Ribbon in one primary row: the P mark, Coursework,
  Grades, Courses, and the fixed Profile box; Tier 2 stays beneath. Refreshed
  the canonical corpus and atlas, then regenerated and inspected the viewport
  averages and native phone captures.
- Made the existing Scores screenshot wait accept both valid states appearing
  across Courses: a Course with released scores and another with none.
- Simplified responsive Ribbon evidence by removing high-zoom/scrollport
  geometry exceptions, an obsolete narrow top-row overflow probe, and a blanket
  touch-target size assertion and full-wordmark-on-phone check for fixed Ribbon
  controls. Keyboard focus and canonical screenshots cover current responsive
  behavior.
- Kept the Ribbon Account avatar at the same visual size on touch and pointer viewports. Public
  headers now show a neutral Not signed in avatar in that same box, or the Account avatar when a
  session is authenticated.
- Reserved the heavier Ribbon label width in every state, allowing a modest selected weight without
  moving the labels or adjacent tabs. Rebuilt the canonical screenshots and regenerated diagnostic
  averages outside the published corpus; direct browser measurements confirmed stable Tier 1
  positions at the sampled Student, Instructor, and Sysadmin viewports.
- Added a pytest gate against alternate viewport folders, screenshot names, and manifest entries
  under the Instructor and Sysadmin corpus, including empty folders. It preserves the laptop
  `webwork_chi_square.png` capture, whose name describes Question content. Screenshot staging now
  creates only role folders; each actual capture creates its viewport folder as needed.
- Temporarily held Ribbon label weights constant to isolate the selection drift; the later width
  reservation above restored a modest selected weight without changing tab geometry.
- Archived the completed Student Progress and Response Stats plan, updated its active-plan links,
  and republished the canonical screenshots after the Student wording change. Markdown-link and
  screenshot-corpus checks passed against the published artifacts.
- Replaced internal "Assessment" wording on Student Course Progress, Due Soon, and Completed pages
  with Coursework language and the item's specific Type in the Progress action. Kept server-time
  window details in the contract rather than the Student page, and removed a copy-specific Progress
  assertion while retaining the score-release and completion behavior checks.
- Applied the permanent-test checklist to Student navigation: removed a duplicate internal Tier 2
  schema test and assertions about failed lookups and request order, kept the route-level Ribbon
  contract, and narrowed the Tier 1 assertion to visible labels and destinations. Simplified the
  Active Attempt lookup to one Course request group that yields no shortcut when its result fails.
- Preserved the enrolled Course order returned by the Courses API in the Student Ribbon, matching
  the Courses list rather than sorting short names a second time.
- Aligned the Attempt breadcrumb contract fixture with the explicit Course and Assessment route
  parameters required by Course-specific navigation.
- Updated the connected Active Attempt assertion to compare the selected Attempt ID from its
  Course-scoped result, made its fixture Assessment available under the Student's schedule, and
  verified that an authorized second Course does not inherit the first Course's Attempt target.
- Ordered the connected Response Stats fixture's Question issue, saved response, and submission
  timestamps consistently with its finalization constraints.
- Kept both timed Attempts in the Active Attempt and duration-checkpoint fixture within their
  Assessment deadline, so the test covers eligible running-clock behavior.
- Matched the global Latest Feedback test to its empty-target behavior for a Student with no Courses;
  Course-scoped reads continue to reject nonmembers.
- Matched the direct Assessment Access oracle to the available, two-Attempt state created by its
  Course Progress fixture.
- Corrected the Ribbon design record to preserve Courses API order, documented the Student plan's
  completion evidence, and clarified that another Course overview destination is a separate future
  choice.
- Published the complete fresh screenshot corpus and passed the fast and full repository gates.
- Kept repository hygiene discovery safe during unstaged file renames by skipping missing tracked
  paths before content filters inspect them.
- Formatted Progress and Attempt History timestamps in the selected Account display zone, while
  keeping the time-zone name on Profile only.
- Added connected PostgreSQL coverage for Latest Feedback with no eligible feedback, newest eligible
  Attempt selection, and Student/Course authorization.
- Marked the older shared-UI plan's Student navigation and time-zone proposals as superseded by the
  current Student contract.
- Let Students move between Questions or submit an Attempt without resending an already saved
  response; navigation and submission still save edited responses.
- Updated representative Question-format captures to open All Coursework after Student Course entry
  changed to match the Student's primary destination.
- Updated the Student Scores screenshot workflow for the all-Courses Scores page and corrected the
  Response Stats capture's declared Tier 1 area to Grades.
- Made the two-Course screenshot wait for the accepted Course to appear before checking the Course
  list count.
- Aligned the Progress screenshot flow with the Course landing page's Course Progress link and its
  Courses Ribbon area.
- Scoped Coursework capture waits to each enrolled Course section so repeated loading messages in
  cross-Course views settle without strict-selector ambiguity.
- Applied the same per-Course section wait to Response Stats and Attempt History captures, which
  also render one loading state per enrolled Course.
- Updated Response Stats capture navigation to select Grades Tier 1 before using its Tier 2 link.
- Kept the Latest Feedback selector behind the private-owner function boundary; the API role no longer
  needs direct access to private Assessment Attempts.
- Replaced current Response Stats source, transport, page, test, Rust module, and SQL file names that
  still said Practice Stats. The current UI, endpoint, and implementation now use Response Stats.
- Kept the all-Course Response Stats page in separately labeled Course sections; each section
  aggregates within its Course and does not merge counts across Courses.
- Waited for loaded Progress content and both Course Tier 2 links before their screenshot captures,
  and changed the Latest Feedback capture to follow the actual shortcut to its review.
- Added a focused Student Ribbon navigation contract test for stable Course ordering and latest
  resumable-Attempt selection.
- Reused the shared All Coursework screenshot helper in the Question-format scenarios, removing a
  stale Course landing route expectation.
- Bound Question-format screenshot navigation waits to the requested Question position so a late
  prior response cannot satisfy the next checkpoint.
- Captured the selected Course landing page at each existing Student viewport and renamed the
  Course Grades screenshot scenario to Scores; removed its now-covered deferred entries.
- Renamed the combined Progress/Response Stats/Attempt History scenario to match its current Stats
  terminology.
- Waited for the seeded Coursework row before capturing selected Course landing pages, removing the
  visible loading state, and added an Attempt History capture with an available Latest Feedback
  shortcut.
- Waited for the single-Course Tier 2 Course link before taking the Courses list capture, matching
  the settled lookup wait already used by the two-Course scenario.
- Captured the two-Course list at each Student viewport through the Courses destination and
  tightened the Progress, Response Stats, and Course History list captures to self-only privacy
  profiles; selected Attempt review retains the released-feedback profile.
- Made screenshot-matrix checks validate each scenario's declared direct captures and
  representative viewport substitutions.
- Split the Student Course capture workflow and connected PostgreSQL oracle into focused modules,
  keeping each authored source file within the repository's line limit.
- Updated the connected PostgreSQL runner to invoke the module-qualified access oracle after the
  split, so its fixture completes before the downstream expiry oracle reads it.
- Moved the Ribbon route fixture's Course label constant before module-time route materialization so
  focused Ribbon contract tests can load reliably.
- Reconciled the active record-presentation migration ledger against the September 23 audit and
  current source. It now assigns every genuine collection a presentation, whole-file owner, and
  completion evidence, while recording selectors, previews, navigation, settings, and fixed
  comparisons as task-specific structures.
- Added shared collection-state presentation and the semantic `RecordSequence`, `RecordTable`,
  `RecordOutlineList`/`RecordOutlineItem`, and `RecordDetailList` components. The focused
  current-source Chromium harness verifies every sibling's loading, empty, and error states;
  native order; table headers and cells; nested membership; expanded review entries; and sequential
  keyboard Tab traversal across their controls.
- Migrated Gradebook and Course roster to `RecordTable`, retaining all task columns, row headers,
  roster actions, and horizontal scrolling. Removed the table stylesheet after a current-user
  search and focused rendered table checks.
- Migrated Assessment entries, QuestionPicker selections, Pool membership and discovery, and
  Blueprint Assessment contents to `RecordSequence`, preserving exact Question Revision identity,
  ordering controls, and caller-owned save behavior.
- Migrated Blueprint detail, saved history, and fork destinations to nested outline and sequence
  presentations while keeping selection and structural changes with their pages.
- Migrated Attempt and recovery reviews, current metadata, discussion, Discipline, and fork
  differences to `RecordDetailList`; recovery Attempt choices and known-fork summaries use
  `RecordList`.
- Migrated the remaining Student, Instructor, and Library/Blueprint scans to `RecordList`, including
  Course Progress, per-Course Attempt History, Course-grouped Response Stats, Instructor Accounts,
  Course and Assessment collections, Library statistics, and starred-Instructor identities.
- Updated the architecture and design records to describe the shared family and each component's
  task. The migration plan remains active until integrated rendered-page evidence and final gates
  are recorded.

## 2026-09-23

### Additions and New Features

- Settled fixed Student Tier 2 destinations and Course-scoped Progress, Practice Stats, Coursework,
  and Attempt History behavior in the active implementation plan. Progress retains Attempts without
  a released score in a distinct **Score not released** state.
- Added the authenticated Course Progress API using the signed-in Student's exact active Course
  membership. Its Rust-owned contract exposes Attempt activity while withholding undisclosed scores
  and score-freshness details.
- Added the Course Progress page and its fixed Courses -> Progress destination. Every released
  Assessment stays in the list, with submitted completion and perfect-score status shown separately;
  activity times follow the browser's local display time.
- Added a self-only, Course-authorized Attempt History API with cursor pagination and per-Attempt
  score disclosure, plus a Grades -> Attempt History page that links submitted Attempts to their
  existing review and preserves access to older pages.
- Added Student Coursework Due Soon and Completed views. Due Soon uses the server-evaluated instant
  and the existing half-open rolling seven-day window; Completed uses the submitted-Attempt count,
  independently of score perfection or a newer open Attempt.
- Added self-only Course Practice Stats from actual submitted outcomes across eligible Assessment
  types, grouped by exact Published Question Revision (not only Practice Question Assignments). The
  API applies both Assessment-score and per-item-correctness disclosure before aggregation; the page
  shows each outcome numerator and denominator and links to a relevant existing Attempt review. Measured
  approximate time shown with the Question includes its sample count; unmeasured durations remain
  not recorded.
- Added nullable cumulative Question display-duration storage within Question Attempt Student Work,
  a self-authorized monotone checkpoint route that closes at finalization, and per-Question saved
  values in Attempt progress for reload recovery.

### Behavior or Interface Changes

- Documented the intended Student home behavior: one active Course opens Progress; multiple active
  Courses open the chooser, while selected-Course content keeps the Tier 2 row fixed by Tier 1.
- Student Tier 2 now derives from the selected Student Tier 1 area on every route. Course, Assessment,
  Attempt, and review navigation keep the same ordered row; Attempt return navigation uses the shared
  All Coursework destination instead of an Attempt-specific row.
- Moved the time-zone preference editor onto Profile and removed the obsolete Account Settings route.
  Other pages format times with that preference without repeating a time-zone label.
- Removed the time-zone name from Instructor Assessment availability messages. Dates and times remain
  formatted for the reader, while Profile stays the only page that identifies the selected zone.

### Fixes and Maintenance

- Updated Student Course-entry browser checks for the `Open Progress` destination and the
  `Available Coursework` list label, and completed the Practice Stats PostgreSQL fixture with the
  exact private Question-source Object Address required by the schema.
- Updated the shared Student Course screenshot helper to select `Open Progress` when the Student has
  multiple active Courses.
- Instructor Tier 2 now follows the fixed role-and-Tier 1 mappings across deeper routes. Course and
  Assessment local workflows use page links and breadcrumbs; Gradebook now has a Course actions
  link. The Assessment row stays Due Soon and Templates because current evidence does not settle a
  general collection task's name and shape; Browse All remains a future opportunity. Student Tier 2
  contents and order remained unresolved before the fixed Student contract was recorded above.
- Removed mechanism-specific font-loading assertions; computed production font-family checks
  remain.
- Updated the Instructor Accounts contract to verify that sign-in times use the viewer's display
  zone without naming it on that page.
- Kept the connected Attempt-expiry oracle compatible with an already-present immutable source
  binding for its shared fixture Question, and gave its zero-credit Gradebook check an isolated
  Student with no earlier scored Attempt.
- Updated Live Demo Student progress convergence to validate the nullable Question display-duration
  field added to the answer-free Attempt progress contract.
- Reused the shared Assessment-entry matcher in the response capture so a rerun can resume an
  interrupted Attempt as well as start a new one.
- Added explicit TypeScript return types to new Student Course transport, page, and screenshot helpers
  identified by the repository's lint gate.
- Applied Prettier to the Student navigation, Progress, Attempt History, Practice Stats, duration, and
  Profile files required by the repository format check.
- Corrected Student screenshot declarations to use laptop captures with documented representative
  coverage for other viewports, matching the scenario evidence actually produced.
- Removed the obsolete Account Settings redirect and selected the newest Attempt for the Course
  History review screenshot.
- Bounded the Course History screenshot fixture at 40 or more submitted Attempts, checked that its
  rows advance and remain available after navigating through Practice Stats, and selected the newest
  submitted Attempt for review. Jack remains the no-released-score example, and Avery remains
  available for question-response captures. Current screenshot coverage descriptions use current
  surface names.
- Combined the Course History and enabled Latest Feedback states in one screenshot: both are visible
  on the same History page, so separate checkpoints produced duplicate image bytes.
- Updated frontend route tests to require the Account Settings path to remain absent.
- Kept Human Guidance limited to concise first-person guidance and recorded detailed Student behavior
  contracts in Design Decisions. Clarified that Practice Stats summarizes actual disclosed outcomes
  across eligible Assessment types.
- RecordList now renders optional region headings on the same subgrid tracks as its rows and applies
  the same responsive priority hiding to both. The production Gradebook no longer owns a parallel
  fractional column grid. Its Chromium route check confirms matching header/region order and aligned
  edges across all four canonical viewports; visual review covered both tablet scroll positions.
- Removed the unused shared `.page` geometry and retargeted course-theme positioning to the scoped
  PageFrame root.
- Removed the duplicate reading-width cap from Assessment creation content; its default PageFrame
  owns page width while the content wrapper keeps only its task layout.
- Removed the Assessment Questions consumer override that replaced RecordList's responsive
  identity/action tracks with a separate one-column grid.
- Assessment Student View and workspace loading/error content now inherit their declared PageFrame
  width instead of narrowing only the body inside a full-width route.
- PageFrame-root `.route-error` content retains its compact 48rem cap while aligning to the frame's
  left origin; standalone error notices keep their centered placement.
- The signed-in shell's route `ErrorBoundary` fallback now uses PageFrame's title, lede, and action
  slots instead of a separate page heading and layout.
- Consolidated RecordList browser contracts into one shared Chromium fixture run and combined the
  Ribbon identity/geometry route sweep. Removed implementation-specific window pixel and DOM-node
  assertions while retaining identity/order, focus, reorder, region, and presentation behavior.
- Removed the unreferenced Ribbon design-variant screenshot sweep; its design matrix was
  implementation evidence, while the shared fixture remains in use by the durable all-theme Ribbon
  density check.
- Avatar presentation checks now protect catalog identity and selection across Gallery/List and
  viewports; layout geometry remains visual-review evidence rather than a CSS-measurement gate.
- M2 WP-B1/B2 checks pass. In a disposable copy, route-scoped tier-one behavior made WP-B1 fail
  when Instructor `courseAssessments` lost `courses`; pre-M1 conditional task-row behavior made
  WP-B2 fail with a 40px offset. The old WP-B1 schema used IDs removed by M1, so the isolated
  reproduction used current valid IDs to confirm the route-dependent behavior.
- Rotated older day blocks to [CHANGELOG-2026-09o.md](CHANGELOG-2026-09o.md), keeping the two
  newest dates active.
- The Canonical Blueprint JSON textarea now uses the shared monospace font token.

### Developer Tests and Notes

- Student Ribbon contract, route-contract, catalog, and component checks passed (35/35).
- Course Progress Rust API compilation, generated contract, strict browser decoding/client, and
  score-state presentation checks passed. The focused Node checks passed 16/16; the Rust wire
  contract test passed 1/1. The disposable PostgreSQL integration test compiled and is queued for
  the full live-stack gate.
- Course Attempt History contract, cursor binding, strict browser decoder/client, fixed Ribbon,
  Due Soon boundary, and Completed-membership checks passed. The focused coursework/history Node
  checks passed 7/7; API compilation and both Rust wire/cursor tests passed. The PostgreSQL oracle
  compiled but could not start because the configured acceptance runtime locator is unavailable.
- Course Practice Stats Rust API compilation and generated TypeScript contract pass. The first live
  schema replay exposed a missing Course ID in the Progress query's lateral projection; the select
  now carries the stored ID into its submission join. The replay also exposed the same omitted ID in
  the released-Assessment CTE; that projection now includes it, and a fresh database replay is pending.
- A fresh live schema replay exposed an ambiguous `assessment_id` in the Student Coursework
  projection. The returned columns now use the projection alias; rerun the database acceptance after
  this correction.
- The next schema replay showed the two new Course API function files were missing from the
  canonical install and grant include lists. Both are now included in base-schema replay; database
  acceptance remains pending.
- The Practice Stats and Attempt History private projections now run under the Student Work owner;
  their authenticated API wrappers resolve the signed-in Student and Course before calling the
  private readers. The history projection casts its public ID domain to the declared browser-safe
  text result.
- Updated the isolated one-Course entry browser contract to expect the settled Progress destination.

- Audit follow-up aligned the Ribbon design guides with the fixed role-and-Tier 1 rule and kept
  unresolved Student Tier 2 contents out of the permanent Task Row topology assertion.
- After the audit follow-up, the consolidated Ribbon contract passed (18/18) and the route contract
  passed (4/4).
- The fixed Instructor rows pass the consolidated Ribbon contract tests (18/18) and route-contract
  tests (4/4). The focused Chromium shell evidence and four-viewport fast UI lane pass. Matched
  laptop/phone route captures retain 40px/44px Task Row bounds across all measured transitions.
- `source ./source_me.sh && python3 local_stack.py acceptance` passed the database baseline,
  installation-data replay, and Course Appearance PostgreSQL/MinIO coherence oracles. A fresh
  canonical screenshot publication completed all scenarios and privacy checks; static manifest
  verification and the screenshot publication tests (9/9) passed.
- `source ./source_me.sh && ./launchers/run_fast_checks.sh` and `all_test.sh` each reached the
  Python suite with 9,279 passed and one failure: the local
  `ribbon_route_scope_audit_2026-09-23.md` links to a missing
  `docs/screenshots/instructor/average.png`. Targeted Markdown-link checks for the changed guidance,
  decision, model, and plan documents passed (27/27); the audit was preserved.
- After the test-surface consolidation, `source source_me.sh && ./launchers/run_fast_ui_checks.sh`
  and the focused browser-scenario contract tests passed; `git diff --check` was clean.
- In the later test-review snapshot, the focused UI lane, five browser-scenario pytest tests,
  formatting, and `git diff --check` passed. The aggregate `run_fast_checks.sh` attempt exited 1
  because sandbox permissions blocked Chromium startup for the unrelated
  `course_instance_assessment_refresh.mjs` test; that test passed when run alone with browser
  launch permission. No aggregate fast-check pass is claimed for this later snapshot.
- The Assessment creation CSS change passed its focused Prettier and whitespace checks; the shared
  PageFrame reading-width rule remains the page-level width source.
- `source ./source_me.sh && ./launchers/run_fast_ui_checks.sh` passed the RecordList four-viewport,
  state, presentation, reorder, window, and production-route Chromium checks after removing the
  Assessment Questions track override.
- `source ./source_me.sh && ./launchers/run_fast_ui_checks.sh` passed the shared RecordList and
  production-route Chromium checks. `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed
  9,289 Python tests and 456 Node tests plus Rust, TypeScript, lint, and formatting. The default
  sandbox's first attempt failed only when Chromium could not bootstrap; the same command passed
  with browser-launch permission.
- A production-shell Gradebook capture compared the synthetic PageFrame error-content state at
  1280x800: the centered 291px wrapper at x=495 now remains 768px wide at x=32, aligned with the
  PageFrame title. Fresh visual review found no overflow or clipping. The focused Chromium UI lane
  and fast compliance gate passed; this is not live route-failure acceptance.
- After the PageFrame error-content alignment and shell ErrorBoundary changes, the focused Chromium
  UI lane and `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed (9,287 pytest tests
  plus Rust, TypeScript, Node, lint, and format checks).
- The Student Attempt PageFrame migration no longer carries the old empty title spacer or nested
  title style in its timer row. `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed
  9,289 pytest tests, 456 Node tests, Rust, TypeScript, lint, and formatting checks.
- Student Course landing and Grades now use the Course long name as the PageFrame title; the
  optional Course banner remains in content and Grades stays a subordinate task heading. Updated
  the Student Grades screenshot-corpus selector. The isolated landing browser check and four-size
  visual review passed; no full-stack Grades capture or full-suite run was repeated.
- The shared Course breadcrumb now uses the long name when the trail fits and the Instructor-defined
  short name when space is constrained. The production-shell Chromium fixture and fresh visual
  review passed at 1280px, 320x640/200% root text, and 393x852/200% root text; the PageFrame heading
  retains the long Course name. Fixtures reuse the production shell and components with synthetic
  route/API/content data, so this is not full-stack acceptance. The 320px/200% Instructor stress
  capture also showed crowded Ribbon controls, outside this plan's Instructor screenshot policy.
- Keep the descriptive Course identity sample local to the production-shell harness; shared route and
  model fixtures retain generic Course labels. The fit-based breadcrumb fallback measures actual
  content width through the shell's existing `ResizeObserver`, avoiding a viewport breakpoint.
- After the Course identity fixture and breadcrumb changes,
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed 9,287 pytest tests plus Rust,
  TypeScript, Node, lint, and formatting checks. The command was rerun with Chromium launch access
  after the sandbox denied the first browser-backed attempt. No `all_test.sh` rerun was made.
- WP-F formatter reuse now passes a focused four-test suite: date helpers accept a prebuilt
  formatter, and component/page scopes reuse it across repeated rows and recovery entries. An
  independent source review approved the change. On 2026-09-23, the focused presentation test,
  `npx tsc --noEmit`, six-file Prettier check, and `git diff --check` passed. The final
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed 9,268 pytest tests plus Rust,
  TypeScript, Node (456 passed), lint, and formatting checks after both final code changes.
- SUI-03's fresh headless review used the production ApplicationShell, AppRibbon,
  BreadcrumbPrelude, PageFrame, and shared styles with synthetic route data and body content. At
  393x852 and 200% root text, Home and full Biochemistry are readable at rest; keyboard focus
  brings each full breadcrumb label into view. There was no document horizontal overflow or page/
  console error. A fresh source reviewer approved the scrollport and image_evaluator passed the
  documented gate. The visible leaf label is partial at rest; this is minor. These artifacts are
  bounded shell evidence, not a full Assessment-page screenshot. The aggregate 9,268-test fast
  check above ran after the final source edits; it does not replace or update earlier full-suite or
  Live Demo replay receipts.
- Current source has no new permanent mutation test. Headless `./launchers/run_fast_ui_checks.sh`
  passed after narrow Chromium sandbox escalation; `source ./source_me.sh &&
./launchers/run_fast_checks.sh` passed 9,267 pytest tests plus Rust, TypeScript, Node, lint, and
  format checks. Screenshot corpus tests passed 9/9, static screenshot verification passed, and
  the 139-capture WP-G1 visual review found no in-scope visual fix. The blank Student WebWork
  preview remains an unconfirmed separate-owner candidate; the crowded Sysadmin account form is
  deferred low-priority cleanup. Screenshots do not establish keyboard or runtime behavior.
- The latest clean semantic replay passed workflow, manifest closure, privacy, dimensions, and
  artifact integrity with 135 byte differences; the warm loop passed in 378 seconds with 112
  observational byte differences. Byte differences are not failure gates; no pixel or byte
  thresholds were added.
- PageFrame evaluates allowed content classes reactively, and the fast UI route check verifies that
  entering and leaving Question Pool review updates the production Library presentation while
  retaining its windowed rows, order, regions, and spacer checks. The headed registry can open
  PageFrame reading and Student Coursework production fixtures optionally, with headless completion
  autonomous. `./launchers/run_fast_ui_checks.sh` exited 0; the Student helper headless smoke had no
  page or console errors, headed selected cases reached ready state, and Node syntax, Prettier, and
  `git diff --check` passed.
- The official pre-change Instructor capture showed the Assessment Student View body ending before
  the full-width PageFrame header. After removing the inner cap, a one-time stack-free production-
  shell probe rendered the real fullWidth PageFrame with the existing
  `assessment-workspace-student-view` content class and measured frame, header, and content widths
  equal at 1216px. This probes the production shell/Frame with feature content, not the full
  Assessment API route or a post-change Live Demo screenshot. The isolated Chromium lane and current
  fast compliance gate passed separately.

## 2026-09-22

### Behavior or Interface Changes

- The Student course short name no longer appears in the top Ribbon row; the full course title
  remains the page heading.
- PageFrame keeps a fixed production root, content origin, and route-owned width mode outside Ribbon
  navigation. Caller-specific styles are confined to the content region; the redundant private
  `.auth-page`/`.roster-page` width is removed, and SignInPage no longer passes the unused
  `auth-page` marker. The shared action slot is limited to recurring page-level collection actions
  admitted by WP-C1.

### Fixes and Maintenance

- Tightened the Student responsive fixes after code review: phone-height Ribbon tokens no longer
  leak onto coarse-pointer tablets, compact breadcrumb behavior covers the 393px phone corpus,
  the Assessment Type history adapter parses its closed value once, and the Attempt context
  comments describe the UUID-bearing browser contract accurately.
- M1 Shell contract: tier-one Ribbon destinations now follow Product Role, while signed-in shell
  rows retain stable height across route scope.
- M2 Shell checks: the role-only tier-one and declared task-row topology are covered by focused
  contract checks.
- M3 Shared components: `PageFrame`, `RecordList`, and the named date-formatting boundary provide
  the shared UI composition layer.
- M4 List options: presentation, reorder, and windowing compose outside `RecordList`; the
  six-site reorder comparison keeps shared mechanics separate from caller-owned workflow policy.
- M5 Seven-page proof: the proof set covers scan, dense, windowed, reorderable, action-bearing,
  Student, and gallery/list record patterns.
- M6 Page sweep: page frames and named date formatting replace per-page heading and date layout
  duplication across the swept page groups.

### Developer Tests and Notes

- 2026-09-22: Visual review of the synthetic M1 WP-A5 shell fixtures passed for the planned roles
  and viewports; corrected Student overview captures show the Course and Assessment breadcrumb.
  The focused Ribbon contract test passed (19/19). Attempt ancestry in the initial structural
  fixture was incomplete because resolved relationship scope was absent; source and contract
  evidence confirm production supplies it. No CSS change was warranted. Evidence is in
  `test-results/m1-wpa5-fixture-shell-20260922/` and
  `test-results/m1-wpa5-student-overview-20260922/`; these are not Live Demo proof. The M1 full
  suite remains pending.
- Plan execution now treats headed Chromium as an optional debugging mode. Required visual review
  consumes saved production-shell and screenshot-corpus artifacts through a fresh image-evaluator
  subagent; plan checkpoints and follow-up decisions are manager/agent-owned.
- Four-size visual inspection passed; `./launchers/run_fast_ui_checks.sh` and
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed (9,266 tests).
- 2026-09-22: After the PageFrame formatting-only normalization,
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed 9,266 Python tests in 7.70 seconds.
- Implemented the M1--M6 UI foundations: role-based tier-one Ribbon destinations and stable shell
  rows; focused shell checks; shared `PageFrame`, `RecordList`, and date formatting; caller-composed
  list presentation, reorder, and windowing; seven record-pattern proofs; and the page-frame/date
  formatting sweep.
- Fast UI developer lane passed: the shared production browser environment source is consumed by
  production and full-environment harnesses; isolated Chromium (`./launchers/run_fast_ui_checks.sh`
  and its documented headed single-case form) exercises the production shell/`PageFrame`,
  `RecordList` primitive states/options, avatar Gallery/List, Gradebook/Library route compositions,
  and Student Coursework. After the final phone action inset, the full isolated Chromium lane passed
  with its canonical viewport checks. An earlier four-size Coursework visual review found no
  responsive overflow; a follow-up 393px visual review confirmed the inset and hidden fixture route
  diagnostic. Overview-unavailable evidence is concise and state-oriented,
  and computed font-family checks retain the Atkinson Next/Mono contract while font response/loading
  assertions were removed; production build-order checks no longer inspect raw font-family output.
  `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed (9,266
  Python tests plus Rust/TypeScript/lint/format gates). No backend stack, Live Demo, or screenshot
  publication ran; the compact full-stack WP-5 parity sample remains pending.
- A fresh screenshot publication after the final UI code provides direct Student laptop, tablet,
  phone, and square captures plus direct Instructor and Sysadmin laptop captures; Public scope is
  unchanged. `--verify-static`, the screenshot-corpus Node tests (13/13), and the atlas Markdown-link
  test passed. `source ./source_me.sh && ./launchers/run_fast_checks.sh` passed its offline aggregate
  checks, including 9,201 Python tests.
- One clean `source ./source_me.sh && ./devel/capture_screenshots.sh --fresh --verify` replay passed
  semantic workflow, manifest closure, scenario privacy, dimensions, and published-artifact
  integrity. It retained 130 byte-different replay PNGs in `test-results/screenshot-corpus/verify/`;
  visible IDs and dates can vary across clean resets, so those byte differences are observational,
  not a screenshot-count or failure gate. The full `all_test.sh` integration receipt is intentionally
  unrun and incomplete after an exit-130 interruption during its offline typecheck/lint phase, before
  `local_stack.py acceptance`; the warm-loop runner is also unrun. The clean M7 semantic replay is
  passed, but full plan acceptance remains pending.
- After the PageFrame/shell boundary tightening, the isolated Chromium lane passed and the offline
  aggregate fast checks passed (9,266 Python tests plus Rust, TypeScript, lint, and format gates).
  Visual review of the current-production Instructor harness confirmed one Ribbon, a reading-width
  CourseList, and full-width Gradebook. Student narrow captures came from the route-only M6 fixture,
  so they do not establish Student shell parity; the full-stack parity sample remains pending.
- M6 ownership/CSS audits confirmed WP-E4 and WP-E7 boundaries; legacy workspace rule names are
  reported as cleanup candidates and were not deleted. Avatar picker CSS had no safe cleanup.
- WP-E5 course-instance assessment refresh browser assertions live under `tests/playwright/`;
  the Node test-discovery entry imports that browser test without importing Playwright.
