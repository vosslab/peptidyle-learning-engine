# Shared search implementation

Source plan: [read-docs-active-plans-active-shared-sea-peaceful-mountain.md](../read-docs-active-plans-active-shared-sea-peaceful-mountain.md).
Product scope and remaining handoff: [SHARED_SEARCH_PAGE_SCHEMATIC.md](../active/SHARED_SEARCH_PAGE_SCHEMATIC.md).

## Status

Implementation ran October 4-5, 2026. The milestone table records acceptance at each milestone,
including the full M16 run before test pruning and screenshot-scope correction. Subsequent
changes passed the aggregate component gates in separate invocations and a full live screenshot
replay. The [independent drift audit](shared_search_drift_audit_2026_10_05.md) records two sets of
six independent passes, corrected code/workflow findings, and current validation receipts. Optional M17
and the separate Pool validation handoff remain outside this implementation scope.

| Milestone | Status | Evidence or next gate |
| --- | --- | --- |
| M1 Search session | Passed narrow gate | Eight session tests; `check_codebase.sh` passes with 544 Node tests |
| M2 Library session | Passed G | Library definition, submit behavior, old session removed; manager codebase and full fast UI gate pass |
| M3 Shared controls/results | Passed G and review | Bulk-state extraction, Browse visibility, chips, shared controls; rendered filter layout verified |
| M4 Blueprint adoption | Passed G, review, and live capture | Name/Tag submit once; shared display; current-source Blueprint screenshot passed |
| M5 Shared page/Browse | Passed G, live gate, and review | Shared responsive shell; automatic opening/reset semantics; desktop/narrow rendered evidence |
| M6 Pool owner | Passed database proof and review | All fork paths route trusted owner; manager fresh PostgreSQL lifecycle passes |
| M7 Pool author removal | Passed narrow gates and review | Removed obsolete SQL author/attribution contract; current Rust/browser types already aligned |
| M8 Membership rules | Passed B, connected proof, rebuild, and audit review | One Question per Pool, immutable Type/Backend; no content repair required |
| M9 Calculated license | Passed B, connected gates, and review | Exact-Revision calculation, seven combinations, downgrade, fork, and helper privilege checks |
| M10 Combined query | Passed B, connected matrix, live gates, speed, and review | One mixed relation; function-local JIT fix; final HTTP medians 15.6-17.6 ms |
| M11 Library links | Passed G, rebuilt live routes, and review | Shared detail dispatcher; Question and Pool retry paths verified |
| M12 Mixed Library | Passed G, rebuilt live journey, speed, and review | Default chip; all kinds and modes; partitioned selection retention |
| M13 Question picker | Passed G, scoped database proof, browser eligibility, and review | Shared dialog; exact pins and ordered tray; fitting members across Pools |
| M14 Assessment picker | Passed G, rebuilt live import journeys, and review | One picker; exact Question batches; explicit Pool fork import |
| M15 Reviews | Passed, including live picker follow-up | Final dialog surface fix passes refreshed live visual review and full browser/accessibility gate |
| M16 Close-out | Historical pre-pruning acceptance; audit findings open | Complete all_test, browser/accessibility, live Library, parity, full screenshot publication/verification, docs and cleanup pass |

## Starting evidence

- At implementation start, the repository plan matched the originally requested `~/.claude/plans/`
  copy byte for byte. Two relative links were subsequently corrected for its repository location.
- The tracked worktree was clean before this implementation. Existing untracked `.agents/`,
  `skills-lock.json`, and the supplied plan are preserved.
- `source ./source_me.sh && ./check_codebase.sh` passed before implementation: TypeScript,
  lint, formatting, and 536 Node tests. Log: `/tmp/ple_shared_search_baseline.log`.
- `source source_me.sh && python3 local_stack.py status --project ple-live-demo-browser`
  confirms the existing Live Demo is ready. The default `containers` project is absent.

## Decisions and limitations

