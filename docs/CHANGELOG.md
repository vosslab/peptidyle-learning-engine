# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was
> changed and believed at the time. They are not product authority. Current
> intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old
> Assignment, Blueprint, lifecycle, grading, role, retention, and UI models.

## 2026-09-16

> September 15 entries are archived in [CHANGELOG-2026-09k.md](CHANGELOG-2026-09k.md).

### Behavior or Interface Changes

- Synchronized the current Human Guidance Assessment-content and Question-Pool membership wording
  with the implementation checklist. The exact Published Question ID-and-Revision reference
  invariant remains open pending authoring-input cutover; Pool fork and no-nesting additions remain
  source-audit pending. The recomputed inventory is 817 bullets: 428 verified, 346 open (340
  owning), and 43 N/A. The Assessment audit gate still reports eight pre-existing invalid evidence
  locators in unrelated Attempt/scoring rows.

- Added bounded Blueprint Assessment-owned Pool/fork contributor evidence without changing Human
  Guidance, its checklist/counts, or any closure. Independent review accepted the actual-server
  43-HTTP-request pass at `/private/tmp/ple-blueprint-owned-pool-artifacts.ySNfXX` and focused
  build log `/private/tmp/ple-blueprint-owned-pool-build-4.log`. Repeated imports minted fresh
  Pool IDs; retained edits created immutable Pool Revisions with exact ordered Question IDs and
  revision pins; retry, stale, and foreign/error paths were covered; and a whole Blueprint fork
  minted fresh Assessment and Pool IDs with the same exact ordered Question revision membership.
  The internal `question_pool_revision.created_in_transaction xid8 DEFAULT pg_current_xact_id()`
  marker replaces a timestamp ownership heuristic and is never public. Independent review also
  accepted the compiled-main browser receipt at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.ytU6GT`: its browser/state JSON, desktop and
  narrow screenshots, and matching compiled hash show lazy exact-member reads, local Cancel,
  reorder, remove/add, and closed-panel missing-attestation Save blocking. One ordinary Save made
  exactly one `PUT` and one Revision while source and sibling Pools remained unchanged. Root pytest
  passed 7,280 tests. This evidence claims no populated Student Work (only an empty
  `student_record`), new Selective Apply contract, current-pair full workflow, login, TLS, full
  accessibility, or full rollback; all relevant Human Guidance closures remain pending. Removed
  three obsolete numeric identifier references from an archived plan.

- Synced the new Blueprint-fork Assessment/Question Pool wording and no-Assessment-history
  comparison row as open. Independent Pool forks are not implemented, and historical
  pin-preserving proof does not close them. The inventory has 811 bullets: 427 verified,
  341 open (335 owning), and 43 N/A.

- Reconciled Blueprint audit/planning records to the latest Human Guidance comparison and history
  authority without changing runtime code or HG. Retained valid bounded read-only evidence;
  reopened visible same-lineage pair coverage, shared-Question-ID Assessment matching, robust
  shared/added/removed correspondence, history/newer indications and connected Apply gaps.
  Recorded origin remains provenance, not a required comparison baseline. The user's fresh local
  Assessment-ID/no-cross-Blueprint-Assessment-lineage clarification requires fork/Apply audit;
  prior internal-ID contributor proof does not close it. Checklist gates pass with 808 bullets:
  427 verified, 338 open (332 owning), 43 N/A.

- Closed only the six bounded Blueprint fork-review HG rows: known-fork owner discovery,
  source-row opening, current-head comparison, recorded-origin baseline, visible source-only and
  fork-only distinction, and requested canonical-JSON calculation. Accepted C881/C882 receipts are
  `/private/tmp/ple-fork-reader-artifacts.nRikDO` and
  `/private/tmp/ple-fork-review-http-artifacts.LTUgsF`; root visual review accepted the compiled
  UI at 1280 by 800 and initially at 390px. The lazy, retryable review is GET-only, `no-store`,
  and leaves all `ple_data` unchanged. The direct fork page has no Compare entry; review begins
  from the source known-fork row. Root's latest pytest run passed 7,232 tests; accepted source
  build, TypeScript, and Node-21 evidence is limited to this scope. C883 selective apply and C413
  whole-workflow closure remain open.

- Added accepted source-only C880 Blueprint fork-comparison contributor evidence without closing
  C881-C884 or C413, changing Human Guidance, or adding runtime workflow behavior. The Question
  Model projection compares canonical Question JSON exports rather than PostgreSQL layout; it keeps
  stable identities and order across unchanged, source-only, fork-only, and overlapping changes,
  and duplicate IDs are constructor errors so map construction cannot silently lose an item. The
  temporary consumer also covered module-parent/current-name changes, pins, and defaults. Root
  `cargo build -p question_model`, `cargo test -p question_model`, and `cargo clippy -p
  question_model --lib -- -D warnings` passed; logs are in
  `/private/tmp/ple-blueprint-fork-comparison/`. No persistence, authorization, server read/apply,
  UI, public comparison-state enum, or permanent test was added.
  The documentation also records the user's clarification that ordinary Blueprint visibility governs
  viewing/comparison and fork ownership governs apply mutation; it changes no lifecycle state or
  product decision.

- Added accepted C881 contributor evidence for the corrected-current-head Blueprint fork read
  boundary. Typed ordinary-visibility reads now supply the recorded origin, source current Revision,
  fork current Revision, and current names; source head may equal origin. The corrected-current-head
  build and connected proof passed at `/private/tmp/ple-fork-reader-artifacts.nRikDO` and
  `/private/tmp/ple-fork-reader-connected.log`; independent evidence review accepted. This records
  the expanded Guidance model's read boundary only: C882-C884 and C413, including server comparison,
  selection/apply, and UI, remain open. No Human Guidance or checklist closure, persistence, public
  comparison-state vocabulary, or permanent test is claimed.

- Added accepted C882 contributor evidence for the on-request Blueprint fork review endpoint.
  Actual HTTP proof at `/private/tmp/ple-fork-review-http-artifacts.LTUgsF` covered exact current
  heads, names/ETags, ordinary visibility, `no-store`, and zero `ple_data` mutation. Root build,
  TypeScript generation/typechecking, and 21 Node tests passed; independent source and evidence
  review accepted. This receipt covers Fixed Questions only; Pool semantics remain with the earlier
  C880 source. C883/C884/C413 full workflow, including selection/apply and UI closure, remain open;
  no Human Guidance or checklist closure and no permanent test are claimed.

- Published the canonical 58-capture screenshot corpus with normal Morgan MFA through the reviewed
  separate CLI, without a bypass. The refresh and independent live replay verification passed at
  `/private/tmp/ple-screenshot-refresh-20260916.log` and
  `/private/tmp/ple-screenshot-refresh-verify-20260916.log`. It includes four new-content captures,
  refreshed Sysadmin captures, and the reviewed atlas-generator alt-text fix. Twenty-nine
  byte-different replay images remain in `test-results/screenshot-corpus/verify`; semantic checks
  passed and no byte-equivalence gate is claimed. Root visually inspected the refreshed Sysadmin
  home and earlier review covered the four new-content captures. Static Markdown links (285) and
  five screenshot tests passed. No Human Guidance closure is implied.

- Closed only the two Archived Blueprint discovery HG rows. The normal Blueprint list defaults to
  excluding Archived records; an explicit strict `includeArchived=true` shows them to every active
  vetted Instructor while Private records remain owner-only. Accepted actual-server HTTP evidence
  covers owner/nonowner default, false, and true membership; nonowner Archived `200`, Private
  `404`, Student `404`, and invalid query `400` results. Accepted compiled-main browser evidence
  covers default off, explicit include, actual read-only Archived detail, return to off, eight GETs,
  and zero writes. Artifacts:
  `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` and
  `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json`.
  The fixture has privileged Published-Question seed data, ordinary vetted account APIs, fixture
  sessions, and an accepted Sysadmin MFA fixture helper; it does not claim login, TLS, pagination,
  concurrency, publisher-caller preservation, forking, adoption, or a broader Course workflow.
  Root `cargo build -p server_core --features local-disposable-storage -p project-tools`, the full
  canonical database baseline (`/private/tmp/ple-archived-discovery-baseline.log`), pytest (7,151),
  21 Blueprint client/UI/model Node tests, and TypeScript typechecking passed. Checklist splice,
  diff, consistency, and the Course gate passed: 423 verified, 318 open, 312 owning-open, and 43
  N/A across 784 bullets. No aggregate `all_test.sh` claim is made.

- Closed only the Archived Blueprint read-only HG row. Owner Save and rename now reject Archived
  Blueprints before replay, CAS, or no-op handling; the extended existing lifecycle regression
  preserves metadata, Revision, content, events, and receipts across denied writes, while restored
  Private/Public writes succeed. Accepted actual HTTP proof recorded five `409` denials with
  unchanged Blueprint state, `200` owner/nonowner historical reads, `404` nonowner writes, and
  `200` restored writes:
  `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json`.
  Root `cargo test --workspace --no-run`, the full canonical database baseline, and pytest (7,151)
  passed. Explicit-include Archived browsing, forking, adoption, populated daughters, concurrency,
  and broader Course completion remain open; no aggregate `all_test.sh` claim is made. Checklist
  splice, diff/consistency, and the Course gate passed: 421 verified, 320 open, 314 owning-open,
  and 43 N/A across 784 bullets.

- Added focused selected-source CLI evidence without changing C838/C839 status or any Human
  Guidance count. The ordinary-Instructor publication command selected the fresh 42-source
  canonical manifest entry `topic05-degrees-of-dominance-which-one`, publishing one available
  WeBWorK PGML Question Revision 1 with exact bytes, checksum, and source provenance; it created
  no Pool or Blueprint. Replay made no additional publication, and unknown source, hash mismatch,
  and missing-path inputs were rejected before writes. The private fixture's vetted Instructor and
  audit seed are explicitly privileged; the publication session is ordinary Instructor. The
  isolated proof is `/private/tmp/ple-canonical-family-artifacts.TOOlBJ`, run by
  `bash /private/tmp/ple-canonical-family-publication-proof.sh --isolated`. Separately,
  `cargo build -p project-tools` and `cargo test -p project-tools --bin project-tools
  curriculum_content::` passed, including four existing curriculum tests. It does not claim
  Sysadmin authentication, rendering,
  browser acceptance, or a retained catalog. Questions checklist splice, diff/consistency, and
  gate passed unchanged at 420 verified, 321 open, 315 owning-open, and 43 N/A across 784 bullets.

- Closed only six Course Blueprint-update HG rows: the three owning Course-summary workflow rows
  and their three duplicate occurrences. An authorized Instructor can lazily review the current
  parent Revision for adopted Assessments only, seeing changed, matching, removed-source,
  Type-mismatch, and automatically-added rows; direct local Assessments are excluded. Accepted
  actual-server and compiled-main proof at 1280 by 900 and 390 by 844 covered lazy open/reopen,
  per-read five-row coherence, Course-to-detail review, zero POST on Cancel, exact source Revision
  2 plus daughter Edit CAS on Apply, and refresh to a matching row. Student and unrelated-
  Instructor reads were nonenumerating `404 no-store`; a private parent was concealed from another
  Instructor in the privileged-availability fixture; Archived review remained available and new
  adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  The earlier detail-only receipt remains the separate proof of exact pins, stale/no-op/invalid-
  Released cases, dates, status, origin, and one populated Assessment Attempt hash:
  `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`. Neither receipt claims direct
  Assessments or all Student Work. No persisted offer, receipt, comparison baseline, or new update
  table is introduced, and no whole-Course lifecycle completion is claimed. Root build, TypeScript,
  LDA clippy, pytest (7,151),
  Node (341), and fresh full database baseline passed; the baseline receipt is
  `/private/tmp/ple-course-summary-baseline.log`. No aggregate `all_test.sh` claim is made.

- Added one accepted retained-Assessment Blueprint-update contributor without closing C410/C411 or
  changing any Human Guidance count. An authorized Instructor can `GET` a derived current-parent
  review and explicitly `POST` Apply for one adopted Assessment using both expected parent Revision
  and daughter Assessment Edit Number. It has no offers, approval receipts, comparison baselines,
  or update tables. Accepted actual-server and compiled-UI proof reviewed complete current/proposed
  Fixed Question and one-member Pool Revision-1 facts, made Cancel issue zero POST requests, then
  applied while preserving dates, status, origin, and one populated Assessment Attempt hash. It also
  covered stale parent/daughter CAS, authorization, no-op, and invalid Released rollback; visual
  review was desktop-only at 1280 by 900. Artifact:
  `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`. Whole-Course discovery/review,
  all correspondences, narrow viewport, dirty/stale UI, missing/type/Archived parent, lock-wait,
  and multi-Revision cases remain open. This proof does not establish normal Student start, future
  Attempts, responses, submissions, grades, or all Student Work tables. Static server/build,
  TypeScript, scoped PostgreSQL LDA clippy, pytest (7,151), and Node (341) lanes passed; the
  independent baseline-fixture reviewer accepted the focused PostgreSQL 17 sequence at
  `/private/tmp/ple-expiry-unrelease-focused-artifacts.iOpO6r`. The full canonical database-baseline
  gate also passed as `database baseline E2E: PASS`; no aggregate `all_test.sh` claim is made.

- Captured four one-time Instructor preview screenshots of new Genetics WeBWorK content from the
  production-shaped HTTPS Live Demo: DNA structure, meiosis prophase, chi-square, and chromosome
  shapes. The ordinary-Instructor runner passed route, Ribbon, privacy, page-error, and origin
  checks for all four 1280x800 images; visual review found the prompts and controls present.
  These 320 KiB files remain untracked review evidence in
  `test-results/screenshot-corpus/new-content/instructor/`, separate from permanent tests and the
  unchanged 54-capture canonical atlas. Canonical publication was not attempted because it requires
  an ordinary Sysadmin MFA session; static canonical verification still passed. No Human Guidance
  closure or product change is claimed.

- Closed only the two HG rows that require an older Blueprint Revision to be obvious on a daughter
  Course Instance. The existing authorized Course load and Course page now show adopted/current
  Revision values and the newer-state notice. Accepted independent actual-server/exact-main proof
  covered empty, current, newer, and synthetic Private-origin states; the newer PNG was visually
  inspected, denials were nonenumerating `404 no-store`, and no extra Blueprint fetch/write or
  browser errors occurred. Original adoption pin, Assessment, and entries remained unchanged;
  Work tables were empty, so no populated-Student-Work claim is made. Artifact:
  `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`. This is not Blueprint update
  offer/review/approval/apply work. The independently accepted focused PostgreSQL 17 access-seed,
  security, expiry, grading, and Assessment Unrelease sequence, including its causal row lock,
  passed at `/private/tmp/ple-expiry-unrelease-focused-artifacts.5Kb3MC`; it is a fixture cutover
  receipt, not a further HG-row closure. Checklist splice, diff, consistency, and the Course gate passed:
  414 verified, 327 open, 318 owning-open, and 43 N/A across 784 bullets. The final full pytest
  rerun passed 7,151 tests. The broad database-baseline gate is not green: after its earlier
  stages passed, it stopped in final Blueprint lifecycle support at `support.rs:82` on a
  permission-denied `ple_data` relational-identity query. No repair is claimed.

- Corrected stale Quiz/Exam disclosure evidence without closing either HG row. The current
  `history_decision` calls `gate_quiz_exam_answers_for_current_cohort`, and
  `project_released_content` applies that gate. Accepted independent PostgreSQL 17 installed-
  predicate proof with administrator-inserted synthetic fixtures covers never-started blocking,
  pending-invitation exclusion, joined-current-membership blocking, Account-deactivation membership
  preservation, Course-end noncompletion, ended-episode exit/new-episode rejoin, and retained
  submissions:
  `/private/tmp/ple-assessment-cohort-transition-artifacts.nWdHzT`. This is not public membership
  API, whole-submit, or HTTP answer-withholding acceptance. Opaque WeBWorK answer display remains
  unimplemented and HTTP verification is pending, so both product rows remain open.

  Baseline fixtures now use an explicit role-scoped provenance read on one `PgConnection`,
  isolated second-test IDs, and the forwarded manifest; no production permission is weakened and no
  permanent test is added. The full 7,151-test pytest run passed. The broad
  database-baseline gate failed because an existing negative fixture used the pre-public-reference
  return column. Its focused Course connected correction passed both existing tests sequentially
  against fresh PostgreSQL 17 with 0 ignored:
  `/private/tmp/ple-course-lifecycle-proof-artifacts.l8ilq0/connected-test.log`. A later broad
  rerun passed the Course tests and security catalog, then failed because
  `tests/e2e/attempt_expiry_connected_oracle.sql` still calls removed
  `ple_api.start_assignment_attempt`; direct caller cutover and rerun remain pending.

- Closed only six HG rows: C15's higher-security Sysadmin row, three automatic-new Blueprint
  Assessment rows, and two existing-Assessment non-silent-change negative invariants. Accepted
  independent SQL/actual-server loopback HTTP proof requires genuine private TOTP
  before Sysadmin session issuance, preserves ordinary Student/Instructor sessions and limited
  grants, and denies binding failures, bad codes, replay/expiry/counter reuse, and a fresh unused
  valid counter after five failed attempts. Artifacts:
  `/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H` and
  `/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`. Connected normal Blueprint
  Save proof preserves exact settings/pins, fresh daughter Pool IDs, Unreleased/null-date copies,
  an inactive daughter, unrelated empty-Course nonmutation, existing actual Student Work,
  original adoption pin, retained source-title change without existing daughter mutation, and replay/no-op/
  stale safety; supplemental bad-payload rollback passed:
  `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. The existing stable product-contract lifecycle
  test was extended with the append helper; temporary SQL/HTTP proof remains outside Git. Existing connected adoption
  lifecycle regression passed 1 test with 0 ignored:
  `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Checklist splice, diff/consistency,
  and three narrow gates passed: 412 verified, 329 open, 320 owning-open, 43 N/A across 784 bullets.
  No Human Guidance wording, deployed TLS/full Live Demo, C410 existing-Assessment update offers,
  whole Course/Assessment milestones, or offline aggregate acceptance is claimed.

- Closed only four Assessment-content HG rows. Accepted actual-server/private bundled-main
  browser proof saved/reloaded mixed Fixed Question/Pool order and exact pins, reordered and
  removed both kinds, retained private retired IDs/old Pool Revision, and reimported distinct fork
  identities with source Pool JSON unchanged. Student direct real HTTP returned 404 without
  current-state change; browser error arrays were empty. Artifact:
  `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. SQL now supports atomic current-
  position swaps/retired-position reuse and excludes retired entries from workspace reads.
  Supplemental current-schema proof (`/private/tmp/ple-assessment-mixed-entries-artifacts.vdrKfr`)
  compared old/new join predicates with four retained/two active entries and rejected duplicate
  current positions at commit (`23505`) without semantic-state change; no old-index or performance claim.
  Existing connected adoption regression passed 1 test with 0 ignored under the new SQL; supplemental Type
  projections retained unchanged pins: `/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`.
  The exact Randomize question order label uses the earlier accepted part 04 actual-main/HTTP
  checkbox receipt, not this mixed-entry proof. Assessment splice, diff/consistency, gate, and
  `git diff --check` passed: 406 verified, 335 open, 324 owning-open, 43 N/A across 784 bullets.
  No permanent tests were added. Student Work history, authentication/TLS, full Live Demo, WeBWorK
  delivery, C505 filename rename, and whole-milestone completion are not claimed.

