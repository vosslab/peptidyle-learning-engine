## 2026-09-12

### Additions and New Features

- Added Base Assignment Policy autosave that persists only valid policy values,
  labels saving, invalid, rejected, failed, and conflicted drafts, and keeps
  Check release and Release unavailable until the visible policy state is saved.

- Completed the WeBWorK opaque backend-owned lifecycle. PLE now delivers an exact authorized
  backend document, bridges generic ordered form pairs, persists the bounded opaque response, and
  sends it to the WeBWorK adapter for grading. The adapter owns WeBWorK response validation and
  renderer interaction; PLE does not interpret PG controls.

- Added the WeBWorK document and two-prefix asset routes, the generic embedded-document baseline,
  and the reviewed `ple_embed` renderer format. Generated renderer assets remain usable through
  PLE while WeBWorK and Question-authored CSS retain document presentation ownership.

- Replaced the accumulated development migration history with a modular,
  PostgreSQL-native canonical base under `schemas/base_schema/`. Its short
  `install.sql` manifest installs the current domain modules directly; the empty
  `schemas/migrations/` directory is reserved for bounded forward-only SQLx
  changes after the first human-approved production deployment.

- The explicit installation-data command provisions the complete known-good
  Live Demo by default as ordinary product data on the canonical schema. Its
  `--without-live-demo` choice leaves the same schema with no Demo product-data
  roots or persona surface.

### Behavior or Interface Changes

- The Assignment Question Editor now protects structural Question changes with
  Save, Discard, and Stay navigation choices.

- Course Appearance theme updates preserve the complete Course Appearance,
  including its current Course Banner.

- Assignment Overview, Questions, and Policies are now backed Ribbon
  destinations; the regenerated destination ledger records their availability.

- Question Type is immutable author-declared educational metadata on each Published Question
  Revision. PLE uses it for search, filtering, and labels; it never infers it from a backend
  document, control, or response shape.

- The Live Demo publishes four WeBWorK Questions alongside native PLE Questions. Its automatic
  sample activity selects PLE-native Questions by model semantics, while connected WeBWorK
  behavior uses the ordinary backend-owned lifecycle.

- The current product model retains immutable Question Revisions and
  save-created Blueprint Revisions. Complete valid Blueprint creation atomically
  produces an Available lineage and Revision 1. A changed explicit Save creates
  the next Revision; a canonical no-op returns the current Revision with
  `changed: false`; there is no persisted Blueprint Draft or separate Blueprint
  publish action. Course and Assignment configuration remain current state, and
  Assignment Attempts and Issued Questions retain the exact evidence needed to
  interpret Student Work after released Assignment edits.

- Blueprint Module and Assignment References are durable course-wide identity
  across Revisions: a move preserves its reference while delete-and-recreate
  allocates a new one. New Course Instances pin only the advertised current
  Blueprint Revision; existing Instances retain their exact historical pins.
  Blueprint short name, long name, and availability share one opaque metadata
  ETag and never create a Revision.

- Blueprint Save uses one unversioned canonical pre-production content encoding,
  checksum, and exact aggregate comparison. The Store-backed Live Demo seed
  creates Revision 1 through the ordinary lifecycle, and the Blueprint editor
  protects dirty local work during initial-creation Close and Escape,
  navigation, Back, reload, and window close.

- Published Question and Blueprint availability now belongs to their stable
  lineages. Archive removes ordinary selection while existing exact Revision
  references remain resolvable; restore is an ordinary availability transition.

- Assignment Unrelease is an authorized, ETag- and title-confirmed transaction
  that deletes only the Assignment's Student Work closure, rebuilds surviving
  statistics, and records a redacted aggregate audit event.

- Question IDs are stored as compact valid seven-character Crockford Base32
  values; `AAA-BBBB` is their presentation form. Pilot Questions use ordinary
  generated IDs, and `PNE-*` is no longer a product identifier namespace.

### Fixes and Maintenance

- Repaired a renderer-token reflection regression at the WeBWorK HTTP-adapter boundary. Exact
  renderer JWT values are rejected before an embed document can be returned or persisted.

