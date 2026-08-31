# Implementation status and handoff

Last updated: 2026-08-31

This is the sole mutable registry for the global current-package handoff and shared migration
allocations. The [implementation plan](implementation_plan.md) and active
[release completion plan](active/release_completion_plan.md) own architecture, scope, dependency
order, validation, and acceptance. SD1 cutover authority is recorded in this registry and its
current plan. Durable product decisions remain in
[Human Guidance](../HUMAN_GUIDANCE.md); package history and detailed receipts remain in the
[changelog](../CHANGELOG.md).

Closed package receipts remain in the changelog and retained package reports.

Work-package labels such as `WP-INST-G2` are temporary plan coordinates. They identify the current
handoff while the plan is active and retire with the planning layer; product contracts and durable
data use domain identifiers.

## Current handoff

- **Current scoped correction package:** `WP-SD1-A-TERM-01` owns direct
  terminology alignment against Human Guidance and the terminology contract.
  Its completed slices include removal of the retired installation-scope and
  generic authorization labels,
  Question Version, Question Variation, Question Seed, Question Feedback,
  Response Item Reference, Question Attempt Limit, Question Pool Selection
  Rule, Question Presentation integrity, Question Attempt Reproduction Details, Question Backend Version, Question Renderer Version, Question Grader Version, Object Address, Object Storage Area, Object Category removal, Object License and Object Data Class, Assignment Scoring State and Assignment Scoring Snapshot, External Question Provider Cache Entry, and iMathAS Draft Question Source terminology through affected code,
  generated contracts, tests, and durable documentation. Evidence includes
  workspace compilation, focused Rust and browser-decoder suites, TypeScript
  checking, Markdown-link checks, formatter gates, and `git diff --check`.
  The fresh baseline's private `question_attempt` record now stores the exact
  Question Seed, generated-parameter SHA-256, closed Question Attempt State,
  and Question Attempt Reproduction Details under their canonical names.
  Deferred schema constraints require `SubmissionAccepted` to commit with one
  Question Submission and require `Open` or `ClosedAtDeadline` to commit
  without one; a forward-only state trigger preserves terminal history.
  The connected disposable database verification remains unrun because the
  configured acceptance-runtime manifest locator is invalid; source, model,
  adapter, TypeScript, documentation, and SQL-style gates are current.
  The Answer-versus-Solution model boundary and database split of generic
  grading-material records remain open owner-dependent work; this scoped
  package does not advance SD1-A acceptance.

- **Current package:** `WP-SD1-A-decisions-and-impact-contract` - establish the single-installation
  ownership model, equal Teaching Team Member authority, open Instructor-visible published-question
  Question Library, stable `QuestionId` lineage with immutable `QuestionVersion`s and explicit forks,
  exact course/Student FERPA authorization, deterministic automated grading, Sysadmin-approved
  `ForcedQuestionCorrection` replacement evidence, the affected-owner register, and the fresh
  migration epoch. A1-A4 implementation and the A5 pre-acceptance documentation slice are
  recorded below; independent A5 architecture/privacy `ACCEPT` remains pending. The unaccepted
  WN1-SR5 PostgreSQL vocabulary work is input to SD1-C rather than an acceptance boundary.
  WN1-OPS1 through WN1-OPS10, WN1-B1 through B5, WN1-GO1, WN1-MG, and WN1-SR1 through SR4A
  remain accepted behavior evidence.
  `WP-SD1-A` records the implemented fixed-role Account and Authenticated Session foundation: each
  account and session carries one immutable Student, Instructor, or Sysadmin role; Student/Instructor
  membership must match it; Sysadmin Course Creation assigns an approved Instructor account without
  Sysadmin membership; and course help remains explicit audited support. The schema, session Store,
  mounted session response, and unmounted browser route contract carry that boundary. The browser
  contract admits a Sysadmin only to platform Instructor approval, while Instructor course and
  Question Library surfaces require an Instructor account. Aggregate service, PostgreSQL/RLS, and
  release acceptance remains open.
  **Question Version identity cutover:** the Question Model now owns one exact
  `QuestionVersionReference { question_id, version_number }`; the Question ID
  is its stable lineage and the positive Question Version Number is its
  immutable version within that lineage. The baseline schema and Question Model
  tests are aligned. Object Storage and the Native and WeBWorK adapters now
  carry the same exact pair; object keys, deterministic object addresses,
  cache identities, and renderer boundaries bind both values. iMathAS now binds
  render caches, provider-grade correlation, restored handles, and scored-embed
  ledgers to the pair. The generated API and browser strict decoders now carry
  the same pair and reject retired fields. The unmounted Base Course installer
  and its unreachable command adapter are retired because they depended on an
  absent publication module and obsolete Store interfaces. Service and the
  mounted live-demo installation lifecycle remain explicit downstream cutover
  work; no compatibility identity is retained.
  **SD1-C Memory curriculum-adoption progress:** the M2-M5 Memory slice is independently accepted.
  It provides stable Blueprint child identity and immutable revision history, the closed five-method
  and seven-variant Store boundary, one-lock account/key/digest replay and rollback, exact immutable
  receipts, controlled updates, selected copies, rollover and term shifting, answer-free audited
  inspection, and receipt-targeted reconciliation. Twenty-five compact public-Store behavior tests,
  strict all-target `test-support` Clippy, and the full feature-enabled LDA suite pass. This is
  Memory acceptance only; PostgreSQL/RLS, service, browser, live-stack, and aggregate evidence remain
  required and are the current downstream cutover.
  **Current pre-WN1:** lower-camel transport remains in
  material source. **Approved target:** Rust Serde owns PLE `snake_case` data-object properties,
  query keys, and portable discriminants while TypeScript functions/locals and registered
  protocols retain owner conventions. C4-IA1 owns the direct item-analysis route/client contract;
  QM-CAPABILITY owns capability-discriminant spelling.
- **SD1-B1 preparatory progress:** `SD1-B1-P0` is accepted as a bounded identity-only receipt and
  does not advance the current handoff or satisfy `WP-SD1-B1`. `SessionId` is distinct from the
  one-way browser credential hash and belongs to a resolved `SessionRecord`. `WP-SD1-B1-P1` follows
  `SD1-C2`, makes `SessionRecord` own `SessionId` and expose its sole resolved-record account/session
  facts, and precedes D1.
  `SD1-B2` through `SD1-B4` define exact-scope contract roots. SD1-C/D then implement and prove
  their schema, Store/RLS, and direct service support. `SD1-B1-F` integrates that completed support
  and removes the retained global-scope `SessionSubject` and duplicate `AccountSession*` models
  in one compile-coordinated convergence. The duplicate code has now been removed and the
  canonical SessionStore foundation checks, including a replacement deployment-gated seeded-demo
  route that issues only the ordinary Authenticated Session. The HTTP policy, composition proof,
  API contract, browser DTO and decoder now expose only `{ account: { id, role } }` for that
  mounted surface; the retired user profile and multi-role session shape are gone. The mounted
  browser entry is the seeded-demo selector only; retired email-completion and email-change pages
  no longer promise absent routes. The
  data-access ceremony contract now returns a trusted existing Account and immutable Product Role
  for either email-code or passkey verification, leaving session creation solely to SessionStore.
  Email-code and passkey adapters must be reconstructed on that route foundation; server-wide
  compilation and B1 acceptance remain open. The browser route map, navigation, client boundary,
  permanent transport tests, and Instructor documentation now expose only the mounted seeded Live
  Demo selector; retired Account-security, email-change, and passkey browser contracts have been
  removed rather than represented as live functionality.
- **SD1-B2/B3 preparatory contract receipts:** `WP-SD1-B2-A` is independently accepted for its
  pure authorization roots: active approval, exact current-course Instructor membership, exact
  Student ownership bound to the protected membership episode, and `CourseCreationIntent` that
  identifies an intended initial Instructor without granting authority. `WP-SD1-B3-A` is
  independently accepted for its server-only Change Proposal lifecycle: checked
  semantic/grading-impact classification, exact-head and minted-successor witnesses, public
  contributor credit, stale rebase/resubmission, and no browser aggregate. These receipts keep
  `WP-SD1-B2` and `WP-SD1-B3` incomplete pending their remaining contract roots and SD1-C/D
  Store, PostgreSQL/RLS, service, and browser implementations. Their completion order remains
  before `SD1-B1-F` and `SD1-B5`; focused offline gates and independent `ACCEPT` cover only the
  listed contract roots, not runtime, PostgreSQL, or browser completion.
- **SD1-B3 curation preparatory receipts:** `WP-SD1-B3-B1` is independently accepted for the
  server-only `QuestionStar` relation intent. Its private fields are only the global `AccountId`
  owner and lineage `QuestionId`; it has no installation scope, institution, session, role, Student, version,
  Question Folder, source, or answer data, has no Serde boundary, and is publicly reachable only as
  `learning_data_access::QuestionStar` through the crate root. Construction and accessors express
  relation presence only; authenticated-session resolution, approved-Instructor authorization, and idempotent
  persistence remain protected service/Store work. `WP-SD1-B3-B2` is independently accepted for
  the private-owner server-only `QuestionWatch` aggregate. Its closed `QuestionWatchTarget` is
  exactly a published `QuestionId` lineage or an exact `QuestionVersionReference`, and its closed
  `QuestionWatchNoticeKind` has exactly `Version`, `Fork`, `ImprovementThread`, and `Impact`.
  Watch has no installation scope, institution, session, role, Student, delivery, notification-preference,
  browser, source, or answer data, has no Serde boundary, and remains non-authorizing; delivery
  belongs to a later service layer. The Star and Watch receipts do not claim Question Folders, Saved
  Question Searches, sharing, selection, SD1-C/D persistence or services, or B5/browser work. `WP-SD1-B3`
  remains incomplete pending those boundaries; focused format/check, the existing 141-warning
  baseline, direct source-size checks, and independent `ACCEPT` cover only these value contracts.
