# Changelog

## 2026-08-29

### Additions and New Features

- Implemented `WP-SD1-C/M5` Memory curriculum-adoption dispatcher cutover. The Memory Store now
  exposes only the five current lifecycle methods and directly dispatches the seven closed
  BlueprintCourse/CourseInstance preview and apply variants. Apply and reconciliation each hold one
  writer transition: current Instructor authorization, canonical actor-bound intent/digest,
  actor/key replay-or-conflict, server record-to-command consumption, exact immutable receipt
  validation/storage, and full-State rollback on every post-replay failure. Reconciliation now has
  its own non-Serde intent with a caller-provided retry key, so retries replay one repair while a
  later repair can carry a new identity. Rollover and term-shift cores return post-state facts;
  their receipt is constructed by the outer transaction from the retained apply record. Term-shift
  receipts validate their committed delivery delta, and whole-course instantiation/rollover receipt
  validation proves the exact immutable whole-course row and canonical Blueprint parentage. The
  feature-enabled LDA library check and question-model test suite pass; current public Memory
  behavior tests, PostgreSQL, services, browser, and live acceptance remain downstream work.

- Implemented preparatory `WP-SD1-C` immutable BlueprintCourse revision storage in Memory. The
  reusable-curriculum Store now separates handle-free creation from expected-head replacement;
  keeps a small owner/head record plus append-only complete revision snapshots; allocates opaque
  stable module and assignment identities only in trusted Memory code; and validates every
  retained child handle against the expected head. Exact historical source snapshots resolve by
  immutable revision and stable assignment identity, current-source resolution refuses a removed
  lineage rather than selecting a positional neighbor, and whole-course instantiation reads stable
  assignment locators from the exact source snapshot. Owner-only edits and approved-Instructor
  reads remain distinct. No-op complete-tree replacements preserve the observed revision. Focused
  deterministic Memory behavior tests cover retained reorder/insert identity, historical source
  resolution, approved-Instructor read, no-op replacement, foreign/stale refusal, and removed-node
  refusal; their feature-enabled test build remains blocked by the separately-owned legacy M5
  dispatch/test corpus. No-default LDA and question-model checks pass with the established warning
  baseline. PostgreSQL, browser, M5 dispatch/receipt/rollback, and end-to-end acceptance remain
  downstream work.

- Recorded `WP-SD1-A` fixed-role account clarification in the technical authorities. The binding
  SD1 product contract requires one immutable Student, Instructor, or Sysadmin role per account and
  session; people needing multiple roles use separate accounts; Student/Instructor membership must
  match the account role; and Sysadmin provisioning assigns an approved Instructor account without
  course membership for the Sysadmin. Pre-SD1 plural source remains cutover input. Course help
  remains explicit audited support. `2026082902` retains singular role-storage ownership and
  `2026082905` retains Instructor-vetting/current-approval ownership. Source, migration,
  PostgreSQL/RLS, service, browser, runtime, and human-acceptance evidence remain pending. The
  database authorization and schema-evolution range summaries now match the status-owned exact
  migration ledger.

- Clarified the pending SD1 bootstrap boundary: closed Sysadmin platform provisioning binds an
  exact Blueprint source, approved assigned Instructor, and server-reserved CourseInstance identity,
  then atomically creates the CourseInstance, first Instructor membership, and audit event. Ordinary
  `SysadminSupportCapability` remains exact-course support after bootstrap; it does not provision a
  course or grant the Sysadmin membership.

- Recorded the SD1 curriculum and actor authority repair in the planning authorities. Minimal
  Blueprint construction, immutable CourseInstance adoption evidence, execute-only adoption brokers,
  and CourseInstance forced RLS now have their assigned migration ownership; `WP-SD1-B1-P1` is the
  required resolved-record actor-factory prerequisite for D1. This documentation-only change leaves
  implementation, PostgreSQL, runtime, browser, and human acceptance open.

- Implemented preparatory `WP-SD1-C/M1` private Memory curriculum-adoption state. The
  replacement has eight BP/CourseInstance-only receipt operation kinds, globally scoped
  `(UserId, CurriculumAdoptionIdempotencyKey)` identity, and one retained canonical request intent:
  its exact source, parsed projection, protocol version, SHA-256 digest, and closed operation are
  available to both Memory idempotency and the later PostgreSQL broker without re-serialization.
  Domain-separated receipt-target digests remain distinct server-only reconciliation bindings.
  Immutable answer-free evidence, exact replay/conflict lookup, retained reconciliation targets, and
  derived-projection rebuild support remain in place. Receipt insertion refuses every occupied target
  identity before state changes, and CourseInstance outcomes bind the retained target destination
  course. It removes Alpha and tenant receipt vocabulary from the owned state roots. The focused
  facade repair keeps `request_digest` private, re-exports the selected intent/digest surface and
  reconciliation helper through the crate facade, and adds deterministic source/projection and
  domain-separation tests. Question-model format/check and 20 focused curriculum-adoption tests
  pass. Existing downstream Memory operation/dispatch code remains intentionally unconverted for
  M2-M5, so the feature-enabled LDA compilation baseline is still red; this is not Store,
  PostgreSQL/RLS, service, browser, or release acceptance.

- Implemented `WP-SD1-C/M2b` Memory source-adoption operations for current BlueprintCourse and
  CourseInstance contracts. Fork, one-assignment adoption, and whole-course instantiation now
  re-read the exact Blueprint source and destination witness under one rollback-capable Memory
  transition; validate Published-only destination pins and deterministic replacement choices; and
  retain immutable answer-free M1 completion evidence for replay/conflict handling. Assignment
  receipts bind the exact created assignment and its immutable import evidence, rejecting a
  same-course assignment swap before replay. The new seam consumes BlueprintCourse and
  CourseInstance creation reservations, records bounded assignment imports, and removes the
  retired Alpha source-instantiation helper from the source slice.
  Current Store dispatch, CourseInstance lifecycle, controlled update/reconciliation,
  PostgreSQL/RLS, service, browser, and release acceptance remain downstream M3-M5 work.

- Implemented the `WP-SD1-C/M3` Memory CourseInstance lifecycle seam. Rollover now has a
  dedicated current-contract operation module, Blueprint-backed ordered source locations, exact
  target-term schedule evidence, reserved CourseInstance creation binding, immutable answer-free
  receipt targets, global actor/key replay conflict checks, and one rollback transition. Term
  shift consumes only the server-resolved schedule set, rechecks the exact witness and instructor,
  advances assignment and course schedule revisions together, and refuses issued work. The
  Alpha-era rollover and term-shift bodies are retired from their former modules. The current
  feature-enabled Memory compile remains blocked by unconverted M4/M5 legacy dispatch,
  reconciliation, and update families; no PostgreSQL/RLS, service, browser, or release acceptance
  is claimed.