- Repaired the PostgreSQL/Rust Blueprint revision-number agreement and the
  forced-RLS Course read required by the security-definer Assignment lifecycle.
  Assignment create, save, inline save, and release now share the intended
  current Instructor authorization boundary without broadening application-role
  write access.

- Course Invitation acceptance now grants its no-login API owner update access
  only to the immutable invitation identity needed for `FOR UPDATE` locking.
  Concurrent claims both converge on one Student Membership and one acceptance
  event; the application role retains no direct Invitation update privilege.

- Simplified database lifecycle handling to one baseline-presence and release-
  identity handshake under the existing advisory lock. SQLx owns dirty-row,
  checksum, and unknown-version enforcement; the application and migrator verify
  the same restricted schema-state projection. The former catalog classifier,
  legacy-ledger scan, and exact-ledger-shape machinery were removed.

- The existing `pre-production` release identity is now the complete baseline
  editing switch: initialization installs or verifies the canonical base and
  forward migration commands remain unavailable. The base transaction also
  rejects unrelated persistent relations or custom schemas, so an accidental
  install into a populated database rolls back without leaving PLE structure.

- Course Banner and Profile media Store acceptance now uses the application
  login for runtime operations and the migrator only for owned fixture setup and
  catalog inspection. Their PostgreSQL functions and Rust return types agree on
  exact work identities, and cleanup receipts report retained versus absent
  objects without manufacturing generic cleanup jobs.

- Consolidated the two libpq child-environment builders into one private,
  redacted implementation shared by baseline and installation-data commands.

- Removed retired Course Schedule, Assignment, Question Change Proposal, and
  Course Retention Revision scaffolding from the baseline, application contracts,
  fixtures, and tests. Retired migration-history and compatibility tests yielded
  to focused lifecycle, authorization, evidence-integrity, and product behavior
  coverage.

### Removals and Deprecations

- Removed speculative Grade Settings and Teaching Operations routes, pages,
  browser/API code, generated DTOs, and unsupported model code. Their direct
  URLs now use ordinary not-found. Separate pending Account Invitations and
  shared enrollment domain concepts remain supported.

- Removed the WeBWorK HTML projection/replay layer, replay persistence, renderer cache plumbing,
  and projection-era browser tests. The compact permanent suite now protects the opaque boundary
  rather than a catalog of PG interaction shapes.

- Applied the permanent-test checklist to the completed database reset and removed
  checks that froze retired command names, generated SQL inventories, exact
  tool paths or network constants, Pilot-content counts, SQLx internals, or mocked
  lifecycle choreography. Durable schema compatibility, authorization, redaction,
  evidence integrity, provisioning, and Unrelease contracts remain covered at
  their lowest meaningful layer.

- Removed unused Live Demo bootstrap request builders and the unowned iMathAS
  PostgreSQL test target. Their only consumers were tests of dormant
  scaffolding, not supported product or acceptance paths.

### Decisions and Failures

- Added a concise execution goal beside the Phase 1 Instructor safety and
  truthful UI plan. The goal keeps Human Guidance and the Terminology Contract
  authoritative and leaves the plan as the primary source for scope and
  implementation detail.

- M2 selected C2, the same-origin iframe with its document CSP, and E1, stateless one-grade-request
  submission. The temporary behavior corpus and a failed privileged SQL oracle were demoted: they
  duplicated stable M11/M12 evidence without a product-facing recovery action.

- The renderer fork remains limited to demonstrated embed-document and asset requirements, keeping
  future upstream WeBWorK updates practical to integrate.

- Retained the 64 KiB raw UTF-8 bound for the canonical browser-captured backend-owned response
  payload. It is separate from the 256 KiB PG/PGML source and 1 MiB renderer document/envelope
  limits; the shared boundary can be deliberately revised if real supported response data reaches it.

- Rotated the September 10 and September 9 day blocks into
  `docs/CHANGELOG-2026-09g.md` after the active changelog exceeded its 800-line
  threshold. The active file retains the two most recent day blocks.

- Added the active production release-readiness phase plan. It groups the nine
  remaining readiness items into four ordered implementation phases with
  milestones inside each phase, keeps the opaque WeBWorK implementation as its
  own active lane, and brings only its Fall teaching walkthrough into the
  release-readiness sequence. The plan centers PLE backend and browser behavior,
  uses valid-change autosave for simple Assignment policies, and excludes
  provider-specific deployment implementation.

