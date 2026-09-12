# Changelog

## 2026-09-12

### Additions and New Features

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

- The current product model retains only immutable Question Revisions and
  Blueprint Revisions. Course and Assignment configuration are current state;
  Assignment Attempts and Issued Questions retain the exact evidence needed to
  interpret Student Work after released Assignment edits. Blueprint creation
  creates a private Draft, and each deliberate publication creates an immutable
  Revision.

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

- Before the first approved production deployment, structural changes belong in
  their owning base-schema module. The production-release decision freezes that
  source and starts forward-only SQLx migrations; it is the only remaining human
  release decision for this reset.

- Live Demo SQL owns only state wholly owned by PostgreSQL. Ordinary publishing,
  object storage, presentation, submission, and grading paths own their
  cross-system effects. The resulting demo data follows ordinary product
  lifecycle and retention rules.

### Developer Tests and Notes

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

- Focused real-stack evidence passed released-Assignment retained-evidence and
  later-Attempt behavior, Blueprint Draft publication/archive/restore, authoring
  API/S3/browser publication, and WebWork worker grading. Backup restore followed
  by ordinary migrate and application-role verify passed. Aggregate Rust,
  TypeScript/Node, and Python gates passed.

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
  Canonical HTTP (session 30817), answer-only and All/Never disclosure policy, and UI (session
  11049) browser lanes passed at `https://localhost:55230`; `m7-ui-accepted.log` records UI
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

## 2026-09-10

### Fixes and Maintenance

- Corrected the Live Demo Course seed after Course-zone retirement. It now sends the strict
  date-only Course Term payload rather than the removed `term.timeZone` field, restoring canonical
  startup without changing Account-zone defaults or seeded Student-work flows.

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

- Completed M4 Assignment Settings defaults. New direct Assignments default to one-Question
  delivery and shuffled Assignment Question order; issuance
  records a stable per-Attempt sequence and resumes by released source identity. Release requires
  an Instructor-saved positive whole-Attempt time limit, without inventing a duration default.
  An exact current full-policy retry now returns the unchanged resource and Edit Number without a
  write; changed authored policy continues to advance once.
  The fresh aggregate passed 420 Node tests, 6,320 pytest tests, Rust, PostgreSQL/MinIO/Profile
  service, and cleanup lanes after bounded explicit demo-duration, `authoredOrder`, and unchanged-
  save fixes. The rebuilt browser acceptance passed seven-policy reload, raw-millisecond due-time,
  chosen-duration release recovery, repeated valid save, Question-order editing, and stable
  two-Question Student resume with restored saved work; supported runtime cleanup passed.

- Completed the M4/WP-DEF1 Assignment-schedule foundation. A new due-date draft defaults to
  11:59 PM in the Instructor's Account time zone and remains a raw local value until the server
  resolves it. Existing seconds and milliseconds survive save and reload, and failed saves retain
  the typed schedule. `Reject` remains the late-work default; a forward migration now rejects
  post-due response saves while preserving finalization of on-time saved work. The six disclosure
  controls and question-presentation settings remain in the active WP-DEF2 and WP-DEF3 packages.

- Completed the M4/WP-DEF2 disclosure package. Assignment Properties retains independent timing
  controls for score, correctness, correct answer, Question feedback, Question answer explanation,
  and class statistics; a separate previous-attempt response control governs only the Student's
  recorded response. New Assignments release score, correctness, and that response after
  submission, while answer-bearing authored content and class statistics default to Never.
  Existing explicit settings and immutable v2 Blueprint content remain usable; new Blueprint
  revisions record the strict v3 policy shape.

- Completed M18 Profile thumbnail. An Instructor can upload a useful still image at any source
  aspect, and PLE commits one centered 256 by 256 lossless WebP thumbnail behind the Profile's
  consistent rounded-square silhouette. The server owns decoded-image validation, crop, rendition,
  typed address, and authorized delivery; replacement uses the established cleanup, storage-check,
  manifest, job, and audit-receipt lineage. Browser replay proved centered wide-image rendering,
  reload, prior-reference concealment, Student denial, and actionable metadata/upload recovery.

- Completed M17 Instructor Profile. An Instructor now reads and saves their own exact IANA time
  zone through the Profile Context Control, with no selected Ribbon tab. Profile explains that
  changing the zone re-renders stored deadline instants without moving them; browser acceptance
  proved an ordinary deadline changed from Los Angeles noon to Chicago 2:00 PM while the stored
  instant remained unchanged. Self-only authorization, retained-choice save recovery, and
  Student/Sysadmin route denial passed.

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

- Completed M16 Course-zone retirement. Course Terms now retain only inclusive ordered calendar
  dates; an Instructor's local input is bounded by those dates before the authenticated Account
  zone resolves it, while persisted deadlines remain absolute instants. The forward migration
  retires the Course clock across the active schema, Rust model, generated contracts, and browser
  without altering schedule-revision evidence or its authorization boundary. Preview and Blueprint
  scheduling use the acting authorized Instructor Account zone at their existing boundaries.

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