- Revised the preparatory `WP-SD1-C/M3` acceptance after the foundation review found that
  assignment-import provenance was incorrectly serving as CourseInstance parentage. Every
  CourseInstance now has a canonical immutable `CourseInstanceBlueprintApplication`, including
  a zero-assignment minimal-Blueprint instance. Rollover resolves and inherits that application
  instead of deriving its parent from imports; inspection presents the immutable initial
  Blueprint application separately from independently versioned assignment provenance. Existing
  destination records, commands, and immutable receipt targets retain the resolved application,
  and unbound hand-built course rows refuse lifecycle/adoption preview and apply paths as an
  integrity failure. The M2 source snapshot boundary now relies on the approved-Instructor
  authorization established by its caller, so every vetted Instructor can reuse a visible
  Blueprint while owner-only replacement remains unchanged. This is preparatory M3/M4 work:
  M5 still owns the closed single-writer envelope and its public-path rollback/replay tests.

- Implemented the `WP-SD1-C/M4` locked Memory cores for controlled Blueprint assignment updates,
  selected Blueprint assignment copies, answer-free CourseInstance provenance inspection, and
  receipt-targeted derived-import reconciliation. The cores re-authorize the current Instructor and
  exact CourseInstance witness, bind the immutable Blueprint application, preserve exact
  per-assignment source/import evidence, refuse issued or divergent work, and materialize selected
  schedules only after server resolution. Selected-copy server records now retain the validated
  replacement set needed to reproduce their source meaning at apply. M5 remains responsible for
  the one outer write transition, replay/conflict handling, server-record issuance, immutable
  receipt insertion, rollback, and completion response. Retired M4's duplicate legacy update,
  reconciliation, and shared helper modules. Question-model no-default compilation passes; the
  feature-enabled Memory suite remains a downstream M5 cutover gate.

- Strengthened preparatory `WP-SD1-C/M4` receipt integrity. Assignment-derived receipts now name
  both their consumed precondition and exact post-mutation outcome; retain the exact applied
  assignment/import evidence, semantic digest, and selected-copy replacements; and are built only
  after structural validation of source, lineage, replacement, import-revision, and witness facts.
  Controlled updates explicitly distinguish changed reusable meaning from a newer source revision
  whose delivered meaning is already equivalent. Immutable Memory evidence is a closed operation
  detail enum, so adoption, controlled-update, and selected-copy facts cannot be partially mixed.
  Reconciliation resolves one receipt-derived assignment/import locator and leaves a newer current
  projection intact. Receipt replay/reconciliation validation now resolves canonical CourseInstance
  and assignment records under the explicit tenant, checks the exact immutable evidence-map key,
  application, outer outcome, original completed receipt, and operation-specific import history.
  Repair actions retain a narrowed original locator while using an independent actor/key/digest;
  their receipts remain non-targetable. Question-model format, 169 deterministic unit tests, and
  strict Clippy pass;
  no-default learning-data-access compilation passes with the established warning baseline. M5
  remains the owner of its closed write transaction, receipt construction/insertion, and public
  behavioral acceptance; the legacy feature-enabled dispatch/test corpus remains its downstream
  cutover work.

- Repaired preparatory `WP-SD1-C/M2-M3` transaction ownership for current BlueprintCourse
  source adoption and CourseInstance lifecycle mutations. Fork, assignment adoption, whole-course
  instantiation, rollover, and term shift now expose synchronous lock-held domain cores that
  revalidate their consumed server-derived command against current state and return exact outcome
  plus immutable evidence material. The forthcoming M5 dispatcher remains the sole owner of
  session authorization, canonical-intent/digest validation, replay/conflict resolution, receipt
  persistence, completion projection, and full-state rollback. Current stable Blueprint child-ID
  history work owns the remaining replacement of transitional source-location construction;
  source adoption deliberately has no positional fallback. The focused question-model no-default
  compile passes. Feature-disabled learning-data-access compilation is presently blocked by the
  in-progress shared qmodel contract rename, so this receipt does not claim M5, Store, PostgreSQL,
  service, browser, or release acceptance.

- Implemented the preparatory `WP-SD1-B2` CurriculumAdoptionStore lifecycle contract.
  One closed, direct-`snake_case` operation envelope now covers exactly fork,
  existing-instance assignment adoption, BlueprintCourse instantiation, rollover,
  term shift, controlled update, and selected copy. Browser apply carries only its
  request and idempotency key; Store implementations own atomic record
  issuance/consumption and immutable receipt persistence. Reconciliation accepts the
  non-Serde receipt target. Focused question-model format/check/test/strict-Clippy
  gates pass, and the no-default `learning-data-access` compile passes with its
  existing 141-warning baseline. Memory/PostgreSQL implementation, service routes,
  browser flows, connected acceptance, and release completion remain downstream
  SD1-C/D work.

- Accepted preparatory `WP-SD1-B3-B6` as a child execution package of `WP-SD1-B3`, without a new
  top-level roadmap package or migration allocation. `ProblemVersionRef` remains the durable selected
  result; one Published-only ordinary-new-selection predicate admits new references, while Deprecated
  and Archived exact pins remain authorized history and re-resolve at their server/Memory destination.
  The repair requires retained pins to keep an existing authorized visible publication; no selection
  aggregate or browser-trusted exact version exists. Formatting and manager gates pass: question-model
  9+3+2, curation 4, curriculum 8, policy 2, reusable curriculum 2, and server 10. B3 remains
  incomplete pending SD1-C/D persistence/RLS/services and browser/live/aggregate closure.

- Accepted preparatory `WP-SD1-B3-B7-improvement-event-contract` as a child execution package of
  `WP-SD1-B3`, with no migration allocation. The immutable server-only, non-Serde
  `QuestionImprovementEvent` retains opaque event identity and exact proposal/base ancestry;
  accepted events retain a same-lineage advancing successor, while resubmissions retain both their
  new proposal/base and distinct predecessor proposal/base link. Contributor credit remains owned
  exclusively by `QuestionChangeProposal`. The focused default `question_stewardship` selector
  passes, while persistence, authorization, transport, browser, SD1-C/D, and release completion
  remain downstream work; `WP-SD1-B3` remains incomplete.

- Accepted preparatory `WP-SD1-B4-J1`: one server-only, non-Serde `JobTargetSelector`
  exhaustively projects the ten current `JobPayload` families into bounded target and generation
  evidence. It is non-authorizing and retains the existing single queue/broker boundary. The jobs
  facade and selector module are below source limits; seven focused tests, formatting, the default
  warning baseline, source-size, and independent `ACCEPT` are green. `WP-SD1-B4` remains
  incomplete while SD1-C/D resolve selectors into locked exact-scope manifests and retire
  tenant-shaped queue authority.