- Before the first approved production deployment, structural changes belong in
  their owning base-schema module. The production-release decision freezes that
  source and starts forward-only SQLx migrations; it is the only remaining human
  release decision for this reset.

- Live Demo SQL owns only state wholly owned by PostgreSQL. Ordinary publishing,
  object storage, presentation, submission, and grading paths own their
  cross-system effects. The resulting demo data follows ordinary product
  lifecycle and retention rules.

### Developer Tests and Notes

- Rotated complete September 12 and September 11 day blocks into
  `docs/CHANGELOG-2026-09h.md` after the active changelog exceeded its 800-line threshold;
  the active changelog retains the September 14 and September 13 blocks.

- WP-G1 passed 88 server tests, 88 learning-data-access unit tests, strict all-target/all-feature
  Clippy, and the canonical fresh PostgreSQL 17 baseline. The connected listener oracle used the
  native worker Service Identity, ignored a WeBWorK notification, and accepted the matching
  native notification. With Tokio time paused, two trivial ready Jobs committed back-to-back with
  zero elapsed task time, proving removal of the former deliberate three-second floor. Fake Store
  tests also prove evaluation-failure and `LeaseLost` continuation, immediate idle shutdown,
  prompt commit draining, and bounded recovery from a hung Store call.

- The corrected WP-F4 oracle proves Student and Course isolation, read-only Instructor status,
  terminal-failure refusal, absence of public and private grading-mutation functions, and unchanged
  `md5(student_response::text)`. Worker lifecycle evidence separately proves automatic
  unfinished-Job requeue, stale-token refusal, and terminal results.

- WP-E0 exact-boundary domain tests, focused Rust and PostgreSQL compile/lint gates, and the
  canonical fresh PostgreSQL 17 database baseline passed. The connected access oracle verifies
  the full restricted Student projection, completed-Attempt counting, effective Student IANA
  time zone, and close/available/limit/late precedence.

- WP-E1 generated 332 browser contract types and passed the strict TypeScript no-emit check,
  12 focused Node decoder/HTTP tests, Rust contract and server checks, warning-denying Clippy,
  the 407-test `./check_codebase.sh` front door, and the canonical fresh PostgreSQL 17 baseline.
  The connected oracle proves scheduled visibility, access/landing decision agreement, and
  cross-Student accommodation isolation.

- WP-E2 passed 17 focused Node tests, including server-side rendering of one instant in two
  Student zones and exact public-reason copy, plus the 408-test `./check_codebase.sh` front door.
  The headless Chromium gate proved identical due copy on landing and pre-start pages, placed the
  time-limit explanation before Start in document order, and reported no serious or critical
  axe accessibility violations on either surface.

- WP-E4 passed the Student-only Rust route test, warning-denying Rust checks, three focused Node
  profile/model tests, and headless Chromium save/re-render evidence with no serious or critical
  axe violations. The canonical fresh PostgreSQL 17 baseline passed with a connected oracle for
  the inviting-Instructor default, existing-Account preservation, Student self-update, and
  Instructor refusal. One initial aggregate run exposed a fixed test-only session-token collision;
  distinct fixture tokens resolved it before the passing clean rerun.

- Connected Assignment browser and service evidence, the Course Appearance
  browser scenario, focused Rust/Node/type/ledger gates, `./check_codebase.sh`,
  and the final `./launchers/all_test.sh` aggregate passed for Phase 1. The
  aggregate covered generated TypeScript contracts, Rust checks and tests,
  Python tests, disposable PostgreSQL baseline and installation-data paths, and
  Course Appearance PostgreSQL/MinIO acceptance.

- M11 boundary checks and accepted M12 connected curl/browser evidence cover render, capture,
  Save, Finish, assets, styling, document headers, grading, PostgreSQL persistence, and the Student
  history outcome. The final reviewed renderer OCI is
  `296f4f4bba83563aec7092c7e89de128bac810a62e0f3215df784874490e4df3`.

- Final permanent-suite pruning retained only stable opaque lifecycle, bounded wire,
  credential-boundary, browser capture, and Question Type separation contracts; speculative
  capability/state inventories and implementation-sequencing checks were removed.

