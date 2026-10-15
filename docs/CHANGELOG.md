# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> October 11 entries are archived in [CHANGELOG-2026-10e.md](CHANGELOG-2026-10e.md).
> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-15

### Behavior or Interface Changes

- Clarified Search Question Library guidance: open an empty spreadsheet with column headings,
  keyword search, and a concise prompt; require a keyword or filter to show results. Clearing
  all criteria returns to the empty spreadsheet. The focused Human Guidance formatting check passed.
- Clarified that Search and Browse share RecordList's column filters, sorting, display modes,
  and record actions. Search uses full-width results with more visible columns; Browse adds a
  compact classification sidebar, leaving narrower results with fewer visible columns.
  RecordList adapts its visible columns to available space. Search adds keyword search above
  RecordList, and sorting alone does not initiate an unfiltered search.
  Renamed the shared filter guidance heading and updated its specification link. All three
  focused formatting/link checks passed; a broader selection also found two existing broken
  links to `publication.rs` in archived Human Guidance audits.
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

## 2026-10-14

### Investigation evidence

- Compared the failing WeBWorK image with an isolated, unmodified renderer HEAD build. Both
  reproduce the Genetics terminology apostrophe syntax error; minimal PGML, required macros,
  and representative unchanged questions pass. Traced the failure to BPQ's YAML matching
  generator choosing plain-Perl escaping instead of its existing complete-literal helper.
  Recorded provenance, controlled results, and ownership in the PGcore investigation report.
  Renderer working-tree changes, BPQ sources, generated BPW content, and the importer were
  preserved; no repair was made in this investigation.

### Behavior or Interface Changes

- PG/PGML imports now inspect the existing WeBWorK renderer's `flags.error_flag` and
  `debug.pg_warn` through an authenticated saved-Draft preview diagnostics option before
  publication. Errors leave the Draft unpublished; warnings are reported with the source and
  seed without blocking publication. QPM remains exclusive to BBQ-to-Native-JSON conversion.

- Replaced the miniature Store/SQL curriculum publisher with one shared host Python HTTP
  importer for all seven usable BPW collections. Genetics, Biochemistry, Biotechnology, and
  Biostatistics become complete Public Blueprint Courses; the other collections enter the Library.
  PG/PGML remains unchanged WeBWorK source, while BBQ passes through the configured QPM CLI to
  Native JSON and ordinary PLE authoring operations. Source topics, ordering, static Pools,
  nine-Question Assessment splits, and editable point defaults retain the approved import rules.
  The result reports incomplete work explicitly through six stable fields.
- The installer invokes that shared importer with explicit source/tool paths from its selected
  stack configuration file, adopts full Biochemistry as BCHM 355, and retains fictional identities
  and enrollment separately. Representative completed, open, and unstarted Student Work uses
  ordinary delivery. `local_stack.py install-data --without-live-demo` retains curriculum while
  omitting the fictional graph on a fresh installation. Temporary ordinary Instructor sessions
  travel through private pipes, use the existing lifetime, and are revoked after success or failure.
- Added generic Native JSON HTML, inline CSS, and image bindings through ordinary Draft,
  publication, and delivery paths. Authored source stays unchanged; display content is sanitized
  and bound images use authenticated routes. Hint and feedback images follow their disclosure
  state. Browser imports retain a saved Draft after an image-binding error and continue later items.
- Made new Blueprint Courses, canonical imports, and forks Public immediately. Removed the
  Private state, Publish, and Return to Private controls and routes. Owners retain editing,
  immutable Save Revisions, archive, and restore authority; active Instructors can read shared
  content and history, and adopt Public Blueprints. Archived content remains read-only and
  excluded from ordinary discovery. Updated current contract and Instructor documentation.
- Removed the partial long-lived Instructor API-token implementation. Content routes continue
  to use the existing authenticated-session boundary; general Instructor API tokens remain deferred.

### Fixes and Maintenance