- Accepted preparatory `WP-SD1-B2-A` and `WP-SD1-B3-A` contract roots after independent final
  `ACCEPT` rechecks. B2-A provides pure active-approval, exact current-course Instructor, and
  exact Student membership-episode authorization plus a non-authorizing course-creation intent.
  B3-A provides a server-only Change Proposal lifecycle with checked semantic/grading-impact
  classification, exact-head and minted-successor witnesses, public contributor credit, stale
  rebase/resubmission, and no browser aggregate. Focused format, crate check, strict domain
  Clippy, and contract-state-machine tests pass. `WP-SD1-B2` and `WP-SD1-B3` remain incomplete
  pending their remaining roots and SD1-C/D Store, PostgreSQL/RLS, runtime-service, and browser
  implementation; this receipt does not claim runtime, PostgreSQL, or browser completion.

- Accepted preparatory `WP-SD1-B3-B1` and `WP-SD1-B3-B2` after independent `ACCEPT` rechecks.
  B3-B1 is the server-only, non-Serde, non-authorizing `QuestionStar` relation intent with only
  global `UserId` ownership and lineage `QuestionId`, exported through the crate root. B3-B2 is
  the private-owner, server-only, non-Serde, non-authorizing `QuestionWatch` aggregate with only
  a published-lineage or exact `ProblemVersionRef` target and exactly four notice kinds:
  `Version`, `Fork`, `ImprovementThread`, and `Impact`. Focused format/check, the existing
  141-warning baseline, direct source-size counts, and independent `ACCEPT` are green. `WP-SD1-B3`
  remains incomplete pending collections, saved searches, sharing, selection, SD1-C/D
  persistence/services, and B5/browser work; no runtime, PostgreSQL/RLS, or browser completion is
  claimed.

- Accepted preparatory `WP-SD1-B3-B3` after the report 40 identity-opacity correction and report 43
  final `ACCEPT` recheck, with reports 34 and 38 supplying architecture and implementation evidence.
  `NamedQuestionCollection` now has a new opaque server identity, global `UserId` ownership,
  canonical validated title, storage-safe strong revision/CAS behavior, and bounded ordered unique
  exact `ProblemVersionRef` pins. Its private child module and selected crate-root API provide no
  browser, tenant, institution, sharing, route, Serde, or authorization path. Eight focused
  deterministic behavioral tests pass. `WP-SD1-B3` remains incomplete pending saved searches,
  collection sharing, selection, SD1-C/D Store/PostgreSQL/RLS/service work, B5, and browser/live
  work; no runtime, persistence, or browser acceptance is claimed.

- Accepted preparatory `WP-SD1-B3-B5` collection sharing after report 46's `REVISE` and report
  47's final `ACCEPT`, using report 42's architecture and report 45's implementation evidence.
  `NamedQuestionCollectionShare` is a server-only, non-Serde, non-authorizing, recipient-specific
  relation over an exact existing `NamedQuestionCollectionId`, immutable owner and distinct
  recipient `UserId`s, and exactly `Active`/`Revoked` state. Self-sharing is refused, and
  grant/reactivation and revoke expose explicit changed/unchanged outcomes. The relation has no
  visibility, access-level, collaborator/editor, publication, tenant, institution, session, role,
  Student, browser, approval, authorization, persistence, or audit field; it does not itself
  grant access. The corrected full-target gate,
  `cargo test -p learning-data-access --features test-support question_curation::collection_share`,
  passes all five matching unit tests and compiles package integration targets with zero matching
  tests. Report 45's `--lib` selector is narrowed evidence only. Focused format/check, the
  existing 141-warning baseline, direct source-size counts (209, 22, and 349 lines), and
  independent acceptance cover only this value contract. SD1-C/D still own authoritative-time
  approval, owner authorization, persistence, transactional uniqueness/owner consistency,
  RLS/broker behavior, concealment, and revoked-read denial; SD1-B5/F owns browser projections
  and workflows. `WP-SD1-B3` remains incomplete pending saved searches, selection, downstream
  Store/PostgreSQL/RLS/service work, B5/F browser work, and live/release completion; no runtime,
  persistence, or browser acceptance is claimed.

- Accepted preparatory `WP-SD1-B3-B4` saved-search value contract after independent `ACCEPT` in
  reports 56, 57, and 59. The server-only `NamedQuestionSavedSearch` retains one immutable global
  `UserId` owner, one opaque server-only UUID identity, one validated title, one normalized no-scope
  `CatalogSearchFilter` (`text`, `bylines`, `backends`, `tags`, `response_families`, `taxonomy`,
  `capabilities`, `licenses`, `evidence`, `used_in_my_courses`, and `authorship`), and one positive
  storage-safe revision. It has no tenant, course, saved-owner identity, cursor, page size, route,
  DTO, browser, or Serde boundary; reruns execute a fresh current-catalog query for the rerunning
  actor. Revision CAS rejects stale expected revisions with expected/actual evidence before candidate
  work, treats normalization-equivalent state as unchanged, increments changed state once, and
  refuses checked exhaustion without mutation. Eight deterministic full-target behavior tests pass.
  C/D still own Store/PostgreSQL persistence, owner/reference mapping, canonical bytes/digest/schema,
  uniqueness/cap/concurrency, authorization/concealment, broker/RLS, and protected service behavior;
  B5/F/G still own browser projections, routes, live-browser, and visual acceptance. `WP-SD1-B3`
  remains incomplete pending selection and downstream completion; no runtime, persistence,
  authorization, RLS, or browser acceptance is claimed.

- Independently reviewed `ACCEPT-PREPARATORY` for
  `WP-SD1-B3-CATALOG-SCOPE-QUERY-RETIREMENT` under architecture report 41, implementation reports
  49-54, and review report 55. One no-scope, direct `snake_case` catalog and saved-search meaning
  now converges across Rust query roots, Memory/PostgreSQL query code, server parsing, regenerated
  TypeScript, and browser clients/models/tests. Passing focused gates are `cargo fmt --all --check`,
  focused `question_model` catalog-facet tests (3/3), Memory catalog search (13/13 plus the
  shared-corpus test 1/1), the PostgreSQL cursor-fingerprint test (1/1), server catalog-query
  (2/2), server catalog HTTP (4/4), saved-search HTTP (7/7), `cargo tools tsgen` (482 declarations),
  both repository TypeScript configurations, the six-file catalog/curation/picker Node lane
  (33/33), and the source-line-limit check (1,856/1,856). Full package acceptance remains
  incomplete pending the fresh SD1-C schema/broker rewrite and connected live PostgreSQL oracle,
  followed by final material-tree gates. Record-level `PublicationScope` remains a separately
  deferred publication/asset security boundary; no persistence, production-browser, or full-package
  acceptance is claimed.

### Fixes and Maintenance