- Closed ten bounded Assessment Type purpose/capability HG rows in documentation only. The real
  Type registry renders Instructor-selected purposes in Course and Blueprint creation; current
  points/extra-credit scoring and Properties controls establish the named capabilities. Existing
  accepted Bonus `8 / 0` and Quiz/Exam one-Attempt receipts remain separate runtime evidence.
  The broad appropriate-defaults and cross-backend Practice immediate-answer rows stay open;
  this does not claim all-settings persistence/enforcement, formal collaboration policy,
  learning-age inference, exam calendars, or complete Student delivery. Assessment splice,
  diff/consistency, gate, and `git diff --check` passed: 402 verified, 339 open, 328 owning-open,
  43 N/A across 784 bullets. No product code or Human Guidance changed.

- Closed only the shared underlying Assessment-model HG row. Canonical teaching types and
  ordinary adoption connect reusable Blueprint content to current Course Instance Assessments;
  distinct storage/lifecycle projections are intentional. Fresh PostgreSQL 17 connected adoption
  proof passed 1 test with 0 ignored, preserving Type, mixed ordered Pool/Fixed entries, nondefault
  teaching rules, exact Revision pins, independent daughter Pool IDs, and unset dates. Artifact:
  `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`. No production rewrite was needed;
  existing stable adoption-test fixtures now use opaque public References and explicitly close
  their shared application pool before the race, without raising connection limits. Supplemental
  checks stayed outside Git. Assessment splice, diff/consistency, gate, and `git diff --check`
  passed: 392 verified, 349 open, 338 owning-open, 43 N/A across 784 bullets. Every Type's Student
  delivery/completion remains outside this architecture receipt.

