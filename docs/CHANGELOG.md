# Changelog

## 2026-09-10

### Fixes and Maintenance

- Kept the Questions-to-Policies link visible after a saved Assignment reload. The Questions
  workspace retains its post-save status message while its Policies route remains keyboard
  reachable from every successfully loaded Questions page.

- Corrected Assignment Workspace's strict display-zone decoder to accept exact browser-supported
  IANA names, including the valid account default `UTC`, while continuing to reject malformed and
  unsupported values. This restores successful handling of the live Assignment-create `201` body.

- Corrected the M1 Create Assignment page to use the existing Course Instance-reference Assignment
  creation client. Title-only creation now posts through the live
  `/api/course-instances/C-n/assignments` contract and enters the new Assignment's Questions task
  without exposing internal Course UUIDs or adding a legacy route alias.

- Completed the M0 documentation-ownership receipt for the active Interface Cleanup plan:
  terminology now owns names and semantic ownership, the design guide owns Ribbon structure, and
  durable decisions record the settled Ribbon, time-zone, mutable-authoring, Course-activity,
  randomization, and Blueprint-provenance direction. This records documentation scope only; later
  interface milestones remain pending.

- Corrected the terminology authority's revision model. Question Revision and Blueprint Revision
  are the two content Revision concepts. Assignments, Course Instances, Draft Questions, Question
  Change Proposals, invitation rules, accommodations, and Course Retention Plans use current state
  with Edit Numbers where concurrency needs them. Assignment Attempts, Issued Questions, grading
  records, and qualified operation evidence retain their exact dependencies directly.
  Repeated activities create separate records or Events; each new Student pass creates a distinct
  Assignment Attempt while earlier Attempts remain retained under the Student Work retention policy.
  Repository-wide schema and code migration remains separate follow-up work.

- Tightened the mutable Assignment terminology. Assignment Unrelease stops new Assignment Attempts
  and permanently deletes that Assignment's existing Student Work; closing and archiving preserve it.
  Changes accepted while Released keep the Assignment release-valid.
  Assignment Source Records describe one copy or update operation, Assignment Attempts retain only
  the facts required to interpret and grade the work, and plain Local Date and Time inputs use the
  authenticated Instructor's Account Time Zone.

- Completed the narrow revision-policy follow-up. Published Questions and Blueprint Courses now own
  mutable Available or Archived state while their exact revisions remain resolvable. Question Title,
  Description, Subject, Subsubject, Tags, Bloom Classification, Classification, and Language are
  current lineage discovery metadata; Authorship, License, Citation, Type, Format, and Backend retain their
  exact Question Revision scope. Assignment adoption of a new Question Revision is described directly
  without Update Choice or Update Receipt nouns. Blueprint schedule resolution names the authenticated
  Instructor's Account Time Zone. Assignment Export terminology is removed because the product has no
  intended compatible Assignment export workflow.

- Completed the final Assignment terminology cleanup. Assignment Question Editor and Assignment
  Properties Editor are the named editing surfaces, and Assignment saves carry the reviewed Assignment
  Edit Number directly. Assignment Unrelease now states once that it removes Student access and
  permanently deletes the Assignment's existing Student Work Records.

- Defined Danger Zone as the closed set of high-consequence administrative actions: Assignment
  Unrelease, Archive Published Question, and Archive Blueprint Course. Assignment Unrelease shows
  affected Student-work counts and requires the exact Assignment title; the reversible Archive actions
  show their shared-availability consequence and require conspicuous confirmation. Restore, Assignment
  Close, and Assignment Archive use ordinary controls. Trusted server checks, atomic transitions, and
  redacted audit evidence define the corresponding ASVS validation and authorization boundary.

- Completed M6 Student vocabulary and Course entry: Student-visible surfaces no longer expose
  Instructor-only Course or release nouns, typed Course references, or authored library Question
  Titles. A Student with one current Course enters it automatically while zero and multiple Course
  states retain an empty state and chooser, including a usable return path. A source-guided sweep
  of the changed Student course, invitation, access, attempt, summary, and navigation surfaces
  found no visible `Course Instance`, `Released Assignment`, `Question Title`, or `C-n` label;
  the compiled M6 evidence passed zero, one, chooser, many, and return states. M5 supplies the
  live Student access and Attempt proof for the surviving delivery lane; invitation wording remains
  source-reviewed in the M6 receipt.