- The post-pruning `source source_me.sh && ./launchers/all_test.sh` rerun exited 0: Rust checks,
  404 Node tests, 6,066
  pytest tests, disposable PostgreSQL baseline, ordinary installation-data provision/opt-out, and
  Course Appearance PostgreSQL-plus-MinIO acceptance all passed, ending `PASS: complete live
acceptance is green.`

- The final six-perspective audit found no comment blocker and identified
  lifecycle, installation-data, race-determinism, duplicated libpq parsing,
  documentation, and dead-scaffolding defects. Each accepted finding was fixed
  in its owning code or canonical document; no parallel audit-report layer was
  retained.

- Final PostgreSQL 17 baseline acceptance passed contaminated-database refusal,
  fresh installation, no-op
  replay, restricted application-role verification, catalog authorization,
  authoring source binding, populated Unrelease closure, and its deterministic
  Assignment-lock race. The focused installation-data lane passed default
  provisioning, replay convergence, and a separate fresh `--without-live-demo`
  absence/non-enumeration proof.

- Connected database, installation-data replay and opt-out, service, and
  Playwright evidence passed the Blueprint Revision lifecycle: atomic Revision 1
  creation, changed and no-op Save, stale-write rejection, archive/restore,
  stable child identity, current-head Course Instance pinning, dirty-work
  protection, and retained historical provenance. Backup restore followed by
  ordinary migrate and application-role verification passed. The complete gate
  regenerated 330 Rust-owned browser types and passed strict Rust checks, 406
  Node tests, 6,044 Python tests, the canonical PostgreSQL baseline,
  installation-data provision/replay/opt-out, and the PostgreSQL-MinIO coherence
  oracle.

- A forced fresh Graphify extraction removed the old migration forest from the
  architecture view. The semantic closeout checks found zero nodes from
  `schemas/migrations/`, `public._sqlx_migrations`, retired
  Assignment/Course Schedule/Proposal/Retention Revision families, Blueprint
  collaboration scaffolding, or `PNE-*` identifiers.

## 2026-09-11

### Additions and New Features

- Completed M10 Assignments Due Soon. The current Instructor's Product-scope Assignments page
  lists only Assignments from Courses they currently teach, in due-instant order, with Course
  identity, Assignment status, Account-zone deadline, public Course and Assignment links, an
  explicit empty state, and local retry recovery. The service remains the exact small
  Instructor-only projection; it carries no Student, Attempt, response, grading, or answer data.

### Behavior or Interface Changes

- The authenticated top bar now presents Product Role once in its boxed plate. Instructor Profile
  is the far-right accessible icon-only rounded-square control after Sign Out; it shows the generic
  user glyph until the existing self-only uploaded thumbnail is available. Browser written UI now
  uses locally bundled Atkinson Hyperlegible Next normal and italic variable fonts, while explicit
  monospace and backend, native-renderer, and export typography remain intentional exceptions.

- Course Instances now have required, independently entered short and long names at the root
  schema, model, and API boundary. The short name supplies the constrained persistent Ribbon scope;
  the long name supplies breadcrumbs, headings, and descriptive rows. There is no derived value,
  fallback, or compatibility alias, and Blueprint naming semantics are unchanged.

- Implemented M7/WP-ACC2 Student Assignment Start and history UI. The Start page now presents the
  server-authorized Assignment title, questions, points possible, and time limit before its single
  primary Start action, with prior Attempt links following that action. The retained summary route
  reads direct `R-n`
  history, presents only server-disclosed scores, responses, and teaching fields, and uses the
  authorized Course and Assignment public references for its return path, Ribbon, and theme.

- Implemented M7/WP-ACC1 readable selected-history responses. A Student whose
  `submitted_response` timing is released now receives only readable content
  from the exact pinned issued presentation for that owned completed Attempt.
  Reproduction, source, checksum, or asset unavailability omits that response
  while retaining the Attempt spine and independently released current grades.
  Raw private answer and feedback content, canonical response IDs, and source
  locators remain below the server boundary; independently authorized safe
  response and teaching projections cross only through their disclosure gates.
  Completed owned response images use the existing ready-asset authorization path
  with active membership.

