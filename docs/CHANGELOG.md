# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> October 11 entries are archived in [CHANGELOG-2026-10e.md](CHANGELOG-2026-10e.md).
> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-18

### Fixes and Maintenance

- Kept validated Library collection responses typed through shared projection, removing duplicate
  page decoding; consolidated Draft list and delete operations in the renamed Draft API client.
- Clarified that every Blueprint Course is Public to Instructors, while Active and Archived
  describe lifecycle. New and forked Blueprints start Active; Archived remains visible and
  forkable, but read-only, outside normal discovery, and unavailable for adoption.
- Simplified shared collection setup: `createSearchState` is the sole reactive controller,
  `SearchBlock` adapts its lifecycle, and `DataSheetBlock` owns controls and presentation. Local
  searches return one state plus fields and query binding; one Library assembly serves its three
  consumers. Removed the unused query kind and kept reorder bindings on stable IDs while preserving
  page and domain behavior.
- Grouped shared PageFrame, DataSheet, and SearchBlock code under named component directories;
  moved the Pool member sort helper beside its caller and folded DataSheet controls into their
  owner. Updated imports and current architecture links without changing collection behavior.
- Moved Blueprint discovery and Question Library workflows into their feature owners. The four
  Library page compositions own PageFrame and route identity; APIs and route behavior remain
  unchanged. Updated current architecture links.
- Canonical fast checks pass after the ownership moves (9,427 tests plus Rust, TypeScript, lint,
  and formatting). Maintained Blueprint and Library runtime captures pass across eleven local
  screenshots; no screenshot artifact was published.
- Deleted the remaining 74 one-time TEMP source/probe files and 3 runner-only configs while
  retaining useful evidence and established test workflows.

## 2026-10-17

### Behavior or Interface Changes