- Removed the obsolete `published` watch-notification enum member found during quality review and
  corrected
  the support-image position offset in its owning SQL function. Corrected importer/installer
  specification findings and aligned the directly affected usage and demo documentation.
  Sixteen malformed-CSS banks were repaired at the Biology Problems source and regenerated into
  BPW; script-dependent banks use QPM's existing `--html-to-image` option.
- Removed the redundant Native prompt length cap in Rust and browser validation after a live
  BPW publication failed within the existing source budget. Nonblank prompt validation and the
  256 KiB source budget remain in place.
- Corrected demo finalization to set a missing Biochemistry adoption due date through its
  ordinary edit route using the existing term end, then release with the returned canonical edit
  number. Corrected handling of top-level workspace entries.
- Registered Native image access in ordinary published Question details, including selected
  revisions. Updated screenshot helpers for imported content and the canonical Instructor role.

### Developer Tests and Notes

- Aggregate Rust checks passed before the final small corrections. TypeScript checks passed,
  including 23 Node tests, and the full pytest suite passed 9,061 cases after cache cleanup.
  Focused installer handoff checks covered secret-free argv, selected gateway CA trust,
  content-only installation, and revocation after successful and failed imports. Fresh
  importer/installer correction review passed 15 temporary cases, an HTTP probe, and a canonical
  edit-number probe; one retained unit check protects the result's six fields. Schema
  regeneration/style and 2,739 ASCII/Markdown checks passed during documentation integration.
- The temporary source probe selected 487 sets: 189 PGML, 73 PG, and 225 BBQ. All 482 BBQ banks
  were accounted for, with 257 superseded by algorithmic sources. Final QPM conversion passed
  every selected BBQ bank with zero failures and emitted 10,887 Native Questions. Eight
  script-dependent banks separately produced 370 Questions and 570 images. These are current
  source observations, not permanent inventory gates or live import totals.
- Public Blueprint and Native specification reviews, Native support-position and importer/installer
  configuration correction reviews, and combined quality review accepted the prior implementation.
  The full HTTP mock passed with 11,149 Questions, 225 Pools, four Blueprints, and 570 images.
  After the live publication failure exposed the redundant prompt cap, temporary catalog checks
  passed all 225 banks and 10,887 Native items through Rust and browser validation. Fresh prompt
  specification and quality reviews accepted the correction. The real HTTP import then completed
  all 487 selected sets, created four Blueprints, and adopted Biochemistry with HTTP 201.
- Fresh full aggregate checks passed Rust, TypeScript/Node, and 9,062 pytest cases. Final demo
  release exposed a missing adoption due date; fresh specification and quality reviews accepted
  the finalization correction, five temporary actual-function HTTP probes passed, and fresh Rust
  checks passed again. The subsequent full live installation passed. A fresh content-only
  installation retained four Blueprints with no fictional Student or Course graph and no active
  sessions. Representative Native Student rendering, D8 hint/feedback disclosure, published
  current/revision image binding, and actual BPW image loading passed temporary runtime probes.
- Selected screenshot workflows passed at four viewports. Course Attempt History capture remains
  failed because of the pre-existing privacy-profile mismatch with authorized post-submit Native
  answers, documented in the deferred report; full screenshot acceptance is not claimed. The
  final full compliance command exited 0: Rust, browser checks including 23 Node tests, 9,062
  pytest cases, database baseline E2E, and the disposable PostgreSQL schema authority persistence
  oracle passed; complete live acceptance reported green. Implementation is complete. Obsolete
  executable probes were removed; temporary receipts and screenshots remain deliberately
  available as evidence. The [BPW import status](active_plans/active/bpw_import_status.md) records
  the completed implementation and separate screenshot limitation.

## 2026-10-13

### Behavior or Interface Changes

- Implemented WeBWorK autosave and save before Question navigation, leaving an Attempt, and Submit.
  Save failures retain editable input and expose Retry; successful saves proceed quietly. Accepted
  saves use an atomic revision check so delayed older requests cannot replace newer work. Save
  acknowledgements preserve the mounted Question, and a lost submission acknowledgement triggers
  an authoritative progress check.