- Implemented M7/WP-ACC1 native PLE selected-history teaching content. Independently released
  feedback and correct-answer fields now derive from one exact authorized source read without
  regrading; selected-choice feedback can appear without an outcome, while outcome feedback uses
  only recorded correctness. Unavailable PLE explanations and all WeBWorK teaching content remain
  absent, as do individual fields whose pinned source, response, or recorded result is unavailable.

- Completed M11 inline Assignment title and due-date editing. The Course Assignment list now edits
  a released Assignment's title and raw local due value in place with the current Assignment Edit
  Number, current-state save semantics, and no revision or undo entry. New Attempts retain their
  started title and due instant, including a real null no-deadline value; legacy Attempts retain
  their exact released revision fallback. Final aggregate acceptance passed Rust, 423 Node tests,
  6,320 pytest tests, the 96-migration PostgreSQL/MinIO/Profile service lanes, and cleanup.
  Disposable browser acceptance proved raw milliseconds, title-only precision, null deadline
  clearing, the new-date 11:59 PM default, keyboard cancellation and focus return, busy state,
  retained-draft 503 retry, and 412 refresh/retry with no unexpected HTTP or page errors.

### Fixes and Maintenance

- Removed the superseded Live Demo foundation audit after its RLS and default-deny conclusions were
  confirmed in the active database-reset plan and canonical authorities. The historical report
  linked a retired migration acceptance test and no longer served as durable guidance.

- Archived the superseded terminology reconciliation plan, its draft successor, and its concern
  note after the accepted database-baseline plan consolidated their decisions.

- The dedicated `database-migrator` image now contains the Debian 13 PostgreSQL 17 client needed
  to execute the canonical base-schema manifest. API, Live Demo, and production targets still
  inherit the client-free non-root runtime base. A built migrator reported `psql 17.11`; the
  runtime-base target confirmed that `psql` is absent and UID 10001 remains active. A full
  production-target rebuild was skipped after its cold Rust compilation exceeded this focused
  container gate; its direct runtime-base inheritance was verified instead.

- The final scoped top-bar audit removed the obsolete `RibbonIcon` compatibility re-export, updated
  its design specimen from retired Account context to Profile, documented the replacement-event
  race guard, and kept the separate Course-name contract classified under Interface Cleanup M2.

- Final M9 visual review repaired the Course action's insufficient contrast and the 393 px Student
  Ribbon clipping that hid the final character of the Course short name. Recapture and independent
  review accepted both repairs; the 49-path semantic/privacy replay remains the evidence boundary.

- Repaired the M7 Student Course landing score disclosure found by the M9 privacy gate: the
  previously unconditional `pointsEarned`/`pointsPossible` projection now emits an all-or-none
  score only when trusted PostgreSQL `feedback_score` permits it and the complete unique immutable
  grading lineage is present. Strict Rust, TypeScript, and UI contracts keep the projection
  coherent. Focused inline fixture-policy acceptance passed in session 15668; the owner declined a
  speculative full timing-permutation matrix. M9 evidence is recorded below; Course retention is
  deferred as a separate capability.

- Reconciled the active Interface Cleanup tracker with accepted M7, account-zone, Profile,
  thumbnail, M9 evidence, and M10 receipts. M19 is owner-deferred and routed as Course Retention
  rather than represented by a misleading interface-only Course status.

- Applied the scoped documentation and comment-audit corrections: durable documentation now states
  current-state Assignment editing accurately, architecture inventories the Profile, selected
  history, and Due Soon boundaries, and public Profile Rust items document their self-only and
  concrete-store responsibilities.

- Completed a fresh scoped six-pass maintenance audit. Its accepted repairs remove two redundant
  M11 source-label assertions, correct evidence status and module inventories and the Due Soon
  exact-key record, and clarify Profile Thumbnail Store rustdoc and migration comment tags.
- Synchronized shared style guides, tests, and repository support files from the starter template.

- The six-pass scoped audit checkpoint found stale M9 and M19 documentation, a dead legacy
  preview-plane client surface, and one timer-generation comment; Test and Style passes found no
  issue. This documentation records the evidence and scope dispositions. Product cleanup remains
  subject to its separate current-source review.

### Removals and Deprecations

