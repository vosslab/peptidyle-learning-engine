# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was
> changed and believed at the time. They are not product authority. Current
> intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old
> Assignment, Blueprint, lifecycle, grading, role, retention, and UI models.

## 2026-09-14

### Additions and New Features

- Added the planned Gmail API Email Delivery Backend specification. It keeps institutional email
  as the PLE Account identity and a dedicated Gmail account as delivery transport, defines the
  provider-neutral adapter, email-code ceremony, abuse controls, operator CLI, OAuth flow,
  host-protected credential file, API-only container mount, failure behavior, recovery, and
  attended delivery acceptance. Enrollment documentation and the future-capability backlog now
  point to the specification. No Gmail route, credential, or runtime behavior is claimed.

- Every installation now publishes the complete free and open-source Biology Problems Website
  Genetics Blueprint as its example course. The default installation also creates the optional
  Live Demo. `--without-live-demo` still publishes Genetics and omits the Live Demo Accounts,
  Course, and activity. Repeating provision does not duplicate the Blueprint. The exact portable
  content validation and fresh default, replay, and opt-out installation checks passed. The full
  `source source_me.sh && ./launchers/all_test.sh` gate passed in session 62894.

- Question Library now returns additional pages through its existing cursor API, so bundled
  Questions no longer cause searches to fail above one page. Server and frontend checks passed.

### Fixes and Maintenance

- Reconciled the complete `docs/` corpus against Human Guidance as current product authority.
  Current specifications, contracts, guides, and active plans now use the same Assessment,
  Blueprint, Question, role, authorization, retention, grading, and interface models. Temporary
  reports inventory all 295 documentation files, preserve compatible detail and dated evidence,
  and separate the two unlocked UI designs from source-owned generated-artifact refreshes. The
  submission language now names only the whole Assessment Attempt as the submission target; saved
  Question responses are finalized together by that action. Assessment is the generic object with
  five Types, while Assignment is not an object or parent category. Practice uses the ordinary
  whole-Attempt boundary, Question Feedback is independent of correct-answer disclosure, unanswered
  Questions receive zero without backend evaluation, and the highest submitted Attempt score is
  used. Scoring uses Question points directly; pilot export is CSV or TSV without a Course-grade
  model. The KISS reconciliation also removed invented rollover, deferred grading, and permanent
  Account-closure models; connected the six-month maximum Active Course lifetime to FERPA retention
  by capping deadline movement while keeping inactivity and deletion as separate transitions;
  clarified that the latest Assessment deadline only starts the FERPA clock while configured policy
  controls later record transitions; and preserved bulk roster import for adding Students while
  limiting removal to one Student at a time. Course banners now use one responsive
  5:1 geometry, recommend 1280 by 256 pixels for authoring, and do not preserve the former 6:1 hero,
  5:2 card crop, or dual-rendition subsystem. Six fresh independent Plan, Test, Style,
  Documentation, Legacy, and Comment reviews
  corrected additional stale authentication, submission, scoring, adapter, and report claims. The
  reports retain their current naming/placement style issue and the limits of structural inventory
  as semantic proof. This was a documentation-only change; source code, schemas, tests,
  configuration, and migrations were not modified.

- Reorganized Human Guidance under the approved Development, vocabulary, Accounts and roles,
  Interface, Data and history, Questions, Courses, and Assessments hierarchy. A temporary exact
  comparison preserved all 744 bullet blocks without wording changes, and links to renamed current
  headings were updated. The focused Human Guidance format gate passed two tests.

- Cleaned only same-subsection Human Guidance duplication and the Course Instance ownership
  contradiction. Course Instances retain equal co-Instructors with no privileged first Instructor;
  Private Blueprint ownership and intentional cross-section reinforcement remain unchanged. A
  read-only follow-up found no remaining same-subsection duplication or move-created contradiction
  requiring correction, and the focused Human Guidance format gate passed two tests.