- Strengthened the preparatory `BlueprintCourse`/`CourseInstance` adoption value contract.
  CourseInstance receipt bindings now retain the authorized actor supplied by the consumed
  server-held record (or rollover creation witness), alongside the existing operation,
  destination, idempotency, digest, and time evidence. Course-instance witnesses and reusable
  rollover manifests now use private checked bounded collections for both browser decoding and
  direct Rust construction. Five strict answer-free CourseInstance completion DTOs and a
  receipt-targeted non-Serde reconciliation projection provide the exact Store-facing result
  shapes without serializing immutable receipt evidence. Focused actor, bounds, closed-decoding,
  answer-free, and reconciliation behavior tests pass, as do question-model format/check/test/
  strict-Clippy and the repository codebase gate. Store, PostgreSQL, service, browser, and
  real-stack acceptance remain downstream SD1-C/D work.

- Restored the SD1 catalog lifecycle contract: an authorized Instructor can
  discover and resolve Published, Deprecated, and Archived publications with
  lifecycle labels. Only Published publications remain eligible for ordinary
  new selection. The catalog lifecycle behavior test covers the valid
  Published -> Deprecated -> Archived transition and a separately Deprecated
  publication, requiring listed lifecycle labels and stable-ID detail for both.
  Memory and PostgreSQL search/list resolution share the same three-state rule.

- Recorded two bounded validation-maintenance receipts. The durable Cargo integration-target feature
  boundary requires `test-support` only for `conformance` and `course_creation_memory`, preserving
  an empty default production feature set; the default B7 selector and both feature-enabled target
  compile gates pass. The B3-B6 Memory conformance fixture creates retained references while
  Published, then deprecates the exact retained visible pin before update, preserving Published-only
  ordinary-new-selection and exact-pin history; its focused conformance gate passes. The separate
  feature-enabled full conformance lifecycle failure remains with `WP-SD1-B3-B6`.

- Accepted preparatory `WP-SD1-B1-P0` with the server-only `SessionId` and
  `ActorContext { user_id, session_id }` root in `learning-data-access`. The durable session-record
  ID remains separate from the hashed browser credential, while `ActorContext` carries no course,
  workspace, Student, role, or capability grant. It is presently unconstructible until
  `SessionRecord` owns `SessionId` and exposes the resolved-record factory in `SD1-B1-F`.
  Focused format, crate-check, and session-contract tests pass; independent recheck accepts this
  preparatory boundary only, and `WP-SD1-B1` remains incomplete pending exact-scope consumer
  conversion and singular session-model convergence.

- Clarified the settled Published Question stewardship decision: owner moderate edits,
  publication-validated exact-base Change Proposals, Instructor full forks, and audited Sysadmin
  forced corrections are distinct paths. The UI says **Suggest an improvement**, while
  Change Proposal remains the domain term and `QuestionChangeProposal` the code type. Authorship,
  contribution, licensing, history, and exact immutable assignment and grading pins remain explicit;
  ordinary later revisions never rewrite those pins automatically. The detailed four-path model
  remains in Design Decisions while Human Guidance retains the owner's higher-level direction and
  open correction question.

- Split the `WP-INST-G1 / G1-W4` accepted-submission contract into the semantic
  [automated_grading_execution_contract.md](active_plans/active/automated_grading_execution_contract.md)
  and its PostgreSQL companion
  [automated_grading_execution_database_contract.md](active_plans/active/automated_grading_execution_database_contract.md).
  Graphify at commit `dc227871d18d` and direct source inspection assigned the
  ten W4 migrations, roles/functions, RLS/ACLs, transaction-held recovery, and
  connected database oracle to the companion while retaining immutable
  execution, evidence, state, handler, route, and learner-status semantics in
  the main contract. Both documents are below 1,000 lines and pass focused
  whitespace, ASCII, and reciprocal-link checks; no runtime acceptance is
  claimed by this documentation-only split.

- Independent review `ACCEPT`ed the SD1 authorization-plan authority split:
  `implementation_status.md` is now the sole complete 32-allocation `WP-SD1-C`
  registry; `single_installation_authorization_plan.md` owns product/privacy and
  concise C/D handoffs; and the new
  `single_installation_database_authorization_plan.md` owns principals, ACLs,
  actor installation, forced RLS, Store parity, staging/promotion, and connected
  acceptance. `release_completion_plan.md` remains release authority. The
  main/companion/status documents are 786/142/825 lines; source-size and
  ASCII/whitespace gates pass, the B6 digest is unchanged, and independent review
  is `ACCEPT`ed. The Markdown link gate remains open only for tracked-target
  recognition of new/concurrent untracked docs; no authored relative path is
  missing. This receipt advances documentation authority only and keeps SD1-A5
  and SD1-C/D implementation/acceptance open.

- Implemented `WP-SD1-A-decisions-and-impact-contract` through its A1-A5 pre-acceptance
  documentation and impact-bookkeeping slices. The owner and authority documents now describe one
  installation with global accounts, equal approved Instructors and co-Instructors, shared
  published-question discovery with lifecycle labels, exact course/Student ownership, the approved
  Instructor predicate for course creation, and the fresh SD1-C migration epoch. Migrations
  `2026081881` and `2026081882` are retained as historical WN1-D evidence/input absorbed by that
  fresh epoch. Focused guidance-format checks (2 passed), ASCII checks (1,823 passed), and
  whitespace checks are green. The tracked-file Markdown-link inventory remains open because the
  new SD1 authority targets are untracked. SD1-A is implemented but acceptance-open pending
  independent architecture/privacy `ACCEPT`; no runtime, PostgreSQL/RLS, browser, or full-suite
  acceptance is claimed.

- Accepted `WP-INST-WN1-SR4A-student-authority-source`. Rust entitlement, materialization,
  assignment visibility, Memory identity, feedback authorization, and Gradebook calculation now
  use canonical Student vocabulary while preserving the `student_user: UserId` and
  `student: StudentId` distinction. PostgreSQL-owned legacy spellings are isolated for SR5.
  Independent re-review accepts the corrected boundary; strict all-target/all-feature Clippy,
  focused entitlement/run/Gradebook behavior, and all 3,790 source-style checks pass.

- Accepted `WP-INST-WN1-SR4-browser-direct-clients`. Browser contracts, strict decoders,
  presentation components, progress and response helpers, recovery helpers, and direct consumers
  now use canonical Student vocabulary without aliases. The ordinary assignment endpoint is
  `/api/assignments/{assignment}/student` end to end. The landing-summary decoder follows its
  distinct `StudentAssignmentLandingSummary` type instead of colliding with the activity summary
  decoder. Independent review accepts the boundary; focused Rust route behavior, the server
  all-target/all-feature check, and the complete five-part codebase gate pass with 387 Node tests.
  The whole-tree follow-up allocated previously omitted Student authority identifiers to SR4A.