### Behavior or Interface Changes

- Added the focused Student Assignment question navigation component. It renders ordered native
  Question controls with visible and announced Not answered, Saved, or Closed states; the current
  Question is marked beyond color, and the compact layout adapts for narrow phones.

- Completed M5 Student delivery. The public one-question Attempt route has Student-only entry, and
  Start replaces the access URL with its returned `R-n` Attempt. A supported Live Demo browser
  journey passed keyboard Start, autosave, truthful reload restoration, question navigation, manual
  save, responsive views, whole-Attempt submission, terminal reload, and axe without serious or
  critical findings. Native PLE leased-job interruption/replacement and WeBWorK deterministic,
  renderer-outage, and replacement recovery also passed.

- Completed the M5 resume path. Assignment Access now returns only the authorized active public
  `R-n` Attempt reference or `null`; returning Students replace-navigate directly into that
  Attempt with a brief resume state instead of receiving another Start control.

- Completed the Student Attempt Ribbon return projection. The active resolved Attempt screen now
  supplies its public Course and Assignment references to the existing Student Assignment Access
  route; loading, rejected, and summary scopes retain no return link.

- Completed M3 Instructor list density and identity. Instructor Course and Assignment lists now
  use semantic, title-first dense rows with bounded actions, while Student Course cards retain
  their existing presentation. An Instructor-authorized Course summary provides only a closed
  Theme value for each Course row, shown with text and a local accent boundary rather than a
  Product-wide Course appearance scope.

- Completed M12 Question-owned answer randomization. PLE-native Question authoring owns the private
  `randomizeChoices` declaration: legacy omission remains false, new declarations serialize
  explicitly, and non-choice declarations are rejected. Native issuance ranks stable authored
  choice IDs from the durable nonce before opaque binding minting, so recovered and graded attempts
  retain the presented semantic order. Generated and issued public response contracts do not expose
  the policy; Question Backend presentation remains backend-owned.

- Completed M13 account-owned time-zone storage. Every Account receives an exact installed-IANA
  preference in a private forced-RLS relation; self-only reads expose no Account-ID capability. A
  new Student receives the authorized Instructor's zone once at enrollment, while later claim,
  import, and re-enrollment preserve the Student-owned preference. Stored deadline instants remain
  unchanged because they carry no zone.

- Completed M14 wall-clock input interpretation. Assignment Workspace now sends a zone-free local
  value; the authenticated Instructor's account zone resolves and projects it within the authorized
  Store transaction, while persisted deadlines remain `timestamptz` instants. Course-zone retirement
  and account-zone display remain separate milestones.

- Completed M15 account-zone display. Every production browser wall-clock formatter receives an
  explicit authenticated viewer or account time zone: Student delivery and activity share and name
  one Student zone, Gradebook submission times name the Instructor zone, Sysadmin sign-in times name
  the viewer zone, and Teaching Team and pending-invitation timestamps append the exact viewer IANA
  zone. The due-date editor names the Instructor zone that interprets local entry. The Attempt timer
  continues from server-supplied `timerRemainingMilliseconds` using monotonic `performance.now()`
  elapsed time and the existing Wasm timing calculation. A targeted final review found no findings
  and 28/28 focused tests passed. Student presentation mounting remains M7 and Course-zone
  retirement remains M16.

- Completed M2 Ribbon, dense top bar, and breadcrumbs. The one dense top bar holds identity, role,
  account, backed Product tabs, and Sign Out; visible Product and task navigation uses icon plus
  text. Unbacked destinations remain truthfully Unavailable without placeholder links. Shell-owned
  breadcrumbs provide real ancestors and a current terminal below the Ribbon without adding a
  Ribbon row, including deferred-route geometry. Keyboard traversal, responsive profiles through
  200% zoom, sprite and ledger checks, and routed-shell visual review passed. M9 retains the final
  static screenshot-corpus refresh.

### Developer Tests and Notes

- The current exact `./launchers/all_test.sh` checkpoint passed end to end: Rust, the frontend
  gate (409 Node tests), 6,252 pytest tests, and both connected acceptance lanes (PostgreSQL
  baseline and PostgreSQL-plus-MinIO Course Appearance). Supported cleanup completed after the
  run. Supported Live Demo startup, the real keyboard/browser/axe journey, and native/WeBWorK
  recovery acceptance completed M5 separately.