- Course Instance creation now adopts every Blueprint Assignment atomically, preserving exact
  Question Revision pins, fixed Questions, pools, points, instructions, and policies with fresh
  teaching identities. Adopted Assignments start Unreleased with dates unset. Removed Blueprint
  relative schedules from the model, API, seed content, and editor. Blueprint discovery now sorts
  by total adoptions or Students ever enrolled, counting each Student once per Course Instance
  even after leaving and rejoining. The fresh-schema isolated PostgreSQL Store probe passed
  adoption of fixed and pooled Questions, initial state, lifetime counts, and concurrent Revision
  checks. The complete offline Rust gate (including WebAssembly), frontend gate (373 tests),
  materialization unit test, and 276 focused Python checks passed. C-3 was not repaired.

- New Live Demo launches choose their HTTPS gateway port from 8000-8399 instead of
  55000-55399. Existing running demos retain their current URL. The focused target and
  developer-controller checks passed 30 tests.

- Clarified demo role choices with Instructor Dr., Student, and Sysadmin labels, sharing teal,
  lavender, and tomato role colors with the Ribbon. Blueprint overviews now list assignments;
  owners enter Course Editor and select one assignment before editing. Names and availability
  are expandable. A direct Create Course Instance action preselects the current Blueprint.
  Removed repeated unavailable-statistics copy and reduced desktop Question Library rows from
  164 to 112 pixels. The demo controller prints and flushes its URL before any browser opener.
  Fixed Blueprint input focus loss caused by remounting the editor on each local change.
  The frontend gate passed 374 Node tests, and focused controller/documentation checks passed
  255 tests. The existing live Blueprint browser journey passed Save, conflict, metadata,
  archive/restore, and unsaved-navigation checks. A temporary real-browser probe created a
  Course Instance from BP-2, verified one selected assignment editor, and found no axe violations
  on that editor or horizontal overflow at 320, 480, 768, and 1920 pixels. Captures are in
  `test-results/interface_polish/`; this is focused evidence, not full aggregate acceptance.

- M4 and Phase 2 are complete as of 2026-09-14. The fresh canonical runtime at
  `https://localhost:55390` published `BP-2`, Revision 1, in session 53170 with the r11 hashes:
  11 topics, 119 banks, and 20,579 rows. The reviewed r5 browser proof reloaded that exact
  Blueprint, created throwaway Course/Assignment records, visibly restored a saved radio selection
  after reload and a new authenticated Mary session, navigated directly to the same Attempt, and
  used the UI to submit wrong then correct responses with Student-history/Gradebook agreement.
  Root inspected the fresh Topic 11 screenshot as legible. The r4 outage proof recorded an actual
  submission `503` and plain copy, restoration before and after renderer recovery, accepted UI
  submission, and a healthy renderer. GET receipt R6 recorded only public API observations and
  reviewed SQL evidence of no Student Work/outcome writes; it does not claim a private database
  snapshot. This remains intentionally disposable, makes no production-deployment claim, and adds
  no target/deployment gate. The earlier r3/r4 installation was canonically removed before the
  aggregate; this is the current fresh publication receipt. Authorized closeout then deleted only
  `tests/_temp/genetics_bank_port` (208,331 files in 1,490 directories, about 3.59 GiB), with no
  archive or permanent fixture. The original 11-topic Biology Problems Website Genetics source,
  runtime/volumes/`BP-2`, and Git index were untouched.

- Corrected the vendored naming gate after aggregate session 68687 stopped following 6,003 passing
  pytest tests. It had applied the permanent Playwright-placement rule to `tests/_temp`, despite
  `PYTEST_STYLE.md` requiring temporary probes there. The narrow generic correction excludes only
  the `tests/_temp` subtree and keeps permanent checks unchanged; it adds no framework, fixture,
  or import workaround. Full pytest passed 6,004 tests and the final-path boundary gate passed
  five checks; independent review approved. The exact rerun
  `source source_me.sh && ./launchers/all_test.sh` passed in session 78713. Vendored propagation
  may overwrite this local correction; upstream modification is out of scope.