- Accepted `WP-INST-WN1-SR3-student-run-store-capability`. Run and Store capabilities, Memory and
  PostgreSQL modules, routing bindings, submission-status projections, assignment behavior, and
  external-tool handoff now use canonical Student vocabulary without aliases. The generated
  run-screen contracts and the complete Gradebook row use Serde-owned `snake_case`, including
  `student_name`. Existing run issuance, authorization, prefetch, replay, answer-free recovery,
  assignment, and provider behavior remain the permanent evidence. Two independent reviews accept
  the boundary, and the full Rust and codebase gates pass, including 387 Node tests.

- Accepted `WP-INST-WN1-SR2-student-assignment-projection`. Assignment landing, progress,
  delivery, detail, late-status, score-state, private snapshot, and inactive-course identities now
  use canonical Student vocabulary. Their Serde-owned browser contracts and generated TypeScript
  use direct `snake_case`; strict decoders and UI adapters preserve score withholding,
  class-statistics disclosure, answer-free detail, and Instructor Student view. The ledger now
  states the separate `QM-ACTIVITY` ownership of the retained internal
  `StudentAssignmentSummary` aggregate. Independent review accepts the clarified boundary, and the
  full Rust and codebase gates pass, including 387 Node tests.

- Accepted `WP-INST-WN1-SR1-disclosure-statistics`. Disclosure and Student class-statistics types,
  Store inputs, PostgreSQL modules, generated TypeScript, reusable-curriculum defaults, and strict
  browser decoders now use one canonical Student vocabulary and direct Serde-owned `snake_case`
  contract. Existing timing, stale-score redaction, k-anonymity, and answer-free projection tests
  remain the permanent evidence; the full Rust and codebase gates pass, including 387 Node tests.

- Accepted `WP-INST-WN1-OPS10-e2e-orchestrators`. Private shell state now follows the naming
  policy, and the non-browser aggregate includes all eight maintained lanes. Full execution also
  hardened generated MinIO credentials against CLI parsing and made the multi-database live-demo
  lifecycle migrate every schema before issuing cluster-wide service-role memberships. The final
  aggregate reports 8 passed and 0 failed with exact disposable cleanup.

- Accepted `WP-INST-WN1-OPS9-e2e-database-baseline`. Private shell state now follows the naming
  policy while explicit immutable fixture constants retain uppercase spelling. The fixed leased
  PostgreSQL owner passed all 109 migrations, idempotency and verification, registered live
  service and RLS oracles, and exact cleanup of its container, volume, and network.

- Accepted `WP-INST-WN1-OPS8-e2e-course-appearance`. Private shell state now follows the naming
  policy, and the course-appearance service oracle runs as a closed profile under the fixed leased
  acceptance owner. Typed mode-0600 runtime files replace ambient object-store credentials; exact
  Compose authority starts PostgreSQL and MinIO, and the real cross-store cleanup gate passes with
  empty final state. The source-size gate also drove focused live-test and item-analysis reducer
  module splits instead of exemptions.

- Accepted `WP-INST-WN1-OPS7-wasm-runner-setup`. The version-matched Wasm test-runner setup uses
  lowercase `snake_case` for private state and derives the repository from its physical script
  path. Shell syntax, a fresh pinned installation, and the subsequent matched-runner reuse path
  pass.

- Accepted `WP-INST-WN1-OPS6-python-setup`. The Python setup script uses lowercase `snake_case`
  for its private root, environment, interpreter, and receipt values, and derives the repository
  from its own physical path instead of repository metadata. The current receipt reuse and PyYAML
  verification path passes.

- Accepted `WP-INST-WN1-OPS5-wasm-build`. The Wasm build uses lowercase `snake_case` for its four
  private path/profile values while preserving argument and output behavior. The debug target
  built both bindgen flavors, and the Node consumer verified format, timer, capability, and
  presentation results.

- Accepted `WP-INST-WN1-OPS4-rust-front-door`. The ordinary Rust gate uses lowercase
  `snake_case` for its private repository path while retaining all eleven stages, argument
  handling, and the visible help contract. Shell syntax and help pass.

- Accepted `WP-INST-WN1-OPS3-browser-front-doors`. The screenshot and Playwright root scripts use
  lowercase `snake_case` for their private repository path while retaining the shared
  production-browser owner, argument forwarding, and visible help contracts. Shell syntax and
  both help paths pass.

- Accepted `WP-INST-WN1-OPS2-root-aggregate`. The root Validation front door now uses lowercase
  `snake_case` for its sole script-private path while retaining its exported process boundary and
  complete gate order. Shell syntax and focused source inspection pass; the aggregate execution
  remains owned by final WN1 acceptance.

- Accepted `WP-INST-WN1-GO1-orphaned-generated-output-retirement`. The two unconsumed `ts-rs`
  bindings are removed, leaving project-tools and `generated/api` as the single browser-contract
  generator. Graphify plus direct consumer inspection found no live dependency; regeneration
  produced 482 declarations, all 63 generator tests pass, both TypeScript configurations compile,
  and strict project-tools Clippy is green.

- Accepted `WP-INST-WN1-MG1D-automated-scoring-persistence-retirement` and the parent automated-only
  grading closure after six independent review passes. The runtime now has one deterministic
  evaluation owner with bounded retry/recalculation, immutable evidence, calculated Gradebook
  totals, and roster score export. Migration `2026081883` closes the parallel manual receipt,
  binder, policy, table, and catalog values while exact catalog rewrites preserve mature function
  identity and authority. Focused Rust, TypeScript, SQL-source, contactless-Student export, and
  fresh 109-migration PostgreSQL/RLS gates pass; retirement inventories remain one-time evidence.

- Accepted `WP-INST-WN1-MG1C-automated-item-analysis-state` after independent review and the full
  registered disposable database baseline. Memory and PostgreSQL now share one closed automated
  evaluation truth table: pending and exception work is visibly unscored, completed grades require
  immutable completion-receipt evidence plus current-generation scores, and contradictions fail
  closed. The Instructor report remains aggregate-only, same-tenant Students are denied, and the
  clean stack passed all 108 tracked migrations, RLS/privacy checks, generation fencing, and exact
  cleanup without widening access to worker-private result material.

- Accepted `WP-INST-WN1-MG1B3-evaluation-status-contracts` after independent review and fresh
  manager gates. The automated evaluation contract now has exactly four direct `snake_case`
  values, generated TypeScript matches Serde, and the answer-free status aggregate rejects
  contradictory durable state. Architecture review split the next automated-only boundary into
  truthful item-analysis state followed by persistence retirement and migration `2026081883`.