The manager resolved the in-scope boundaries below while applying the approved plan. The
shared-session and ordered-tray decisions are recorded in
[Design Decisions](../../DESIGN_DECISIONS.md#searches-share-session-behavior); Pool ownership,
membership, license, and combined-search decisions are recorded in its neighboring Pool sections.
No additional product workflow was added.

- Search remains in browser memory and uses the existing content-specific APIs. The session
  owns request ordering and paging; definitions own domain filters and row descriptions.
- Pool mismatch/release behavior, unordered member storage, and the sortable member editor
  remain the separate handoff named by the plan.
- The Question picker's ordered tray remains domain state across search refinement and
  search Clear; its separate Clear selection action and source changes empty the tray.
  The Assessment content picker follows shared query-bound selection clearing.
- Existing Pool eligibility comes from the Pool's own classification, Type, and Backend,
  through the already-authorized Blueprint reader. Member Question reclassification does
  not redefine the Pool. All three supported licenses are compatible with membership.
- Mixed Assessment selection carries the actual Pool membership edit number, distinct from
  the metadata edit number, and preserves the Course's explicit fork-import confirmation.
- Baseline checks prove the starting tree only. They do not certify later edits or completion.
- `docs/SEARCH_PAGE_ARCHITECTURE.md` describes the implemented mixed Library, Blueprint search,
  and both pickers, including the Question picker's separately owned ordered tray.
- Architecture/file-layout docs now identify the implemented shared modules. Human Guidance's
  approved three-display rule and its source/generated checklist match at 1,196 bullets; the
  three-display item is verified by final mixed Library and picker evidence. Final accessibility
  and ephemeral-state probes cover the implemented consumers.

## Milestone evidence

### M1 shared session

- `src/features/search/search_session.ts` owns request numbering, cursor paging, exact retry,
  replacement-query clearing, bounded selection, and disposal. Content definitions supply
  decoding, query cleanup, and one record description.
- `source source_me.sh && node --import tsx --test tests/test_library_classification_search.mjs
  tests/test_search_session.mjs` passed; the subsequent full codebase check includes all eight
  final session tests and passes all 544 Node tests. Clear also invalidates in-flight responses.
- Review during integration identified a failed page-size change retaining obsolete cursors.
  The session now clears that cursor stack at the start of the replacement request and a test
  protects retry/refresh behavior after failure. Fetch error handling excludes UI callback errors.
- Full codebase log: `/tmp/ple_shared_search_m1_m2.log`.

### M2 Library adoption

- Library uses `SearchSession` and a domain definition; typed text applies only on submit.
  Facet counts clear with rows on replacement while selected values stay in controls.
- Updated the Library continuation test, screenshot interactions, live browser interactions,
  and the display harness for the shared snapshot.
- The fast browser gate launched outside the macOS sandbox after a Mach-port denial. RecordList,
  sorting, paging, avatar, and Ribbon checks passed before the Student Course entry test timed
  out looking for `Active Attempt destination`. Its stale same-tab expectation was repaired and
  passed. The next gate run exposed an Assessment delivery harness that failed to supply its
  fixed private Attempt selection to route scope. That harness was corrected; G remains open
  until the manager's complete rerun passes.
- Focused Bloom/Pool-isolation browser tests and a one-time typing-versus-Enter browser check pass.
  Independent M1/M2 review found no blocker; targeted tests passed 22/22 before adding Clear's
  pending-response regression.

### Parallel backend and review evidence

- M6 source, generated contracts, strict decoders, and fixtures now carry Pool owner. An isolated
  PostgreSQL install and saved-response lifecycle run passed the creation/Assessment/Blueprint
  fork owner assertions. Initial Course-adoption and later update proof remain separate gates.
- Independent M6 review caught the later Course Blueprint-update path incorrectly assigning new
  forks to the original Course assignee. That path now uses the applying Instructor, and the
  reviewer confirmed the correction. A distinct co-Instructor database case is still required
  before closing M6; connected Rust tests require `--features postgres`.
- [SHARED_SEARCH_SPEED.md](SHARED_SEARCH_SPEED.md) records the pre-M10 endpoint baseline.
- [SHARED_SEARCH_LICENSE_REVIEW.md](SHARED_SEARCH_LICENSE_REVIEW.md) approves the M9 table as
  Pool collection policy while preserving every member's original license.
- The ready Live Demo supervisor PID 30935 normally holds the browser lease. It is not evidence
  of a stuck test. The isolated PostgreSQL runner provides SQL evidence while baseline/audit
  reads finish; required later rebuilds use the normal Live Demo stop/start lifecycle.
- The manager independently reran `tests/e2e/e2e_assessment_saved_response.sh` (passed), the 454
  Rust library tests (passed), and schema generation/style checks (clean). These gates do not
  substitute for the connected initial-adoption and co-Instructor later-Apply checks.
- [SHARED_SEARCH_POOL_AUDIT.md](SHARED_SEARCH_POOL_AUDIT.md) records the read-only pre-rebuild
  audit: the existing Live Demo contains zero Pools and zero Pool members, so no existing
  Assessment entries require repair. Static fixture candidates remain separate from that fact.
- The connected M6 lifecycle runner exposed duplicate Account IDs in an old fixture. The fixture
  repair and actual initial-adoption/co-Instructor proof are in progress; passing unit tests
  alone do not close those database acceptance cases.
- Connected adoption then exposed an M6 regression: Pool creation read an audit event before it
  was recorded, through a role that cannot read audit tables. The correction passes the trusted
  Instructor identity directly through internal materialization functions. Initial adoption uses
  the validated assigned Instructor; later operations use the authorized acting Instructor.
- The manager's M3 codebase gate passed 544 tests; its browser gate caught Browse rows appearing
  before an exact filter. Independent review confirmed that regression and found bulk-save
  feedback cleared by query-change handling. Both fixes remain part of the M3 acceptance gate.
- Current documentation link and guidance-format checks pass all 397 cases.

### M3 shared controls and results

- The manager's final `check_codebase.sh` and complete `run_fast_ui_checks.sh` pass; logs are
  `/tmp/ple_shared_search_m3_final_code.log` and `/tmp/ple_shared_search_m3_final_ui.log`.
- Independent review confirmed corrected Browse visibility, persistent bulk-save feedback via
  session refresh, and chips for all active filters. Its 25 focused checks pass.
- All five shared search modules import only Solid, shared components, or their own modules.
- The one-time submit check passes against the extracted controls. Screenshot inspection found
  and corrected lost filter-label layout; the final capture is `/tmp/ple_shared_search_m3.png`.
- The old Library row renderer is deleted. Library bulk state, editor rendering, title tracking,
  and completion feedback now live in `library_bulk_actions.tsx`.

### M6 Pool ownership

- Pool owner is required and immutable, with trusted creation and fork identities propagated
  through SQL, Rust, generated contracts, and strict browser decoders.
- Initial adoption passes the assigned Instructor directly through internal materialization;
  later append and Apply use the authenticated actor. Internal grants match the new signatures.
  No audit-table read privilege was added.
- Connected testing found Apply passing a Blueprint ID where `save_assessment` requires a
  Course Instance ID. That call now passes the correct Course Instance ID.
- The manager independently replayed the fresh PostgreSQL lifecycle: one connected test passed
  with initial-adoption ownership and distinct co-Instructor replacement-fork ownership asserted.
  Log: `/tmp/ple_shared_search_m6_connected_manager.log`. The saved-response SQL lifecycle also
  passes on the corrected schema (`/tmp/ple_shared_search_m6_final_sql.log`).
- Final independent review found no blocker. The lifecycle helper split keeps the main test
  under the repository's 1,000-line limit; schema generation/style are clean.

### M4 Blueprint adoption

- Blueprint uses the shared state, controls, and results. Its definition owns query cleanup,
  paging adapter, sorting, chips, and row content; the page holds only auth and domain wiring.
- Name and Tag drafts submit together once. Immediate filters preserve unapplied text. Clear
  returns to idle without a fetch, including an initial Tag-only draft; chip removal updates
  the committed Tag and its draft. Small shared submit/clear/draft hooks support the second field.
- Final manager G passes after the last Tag fix: `/tmp/ple_shared_search_m4_final_code.log`
  (544 Node tests) and `/tmp/ple_shared_search_m4_final_ui.log` (includes both shared browser cases).
  Independent review found no blocker.
- `devel/capture_screenshots.sh --only instructor_public_blueprint_search` passed against the
  rebuilt Live Demo. The manager inspected the actual capture at
  `test-results/screenshot-corpus/staging/instructor/courses-search-filtered_results.webp`.
  Log: `/tmp/ple_shared_search_m4_capture.log`. The ready demo origin is now port 8373.
- M7 source edits wait until M5's frontend-only live gate completes. This avoids invalidating the
  just-built API/migrator images while proving the shared page; the first cold rebuild took
  roughly 21 minutes with the repository's configured single-job container builds.
- The M5 API precheck exposed a stale test assumption that the whole Library contained exactly
  eight Pilot Questions. The repaired API gate separately checks global paging and exact-title
  Pilot contracts, propagates validation failures, and restores archived test data on exit.
  Its `--api` run passes; the complete M5 live gate remains pending the page changes.

### M5 shared page integration

- Library Search, Browse, and Blueprint now use the same responsive `SearchPage` layout and
  leave guard. Automatic URL/Browse opening does not arm the warning; typed drafts and user
  searches do. The Pool task is mounted beside the hidden search wrapper.
- Initial manager G passes with 544 Node tests and three shared browser cases. Independent
  review found Browse Start over still armed the warning; the corrected automatic reset and
  regression now pass independent re-review. The test changes a facet before resetting.
- Live Library and Blueprint captures pass with a client-only bundle refresh and no container
  replacement. The manager inspected the Library filtered capture. The full live API journey
  passes. The browser's fixed 50-row expectation timed out because the actual empty-query page
  contained 49 rows. Its corrected assertion checks a nonempty bounded page; exact-title and
  pagination contracts remain separate. A second failure identified raw DOM selectors requiring
  an explicit region attribute; the shared named section has that role implicitly. Accessible
  locators now pass the complete live API and browser journey.
- Rendered Blueprint inspection found name/Tag controls overlapping the results column. Shared
  toolbar shrink sizing and the scoped Tag input width now fit. Measured input/toolbar right edges
  match at 416 px on a 1280 px viewport and 366 px on a 390 px viewport; neither document overflows.
  The manager inspected the final live desktop capture and narrow harness capture.
- Final manager G passes after the CSS correction (544 Node tests and the full fast UI suite).
  Independent re-review is clean. Logs: `/tmp/ple_shared_search_m5_final_code.log`,
  `/tmp/ple_shared_search_m5_final_ui.log`, `/tmp/ple_shared_search_m5_final_capture.log`, and
  `/tmp/ple_shared_search_m5_final_live.log`. M7 implementation is now released.

### M7 Pool author removal

- Removed the obsolete Pool authorship table, attribution column, related index/policies/grants,
  and author/attribution parameters/results from SQL provenance functions. Updated the existing
  optimistic-concurrency fixture to the reduced signature. Owner, source-Pool links, support
  fields, and member authorship are preserved. Manual license/source text/source URL remain M9 work.
- Current Rust, generated API, browser decoders, and editors already have no separate Pool
  authorship/provenance contract. No redundant replacement interface was introduced. Regenerated
  TypeScript contracts remain unchanged; schema tables and catalog were regenerated.
- Manager gates pass: codebase with 544 Node tests, fresh saved-response PostgreSQL lifecycle,
  schema generation/style, and five focused Rust Pool tests. Logs are
  `/tmp/ple_shared_search_m7_{code,sql,rust}.log`. Independent review is clean after correcting
  `QUESTION_MODEL.md`'s stale Author-storage gap. Documentation links/format pass 398 tests.

### M8 Pool membership rules

- Pool Type and Backend are required, derived from the first exact member Revision, and immutable.
  One unique Pool/Question pair excludes multiple Revisions of the same Question. The central
  member-insert guard protects creation, replacement, and every copied fork membership.
- Pool reads and strict API decoders carry both fields. The existing Rust Blueprint domain
  duplicate check already compares Question identity and did not need replacement.
- Existing direct Pool fixtures were already compatible and needed only the required pair
  populated from their pinned Revision. New fixtures exercise rejection on create and member
  replacement for duplicate Revisions, wrong Type, and wrong Backend, plus fork pair preservation.
- Independent source review is clean. Manager fresh PostgreSQL SQL lifecycle and connected
  initial-adoption/co-Instructor Apply lifecycle pass. Logs:
  `/tmp/ple_shared_search_m8_sql.log` and `/tmp/ple_shared_search_m8_connected.log`.
- The complete manager Rust gate passes, including strict Clippy, all-feature tests, doctests,
  and the Wasm target. Codebase checking found four stale Pool decoder fixtures missing the new
  required pair; corrected fixtures now also assert missing/invalid field rejection. Final
  codebase passes 544 tests, and schema generation/style are clean.
- The manager rebuilt the Live Demo from the frozen M8 source and captured the Library scene.
  Log: `/tmp/ple_shared_search_m8_capture.log`. The current demo origin is port 8116.
  The manager's post-rebuild audit (`/tmp/ple_shared_search_m8_after_audit.log`) and an independent
  rerun confirm zero Pools, members, or affected Assessment entries before and after. Audit and
  fixture review are clean; no content repair or selection-chance change was required.

### M9 calculated Pool license

- The required Pool license is calculated from exact member Revision licenses at creation,
  complete member replacement, and every fork. All seven supported combinations are checked,
  including CC BY plus CC BY-SA. A later Revision with a different license cannot change a
  Pool pinned to the earlier Revision; a BY-to-CC0 member replacement proves recalculation downward.
- The obsolete manual provenance table, functions, policies, and grants are removed. Pool views
  expose the calculated collection license and render members' existing original license and
  authorship fields; no duplicate member-credit contract was added.
- Manager fresh SQL lifecycle, schema, 544-test codebase gate, connected adoption/co-Instructor
  lifecycle, and the full Rust gate including Wasm pass. Receipts are
  `/tmp/ple_shared_search_m9_{sql,schema,code,connected,rust}.log`.
- Independent review caught missing PUBLIC revokes on the two internal calculation helpers.
  Explicit revokes and an application-role privilege-denial assertion now pass the fresh SQL
  lifecycle. Final independent source review is clean; the next Live Demo rebuild will combine
  M9 and M10 as planned.

### M10 combined Library query

- One SQL relation now combines Question and Pool rows before global sorting, cursor paging,
  and facets. Kind, membership, and owner are bound into the normalized request and version-3
  cursor. Pool-owned metadata supplies Pool matches; Question-only filters exclude Pools.
- Generated tagged rows reuse existing Question and Pool summary contracts. Source-object and
  statistics reads run only for returned Questions, including a zero-read Pool-only case.
  The Library frontend still explicitly requests Questions with all memberships at this stage.
- Manager Rust/Clippy/Wasm, 547-test codebase, fast UI, schema, and SQL lifecycle gates pass:
  `/tmp/ple_shared_search_m10_{rust,code,ui,schema,sql}.log`. These precede the final expanded
  connected matrix and do not close this milestone.
- Connected testing exposed synthetic Questions without ownership events; fixtures now match
  the authoritative owner contract. Review also aligned kind lookup
  with search visibility: Questions require current owners and Pools require members.
- The complete connected matrix passes against fresh PostgreSQL, including an actual Assessment
  fork with a Question found only in that fork, strict kind filters, member-only text/Tags,
  Question-only semantic filters, owner/Bloom, and exact global paging. Kind lookup checks actual,
  reserved, archived, empty, and unauthorized objects. Independent review is clean.
- The first current-source Live Demo rebuild, screenshot capture, mixed API acceptance, and
  browser journey pass. Logs are `/tmp/ple_shared_search_m10_{capture,live_api,live_browser}.log`.
- Speed testing found JIT compilation dominated the query. A measured function-local JIT
  correction passes independent review and the same complete matrix (3.39 seconds, previously
  81.39 seconds). Its final current-source rebuild and HTTP speed replay pass; see
  [SHARED_SEARCH_SPEED.md](SHARED_SEARCH_SPEED.md).
- Final Rust checks pass. The broader offline run passed code, schema, and 10,335 Python checks
  but found three generated-artifact storage budget failures. Removed local debug build artifacts
  and four inspected obsolete PLE image build layers. The storage checks and then all 10,338
  Python checks pass (`/tmp/ple_shared_search_m10_python.log`). Existing source-line proximity
  warnings remain advisory. The combined offline launcher is rerun at final close-out.
- Final rebuilt Live Demo capture, API, and browser journeys pass:
  `/tmp/ple_shared_search_m10_speed_capture.log`, `m10_final_api.log`, and `m10_final_browser.log`
  (the latter two use the same `/tmp/ple_shared_search_` prefix). Final HTTP medians are
  17.0/15.6/17.4/17.6 ms for default/text/Discipline/All Questions, all below baseline p90.
  The installed function's final EXPLAIN takes 22.3 ms without changing caller or cluster settings.
  The verification dataset contains 49 Questions, one Pool, and one member. M10 is complete.

### M11 Library object links

- `/library/{id}` resolves the authorized object kind and mounts the existing Question view
  or the extracted Pool detail view. Both publish their title to the Library breadcrumb.
  Pool discovery and Watch inbox links open the shared detail URL.
- Pool detail retains exact member credits, calculated license, classification, Bloom, and
  Instructor-only support/Star/Watch controls. Rejected detail resources retain a local retry
  surface; keyed dispatch resets component state when the resolved object changes.
- Manager codebase checks pass all 547 Node tests; full fast UI passes. Logs:
  `/tmp/ple_shared_search_m11_code.log` and `/tmp/ple_shared_search_m11_ui.log`.
- Normal capture refreshed the source build and reused cached application images; current
  Live Demo remains at port 8314. The temporary browser proof opens a Question and Pool
  directly, verifies breadcrumb titles, and retries both a failed kind lookup and failed Pool
  detail request. Logs: `/tmp/ple_shared_search_m11_capture.log` and
  `/tmp/ple_shared_search_m11_direct_routes.log`. Independent review found no blocker.

### M12 mixed Library

- One Library definition requests Both and Questions in no Pool by default. Its active default
  appears before the first search as a removable chip; removing it broadens to All Questions.
  Show, membership, and Owner filters round-trip through strict URL state. Pools-only clears
  structured Question-only predicates while preserving server-parsed search text.
- Tagged Library rows render Question and Pool details through all three shared displays.
  The old Pool discovery panel and `?pool=` path are removed. Question-only pickers and My
  Questions explicitly request Questions/All, including source-bound Pool creation.
- Selection has one 1,000-item maximum and retains kind across pages. Existing metadata editors
  receive their own kind; completing one editor removes only that kind from selection. A
  deterministic browser check saves Question metadata and verifies the selected Pool remains.
- Manager full G passes: 549 Node tests and the complete fast UI launcher. Logs are
  `/tmp/ple_shared_search_m12_final_code.log` and `/tmp/ple_shared_search_m12_final_ui.log`.
  The final test-only bulk-save assertion also passes the shared browser suite (4/4), TypeScript,
  lint typecheck, and diff check. Independent review's 45 focused tests pass with no blocker.
- Current-source normal capture passes and the manager inspected the initial default chip.
  The full Live Demo API/browser journey proves default member exclusion, All Questions,
  Pools-only, Pool presence in Compact/List/Visual boxes, and protected new-tab detail. Logs:
  `/tmp/ple_shared_search_m12_final_capture.log` and `/tmp/ple_shared_search_m12_final_live.log`.
- The M12 speed replay records 17.7-24.0 ms medians, below the original baseline p90 for every
  case. See [SHARED_SEARCH_SPEED.md](SHARED_SEARCH_SPEED.md).
- Integration corrected no-op default chips, retained selection after one-kind save, source
  picker defaults, and strict harness types. Exact combobox selectors fixed a browser-test
  false positive; the simple initial search layout was preserved.

### M13 backend eligibility proof (frontend milestone still open)

- The existing Blueprint-scoped Pool-members reader now returns required Pool Discipline UUID,
  Subject UUID, Question Type, and Backend. Values come from the authorized Pool row; the
  existing Blueprint visibility and Assessment-membership checks remain the boundary.
- Store, browser contract, generated TypeScript, and strict decoder agree. Review corrected
  an initially nullable Subject to match the database's required Pool Subject.
- The manager's fresh disposable PostgreSQL replay passes (1/1), including stability after a
  member Question moves to another valid Subject and concealment of an unrelated Pool. Log:
  `/tmp/ple_shared_search_m13_connected.log`. Its container and volumes were removed.
- Full Rust/Clippy/WASM gates pass (`/tmp/ple_shared_search_m13_rust.log`); schema generation
  and style pass (`/tmp/ple_shared_search_m13_schema.log`). Independent source review is clean.
- Shared Question-picker UI, eligibility binding, and browser acceptance remain in progress;
  these backend receipts do not close M13.

### M13 Question picker completion

- The Question picker now uses the shared session, controls, results, display, and selection bar.
  Its domain still owns sources, inspection, exact Revision pins, and the reorderable tray.
  The duplicated picker request session is removed.
- Pool creation, source-bound creation, Blueprint member editing, and Assessment fork editing
  use authoritative Discipline/Subject UUIDs and Type/Backend eligibility. Unbound creation
  resolves the first selected Question as its anchor. All three supported licenses fit.
  Question-only discovery explicitly uses All membership, including members of another Pool.
- The compiled browser proof checks the excluded exact starting anchor, an eligible Question
  already in another Pool, constrained Backend, and the restored Pool review flow. Focused
  tests cover final mismatch rejection and all supported licenses.
- Manager G passes with 549 Node tests and the complete fast UI launcher; logs are
  `/tmp/ple_shared_search_m13_m14_final_code.log` and
  `/tmp/ple_shared_search_m13_m14_final_ui.log`. Independent M13 re-review is clean and
  independently passes TypeScript, 13 focused Node tests, and diff checks.
- Review restored the accidentally removed tray reorder control. Browser integration supplied
  the fixture detail reader required for first-selection eligibility; it was a test fixture gap.
  The separate full Live Demo rebuild remains part of the M14 integration gate.

### M14 Assessment content picker

- One shared Assessment content picker replaces the old Pool-only picker in Blueprint and
  Course Assessment editors. It returns an ordered exact-Question batch or one Pool with its
  actual membership edit number. Blueprint retains draft append behavior; Course retains
  staged Pool settings and explicit import confirmation.
- Independent review corrected a metadata/membership edit-token mix-up; a divergent-token
  regression now uses metadata version 9 and Pool version 3 and requires version 3. Retained
  row snapshots reconcile with shared selection so Clear/chips/sort cannot confirm stale rows.
- G passes on the final sources (same manager logs recorded for M13). Source re-review is clean.
  The normal full rebuild installed the scoped eligibility schema/API and a client-only refresh
  picked up frontend fixes made during compilation. Logs:
  `/tmp/ple_shared_search_m13_m14_capture.log` and
  `/tmp/ple_shared_search_m14_client_refresh.log`. Current origin is port 8380.
- `tests/_temp/m14_assessment_content_picker_live.mjs 8380` passed against that fresh stack.
  It creates content through normal UI, adds Question and Pool entries to a Blueprint, saves
  Course Question pins, verifies Pool staging, imports explicitly, and verifies the Assessment
  fork ID differs from the original source ID. It also exercises Clear and chip removal before
  confirmation. The selected source ID is retained to distinguish same-title forks.
- The manager's complete rebuilt Library API/browser integration passes afterward:
  `/tmp/ple_shared_search_m14_library_live.log`. M15 reviews remain open.

### Final review correction

- Independent review found Pool Type matching through ordinary text and the `question_type`
  text field, contrary to the approved Pool text boundary. Both text clauses now restrict
  Type matching to Question rows; the structured Type filter still includes Pools.
- A fresh disposable PostgreSQL mixed-query regression passed (one test, 3.68 seconds),
  proving both text forms exclude Pools while retaining Questions, and structured Type
  includes both kinds. The proof runner removed its container and volumes. A separate
  reviewer confirmed the correction. The normal screenshot workflow is rebuilding the
  Live Demo from that schema before final integration checks.

### M15 review progress

- Final accessibility tests pass all four cases with zero serious or critical axe findings,
  including the mixed Library, Blueprint search, Instructor Pool detail, and both active
  picker dialogs. The test is registered in the normal fast UI launcher.
- Independent visual review found Compact hiding descriptions. RecordList now groups title
  and description, retains a dense desktop grid, and stacks at narrow widths. A permanent
  shared-search browser assertion compares visible description text across all three modes.
  The reviewer passed the revised Compact and narrow Pool captures; both medium findings
  are resolved. See [the visual review](SHARED_SEARCH_VISUAL_REVIEW.md).
- The final one-time picker probe confirms explicit submit, display changes without requests,
  empty browser storage, and late-response protection. Temporary-check dispositions are
  recorded in [the ephemeral report](SHARED_SEARCH_EPHEMERAL_CHECKS.md).
- Four live usability tasks pass. Blueprint Stars, manager G, and the rebuilt final integration
  remain pending; these preliminary receipts do not yet close M15.
- The manager's complete post-correction fast UI launcher passes, including all four final
  accessibility cases: `/tmp/ple_shared_search_m15_final_ui.log`. The offline aggregate is
  rerunning after screenshot-scenario type and formatting corrections.
- The latest offline run passes schema, Rust, TypeScript, formatting, lint, and all 549 Node
  tests. Python passes 10,328 checks and flags a new-directory documentation link (corrected
  to the concrete picker entry file) plus the planned temporary-browser-script cleanup.
  Final aggregate acceptance still requires the cleanup and a passing rerun.

### M15 completion

- All five independent live usability tasks pass, including Blueprint Stars on the rebuilt
  port 8292. No high or medium usability findings remain. Accessibility and visual reviews
  pass; the measured speed report already covers the required baseline/M10/M12 comparisons.
- Final manager G and the complete offline aggregate pass: all 549 Node tests and 10,330
  Python checks, schema/Rust/WASM checks, and the full browser launcher. Logs are
  `/tmp/ple_shared_search_m16_fast_final.log` and `/tmp/ple_shared_search_m15_final_ui.log`.
- `tests/_temp/` is empty after the documented disposition. Human Guidance's three-display
  checklist item now carries source, permanent-test, and independent visual evidence; the
  source-part splice validates successfully. M16 live corpus and integration gates remain.

### M16 live surface follow-up

- Production `ui_backbone_parity` passes at port 8292 after correcting stale test assumptions
  about Course links opening new tabs: `/tmp/ple_shared_search_m16_parity_final.log`.
- Eight normal Library capture checkpoints pass on the rebuilt Live Demo, with independent
  visual inspection confirming desktop modes, Pool detail, and narrow layout.
- The dedicated real Assessment picker capture exposed undefined dialog color tokens and
  overlapping background text that the isolated fixture did not expose. The picker now uses
  current surface, text, border, spacing, and shadow tokens plus the normal modal backdrop.
  Refreshed live captures and affected browser/visual rechecks are required before closure.
- The affected follow-up now passes: current live Assessment picker captures are opaque and
  readable at laptop and square viewports, confirmed independently. The complete post-fix
  browser gate passes (`/tmp/ple_shared_search_m16_dialog_ui.log`), and final Library API/browser
  integration passes (`/tmp/ple_shared_search_m16_library_final.log`). Full corpus publication
  and verification remain the next gate.

- Full normal screenshot publication and `--verify` now pass, including manifest closure,
  scenario privacy, and published-artifact integrity. Receipts are
  `/tmp/ple_shared_search_m16_publish.log` and `/tmp/ple_shared_search_m16_verify.log`.
  Verification retains 179 byte-different replay images for review; this gate does not claim
  pixel equality. The new Library and picker surfaces also have independent rendered review.
- Human Guidance evidence now cites current modules instead of the five modules removed by
  this plan. Six canonical checklist parts were spliced; all six part gates, 1196/1196 bullet
  consistency, and 406 guidance-format/link tests pass. Independent re-review is clean.
- The first final aggregate correctly rejected three narrow Instructor captures under the
  laptop-only screenshot policy. The implementation incorrectly added exceptions and then
  passed all 10,330 Python checks. The user identified this conflict with Human Guidance;
  those captures and exceptions are now removed, restoring the original policy.
- The next live acceptance stage exposed three stale positional Assessment policy fixtures:
  they passed six feedback-release arguments where the current function accepts five, placing
  `after_submit` in the Assessment-type position. The three fixtures now match the existing
  signature; production schema and behavior are unchanged. Formatting and test compilation pass,
  and the complete aggregate is rerunning for real-service proof.
- The connected Unrelease oracle exposed the same positional drift in SQL fixtures. A full
  repository call-site audit corrected seven SQL calls across Unrelease, duration accommodation,
  attempt expiry, and resume expiration (ten fixture calls total including the three Rust calls).
  Saved-response and production calls already matched the signature and remain unchanged.
  All three affected Rust test targets compile; shell syntax and formatting checks pass.
- The following aggregate passed the corrected policy fixtures and connected Unrelease.
  It then exposed an issued-public-ID collision in shared Blueprint test setup. This remains
  an open acceptance-fixture issue; the final aggregate has not passed yet.
- The Blueprint collision is repaired in test setup: reader/student public IDs use distinct
  Blueprint prefixes, and shared-seed reuse restores all three account identities. The fixture
  preserves permanent public-ID reservation semantics. A fresh serial database proof passes
  grading lifecycle (4 tests), Blueprint lifecycle, discovery, and Question Library (1 each).
  Receipt: `/private/tmp/ple_shared_search_m16_blueprint_sequence.log`; its container and volumes
  were removed. The required complete aggregate is rerunning after this proof.

### M16 completion

- `source source_me.sh && ./launchers/all_test.sh` passes on the final source: schema/style,
  Rust/Clippy/WASM, TypeScript/lint/format, 549 Node tests, and 10,330 Python checks. All three
  real-service lanes pass: PostgreSQL baseline, installation-data provision/replay, and
  PostgreSQL/MinIO course-appearance consistency. Receipt:
  `/tmp/ple_shared_search_m16_final_acceptance.log` (exit 0). Disposable services, volumes, and
  network were removed by their owners; the Live Demo is stopped.
- The final browser/accessibility gate, live Library API/browser gate, production parity,
  screenshot publication, and screenshot verification receipts are recorded above. Independent
  review clears the implementation, rendered surfaces, documentation, and final fixture repairs.
- The architecture guide, source/generated Human Guidance checklist, schematic, terminology,
  design decisions, architecture/file map, Question model, and changelog are updated. Shared page
  size constants have one owner; `tests/_temp/` is empty after its documented disposition.
- Shared search, mixed Library search, Pool ownership, Type/Backend, unique Question membership,
  and calculated licenses are complete. The schematic remains active for its separate five-item
  Pool validation handoff; optional M17 was not required or implemented.
- Final completion-document checks pass: 406 guidance-format and Markdown-link tests plus
  `git diff --check`. The supplied plan's staged state and existing untracked agent/skill files
  are preserved; no commit or staging was performed.

### Fresh screenshot follow-up

- At the user's request, rebuilt a clean Live Demo and published all 259 repository screenshots
  again after implementation. This exposed a capture-fixture dependency: the Assessment picker
  assumed a Pool already existed, and the shared setup's browser function was defined without
  invocation. The picker now owns setup, and the helper invokes it. A live create/reuse probe
  confirms exactly one Pool. Product code is unchanged.
- Fresh publication and normal verification pass. Receipts:
  `/tmp/ple_shared_search_fresh_screenshots_published.log` and
  `/tmp/ple_shared_search_fresh_screenshots_verify.log`. Verification covers manifest closure,
  privacy, and artifact integrity; 182 byte-different replay images remain for review, with no
  pixel-equality claim. The fresh Assessment picker was visually inspected.
- TypeScript, ESLint, Prettier, 12 corpus tests, 405 viewport/link checks, and diff checks pass.
  Updated screenshot galleries and manifest are published. The Live Demo remains available at
  `https://localhost:8304/` after this follow-up.

### Permanent-test review requested by the user

The October 5 review applies all six questions in `docs/PYTEST_STYLE.md`: named regression,
important stable behavior, no clear duplicate, justified precision, actionable failure, and
lasting value beyond implementation proof. The resulting coverage has these owners:

- Shared session: stale rows/facets, retry, query reset, cursor recovery, and bounded selection.
  Async overlap and the observed page-size recovery bug justify deterministic unit checks;
  failures require fixing request ordering or recovery. Consumer copies are removed.
- Existing live Library journey: explicit submit, local display changes with descriptions,
  membership, new-tab detail navigation, and leave confirmation. Browser behavior needs the
  real UI; one journey owns it. Failures require fixing the interaction or documenting an
  intentional product change. Mocked repeated journeys and CSS-class assertions are removed.
- Connected database: mixed paging and visibility, immutable Pool shape, unique membership,
  calculated licenses, fork ownership, and Pool-owned eligibility. These protect persisted
  integrity and authority. Failures require repairing that boundary. Repeated API/filter/sort
  matrices, every-license-combination probes, and duplicate create/save assertions are removed.
- Picker unit boundary: exact Question Revision pins/order, distinct Pool edit token, and
  eligibility at selection. Actual token confusion and independently changing metadata justify
  these small domain cases; generic decoder-key/enum inventories are removed.
- Screenshot scenes remain the requested repository capture workflow. Their 377 lines are
  separate from permanent product-behavior gates. Axe, keyboard, layout, and consumer matrices
  remain one-time review evidence; the 1,325-line duplicate browser/accessibility suites and
  their dedicated harness were deleted, not relocated.

`git diff --stat HEAD -- tests` fell from 3,507 insertions / 488 deletions (net +3,019) to
1,508 insertions / 543 deletions (net +965): 2,054 fewer net lines, a 68% reduction. Rust test
additions outside that directory also fell by 366 lines. Existing fixture compatibility repairs
remain. The architecture guide, test strategy, and generated HG evidence now describe the
retained owners rather than prescribing a test or harness for each consumer.

Validation after pruning passes:

- `run_fast_checks.sh`: 541 Node tests, 10,475 Python checks, Rust/Clippy/WASM,
  TypeScript/lint/format/schema checks (`/tmp/ple_test_pruning_fast_clean.log`). Stale Rust
  package artifacts initially exceeded the existing disk budget; package-scoped cleanup
  removed 3.1 GiB before the passing rerun. The budget test remains unchanged.
- Retained `run_fast_ui_checks.sh` (`/tmp/ple_test_pruning_ui.log`) and the real Library
  API/browser journey (`/tmp/ple_test_pruning_live.log`) pass against the existing Live Demo.
- The compact mixed PostgreSQL test passes (1 test, 1.41 seconds, private disposable proof).
  `tests/e2e/e2e_assessment_saved_response.sh` passes the full saved-response/Pool-fork SQL oracle.
- Independent permanent-test review is clean. Documentation checks pass 406 tests, HG source
  and checklist match at 1,196 bullets, `git diff --check` passes, and `tests/_temp/` is empty.
  No product code, screenshot corpus, or index state was changed by this pruning.


## Instructor screenshot scope correction

The user identified that the published Library phone capture and two square picker captures
conflicted with Human Guidance's 1280x800 laptop target for Instructor workflows. Removed all
three scenario declarations and images, restored the original Python and Node laptop-only
checks, and corrected the plan's visual-review target. The previous exceptions were an
implementation error, not an approved change to Human Guidance.

The corpus now contains 256 captures. Existing fresh images were retained; the manifest,
receipt, atlas, and folder galleries were regenerated through the existing publication helpers.
Static artifact verification, 20 corpus tests, 405 viewport/link checks, TypeScript, lint,
formatting, and diff whitespace checks pass. No new permanent tests were added.