- Corrected the backend-owned Question iframe sizing demonstrated by the Genetics screenshot: its
  304 by 154 frame clipped rendered content. The existing iMathAS width/minimum-height rule now
  also applies to `backend-owned-document__frame`. This is CSS-only; it adds no Question Backend
  protocol, parser, or permanent test. Independent review, Prettier, TypeScript, and diff checks
  passed. Fresh desktop and narrow browser captures after the frontend rebuild verify full-width
  desktop rendering and reachable narrow-layout document scrolling. The later exact aggregate
  passed in session 78713, and the fresh r5 integration proof completed the normal saved/graded
  delivery evidence.

- The Phase 2 audit corrected durable guidance that still described public grading status,
  Instructor attention, retry/requeue behavior, and grading Jobs. Current guidance records one
  immutable backend credit fraction, read-time current-point scoring, ordinary expiry
  finalization, and only hidden backend-specific completion. It keeps public-asset Jobs separate.
  The aggregate `source source_me.sh && ./launchers/all_test.sh` exited 0 in session 36718, but
  the audit found a focused expiry Cargo filter that matched zero tests and obsolete
  grading-job/failure residue. Follow-up removed the stale private-table inventory catalog check;
  restored the Assignment-root update authority required by the expiry lock; and found that the
  corrected expiry test executed once, while its stale global-empty assertion conflicted with two
  intentionally retained expired Attempts from the preceding race oracle. The redundant
  `attempt_expiry_store_postgres` test and runner were removed under the test policy, with no
  replacement. Existing security-catalog, expiry-SQL, race, and no-browser journey evidence
  remain. The fresh exact aggregate `source source_me.sh && ./launchers/all_test.sh` passed in
  session 38324: 6,004 pytest tests, database-baseline E2E, installation-data provision and
  opt-out, Course Appearance PostgreSQL/MinIO, and complete live acceptance. Canonical cleanup
  was empty. Approved audit reviews cover the native/iMathAS unused-job cleanup, permissions and
  identity boundary, documentation, and stale-test removal. M2.5 is complete. M4 behavior and
  authorized temporary-corpus closeout are complete by the later fresh-runtime receipt.

- The Genetics preparation receipt now records the verified r11 conversion path: all six observed
  BBQ response shapes compile to opaque static PG, and private renderer proof passed all 20,579
  generated documents. Ordinary r11 publication then succeeded on the disposable runtime as
  Blueprint `BP-2`, Revision 1, with 11 topics, 119 banks, and 20,579 rows; its retained receipt
  was regenerated. The current fresh-runtime r5 proof established visible saved-control restoration
  after reload and a new Mary session, direct same-Attempt navigation, UI wrong/correct submission,
  and Student-history/Gradebook agreement. Fresh outage and GET receipts separately establish
  recovery and read-only document behavior. Session 78713's exact aggregate passed before fresh
  M4 startup with corrected PG baseline, installation provision/opt-out, Course Appearance
  PostgreSQL/MinIO, complete live acceptance, and empty owned inventory. Authorized closeout
  removed the temporary corpus/probe tree without creating a fixture or recurring inventory gate.

- Corrected the backend-owned resume boundary that the r4 evidence exposed. The existing
  authorized WeBWorK document read now carries the private saved opaque input and exact
  Question Attempt source pins to create an ephemeral resumed document; the original issued HTML
  remains immutable. It adds no API/table, expiry policy, GET write, outcome persistence, control
  parser, queue, or submission/grade action. The renderer transport still rejects answer/preview,
  process, and source-URL overrides. Independent review and focused gates pass, including
  temporary backend-only rendering of real r11 MC, MA, FIB, NUM, and duplicate-MA cases with
  restored visible controls and no feedback. The later fresh r5 PLE proof completed reload through
  a new Mary session and UI Submit. Session 78713's exact aggregate remains a separate receipt
  that predates fresh M4 startup.

- Corrected the Genetics walkthrough's retired automatic-grading Job wording. Its Attempt authority
  section now preserves saved responses across backend failure, permits Student submission within
  the existing time limit, and keeps expired Attempts closed to edits until ordinary finalization.

