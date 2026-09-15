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

### Developer Tests and Notes

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

- Removed the author-supplied `externalDependencies[].{id,cdnUrl,localPath}`
  native PLE Question JSON shape and its validators. Native author JavaScript
  may request only the closed `libraries` enum (`rdkit`); a future server-owned
  reviewed registry, never an author URL or path, selects runtime assets.

- Removed C870's dormant H5P adapter, source/revision bindings, workspace-import value, Question
  Backend/Format values, and speculative secondary-attempt binding. H5P has no delivered PLE seam
  until its explicit content/xAPI product decision is answered; PLE, WeBWorK, and iMathAS behavior
  remains intact.

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

- Reopened both pre-production direct-design Human Guidance occurrences. The database migration
  guard is source evidence only; the remaining live alternate-path audit must establish that no
  obsolete legacy behavior survives. No permanent audit test or framework was added.

- Corrected the Assessment direct-cutover decision to match Human Guidance: remove legacy
  Assignment routes, APIs, decoders, compatibility tests, and their callers in the same
  preproduction change. No one-release redirect or 410 compatibility surface is retained.

- Recorded accepted temporary evidence without overstating product closure: C910's fresh-PG17
  general-feedback revision proof and C839's 42 canonical PGML-source acceptance both passed,
  while Student HTTP feedback delivery and Genetics catalog reconciliation remain open behind the
  current AWS Smithy server-build incompatibility and their normal closure work. The user removed
  the redundant static source bulk; the canonical source layout is now `content/genetics/pg/topicNN/`,
  which does not itself prove publication or catalog reconciliation.

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