- Added completed Question-and-recorded-response review independently of correct-answer disclosure.
  WeBWorK replays the retained source and seed, with retained prompt context when replay is unavailable.
  Native review includes original prompts, choices, and retained isolated author content. Removed
  Instructor timing controls for the Student's own response and correctness while preserving optional
  answer and teaching-feedback settings. The complete repository gate passed with 9,090 pytest
  cases and fresh PostgreSQL acceptance; representative renderer replay, browser fault journeys,
  and all 257 live screenshot captures passed. Inspected the changed Student review views.
- Fixed a readiness race found during full screenshot verification: navigating away from a visible
  WeBWorK form now waits for its bridge's ready message before capture. The existing deadline keeps
  missing messages retryable. A browser regression check verifies that an untouched Question stays
  unanswered when navigation begins before the frame finishes loading.

### Fixes and Maintenance

- Completed six fresh independent Student-work reviews: Plan, Test, Style, Docs, Legacy, and
  Comment. No production defect was confirmed. A suspected first-save race was withdrawn after
  tracing the existing parent-row locks. Added selector-owner references to the browser scenario
  and shortened a source banner; focused formatting passed. One recommendation remains for human
  review: strengthen the existing completed-review scenario with a direct protected-answer-content
  assertion. This audit reused the completed implementation's full-suite and browser evidence.
- Added the [WeBWorK Student work plan](active_plans/decisions/webwork_student_work_plan.md)
  with the selected approach: autosave while working and save before navigation, with recovery
  actions for failed saves and no routine confirmation prompts. Excluded crash recovery at the
  human's direction. Local ADAPT inspection found open-ended text autosave and submitted WeBWorK
  session restoration, but no matching WeBWorK autosave/save-before-navigation path. Recorded the human
  requirements for stable MC/MA/MATCH positions within an Attempt, new seeds only on new Attempts,
  and Question/recorded-response/correctness review even when correct answers are hidden.
  Included a focused FIB typing, immediate-navigation, and clearing check alongside MC/MA/MATCH.
  Grounded acceptance in Student-visible behavior and representative cases; tool-version changes
  alone do not block working replay, and byte/pixel equivalence is not an acceptance requirement.
  Clarified adaptability over speculative edge-case handling, excluded unsaved browser-crash work
  from implementation and acceptance, and removed visibility/shutdown save handling from the plan.
  Documentation checks passed for the planning change; implementation is recorded above.
- Refocused launch readiness on actual Student work in the
  [Student launch audit](active_plans/audits/STUDENT_LAUNCH_READINESS_AUDIT_2026_10_13.md).
  Reproduced WeBWorK answer loss on navigation, native answer loss after leaving a failed save,
  disabled WeBWorK save recovery, and false submission status after a lost acknowledgement;
  recorded native review's missing Question context. Confirmed saved-response restoration, 2X
  accommodations, background deadline submission, and sampled accessibility checks. This is audit
  evidence at the reviewed baseline; implementation and acceptance are recorded above.
- Added the [launch readiness audit](active_plans/audits/LAUNCH_READINESS_AUDIT_2026_10_13.md)
  with six independent review passes, verified hosting/email configuration gaps, first real
  administrator bootstrap requirements, and conditional course-content scope. Classroom production
  launch remains on hold; stale Bloom and grade-export blockers are identified as documentation drift.
- Recorded passing offline gates (including 9,072 pytest cases), a separate fresh PostgreSQL
  acceptance rerun after releasing an orphaned demo lease, all 257 live screenshot captures, and
  actual CSV/TSV grade downloads. Kept tracked screenshots and production code unchanged; dependency
  scans found one informational Rust YAML-parser maintenance advisory and no Node or direct Python
  runtime advisories.

## 2026-10-12

### Behavior or Interface Changes

- Renamed PLE's QPM WASM consumer contract to the project-owned `qpm-wasm` package and
  `qpm_wasm` generated artifacts. The importer and vendor tool now load the producer's QPM
  namespace; QTI remains the name of the assessment interoperability standard. Removed the unused
  QTI runtime environment setting from the browser-test composition.
- Rotated the completed October 6 and October 7 change blocks to
  [CHANGELOG-2026-10e.md](CHANGELOG-2026-10e.md).