- M2's exact final `./launchers/all_test.sh` checkpoint passed with 414 Node tests, 6,252 pytest
  tests, connected PostgreSQL baseline and PostgreSQL-plus-MinIO Course Appearance acceptance, and
  a clean final diff. A fresh six-pass audit and final rereview closed the concrete M2 findings;
  M3 already has its receipt, so M16 is next in dependency order.

- M5 retired the duplicate frontend per-presentation route and nonce browser client, and retired
  the server's old nonce `POST`; the retained status route is GET-only and its focused server
  regression suite passed 45 tests. The Attempt context was extracted into a focused module to
  satisfy the repository source-size gate. The navigation SQL oracle now finalizes before its
  separate stable reads and proves the exact two-position submitted state. Live Demo provisioning
  uses current public Attempt APIs: Mary remains finalized and graded, Jack remains open with two
  saved responses, and replay waits for Mary's pending grade without creating another Attempt.

- The screenshot manifest removed ten obsolete Student-delivery captures and the retired coverage
  row without changing their historical PNGs or receipt. Current Attempt capture, receipt, and
  atlas replacement remain explicit M9 work; static corpus replay is intentionally pending that
  replacement. iMathAS end-to-end Student issuance and launch was unbacked before this plan and is
  recorded as a separate backend-delivery follow-on, rather than M5 scope.

- Audit remediation refreshed the durable database, contract, architecture, and
  usage maps for the checked-in `2026091011` saved-response/submission and
  `2026091012` Attempt-context migrations. Their focused SQL oracles pass.
  The initial aggregate `launchers/all_test.sh` run passed Rust and then failed
  in the frontend gate; focused fixes and tests followed. The clean rerun passed
  Rust, `check_codebase.sh` (408 Node tests), 6,246 pytest tests, and both
  connected acceptance lanes: PostgreSQL baseline and PostgreSQL plus MinIO
  Course Appearance.

- Six fresh checkpoint reviews (Plan, Test, Style, Documentation, Legacy, and
  Comment) found and led to repairs for syntax and stale tests, missing Attempt-context
  registration, the response validation/save recovery path, and shared SQL fixture
  leakage. Focused independent re-reviews accepted those repairs. The open M5 browser
  journey and remaining frontend route/client cleanup retain their own acceptance work.

- Rotated the complete 2026-09-08 and 2026-09-07 day blocks into
  `CHANGELOG-2026-09f.md`; the active changelog retains its two newest day blocks.

- The focused navigation repair passed 9/9 Node tests with `node --import tsx --test
  tests/test_assignment_attempt_navigation.mjs tests/test_student_assignment_attempt_navigation.mjs`.
  `git diff --check HEAD` also passed.

- M3, M12, M13, and M14 work was recorded ahead of the patch-reporting order. Those receipts retain
  their existing evidence; after M5 closes, remaining closure follows the original sequence beginning
  M12, M13, M14, M15, M2, and the subsequent listed patches.

- M1 acceptance passed: `./check_codebase.sh` reported 389/389 Node tests; the exact Markdown,
  ASCII, and whitespace Python gate reported 1,948 tests; `git diff --check`, the Ribbon E2E gate,
  and service-only plus browser-only Assignment Release gates passed. Composition image review and
  keyboard ordering/save/reload evidence passed, and fresh independent review found no blocker or
  major finding. Final screenshot recapture, receipt, and atlas publication remain M9 work.

- M2 test maintenance aligned the static Ribbon design and compiled-shell fixtures with the current
  catalog, glyph vocabulary, and Course summary theme contract. The remaining M2 router and
  pending-navigation failures are recorded separately; this fixture-only change does not alter
  production Ribbon behavior.

- M3's 11 focused Node summary/theme/scope tests, learning-data-access and server-core Cargo
  checks, TypeScript check before concurrent M2 Ribbon drift, format, diff, and source-line checks
  passed. Independent code review passed, and browser-rendered visual evidence passed at desktop,
  tablet, and narrow-phone profiles plus forced colors: no row clipping or row-originated overflow,
  44px narrow actions, visible focus, and text-plus-boundary Theme identity.