- Closed only HG608's distinct algorithmic-Question Pool purpose. Accepted fresh PostgreSQL
  17/MinIO actual-server and private bundled-main HTTP-proxy browser proof selected two distinct
  canonical PGML Questions with Instructor interchangeability attestation, created a reusable
  Pool and distinct Assessment-owned fork selecting one Question, preserved exact provenance,
  reproduction facts, and radio response on resume, and submitted before a fresh new Attempt.
  Artifact: `/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. Correct answer Never and a
  3600-second time limit bounded the release; answer disclosure, full Live Demo/authentication/TLS,
  all-backend acceptance, and other HG rows remain outside this receipt. Questions splice,
  diff/consistency, gate, and `git diff --check` passed; totals are 391 verified, 350 open,
  339 owning-open, and 43 N/A across 784 bullets.

- Closed the seven bounded algorithmic-source Human Guidance rows. Accepted fresh PostgreSQL
  17/MinIO evidence published the 42 canonical Genetics PGML sources as ordinary WeBWorK
  Revision-1 Questions in nine topics with 42 Fixed entries and zero Pools; exact replay and a
  same-short-name conflict made no mutation. Pilot source/tests enforce explicit PG/PGML format and
  matching extension; connected binding proof preserved source SHA/size/path, immutable replay, and
  stale refusal after an intervening metadata edit. Artifacts: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`
  and `/private/tmp/ple-pilot-format-binding-artifacts.CKzka1`. Chargaff, the non-published
  76-bank/13,434-row inventory, and conditional retained-catalog C840--C841 work remain open.
  The earlier Pilot PGML mismatch statement is superseded by this receipt.