- **SD1-B3-B3 preparatory Question Folder receipt:** `WP-SD1-B3-B3` is independently accepted after
  the report 40 identity-opacity correction and report 43 final recheck, with reports 34 and 38
  recording the approved architecture and implementation evidence. `NamedQuestionCollection`
  owns a new opaque server identity, immutable global `AccountId` owner, canonical validated title,
  storage-safe strong revision/CAS behavior (explicit stale expected/actual conflict, equal-state
  no-op, and checked exhaustion), and bounded ordered unique exact `QuestionVersionReference` pins. The
  child module is private and the selected API is crate-rooted; no browser, installation scope, institution,
  sharing, route, Serde, or authorization path enters this value contract. Eight focused
  deterministic behavioral tests pass. `WP-SD1-B3` remains incomplete pending saved searches,
  Question Folder sharing, selection, SD1-C/D Store/PostgreSQL/RLS/service work, B5, and browser/live
  work; this receipt claims no runtime, persistence, or browser acceptance.
- **SD1-B3-B5 preparatory Question Folder-sharing receipt:** `WP-SD1-B3-B5` is independently accepted
  after report 46's `REVISE` and report 47's final `ACCEPT`, using report 42's architecture and
  report 45's implementation evidence. `NamedQuestionCollectionShare` is one server-only,
  non-Serde, non-authorizing, recipient-specific relation over an exact existing
  `NamedQuestionCollectionId`, immutable owner and distinct recipient `AccountId`s, and exactly
  `Active`/`Revoked` state. Self-sharing is refused; grant/reactivation and revoke expose
  explicit changed/unchanged outcomes. The private child module selectively re-exports its
  closed API through the curation facade and crate root. The relation carries no visibility,
  access-level, collaborator/editor, publication, installation scope, institution, session, role, Student,
  browser, approval, authorization, persistence, or audit field, and does not itself grant
  access. The corrected full-target gate is
  `cargo test -p learning-data-access --features test-support question_curation::collection_share`;
  it passes all five matching unit tests and compiles the package integration targets with zero
  matching tests. Report 45's `--lib` selector is retained only as narrowed evidence, not the
  acceptance gate. Focused format/check, the existing 141-warning baseline, direct source-size
  counts (209, 22, and 349 lines), and independent acceptance cover only this value contract.
  SD1-C/D still own authoritative-time B2-A recipient approval, owner-only authorization,
  transactional uniqueness and owner consistency, persistence, RLS/broker behavior, absent
  concealment, revoked-read denial, and any later audit. SD1-B5/F owns browser-safe projections
  and visible owner/recipient workflows. `WP-SD1-B3` remains incomplete pending saved searches,
  selection, these downstream Store/PostgreSQL/RLS/service boundaries, B5/F browser work, and
  live/release completion; no runtime, persistence, or browser acceptance is claimed.
- **SD1-B3-B4 preparatory saved-search receipt:** `WP-SD1-B3-B4` is independently accepted from
  reports 56, 57, and 59 for the server-only `NamedQuestionSavedSearch` value aggregate. It retains
  one immutable global `AccountId` owner, one opaque server-only UUID identity, one validated title,
  one normalized no-scope `QuestionSearchFilter` (`text`, `bylines`, `backends`, `tags`,
  `question_types`, `taxonomy`, `capabilities`, `licenses`, `evidence`, `used_in_my_courses`,
  and `authorship`), and one positive storage-safe revision. The aggregate has no installation scope, course,
  saved-owner identity, cursor, page size, route, DTO, browser, or Serde boundary; reruns use a fresh
  current-Question-Library query, with account-bound filters evaluated for the rerunning Account. Its revision-CAS
  boundary rejects stale expected revisions with explicit expected/actual evidence before candidate
  work, makes normalization-equivalent state an `Unchanged` no-op, increments changed state exactly
  once, and refuses checked exhaustion without mutation. Eight deterministic full-target behavior
  tests pass, covering owner/identity opacity, title/filter rejection, initial and canonical filter
  state, fresh-query continuation absence, normalized no-op, changed replacement, stale conflict, and
  exhaustion. C/D still own Store/PostgreSQL persistence, global-owner and `PS-*` mapping, canonical
  bytes/digest/schema validation, uniqueness/cap/concurrency, authorization/concealment, broker/RLS,
  and protected service behavior; B5 owns browser-safe projections and B5/F/G own route, live-browser,
  and visual acceptance. `WP-SD1-B3` remains incomplete pending selection and these downstream
  completion boundaries; this receipt claims no runtime, persistence, authorization, RLS, or browser
  acceptance.
- **SD1-B3-B6 preparatory selection receipt:** `WP-SD1-B3-B6` is a child execution package under
  existing `WP-SD1-B3`, not a new top-level roadmap package or migration allocation. Its durable
  selected result remains `QuestionVersionReference`; exactly one `is_eligible_for_ordinary_new_selection`
  predicate admits only Published versions for new references. Deprecated and Archived versions remain
  authorized exact-pin history. Current server and Memory consumers re-resolve at the destination;
  no selection aggregate or browser-trusted exact version exists. The manager repair requires a retained
  Deprecated/Archived pin to keep an existing authorized, visible publication. Passed manager gates:
  `cargo fmt --all --check`; question-model 9+3+2; curation 4; curriculum 8; policy 2; reusable
  curriculum 2; server 10. This receipt does not claim SD1-C/D persistence, PostgreSQL/RLS, services,
  browser/live, or aggregate acceptance. `WP-SD1-B3` remains incomplete pending those boundaries.
- **SD1-B3-B7 preparatory improvement-event receipt:**
  `WP-SD1-B3-B7-improvement-event-contract` is an accepted preparatory child of existing
  `WP-SD1-B3`, with no new top-level roadmap package or migration allocation.
  `QuestionImprovementEvent` is one immutable server-only, non-Serde value that retains its opaque
  event identity with the exact proposal and base-version ancestry. An accepted event retains its
  same-lineage advancing immutable successor; a resubmission retains its new exact proposal/base
  ancestry and its distinct predecessor proposal/base linkage. The value contract rejects
  self-reference, lineage drift, and non-advancing predecessor or successor versions. Contributor
  credit remains owned only by `QuestionChangeProposal`, so the event contains no byline or other
  credit field. The crate facade exports only the selected event surface; persistence ordering,
  authorization, services, transport, browser behavior, SD1-C/D, and release closure remain with
  their downstream owners. The default `question_stewardship` selector passes after the durable
  Cargo integration-target boundary marks only `conformance` and `course_creation_memory` as
  `test-support` targets; the default production feature set remains empty. The B3-B6 Memory
  conformance fixture now creates its retained pins while Published and then deprecates the exact
  retained visible pin before updating, preserving the Published-only ordinary-new-selection rule.
  These focused acceptance and maintenance receipts keep `WP-SD1-B3` and SD1-C/D/browser/live/
  aggregate completion open.
- **SD1-B3 Question Library scope query-retirement receipt:** `WP-SD1-B3-CATALOG-SCOPE-QUERY-RETIREMENT`
  is independently reviewed `ACCEPT-PREPARATORY` under report 41 and implementation reports
  49-54, with report 55's final review. One no-scope, direct `snake_case` Question Library and Saved Question Search
  meaning now converges across the Rust query roots, Memory and PostgreSQL query code, server
  parsing and saved-search boundary, regenerated TypeScript contracts, and browser clients,
  feature models, and tests. The passing focused gates are `cargo fmt --all --check`, focused
  `question_model` Question Library-facet tests (3/3), Memory Question Library search (13/13 plus the shared-Question-Library
  test 1/1), the PostgreSQL cursor-fingerprint test (1/1), server Question Library query (2/2), server
  Question Library HTTP (4/4), Saved Question Search HTTP (7/7), `cargo tools tsgen` (482 declarations), both
  repository TypeScript configurations, the six-file Question Library/curation/picker Node lane (33/33),
  and the source-line-limit check (1,856/1,856). `WP-SD1-B3` and this package remain incomplete
  pending the fresh SD1-C schema/broker rewrite and its connected live PostgreSQL oracle, followed
  by the required final material-tree gates. Record-level `PublicationScope` remains a separately
  deferred publication/asset security boundary; this receipt does not retire that record authority
  or claim persistence, production-browser, or full-package acceptance.
- **SD1-B4 preparatory contract receipt:** `WP-SD1-B4-J1` is independently accepted for one
  server-only, non-Serde `JobTargetSelector` that exhaustively projects the ten current
  `JobPayload` families into bounded target and generation evidence. It is non-authorizing and
  retains one queue/broker boundary. The `jobs` facade and selector module are both below the
  source-size limit; seven focused tests, formatting, the default warning baseline, source-size,
  and independent `ACCEPT` are green. `WP-SD1-B4` remains incomplete: SD1-C/D must resolve these
  selectors into locked exact-scope manifests and retire obsolete global-scope queue authority.
- **Acceptance-open predecessor:** `WP-INST-G2` is implemented and acceptance-open behind
  `WP-INST-WN1` and its remaining G2 visual/documentation close-out. Its approved
  retained Gradebook evidence establishes the roster-first `CourseGradebookStore`, atomic-audit
  `StudentWorkInspectionStore`, and
  migrations `2026081870` through `2026081878`; G2 W5/W6 resume after WN1 acceptance.
  `CourseGradebookStore` owns the roster-first, server-calculated page; a dedicated
  `StudentWorkInspectionStore` owns one explicit, atomic-audit, solution-free detail read. The package reserves
  migrations `2026081870` through `2026081878` as authority foundation, private immutable witness,
  only app-executable broker, query evidence, historically global-scope-bound worker failure, a forward broker
  rowset-contract repair, and server-owned safe detail labels; it preserves
  answer-free navigation and G1 receipts and proves the ordinary Student-to-Instructor workflow on
  the canonical real stack.
- **Current acceptance predecessor:** `WP-INST-G1` accepted 2026-08-28. Course-scoped Instructor
  operations route deterministic grader exceptions through bounded retry and generation-fenced
  recalculation; immutable accepted Student work recovers through an answer-free status reader;
  and the ordinary grading worker publishes the current total. The final aggregate passed Rust and
  Wasm, 369 Node tests, 7,978 pytest checks, every production-browser scenario, all 99 migrations
  and PostgreSQL/RLS/worker oracles, isolated WebWork, replica restart and durable replay, and exact
  cleanup. Independent architecture, security/privacy, and HCI reviews accepted the boundary with
  no P0/P1/P2 finding.