- Removed the unbacked generic browser Course-create client surface; the supported Course Instance
  creation path remains the only browser contract.

- Removed the dead generic Assignment Attempt Summary transport and its decoder-only surface while
  preserving the live `/assignment-attempts/R-n/summary` browser presentation route and its pinned
  history endpoint. The separately found duplicate local date-time parser name is deferred by
  owner direction.

### Decisions and Failures

- The human owner directed that investigation, review, and disposition materials remain temporary
  working evidence outside the repository. Accepted conclusions are folded into the active plan,
  canonical documentation, code, behavior-focused tests, and final changelog evidence only after
  checking whether an artifact duplicates or supersedes existing evidence.

- The approved fresh-installation target builds DDL-only structure, then its production-installation
  orchestrator defaults to the complete known-good Live Demo with an explicit opt-out. One
  data-only manifest may create all wholly PostgreSQL-owned teaching state after the Pilot
  compiler/object publisher establishes exact Question Revisions; existing owner paths create only
  genuine cross-system effects. The private bootstrap/local persona selector is absent before a
  public gateway. This records an accepted boundary, not completed provisioning implementation.

- The completed Blueprint-surface review clarified that `blueprint_collaborator_event` is
  revision-keyed schema scaffolding, not a live vertical capability. It leaves the new baseline;
  the owner Draft lifecycle remains, and a future collaboration feature requires a bounded
  Draft-keyed Store, Server, authorization, browser, and publication-transition design.

- The human owner approved the Database Baseline and Revision Model Reset plan and delegated
  implementation. Before the first production deployment, its canonical base schema and required
  reference seed data remain directly editable; later structural changes will use forward SQLx
  migrations.

- The Course-name replacement is an owner-directed pre-production baseline correction: it changes
  the disposable root schema directly rather than retaining a compatibility path. Full fresh and
  no-op migration plus aggregate gates passed. Blueprint short/long names remain a future mutable
  Blueprint-metadata decision and do not change current Blueprint Revision content or checksums.

- The mutation-heavy clear/zone-switch browser harness was denied as optional and outside M10
  acceptance. The accepted replacement is materially safer read-only replay against the existing
  fixture; Profile zone changes remain M17 evidence.

- M9/WP-EVI1 and WP-EVI2 are accepted. Session 9103 found no serious or critical axe issue on all
  four changed Student surfaces. The canonical replacement publication contains exactly 49 paths
  and receipt manifest digest
  `34692bd7355560ed67b57bd41259fc3dccd88373ced50f1de2ad9e7b8eb773dc`.
  Semantic and privacy `--verify` replay passed all 49 while preserving the published files;
  independent image review accepted all 18 Student and all 31 Instructor/shared captures after
  rereviewing the two corrected images. Five replay-only byte differences are informational
  human-review evidence, not a pixel gate.

- WP-EVI3 keeps axe in the explicit connected Playwright evidence lane rather than
  `check_codebase.sh`: it needs a live browser and service, and a fast-lane failure has no useful
  recovery action. M19 is owner-deferred. Existing Course Retention Plan Revision, typed job/lease,
  and retention-event scaffolding are preparation only; no executable atomic Course-wide
  FERPA-stripping transition or Course-bound receipt that attests that transition exists. The
  existing `course_retention_event` is indirect preparation, not attestation that stripping
  occurred. Active and Inactive destinations remain honestly Unavailable until a separate Course
  Retention capability supplies that boundary.

### Developer Tests and Notes

- `node --import tsx devel/generate_ribbon_destination_ledger.mjs --check` passed with `Ribbon
destination ledger generated section is current.`

- Final aggregate receipt: `./launchers/all_test.sh` exited 0 with 432 Node checks, 6,456 pytest
  checks, Rust/Wasm, fresh/no-op PostgreSQL authority and persistence, MinIO, Course Appearance,
  and Profile Thumbnail. Disposable cleanup completed; no containers or pods remained. An initial
  source-file line-limit failure was repaired by the dedicated font stylesheet; `src/style.css`
  finished at 999 lines.