- Accepted `WP-INST-WN1-MG1B2-attempt-status` after an independent `ACCEPT` and fresh manager
  gates. Attempt lifecycle now has five direct `snake_case` values; Instructor force-submit
  atomically closes active work as answer-free `AutoSubmitted` in Memory and PostgreSQL, preserves
  exact replay, timing cleanup, and audit evidence, and creates no response or grade. The separate
  transitional manual-evaluation bridge remains allocated to its successors. Rotated complete
  older changelog day blocks under the repository's documented 800-line policy.

## 2026-08-28

### Fixes and Maintenance

- Accepted `WP-INST-WN1-A` after two `REVISE` rounds and a fresh `ACCEPT`. Its ledger binds
  automated-only grading, item analysis, exact Student roles, naming, C6 routing, authority, and
  migrations `2026081879` through `2026081888`. B1 adds the pure Serde-only browser contract crate;
  B2 splits the generator into focused owners; B3 adds strict Serde names and safe TypeScript;
  B4/B5 secure output and roots; OPS1 aligns private shell variables. A follow-up allocates orphaned
  `ts-rs` output and remaining PLE shell families. MG1A retires manual HTTP; MG1B1 removes manual outcome, key, and
  disposition variants and gives graded file uploads a typed deterministic-grader refusal. Six
  inventory-only Markdown failures remain open; final gates remain behavioral checks, TypeScript
  compilation, and final validation.

- Moved the Instructor course ribbon into one authorized course-route frame. Course identity and
  all eight course-management tabs now retain one desktop position while Assignments, authoring,
  roster, teaching operations, curriculum changes, Gradebook, grade settings, appearance, and
  their nested task pages replace only the content below the ribbon.

- Regenerated and visually reviewed the 64-artifact production-stack screenshot corpus. The
  automated-grading recovery journey now includes audited Student work as its third declared
  Instructor state, and the fresh 1280 by 800 evidence confirms the shared course title and ribbon
  remain spatially stable across course-management workspaces.

- Separated browser course-total rows from export-only roster identity. Server-calculated totals
  now remain available for ordinary connected Students whose optional institutional roster ID and
  email were never collected; the browser receives only display labels and outcomes, while the
  audited CSV represents absent optional roster fields as blank cells.

- Repaired answer-free audited Student-work inspection for identifier-bearing responses. Accepted
  browser responses are now validated directly against the reconstructed public presentation and
  retain the exact rendered identifiers the Student saw; the inspection boundary no longer tries
  to recover durable identifiers from an intentionally durable-ID-free public snapshot.

- Added an execute-only Instructor broker for resolving grading-operation Gradebook navigation.
  The PostgreSQL adapter no longer depends on direct application-table access that the existing
  least-authority migration had correctly revoked, and submission operations resolve through the
  enrollment's exact course-membership binding.

- Completed synchronous Base Course score convergence through the ordinary worker contract. The
  accepted-submission outcome carries its exact recalculation job, PostgreSQL can claim that exact
  typed job through the queue broker, and host-only installation executes the normal scoring
  handler before checking the installed completion witness.

- Bound the installed Blueprint Course's deterministic completed answer to its issued presentation
  before sending it through the ordinary accepted-submission service. The production seed now uses
  the same rendered response contract as a live Student browser and still persists the canonical
  durable response after server-side translation.

- Made the learning-data-access `test-support` feature self-contained by enabling the
  server-owned Question identifier generation its focused test builders require. The crate's
  isolated all-target gate now exercises the same explicit test capability as the workspace gate.

- Preserved durable submission acceptance when the optional grading fast path is unavailable.
  The browser now receives the stable pending state while the queued worker retains recovery
  ownership, instead of receiving a post-commit service failure.

- Aligned the retained calculated-Gradebook route coverage with its real same-origin browser
  contract, so Fetch Metadata concealment remains meaningful instead of making cookie-only test
  requests look like Gradebook failures. Student access now asserts the route's established
  non-enumerating response alongside Sysadmin and outsider access.

- Hardened G2-W4B Gradebook async sessions: retries retire obsolete chooser continuation gates,
  continuation pages reject visible or intra-page duplicate identities atomically, and focused
  deferred tests cover stale completion fencing, retained-row recovery, liveness, and disposal.

- Initial Gradebook, operation-selection, and submitted-run chooser pages now reject duplicate
  public identities before ready publication and route invalid responses through their existing
  visible error/retry states.

- Added the current validated Student display label and assignment title to the one audited
  Student-work detail response. The registered route continues to return the same immutable,
  solution-free evidence and closed return context with no-store delivery.

- Refactored the registered Gradebook route completion fixture behind a small borrowed harness
  and explicit completion identity, removing the need for a Clippy suppression while preserving
  route behavior and focused assertions.

- Implemented the G2-W4B Instructor Gradebook selection surface. A closed one-filter route now
  survives Gradebook continuation and reload; grading-operation context gives an Instructor a
  bounded named-Student choice; and a semantic submitted-run chooser requires one exact immutable
  run before inspected Student work opens. The chooser restores focus to its invoking Gradebook
  control, retains the optional public operation reference, and keeps the server's total-first
  Gradebook table and failure-routing action visible.

- Completed the G2-W4B audited Student-work return flow. One inspected-work request now carries
  the verified Gradebook or grading-operation return context; operation rows visibly enter their
  bounded Gradebook selection and regain focus on return. The same audited response supplies
  bounded server-owned Student and assignment labels after direct navigation or reload.

- Added the pure G2 Gradebook navigation owner for closed filters, public focus IDs, and
  context-preserving Gradebook, grading-operation, and audited Student-work URLs.

- Closed the G2-W4B calculated-Gradebook browser boundary. Strict decoders and same-origin,
  no-store clients now cover grading-operation filters, named-Student selection, submitted-run
  choices, and audited Student-work inspection with exact Gradebook or grading-operation return
  identity and focus binding. Malformed references and cursors, extra fields, cache drift, and
  echoed request-identity drift fail closed.

- Added permanent offline client evidence for nested calculated-page decoding, closed extra-field
  rejection, and canonical assignmentRef and membershipRef cursor/page-size URLs.

- Closed the G2-W4A registered Gradebook route boundary with offline Memory-backed HTTP coverage
  for closed selection projection, Fetch Metadata concealment before inspection audits, and exact
  operation-origin return/focus context.

- Retired the obsolete ignored PostgreSQL Base Course installation oracle frozen at migration
  `2026081808` and removed its now-unused `base_course_installation` test dependency. Current
  full-schema database authority and canonical live-demo lifecycle acceptance remain the owners.

- Repaired the calculated Gradebook route test to assert the parser's stable
  `GradebookFilterRequest` contract before server normalization.

- Split accepted-submission service, replay, and fast-path projection tests into a focused module;
  route submission coverage remains in the route-owned test module and both stay below the source
  line limit.

- Split G2-W4A Gradebook route tests into focused selection and inspection modules. Shared route
  execution helpers, deterministic backend support, and Fetch Metadata helpers now have one support
  owner; legacy Gradebook tests remain in their original module with behavior unchanged.