- M2.5 implementation and connected/live acceptance passed. The full registered Live Demo Attempt
  journey passed in session 26042 on the owned rebuilt stack: start authorization, exact released
  revision boundary, native immediate submission with immutable credit/current-point scoring,
  no-browser expiry, opaque WeBWorK capture/save/reload, outage preservation and recovery,
  immediate score, and Gradebook agreement. The database baseline passed earlier; latest Rust and
  pytest checks also passed. The exact aggregate `source source_me.sh && ./launchers/all_test.sh`
  passed in session 98225, including Rust, frontend, 6,039 Python tests, database baseline,
  installation-data provision and opt-out, Course Appearance/MinIO, and banner/thumbnail saga
  lanes. Genetics/M4 corpus and publication acceptance remain pending, so Phase 2 is not complete.

- Question Library search now decodes the browser's repeated query filter keys, including a
  one-value `backends=ple` filter, through the maintained multi-value Axum query extractor.
  Typed enum validation, unknown-field refusal, scalar duplicate-key refusal, normalization,
  and page bounds remain unchanged.

- The connected database baseline E2E passed in session 51122, including the direct-finalization
  late-save and commit/expiry races, immutable-credit/current-point/expiry-Gradebook SQL proofs,
  and the Unrelease fixture path. The canonical owned-runtime inventory was empty after cleanup.
  An independent final `./check_rust.sh` passed full all-feature Clippy, tests, and Wasm; the full
  `./check_codebase.sh` passed both TypeScript checks, lint, format, and 374 Node tests. Full
  aggregate, live-browser, and M4 acceptance remain pending.

- Corrected current Phase 2 fixtures and improved redacted diagnostics while preserving their
  bounded evidence boundaries. The refreshed offline gate `source source_me.sh &&
./launchers/run_fast_checks.sh` passed in session 37638: 6,039 pytest tests in 10.84 seconds,
  372 Node tests, and the Rust, TypeScript, lint, and formatting checks. `git diff --check` is
  clean. Connected and live acceptance remain pending; M4 acceptance is not yet approved.

- Raised only the Blueprint client response budget to 16 Mi characters through an optional internal
  constant after the real R8 response-shape estimate measured 15,526,628 ASCII characters. All
  other API clients retain the shared 4 Mi character default. This is a capacity correction, not
  paging or projection work, and adds no Genetics fixture. Seven generic protocol tests plus
  TypeScript, lint, format, and diff checks pass; real-browser 15 Mi character evidence remains
  pending. The two-connection harness uses a test-local pool of two with `CONNECTION LIMIT 2` for
  its holder observation, without a production-role change.

## 2026-09-13

### Decisions and Failures

- Corrected the M2.5 background-execution boundary in the active plans. The existing worker stays
  for abandoned Attempt expiry and hidden deferred-backend completion; public grading states,
  attention, polling, retry-grading, and score queues remain out of scope. Canvas and Blackboard
  sources now document the bounded background precedent. This is a plan correction only.

- Clarified the M2.5 plan boundary: the Retry reset removed manual retry behavior, while M2.5
  completes the grading-lifecycle correction. Before-expiry backend failure preserves saved work
  for another submit within the existing time limit; expired Attempts remain closed until their
  saved work finalizes. Canvas is architectural precedent for lazy reconciliation, and Blackboard
  is behavioral corroboration only. This is a documentation correction; no runtime code changed.

- Replaced the superseded Phase 2 grading-lifecycle plan with the approved M2.5 boundary: a
  Question Backend produces one immutable normalized credit fraction for a submitted response;
  PLE calculates current Assignment scores from that credit, the issued scoring rule, and current
  Entry point values without another backend interaction. The plan removes public grading states,
  polling, attention counts, and generic regrading/retry-grading from scope. It preserves the
  existing worker for abandoned expiry finalization and hidden deferred-backend completion. The
  completed M1 Attempt behavior and retained eleven-topic Genetics Blueprint scope remain intact.
  This is a planning correction; no runtime code or tests changed.

- Corrected Phase 2 M4 from a temporary representative-content walkthrough to its requested
  deliverable: one reusable Genetics Blueprint Course with one Assignment for each ordered Biology
  Problems Website Genetics topic. The plan now treats current `bbq-*-questions.txt` banks as
  membership authority, preserves curriculum/provenance as product data, maps bank variation
  through existing Question Pools, and requires retained-Blueprint reload plus a disposable Course
  Instance behavior check. It records the verified rich-table presentation gap as an evidence-led
  source/render decision; no content has been provisioned by this documentation correction.