- **Accepted prerequisites:** `WP-INST-S1` through `S7`, `T1` through `T3`, `BS1`, `LD1` through
  `LD3`, `T5`, `T6`, `D1`, `D2`, `B1`, `B2`, and `G1` are accepted. Their scopes and evidence are
  retained in the owning plans and changelog.
- **Release handoff:** `WP-RC8` remains parked and acceptance-open. It owns provider/mailbox,
  unrelated passkey, multi-replica, security, HCI, and release gates. Instructor live-demo work does
  not imply production onboarding, deployment, or release acceptance.

## SD1-A implementation receipts

`SD1-A1` implementation is complete. Human Guidance, Design Decisions, User Roles, and the SD1
authority documents now bind one installation with global accounts, equal approved Instructors and
Teaching Team Members, exact course/Student ownership, shared published-question discovery, and the
approved-Instructor predicate for course creation. Sysadmin status alone is insufficient; current
Instructor approval is required.

`SD1-A2` implementation is complete. Graphify-assisted navigation and direct source inspection
record the affected Rust, migration, browser, worker, object, live-stack, and documentation owners
in the SD1 scope register. The graph and inventories are one-time evidence; current source remains
the authority.

`SD1-A3` implementation is complete. The PostgreSQL table, key, policy, grant, broker, and typed
scope register allocates the fresh `WP-SD1-C` epoch as `2026082901` through `2026082936`. Historical
`2026081881` and `2026081882` WN1-D work is retained as evidence/input absorbed by that fresh epoch,
not as an active SD1 schema dependency.

`SD1-A4` implementation is complete. Browser, local-stack, live-demo, and binding documentation
consumers are assigned successors in the scope register; the canonical database authorization
reference replaces the retired global-scope authority. Focused per-file ASCII, whitespace, and the
repository-wide Markdown-link gate pass; deferred target paths are named as target work rather than
available interfaces.

`SD1-A5` implementation is complete for the pre-acceptance documentation and authority-repair
slice. The supplied independent architecture/privacy review remains `REVISE`, and the handoff
review remains `BLOCKED`; no independent `ACCEPT` is recorded. Runtime, PostgreSQL/RLS, browser,
and full-suite acceptance remain later SD1 gates.

`WP-SD1-A` fixed-role account clarification is documented. The `2026082906` protected session
broker and `SessionStore` now derive the session Product Role from the immutable Account record;
the staged PostgreSQL gate and focused Rust tests pass. Migration `2026082902` retains ownership
of singular immutable account/session role storage; `2026082905` retains ownership of Instructor
vetting and current approval predicates. Passwordless-ceremony, browser, broader PostgreSQL/RLS,
and human-acceptance evidence remain pending.

`WP-SD1-A` Student Record ownership repair is implemented. Migration `2026082915` now makes the
protected Student Record unique for its exact Student Account and Course Instance, and each Student
Course Membership binds that stable record. Domain authority checks and the PostgreSQL capability
broker use the same relationship, so a re-enrollment receives a new membership episode while
retaining its original Student Record. The staged PostgreSQL baseline, focused domain tests, and
Markdown-link gate pass; broader WP-SD1 acceptance remains separately allocated.

`WP-SD1-A` terminology repair now distinguishes the human **Grader** relationship from the
server-only `ple_automated_grading` capability and its `ple_grading_reader` login. Active source,
browser copy, tests, and durable documentation use Student terminology rather than the retired
role label. Rust contracts, local runtime-manifest checks, and the staged PostgreSQL baseline
pass; broader WP-SD1 acceptance remains separately allocated.

`TERMINOLOGY_CONTRACT.md` now records the settled Instructor-facing assignment hierarchy:
Assignment, Assignment Attempt, Question Attempt, Submission, and Response. The public
`RunPolicies` has been cleanly replaced by `AssignmentActivityRules` in the Rust model,
the `assignment_activity_rules.rs` module, generated TypeScript surface, browser imports, and
focused documentation. The eight rules now remain separate through their model-to-browser contracts,
the complete immutable `AssignmentRevisionDefinition`, and explicit constrained
`assignment_revision` baseline fields; broader SD1 write-path and delivery enforcement remain
separately allocated.

The browser boundary now uses an explicit `StudentQuestionAttemptView` generated from the
durable server-held `QuestionAttempt`. The view carries only Student-visible identity,
presentation binding, submission, state, timing, and issued capability; reproduction details and
generated-parameter evidence are absent from both its Rust and TypeScript contracts. Scoring
freshness and Question Pool selection remain the separate authorized additions to that Student
view at the browser transport boundary.

The public navigation seam is no longer part of that remaining route vocabulary:
`NavigationResolution::AssignmentAttempt` carries `assignment_attempt_id`, and its generated
`assignmentAttempt` wire variant is strictly decoded before browser route resolution.

The Assignment Attempt browser boundary now completes its direct public cutover: browser routes,
Gradebook transport, page and recovery owners, route parameters, Course Theme routing, and visible
inspection copy use `assignment-attempts` and Assignment Attempt terminology. `npx tsc --noEmit
-p tsconfig.json`, the calculated-Gradebook/navigation/recovery browser-contract tests (12), and
`git diff --check` pass. The broader Course Theme suite remains separately incomplete because its
pre-existing Course Curriculum route expectation has no executable route owner; it is unrelated to
Assignment Attempt route resolution.

The scored-completion cutover is also complete at its immediate contract boundary:
`CompletedAssignmentAttemptScore` and `AssignmentAttemptGradeSelection` carry explicit
Assignment Attempt identities, the grade selector and recalculation Store use those terms,
and terminal browser copy names an Assignment Attempt. Focused domain scoring tests,
learning-data-access compilation, the completion-copy regression test, and the TypeScript
check pass. Broader `Run*` filenames, CSS hooks, and route internals remain separately
allocated mechanical input; they do not define the product or public data contract.

The derived activity cutover is complete at its immediate contract boundary:
the Student-facing `AssignmentProgress` generated contract is key-free, while
`AssignmentProgressRecord` is the server-side projection carrying exact Student Record and
Assignment references. It remains distinct from immutable Assignment Attempts and the selected
`AssignmentGrade`. Question-model activity tests, domain continuation tests, the strict Student
progress decoder regression, contract regeneration, Rust compilation, and TypeScript checking pass.

The Assignment Attempt Completion cutover is complete at its immediate contract boundary:
`AssignmentAttemptCompletion` is the derived `InProgress` or `Completed` value, and the receipt
field is `assignmentAttemptCompletion`. Completion derivation, strict receipt decoding, the
attempt-state machine, terminal presentation, generated contract, and focused fixtures use that
term without treating availability of later Assignment Attempts as completion evidence.

`AssignmentProgressScoreState` now names the no-activity, withheld, and available disclosure
states within Assignment Progress. It is derived from retained activity and the Student Feedback
release decision; it carries no scoring authority. The generated contract, Student-progress decoder,
Question Model activity tests, focused browser tests, TypeScript check, and Markdown gate pass.

The obsolete `domain::attempt` lifecycle model has been removed. It had no production consumer and
collapsed Issued Question Progress, Question Attempt state, Question Submission, and Grading Result
facts into one competing abstraction. Domain tests now exercise the retained authoritative models;
the complete Domain library suite and the Markdown gate pass.

The Question Submission and Grading Result ownership cutover is complete at its immediate contract
boundary: a Question Submission owns the accepted Student Response, and its optional Grading Result
is bound to the exact submission, automated grading operation, and receipt. The strict generated
browser contract and decoder preserve that nesting; the fixture reproduction test, backend adapters,
and fresh SD1 PostgreSQL acceptance prove the same chain.

The unreachable mixed `CompletedSubmissionReceipt` and submission-completion source have been
removed. They were not declared by the active data-access crate and referenced a nonexistent
`SubmissionRecord`, so they could not provide an executable persistence contract. A future Question
Submission Receipt remains allocated only with its exact immutable evidence and active Store boundary.

The SD1 baseline schema now stores every immutable Question Version as the canonical
`(question_id, version_number)` pair. Composite foreign keys cover Question Library lifecycle, stewardship,
private grading, Issued Questions, analysis, corrections, Jobs, external-tool state, and object
delivery. The fresh/no-op staged PostgreSQL acceptance and restricted principal probes pass.

## WN1-A review receipt

`WP-INST-WN1-A` is accepted on 2026-08-28. Two independent repair reviews returned `REVISE`; the
fresh v3 review returned `ACCEPT` after the ledger added exact automated item-analysis fields,
retained-or-retired Student targets and consumers, root-script mappings, live-product-document and
filename dispositions, four authority signatures and direct SQL callers, C-row ownership for
normative examples, C6 routing ownership, and atomic migrations `2026081879` through `2026081888`.
Atomic C1-C6 children name every matrix-identified Axum producer, direct DTO, browser reader,
dependency, and narrow gate. Source/type-level QM children, WN1-WA bridges, and WN1-D durable owners
preserve external QTI XML/archive, standard headers/static paths, and other registered boundaries.
`WN1-MG` precedes C3. `WN1-C4-IA1` closes the current item-analysis browser gap, and
`WP-INST-G3-IA1` supplies the later visible Instructor workflow. G2 remains implemented and
acceptance-open; W5/W6 resume only after WN1-F accepts the final tree.

Focused evidence is one-time Graphify-assisted source inspection, three independent allocation
reviews, and the documentation hygiene commands recorded in the WN1-A handoff. The graph is
commit-mapped navigation evidence; the current material source is final truth. The exact ledger
registers cover 21 Store/status names, eight browser decoders, twelve broker policies, four
functions/fences, two root scripts, live product documents, and current filename dispositions.
The known six Markdown-link failures remain final material-tree evidence-open because the two WN1
documents are absent from the current material-tree inventory. A 2026-08-29 current-source naming
review subsequently found two allocation deltas: orphaned legacy `ts-rs` output and private shell
state beyond the two OPS1 scripts. The ledger now assigns them to `WN1-GO1` and atomic
`WN1-OPS2` through `WN1-OPS10` children; the shared-template scripts retain their external owner.
This receipt accepts the reviewed architecture plus that explicit delta allocation; it does not
claim product behavior, WN1 implementation, or final material-tree acceptance.