- Focused TypeScript, lint, 22 Ribbon tests, and build passed. One-time connected Instructor proof
  covered Sign out then the icon-only Profile control, generic fallback, `POST` 200, visible
  success, same-document avatar replacement without reload, navigation persistence, and local
  Atkinson normal/italic WOFF2 delivery. Corrected rendered runtime-injected monospace proof passed.

- Published 49 screenshot paths and passed semantic/privacy `--verify`; five byte differences were
  retained only for human review. Independent visual acceptance passed.

- Final connected-browser receipt: on 2026-09-11, the owner ran
  `./devel/run_playwright_tests.sh` against a fresh disposable HTTPS stack; it exited 0. All four
  Playwright scenarios passed: authentication and authorization, Instructor authoring, Course
  Appearance propagation, and learner native-PLE recovery. The maintained Assignment release,
  WeBWorK render, Instructor Accounts, support capability, invitation export, and Course-seed
  journeys also passed.

- M10 acceptance combines the accepted WP-DUE1 PostgreSQL 17 fresh/no-op migration and revocation
  receipt with a final read-only browser run at `https://localhost:55104` that exited 0. It proved
  the current Instructor-only view, Account-zone rendering, status/order, links, and one expected
  intercepted `503` retry with no unexpected HTTP, page, or console errors. Independent visual
  review passed the current populated, error, and recovered captures. A same-build empty capture
  is historical evidence only. The six scoped Plan, Tests, Style, Documentation, Legacy, and
  Comment audit reports were reviewed; their accepted fixes are recorded above.

- Accepted M10/WP-DUE1's narrow cross-course Due Soon read path. The authenticated Instructor-only
  `GET /api/assignments/due-soon` response is exactly `{ items, nextCursor: null, displayTimeZone
}`; each item contains only Course reference/long name, Assignment reference/title/status, and
  `dueAtMillis`. The implementation uses an owner-selected rolling next-seven-days window for
  unreleased and released Assignments. It is not a new product-policy or human-guidance decision.
  The Store independently applies the existing active-Instructor membership predicate, so revoked
  membership removes Course rows, and returns no Student, response, Attempt, grading, or answer
  data. The reviewed PostgreSQL 17 acceptance run passed fresh and no-op migration application,
  the full catalog through migration `2026091028`, the Due Soon window/status/revocation oracle,
  restricted-login probes, persistence probes, and disposable cleanup
  (`/private/tmp/ple-interface-cleanup.QVsF3M/m10-due1-postgres-repaired.log`). Offline
  `./check_rust.sh` and `./check_codebase.sh` also passed, including 433 Node tests. The initial
  forced-RLS fixture failure was repaired using the existing authorized writer and remains only a
  diagnostic history. This WP-DUE1 receipt did not itself claim M9 evidence close-out or a current
  full `all_test.sh` run.

- M7 acceptance evidence: session 40212 `./launchers/all_test.sh` exited 0 with Rust, 431 Node,
  6,321 pytest, 99 fresh/no-op migrations, PostgreSQL, MinIO, Profile, Course Appearance, and
  cleanup lanes passing (`/private/tmp/ple-interface-cleanup.QVsF3M/m7-final-all-test.log`). The
  prior failed backend and stale-seed receipts remain historical diagnostics, not current status.
  Canonical HTTP (session 30817), answer-only and All/Never disclosure policy, and UI (session 11049) browser lanes passed at `https://localhost:55230`; `m7-ui-accepted.log` records UI
  automation passing all profiles, axe 0, focus, 503 retry, and resume. Canonical stop session
  48488 exited 0 (`m7-accepted-stop.log`). The native PLE source
  mapping now transports `question_attempt_id`, `source_object_id`, and `question_seed` as text,
  and seed consumers read semantic access facts rather than exact obsolete response shapes.

- M7 is accepted. Independent canonical visual review passed all eight actual captures, including
  the fresh 390 px Start state with no header overlap. Shared Start/history CSS is owned by the
  browser entry and compact breadcrumbs remain intact. M9 remains separate: this receipt does not
  claim its final six-pass audit or full screenshot corpus. The stale M10/Profile keyboard-order
  harness belongs to M9 evidence and M17 Profile follow-through, not M10 Due Soon scope. Next
  dependency-ordered product milestone: M10 Assignments Due Soon, then M19 Active and Inactive
  Courses.