- Split the Memory and PostgreSQL Gradebook selection and submitted-run chooser responsibilities
  into focused backend modules; parent stores remain small trait coordinators and all authored Rust
  files stay below the repository's 999-line source limit.

- Added the G2 Gradebook server selection surface. Calculated pages normalize public grading
  operation filters before Store reads, while direct-Instructor, Fetch-Metadata-gated `no-store`
  routes provide bounded answer-free named-Student and submitted-run choices with concealed
  unavailable responses.

- Implemented the G2 Gradebook Store selection and submitted-run chooser in both Memory and
  PostgreSQL. Instructor-authorized selection is active-roster ordered, operation-bound, and
  cursor-bounded; submitted runs retain a stable completion order, mark the current score-selected
  run, and conceal stale, foreign, unavailable, or mismatched continuation state.

- Completed the Base Course canonical accepted-submission seed composition: its child-only lifecycle
  now carries distinct installer, application, and exact-fast-path PostgreSQL capabilities; the
  deterministic Mary submission uses the shared server acceptance/execution boundary, and focused
  route coverage proves durable first effect, idempotent answer-free replay, and one exact execution.

- Completed the PostgreSQL half of the G2 calculated-Gradebook and audited Student-work boundary.
  Worker failure now preserves tenant context through the queue capability, the connected fixture
  creates immutable accepted work through the production submission and scoring path, and a forward
  migration aligns the inspection broker's transient JSON rowset with exact PostgreSQL field names.
  The disposable 105-migration database baseline passed typed inspection, paired audit writes,
  broker-only private-response access, RLS, and representative role denial. The raw broker probe
  used to localize the rowset mismatch was removed after diagnosis.

- Completed the deterministic G2-W2 calculated-Gradebook and audited Student-work contracts.
  Gradebook pages are roster-first, structurally continued, and calculated from current
  server-owned scores; inspected work is immutable-evidence-bound, solution-free, retention-aware,
  and paired with internal audit witnesses. Large conformance modules were split by ownership while
  retaining one shared contract suite for Memory and future PostgreSQL implementations.

- Distinguished feedback release from score freshness throughout the Student run and history
  surfaces. Current, recalculating, and failed scores now have truthful visible and live-region
  messages, completed recalculation refreshes through the status read without resubmitting the
  answer, and the fresh production Instructor-authoring journey observes the resulting score.

- Replaced an implementation-spelling blacklist in the iMathAS launch test with durable shell
  capability assertions and exact non-disclosure of the concrete provider fixture credentials.
  Removed the similarly vocabulary-based teaching-preview key blacklist. Broad credential and
  private-field name searches remain one-time implementation audit evidence rather than permanent
  test contracts.

- Corrected the planned secure file-upload capability to use canonical Student terminology: the
  active plan is now `secure_student_file_upload_plan.md`, its proposed `StudentUploadId`,
  `student_upload`, `secure_student_uploads`, and `CON-STUDENT-UPLOAD` names are aligned across
  current contracts, and historical dated reports retain their original wording as evidence.

- Standardized the active Instructor roadmap on the temporary `WP-INST-*` package namespace and
  renamed its capability plan accordingly. These labels are disposable plan coordinates that retire
  with the planning layer. Product and source vocabulary now treats Student,
  Instructor, and Sysadmin as the sole human-role terms; new G2 contracts use
  `StudentWorkInspectionStore` and `InspectedStudentResponseV1`. Accepted migration files retain
  their historical package comments while current planning identifiers and future capabilities use
  the canonical namespace.

- Strengthened the approved `WP-INST-G2` binding plan for one roster-first calculated Gradebook and
  one explicit audited Student-work inspection read. It now binds operation filters and the
  `singleStudent`/`studentSelection` choice into structural cursor continuation; gives each later
  page its own live scoring witness; assigns safe response rendering to
  `question_model::presentation`; and specifies Fetch Metadata, server-owned audit facts,
  parameter-bound SQL, secure errors, and separate security telemetry. The four migrations now
  close authority in order: foundation (`1870`), private immutable witness (`1871`), the only
  app-executable broker with atomic audits (`1872`), and demonstrated indexes (`1873`).

- Closed the final G2 architecture findings with an exact typed Student-selection row that reuses
  the Gradebook run-choice union and a Fetch Metadata decision table covering same-origin requests
  plus explicit user-initiated top-level navigation. Independent HCI and security rereviews accept
  the interaction, evidence, and privacy boundaries.

- Reconciled the repository documentation set with current live-demo ownership, exact role
  viewports, state-derived grading recovery, project-scoped Podman cleanup, release notes, and the
  active G1 evidence boundary. The root `AGENTS.md` now routes agents to canonical documents rather
  than restating their content.
- Improved the screenshot evidence states: explicit grade-setting reloads announce the latest
  server settings, the shared assignment picker is captured while open with a selected candidate,
  and curriculum-recovery and authorized-usage captures frame their complete actionable regions.
- Tightened the root `AGENTS.md` to bare-path pointers while retaining active-plan authority,
  package identity reservations, ownership boundaries, dependency-order workflow, and the
  final-tree Validation completion rule.
- Refreshed `docs/TROUBLESHOOTING.md` with the fixed Python environment and
  owner-scoped browser, screenshot, Podman, cleanup, and migration recovery
  paths; corrected the destructive scope of `./run_live_demo.sh stop`.
- Added a typed real-stack route-surface readiness helper that uses the configured Playwright
  action timeout for assignment-overview and practice-entry waits in catalog discovery evidence.
- Reordered the shared learner assignment presentation so the single primary Start/continue action
  follows assignment identity before instructions and progress/details. The action region adapts
  across widths without overlays, and answer-free Instructor Student view omits it when no primary
  action is supplied.
- Made the live-demo Python runtime self-contained: `run_live_demo.sh` now creates or refreshes a
  fixed Python 3.12 `.venv` through `devel/setup_python.sh`, installs the declared manifests, and
  executes the controller through that environment for both start and stop. `all_test.sh` reuses
  that same owner before its pytest and connected acceptance gates. The pinned `PyYAML==6.0.3`
  runtime requirement now belongs in `pip_requirements.txt`, while developer tools extend it through
  `pip_requirements-dev.txt`.
- Reused the shared copyable Question ID control in Instructor grading operations. The operation
  keeps the question title as its heading, exposes the stable public ID with accessible copy success
  and manual-copy fallback status, and keeps the retry action bound to the exact title and ID.
- Aligned current operator documentation with the repo-owned Python runtime: `./run_live_demo.sh`
  remains the ordinary live-demo entry, while direct controller and pytest work use the prepared
  `.venv/bin/python` after sourcing repository settings.