- A temporary, ignored feasibility probe converted one real Genetics monohybrid-degrees-of-
  dominance source row to supported legacy PG `RadioButtons` plus `BEGIN_TEXT`/`MODES` HTML.
  Renderer review preserved its colored Punnett table and choices without warnings; correct and
  incorrect grading returned `1` and `0`, and repeated rendering at one seed was deterministic.
  This establishes a single-row renderer/grader path only, not full-bank population, PLE
  publication, Blueprint provisioning, or a permanent test. No production runtime or upstream
  source changed. The probe and its raw answer-bearing evidence are removed; no containers were
  touched. Documentation verification passed: 245 Markdown-link/guidance checks, scoped Prettier,
  and `git diff --check`.

### Behavior or Interface Changes

- Added `cargo tools curriculum-content validate|publish <manifest>` as the retained curriculum
  import boundary. It validates the whole ordered source hierarchy, contained relative paths,
  checksums, unique identities, source attribution, supported WeBWorK metadata, and Pool bounds
  before writing. The caller supplies an explicit canonical curriculum-content root rather than
  relying on a build-path fallback. Publication reuses exact compatible provenance or follows the ordinary Draft,
  source-object binding, Question publication, and Blueprint stores; it emits only an opaque
  Blueprint receipt and source revision summary. It does not add a PLE runtime parser, account,
  deployment, enrollment, deadline, or test-fixture path.

- Corrected the remaining grading-worker test vocabulary after the Retry reset. Student recovery
  now refers only to saved-response auto-submission at Assignment Attempt expiry; the former
  `learner_native_ple_recovery` scenario and submission-recovery shell fixture are named for their
  actual worker interruption. A non-default `e2e-grader-fault` build provides one bounded native
  PLE lease hold plus closed WeBWorK transient, final, and expired-lease sequences. The stack
  controller can recreate only the corresponding worker with one fixed mode; no browser or product
  route selects a fault. The ordinary Live Demo and production binaries contain no fault path. The
  Gradebook fixture now checks its complete progress-count field
  set and the Course-authorized, answer-free, read-only Instructor detail route. Rust feature tests,
  shell syntax, focused browser-contract tests, and 1,253 focused Python policy tests pass. The
  Store boundary is named `InstructorGradingStatusStore` so it cannot be mistaken for an Instructor
  grading capability. After removing the retired browser contracts and tests, the exact offline
  aggregate generated 337 TypeScript declarations and passed 374 Node tests and 6,126 Python
  tests. Fresh PostgreSQL/database-baseline evidence, Chromium expiry auto-submission, native
  worker replacement, and the fixed WeBWorK transient, final, and expired-lease scenarios pass in
  the disposable stack. The exact full aggregate ends with complete live acceptance green.

- Student Assignment Attempt and summary pages now read an answer-free automatic-grading status,
  poll only while accepted work remains queued or grading, and stop at terminal results. Course
  Instructors receive the same Course-scoped status as read-only Gradebook metadata; neither role
  receives a grading, regrading, or retry action.

- Grading and public-asset Jobs now enforce a closed database transition graph. A transient
  infrastructure failure may move a leased, unfinished Job back to ready with bounded backoff;
  failed and completed Jobs remain terminal, and graded or exempt results cannot reopen. Each
  transaction that reaches ready emits one worker-kind-only PostgreSQL notification after commit.

- An expired final grading lease moves both its Job and accepted-response grading row to Instructor
  attention atomically. Transient pre-result infrastructure failures requeue the same unfinished Job with
  caller-supplied backoff until the claim budget is exhausted; final failures retain only a bounded
  private reason class and converge idempotently. Commit and failure operations reject a stale
  lease token with no write, and the three grading workers receive only their fixed-backend failure
  procedures.