- M0's narrow documentation gate passed: `source source_me.sh && python3 -m pytest
  tests/test_markdown_links.py` reported 228 passed, `git diff --check` passed, and an independent
  M0 re-review found no blocker, major, or minor plan-conformance finding.

- M6 focused TypeScript, attempt-recovery, Prettier, and diff checks passed; its initial
  repository gate passed typecheck, lint, format, and 362 Node tests. The compiled browser proof
  `node --import tsx tests/playwright/student_course_entry_m6_evidence.mjs` passed on rerun for
  zero, one, chooser, many, and back-navigation states, and the independent M6 re-review passed.

- M12's independent re-review passed. Rust presentation and PLE Question JSON tests, 37 Node
  authoring tests, unchanged `cargo tools tsgen` output, `npx tsc --noEmit`, a 966-line
  presentation builder check, and both diff checks passed. The repository-wide source-file limit
  failure was isolated to concurrent `src/style.css` at 1008 lines.

- M13's clean PostgreSQL 17 migration acceptance and compatible second no-op run passed. Its
  connected oracle covered direct-table and RLS denial, self-only reads, every-Account defaults,
  and the Student roster lifecycle; the disposable container, volume, and network were removed.
  Focused Rust tests passed, the full Python suite reported 6120 passed, and independent
  architecture and security reviews both passed.

- M14's 38 focused Rust tests, 25 focused Node tests, generated TypeScript, TypeScript check, and
  independent re-review passed. Its disposable connected acceptance used a zone different from the
  Course's former zone to verify CAS save/reload, the stored instant, exact local projection, and
  foreign or missing-context concealment; fresh and compatible no-op runs both passed.

## 2026-09-09

### Additions and New Features

- Replaced the eight-image Live Demo capture path with a 52-image, manifest-driven atlas spanning
  Public, Instructor, Student, and Sysadmin workflows. The active corpus is flat within four
  role-owned folders and uses the canonical laptop, tablet, phone, and square viewport profiles.

- Added a generated scan-oriented Screenshot Atlas, a receipt binding the manifest digest and
  exact PNG path/dimension/hash set, and a coverage ledger accounting for every current route and
  Ribbon destination as captured, covered by equivalent visual evidence, or deferred for a named
  missing product capability.

- Added typed screenshot scenarios with one registry mapping manifest checkpoints to executable
  workflows. Captures now exercise ordinary Live Demo routes and persisted mutations for Question
  publication, Course invitation claim, Assignment release and Student response progression,
  Instructor Account lifecycle, and scoped Sysadmin support.

- M1 made the Rust `CourseTheme` contract the single source for the browser's generated closed
  theme vocabulary, removing separately maintained theme-ID lists while preserving all 15
  reviewed biome and habitat palettes.

- M2 added forward migration `2026090902`: every Course Instance now owns a scalar
  `course_theme` with the database-owned `grass` default. The current appearance contract no
  longer carries an appearance revision or retained appearance history.

### Behavior or Interface Changes

- M10 admitted Appearance as the Instructor-only Course Setup Ribbon task after its complete
  route/page/client/server/store path and M9 accessibility evidence. The catalog now names its
  executable route, the capability registry records aggregate read plus independent theme/banner
  mutations and the registered router, and the generated ledger records the role ceiling. Ribbon
  visibility remains a UI ceiling; Course Membership authorization stays at the route and server.
  The all-15-theme Ribbon oracle confirms unchanged rows/control positions and reachable,
  unclipped Appearance controls. The page now distinguishes scope loading from a failed read and
  offers a fresh scoped retry rather than presenting either as permanent unavailability. M11
  cross-member propagation is recorded separately.

- M11 completed the registered production-browser Course Appearance propagation gate. An
  Instructor reaches Appearance from the visible Course actions navigation, independently saves a
  Forest theme and server-decoded PNG banner, and reloads the same saved state. An enrolled Student
  then opens the normal Course landing and receives the same theme scope/token plus a loaded Course
  entry banner, while a second Instructor-created Course stays Grass with no banner.

- M9 added compiled-over-HTTP Course Appearance accessibility evidence: native keyboard theme and
  keyboard-reachable banner-input paths, named non-color theme state, status/error announcements,
  four canonical reflow profiles, and an axe serious/critical gate. The focused gate corrected phone
  banner-preview overflow and non-contrasting raw accent preview labels while reusing the
  application's existing forced-colors and reduced-motion coverage.

- M8 completed the Instructor Appearance page's independent Banner section: pending PNG, JPEG, and
  WebP files remain local until explicit save; centered 6:1 Course and 5:2 Course-card previews
  show the actual crop; alternative text is explicitly decorative or bounded informative text; and
  saved banners can be replaced or removed without disturbing a pending theme selection.

- M7 restored the Instructor Course Appearance route with a complete Theme section. All 15
  generated biome and habitat themes now use native radio controls, named Canvas/Secondary/Accent
  palette-role previews, live local preview, and an independent save that releases its preview on
  success, failure, or page abandonment.

- M4 implemented the no-store course appearance read model. Active Instructor and Student Course
  Members receive one current `{ theme, banner }` response; anonymous callers, nonmembers, and
  foreign Instructors receive the same concealed refusal.

- M5 added the Instructor Course Membership-authorized theme mutation path. It persists one valid
  closed-vocabulary theme independently and rejects unknown theme IDs without changing stored
  appearance.

- M6 added Course Banner upload, replace, remove, and authorized rendition delivery. The durable
  PostgreSQL-plus-MinIO saga binds uploads to their Course and Instructor, validates decoded image
  bytes, promotes one source with complete hero/card renditions, and leaves uncertain external
  work repairable rather than exposing partial banner state.

### Fixes and Maintenance

- Applied the six-pass Course Appearance audit's low-risk cleanup: named both banner previews,
  reused the existing rendition dimensions during normalization, clarified preview URL ownership
  and Banner Store/upload documentation, and removed execution labels from changed code comments.
  The review also updates stale route, ownership, classification, and concurrency documentation.

- Closed Course Appearance documentation and evidence ownership: the durable contract, design,
  terminology, persistence inventory, roadmap, test-evidence model, Ribbon ledger, and screenshot
  manifest/receipt/atlas now describe the delivered current-state model; the completed
  [Course Appearance plan](archive/cryptic_foraging_hennessy.md) is archived. Final working-tree
  evidence passed `./check_codebase.sh` (362/362), Rust fmt/check/test/strict Clippy, and 6,090
  Python tests; fresh M11 production-browser, appearance HTTP plus `2026090904` migration, and M6
  PostgreSQL-plus-MinIO saga acceptance also passed with zero project containers or volumes left.

- Made `devel/capture_screenshots.sh` the single capture and replay operator contract. Publication
  validates the complete staging corpus before promotion, retains a full recovery backup during
  the portable multi-path replacement, rolls back ordinary failures, and rejects stale recovery
  evidence instead of claiming unsupported filesystem atomicity.

- Expanded screenshot validation around semantic state, same-origin navigation and transport,
  page errors, closed privacy profiles, canonical dimensions, exact output closure, duplicate
  bytes, manifest-to-registry closure, and deterministic gallery generation. Live verification
  reports byte differences for review without using pixel equivalence as a machine gate.

- Closed the pre-merge screenshot audit findings without adding another orchestration layer. The
  publisher now rejects unknown root files, folders, and role-folder entries; its ordinary
  mid-replacement rollback is fault-tested; receipt tampering and privacy-profile selection have
  focused coverage; and the focused corpus suite is discovered by the canonical Node test lane.

- Replaced the Student grading sleep with the existing state-based Playwright polling idiom. The
  two workflows that intentionally navigate immediately after loading role data now wait for and
  drain those exact completed responses before navigation, so privacy inspection remains strict
  without racing response-body disposal.

- Added selector-contract source citations to the four scenario-family modules and documented the
  intentional same-origin scoped-support capability preparation. Updated screenshot troubleshooting
  and removed a stale direct JavaScript capture command from an older active plan.

- Recorded the Assignment Attempt time limit and unlimited-retry learning model redundantly in the
  Student guidance so an independently read role section preserves the intended practice policy.

- Archived the completed
  [archive/durable_live_demo_screenshot_corpus.md](archive/durable_live_demo_screenshot_corpus.md)
  with its final verifier summary, closeout gates, and implementation findings. The replay atlas
  and all 52 replay PNGs remain in the ignored `test-results/screenshot-corpus/verify/` review lane.

- Corrected the Assignment Attempt completion trigger's execution authority
  with forward migration `2026090901`. Authorized PLE, WeBWorK, and iMathAS
  Question Backend grading writers now apply the released Assignment
  Completion Rule through `ple_private_owner`, the Database Schema Owner Role
  for `ple_private`, without receiving direct `completed_at` update access.

- Added migration-catalog evidence for the trigger's exact owner,
  security-definer mode, search path, and closed execute ACL. The existing
  iMathAS PostgreSQL Store test continues to prove that one committed Grading
  Result records its Question Statistics Observation exactly once and now also
  reaches the completion invariant successfully.

- Renamed `source_me.sh`'s temporary repository-root variable so sourcing the
  environment inside a launcher that owns a read-only `REPO_ROOT` no longer
  emits a shell assignment error during an otherwise successful acceptance
  lane.

- Scoped the Course Appearance bucket initializer to an explicit Compose
  profile and disabled its pseudo-TTY. Its one-shot container remains disposable
  with `--rm`, while the default cross-store teardown no longer asks Compose to
  remove that already absent service container or emits interactive-terminal
  noise.

### Removals and Deprecations

- Applied the permanent-test checklist to Course Appearance evidence. Removed two trivial Rust
  response-constructor tests, the Ribbon descriptor/task snapshot, palette/control inventories,
  exact banner and Ribbon geometry assertions, SQL source-body matching and default/constraint-name
  snapshots, and their unused harness accessors. Retained checks protect observable protocol,
  authorization, state-transition, and cleanup behavior in their fast or explicit E2E lanes.

### Decisions and Failures

- Classified rebuild/sizing probes as one-time evidence and withdrew the audit's demand to give
  every focused browser script a canonical runner. Directly invoked behavioral E2E checks remain
  separate from permanent fast tests; old rebuild receipts do not require permanent probe code.
  The theme-response, expired-upload cleanup, and production-accessibility findings remain open.

- The independent [Course Appearance audit](active_plans/audits/course_appearance_six_pass_review.md)
  found that theme-save responses hide an existing banner from the page cache and expired abandoned
  uploads lack an executable cleanup consumer. Production-page accessibility coverage and a maintained
  execution owner for the focused browser checks also remain open. These findings qualify the earlier
  completion receipt despite the passing aggregate; the audit records owners and validation criteria.

- M3 selected a clean pre-production forward migration with no legacy banner backfill or
  compatibility rendition. Measured canonical-profile specimens selected a 1200-by-200 (6:1)
  hero and 1000-by-400 (5:2) card rendition, using server-derived centered crops and previews
  rather than automatic focal-point detection or a crop editor.

- The restored read-path investigation found that the browser had long requested Course Appearance
  while no server handler existed. It also found that prior agent-authored revision, strong-ETag
  compare-and-swap, and current-pointer contract prose described architecture the repository had
  never implemented; this capability intentionally stores current theme state without appearance
  revisions or compare-and-swap.

- Connected Course Banner saga work exposed and corrected PL/pgSQL name ambiguity, a private-table
  boundary violation, a reference to a nonexistent digest function, and repair sequencing/state
  errors. The final fresh PostgreSQL-plus-MinIO acceptance exited successfully after those repairs.

- Closed the screenshot-corpus plan without extending its architecture from implementation-run
  visual observations. Human review of the 52-image atlas is the next separate activity; concrete
  UI defects or coverage gaps discovered there become focused work of their own.

- Six independent Plan, Test, Style, Documentation, Legacy, and Comment audit passes found no
  blocker or high-severity issue. Live verification then exposed a navigation race in the privacy
  monitor: completed JSON response bodies could be discarded if a workflow immediately left the
  page. Starting reads for every response was rejected after canceled traffic could wait
  indefinitely; exact request-completion boundaries fixed the demonstrated cases without weakening
  privacy checks.

- Diagnosed the 2026-09-08 aggregate failure as an execution-owner mismatch:
  the accepted completion trigger ran as `ple_api_owner`, whose deliberately
  narrow Assignment Attempt privilege permits a row lock but not mutation of
  `completed_at`. The repair preserves that least-privilege boundary in a
  forward migration instead of changing accepted migration history or
  widening API and worker table grants.

### Developer Tests and Notes

- After classifying and pruning implementation-only Course Appearance checks, the exact
  `source source_me.sh && ./launchers/all_test.sh` command passed with exit code 0, including
  fresh PostgreSQL and MinIO/Banner saga acceptance and disposable-resource cleanup. The four
  focused browser scripts and Ribbon visibility/contrast check also passed. Independent review
  retained keyboard theme selection and named non-color state as required durable behavior,
  without restoring option-order or geometry snapshots. Three substantive audit findings remain
  open; the missing-runner finding was withdrawn.

- After the audit's preview-label and comment cleanup, all four focused Course Appearance browser
  scripts passed: theme transitions, banner behavior, accessibility, and scope recovery. Chromium
  required execution outside the filesystem sandbox because macOS denied its Mach-port startup.
  These are component-harness checks, not production-route accessibility acceptance.

- The post-audit `source source_me.sh && ./launchers/all_test.sh` rerun passed with exit code 0:
  Rust checks, strict Clippy, tests/doctests and Wasm; all 362 Node tests; 6,091 Python checks;
  fresh PostgreSQL acceptance; and MinIO/Banner saga acceptance. Both disposable service lanes
  cleaned their resources. An independent review accepted the bounded cleanup; the four substantive
  audit findings remain open because these gates do not cover their missing behaviors.

- `source source_me.sh && ./launchers/all_test.sh` passed end-to-end on the current mixed working
  tree, ending `PASS: complete live acceptance is green.` It covered 416 generated types and three
  fixtures; default, all-target, and all-feature Rust checks; strict Clippy; workspace/all-feature
  tests and doctests; Wasm; the frontend gate; 6,090 Python tests; fresh PostgreSQL apply/no-op,
  catalog/restricted probes, and three iMathAS database tests; plus MinIO conformance and the real
  Course Banner saga. Both disposable lanes cleaned their containers, volumes, and networks.

- `node --import tsx --test tests/test_screenshot_corpus.mjs` passed all nine focused corpus tests.

- `./check_codebase.sh` passed strict TypeScript, ESLint, Prettier, and all 359 Node tests after the
  screenshot-corpus integration.

- `./devel/run_playwright_tests.sh` passed the complete serial production-browser suite against a
  fresh stack: authorization, Instructor authoring, native Student recovery, Assignment release,
  WeBWorK rendering, Instructor Account lifecycle, scoped support, invitation export, and Course
  seed journeys. An initial restricted-shell launch was denied by macOS Mach-port sandboxing before
  the first browser action; the unrestricted rerun passed without an application failure.

- `./devel/capture_screenshots.sh` published all 52 captures from a clean Live Demo with role counts
  `3/19/23/7`, no unmanaged or byte-identical active PNGs, a bound receipt and atlas, and clean
  stack teardown.

- `./devel/capture_screenshots.sh --verify` passed static validation and all 52 live replay
  checkpoints from another clean stack, preserved the tracked corpus, reported five byte-level
  differences for human review, retained the temporary atlas, and proved teardown.

- `source source_me.sh && ./launchers/all_test.sh` passed the final closeout tree: 416 generated
  Rust-owned TypeScript contracts, three fixture contracts, Rust formatting/checks/strict Clippy/
  tests/doctests, the browser Wasm target, 359 Node tests, 6,004 Python tests, PostgreSQL migration/
  authority/persistence acceptance, and PostgreSQL-plus-MinIO Course Appearance coherence. Both
  real-service lanes removed their disposable resources, and the aggregate ended with
  `PASS: complete live acceptance is green.`

- The final post-audit `./devel/capture_screenshots.sh --verify` replayed all 52 captures, reported
  five non-gating byte differences, preserved the tracked corpus, retained the temporary atlas,
  and ended with a clean owned stack.

- The focused Markdown-link, ASCII, and whitespace suite passed 1,886 checks, and
  `git diff HEAD --check` passed on the mixed staged and unstaged working tree.

- `source source_me.sh && python3 local_stack.py acceptance` passed the fresh
  74-migration apply, no-op replay, PostgreSQL 17 catalog and restricted-login
  probes, all three iMathAS Question Backend PostgreSQL Store tests, cleanup,
  and the PostgreSQL-plus-MinIO Course Appearance coherence oracle.

- `source source_me.sh && ./launchers/all_test.sh` passed on the final material
  tree: 416 generated Rust-owned TypeScript contracts, three tracked fixture
  contracts, Rust formatting/checks/strict Clippy/tests/doctests, 350 Node
  tests, 5,969 pytest cases, the disposable PostgreSQL 17 schema/authority/
  persistence lane, and PostgreSQL-plus-MinIO Course Appearance coherence.