## WN1-B implementation receipt

`WN1-OPS1` is accepted on 2026-08-29. The root live-demo and build front doors now use lowercase
`snake_case` for every plan-allocated script-private variable while retaining their exported-process
boundary, argument contract, build stages, and visible output. Shell syntax, both help paths, and the
exact retired-name inventory pass. No permanent source-inventory test was added.

`WN1-GO1-orphaned-generated-output-retirement` is accepted on 2026-08-29. Graphify and direct
source inspection found that the legacy `crates/question_model/bindings/BackendCapabilities.ts`
reached only its sibling `Capability.ts`, with no production consumer. Both files are removed;
`crates/project-tools -> generated/api` is the sole active TypeScript contract owner. The canonical
generator regenerated 482 direct declarations, all 63 project-tools tests pass, both repository
TypeScript configurations compile, formatting and strict project-tools Clippy pass, and the final
consumer inventory is empty. Regeneration and consumer inspection are one-time evidence; existing
generator behavior and TypeScript compilation remain the permanent gates.

`WN1-OPS2-root-aggregate` is accepted on 2026-08-29. `all_test.sh` uses lowercase
`script_directory` for its sole private path while preserving the five-stage Validation front
door and exported process boundary. Shell syntax passes; the exact uppercase-private-name
inspection is one-time evidence. The complete aggregate remains the required WN1-F and final-tree
gate rather than becoming a duplicate test for this private-name change.

`WN1-OPS3-browser-front-doors` is accepted historical evidence from 2026-08-29.
Its screenshot-corpus command and browser-owner configuration were later
retired with the former corpus. The current tree has no runnable browser
acceptance owner; restoring one is separate work and remains required before
browser or visual acceptance can be claimed.

`WN1-OPS4-rust-front-door` is accepted on 2026-08-29. `check_rust.sh` uses lowercase
`script_directory` for its private repository path while retaining the exact eleven-stage offline
Rust gate, argument handling, and exported process boundary. Shell syntax and the visible help
contract pass. The next ordinary Rust lane remains the permanent behavior evidence.

`WN1-OPS5-wasm-build` is accepted on 2026-08-29. `pipeline/build_wasm.sh` uses lowercase
`script_directory`, `cargo_profile`, `profile_dir`, and `wasm_input` while retaining its command
arguments and generated-output boundary. Shell syntax passes; the debug Wasm build produced both
web and Node bindgen flavors, and the Node consumer verified format, timer, capability, and
presentation results.

`WN1-OPS6-python-setup` is accepted on 2026-08-29. `devel/setup_python.sh` uses lowercase
`repo_root`, `venv_directory`, `venv_python`, `receipt_path`, and `python_312`. Repository discovery
now comes from the script's physical path, so first-launch setup has no repository-metadata
dependency. Shell syntax passes; the current receipt was reused and the installed PyYAML import
verified without rebuilding the environment.

`WN1-OPS7-wasm-runner-setup` is accepted on 2026-08-29. `devel/setup_wasm_tests.sh` uses lowercase
`repo_root`, `runner_package_id`, `runner_version`, `runner_root`, `runner`, and `actual_version`.
Repository discovery comes from the script's physical path. Shell syntax passes; a fresh pinned
`wasm-bindgen-test-runner` installation succeeded and a second invocation verified the matched
runner reuse path. The exact retired-name inventory is one-time evidence.

`WN1-OPS8-e2e-course-appearance` is accepted on 2026-08-29. The shell now uses lowercase
`snake_case` for its private path and lifecycle state and delegates ownership to the fixed leased
acceptance controller. The closed `course_appearance_cross_store` profile replaces the obsolete
self-provisioned manifest with descriptor-validated PostgreSQL and MinIO inputs, exact Compose
authority, and pre/post reset. Focused Rust, Python, shell, source-size, and naming gates pass; the
real PostgreSQL-to-MinIO cleanup oracle passed and removed both disposable volumes. PostgreSQL
item-analysis reducer tests were also separated from their 839-line adapter facade when the same
source-size gate exposed that prior organization debt.

`WN1-OPS9-e2e-database-baseline` is accepted on 2026-08-29. The database-baseline script uses
lowercase `snake_case` for its physical script path, disposable workspace, Compose lifecycle,
failure counter, project and volume identity, and expected migration count while retaining
explicit immutable fixture constants in uppercase. Shell syntax and exact retired-name inspection
pass. The fixed leased PostgreSQL owner then passed all 109 tracked migrations, migration
idempotency and verification, the registered live service and RLS oracles, and exact cleanup of its
container, volume, and network.

`WN1-OPS10-e2e-orchestrators` is accepted on 2026-08-29. Both orchestrators use lowercase
`snake_case` for private paths, counters, and failure state. The aggregate now owns all eight
maintained non-browser lanes, including the course-appearance cross-store and isolated WebWork
oracles. Full execution exposed and closed two real boundary defects: generated MinIO credentials
now use one lowercase-hex contract that remains opaque to CLI parsing, and the multi-database
live-demo lifecycle migrates every database before issuing cluster-wide service-role memberships
while the then-current global-scope setup writes carry their registered context. This receipt records
pre-SD1 disposable-stack behavior. Rust formatting, strict Clippy,
22 focused Python tests, 11 runtime tests, Python static analysis, shell syntax, the individual
repaired service lanes, and the final aggregate pass; the aggregate reports 8 passed and 0 failed
with exact disposable cleanup.

`WN1-B1-contract-root` is accepted on 2026-08-28. The workspace now contains pure
`browser-api-contract` with workspace package metadata, only `serde.workspace = true`, and a
documented `#![forbid(unsafe_code)]` admission facade. Cargo generated its lockfile entry. Focused
format, check, zero-test crate execution, strict Clippy, and dependency-tree inspection passed; a
manager `cargo check -p browser-api-contract` also passed. B1 intentionally adds no placeholder DTO
or trivial permanent test. `WN1-B2-source-model` is accepted on 2026-08-28. The former 969-line
generator is now an 87-line facade over focused source, Serde, model, output, and test modules;
every file remains below the repository source-size limit. Manager review confirmed the public
one-root API, source exclusions, cleanup ordering, marker text, generated formatting, and existing
Serde behavior are unchanged. Format, check, strict all-target Clippy, and all 16 focused generator
tests passed, establishing the behavior-preserving baseline for B3-B5.

`WN1-B3-types` is accepted on 2026-08-28 after independent review found and implementation repaired
one invalid-TypeScript edge: exact Serde names containing punctuation now render as JSON-compatible
quoted property keys, while ordinary ASCII TypeScript identifiers retain their existing form. All
wire string literals use the same safe escaping boundary. Explicit literal rename precedence,
container `rename_all` versus `rename_all_fields`, exact tag keys, and fail-closed directional,
duplicate, alias, and unsupported metadata are covered by three focused behavioral tests. The first
review returned `REVISE`; the fresh review returned `ACCEPT`. Format, check, strict all-target
Clippy, and all 19 generator tests pass.

`WN1-B4-render` is accepted on 2026-08-28 after independent review drove two output-safety repairs.
Every emitted declaration retains its source origin; global duplicate names fail with both paths
before cleanup. The renderer uses the truthful contract-roots marker, accepts exactly the two
historical markers for migration, and imports only sorted, non-self dependencies present in the
generated declaration set. Cleanup first validates every TypeScript file, then removes owned files,
so authored or spoofed-marker refusal preserves the existing output byte-for-byte. Two reviews
returned `REVISE`; the final review returned `ACCEPT`. Format, check, strict all-target Clippy, and
all 21 generator tests pass.

`WN1-B5-runner` is accepted on 2026-08-28. The public generator requires a nonempty explicit root
slice; the application owns exactly `crates/question_model/src` and
`crates/browser-api-contract/src`, and `cargo tools tsgen [out-dir]` accepts only an optional output
override. A two-root behavior test proves the direct cross-root import graph, while an empty-root
guard preserves existing output. Independent review returned `ACCEPT`. Format, check, strict
all-target Clippy, and all 23 generator tests pass. One-time regeneration wrote 482 declarations
under the contract-roots marker, and both TypeScript no-emit configurations pass.

`WN1-MG1A-route` is accepted on 2026-08-28. The human-credit GET/PUT module, router declaration,
route-policy authority, and HTTP tests are retired. Automated operations, accepted-submission
processing, normal submission/status, and calculated Gradebook remain the active grading model.

`WN1-MG1B1-outcome` is accepted on 2026-08-29. `QuestionGradingOutcome::NeedsManualGrading`,
`AnswerKey::FileUpload`, and `SubmissionDisposition::NeedsManualGrading` are retired. The incomplete
File Upload Question Response Format, browser control, and free-form object-key Student Response are
also retired: no current response path represents an upload. Independent review returned `ACCEPT`.
Format and affected package checks pass; grading, accepted-submission worker, run,
and project-tools suites pass 6, 18, 43, and 63 tests. Supported graded and ungraded paths, external
committed outcomes, worker retry/fencing, Gradebook, and the transitional attempt/evaluation/store
bridge remain intact.

`WN1-MG1B2-attempt-state` is accepted on 2026-08-29. `QuestionAttemptState` now has exactly
`Open`, `SubmissionAccepted`, and `ClosedAtDeadline`, with direct Serde-owned `snake_case` generation
and strict browser decoding. Memory and PostgreSQL deadline closure atomically close Open work as
answer-free Closed at Deadline state, retain exact action replay, timing cleanup, and audited
evidence, and fabricate neither a response nor a result. Question Attempt Exclusion and Issued
Question Exemption remain separately owned records. The temporary manual Store bridge now uses
Submission Accepted state plus its separate manual evaluation record, and item analysis reads that evaluation
state directly. Independent review returned `ACCEPT`. Manager
format, check, strict Clippy, question-model, Memory/PostgreSQL-capable Store, conformance,
project-tools, TypeScript, and decoder gates pass; the connected absence-evidence worker closure
remains explicitly assigned to a later MG child.