- PostgreSQL now projects accepted-response grading as the closed public states `queued`,
  `grading`, `graded`, and `needsInstructorAttention`. A Student can read only positions and
  states for their own Assignment Attempt; a Course Instructor receives the same read-only state
  within Course authority. Gradebook evidence distinguishes attention and in-flight Question
  counts without exposing responses, worker capabilities, or private failure reasons. No public
  grading or regrading mutation exists.

- Native and WeBWorK grading Stores now share provider-neutral failure and outcome contracts,
  require the exact current lease token, and map a rejected lease to `LeaseLost`. Separate
  Student and Instructor Store boundaries expose only their role-appropriate grading projection;
  fake-contract and connected PostgreSQL adapter tests prove transient pre-result requeue, final attention,
  stale failure refusal, and exact answer-free Gradebook counts.

- Native PLE and WeBWorK grading now run through one typed worker loop. Missing sources and
  renderer outages requeue with bounded backoff; malformed stored responses, native evaluation
  failures, ungraded renderer results, and invalid renderer output converge on Instructor
  attention. A stale lease is logged and skipped without ending the process. Idle workers use a
  dedicated, attested PostgreSQL listener connection for the worker-kind-only `ple_job_ready`
  notification with a five-second fallback claim, while shutdown interrupts idle waiting and
  bounds in-flight Store draining.

- Timed Assignment Attempts now retain one immutable server expiry from their start-time limit
  and close instant. Reload or another authenticated session resumes the same saved work; at
  expiry, saved responses become immutable submissions and grading jobs while unanswered
  Questions close at zero. The Attempt page shows the exact instant in the Student's chosen time
  zone, refreshes authoritative state when its monotonic countdown reaches zero, preserves edits
  through connection failures, and resumes the same active Attempt after reconnect. A bounded generic-worker sweep finalizes abandoned
  Attempts, while idempotent finalization and late-save refusal prevent duplicate or replacement
  evidence.

- Assignment Start Decisions now use one PostgreSQL rule and precedence order at exact
  boundaries: close, availability, completed-Attempt limit, then late-work policy. The live
  access read evaluates once and returns the effective available, due, close, Attempt-limit,
  late-work, server-evaluation, and Student-time-zone values; start and save mutations use the
  authoritative database clock. An already-active resumable Attempt remains usable after its
  due instant, while close and time-limit expiry reject further saves.

- Assignment Access and every Student Course landing card now embed one Rust-owned, generated
  `StudentAssignmentDecisionSummary`. It carries UTC-millisecond schedule and evaluation
  instants, effective limits and late-work rule, the Student's IANA display zone, one closed
  Start Decision, and matching public reason. Future-scheduled released Assignments remain
  visible; accommodation identity and other Students' effective values remain private.

- Student Assignment landing and pre-start pages now share one direct decision presentation.
  It answers whether the Student can start, distinguishes availability, due, and close instants
  in the supplied Student time zone, and explains the Attempt limit, time limit, and late-work
  rule before Start. The browser displays the server's decision and public reason without using
  its own clock to infer permission.

- Students can now read and save their own IANA display time zone from the Course landing.
  A confirmed change immediately re-renders the same stored Assignment instants without moving
  their deadlines. A newly roster-created Student Account receives the inviting Instructor's
  zone once when its invitation is accepted; existing Accounts and an earlier Student choice keep
  their zone. The Student route accepts no Account identity, and Instructor sessions cannot use it.

- PLE Question JSON is unversioned and uses one current source shape without a `version` member.
  `format: "pleQuestionJson"` identifies the document across Rust compilation, browser authoring,
  QTI mapping, fixtures, and connected authoring scenarios.

### Development Workflow

- Added `launchers/run_fast_checks.sh` as the exact offline subset of the aggregate gate. It runs
  Rust, TypeScript/Node, and Python validation without starting the disposable live stack; the
  authoritative final gate remains `source source_me.sh && ./launchers/all_test.sh`.

### Fixes and Maintenance