- Classified `.venv` consistently as installed dependency state in Git, ESLint, Prettier, and
  hygiene discovery after the aggregate gate exposed ESLint traversing pip-vendored JavaScript.
- Reconciled the remaining live controller entry points with the repo-owned `.venv`, added negative
  capability-receipt coverage, removed duplicate readiness coverage, isolated the completed-receipt
  privacy test from an unrelated fixture, and clarified the response-redaction and selector-owner
  contracts found during audit.
- Updated the connected PostgreSQL G1 oracle to exercise the five-UUID retry V2 denial with SQLSTATE
  `42501` and to verify Instructor retry receipt category, actor provenance, and worker exclusivity.

### Decisions and Failures

- Recorded the owner-confirmed single-installation model in Human Guidance, Design Decisions, User
  Roles, the active handoff, and a new atomic implementation plan. PLE uses global accounts, one
  open published-question catalog for approved Instructors, private drafts, exact course/Student
  FERPA authority, and multiple equal co-Instructors. Student question delivery remains
  assignment-bound, while anonymous web requests receive no catalog authority. `WP-SD1` owns the
  clean pre-production schema and contract rebase; broad Graphify/source inventories remain
  one-time evidence.
- Advanced WP-INST-G1's accepted-input boundary: immutable server-private submissions remain the
  grading authority, replay returns the original receipt, and answer-free learner and Instructor
  projections keep response material private. Source/projection digests, receipt immutability,
  integrity-failure routing, worker readiness, and the existing generation-fenced score path remain
  explicit contracts rather than compatibility behavior.
- Proved the learner terminal path through the canonical production browser: a successful accepted
  response clears the answer buffer, reaches `acceptedPending`, exposes status-only recovery, moves
  through deterministic Instructor attention, and reaches completed feedback after one Instructor
  retry without another learner answer POST. The answer-free audit covers every submitted,
  completed, operation-list, and retry response variant.
- Accepted the connected G1 evidence package for the canonical production-browser journey, the
  fresh pre-reconciliation 95-migration PostgreSQL/RLS and worker oracle, WebWork service, and
  replica-restart acceptance. The package also atomically published and verified the 63-artifact
  screenshot corpus; HCI review repairs the dense operation and learner layouts, and independent
  architecture and security/privacy reviews approve the resulting boundaries.
- Completed the approved G1-W7 forward reconciliation: accepted migrations `2026081849`, `1850`,
  `1855`, `1859`, `1860`, `1861`, and `1865` are restored byte-for-byte, and the closeout source
  is implemented across `2026081866` through `2026081869`. The four atomic owners are receipt
  schema/preflight, execution writers, the 36-input commit-v2 writer, and Instructor writers with
  retry V2 and public V1 retirement. The affected live evidence is green on the 99-migration tree.
- Accepted `WP-INST-G1` after the final material-tree aggregate passed every required gate. G2 now
  owns audited learner-work inspection and the grade-scheme-aware calculated Gradebook.

### Developer Tests and Notes

- Rebased the Memory M2a source primitives on the current BlueprintCourse/CourseInstance
  contracts. Exact source snapshots now consume `ObservedBlueprintSource`; destination witnesses
  use bounded `CourseInstanceWitness` assignment revisions; pin recovery carries its exact source
  and permits only deterministic, visible Published replacements. The former Alpha source enum,
  retired schedule witness, import-inspection DTOs, and fast-forward projection are removed from
  this helper seam. M2b-M5 retain operation dispatch, lifecycle, update, and integration work.

- Repaired the BlueprintCourse/CourseInstance server-authority contract: non-Serde server-held
  records now exclusively construct fork, assignment-adoption, instantiation, rollover, term-shift,
  controlled-update, selected-copy, and receipt-targeted reconciliation commands. Each record
  retains its exact source/destination, import or schedule evidence where applicable, authenticated
  actor, canonical request digest, idempotency key, and creation reservation where applicable.
  Immutable controlled-update, selected-copy, and reconciliation receipts now provide checked
  construction and read-only evidence access. Browser previews remain answer-free explanation;
  deterministic mutation-resistance tests prove later preview changes cannot affect a record-built
  command. Store/PostgreSQL integration remains the downstream owner of record persistence,
  atomic compare-and-consume, and runtime authorization.

- Regenerated all 63 screenshots through the canonical production-stack owner after the evidence
  fixes. Every ordinary scenario and the isolated deterministic grader-exception profile passed,
  publication completed atomically, cleanup was exact, the 97-case publisher suite passed, and the
  offline corpus verifier accepted the current production-dist provenance.
- Passed focused TypeScript, ESLint, Prettier, learner-presentation Node, Student-view contract, and
  `git diff --check` gates. The focused production G1 browser journey then passed clipboard
  confirmation, retry-focus, learner-completion, Gradebook, answer-free network, single-origin, and
  exact-cleanup assertions.
- Regenerated the canonical live-stack screenshot corpus: 63 PNGs, including 54 desktop artifacts
  at 1280 by 800 plus three Student artifacts at each of tablet 800 by 1280, iPhone Pro 393 by
  852, and square 800 by 800. The 97-case screenshot publisher suite passed, and the independent
  offline corpus verifier passed after its receipt contract aligned per-artifact origin and
  generation-digest validation with the publisher.
- The first exact aggregate Validation attempt passed the complete Rust gate, then exposed and
  stopped at the `.venv` ESLint ownership defect. After the ownership repair, all five frontend
  gates passed with 369 Node tests, and the permanent Python suite outside the known
  tracking-dependent Markdown-link module passed 7,654 tests.
- The audit repair gate passed 65 focused Python tests, 7 focused Node tests, shell syntax,
  Prettier, controller help, and `git diff --check`.
- An intermediate aggregate passed the complete Rust and codebase gates, then reported 7,912
  Python checks before local-link scope stopped that run. Complete live acceptance separately passed
  every production-browser scenario, all 99 migrations and connected PostgreSQL/RLS/worker oracles,
  isolated WebWork, replica restart and durable replay, and exact cleanup.
- A one-time shadow-index diagnostic added exactly the 13 intended durable artifacts to an isolated
  temporary Git index while preserving the real repository index. The unchanged aggregate then
  passed Rust/Wasm, 369 Node tests, 7,978 pytest checks, every production-browser scenario, all 99
  migrations and connected PostgreSQL/RLS/worker oracles, isolated WebWork, replica restart/durable
  replay, and exact cleanup. This one-time implementation probe established the remaining gate path.
- Final `source source_me.sh && ./all_test.sh` Validation passed on the material tree: Rust checks,
  tests, doctests, strict Clippy, and browser Wasm; 369 Node tests; 7,978 pytest checks; every
  canonical production-browser scenario; all 99 migrations and connected PostgreSQL/RLS/worker
  oracles; isolated WebWork; replica restart and durable replay; and exact disposable cleanup.