`WN1-MG1B3-question-submission-grading-state` is accepted on 2026-08-29. The authoritative
Question Submission Grading State now has exactly `pending`, `instructor_attention`, `graded`, and
`exempt`; Rust Serde and the generated TypeScript union share that direct `snake_case` contract.
The Student projection admits answer-free Pending, Instructor Attention, or Graded state. The Memory
state aggregate accepts only coherent receipt, execution, and grading-state tuples. Independent
review returned `ACCEPT`. Manager
format, check, strict Clippy, question-model, Store, conformance, project-tools, and TypeScript
gates pass. Architecture review approved MG1C automated item-analysis state followed by MG1D
automated-scoring persistence retirement; the route/browser item-analysis contract remains with
C4-IA1.

`WN1-MG1C-automated-item-analysis-state` is accepted on 2026-08-29. Memory and PostgreSQL now use
one closed automated-evaluation truth table: pending and exception attempts contribute only to
`unscored_attempt_count`; coherent completed grades require immutable completion-receipt evidence
and current-generation scores; and contradictory tuples fail closed. Score-derived assignment
metrics remain suppressed while scoring is incomplete, Student statistics remain concealed, and
the persisted Instructor report contains aggregate fields without Student, attempt, response,
answer, or object identity. PostgreSQL reads only the sealed completion receipt as its boolean
witness and retains worker-private canonical result columns behind their existing capability.
Independent review returned `ACCEPT`. Focused format, check, strict Clippy, domain, question-model,
Memory conformance, PostgreSQL reducer, server projection, TypeScript, and naming/test-tier gates
pass. The registered disposable database baseline passes all 108 tracked migrations, the
Student-owner denial, Instructor/RLS/privacy oracle, generation fencing, and exact cleanup.
MG1D now owns the automated-only runtime and persistence boundary plus migration `2026081883`;
C4-IA1 retains the later direct route/browser contract.

`WN1-MG1D-automated-scoring-persistence-retirement` and the parent `WN1-MG` are accepted on
2026-08-29. Runtime composition now has one automated evaluation owner: deterministic completion,
answer-free exception state, bounded retry/recalculation, immutable grading evidence, calculated
Gradebook totals, and roster score export. Migration `2026081883` closes the parallel manual
receipt, binder, policy, table, and Question Library values while preserving mature invalidation function
bodies through exact fail-closed Question Library rewrites and unchanged identity, owner, ACL, configuration,
and security-mode assertions. The audit found no reachable manual-scoring mutation path; the
assignment-level score-download surface is retired, and decoder plus route-policy checks keep its
former inputs unavailable.

Permanent evidence includes automated Store/worker behavior, strict status decoding, route-policy
authority, and contactless-Student Gradebook/export coverage in Memory and PostgreSQL. One-time
retirement inventory and clean-volume installation remain outside the permanent suite. Format,
focused check and strict Clippy, 236 learning-data-access tests, 81 conformance tests, 423 server
tests with three intentional ignores, 63 project-tools tests, TypeScript compilation, 117 SQL-line
checks, and the fresh 109-migration PostgreSQL/RLS baseline pass. Six independent review passes
accepted the runtime boundary after canonical lifecycle/plan wording, migration ownership,
domain-only diagnostics, and the contactless export gap were repaired. The final WN1 aggregate and
full Validation suite remain later acceptance gates.

`WN1-SR1-disclosure-statistics` is accepted on 2026-08-29. The complete Student Feedback Release
and class-statistics source graph now uses `StudentFeedbackReleaseTiming`,
`StudentFeedbackReleaseRule`, `StudentFeedbackReleaseDecision`, `StudentFeedbackReleaseInput`,
and `StudentClassStatistics`; private Store methods and PostgreSQL modules use
`student_feedback_release` and `student_class_statistics`. Effective Serde, regenerated TypeScript,
reusable-curriculum defaults, and strict browser decoders share one direct `snake_case` contract,
including `student_feedback_release`, `insufficient_evidence`, and
`completed_student_cohort_size`. PostgreSQL columns and stored timing values were already
domain-correct and remain unchanged. Independent review found no remaining naming or behavior
defect in the SR1 material-tree scope after Student terminology was completed.

Permanent evidence retains existing disclosure-timing, stale-score redaction, k-anonymity,
answer-free HTTP projection, Store conformance, generated-client, and strict-decoder behavior; one
available-statistics Serde assertion closes the previously uncovered direct-wire shape. Retired-name
and generated-file inventories remain one-time evidence. The complete Rust front door passes both
feature matrices, strict Clippy, workspace tests, doctests, and the browser Wasm check. The complete
codebase gate passes both TypeScript configurations, ESLint, Prettier, and all 387 Node tests.

`WN1-SR2-student-assignment-projection` is accepted on 2026-08-29. The assignment projection graph
now uses `AssignmentProgressScoreState`, `AssignmentProgress`, `StudentAssignmentLandingSummary`,
`StudentLateStatus`, `StudentAssignmentDelivery`, `StudentAssignmentDetail`,
`StudentAssignmentSummarySnapshot`, and `StudentNotActiveCourse`. The six public projection types
use direct Serde-owned `snake_case` fields and discriminants; regenerated TypeScript and strict
browser decoders match them exactly. Store, Memory/PostgreSQL, server run/course, page, client, and
component type consumers use the canonical identities while the SR3 run/Store vocabulary and SR4
browser function/component/file names remain with their registered successors. The existing
`StudentAssignmentSummary` aggregate retains its separate `QM-ACTIVITY` wire ownership; SR2
renames only the private snapshot that carries it. No PostgreSQL migration was required because
the relational `student_assignment_summary` schema was already domain-correct.

Permanent evidence covers score disclosure and stale-score redaction, pre-activity reads without
receipt materialization, class-statistics disclosure, answer-free Student detail, Instructor
Student view, strict generated-client decoding, and entitlement behavior. Retired-name and
generated-file inventories remain one-time evidence. An independent review first identified an
ownership ambiguity around `StudentAssignmentSummary`; the ledger now states the QM-ACTIVITY
boundary explicitly, and the fresh re-review returned `ACCEPT`. The complete Rust front door passes
contract generation, fixture verification, both check and strict-Clippy matrices, workspace tests,
doctests, and browser Wasm. The complete codebase gate passes both TypeScript configurations,
ESLint, Prettier, and all 387 Node tests.

`WN1-SR3-student-run-store-capability` is accepted on 2026-08-29. The run and Store graph now uses
canonical `Student*` types, `student_*` public and capability methods, `StudentWorkRoutingBinding`,
`StudentSubmissionStatusStore`, Student-named Memory/PostgreSQL modules, and Student-named server
run helpers. Assignment production and behavior modules moved together to `student.rs`; the
external-tool and IMathAS routing-binding graph changed atomically. `GradebookSummaryRow` now
projects `student_name` and its complete PLE-owned DTO uses direct Serde `snake_case`. Generated
run-screen and attempt-descriptor TypeScript modules derive from the renamed Rust owners. No
PostgreSQL migration was required; SR5 retains the coupled pre-migration broker witness vocabulary
until its forward schema and SQLx change.

Permanent evidence covers run issuance, active-membership authorization, prefetch, submission
replay and recovery, answer-free status projection, cross-Student denial, assignment projection,
and external-tool handoff. Retired-name, generated-module, and source-file inventories remain
one-time evidence. Two independent post-implementation reviews returned `ACCEPT`. The complete
Rust front door passes generation, fixture verification, both check and strict-Clippy matrices,
workspace and all-feature tests, doctests, and browser Wasm. The complete codebase gate passes both
TypeScript configurations, ESLint, Prettier, and all 387 Node tests.

`WN1-SR4-browser-direct-clients` is accepted on 2026-08-29. Browser assignment contracts,
strict decoders, route builders, presentation components, progress helpers, response projection,
and recovery helpers now use canonical Student vocabulary without aliases. The ordinary Student
assignment endpoint is `/api/assignments/{assignment}/student` from route policy and Rust handler
through the direct browser client and route tests. `decodeStudentAssignmentLandingSummary` follows
its exact landing type and remains distinct from the activity aggregate's
`decodeStudentAssignmentSummary`, eliminating the former ambiguous export design. Active component,
progress, and response source files use Student names, and the old SR4 symbols and paths are absent.

Permanent evidence covers strict decoding, score disclosure, answer-free Student detail and
response projection, submission recovery, route authorization, and route-policy composition.
One-time searches prove the exact old register absent. Independent review returned `ACCEPT`; the
server all-target/all-feature check, focused Rust route tests, both TypeScript configurations,
ESLint, Prettier, and all 387 Node tests pass. The same review identified non-serialized entitlement
authority names outside SR4, now allocated to SR4A, plus product prose/evidence names retained for
SR6 and final filename disposition.

`WN1-SR4A-student-authority-source` is accepted on 2026-08-29. Non-serialized Rust entitlement,
materialization, assignment-visibility, Memory identity, feedback authorization, and Gradebook
calculation vocabulary now distinguishes `student_account: AccountId` from `student_record: StudentRecordId` and uses
canonical Student names throughout the direct source graph. Obsolete PostgreSQL course-record columns,
database error literals, and authority names remain isolated at the SQL decoder boundary for the
registered SR5 migrations.

Permanent evidence covers entitlement decisions and visible-assignment pagination, run API
authorization, and roster-first Gradebook totals. Exact retired-identifier searches and boundary
inspection remain one-time evidence. Independent re-review returned `ACCEPT`; strict Clippy for
domain, learning-data-access, and server all targets/features, the focused 10 Rust behavior tests,
and all 3,790 source-style checks pass.

## G2-W1 architecture handoff