- M2.5 checkpoint: PLE now persists immutable normalized backend credit and calculates Assignment
  scores on read from current Entry point values. Native PLE and WeBWorK saved-response finalization
  use the ordinary direct submission path; the existing expiry path is being reused, and public
  grading-status/detail surfaces are removed. The exact offline gate `source source_me.sh &&
./launchers/run_fast_checks.sh` passed in session 66981: Rust checks, tests, doctests, Wasm,
  all-strict Clippy; both frontend TypeScript checks, ESLint, Prettier, and 372 Node tests; and
  6,039 pytest tests. Removing the obsolete grader-fault feature, source, and tests reduces counts;
  it is not a coverage claim. Connected database acceptance is still running (session 23522), and
  live journey and Genetics renderer/publication evidence remain pending.

- Aligned the live Phase 2 contracts with the approved grading and expiry model: Question Backends
  produce immutable normalized credit, scores are calculated on read from current point values,
  reads do not finalize Student Work, and internal background execution is limited to expiry
  submission and backend-specific completion polling. Removed stale public grading-state, polling,
  attention, retry, and score-recalculation wording. Repaired only relocated archive-link targets;
  this documentation update does not claim the in-progress finalization implementation is verified.

- Replaced the stale payload, determinism, concurrency, enrollment, Instructor, and accessibility
  documentation that still described per-Question submission receipts, browser prefetch,
  successor promotion, or Instructor grading recovery. The current contract saves responses by
  position, finalizes one whole Assignment Attempt, defines recovery solely as expiry
  auto-submission, and exposes automatic grading as read-only status.

- Strengthened grading evidence with a connected successful-commit case that proves a terminal
  immutable result cannot be replaced or requeued and a two-connection claim case that proves only
  one worker leases a Job. The focused Chromium expiry journey now proves a failed autosave keeps
  the edit, the next save succeeds, expiry auto-submits, and polling stops after terminal grading.

- Audited the Blueprint Revision-only cutover, removed unused command and event
  contracts plus stale Draft/publish wording, corrected current-Revision
  availability documentation, and made connected browser journeys select the
  exact Blueprint they create. The existing PostgreSQL lifecycle oracle now
  also proves that a non-owner Instructor can discover and read an Available
  Blueprint through ordinary application authorization. Newly issued Question
  seeds now remain exactly representable by the browser's numeric JSON contract,
  and Students can read nonce-bound grading status after submitting the whole
  Assignment Attempt. Static PLE Question JSON now explicitly excludes Question
  Seeds and runnable code; removing its generic issuance seed is bounded
  follow-up work.

### Removals and Deprecations

- Removed the unmounted browser recovery state machine, completion/recovery helpers, prefetch
  binding, and their isolated tests. Removed the dead per-Question submission, status, and prefetch
  browser methods and the nonce-scoped native PLE submission Store, SQL functions, route, generated
  contracts, and tests. The mounted Assignment Attempt page now owns save, whole-Attempt
  finalization, expiry auto-submission, and read-only grading status directly.

- Removed the derived private-grading schema counter, the transient H5P importer-iteration counter,
  the two repository-owned fixture counters, and the browser-scenario module's two unread constants.
  Private grading is recomputed from source, raw H5P packages remain available for re-import, and
  repository fixtures change together with their consumers.

### Decisions and Failures

- A human review found that the initial M2 plan and implementation had converted Student
  connectivity recovery into an unapproved Instructor grading Retry. The reset removes that
  mutation from current plans, schema, browser and Store contracts, tests, and durable docs.
  Autosave preserves working responses, and reconnect or reload resumes the same active Assignment
  Attempt. Recovery is only the server-owned auto-submission of saved responses at expiry.
  Automatic worker requeue remains limited to an unfinished Job for which no Grading Result exists
  and is not recovery or regrading.

- Native PLE Question JSON remains unversioned across every shape change. All stored native JSON
  Questions and readers are upgraded together.

- Retained the runtime manifest `schema_version` as an independent cross-language operator boundary.
  Browser Suite coordination/configuration records, screenshot capture/publication evidence, and
  walked-journey baselines also retain their own counters because independently consumed process or
  evidence artifacts may outlive the writer; QTI profile/mapping and package/release versions name
  separate external or durable identities unrelated to native PLE Question JSON's unversioned shape.

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