- Completed shared collection integration across Library, Pools, Assessments, Blueprint editors,
  personal/Course/Student views and remaining consumers, with separate accepted evidence for
  numbered full-query paging, independent modes/filters, exact revision pins, real stale-edit
  conflicts, dirty-draft recovery, access concealment and native/keyboard reorder/transfer.
  All 37 approved milestones are verified. Accepted receipts and proof limits are recorded in
  [datasheet_collection_status.md](active_plans/reports/datasheet_collection_status.md#current-recovery-checkpoint---2026-10-17).
- Connected Native math/image preview and immutable seed-zero thumbnail publication/correction,
  private MathJax capture, importer/backfill and zero-capture browsing. Actual missing-license
  HTTP 422 returned no artifact bytes; renderer HTTP 503 preserved the Draft and same-container
  recovery passed. These lifecycle results also passed the final runtime gates.
- Preserved protected Course Blueprint Entry identity through updates and grade changes; source-fork
  saves now tolerate an advanced source revision while retaining immutable origin, parent identity,
  authorization, locking and current-revision/Edit-CAS checks.
- Corrected MC/MA authoring validation at canonical indexed choice paths while preserving semantic
  answer identity and actionable MA guidance. Focused TypeScript/lint/format and 13 isolated mapping
  checks passed; actual C62 authoring interaction, failed-save Stay/Discard and reload passed.
- Composed the ordered Assessment heading and Bloom reorder action in a naturally wrapping header;
  the action names its draft-order effect and retains availability feedback.
- Completed Instructor, Student and Admin content-first layout acceptance, including
  default-closed filters, on-demand ID entry and retained failed input. Accepted narrow evidence
  exposes seven rows initially. Current Student state captures, Instructor task checkpoints,
  desktop/theme/zoom review and phone Add/Remove/reorder/save/reload passed.

### Fixes and Maintenance

- Removed unused collection facades, reorder/controls/style artifacts, picker/query code, dead helpers
  and result-cursor client routes; consolidated numbered sizing while retaining live provider/facet/
  native cursors. Focused checks and bounded source SPEC/QUALITY passed. Requested decoder and
  Instructor workflow splits passed. The lifecycle split passed with canonical CLI imports, the M19
  TEMP contract, pyflakes, and eight one-time hygiene checks. One-time cleanup retired the 631-line
  fixture producer and six production-file plumbing paths while retaining real backfill. Six fresh
  growth-audit passes completed.
- Simplified shared local sources across 38 consumers, removing 23 manufactured Resources while
  retaining real loading, error and refresh behavior. Corrected fresh SPEC and distinct QUALITY passed; ROOT
  accepted the bounded cleanup. Evidence: `tests/_temp/datasheet/growth_cleanup/local_source_evidence.md`.
- Consolidated DataSheet evidence documentation after corrected fresh SPEC and distinct QUALITY;
  retained original failed review receipts and all 37 requirements. Aligned current architecture
  guides with implemented four-mode owners, numbered paging, reactive complete sources and accepted
  thumbnail publication. Final architecture documentation SPEC/QUALITY and paging-scope SPEC/QUALITY
  passed. Published-preview source repair passed separate SPEC/QUALITY and focused
  TypeScript/lint/format checks. Startup handle 1595 exited 0 and produced Demo `https://localhost:8351`;
  subsequent current-client spatial captures and acceptance passed. Integrated
  journey reconciliation and current native recovery passed.
- Repaired retained-work retry: retrying one document remounts that iframe; whole-History retry evicts
  its exact router query cache key. Both actual retry paths passed, preserving unaffected native
  question/response DOM, rows, score, query and mode through HTTP 503-to-200 recovery.
- Corrected shared Search CSS containment after paired runtime diagnosis; C16 modes, save, navigation
  guards and real 412 recovery passed. Browse wide/narrow four-mode layouts passed without overflow.
- Fixed shared Compact reorder controls to keep titles readable at fine and coarse sizes, and made
  deleted-file test discovery check existence before reading. Maintained Student screenshots now
  verify state without requiring the responsive-hidden Status column and navigate global Coursework
  with composite Course:Assessment IDs. The canonical fast check passed (9,413 pytest); actual phone
  Add/Remove/reorder/save/reload and Student capture7 passed.
- Added redacted failure-only StudentStart/presentation SQLx diagnostics, corrected generated-graph
  image CORP and clarified the API image contract. Tightened modular UI product guidance and TEMP
  discovery policy without adding permanent tests or new completion gates.
- Consolidated 39 DataSheet reports into the status ledger, consumer inventory and spatial baseline.
  Folded unique behavior/evidence/limitations from 36 reports, removed superseded dispatch chronology
  and repaired incoming links. Inventory counts, unique measurements and all 37 requirements remain.
- Closed the modular UI plan after current fast/full checks, connected and visual acceptance, and
  independent reviews passed. Retired one-time proof executables and configuration while retaining
  the useful receipts, captures and final-gate results.

## 2026-10-16

### Additions and New Features

- Added a shared DataSheetTable for Compact and wrapping List with independent temporary column settings.
- Clarified that Search and Browse share SearchBlock and DataSheetBlock while preserving their distinct goals and layouts, and that source capture preserves rendered Question proportions without raster enlargement.
- Finalized the seven shared collection definitions and ASCII hierarchy, interaction ownership, and square thumbnail crop rules: preserve aspect ratio, center horizontal crops, top-align vertical crops, and capture at sufficient source resolution.
- Added a private Question Thumbnail Renderer to Compose using the existing local lifecycle, Chromium health readiness, and private networking without published ports, volumes, or credentials.
- Extracted shared collection rendering and presentation state from RecordList into the named DataSheet components.

### Behavior or Interface Changes

- Refined Search/Browse guidance and shared collection ownership, including PageFrame layout, search entry, empty states, column behavior, and meaningful unsaved-change confirmation.
- Completed earlier modular UI behavior and interface work across RecordList modes, sorting, filtering, selection, responsive columns, editor guards, pickers, proposals, administration, and student workflows. The earlier completed-plan namespace retains its historical receipts.
- Corrected bulk Published Question metadata authorization at its SQL owner boundary and simplified shared Compact sort/filter controls while preserving the existing authorization and query contracts.
- Reduced the permanent modular UI test burden under the temporary-first policy; retained bounded acceptance evidence and historical validation separately from current gates.

### Fixes and Maintenance

- Corrected source-catalog parsing of multiple foreign keys in one `ALTER TABLE` statement and regenerated schema documentation and the catalog snapshot, including `ple_private.question_revision_thumbnail` and four previously omitted foreign keys.
- Restored keyboard focus after shared DataSheet column reordering by refocusing the moved column's enabled move control.
- Narrowed the shared container build context to the API and thumbnail renderer's required inputs.
- Corrected Library Watch pagination counts, page offsets, and deterministic result ordering.
- Added the shared search-session count guard for numbered pages, including BlueprintWatch.
- Enforced the shared u32 page-number limit in Blueprint history SQL and capped proposal pages at 100.
- Recovered retained-work pagination races by preserving active request intent across retry and refresh.
- Scoped Blueprint view hydration to referenced fixed Question revisions after the confirmed M22 assembly failure.
- Repaired redacted PostgreSQL diagnostics to preserve nested Rust causes and fixed browser-callback local-global linting; fresh specification and quality reviews and 20 one-time helper checks passed. Corrected Human Guidance, active-report, and archive links also passed their separate reviews.
- Historical local verification after the M22 recovery attempt: the six-stage debug build, `./check_codebase.sh`, and full source-bootstrap Python suite passed (9,536 tests). The canonical attempt then failed database initialization with an unknown SQLx cause; later runtime recovery and acceptance are recorded in the current ledger. These are historical results, not current final gates.

## 2026-10-15

### Behavior or Interface Changes

- Reconciled the approved modular UI plan and ledger at the current checkpoint: M1-M18, M20,
  M23-M25, M27, and M29 verified; M19, M21-M22, M26, M28, and M30-M31 active; M32 pending (24/7/1).
  This supersedes the earlier 19/12/1 checkpoint; its earlier milestone receipts and database
  attempt history remain preserved. The sixth canonical-profile owned database run passed
  eight domain oracles plus the M24 helper ACL check. Its first M29 attempt failed because a fixture
  expected-UUID selector included a preexisting row; a reviewed fixture-only retry passed 108 own
  attempts and six complete sort walks with the product fingerprint unchanged. The original failure
  and the canonical cleanup receipts remain in ignored `tests/_temp/`.
- The accepted offline fast gate passed: schema/style, Rust checks and tests, TypeScript/lint/format,
  Node 26/26, and Python 9,213. M18 attempt 12 remains the accepted three-page browser proof.
  The final full gate and post-source current rerun remain M32 work; the full gate shares and resets
  the Live Demo lease, so it follows Phase C runtime completion and cleanup. Remaining post-M18 integration
  and role-denial browser gates, visual acceptance, HTTP cursor checks, and integrated M32
  acceptance remain open.
- M20 private collections passed fresh runtime specification and distinct quality review, the actual
  lifecycle attempt `1792122748285874000`, and failed-autosave Stay/Discard recovery against server
  baseline `1792124467456439000`; it is verified. M21 shared-parser HTTP/UI attempt
  `1792124421868435000` and native Star PUT/persisted row `1792124879399927000` passed; authenticated
  active-Sysadmin denial for M19, M21, and M22 is assigned to Phase C. M22 native correction events
  passed `1792124521491696000`. Fresh `/root/phase_b_remaining_runtime_spec` and distinct
  `/root/phase_b_remaining_runtime_quality` accepted M23, M24, M25, and M27 as verified. Their
  exact attempts, captures, and restoration are in ignored `tests/_temp/modular_ui/phase_b/FINAL_REPORT.md`.
  M25 update review recovered from a real 503 through shared retry to HTTP 200 with 24 rows; its
  native Blueprint advanced to Revision 2 while retaining 24 copied Assessments. The adopted M24
  Course remains Revision 1. M27 numeric timestamp sorting and account-zone date display passed.
- Canonical `cargo tsgen` generated 411 transport types. Main and lint TypeScript, Cargo check, and
  combined strict Clippy pass. Removed the orphan Account Pending Invitation DTO while retaining the
  real `CourseInvitationId` domain. M29 source is SPEC/QUALITY PASS; its sixth owned-database
  oracle passed after the reviewed fixture-only retry, covering 108 attempts and six complete sort
  walks. M29 is verified after actual Phase C attempt
  `tests/_temp/modular_ui/phase_c/m29/attempt_1792126226531424000` and fresh
  `/root/m29_actual_runtime_spec` plus distinct `/root/m29_actual_runtime_quality` PASS. The initial
  invitation attempt `1792126037283976000` imported a roster (201) and claimed its invitation
  (200), then hit a temporary locator fault; the accepted run reused that active enrollment.
  Course `CINRJMFZQR`, Assessment `A0FJ1J468`, and second Course `CIJTEQ3HGY` established
  two-Course identity; absent Score became 1/1 after submit. Global Coursework, Scores, and History,
  native sort/facet, and dark phone at 125% capture passed.
- M26 source implements MyChangeProposalRibbon and flat query/forks/stewardship/history; the sixth
  owned-database discovery oracle passed. SQL/transport decoding for all four names and the v2
  session/query/page-size cursor binding passed fresh specification and distinct quality reviews;
  HTTP/browser proof remains open. M28 shared picker search, typed comparators, exact selected
  revision tuple/order checks through the existing revision service, current metadata eligibility,
  and declined-confirmation retry passed fresh specification and distinct quality reviews after
  all three consumers were inspected. Picker eyebrow, introduction, and current-revision claim
  copy was corrected without logic changes; TypeScript, lint, and format passed. Browser proof
  remains open.
- M30 shared roster/gradebook queries and exact AccountID/Assessment facet behavior pass source
  SPEC/QUALITY. HG 282-284 applies only to Sysadmin FERPA lookup; Instructor recovery requires no
  additional confirmation/audit, and selection persistence is intentional. M30 Admin and Recovery
  SQL oracles passed. M31 Question Use now supplies exact global fixed-revision Course/Assessment
  counts through the SD data owner and the full authorized own-Course array through the API owner;
  generated transport decoding, fresh source reviews, and the owned-database oracle passed.
- M31 Course Blueprint Assessment update review now uses the complete resource and shared local
  search adapter with typed Assessment fields and SearchBlock, retaining source context and review
  links; fresh specification and distinct quality reviews passed. Its current consumer inventory
  found no unexplained flat-collection bypass; bounded points editing, singleton Course detail,
  and the latest-25 Watch excerpt remain explicit semantic exceptions. M32's fresh source
  architecture specification and distinct source/docs quality review passed source composition;
  fresh independent specification and distinct quality re-reviews passed the corrected
  M26/M28/M31/M32 source/docs status. Post-M18
  browser, visual, HTTP cursor, full-suite, and integrated acceptance remain open. Its flat canonical
  `question_pool_id` decoder fix is included. Actual update-review attempt `1792125762434511000`
  passed 24 exact API and DOM rows, both heading sorts, 503/shared retry/200 recovery, modes, and
  actions. Fresh `/root/m31_update_review_header_runtime_spec` and distinct
  `/root/m31_update_review_runtime_quality` PASS accepted that leaf; whole M31 remains active.
- Phase B restored all three registered overlays and released its sole runtime baton. Phase C is
  running six prepared slices on bundle `1d6138fa`; no human gate is required. Navigation documentation
  alignment covers 14 docs and four accepted corrections, with final fresh SPEC/QUALITY
  (`/root/m32_docs_corrected_final_quality`) for source/docs only. The historical 16-file/3,611-line
  pruning checkpoint remains accepted; one-time evidence stays in ignored `tests/_temp/` without a test quota.
  M28 recovery and new sort-only actual attempt `1792126606160498000` passed the owner gate; final
  fresh reviews remain pending, so M28 stays active.

- Search Blueprint Courses now includes Institution in its shared record fields, and the visible
  Ribbon label, route surface, breadcrumb, and screenshot scenario use the Human Guidance wording.
- Shared RecordTable scrolls a newly focused control fully into view within horizontal overflow,
  so native Tab reaches clipped actions on narrow screens. TypeScript, focused ESLint, and
  Prettier passed; the three-page browser probe remains the runtime acceptance gate.
- Corrected Library empty-state copy: Browse points to Discipline or column filters, My Questions
  has no initial empty-state message, and completed empty results retain only their filter-specific
  title. Removed the repeated Browse classification heading and show Browse handoff actions only
  after criteria are selected.
- Compact RecordList now names the active sort in the closed Sort control when its column is
  hidden or the order has no column; visible sorted headings retain the concise Sort label.
- Completed the focused PageFrame contract: content, actions, and layout now render without the
  former heading props/markup, and breadcrumb identity reaches the named main. Fresh source
  specification and quality checks passed; four browser scenarios, 20 captures, and focused
  authorization 1/1 are recorded in `tests/_temp/modular_ui/m2/proof_receipt.md`. Full integrated
  acceptance remains M32 work.
- Completed M11 at the shared RecordList boundary. RecordList owns local Compact, List, and Visual
  boxes modes and the common selection toolbar across direct, search, and avatar callers; keyed
  body DOM stays mounted across active presentations without refetching. Guarded Library bulk
  callbacks and domain editors remain caller-owned. Removed duplicate SearchState display state
  and an unused presentation helper, and corrected List checkbox CSS scope. Fresh spec and quality
  reviews, 36 focused owner checks, TypeScript, lint, and Prettier passed; RecordList is 714 lines.
- Completed M12 multi-value contracts. Fresh specification and quality reviews, strict Rust gates,
  and the static SQL oracle passed. The canonical database baseline E2E passed against actual PLE
  Instructor and Student sessions, covering array OR/AND, classification and Bloom hierarchy,
  whole-category counts, keyset paging, owner/public access, Student denial, and rollback. A fixture
  helper failure was repaired with direct named calls and NULL-safe assertions without relaxing
  permissions.
- Completed M17 Library editor protection after adding four actual QuestionBulkMetadataEditor
  harness cases for the identified coverage gap. Fresh quality re-review and 20 shared/owner cases
  passed; the prior Pool, Picker, and query-controls set had 31 focused cases passing. TypeScript,
  lint, and Prettier passed. Three-page M18 acceptance remains pending.
- Added the typed SearchBlock composition for Library and the embedded Question picker. Required
  fields and query feed one mounted RecordList, with controls explicitly above or beside it.
  SearchSession eligibility now keeps an empty Search spreadsheet until a keyword or filter is
  present, while predefined scope can load and reload after Clear. Fresh specification and
  quality reviews passed, with a maintained component test and four independent Node checks.
- Accepted RecordList column visibility and responsive behavior at the focused component gate.
  Auto/show/hide state lasts for the current view, canonical identity and actions remain
  accessible, and a 14rem actions budget fits the responsive layout. Fresh specification and
  quality reviews plus focused 740px, root-font, overflow, and resize browser checks passed.
  Three-page Library integration remains pending.
- Added the stable typed field/query contract for RecordList: field IDs, raw scalar values,
  read-only arrays, nulls, rendering, typed sort/filter keys, and default columns. RecordList
  remains paired with fields and query while preserving existing actions, selection, and keyed
  models. The focused type test, main and lint TypeScript checks, and focused ESLint/Prettier
  checks passed at handoff; quality review also passed lint and TypeScript checks. Its later main
  check overlapped active M12 work and is not evidence of an M4 failure. Browser behavior remains
  pending.
- Accepted Dense Compact at the focused component boundary after a 19.8 CSS-pixel probe, shared
  selection CSS correction, and maintained tests. Three-page integration remains pending.
- Accepted column resizing at the focused component boundary after component spec and quality
  review of pointer/keyboard width adjustment and state preservation. Three-page integration
  remains pending.
- Accepted RecordList column rearrangement: headings move by drag, keyboard, or touch while
  identity stays movable and actions stay last. Hidden order, widths, editor DOM, selection, and
  record order persist. Fresh specification/quality, TypeScript, lint, Prettier, ASCII, and
  focused browser/bot proof passed; M18/M32 integration remains pending.
- Completed the shared unsaved-change guard for optional saves, pending writes, manual actions,
  and multiple editors. Blueprint detail keeps content, names, and Theme drafts independent;
  refresh and structure discard preserve unrelated drafts, and owner-merged classification,
  archive, and restore actions remain available. Final specification/quality and 25 component
  browser scenarios passed; M18/M32 integration remains pending.
- Clarified Search Question Library guidance: open an empty spreadsheet with column headings,
  keyword search, and a concise prompt; require a keyword or filter to show results. Clearing
  all criteria returns to the empty spreadsheet. The focused Human Guidance formatting check passed.
- Clarified that Search and Browse share RecordList's column filters, sorting, display modes,
  and record actions. Search uses full-width results with more visible columns; Browse adds a
  compact classification sidebar, leaving narrower results with fewer visible columns.
- Recorded that RecordList column visibility and arrangement last only for the current view
  and reset to defaults when reopened. The focused Human Guidance formatting check passed.
- Made responsive column hiding, temporary column configurations, whole-result column sorting,
  and combined column filters explicit shared RecordList features, including filtering hidden
  columns. Renamed the guidance section to Shared Question Library spreadsheet and updated its
  specification link. All three focused formatting/link checks passed.
- Set Google Sheets as the reference for RecordList column resizing, hide/show, and filtering
  capabilities. Column widths also reset when the view reopens. The focused formatting check passed.
- Organized Human Guidance into shared RecordList and PageFrame interface sections. Moved common
  collection mechanics out of Question Library guidance while retaining its classification,
  Search, and Browse workflows. Navigation guidance now reserves confirmation for meaningful
  unsaved edits. Aligned the search specification and filter link; all four focused formatting
  and link checks passed.

### Fixes and Maintenance

- Renamed the active 37-milestone plan to `modular_ui_architecture_completion_plan.md`, updated its active links, and clarified that M19 runs as a separate process using the
  repository-root npm manifests.
- Repaired bulk Published Question metadata saves at their SQL owner boundary: the private
  `SECURITY DEFINER` function and its grants now use `ple_private_owner`, while the public wrapper
  keeps `ple_api_owner`. Each locked target must belong to the active Instructor before any batch
  write; missing, archived, and other-owned targets share the same refusal. Fresh disposable
  database proof passed for the role/grant split, owner save, nonowner and mixed-owner refusal,
  stale Edit Number, missing and archived targets, and unchanged rows after refusals. SQL style
  passed; Live Demo rebuild and three-page M18 acceptance remain separate.
- Reduced repeated controls in shared RecordList Compact mode: the full sort menu and filter-field
  toolbar now open through native Sort and Filters disclosures. The Filters summary counts selected
  values, Clear column filters stays visible when active, and the common dialog remains available
  for hidden columns. List and Visual boxes retain their full controls. Both TypeScript checks,
  focused ESLint, and Prettier passed; three-page browser acceptance remains pending.
- Let shared RecordList Compact titles shrink beside selection controls, keeping narrow Title
  cells on one dense line while full-width editor bodies still wrap below. Source inspection and
  focused Prettier passed; browser remeasurement remains pending.
- Reconciled the Question Library search specification with the nine typed Library sort orders,
  v4 result-cursor binding, read-only facet choices, shared SearchBlock/RecordList structure,
  empty Search entry, and editor-owned unsaved-change protection. This is documentation
  alignment; M18 three-page browser and visual acceptance remains pending.
- Verified M13-M15 at the owned-database boundary. `tests/_temp/m13_m15_connected_current.log`
  exited 0 with selected-choice and whole-scope M13 assertions, M14 Kind facet/counts, and the
  M15 nine-sort oracle. The evidence covered 70 choices, 40 selected values, zero off-page
  leakage, owner-field exclusion, search continuation, role denial, and nine sorts over seven IDs
  across at least three pages. This proves backend, typed API, and database behavior; HTTP
  opaque-cursor and browser behavior remain open under M18.
- Accepted M18 source repairs after fresh specification/quality review: shared guarded facet updates
  queue intent before async work; the long-lived Review guard preserves six baseline/discard fields
  through source edits and autosave; Native failed-publish saved-state logic passed fresh review, and
  backend author-link and parent-GET changes passed fresh review, 18 tests, and Clippy. Temporary
  checks passed, including 26 shared facet/guard cases. The temporary Native/Webwork wrapper prep
  remains unlaunched pending its own fresh reviews. The genuine canonical
  Priya + question metadata + auth journey still needs its reversible three-file overlay, final
  hash restoration, real Search/Browse/My Questions behavior, and independent architecture/visual
  review. M18 remains active; M3 role-render proof and M19-M32 remain open.

- Reduced modular UI permanent-test burden under the temporary-first policy. Cumulative pruning removed
  17 permanent files and 3,698 physical lines: five Playwright specs (1,610 lines), seven harnesses plus an
  esbuild loader (1,397 lines), Node activation/type proof (209 lines), and the connected SQL
  oracle (395 lines). The added browser-dispatch mechanism and connected invocation were removed;
  existing real-stack scenarios, older tests, policy gates, and PageFrame selectors/migrations
  remain. Earlier passes remain historical evidence, not current maintained-test counts; no new
  permanent-test quota was added, and one-time proof remains in ignored `tests/_temp/`. The two later pure
  guard cases were also removed because one-time native proof already covers their behavior; Node is restored to
  its 24-case baseline. Both TypeScript checks, ASCII/whitespace 3,202, shell syntax, controller compilation,
  reference scan, and naming 5/5 passed; naming found 12 non-temporary candidates and zero
  temporary candidates. Fresh specification and quality re-review passed. The permanent
  `e2e_unrelease_connected.sh` remains the permanent runner and reads the existing tracked
  `tests/e2e/unrelease_connected_oracle.sql`; no `tests/_temp` dependency remains, and the three
  existing registered real-stack specs remain. M12's canonical connected pass remains accepted
  historical evidence. M13's deeper SQL fixture initially chose a nonvisible Question; correcting
  the fixture to use a currently visible Question addressed a test setup mismatch, not a product
  failure. The choice cap is 64, with ignored temporary decoder probes passing at 64, 65, and
  malformed input. M14 now projects classification UUID/name pairs on both mixed result arms while
  leaving generic `QuestionSummary` unchanged; its real full-query Kind counts, own-item exclusion,
  selected-zero choices, all four hierarchy facets, type/metadata/capability filters, descendant
  retention, and same-query choice counts passed temporary probes and fresh source reviews. The
  prerequisite dialog review moved from 5/5 findings to a complete Clear criterion and 6/6 after
  correction. M15 defines nine backend orders and v4 cursors, with source gates and temporary
  stale-sort/filter probes passing; its three Library sort controls use the full query and the old
  dropdown is removed. Fresh Kind/cap and combined specification/quality reviews passed. The
  combined owned-database session 6042 was lost when the terminal tool aborted (`Unknown process`);
  zero Podman containers remained and exact stdout is unavailable. It supplies neither a pass nor
  a known SQL failure. A temporary static privilege fixture was corrected to use GUC JSON, and
  fresh review found a future Pool member timestamp CHECK issue; the owner is setting `updated_at`
  to `pool_time`. A new M13 profile launch has been released; its result is pending owner evidence.
- Prepared M18's three-page source and browser proof. My Questions uses the Library wrapper with
  authorship scope, Kind clamp, autoload, and Clear. Fresh review found that native publication
  left authors unlinked, so an authored, nonowned Question could be absent. The owning repair takes
  an explicit 0-based `selfAuthorPosition` wire field (Rust `self_author_position`) or None,
  links only the chosen validated native-source name to the authenticated AccountId using the
  canonical U-prefixed `ple_data.account_id` domain. Forks and revisions preserve existing links;
  existing feedback GET supplies a parent tuple without a full Draft-list fetch. Backend Rustfmt,
  Rust check, 18 tests, Clippy, and 23-arity/4-grant source gates passed. Fresh quality review then
  found name validation after object I/O; the bounded correction is active. A long-lived Review
  guard owner candidate passed TypeScript
  and a temporary model probe, with fresh reviews pending. Two owners are repairing shared
  parent/child filter intent and running temporary checks. Browse facet source review passed. Seven
  existing capture-workflow selectors were migrated in two files with TypeScript, lint, and
  formatting passing; reviews remain active. The canonical Priya authored-nonowned journey and
  native None/linked publication probes
  are prepared only. Real-app journeys, visual review, and M32 acceptance remain pending.
- Combined overlapping SearchBlock bullets in Human Guidance while preserving its required
  RecordList relationship and shared responsibilities. `git diff --check` passed.
- Made SearchBlock and its required RecordList the intended structure for searchable collections,
  including personal Question collections. Pages supply criteria and controls; PageFrame supplies
  page layout. Updated Human Guidance, the UI guide, and architecture recommendations. Three
  focused link checks and `git diff --check` passed; application code remains unchanged.
- Clarified that navigation and editor-closing confirmation depends on discarding meaningful
  unsaved edits. Search terms, filters, results, and opening an editor do not qualify. Aligned
  Human Guidance and the UI guide; both focused link checks and `git diff --check` passed.
- Reviewed shared unsaved-change protection and its editor callers. Documented Pool creation's
  proposed Stay/Discard behavior, metadata Save/Stay/Discard behavior, local close/replacement
  handling, and pending-save limits. Added intended SearchBlock guidance after reviewing all
  SearchPage responsibilities, preserving page-owned framing and dialogs plus shared RecordList
  behavior. Documentation only; three focused link checks and `git diff --check` passed.
- Refined PageFrame wording in Human Guidance and the UI guide to state its layout responsibilities
  and assign page identification to the Ribbon and breadcrumbs. `git diff --check` passed.
- Audited SearchPage layout and leave protection from source. Documented state-driven sidebar
  layout, missing Browse sort/display controls, search-only guard inputs, and incidental rather
  than edit-aware protection for Library editors. Recommended explicit layouts with one shared
  RecordList and editor-owned unsaved-change protection. Aligned Human Guidance and the UI guide.
  Three focused Markdown link checks and `git diff --check` passed. No application code changed;
  browser behavior was not tested.
- Updated PageFrame guidance to provide layout, spacing, actions, and content without page
  headings, short labels, or introductory text. Aligned the UI design guide's heading rules.
  This documentation-only change leaves implementation unchanged. Both focused Markdown link
  checks and `git diff --check` passed.
- Replaced the broad Google Sheets comparison in Human Guidance and the UI design guide with
  explicit column edge resizing, individual hide/show, heading dragging, and available-value
  filtering. Retained shared sorting, combined filters, responsive behavior, and temporary
  configurations. Both focused Markdown link checks and `git diff --check` passed.
- Reviewed Human Guidance's shared RecordList, PageFrame, and Search/Browse navigation rules.
  Clarified plural Pool creation wording and that default columns may evolve with use and screen
  width while view configurations remain temporary. The focused Human Guidance link check and
  `git diff --check` passed.
- Refocused Question specifications on formats, import/export, stored data, lifecycle and Draft
  workflow transitions, query semantics, and API contracts. Moved web layouts, shared RecordList
  and PageFrame behavior, discovery controls, and MATCH interaction guidance into the UI design
  guide. Updated the spec index and references; documented current search implementation gaps
  separately. All 28 scoped Markdown link checks passed. A broader keyword selection also found
  four existing broken source links in two archived Question-spec reports; those remain unchanged.

- Moved the shared BPW curriculum importer from `tools/ple_import/` into the root-level
  `ple_import/` application package, with absolute package imports and a thin
  `launchers/import_bpw.py` entry point. Updated the host installer, usage links, and directory
  map. Verified launcher help from the checkout and another directory, package discovery of
  bundled content, Pyflakes, and 3,570 focused import-boundary, dependency, shebang, and Markdown
  checks. Direct syntax, import-boundary, shebang, and ASCII checks also covered the new files.
  A live curriculum import was not repeated for this placement change.