The binding is implementation-ready on 2026-08-28. Independent architecture, security, and HCI
rereviews accept the roster-first calculated Gradebook, exact Student/run choice, browser-valid
Fetch Metadata decision table, solution-free response projection, atomic audit boundary, accessible
recovery, and permanent/connected/visual evidence allocation with no remaining P0-P3 finding.
`G2-W2A` and `G2-W2B` proceeded as independent contract slices and are accepted below.

The W1 focused gate is green on the current material tree: the combined ASCII, source-size,
SQL-line, and Markdown run passes 3,954 cases, and the focused naming/Markdown run passes 265 cases.
The renamed Instructor and G2 plans now resolve through the repository inventory, closing the W1
documentation condition without changing the accepted contract.

## G2-W2 accepted evidence

`WP-INST-G2 / G2-W2A` and `G2-W2B` are accepted on 2026-08-28 for their deterministic
model/Memory contracts. G2-W3 now owns the PostgreSQL implementations and four reserved inspection
migrations; this W2 acceptance does not accept G2 as a whole.

- **Calculated Gradebook:** `CourseGradebookStore` is roster-first and includes active Students
  without submissions. Its structural cursor binds the selected scheme, filters, Student order,
  and page-local live-scoring witness; totals select current server-owned scores and preserve the
  exact run-choice evidence.
- **Audited Student work:** `StudentWorkInspectionStore` verifies the exact immutable submission,
  receipt presentation, response digest, composite identity, retention state, and closed
  `IssuedPresentation` or verified ExternalTool `PresentationNotApplicable` evidence before
  returning a solution-free response projection. Successful reads create paired internal audit
  witnesses; concealed unavailable reads create neither success witness.
- **Focused permanent evidence:** six Gradebook Store conformance cases, eight inspection Store
  conformance cases, eleven presentation cases, nine disclosure-policy cases, and thirty browser
  state/presentation cases pass. The previously failing production Instructor-authoring journey
  also passes from a fresh build and observes the same Student submission and refreshed score
  without a second answer submission.
- **Evidence classification:** permanent checks assert observable calculation, disclosure,
  state-transition, and exact fixture-secret boundaries. Generic searches for possible credential
  field spellings are implementation-audit evidence and remain outside the permanent suite.
- **Independent review:** final architecture, logic, security/privacy, and test-ownership rereviews
  accept the W2 boundary. The final feedback rereview confirms that current, recalculating, and
  failed scoring states are visibly and accessibly distinct while the refresh remains GET-only.

## G2-W3/W4 implementation handoff

The current tree implements the PostgreSQL calculated Gradebook and audited inspection Store,
migrations `1870` through `1878`, registered no-store server routes with Fetch Metadata policy,
strict browser decoders, the roster-first Gradebook, exact submitted-run chooser, named immutable
Student-work detail, and focus-preserving Gradebook and grading-operation returns. Focused Store,
route, decoder, navigation, and page-model gates are the permanent behavioral evidence for these
owners.

The focused production-stack Instructor-authoring, ordinary Student-delivery, and deterministic
grader-recovery journeys are green, including real score propagation, both audited inspection
entry paths, reload, and exact return focus. `G2-W5` still owns the aggregate connected
PostgreSQL/RLS/browser matrix and 1280x800 Instructor visual review. `G2-W6` then owns final
material-tree validation. These remaining evidence waves keep `WP-INST-G2` acceptance-open.

The live Instructor course surface now has one route-scope-owned course frame and one stable ribbon
for all eight course-management tasks. A one-time 1280 by 800 browser diagnostic traversed every tab
on the production stack and observed the same frame, title, and navigation origin while each active
tab and task content changed. The diagnostic was removed after inspection; connected journeys and
the canonical screenshot corpus remain the durable acceptance lanes. The current 64-artifact corpus
has been regenerated through the production-stack owner, including the audited Student-work state,
and the fresh 1280 by 800 Instructor surfaces passed visual review for stable ribbon placement. The
same live pass confirmed that Grade settings returns server-calculated totals for Students without
optional external roster metadata, while roster ID and email remain confined to the audited
CSV projection.

### Active-system invariants

- Use the canonical disposable production-shaped HTTPS stack and visible UI-created product state.
- Keep grading deterministic and server-owned; browser contracts remain answer-free.
- Preserve exact course and Student authorization isolation, immutable published content, draft-versus-publication identity,
  immutable evidence, and stateless API replicas.
- Keep one `BlueprintCourse`/`CourseInstance` model, with ADAPT Alpha retained as comparison
  vocabulary only. Pin assignments and evidence to immutable question versions, and use explicit
  forks and controlled updates for change.
- Let a question owner commit moderate immutable versions within one `QuestionId` lineage. Let
  any vetted Instructor turn a full fork into a private draft and publish it as a separately
  authored lineage with source attribution and a compatible CC license.
- Use the `Change Proposal` domain term for the `Suggest an improvement` action. Each proposal
  targets an exact base version and carries validated content plus semantic and grading impact.
  The lineage owner accepts or rejects it; a stale base requires rebase and resubmission. An
  accepted proposal creates an immutable same-lineage version with contributor credit, preserves
  authorship, history, and compatible CC licensing, and leaves exact assignment and evidence
  pins unchanged.
- Route a Sysadmin-approved `ForcedQuestionCorrection` through an immutable replacement mapping,
  bounded remediation, and audited impact evidence.
- Keep the learning engine question-agnostic. Biology examples are fixtures rather than policy.
- Retain direct-entry evidence for the five fixed seeded personas. Elena Instructor and Morgan
  Sysadmin each retain an independent generic passkey journey.

## Shared migration ledger and allocation

The release integrator owns migration ordering and this ledger. The reviewed pre-production v1 reset above is the
explicit clean-cluster baseline decision. After v1 ships, accepted files are immutable; future schema packages receive
an allocation before implementation. Non-schema packages do not receive an implicit allocation.

The owner-confirmed single-installation correction keeps PLE in its pre-production clean-cluster
state. `WP-SD1-C` owns a fresh active epoch in `2026082901` through `2026082936`, split by focused
capability as registered in its plan. Earlier allocations remain package-history evidence until
SD1-C records the exact replacement ledger.