- Closed the exact C351 manual Draft-deletion checklist row. Accepted isolated PostgreSQL 17/MinIO
  actual-server and focused browser proof covered owner cancel/confirm and list-reload persistence,
  owner-only current-ETag deletion, denial without source/Edit Number change for collaborator,
  unrelated Instructor, Student, Sysadmin, and anonymous callers, precondition failures, preserved
  published lineage/Revision JSON, and 404 repeated mutations. The artifact is
  `/private/tmp/ple-draft-delete-artifacts.km9ybM`; this narrow receipt does not claim S3 erasure,
  automatic cleanup, authentication acceptance, full Live Demo browser acceptance, or healthy
  backend behavior with the renderer disabled.

- A fresh Live Demo run passed all 51 screenshot captures and canonical corpus promotion; `SCREENSHOT_ATLAS.md` was
  regenerated, the root static verifier passed, and the three new published PNGs (Template, Pool review, and canonical
  Genetics PGML) received visual inspection. The two real repairs are the native Draft codec and Student View's stale
  `assessment_attempt_limit` manifest key, now `attempt_limit` in its audited reader projection. Current capture
  selectors and the released-feedback privacy profile are corrected. Thirty-one focused Node tests, 7,154 Python tests,
  and PostgreSQL `cargo check` passed. Standard `--headed` operator recapture is documented; the external authenticator
  was temporary proof-only and adds no permanent credential plumbing. This does not claim broader Human Guidance
  completion. Pilot PGML source mislabeled PG by `pilot_content/publication.rs` remains a separate open mismatch.

- Repaired Live Demo screenshot startup: removed the stale browser-compose identity-initializer command and API
  volume overrides, inheriting the canonical TOTP seed, wrapping-key initialization, and read-only API mount.
  Local topology now owns the legitimate `ple_sysadmin_totp_runtime`; focused 61-test ownership/startup checks
  passed, and `./launchers/run_live_demo.sh stop` reports `Developer browser stopped: ple-live-demo-browser`. Startup
  now succeeds; 13 installer and 11 diagnostic tests passed. Broader C351 runtime behavior and final runtime
  acceptance are not claimed.

- A fresh Live Demo replay promoted all 54 canonical screenshot captures and regenerated the atlas and receipt.
  The final three published 1280x800 PNGs were visually inspected: HLA offspring, monohybrid matching, and X-linked
  offspring counts; each was readable unanswered biology. The capture selector now recognizes renderer CSS tables.
  Static verification, 5 Node tests, and 7,151 pytest tests passed. The temporary private external-authenticator
  runner added no permanent credential plumbing, application code, or tests; this narrow evidence does not claim
  broader Human Guidance closure.