| Allocation                | Package                | Current disposition                                                                                                                                                                                 |
| ------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `2026080801`-`2026080806` | Foundational baseline  | Accepted six-file baseline                                                                                                                                                                          |
| `2026080907`              | `WP-RC1`               | Accepted course appearance                                                                                                                                                                          |
| `2026080908`              | `WP-P2`                | Allocated secure question-grading payloads and the post-G1-W2 legacy-consumer/grant-reduction transition                                                                                            |
| `2026080909`              | `WP-RC8`               | Allocated passwordless identity and enrollment                                                                                                                                                      |
| `2026080910`              | `WP-RC7`               | Reserved Object Storage Checks                                                                                                                                                                      |
| `2026080911`              | `WP-RC9`               | Reserved LTI Advantage                                                                                                                                                                              |
| `2026080912`              | `WP-FU1`-`WP-FU6`      | Reserved secure Student uploads                                                                                                                                                                     |
| `2026080914`-`2026080935` | Release packages       | Existing forward allocations                                                                                                                                                                        |
| `2026081401`              | `WP-R0`                | Existing ranked-catalog allocation                                                                                                                                                                  |
| `2026081501`-`2026081504` | `WP-RC8` repairs       | Existing forward allocations                                                                                                                                                                        |
| `2026081801`              | `WP-INST-S2`           | Accepted term and time zone                                                                                                                                                                         |
| `2026081802`              | `WP-INST-S7`           | Accepted typed references and bylines                                                                                                                                                               |
| `2026081803`              | `WP-INST-S5`           | Accepted entitlement and materialization                                                                                                                                                            |
| `2026081804`              | `WP-INST-S3`           | Accepted effective-policy resolver                                                                                                                                                                  |
| `2026081805`              | `WP-INST-S4`           | Accepted disclosure policy                                                                                                                                                                          |
| `2026081806`              | `WP-INST-S6`           | Accepted course grade scheme                                                                                                                                                                        |
| `2026081807`              | `WP-INST-T2`           | Accepted teaching operations                                                                                                                                                                        |
| `2026081808`              | `WP-INST-LD1`          | Accepted live-demo installation state                                                                                                                                                               |
| `2026081809`              | `WP-INST-LD2`          | Accepted Sysadmin candidate and completed-install brokers                                                                                                                                           |
| `2026081810`              | `WP-INST-LD2`          | Accepted Student authorization-context repair in the pre-SD1 schema                                                                                                                                 |
| `2026081811`              | Reserved               | Reserved numeric identity                                                                                                                                                                           |
| `2026081812`              | `WP-INST-LD3`          | Accepted ordinary assignment mutation authority                                                                                                                                                     |
| `2026081813`              | Reserved               | Reserved numeric identity                                                                                                                                                                           |
| `2026081814`              | `WP-INST-LD3`          | Accepted assignment-definition capability                                                                                                                                                           |
| `2026081815`              | Reserved               | Reserved numeric identity                                                                                                                                                                           |
| `2026081816`              | `WP-INST-LD3`          | Accepted course-group mutation brokers                                                                                                                                                              |
| `2026081817`              | `WP-INST-LD3`          | Accepted Student-work source and execution snapshots                                                                                                                                                |
| `2026081818`              | `WP-INST-LD3`          | Canonical v1 course provisioning and installed-course attestation                                                                                                                                   |
| `2026081819`              | `WP-INST-LD3`          | Accepted grade control and export audit                                                                                                                                                             |
| `2026081820`              | `WP-INST-LD3`          | Accepted scoring preparation and finalization                                                                                                                                                       |
| `2026081821`-`2026081822` | Reserved               | Reserved numeric identities                                                                                                                                                                         |
| `2026081823`              | `WP-INST-LD3`          | Accepted teaching-invitation mutation authority                                                                                                                                                     |
| `2026081824`              | `WP-INST-LD3`          | Accepted roster procedure ambiguity repair                                                                                                                                                          |
| `2026081825`              | `WP-INST-LD3`          | Accepted inactive-Student materialization decision                                                                                                                                                  |
| `2026081826`              | `WP-INST-T5`           | Accepted pre-issue assignment-definition replacement                                                                                                                                                |
| `2026081827`              | `WP-INST-D1`           | Accepted discovery evidence and response-family projection                                                                                                                                          |
| `2026081828`              | `WP-INST-D1`           | Accepted account usage snapshots and Library facets                                                                                                                                                 |
| `2026081829`              | `WP-INST-LD3`          | Reserved Student-work broker capability                                                                                                                                                             |
| `2026081830`              | `WP-INST-G1`           | Reserved assignment recalculation enqueue capability                                                                                                                                                |
| `2026081831`              | `WP-INST-G1`           | Reserved scoring-generation publication                                                                                                                                                             |
| `2026081832`              | `WP-INST-G3`           | Reserved item-analysis publication and cleanup                                                                                                                                                      |
| `2026081833`              | `WP-INST-T5`           | Reserved assignment-definition scratch isolation                                                                                                                                                    |
| `2026081834`              | `WP-INST-LD3`          | Reserved course-group policy broker repair                                                                                                                                                          |
| `2026081835`              | `WP-INST-LD1`          | Reserved catalog-derived Base Course freshness authority                                                                                                                                            |
| `2026081836`              | `WP-INST-D2`           | Accepted question curation capabilities                                                                                                                                                             |
| `2026081837`              | `WP-INST-B1`           | Accepted historical pre-SD1 reusable-course capabilities; SD1 target consolidates them into BlueprintCourse/CourseInstance                                                                          |
| `2026081838`              | `WP-INST-B2`           | Accepted curriculum-adoption schema, lineage, schedule, provenance, receipt, integrity, and forced RLS foundation                                                                                   |
| `2026081839`              | `WP-INST-B2`           | Accepted curriculum-adoption common broker authority, retention integration, and shared capability boundary                                                                                         |
| `2026081840`              | `WP-INST-B2`           | Accepted curriculum-adoption relational snapshots, locked preparation, inspection, and reconciliation helpers                                                                                       |
| `2026081841`              | `WP-INST-B2`           | Accepted canonical ordinary-course topology, issued-work fencing, and topology capability assertions                                                                                                |
| `2026081842`              | `WP-INST-B2`           | Accepted curriculum-adoption source authorization, closed request validation, and source snapshot facts                                                                                             |
| `2026081843`              | `WP-INST-B2`           | Accepted teaching-course, import, inspection, reconciliation, and controlled schedule snapshot facts                                                                                                |
| `2026081844`              | `WP-INST-B2`           | Accepted curriculum-adoption shared materializer validation, idempotency, receipt, and evidence helpers                                                                                             |
| `2026081845`              | `WP-INST-B2`           | Accepted fork, assignment adoption, fast-forward, and reconciliation materializers                                                                                                                  |
| `2026081846`              | `WP-INST-B2`           | Accepted whole-course instantiation, rollover, and term-shift materializers                                                                                                                         |
| `2026081847`              | `WP-INST-B2`           | Accepted canonical public bridge completion and final broker catalog assertions                                                                                                                     |
| `2026081848`              | `WP-INST-T6`           | Allocated assignment-workspace capability migration: empty Draft/Archived definitions and Published readiness                                                                                       |
| `2026081849`              | `WP-INST-G1`           | Accepted W2 operation/evaluation/execution schema prerequisite and receipts                                                                                                                         |
| `2026081850`              | `WP-INST-G1`           | Accepted W2 private accepted-response, acceptance/replay, retention/RLS, and lease-fenced execution boundary                                                                                        |
| `2026081851`              | `WP-INST-G1 / G1-W4`   | Schema and roles; proof: fresh schema/role shape query                                                                                                                                              |
| `2026081852`              | `WP-INST-G1 / G1-W4`   | Integrity guards and triggers; proof: immutable-write rejection                                                                                                                                     |
| `2026081853`              | `WP-INST-G1 / G1-W4`   | Public function authority; proof: effective catalog closes PUBLIC/default EXECUTE and legacy v1 load                                                                                                |
| `2026081854`              | `WP-INST-G1 / G1-W4`   | Witness/RLS/table authority and receipt version SELECT; proof: exact authority and ACL matrix                                                                                                       |
| `2026081855`              | `WP-INST-G1 / G1-W4`   | Split generic/exact claim and ready/max convergence; proof: one winner and sibling denial                                                                                                           |
| `2026081856`              | `WP-INST-G1 / G1-W4`   | Four-key structural verified read; proof: entitled route succeeds and changed key fails                                                                                                             |
| `2026081857`              | `WP-INST-G1 / G1-W4`   | Exact private execution load; proof: exact claim loads once and rejects mismatches                                                                                                                  |
| `2026081858`              | `WP-INST-G1 / G1-W4`   | Completion lock; proof: lock fences stale or duplicate completion                                                                                                                                   |
| `2026081859`              | `WP-INST-G1 / G1-W4`   | Commit-v2; proof: full 36-input signature commits one immutable aggregate                                                                                                                           |
| `2026081860`              | `WP-INST-G1 / G1-W4`   | Fail; proof: NULL-safe closed failure validation preserves invalid-call state                                                                                                                       |
| `2026081861`              | `WP-INST-G1 / G1-W5`   | W5 Instructor grading-operation capability and broker surface                                                                                                                                       |
| `2026081862`              | `WP-INST-G1 / G1-W5`   | W5 worker-authoritative grading-operation lifecycle projection                                                                                                                                      |
| `2026081863`              | `WP-INST-G1 / G1-W5`   | W5 immutable scoring-invalidation origin evidence                                                                                                                                                   |
| `2026081864`              | `WP-INST-G1 / G1-W5`   | W5 canonical generation, job, operation, and supersession capability                                                                                                                                |
| `2026081865`              | `WP-INST-G1 / G1-W5`   | W5 source-specific invalidation witnesses and least-privilege adapters                                                                                                                              |
| `2026081866`              | `WP-INST-G1 / G1-W7`   | `2026081866_g1_receipt_provenance_schema.sql`: clean-volume preflight; execution and operation receipt provenance/category schema and closed constraints                                            |
| `2026081867`              | `WP-INST-G1 / G1-W7`   | `2026081867_g1_execution_receipt_writers.sql`: acceptance, claim, and failure writer bodies; exact ACL/catalog proof                                                                                |
| `2026081868`              | `WP-INST-G1 / G1-W7`   | `2026081868_g1_completion_receipt_writer.sql`: 36-input commit-v2 body and narrow ACL/catalog proof                                                                                                 |
| `2026081869`              | `WP-INST-G1 / G1-W7`   | `2026081869_g1_instructor_receipt_writers.sql`: V2 retry transition, public retry routing/V1 retirement, and final (1865) recalculation body with broker ACL proof                                  |
| `2026081870`              | `WP-INST-G2 / G2-W3B`  | `2026081870_student_work_inspection_authority.sql`: dedicated inspection owner, fixed search path, baseline revocations, narrow grants, and catalog proof                                           |
| `2026081871`              | `WP-INST-G2 / G2-W3B`  | `2026081871_student_work_inspection_witness.sql`: private immutable receipt/presentation/response witness, integrity boundary, and catalog proof                                                    |
| `2026081872`              | `WP-INST-G2 / G2-W3B`  | `2026081872_student_work_inspection_capability.sql`: only app-executable inspection broker, parameter-bound composite resolution, atomic audits, and closed ACL proof                               |
| `2026081873`              | `WP-INST-G2 / G2-W3B`  | `2026081873_student_work_inspection_indexes.sql`: query-demonstrated inspection/audit indexes with retained closed broker authority                                                                 |
| `2026081874`              | `WP-INST-G2 / G2-W3A`  | Historical pre-SD1 queue-failure capability, superseded by the SD1 course-bound worker scope                                                                                                        |
| `2026081875`              | `WP-INST-G2 / G2-W3B`  | `2026081875_student_work_inspection_rowset_contract.sql`: forward repair aligning the broker's transient JSON rowset with exact PostgreSQL field names                                              |
| `2026081876`              | `WP-INST-G2 / G2-W3B`  | `2026081876_student_work_inspection_safe_labels.sql`: server-owned validated Student display label and assignment title returned by the existing audited inspection broker, without entering audits |
| `2026081877`              | `WP-INST-G2 / G2-W5`   | `2026081877_base_course_accepted_submission_completion.sql`: host-only fast-path identity, exact typed queue claim, and accepted-private-response-aware Base Course completion verification         |
| `2026081878`              | `WP-INST-G2 / G2-W5`   | `2026081878_gradebook_operation_selection.sql`: execute-only Instructor broker for exact public grading-operation Gradebook selection without direct application-table access                       |
| `2026081879`              | `WP-INST-WN1 / WN1-D`  | Course-authority broker ownership, narrow RLS policies, explicit ACLs, and forced RLS                                                                                                               |
| `2026081880`              | `WP-INST-WN1 / WN1-D`  | Exact authority-function argument rebinding and dependent recreation with unchanged authorization behavior                                                                                          |
| `2026081881`              | `WP-SD1-C`             | Historical WN1-D Student-role schema vocabulary retained as evidence/input and absorbed by the fresh SD1-C epoch                                                                                    |
| `2026081882`              | `WP-SD1-C`             | Historical WN1-D Student-work broker vocabulary retained as evidence/input and absorbed by the fresh SD1-C epoch                                                                                    |
| `2026081883`              | `WP-INST-WN1 / WN1-MG` | Automated-only scoring constraints and manual-grade persistence retirement                                                                                                                          |
| `2026081884`              | `WP-INST-WN1 / WN1-D`  | Direct Student-work payload contracts for current run, attempt, submission, feedback, and summary records                                                                                           |
| `2026081885`              | `WP-INST-WN1 / WN1-D`  | Canonical receipt payload V2 for new immutable evidence while retaining V1 bytes and readers                                                                                                        |
| `2026081886`              | `WP-INST-WN1 / WN1-D`  | Catalog, workspace, publication, and flat-asset payload contracts                                                                                                                                   |
| `2026081887`              | `WP-INST-WN1 / WN1-D`  | Curriculum-adoption request, inspection, and reconciliation payload contracts                                                                                                                       |
| `2026081888`              | `WP-INST-WN1 / WN1-D`  | Operational worker, retention, delivery, roster/account, provider-cache, and export payload contracts                                                                                               |
| `2026082901`              | `WP-SD1-C`             | Migration-principal baseline, NOLOGIN capability roles, schemas, default ACLs                                                                                                                       |
| `2026082902`              | `WP-SD1-C`             | Accounts and primary sessions, including singular immutable account/session role storage                                                                                                            |
| `2026082903`              | `WP-SD1-C`             | Private verified Authentication Email relation, email challenges, and rate limits                                                                                                                   |
| `2026082904`              | `WP-SD1-C`             | WebAuthn ceremonies and passkeys                                                                                                                                                                    |
| `2026082905`              | `WP-SD1-C`             | Instructor vetting, current approval predicates, and role predicates                                                                                                                                |
| `2026082906`              | `WP-SD1-C`             | Authenticated-session resolver, installer, and session RLS broker                                                                                                                                   |
| `2026082907`              | `WP-SD1-C`             | Question Library roots and immutable versions                                                                                                                                                       |
| `2026082908`              | `WP-SD1-C`             | Question Publication and Question Version Availability evidence                                                                                                                                     |
| `2026082909`              | `WP-SD1-C`             | Question Library lineage, proposals, Stars, Watches, and improvement audit                                                                                                                          |
| `2026082910`              | `WP-SD1-C`             | Workspaces, immutable Workspace Collaborator Events, and private authoring roots                                                                                                                    |
| `2026082911`              | `WP-SD1-C`             | BlueprintCourse tree, immutable revisions, question-version pins, and minimal-Blueprint construction                                                                                                |
| `2026082912`              | `WP-SD1-C`             | Collections, saved searches, and authoring projections                                                                                                                                              |
| `2026082913`              | `WP-SD1-C`             | CourseInstance roots, non-null immutable Blueprint binding, and immutable curriculum-adoption receipt/idempotency/evidence records                                                                  |
| `2026082914`              | `WP-SD1-C`             | Immutable Course Membership and Course Invitation Event ledgers with equal Teaching Team Member authority                                                                                           |
| `2026082915`              | `WP-SD1-C`             | Student enrollment, ownership, and future relationship roots                                                                                                                                        |
| `2026082916`              | `WP-SD1-C`             | Relative schedules, propagation, release, and delivery divergence                                                                                                                                   |
| `2026082917`              | `WP-SD1-C`             | Assignment delivery, Assignment Attempts, and Issued Questions                                                                                                                                      |
| `2026082918`              | `WP-SD1-C`             | Submissions, response artifacts, and Student feedback                                                                                                                                               |
| `2026082919`              | `WP-SD1-C`             | Course artifacts and object metadata scopes                                                                                                                                                         |
| `2026082920`              | `WP-SD1-C`             | Delivery indexes, partitions, and exact read projections                                                                                                                                            |
| `2026082921`              | `WP-SD1-C`             | Automated grading operations and immutable receipts                                                                                                                                                 |
| `2026082922`              | `WP-SD1-C`             | Gradebook and grade-control evidence                                                                                                                                                                |
| `2026082923`              | `WP-SD1-C`             | Item/course analysis and thresholded evidence                                                                                                                                                       |
| `2026082924`              | `WP-SD1-C`             | Correction manifests, recalculation, and improvement links                                                                                                                                          |
| `2026082925`              | `WP-SD1-C`             | Typed jobs, leases, and worker scope derivation                                                                                                                                                     |
| `2026082926`              | `WP-SD1-C`             | Exports, retention, and audit lifecycle                                                                                                                                                             |
| `2026082927`              | `WP-SD1-C`             | External-tool launches, provider cache, and passback state                                                                                                                                          |
| `2026082928`              | `WP-SD1-C`             | Object Delivery, Object Storage Checks, and Object Cleanup authority                                                                                                                                |
| `2026082929`              | `WP-SD1-C`             | Course/catalog/workspace capability brokers, including execute-only curriculum-adoption apply/reconcile brokers                                                                                     |
| `2026082930`              | `WP-SD1-C`             | CourseInstance, Student-record, and worker-lease forced RLS policies                                                                                                                                |
| `2026082931`              | `WP-SD1-C`             | Final table, sequence, type, and function ACL closure                                                                                                                                               |
| `2026082932`              | `WP-SD1-C`             | Schema acceptance helpers and complete-ledger witness                                                                                                                                               |
| `2026082933`              | `WP-SD1-C`             | Atomic email-challenge and validated-passkey completion brokers for existing Accounts                                                                                                               |
| `2026082934`              | `WP-SD1-C`             | Sysadmin-only global Account Creation broker with immutable Product Role                                                                                                                            |
| `2026082935`              | `WP-SD1-C`             | Exact Draft Blueprint Revision collaboration, publication, and availability evidence                                                                                                                |
| `2026082936`              | `WP-SD1-C`             | Identity-free exact Question Version Statistics and idempotent accepted-grade observation evidence                                                                                                  |

This registry is the complete, current number-to-capability ledger for `WP-SD1-C`; historical
`2026081881` and `2026081882` remain immutable evidence/input.

`2026081803` (`S5`), `2026081804` (`S3`), and `2026081805` (`S4`) reflect the accepted
pre-file allocation reorder. Allocations `2026081811`, `1813`, `1815`, `1821`, and `1822` retain
their numeric identities. T6 owns `2026081848`; G1 accepted `2026081849` and `2026081850` in
addition to reserved enqueue/publication capabilities `2026081830` and `2026081831`. G3 retains
`2026081832`. G1-W4 owns ordered forward allocations `2026081851` through `2026081860`: schema/roles,
integrity, public-function authority, table authority, claim, read, load, completion lock, commit,
then fail. G1-W5 owns `2026081861` through `2026081865`: Instructor operations, lifecycle
projection, immutable invalidation origins, the canonical invalidation capability, and
source-specific least-privilege witnesses. The historical accepted migrations are retained as
evidence, while the fresh baseline is authoritative.
byte-for-byte. The four allocated closeout migrations are implemented in order: migration 1866
fails closed when either `grading_execution_receipt` or `grading_operation_receipt` is nonempty
before adding provenance/category fields; it preserves immutable receipt history. Migration 1869
creates the five-input account-bound retry V2 capability, routes the unchanged public retry caller
through it, revokes V1 execute, and drops the four-input V1 with `RESTRICT`. The 99-migration live
database, RLS, worker, browser, WebWork, and replica-restart evidence is green. These rows remain
allocated, and G1-W7 plus `WP-INST-G1` are accepted on the final 99-migration material tree. The
Instructor plan owns dependencies among reserved capabilities.

## Accepted package pointers

| Package                     | Current durable result                                                                                         | Owning evidence                                                                 |
| --------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `WP-INST-LD1`               | Base Course installation lifecycle and retained-state rules                                                    | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-LD2`               | Seeded entry and connected live authoring boundary                                                             | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-LD3`               | Ordinary live assignment, Student work, and immutable evidence path                                            | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-T5`                | Fixed-or-pool assignment editing and deterministic issued draws                                                | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-T6`                | Accepted assignment workspace, focused replacement, and live Student view                                      | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-D1`                | Canonical Library discovery and evidence-backed question detail                                                | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-D2`                | Live curation and shared problem selection                                                                     | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-B1`                | Historical pre-SD1 reusable-course capability; SD1 target uses BlueprintCourse and shared reuse                | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-B2`                | Historical curriculum adoption, rollover, term shifting, and controlled update; SD1 target uses CourseInstance | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-G1`                | Automated-grading exception routing, retry, and recalculation                                                  | [changelog](../CHANGELOG.md)                                                    |
| `WP-INST-G2`                | Calculated Gradebook and audited Student-work inspection                                                       | [changelog](../CHANGELOG.md)                                                    |
| `WP-R0`-`WP-R2`, `WP-PY-L1` | Accepted cross-roadmap capabilities                                                                            | [Release plan](active/release_completion_plan.md), [changelog](../CHANGELOG.md) |

## Dependency-ordered queue

The authoritative package sequence is in the [release completion plan](active/release_completion_plan.md).
The current handoff is:

1. Complete the independent architecture/privacy `ACCEPT` for current `WP-SD1-A`. A1-A4 and the
   A5 pre-acceptance implementation receipts are recorded above; the supplied A5 review remains
   `REVISE`/`BLOCKED`. After A5 acceptance, select `WP-SD1-B1` for account, session, and exact authorization
   contracts.
2. `SD1-B1-P0` is accepted preparatory identity-only work. Complete `SD1-B2` through `SD1-B4` as
   exact-scope contract roots, without claiming route conversion.
3. Implement `WP-SD1-C` as the fresh PostgreSQL epoch in `2026082901` through `2026082936`, then
   implement SD1-D Store/RLS and direct protected-service support with its connected proof. The
   historical `2026081881` and `2026081882` work is evidence/input to C, not an active WN1 queue
   item.
4. Finish `SD1-B1-F` as the integrated singular session/auth cutover: resolve account/session facts
   through one SessionStore; reconstruct email-code, passkey, and seeded-demo entry on that record;
   and prove the server-wide route replacement. The retired `SessionSubject` and `AccountSession*`
   models are already removed. `SD1-B5` then generates the browser-safe account contracts. This
   sequence provides no global-scope fallback or persistent dual model.
5. Continue through SD1-E services/workers/objects/adapters, SD1-F browser/live-demo workflow, and
   SD1-G real-stack and release-plan closure in the release plan's dependency order.
6. Run the complete final-material-tree Validation suite before declaring the goal complete.

## Operational references

- [LIVE_DEMO_SPEC.md](../LIVE_DEMO_SPEC.md) defines the live demo behavior.
- [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) defines required Validation evidence.
- [DEVELOPMENT.md](../DEVELOPMENT.md), [INSTALL.md](../INSTALL.md), [USAGE.md](../USAGE.md), and
  [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) own operational instructions.
- The dated comparison snapshot is
  [project_status_report_2026-08-10.md](reports/project_status_report_2026-08-10.md); older status
  notes and `partial_commit_status.md` are historical references.
