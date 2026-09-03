# Changelog

## 2026-09-03

### Behavior or Interface Changes

- Allocated `WP-SD1-A-TERM-01-PR1` for the approved direct Product Role cutover. It replaces the global Account and Authenticated Session role names through fresh schema, Rust, generated contract, strict browser decoder, route gate, Live Demo selector, direct PostgreSQL oracle, and current documentation while preserving distinct Course Membership Role and authorization/RLS behavior. Vocabulary row 446 remains unchecked pending independent review and full required acceptance evidence.

- Allocated `WP-SD1-A-TERM-01-QLB1` for the Question Library Browse boundary. The browser-only flattened row, evidence, facet aggregate, query, page, repository, state, session, decoder, normalization, virtual-window, and bound now use direct `QuestionLibraryBrowse*` names in the Library, Question Picker, Assignment Editor, and sole API adapter. Generated `QuestionSearchRequest`, `QuestionSearchResult`, and `QuestionSearchPage` remain the sole transport vocabulary; `QuestionSearchAuthorship` and `questionSearchRequest()` remain generated-request vocabulary. The scoped residual detector is zero; 18 focused Node tests, both TypeScript checks, QLB1 Prettier, and `git diff --check` pass. `check_codebase.sh` is non-green only for unrelated formatting drift in `src/api/decoders/assignment_attempt.ts` and `src/route_access_boundary.tsx`, so QLB1 is ready for independent implementation review but not accepted; vocabulary row 318 remains unchecked. No Store, route, schema, generated transport source, fixture, feature, or alias changed.

- Completed `WP-SD1-A-TERM-01-QC2` by unmounting the unsupported generic Question Curation
  aggregate: its Question Model contracts, generated declarations, browser transport, repository,
  panel, Folder picker source, mocked scenario, and feature-only tests are removed. Independent
  implementation review and final exact-tree evidence retire the completed generic-aggregate
  detector only. The Question Library picker uses its mounted search boundary and exact
  current-account authorship scope for My Questions. `2026082912_question_folders.sql` remains the
  inactive Account-owned Question Folder and Saved Question Search schema foundation with forced
  RLS and revoked public grants. Question Curation remains documented Instructor workflow
  terminology. The separate 50-match contextual privacy audit, Question Folder Share, and future
  exact Folder/Search operations remain open.

- Completed `WN1-QM-QUESTION-SEED-WIRE`: the six direct portable Question Seed fields on
  Question Variation, Question Presentation, Question Attempt, Student Question Attempt View,
  Next Issued Attempt, and Prefetched Next Question now use Rust-Serde-owned `question_seed`
  through generated declarations, strict readers, browser consumers, and existing fixture data.
  The aggregate gate found and corrected the existing inline Wasm Question Presentation payload
  from `seed` to `question_seed`. The descriptor codec still hashes the numeric Question Seed, so
  its expectation, descriptor bytes, Question Presentation Checksums, and public Question
  Presentation Tokens are unchanged. Concurrent response-item-reference and Question Presentation
  Response Format vector changes are outside this package. The exact zero-residual bare-portable-
  seed detector is retired after independent review. The broader contextual Question Seed audit and
  source-owned generator and visible authoring work remain open. This direct field cutover adds no
  schema, route, Store, migration, backend protocol, fixture family, feature, compatibility alias,
  or permanent test.

- Completed `WP-SD1-A-TERM-01-QF2`: SQLx migration `2026082912` now uses the direct
  `question_folders` filename, and `ple_private.question_folder` states its exact Account-owned
  Published Question-reference relationship. Membership grants neither access nor ownership.
  The three existing private physical tables, SQL behavior, columns, grants, forced RLS, routes,
  Stores, generated contracts, fixtures, tests, and Question Curation remain unchanged. The exact
  QF2 content-and-filename detector is zero; the broad contextual privacy audit remains active,
  and vocabulary row 470 stays open. Migration embedding, disposable PostgreSQL Migration
  Acceptance Runtime, Markdown links, and diff checks pass.

- Completed `WN1-C6-QM-STUDENT-VIEW-SCENARIO-WIRE` and vocabulary row 347. The complete
  Student View Scenario wire now uses direct Rust-Serde `snake_case` through the preview-plane
  producer, regenerated declarations (447), strict decoder, browser request writer, UI consumers,
  and existing focused evidence. The retired `selectedMoment`/`timeZone` spelling remains only in
  one hostile Rust refusal payload; seven `selectedStudent` matches are TypeScript-local
  `StudentViewScenarioBuilder` interaction state rather than serialized PLE meaning. The exact
  Scenario detector is retired after contextual audit and independent review. Focused Rust (2 +
  6), Node (4), generation, TypeScript, Rust formatting, strict Clippy, and diff gates pass.
  Repository-wide Prettier is an existing non-green baseline and is not attributed to this package.
  This direct cutover adds no alias, route, Store, schema, PostgreSQL persistence, fixture,
  feature, or permanent test; those delivery boundaries remain declared but unmounted.

- Completed `WP-SD1-A-TERM-01-QSRC1`: PLE Question Implementation registration,
  Draft Question Revision, Question Revision, and removal of the hard-coded
  authored PLE implementation are independently checked terminology concepts.
  The retired PLE registry/one-variant execution family, algorithmic PLE format,
  hard-coded authored Question implementation, and Question Definition names
  are absent from the current model, schema, adapters, generated contracts,
  browser boundary, fixtures, and durable documentation. Direct static PLE
  Question JSON selects by exact Question Format and Question Type; Draft
  Question Revision and Question Revision name the accepted private and
  immutable published records. The completed exact algorithmic, hard-coded
  implementation, and Question Definition detectors are retired after
  contextual zero residual and independent review. Future source-owned Question
  Generator/publication work, the seeded-generator lifecycle, and Question
  Watch remain open with their current detectors. Existing focused model,
  adapter, generated-contract, browser, formatting, PostgreSQL, residual, and
  diff gates pass. This direct deletion-only correction adds no Store, route,
  publication coordinator, schema capability, fixture family, feature, alias,
  or permanent test.

- Completed `WN1-C6-QM-ASSIGNMENT-GRADE-PROGRESS-WIRE` and vocabulary row 126: the retired
  mixed Student Assignment Summary is structurally split. Keyed `AssignmentGrade` owns grade
  selection and score facts; keyed `AssignmentProgressRecord` owns completed Assignment Attempt
  count, Question Attempt count, and latest activity time. Key-free `StudentAssignmentGrade` and
  `AssignmentProgress` remain distinct browser-safe values, nested by `StudentAssignmentProgress`
  and `GradebookSummaryRow`. The direct Rust-Serde wire contract, regenerated declarations, strict
  decoder, unmounted browser facade, Student display, existing focused Node summary data, and current
  documentation now use `snake_case`; strict decoding and the Gradebook test refuse the retired
  flattened shape. The completed exact detector was retired after contextual zero residual and
  independent review. This correction added no Store, route, schema, persistence, worker, fixture
  family, service, or permanent test; those delivery boundaries remain unmounted.

- Completed `WP-SD1-A-TERM-01-SD1LABEL` and vocabulary row 96: exact local component,
  capability, responsibility, and guarantee language replaces 52 transient `SD1` comment or
  diagnostic occurrences in 34 active SQL migrations and 71 occurrences in 18 durable
  current-state files (17 `docs/*.md` files and `README.md`). The scoped current-state residual
  audit is zero, and its completed temporary detector is retired. `SD1` remains only in approved
  active-plan/status allocation, dated changelog or archive history, and the vocabulary ledger.
  The SQL edits change comments and diagnostic messages only; object names, predicates,
  privileges, data shape, and authorization behavior are unchanged. PostgreSQL acceptance,
  documentation/style/Prettier gates, and independent review pass. Row 630 remains separately
  checked; this terminology-only receipt does not claim overall terminology or `WP-SD1-A` release
  acceptance.

- Completed `WP-SD1-A-TERM-01-AEM1` and vocabulary row 530: PLE now has no current Assignment
  Export persistence record, Job kind or target, Store, route, worker, delivery path, browser
  contract, or service. The retired request/artifact identifiers and export Job/catalog family
  are absent; four durable catalog-oracle absence assertions protect that baseline. Course Grade
  CSV export, QTI interchange, and the answer-key-free pure `export_crate` DOCX/PDF renderer
  remain. Assignment Export Manifest remains future admission for a complete authorized service:
  a private immutable typed frozen input, never an Object ID. Fresh PostgreSQL catalog,
  print-renderer, documentation, Rust, residual, and independent-review evidence pass. This
  package adds no preparatory schema, route, Store, browser contract, fixture, permanent test, or
  feature; this entry does not claim the separate aggregate `all_test.sh` gate.

- Completed `WP-SD1-A-TERM-01-PMAR1` and vocabulary row 630: seven `R100`
  renames establish the Local Stack Controller-owned PostgreSQL Migration
  Acceptance Runtime, its `postgres_migration_acceptance/` private state,
  closed manifest boundary, canonical migration-acceptance commands, and
  `e2e_postgres_migration_acceptance.sh` dispatcher. The validated migrator
  `PostgresUrl`, manifest and permission checks, fixed Browser Suite lease,
  role restriction, and redaction are unchanged. This terminology cutover adds
  no alias, schema, Store, route, browser surface, fixture, feature, or
  permanent test. Controlled detector evidence, scoped residual audit,
  focused gates, earlier connected PostgreSQL and Course Appearance evidence,
  and independent acceptance pass. The shared exact-tree aggregate receipt
  below supplies final `all_test.sh` evidence.

- Completed `WN1-QM-TEACHING-ACCOMMODATION-ADJUSTMENT-WIRE`: the direct
  Accommodation Adjustment producer, generated declarations, strict decoder,
  browser request writer, policy dialog, and hypothetical Student View Scenario
  modifier use Rust-Serde `snake_case` and `extend_only`. The strict decoder
  refuses the retired lower-camel shape. The focused Question Model (11),
  preview-plane (2), Domain preview-plane (6), and Node (14) gates; generation
  of 448 declarations; TypeScript; Rust formatting; strict Clippy; and diff
  checks pass, as does independent review. The scoped Accommodation residual is
  contextually zero, so its completed temporary detector is retired. No alias,
  route, Store, schema, persistence, fixture, feature, or permanent test was
  added. Vocabulary row 347 remains open: the Student View Scenario and Late
  Work/Assignment Deadline wire inventories require their own closures.

- Completed `WN1-QM-ASSIGNMENT-LATE-WORK-DEADLINE-WIRE`: the seven direct
  Assignment policy fields-`available_at`, `due_at`, `closes_at`,
  `assignment_attempt_time_limit_seconds`, `attempt_limit`, `late_work_rule`,
  and `assignment_deadline_rule`-now use direct Rust-Serde `snake_case`
  through current producers, generated declarations, strict readers, browser
  writers, and UI consumers. `mark_late` and `auto_submit` are likewise the
  sole portable enum values. Three existing hostile assertions retain
  `markLate`/`autoSubmit` only to prove strict rejection. The direct
  Late Work/Assignment Deadline detector retired after its contextual audit and
  independent review. Existing focused Question Model, Domain preview-plane,
  Node, generated-contract, TypeScript, formatting, strict Clippy, and diff
  gates pass. No alias, route, Store, schema, persistence, fixture, feature, or
  permanent test was added. Vocabulary row 347 remains open solely for the
  independently allocated Student View Scenario wire cutover.

- Historical `WP-SD1-A-TERM-01-SVS1` receipt: selected and
  hypothetical Student View Scenario requests use exact selected-membership
  and sole identity-free modifier inputs, and `StudentViewScenario` is closed
  to `selectedStudent` or `hypothetical`. The public
  `PreviewEvaluation::Allowed.student_view_scenario_admission` pairs those
  origins only with `selectedStudentActiveStudentCourseMembership` or
  `hypotheticalStudentViewScenarioAdmission`; the strict decoder rejects a
  cross-paired payload. Actual Student access remains
  `AssignmentAccessDecision`, while the identity-free resolver returns private
  `HypotheticalStudentViewScenarioPolicyDecision`. Browser operations, UI, and
  generated declarations use the exact Student View Scenario vocabulary.
  Route, Store, schema, PostgreSQL persistence, fixture, and feature remain
  declared but unmounted. This direct cutover added no compatibility alias,
  mounted delivery behavior, fixture, permanent test, or feature. Generated
  contracts contain 448 declarations, 3 tracked fixtures validate, focused
  Rust/Node/TypeScript and documentation/residual/diff gates pass, and
  independent acceptance is recorded at
  `/private/tmp/ple_svs1_final_acceptance_20260903.md`. The shared exact-tree
  aggregate receipt below supplies final `all_test.sh` evidence. The later WN1
  wire audit reopened vocabulary row 347; it remains open until its direct
  snake-case contract cutover is complete.

- Final exact-tree aggregate receipt for completed `WP-SD1-A-TERM-01-SVS1`
  and `WP-SD1-A-TERM-01-PMAR1`: `source source_me.sh && ./all_test.sh`
  generated 448 TypeScript declarations and validated 3 tracked fixtures;
  passed Rust format, checks, strict Clippy, tests, doctests, and Wasm; 327
  Node and 5,005 Python tests; PostgreSQL 17 fresh/no-op/ACL/restricted and
  iMathAS three-oracle lanes; PostgreSQL-plus-MinIO Course Appearance;
  cleanup; and complete live acceptance. Expected opt-in ignores were 3
  iMathAS HTTP, 7 WeBWorK HTTP, 4 PDF/DOCX reader, 3 ordinary iMathAS
  PostgreSQL, and 1 ordinary MinIO conformance test. The tester made no
  edits.

- Completed vocabulary row 200: standard Student delivery now carries the
  nonce-bound `QuestionPresentation` rather than re-projecting it as a
  pre-issuance `QuestionVariationPresentation`. The iMathAS `/question` wire
  includes the exact `imathasQuestionBackend` Question Presentation Response
  Format marker and deterministic codec tag; `NotApplicable` remains only a
  session/launch capability fact. The Student contract retains revision, seed,
  nonce, and presentation-scoped Response Item References through decoding,
  state, rendering, response controls, Student Response display, and Instructor
  Student Work Inspection. Editor Preview, Question Backend, cache, and
  reproduction retain Question Variation Presentation; server evidence remains
  Issued Question Presentation. Dead delivery decoder/unions/casts are removed.
  Aggregate validation corrected one stale pre-issuance shape in the existing
  Student Response projection fixture to the issued `QuestionPresentation`
  identity and `singleChoice` response-format shape, retaining its public-only
  mismatch-refusal purpose. No compatibility alias, fixture family, schema
  migration, feature, permanent test, wire fallback, or persistence change was
  added. Focused Node, Question Model/Domain, generated-contract, TypeScript,
  lint/format, Rust/Wasm/Clippy, documentation/residual, and diff gates pass,
  with two independent closure reviews passing. Final aggregate validation
  generated 448 TypeScript declarations; validated 3 tracked fixtures; passed
  Rust format/default/all-feature checks, strict Clippy, workspace
  tests/doctests/Wasm, 5 frontend checks with 327 Node tests, 5,005 Python
  tests, PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS (3 tests),
  PostgreSQL-plus-MinIO Course Appearance, and complete live acceptance. Three
  established opt-in iMathAS loopback HTTP tests remained ignored; connected
  Playwright is outside this aggregate gate.

- Completed vocabulary rows 194--196: the sole current Question Presentation
  terms are Presentation Response Item Reference, Response Item Role, and
  Response Item Binding across the model, fail-closed response translation,
  generated contracts, strict browser decoders, and durable documentation. This
  pre-production direct domain-separation label cutover changes deterministic
  Presentation Response Item Reference, Question Presentation Checksum, and
  public Question Presentation Token values and vectors. It preserves the
  security semantics and uses no compatibility path. An exhaustive residual
  scan retains `RenderedItem*`/rendered-item spelling only as historical
  evidence in the replacement ledger, changelog, and dated project-status
  report. Existing Question Model presentation, generated-contract,
  strict-decoder, and browser gates pass.

### Fixes and Maintenance

- Aligned the processed WebAssembly export allowlist with the existing
  `validate_presentation_response_format` function. That function is the key-free
  Question Presentation Response Format plus Student Response validation boundary
  already owned by the Wasm bridge and browser facade. The existing export allowlist
  and bridge-parity E2E checks pass, as does independent review. No production code,
  generated output, fixture, schema, route, Store, browser surface, or new test changed.

- Aligned current-state documentation with the mounted application: user and
  operational guides distinguish seeded Account-session entry from future
  teaching workflows; architecture, migration, retention, and evidence
  inventories describe the current tree and release-blocking browser gap.
  Precise source comments and ASCII JSX separators now describe their exact
  owners without changing rendered behavior. Removed the unused browser
  transport compatibility bridge and the `BaseAssignmentPolicy` compatibility
  re-export; the owned Question Model import remains. Existing focused
  Markdown, Rust, TypeScript, and HTTP-client validation passed. This records
  no new test or feature and does not close the separate SD1 or SVS work.

- Replaced transient WN1/T2/S4/S3/B3 and vague plan prose in production
  comments with durable mounted-route, Teaching Operations, Student projection,
  Assignment Access, typed-storage, grading-boundary, and database-design
  language. Two affected declarations were regenerated; behavior is unchanged,
  and independent review passed. The connected SD1 production-family rename
  remains open and is not claimed by this comment-only correction.

- Completed vocabulary row 344: the sole browser decoder helper and its
  unknown-kind diagnostic now use exact Assignment Start Decision language.
  Rust-generated `TeachingAssignmentStartDecision` documentation matches. The
  bounded `start` wire member and existing Assignment Access decision owners
  are unchanged; the policy-preview route and Store remain unmounted. This
  narrow correction adds no schema, route, generated-contract change, fixture,
  permanent test, or feature. Focused decoder, TypeScript, lint/format,
  Question Model check, generated-contract parity, Markdown, residual,
  tracker, and diff gates pass with independent review.

- Recorded the open Question Pool Selection Position row's browser-safe slice:
  `selectedQuestionNumber` and `selectedQuestionCount` are the only position
  fields, a strict closed decoder admits precisely that shape, and Student copy
  presents only ordinal/count. The exact retired-field scan is one-time
  migration evidence; the existing focused strict closed-decoder test rejects a
  surplus `seed` field, so no redundant retired-field-specific permanent test
  was added. Persisted Question Pool Selection delivery/replay remains
  unmounted and keeps row 147 open.

- Recorded the open Question Pool row's completed reusable Blueprint slice:
  nested reusable Blueprint Question Pools use `items` and Question Pool Items
  through Rust, generated contracts, strict browser decoding, and editor UI.
  Sole-current Blueprint Revision Content encoding v2 serializes nested pool
  `items`; this direct pre-production cutover changes Blueprint Content
  Checksum values and has no compatibility reader or alias. Outer Assignment
  `entries` remains Assignment Entries, while the retired nested spelling
  remains only in strict-rejection evidence. The Blackboard Workspace Import,
  Draft Question, and explicit pool-conversion work remains open. Existing
  focused Question Model, generated-contract, and strict browser gates pass;
  no feature, fixture, or permanent test was added.

- Recorded the Blueprint Revision Content v2 checksum-encoding cutover: private
  helpers and serializer shapes name their exact encoding role, nested Question
  Pool `entries` are `items`, and outer Assignment `entries` remain unchanged.
  The changed versioned encoding deliberately changes every Blueprint Content
  Checksum; no legacy reader, alias, caller, or vector remains. No new test was
  added.

- Completed vocabulary row 432: PLE now uses Account for global identity,
  Product Role for its immutable product classification, and Course Membership
  Role for participation in one Course Instance. UA1--UA6 completed the direct
  cutover through the Account, Student Record, ownership, schema, and local
  stack boundaries; Live Demo configuration uses only explicit
  `*_ACCOUNT_ID` values. The fresh PostgreSQL 17 receipt passed apply/no-op,
  catalog, restricted-login and role probes, Object Delivery, iMathAS Question
  Backend authority, and cleanup. Remaining `user` spelling is contextual
  non-PLE vocabulary, explicit rejected legacy decoder input, or retained
  history; its broad review inventory remains active.

- Completed vocabulary row 403: PLE-authored assessment content now uses Question and
  Question Revision, and repaired Student Question Attempt View mocks use the strict
  `issuedQuestion` shape. Remaining `problem` matches are external-system, renderer/DOM,
  negative-evidence, ordinary-prose, or historical uses; the broad contextual detector
  remains active. No feature, schema, route, fixture, or permanent test was added.

- Completed vocabulary row 514: QTI integrity values now use exact QTI Import
  Checksum and Normalized QTI Item Fingerprint names, while deterministic JSON
  encoding remains a private calculation detail. The independently audited
  closure preserves existing golden-vector, semantic-sensitivity,
  public/private-binding, item-correspondence, and redaction validation; the
  exact-QTI detector is retired. Row 513 remains independently open for the
  format-agnostic Workspace Import Item Result Store/service projection. No
  behavior, schema, route, fixture, or permanent test changed.

## 2026-09-02

### Fixes and Maintenance

- Completed vocabulary row 99: validation, normalization, sorting, renumbering, and
  checksum work now uses its exact operation or subject-qualified Checksum name. The final
  contextual audit retains canonical only for exact representations/grammars, deterministic
  encoding, platform or security APIs, approved interface/type-policy vocabulary, and
  historical or negative evidence. The global Canonical detector is retired. QTI row 514
  remains open with an exact-owner detector for its Report-Digest and checksum-encoding
  names. No behavior, schema, route, generated contract, fixture, or permanent test changed.

- Completed vocabulary row 97: PLE verification facts use subject-qualified Checksum
  names. The final contextual audit retains Digest only for registered cryptographic APIs,
  content identity/fingerprints, cache discriminators, OCI/platform vocabulary, the Local
  Stack Controller's private capability fingerprint, fixed iMathAS `sourceDigest` wire
  spelling with strict ignored loopback shape proof, negative retired-field/SQL-absence
  checks, and history. The temporary Digest detector is retired; QTI row 514 remains an
  independent exact-QTI-Checksum migration. No behavior, schema, route, fixture, or test
  changed.

- Recorded verified partial slices for the open Canonicalization terminology review. CAN1 uses
  `renumberAssignmentsByCategory` and `NonSequentialCategoryPosition`; CAN2 uses
  `validate_and_normalize_package_import` and H5P Package Checksum; CAN3a--d use exact ordering
  derivation, Question ID normalization, expected iMathAS launch-path comparison, and Question
  Search filter normalization names. The calculated Gradebook client's private helper is
  `requirePublicReference`. Genuine canonical Question ID, deterministic-encoding, and unique-
  representation uses remain. Existing focused Rust/Node, TypeScript, and formatting gates pass;
  no behavior, route, wire value, generated contract, schema, fixture, or permanent test changed.
  The Canonicalization row and detector remain open pending its complete contextual audit.

- Closed the generic PLE-owned Definition terminology row. Source, schema, API, browser, Wasm,
  generated consumers, tests, fixtures, durable documentation, and current plans now use exact
  Content, Question Response Format, Rule, Input, View, or owning PLE terms. The final independent
  contextual audit found remaining Definition matches only in language/framework/platform/legal,
  historical, or ordinary-prose uses, so the temporary broad detector is retired with the checked
  ledger row. This terminology closure adds no feature, persistence record, route, generated
  compatibility alias, fixture, or permanent test.

- Narrowed the then-open Definition terminology review in Question Model comments:
  Assignment Revision preserves immutable Assignment Content and delivery rules;
  the WeBWorK capability carries server-only Question Grading Input and replay details;
  and preview inputs serve an Instructor Question Pool Preview. The focused Question
  Model formatting, check, and test gates pass. No behavior, schema, API, wire contract,
  fixture, or permanent test changed.

- Replaced audited PLE-owned generic Definition wording in durable architecture,
  security, lifecycle, backend, cache, payload, Instructor, identity, retention,
  FAQ, cookbook, pilot, and design documents with the established Blueprint
  Revision Content, Assignment Content or Assignment Revision, Question Content
  or Question Revision, Question Response Format, Question Grading Rule,
  Question Grading Input, and Question Source terms. At that stage the Definition
  checklist row remained open for current source and broader inventory work; the
  final contextual audit above subsequently closed it. No behavior, schema, API,
  wire contract, fixture, or test changed.

- Narrowed the then-open Definition terminology review in PLE Question JSON authoring: the
  key-free public-preview builder now names Question Response Format and its
  multiple-choice helper, while the focused browser test calls the generated payload
  `draftQuestionContent` and its local object `content`. Wire shapes, generated types,
  behavior, fixtures, and test inventory remain unchanged.

- Clarified the Student Response Format Check boundary: Question Response Format values and
  serialized values now use `responseFormat`/`response_format` and
  `responseFormatJson`/`response_format_json` across the browser, Wasm, domain validation,
  unmounted request body, and shared answer-free fixture readers. The route, generated contracts,
  schema, stores, and validation behavior remain unchanged.

- Narrowed the then-open Definition terminology review at the browser Question Response Control
  boundary. Its seven response-shape aliases now use exact `*ResponseFormat` names, and its
  component props and local validator use `responseFormat` through the dispatcher, direct callers,
  and existing focused test. Question Response Format wire shapes and generated contracts remain
  unchanged; at that stage the broader Definition row remained open and the final contextual audit
  above subsequently closed it.

- Closed the bare Response Control vocabulary row. Current PLE prose, comments, error text,
  accessibility evidence, and component-test descriptions name Question Response Control; the three
  text-entry inputs and their shared styles now use the exact
  `question-response-control__input` BEM class. The concrete
  `imathas-question-backend-response-control` marker remains the iMathAS Question Backend
  boundary. No browser behavior, DOM semantics, API, wire contract, schema, generated declaration,
  fixture, or permanent test changed; no dedicated detector exists.

- Corrected the transient plan-label closure record. PNT2 deployment/configuration comments and PNT3
  current durable documentation now name their actual components, responsibilities, and capabilities,
  and both completed slices received independent review. The SD1 implementation namespace remains open:
  its required six tracked file renames could not proceed because the read-only `.git` directory blocks
  `git mv`, and no fallback move was used. The pending technical boundary is PostgreSQL Migration
  Acceptance Runtime; this entry does not claim that implementation cutover. Historical work-package
  receipts remain preserved in their approved historical and planning homes.

- Closed the Response Widget vocabulary row. The eight browser Question Response Control roots
  and their shared styles use `question-response-control`; current decoder and browser-contract
  prose names Question Response Control. The text-entry inputs and shared styles now use
  `question-response-control__input`; `imathas-question-backend-response-control` remains the
  concrete iMathAS Question Backend Response Control marker. Focused browser tests and
  current-source/documentation checks pass.

- Accepted `WP-SD1-A-TERM-01-IAA1`: direct Create Instructor Account replaces generic
  role/account-ID creation, while private Authentication Email integrity is role-qualified.
  Student email remains immutable; verified Instructor replacement and complete Course Roster
  Import/Student resolve-or-create delivery remain future `WP-RC8` work. Current documentation,
  independent reviews, final residual audit, and the existing SD1 PostgreSQL 17
  fresh/no-op/catalog/restricted-login/iMathAS lane pass; its focused
  `sd1_instructor_account_creation.sql` oracle remains in that lane. No dedicated detector exists.

- Closed the retired `StudentSubmissionStatus` vocabulary row: the stale current
  `StudentSubmissionStatusStore` registry claim now names the separate Question Submission Receipt
  and Question Submission Grading State boundaries. No detector exists for this row, so none was
  removed. No source, schema, API, route, generated declaration, fixture, test, or feature changed.

- Completed the Local Stack Controller Developer Browser Suite lifecycle cutover. The direct
  operations are `request_developer_browser_suite_stop`,
  `purge_orphaned_developer_browser_suite`, `clear_developer_browser_suite`, and
  `start_developer_browser_suite`. `clear_developer_browser_suite` leaves the fixed suite empty
  through authenticated stop or held-lease orphan purge; the control socket, capability, expiry,
  one-use completion, and lease checks remain unchanged. Controller, CLI, focused tests, and
  operator copy now use the exact lifecycle names. This changes no Account Authenticated Session
  terminology, persistence, API, schema, compatibility layer, or test inventory.

- Closed the checked Student-file terminology migration detector lifecycle. A final contextual
  audit found no Student Upload, Student Artifact, Student Feedback Attachment, or `WP-FU1`--`WP-FU6`
  owner in implementation, schema, API, generated contracts, tests, fixtures, or active plans; the
  exact `2026080912` implementation-status row remains the sole broad historical match. Retired the
  three temporary row-213 searches together and removed their now-unused active-plan helper. This is
  a documentation/detector-only closure supported by one-time searches; it adds no permanent test.

- Closed the retired repository-owned fixed `.venv` launcher/developer-interpreter terminology.
  The Local Stack launcher, controller, aggregate validation, reset helpers, and documented developer
  command paths source `source_me.sh` before invoking selected Python 3.12 `python3`; the
  `local_stack.py` controller then runs under that selected interpreter. Runtime and developer/test
  dependencies remain independently declared in `pip_requirements.txt` and `pip_requirements-dev.txt`.
  The vocabulary detector was retired after contextual review confirmed its eight current matches are
  exact dependency-directory cleanup/exclusion names in maintenance and tooling boundaries. This
  documentation-and-detector closure adds no permanent test.

- Accepted `WP-SD1-A-TERM-01-PI2`, the direct Assignment Question Analysis domain/schema/typed-Job
  cutover in fresh migration `2026082923`. `AssignmentQuestionAnalysis` and
  `assignment_question_analysis` own the Course Instance-and-Assignment-scoped, Scoring
  Generation-bound analysis for one source Assignment Entry and exact Question Revision; the
  four-category Question Outcome Distribution remains separate from Unscored Attempt Count. The
  production repair constrains the typed Job target; two fixture/oracle composition repairs complete
  existing evidence. `assignment_analysis_course_assignment_matches` proves the composite Course
  Instance-plus-Assignment relationship and rejects a cross-Course Assignment. Focused model/schema
  evidence, independent inventory, and the live SD1 least-privilege PostgreSQL lane pass. No Store,
  route, browser, generated contract, worker, new test file, or fixture suite was added; one compact
  reciprocal case was added to the existing durable SD1 oracle. This package precedes but does not
  accept the existing `WP-SD1-A` architecture/privacy final SD1 gate.

- Recorded TX1 as a direct pre-production removal of the unsupported generic Question
  Classification surface. Current PLE Question JSON, Question Metadata/Search, strict browser
  decoders, Instructor metadata, the client-only classification route, and the QTI empty default
  no longer promise arbitrary System/Code/Name mappings. This is not a substitute taxonomy
  feature: Question Classification remains future-only until one real Classification System exists;
  Question Subject, Question Subsubject, and Question Bloom Classification stay independently
  open, and QTI vendor classification data remains source-format vocabulary. Focused Question
  Model/PLE/QTI adapter tests, workspace check, 76 focused Node tests, Markdown/source-line tests
  (1,126 passed), focused Prettier, and diff checks passed. Normal generated API regeneration
  removed the stale generic declarations; the generated API now has no generic classification
  declaration and `npx tsc --noEmit` passes.

- Unified repository Python execution on `source source_me.sh && python3 ...` across the live-demo,
  aggregate validation, Podman reset, controller help, installation, development, operations,
  troubleshooting, security, test-evidence, and release documentation. `pip_requirements.txt`
  supplies the live-demo and controller runtime dependency, while full developer and test setup
  installs it together with `pip_requirements-dev.txt` into the selected Python 3.12 interpreter.
  Shell syntax, controller help, 320 focused controller tests, 196 documentation and vendored-header
  tests, formatting, current-source searches, and the diff check pass.

- Completed the Tier 2 terminology closure for Entitlement, Material, Materialization,
  Disposition, Ledger, and Question Presentation terminology. Current boundaries use Assignment Access and Active Student Course
  Membership; exact operation records and Receipts; Answer Key, Question Grading Input, Question
  Feedback, Question Answer Explanation, Question Hint, and format-specific records; Migration
  Check Result; precise Object Cleanup Receipt outcomes; registered HTTP headers; and SQLx's
  platform migration ledger. Workspace Import Item Result Store/service projection (row 508),
  Course Retention rows 585--593 remain open. The Question Presentation closure replaces its six
  current PLE-owned documentation and active-plan uses with Question Variation Presentation,
  Question Presentation, exact private record names, and closed Blueprint-operation browser
  contract wording. The row's retained `presentationEnvelope` test literal is strict retired-wire
  rejection evidence; no product, schema, API, generated artifact, test, or fixture changed.

- Completed the Tier 2 Witness and Locator terminology closures. Question Statistics Observation
  Receipt now has an atomic accepted-grade recording path that derives correctness from the stored
  Grading Result; its PostgreSQL oracle proves eligibility, replay idempotency, ineligible
  non-contribution, and direct-execute denial. Current Question Model, browser, and test comments
  name their exact References, Question Sources, or request facts; generated API comments were
  regenerated from those owners. Remaining raw terms are classified third-party Playwright APIs,
  host-runtime mechanics, exact iMathAS transport values, strict retired-wire rejection tests, or
  retained evidence. Focused generation, Question Model/project-tools, TypeScript, browser-decoder,
  Markdown-link, source-line, shell-syntax, formatting, and diff gates passed.

- Completed the broad Projection-to-View terminology closure. Independent current-tree review
  classified all 88 remaining `projection` matches as precise technical mechanics, the terminology
  definition, or frozen accepted-package history; no current PLE reader shape or interface uses
  generic Projection. The temporary vocabulary tracker now omits completed Projection, Family, and
  Factory reviews. Family retains 23 justified technical, ordinary-language, or historical matches
  after six transient-plan phrases became "all supported Question Types"; Factory's sole match is
  the terminology contract's precise construction-pattern definition. Open Job Kind Registration
  rows remain separate.

- Corrected retained PV1--PV5 View/read-shape receipts to state their historical open scope and the
  later PV6 closure. This record repair changes no implementation, schema, API, generated contract,
  fixture, test, or behavior.

- The source-file line-limit gate now automatically excludes Markdown under `docs/active_plans/`
  and `docs/archive/`; other source types there remain covered. Removed redundant active-plan
  exact-path overrides while retaining the intentional single-document `DESIGN_DECISIONS.md` and
  `TERMINOLOGY_CONTRACT.md` canonical-authority exceptions. Code and other authored sources remain
  subject to the 999-line cap.

- Recorded the owner-established role-qualified email boundary. A Student Account is global across
  courses and semesters, its institutional Student Authentication Email is immutable, Course
  Roster Import resolves or creates that Account by email, and each Course Instance owns its
  separate Student Record and Student Course Membership. An Instructor Authentication Email may
  change after verification while the Instructor Account retains its Product Role, authored
  content, Question relationships, Course relationships, and teaching history. Student Work follows
  the Course Retention Plan independently of the Account lifetime. The terminology contract,
  account decisions, and still-open vocabulary migration row now agree; implementation and schema
  correction remain open.

- Completed the three post-Course-Route PV4 Support source batches: Workspace Editor (six comments),
  PLE Question JSON Public Preview (five comments), and Instructor Preview client (four comments).
  Independent allocation and implementation reviews pass; natural-language Editor Draft wording
  unambiguously describes local `EditorDraft`. No wire/schema/routes/generated/security/behavior
  boundary, fixture, or test changed. All source-owned View/read-shape migration is complete: the
  former 18 Support residuals now equal only the justified fixed SQL query projection in
  `local_stack_control/disposable_stack_adapter.py:579`. At that PV4 stage, PV5 current
  documentation was the sole drift, so the broad View row and temporary projection queue remained
  open; PV6 subsequently closed the broad row.

- Completed `WP-SD1-A-TERM-01-PV4-SUPPORT-COURSE-ROUTE`: exactly two Course Appearance comments now
  name the authorized `CourseRouteView` already loaded by route scope and the `THEME_MIX`
  color-derivation recipe. `courseRouteView()` retains its established Course Route View, route data,
  banner delivery, and CSS; `THEME_MIX` remains a local technical CSS `color-mix()` calculation, not a
  product View. Routes, Course Appearance behavior/theme scope, CSS variables, JSON/wire/generated
  contracts, persistence, fixtures, tests, behavior, and technical SQL/query projections remain
  unchanged. Focused Course Appearance Node tests, TypeScript, target-source Prettier and line caps,
  exact two-path retired-phrase searches, focused Markdown-link checks, and diff gates passed. No abstraction, test, or fixture
  was added. At that PV4 stage, the three remaining Support packages and PV5 were open, so the broad
  Projection/View ledger row was unchecked; PV6 subsequently closed the broad row.

- Completed `WP-SD1-A-TERM-01-PV4-SUPPORT-SESSION`: exactly two scoped comments now say browser-safe authenticated Account data in `AuthSessionResponse` and cached router query data at the browser session boundary. `AuthSessionResponse` and `AuthAccountResponse` remain technical DTO symbols; the Authenticated Session record, `Active Account -> Authenticated Session` path, `Authenticated Session -> Active Sysadmin Account -> exact audited support operation` path, routes, persistence, JSON/wire/generated contracts, fixtures, and behavior remain unchanged. The focused frontend session-boundary test, Rust formatting, TypeScript, exact-scope search, documentation/link, source-line-cap, and diff gates passed. At that PV4 stage, the 18 Support residuals and PV5 were open, so the broad Projection/View ledger row was unchecked; PV6 subsequently closed the broad row.

- Completed `WP-SD1-A-TERM-01-PV4-BLUEPRINT`: seven Question Model owner files now use exact Reusable Question/Pool/Pool Entry, Blueprint Assignment Entry, Blueprint Course Summary/Course, Student Membership, Teaching Account, and existing `CourseInstanceBlueprintInspectionView` reader language. The module names Blueprint Revision Content and target-term schedule; Course Retention Notice is exact; and `convert_teaching_preview_time_field` is the only local-helper rename, retaining its `TeachingPreviewTimeField` result and behavior. `AssignmentImportRepairPreview` remains a server-only, receipt-bound, non-Serde technical derived projection. Operation-specific Preview, Preview Request, Command, Receipt, and Result names; authenticated Account, exact Request Checksum, RequestRetryToken, and accepted-Receipt authority; Course Instance creation/update; persistence, routes, SQL/query projections, wire/generated contracts, fixtures, and behavior remain unchanged. Focused Blueprint (19) and teaching-operation (12) Question Model tests, Rust formatting, TypeScript, exact-search, source-line cap, and diff gates passed; independent review passed. No test, fixture, abstraction, schema, route, or feature was added. At that PV4 stage, support and PV5 documentation were open, so the broad Projection/View ledger row was unchecked; PV6 subsequently closed the broad row.

- Completed `WP-SD1-A-TERM-01-PV4-DELIVERY`: all 40 scoped Assignment-delivery reader owners now use exact reader-and-subject terminology: Assignment Overview, Student Assignment Landing Summary, Gradebook Summary Row, Assignment Progress Record, Assignment Release Validation, Instructor Student View, Instructor Assignment Authored Content Local, Student Question Attempt View, Question Statistics View, Grading Operation Visible State, Student Feedback, and Student Response Inspection Feedback. The closed `preview_plane` result is Assignment Release Validation; the key-free Student aggregate is Student Assignment Progress. Only two browser-local `projection` bindings became `studentView` and `availability`, and test-only `semanticProjection` became `questionPublicationReviewCurrent`. Rust/Serde and TypeScript/generated contracts, JSON/wire fields, persistence/schema, routes, SQL/query projections, Object Delivery/storage/signing/concealment/security boundaries, fixtures, and behavior remain unchanged. Existing focused Question Model/Domain and Node reader tests, Rust formatting, TypeScript, exact-search, source-line cap, and diff gates passed. No test, fixture, abstraction, route, schema, or feature was added. At that PV4 stage, Blueprint, support, and PV5 documentation were open, so the broad Projection/View ledger row was unchecked; PV6 subsequently closed the broad row.

- Completed `WP-SD1-A-TERM-01-PV4-LIB`: the six Question Library-owned read-shape owners now use exact Question Use Details, Question Details, Question Folder, Saved Question Search, Question Summary command-result, and Question Search Results View wording. The strict decoder-local `projectionKind` binding is now `promptKind`; JSON `kind`, `static`/`generatedExample`, decoder shape, generated contracts, routes, schemas, fixtures, and behavior remain unchanged. Existing Question Model, focused Question Curation browser, TypeScript, formatting, exact-scope retired-name, and diff gates passed; independent review passed. No test, fixture, abstraction, route, schema, or feature was added. At that PV4 stage, delivery, Blueprint, support, and PV5 documentation were separately open, so the broad Projection/View ledger row was unchecked; PV6 subsequently closed the broad row.

- Completed `WP-SD1-A-TERM-01-PJAF1`: removed the unsupported PLE Question JSON browser file-authoring client, picker, hotspot editor/model, page/field/style/callback wiring, protocol-only client suite, deleted-only editor assertions, and unconsumed `ObjectAddress::WorkspaceQuestionAsset`. The browser has no PLE Question JSON file input, endpoint, Store, schema, SQL, generated declaration, cache/wire field, Object/URL field, fixture, substitute abstraction, or replacement test. `WorkspaceImportAsset`, imported/trusted `QuestionAssetReference` bindings, Object Delivery, parser/compiler, HOTSPOT grading and presentation geometry, and `StudentResponse::Hotspot` remain. Existing object, adapter/model, Question Model, TypeScript, editor, Markdown, formatting, exact-search, and diff gates passed; one-time absence and retained-binding checks passed. Student Upload remains separately accepted under row 213.

- Completed `WP-SD1-A-TERM-01-SU2`: retired the obsolete `WP-FU1`--`WP-FU6` release path before implementation across every declared current owner, including the dated executive snapshot. The snapshot contains no related retirement history; the exact `2026080912` status migration row is the sole history owner. The combined authority detector counts every matching package or Student/learner file-capability occurrence across active plans, implementation plan, status registry, and that snapshot, excluding only that anchored row; both current counts are zero. One-time probes passed `0/1/1/1/1/1` for exact history, planned-before/planned-after package claims, package retirement wording, Student retirement wording, and learner retirement wording. Course Banner Upload remains separate. Adapter-owned Question Asset import remains separate; PLE Question JSON browser file authoring was separately open under row 278 at this record. Independent implementation review passed; independent record review remains required. Documentation checks, shell syntax, focused Prettier, registry cap, and diff gates passed. The shared status registry retains documented pre-existing whole-file Prettier drift; no broad format rewrite was made. No application code, schema, API, generated contract, fixture, test, behavior, or feature changed.

- Completed `WP-SD1-A-TERM-01-PV3`: removed the unowned browser sanitized-markup override and duplicate WeBWorK raw-HTML delivery/cache branch. The browser renders closed Question Content Blocks directly. Bounded strict parsing, protected-value and hostile/malformed-input refusal, source binding, renderer identity, Question Asset routes, private replay/grading, and typed delivery remain. `CACHE_SCHEMA_VERSION` and the deterministic render-key namespace now use v2; retained `ObjectAddress::QuestionRender`, immutable put/`AlreadyExists` recovery, and current decode/validation semantics preserve an immutable rebuild boundary. A one-time cache probe proved separate v1/v2 keys, v1-byte preservation, and v2 rebuild, then was removed. Normal TypeScript generation, focused adapter/renderer/type/format/documentation/diff gates, and independent review passed. No replacement markup record, wire field, compatibility reader, fixture, or permanent test was added. At the PV3 stage, broader View/read-shape terminology remained open; PV6 subsequently closed the broad View row.

- Completed `WP-SD1-A-TERM-01-RQ-CLOSE`: closed the retired generic Reconciliation checklist rows after fresh Graphify-assisted independent classification confirmed no current PLE-owned Reconciliation in source, schema, routes, generated contracts, browser controls, fixtures, test contracts, product documentation, or active plans. The 14 exact queue matches are frozen history/status/audit material, so the temporary current-work queue omits `reconciliation`. The separately scheduled Local Stack Developer Browser Suite lifecycle correction did not describe or block a product Reconciliation boundary. No code, schema, API, behavior, fixture, test, or feature changed.

- Completed `WP-SD1-A-TERM-01-SU1`: removed the unsupported Student Upload/file-response product path without a replacement. Student Response remains the closed values accepted by a supported Question Response Format; Student Feedback remains authorized result data and optional Question-authored content. Current stored-object examples are Question Submission, Student-specific exports, and annotated exams. Course Banner Upload remains separate; adapter-owned Question Asset import remains separate; PLE Question JSON browser file authoring was separately open under row 278 at this record. Independent implementation review and re-review, existing response/negative-shape tests, normal TypeScript generation (466 declarations), TypeScript, Rust formatting, focused SU1-owner Markdown Prettier, 1,246 documentation checks, contextual current-match classification, registry-cap, and diff gates passed. No schema, route, Store, capability, response format, Object behavior, fixture, test, or feature was added.

- Completed `WP-SD1-A-TERM-01-PV2`: deleted the unowned pre-production `ple_private.student_feedback_release` baseline table, its generic `projection jsonb`, uniqueness constraint, table-local RLS/FORCE-RLS, and shared revoke entry. Question Submission and Assignment Submission evidence, triggers, RLS/FORCE-RLS, and revokes remain. Student Feedback remains a transient policy-redacted DTO, while Student Feedback Release remains the rule/preview boundary; unsupported persistence/audit claims are gone. Independent implementation review, final staged PostgreSQL 17 fresh-apply/no-op/catalog/restricted-login/iMathAS authority, documentation, exact-search, cap, and diff gates passed. No replacement table, compatibility, Store, SQL function, API, generated contract, browser change, fixture, test, or feature was created. At the PV2 stage, broader View terminology remained open; PV6 subsequently closed the broad View row.

- Completed `WP-SD1-A-TERM-01-PV1`: the embedded answer-free Question Details prompt now has the exact `QuestionDetailsPromptView` name across the Question Model, normally regenerated TypeScript contract (466 declarations), and strict browser decoder. The direct cutover preserves wire `prompt`, `static`/`generatedExample`, ordered Question Content Blocks, answer-free serialization, exact Question ID verification, and existing browser behavior. Focused model/decoder/type, documentation/search, and diff gates passed. No schema, route, fixture, test, behavior, or feature changed. At the PV1 stage, broader View/read-shape terminology remained open; PV6 subsequently closed the broad View row.

- Tightened the terminology authority around authored and submitted files. Student Response now consists only of values accepted by a supported Question Response Format, and Student Feedback remains result data plus optional Question-authored content blocks. PLE Question JSON Instructor authoring is defined through Authoring Workspace fields; adapter-owned imports remain the separate boundary that may supply Question Assets. Removed the promised future Student Upload, Student Artifact, and Student Feedback Attachment concepts from the contract/ledger, and recorded the current PLE Question JSON browser image-upload client as open implementation correction. The temporary vocabulary report now tracks both Student file-capability names and PLE Question JSON browser file-authoring owners. No application code, schema, API, generated contract, fixture, test, behavior, or feature changed.

- Completed `WP-SD1-A-TERM-01-RQ4`: six documentation owners made seven exact reconciliation cutovers to Object Storage Check/Object Storage Repair, data-only host installation/Migration Check, Blueprint-operation application/Assignment Import Repair, and Assignment Arrangement creation using the existing Student Course Membership. The unsupported `CON-STUDENT-UPLOAD` contract row was deleted because PLE has no Student Upload source, route, schema, or consumer. Independent implementation review, documentation/search/formatting, diff, and registry-cap gates passed. No code, schema, API, generated contract, fixture, test, behavior, or feature changed. SU1 subsequently removed the remaining false future-upload prose without replacement; reconciliation remains open only for its separately reviewed current-match classification and queue update.

- Completed `WP-SD1-A-TERM-01-RQ3`: five exact operational comment/prose owners now name embedded SQLx DDL application, Migration Check evidence, live-demo Browser Suite reset, Compose startup, and Store-owned Course Roster Import/Course Membership state. Existing focused Rust, Markdown, SQL-source-line, contextual-search, and diff gates passed. No schema, API, generated contract, fixture, test, behavior, or feature changed. A fresh residual review kept row 623 and the temporary reconciliation queue open for five distinct current generic-product documentation corrections.

- Completed `WP-SD1-A-TERM-01-RQ2`: six planned inventory-wide documentation owners now distinguish an Object Storage Check, which records verified, missing, or mismatched evidence, from authorized Object Storage Repair. Object Cleanup Manifest and Object Cleanup Receipt remain separate. Current schema, API, generated contracts, tests, object crate, and existing Object Storage Check/Cleanup evidence were already canonical and unchanged. Existing Markdown-link, ASCII, whitespace, source-line, Prettier, contextual-search, and diff gates passed. WP-RC7 remains planned and unimplemented; this documentation cutover supplies no inventory-worker operational proof. No code, schema, API, generated contract, fixture, test, behavior, or feature changed.

- Completed `WP-SD1-A-TERM-01-COR5`: the final current Corpus residuals now use Pilot Question Set, historical screenshot reference, the exact laptop viewport profile, Question Library, or Question Pool according to their actual owners. Project-tools file-helper/test names changed without altering fixture bytes, manifest shape, test body, behavior, or coverage. Independent review and final-tree project-tools (43), documentation (1,222), Rust/Markdown formatting, exact residual searches, 999-line caps, and diff gates passed. Row 390 is complete; retained `corpus` uses are frozen vendor-QTI parser fixture evidence, technical changelog-query corpus/`--corpus`, dated audits/workstreams, or ordinary technical/security/style prose. No schema, API, browser/screenshot owner, generated contract, fixture, test, or feature was added.

- Completed `WP-SD1-A-TERM-01-RQ1`: the five Question Model contract files, current same-boundary documentation, and normal generated TypeScript output now use server-held `AssignmentImportRepair*` and `BlueprintOperationKind::RepairAssignmentImport`. The obsolete `ReconcileCourseInstanceCompleted` Serde declaration and its generated artifact were deleted without replacement. Assignment Import Repair preserves the closed two-variant Assignment Import Receipt, Course Origin/Course Instance match, distinct Account/Request Checksum/Request Retry Token binding, readiness guards, and exactly six browser Blueprint operations. Rust formatting, strict Question Model Clippy/tests, normal TypeScript generation (466 declarations), TypeScript, documentation, exact-boundary searches, and diff validation passed. No schema, Store, PostgreSQL procedure, route, browser control, fixture, test, behavior, or feature changed. Object Storage Check/Repair and ordinary technical reconciliation wording remain RQ2/RQ3 work.

- Completed `WP-SD1-A-TERM-01-COR3`: directly unmounted the half-restored screenshot-capture boundary because no JSON authority, publisher, publication receipt, or browser owner exists. Two dead capture helpers and the capture-only imports, calls, fields, declarations, obsolete assertions, and former viewport dependency are gone. All eight real-stack journey behavioral assertions remain, including the Instructor Grade Settings laptop overflow check, Question Curation target visibility/overflow checks, and Question Library visibility, scroll-placement, keyboard-path, and overflow checks. Current documentation calls the retained screenshots historical reference and preserves dated evidence. Existing TypeScript, focused browser-contract, documentation, exact-search, and diff gates passed. No fixture, publisher, receipt, generated artifact, browser feature, or permanent test was added. The shared Corpus row remains open for final residual classification and independent record review.

- Completed `WP-SD1-A-TERM-01-COR4`: the sole current Cookbook phrase now reads `four-question Chapter 1 Pilot Question Set`. Markdown/ASCII/source-line, contextual-search, and diff gates passed. This documentation-only cutover changes no schema, API, source code, generated contract, fixture, test, behavior, or feature. The shared replacement ledger's pre-existing Prettier drift is recorded in the package receipt. COR3 was subsequently accepted; the broader Corpus migration/checklist row remains open for final residual classification.

- Completed `WP-SD1-A-TERM-01-COR2`: two Determinism Contract references and three active native/browser-Wasm test labels now use the answer-free Question Response Format Fixture Set and canonical `crates/wasm/ple_question_json_response_format_fixture_set.json` path. Rust changes were comment/test identifiers only; fixture bytes, path, JSON shape, test bodies and coverage, behavior, contracts, and generated output remain unchanged. Existing native/Wasm/Node/docs gates, formatting, contextual searches, and diff validation passed. No test or feature was added. COR3 and COR4 remain open, so the broader Corpus migration/checklist row remains open.

- Completed `WP-SD1-A-TERM-01-COR1`: the six allocated deterministic seed-vector reader, regenerator, caller, Question Model, and documentation files now use Deterministic Seed Vector Fixture Set / Question Generator Seed Vector Set and exact private identifiers. Fixture JSON bytes, data, shape, serialized fields, behavior, API/wire format, and test coverage remain unchanged. Existing native determinism tests passed; the Wasm32-only host invocation compiled with zero host tests. Rust formatting, exact retired-name/scoped-wording search, and diff validation passed. No test or feature was added. COR2 and COR3 remain open, so the broader Corpus migration/checklist row remains open.

- Completed `WP-SD1-A-TERM-01-GRP1`: five audited documentation areas now use exact Course Membership, Active Student Course Membership, Student Accommodation, and explicitly labeled retired-wire names. No live PLE Group domain remains in that scope; retained technical, scientific, and ordinary uses remain exact. Markdown, ASCII, source-line, Prettier, contextual-search, and diff gates passed. No code, schema, API, generated contract, fixture, test, or feature changed.

- Completed `WP-SD1-A-TERM-01-QP1A`: current screenshot capture, Enrollment Design, reusable-course, renderer-controller, architecture, NEWS, and active-plan language now assigns each fact to Capture Manifest/Publication Receipt, Assignment Access evidence/Course Membership history, Course Origin/Assignment Source Record, or digest-qualified renderer OCI identity. Independent review and existing Markdown/link/ASCII/source-line/guidance/pyflakes checks (2,336), Prettier, contextual search, and diff validation passed. Remaining provenance matches are individually justified technical or frozen historical evidence. No code, schema, API, generated contract, wire, behavior, fixture, test, or feature changed; rows 311, 312, and 632 are complete while row 636 remains checked.

- Completed `WP-SD1-A-TERM-01-QLC1`: this terminology-only correction updates existing Rust module documentation, contracts, and active-workstream plans with one exact Question Library module ID, declared/browser contract routes and scope, Question Details, Question Revision, Question Publication, and the schema's exact `question_revision` Job Target. The non-existent `catalog.rs` ownership claim is removed. No server Question Library route is mounted; server route mounting remains future work. Independent implementation re-review, Markdown/source-line checks, direct `2026082925` schema comparison, Prettier, exact active-name searches, and diff validation passed. Remaining catalog matches are technical PostgreSQL inventory, PDF, dependency, audit, release-probe, or historical evidence. No behavior, schema, API, generated contract, fixture, test, or feature changed.

- Re-audited every currently checked vocabulary-replacement row against source, schemas, generated
  contracts, browser models, tests, active plans, and accepted package evidence. The audit reopened
  Course provenance, Assignment provenance, and renderer provenance for QP1A; QP1A subsequently
  completed and closed those rows. No additional completed row required reopening. Corrected the
  stale row-414 note so it recognizes the accepted broader Broker correction in PAO1, PAO2, and row 415. The checklist is 295 checked and 181 open at the audit snapshot.
- Completed `WP-SD1-A-TERM-01-QF1`: removed the unreachable optional Question Picker collection hook and unused CSS. Question Picker fallback/source copy now uses Question Folders and Question Library; browser-scenario resource/evidence labels and exact model/docs descriptions name their actual owners; the real Question Curation folder workflow remains. No schema, API, wire, behavior, fixture, or test changed. TypeScript, 24 focused Node tests, Question Model tests, browser-scenario registry, formatting, Markdown/link/source-line, exact-search, and diff gates passed. Question Folder Share remains separately open.
- Completed `WP-SD1-A-TERM-01-FAM1`: four source/test files and current documentation now use exact Question Type, response-item, Route Surface, data-category, Object Address, ownership, and producer-reader boundaries instead of PLE-owned family meaning. Remaining family matches are technical or historical with individually reviewed local meanings. Independent source/docs reviews and the final-tree full aggregate passed: 467 generated declarations, 3 fixtures, Rust checks/Clippy/tests/doctests/Wasm, five frontend checks and 340 Node tests, 4,983 Python tests, PostgreSQL 17/iMathAS, PostgreSQL-plus-MinIO, and complete live acceptance. Three opt-in iMathAS loopback tests remain ignored; connected Playwright was not run. No behavior, schema, API, generated source, fixture, new test, or feature changed; rows 231, 598, and 603 remain open and row 233 is closed.
- Completed `WP-SD1-A-TERM-01-SLS1`: container environment/Compose terminology now names the Local Stack Controller and Service Login Setup. The syntactically valid but generator-impossible `service-login-setup-required` Compose credential sentinel remains fail-closed; lifecycle rotates the login and writes its private URL after migration and before API startup. Existing focused process-login/lifecycle/local-stack-controller tests (80), Compose configuration, ASCII/search/diff checks passed. No behavior, secret, authority, API, schema, generated contract, fixture, or new test changed; row 420 remains independently checked.
- Completed `WP-SD1-A-TERM-01-H5P2`: `FILE_STRUCTURE` now agrees with the closed PLE, QTI, iMathAS, and WeBWorK Question Backend set plus H5P Package Import, matching current architecture, contracts, input, adapter, security, decision, and Question Backend/Locator boundaries. H5P remains `h5p` Question Format and H5P Package Import only; customer-spec historical discovery input retains historical wording. Independent review found no active current contradiction. Documentation Prettier, 2,220 Markdown/ASCII/source-line checks, 999-line caps, and diff checks passed. No source, schema, API, test, fixture, or feature changed.
- Completed `WP-SD1-A-TERM-01-PAO2`: nine operational documents, four active plans, and current operative status now use exact protected-operation, Authenticated Session Resolution, Job lifecycle, and iMathAS/WeBWorK Question Backend execution terms. MOD-WORKER is honestly planned/no current runtime, and connected accessibility evidence remains unrun. The final independent audit found zero Broker matches in source/schema/tests/generated; retained documentation is exact Message broker definition, checklist/audit evidence, explicit historical identifiers, immutable history, or package receipts. PAO1 supplies the predecessor source/schema and PostgreSQL evidence. This documentation-only correction adds no code, schema, API, test, fixture, or feature; row 415 is complete.
- Completed `WP-SD1-A-TERM-01-PAO1`: the first generic Broker dependency slice removes four source/schema aliases and six security/authorization-document aliases in favor of exact Session, private-owner policy, Sysadmin Account Creation, authorization-function, retention-Job, and Job claim-and-lease names. Functions, roles, predicates, and security behavior remain unchanged. Focused LDA formatting/check/strict Clippy/35 tests, SQL line tests, independent reviews, documentation gates, and the final-tree staged PostgreSQL 17 fresh/no-op/catalog/restricted-login/RLS plus 2/2 iMathAS Session tests passed. PAO2 retains architecture/current active-plan documentation and final row-415 closure; no compatibility path, generated contract, API, fixture, new test, or feature was added.
- Completed `WP-SD1-A-TERM-01-AB1`: current database structure, architecture, and authorization documentation now use Authorization Checks, Authenticated Session Resolution, and Credential Authentication Completion for existing database operations. Credential Authentication Completion remains baseline-present and application-unmounted; generic Broker work remains open. Independent review, documentation gates, exact searches, and the final-tree staged PostgreSQL fresh apply/no-op/catalog/restricted-login probes plus 2/2 iMathAS Session PostgreSQL tests passed. No code, schema, API, test, fixture, or feature changed.
- Synchronized shared style guides, tests, and repository support files from the starter template.

## 2026-09-01

### Fixes and Maintenance

- Completed `WP-SD1-A-TERM-01-APS1`: the direct domain/private-helper rename now uses Assignment Policy Source while preserving the richer authorized source, identity-free Assignment Policy Source Kind, membership privacy, and wire-owned `source` field. Focused 22 Rust and 14 Node tests, TypeScript, formatting, exact-search, and diff gates passed. No schema, generated contract, wire, fixture, feature, or test was added.
- Completed `WP-SD1-A-TERM-01-QP1`: current documentation assigns Question Source bytes, source identity/checksums, Question Authorship/Question Citation, Question Fork Source, Question License, attempt reproduction details, and QTI checksums to their exact existing owners; the adjacent source-family wording is now Question Format. The independent review's current Question-facing residuals were repaired. Markdown-link, ASCII, source-line, contextual-search, Prettier, and diff gates passed. No code, schema, API, fixture, or test changed.
- Shared final-tree aggregate for `WP-SD1-A-TERM-01-APS1` and `WP-SD1-A-TERM-01-QP1`: `source source_me.sh && ./all_test.sh` generated 467 TypeScript types and validated 3 tracked fixture contracts; passed Rust formatting, default and all-target/all-feature checks, strict production/test/all-feature Clippy, workspace/all-feature tests and doctests, and Wasm; all 5 frontend checks and 340/340 Node tests; 4,983/4,983 Python tests; PostgreSQL 17 fresh apply/no-op/catalog/restricted-login/iMathAS Session tests; PostgreSQL-plus-MinIO Course Appearance conformance; and complete live acceptance. Three opt-in iMathAS loopback HTTP tests were ignored; connected Playwright is outside this gate. This aggregate validates the final tree without broadening either package's narrow correction scope.
- Completed `WP-SD1-A-TERM-01-QA1`: fresh Question Authorship schema now bounds ordered positions to 1--16, requires trimmed control-free reviewed display names, and requires contiguous positions before publication; nullable Account credit and separate Question Owner authority remain exact. Existing focused Rust/browser tests and the live integrated PostgreSQL apply/no-op/catalog/restricted-login/iMathAS oracle passed. No new test file, fixture, compatibility path, or feature was added.
- Completed `WP-SD1-A-TERM-01-QC1A`: `QtiPleDefault::EmptyTaxonomy` is now `EmptyQuestionClassifications`, and the README names Question Classification. Focused QTI, formatting, Markdown-link, ASCII, and diff gates passed. Controlled Question Subject/Subsubject, persistence, authoring, and import/export work remains open. No new test file, fixture, compatibility path, or feature was added.
- Completed `WP-SD1-A-TERM-01-H5P1`: H5P is Question Format/package-import-only, not a Question Backend or locator; the closed Question Backend set is PLE, WeBWorK, QTI, and iMathAS. Focused Rust/browser/generated-contract checks and the shared live integrated PostgreSQL apply/no-op/catalog/restricted-login/iMathAS oracle passed. No new test file, fixture, compatibility path, or feature was added.
- Final aggregate for `WP-SD1-A-TERM-01-QA1`, `WP-SD1-A-TERM-01-QC1A`, and `WP-SD1-A-TERM-01-H5P1`: `source source_me.sh && ./all_test.sh` on the corrected final tree generated 467 Rust-owned TypeScript declarations; validated 3 tracked fixtures; passed Rust formatting, default and all-target/all-feature checks, production/test/all-feature strict Clippy, workspace tests and doctests, and browser Wasm; TypeScript typecheck/lint, ESLint, Prettier, and 340 Node tests; 4,983 Python tests; the disposable PostgreSQL 17 fresh apply/no-op/catalog/restricted-login and iMathAS Session PostgreSQL tests; PostgreSQL-plus-MinIO Course Appearance coherence; and complete live acceptance. The three established opt-in iMathAS loopback HTTP tests remained ignored, and connected Playwright was not run by this gate.
- Completed `WP-SD1-A-TERM-01-AAT1`: Assignment Attempt now names the whole Student pass, history, count, resume, and detail through Rust/Serde, regenerated contracts, strict browser decoding, route/CSS ownership, scenarios/helpers, current schema, the Rust-Serde-owned `assignment_attempts` fixture, current documentation, and active plans; Question Attempt remains the narrower issued-question record. Fresh migration `2026082923_item_course_analysis.sql` was amended directly with `completed_assignment_attempt_count` and `in_progress_assignment_attempt_count`. Retained former route/page/API/label strings are explicitly historical evidence. The direct pre-production correction adds no compatibility alias, feature, fixture, or permanent test. The independent final audit and focused gates passed. The final aggregate generated contracts; validated three fixtures; passed Rust formatting, workspace checks, strict Clippy, workspace tests and doctests; frontend type/lint/format and 340 Node tests; 4,983 Python tests; the disposable PostgreSQL 17 schema/authority/persistence lane including the iMathAS Session oracle; and PostgreSQL-plus-MinIO Course Appearance coherence. Complete live acceptance is green; connected Playwright was not run.
- Completed vocabulary row 121: no active Assignment Run model remains; the whole Student pass,
  history, count, resume, and detail use Assignment Attempt. The final contextual audit retains
  `run` only for technical execution/runtime/WeBWorK meaning, ordinary prose, rejected retired-wire
  evidence, and dated history. The broad Run tracker remains a contextual review, not a raw-zero
  migration detector. No behavior, schema, route, fixture, or permanent test changed.
- Completed `WP-SD1-A-TERM-01-CT1`: `CourseTheme` now names the selected closed visual palette for one complete Course Appearance through the Question Model, presentation scope, public facade, regenerated TypeScript, strict decoder, and closed browser registry. The direct cutover retires `CourseThemeId` while preserving the `theme` JSON property, all fifteen stable kebab-case palette values, parsing/default behavior, registry order, and contrast behavior. `CourseTheme` is a visual palette selector rather than a database identity. Existing focused behavior tests, generation, TypeScript, formatting, documentation, exact-search, and diff gates passed. This terminology correction leaves the deferred Course Appearance Store, schema/current pointer, PostgreSQL migration, server route, authorization oracle, upload-promotion/cleanup, and mounted editor unchanged.
- Completed `WP-SD1-A-TERM-01-RRT1`: `RequestRetryToken`, `RequestRetryBinding`, and `MAX_REQUEST_RETRY_TOKEN_BYTES` now name the shared technical value and server-held binding for one repeated Instructor write request. The direct Question Model, generated-contract, and strict browser-client cutover preserves the `retry_token` wire field and closed `BlueprintOperationApplyIntent`. An authenticated Account, exact Request Checksum, and typed request/Receipt context bind the token; it grants no authority and no Blueprint is retried. This correction creates no route, persistence, or durable replay claim. Focused Rust (13), generation, TypeScript, Node (2), formatting, Markdown-link/source-line, exact-search, and diff gates passed.
- Completed `WP-SD1-A-TERM-01-SFAP1`: browser-local `Feedback`, `FeedbackPresentation`, and `FeedbackPanel` now name Student Feedback Availability, Student Feedback Presentation, and Student Feedback Panel through the exact `studentFeedback` attempt-state phase, component, styles, pages, and connected-browser helper. The browser client uses `StudentFeedbackReleaseResponse`, `decodeStudentFeedbackReleaseResponse`, and `releaseStudentFeedback` at the exact Student Feedback release path; its receipt remains `{ released: true }`. The generated and wire-owned `feedback` fields remain the `StudentFeedback` DTO contract. Choice, Correct, and Incorrect Feedback labels name the three Question Feedback forms in authoring, preview, released-panel, and policy copy. Human Guidance, the terminology contract, and Student Guide keep outcome and continuation independent of optional Question Feedback. TypeScript, focused Node (70), connected-browser selector update, Prettier, Markdown-link/source-line, exact-search, and diff gates passed; the real-stack browser suite remains unmounted. No schema, generated-contract, or permanent-test expansion was needed.
- Completed `WP-SD1-A-TERM-01-RQB2` after accepted RQB1: the direct pre-production naming cutover gives the iMathAS integration exact Session, Challenge, Authentication, Grading Context, Result Token, Result Exchange, Result, and Question Backend Transport names, including the `ImathasQuestionBackend` marker, and directly amends fresh migration `2026090102` without compatibility support. Existing lifecycle, submission-marker, relationship, procedure, browser-launch, security, and test behavior is preserved. The final LDA facade uses explicit public and crate-private exports, preserving the pre-cutover public API and visibility boundary. Generated 467 TypeScript declarations; adapter (13 pass; 3 established opt-in ignored), LDA (54 unit plus 1 integration pass and 2 environment-gated PostgreSQL tests), focused Node (51), TypeScript, Prettier, rustfmt, strict Clippy, live SD1 fresh-apply/no-op/catalog/restricted-login and both iMathAS PostgreSQL tests, least-privilege review, and complete `source source_me.sh && ./all_test.sh` (4,935 pytest cases plus live PostgreSQL/MinIO lanes) passed on the final material tree. This closes vocabulary rows 222, 224, 234, 517-518, and 530-535; historical ET* receipts below remain predecessor evidence rather than their current-row completion claim. The overall terminology program remains open.
- Completed `WP-SD1-A-TERM-01-SRF1`: the answer-free domain-owned `StudentResponseFormatCheck { issues }` and its thirteen exact `StudentResponseFormatIssue` variants now own the direct no-compatibility cutover through Wasm, one strict shared browser decoder, and Response Format Messages. The correction retires report/violation names, top-level `violations`, and `missingUploadReference`; independent audit findings were repaired and revalidated. Focused gates and complete `source source_me.sh && ./all_test.sh` acceptance passed. This no-schema slice leaves the unmounted key-free server fallback route as a separately allocated future boundary.
- Completed `WP-SD1-A-TERM-01-RQB1`: WeBWorK and iMathAS remain PLE-managed Question Backends. RQB1 established the server-managed iMathAS Session and Result Exchange boundary that RQB2 now gives exact concrete names. Question Model owns the exact `ImathasQuestionBackendBinding`; LDA persists it and the Session evidence; the iMathAS adapter owns Launch, Result verification, transport, and Render Cache records. The fresh schema removed duplicated deployment/item/profile wrappers and the orphaned LTI Grade Return without aliases. `ple_worker_login` can assume only the execute-only iMathAS grading-worker capability for exact claim/commit procedures and cannot assume the procedure owner or read protected tables directly. Fresh migration/no-op/catalog/restricted-login evidence, both PostgreSQL Store service tests, independent least-privilege review, and complete `source source_me.sh && ./all_test.sh` acceptance passed.
- Repaired the SD1 accepted-revision fixture order: private Question records are now seeded before Question Revision acceptance, so the existing exact `accepted_at` trigger invariant is tested against a real revision. This fixture repair does not change RLS or trigger behavior. Final `./all_test.sh` complete live acceptance passed.
- Completed `WN1-QM-PRESENTATION-COURSE-APPEARANCE-VIEW`: `CourseAppearanceView` now names the direct browser reader shape exactly `{ theme, revision, banner }`, with `CourseTheme`, `CourseAppearanceRevision`, and nullable `CourseBanner`. The Question Model/public facade, generated declaration, strict browser decoder, same-origin reader/client, fixtures, tests, and affected documentation retire the PLE-owned Course Appearance projection without aliases or dual DTOs. Strict decoding rejects invalid Course Theme values/revisions, surplus fields, retired banner `id`, and invalid alternative text; the reader sends no-store GETs and requires the matching strong revision ETag. Focused model (6), generated-output parity, TypeScript, Node (8), formatting, and diff gates passed, and independent final review passed. `CourseAppearance` retains its durable record meaning. This reader-only receipt defers the Course Appearance Store, current pointer/schema, PostgreSQL migration, route, authorization oracle, upload-promotion/cleanup, and mounted editor.
- Completed `WN1-QM-PRESENTATION-COURSE-BANNER-REFERENCE`: `CourseBannerReference` now names the opaque same-origin Course Banner identity, and the browser-safe `CourseBanner` reader serializes exactly as `{ reference, alternativeText }`. The direct cutover retired `CourseBannerId`, `CourseBannerPresentation`, and the `id` wire property through the model, object binding, generated API, strict decoder, delivery consumer, Course Entry identity, fixtures, and tests; the final-tree generator run wrote 461 types. Course Banner Alternative Text remains independent, and Course Appearance View is complete for its separate browser-reader scope. This receipt defers the Course Appearance Store, schema/current pointer, PostgreSQL migration, route, authorization, upload-promotion/cleanup, and mounted editor.
- Completed `WN1-QM-PRESENTATION-COURSE-BANNER-INFORMATIVE-TEXT`: Course Banner Alternative Text remains the closed Decorative-or-Informative policy, Course Banner Informative Text is the validated Informative-branch scalar, and `alternativeText` remains the JSON property. The direct pre-production cutover retired `CourseBannerAltText` from the Question Model facade and generated API; focused model (6 tests), generation, TypeScript, strict decoder/renderer (5 tests), formatting, and diff gates passed. This does not claim the deferred Course Appearance Store, current-revision schema, server route, authorization oracle, or mounted editor.
- Completed focused Question-boundary terminology corrections: browser-safe Question Presentation Token, closed Response Selection Rule decoding, canonical `/api/questions` and Assignment Question labels, and separate Question Feedback, Question Answer, and optional Question Answer Explanation fields through the trusted PLE grading evaluations. The focused Rust, browser, formatting, generated-contract, and full repository format/check/Clippy/Rust/Node/Pytest gates passed.
- Completed the iMathAS Question Backend Session authentication-state terminology correction: one HMAC codec binds the exact iMathAS Grading Context and iMathAS Session Challenge, canonical protected state persists only with its iMathAS Question Backend Session, and the iMathAS Result Exchange retains direct Session lineage with one-use forward transitions. The accepted-revision fixture-order repair allowed the dynamic PostgreSQL oracle to pass malformed-state, replay, backward-transition, and unrelated-grade cases; complete `./all_test.sh` live acceptance passed. The separate iMathAS Result terminology cutover was still open at that predecessor checkpoint.
- Completed `WP-SD1-A-TERM-01-ETLS1` and vocabulary row 531: LDA now owns the sole server-only iMathAS Question Backend Session, typed Reference, preparation/restore/lease/verified-Result-Exchange Store boundary, XChaCha20-Poly1305 iMathAS-state protection/key rotation, and Memory/PostgreSQL Stores. The iMathAS adapter owns only strict versioned Launch State bytes plus HMAC and iMathAS protocol validation; migration `2026090102` owns exact persistence, lifecycle, RLS, and least-privilege SECURITY DEFINER enforcement. Adapter (13) and LDA (42) evidence are current. The SolidJS shell POSTs a same-origin request, accepts only validated `{ launchUrl }`, and opens an iframe without Session, Challenge, or iMathAS secrets. Its LDA-backed Rust route, cookie/env production iMathAS composition, and live iMathAS acceptance remain absent; row 535 was still open at that predecessor checkpoint.
- Completed `WP-SD1-A-TERM-01-ETLC1` and vocabulary row 532: LDA solely owns the fresh OS-CSPRNG 256-bit iMathAS Session Challenge, all-zero retry, and validated private-storage reconstruction. One immutable Challenge belongs to one iMathAS Question Backend Session, expires with it, and is accepted once only by a verified iMathAS Result Exchange; iMathAS carries only signed `ple_launch_challenge`. Migration `2026090102` and its oracle prove direct `ple_api_owner` mutation fails. The browser shell has no Challenge DTO; its LDA-backed Rust route, cookie/env iMathAS composition, and live iMathAS acceptance remain absent. LDA, adapter, PostgreSQL, independent-review, and complete `./all_test.sh` evidence are accepted; row 535 was unchanged at that predecessor checkpoint.
- Completed `WP-SD1-A-TERM-01-ETGC1` and vocabulary row 534: LDA solely owns private/redacted/non-Serde iMathAS Grading Context `{ QuestionAttemptId, QuestionRevisionReference, QuestionSeed }` across the iMathAS Question Backend Session, Store, and adapter. Its accepted `authentication_payload_v1` bytes persist, and authority inherits through the Session and Question Attempt; it is distinct from the Qualified Launch Binding Digest, iMathAS Session Challenge, iMathAS Result Token, and iMathAS Result. The direct `question_attempt_id` cutover, four-axis mismatch/live evidence, and independent review are accepted. The browser shell has no Context DTO; row 535 was still open at that predecessor checkpoint.
- Completed `WP-SD1-A-TERM-01-ETPRT1` and vocabulary row 533: LDA solely owns the bounded opaque, redacted, non-Serde iMathAS Result Token and exact checksum. iMathAS verifies the server-to-server iMathAS response before deriving its checksum; one verified iMathAS Result Exchange persists `imathas_result_token_sha256` only in its atomic single-use consume transition. The direct `2026090102` fresh-schema cutover retires the Session pre-token and generic Result Exchange verification checksums. Focused LDA, adapter, PostgreSQL Store (2/2), independent re-review, and complete `./all_test.sh` live acceptance are green. Row 535, iMathAS Result plus Grading Result, was still open as the separate ordinary Question Submission grading follow-on at that predecessor checkpoint.
- Continued the Human-Guidance terminology alignment: completed the Question Library fixture and pagination naming cutover, named the shared PLE/Wasm JSON data the Question Response Format fixture set, and replaced PLE-facing materialization wording with the exact construction, generation, creation, or update operation. Focused Rust, Wasm, browser, Markdown, formatting, fixture, and diff checks passed where applicable.
- Rotated the complete 2026-08-31 history into `CHANGELOG-2026-08d.md`; the active changelog now begins with the current categorized maintenance block and stays below the source-file limit.
- Replaced obsolete PLE `ProblemSource`, `ProblemAsset`, and problem-version documentation with the current Question Source, Question Asset, Question Revision, and Question Revision public-model terminology. Upstream WeBWorK request keys remain adapter-scoped.
- Replaced the remaining PLE Question Type and Question Backend "family" descriptions in the PLE JSON test and active assessment/determinism contracts; platform, visual, external-format, and historical uses remain contextually exact.
- Renamed the private import-byte Object Address from generic `WorkspaceSource` to `WorkspaceImportSource`, including its serialized contract tag, object tests, and storage documentation. The separate `WorkspaceQuestionSource` continues to name authored private Question source bytes.
- Renamed the matching imported-asset Object Address from `WorkspaceAsset` to `WorkspaceImportAsset`, preserving the distinct `WorkspaceQuestionAsset` boundary for directly authored private assets.
- Renamed the Question Curation panel's local state, actions, and accessibility identifiers from generic Collection to exact Question Folder terminology; repository and browser behavior remain unchanged.
- Closed the retained Favorite vocabulary check: current product boundaries use Question Star and Starred Questions for endorsement, Question Folder for private organization, and Saved Question Search for retained criteria; remaining matches are authored or external content.
- Closed the My Question Drafts navigation-label check: Draft Question remains the exact private content identity, while current Instructor navigation and browser contracts use the explicit My Question Drafts view.
- Replaced remaining authoring-route workspace recovery, status, save, preview, and accessibility copy with My Question Drafts or Draft Question; Authoring Workspace remains the underlying private relationship and route-reference concept.
- Replaced the mounted Curriculum product route, route identities, components, CSS ownership, links, and Ribbon copy with direct Blueprint Course terminology and `/blueprint-courses`. Removed the unimplemented Course Instance Curriculum changes link rather than presenting a false route; Blueprint Updates remains the separately tracked future surface.
- Replaced the ambiguous browser `CourseRouteData` transport composite with the exact `CourseRouteView` across its decoder, route-scoped query, context, pages, and contracts. The View contains one authorized Course Summary and Course Appearance for route readers; low-level database projection remains technical query vocabulary.
- Completed the retained generic Candidate audit: PLE boundary documentation and model comments now name direct records, selected Question Pool Items, authored choices, and delivery eligibility. Remaining matches are ordinary algorithmic, external-format, test, or authored-content language.
- Tightened the Hotspot contract wording: a current PLE Hotspot has named rectangular Hotspot Regions, and a Student Hotspot Selection identifies one selected region without carrying geometry. The browser control and JSON-format documentation now state the same model.
- Renamed the fresh SD1 `course_assignment_analysis` and `course_analysis_evidence` schema to exact Assignment Analysis and Assignment Analysis Receipt tables, including their parent keys, completed time, receipt, and Assignment Analysis Checksum fields. The real PostgreSQL apply/no-op/catalog/restricted-login acceptance lane passed.
- Replaced the duplicated `automated_grading_operation` lifecycle table with direct Question Submission Grading bound to one typed grading Job. Job State now owns execution; Question Submission Grading State, Grading Result, and Automated Grading Receipt own their exact records and checksum. The PostgreSQL oracle now proves the direct state, Job, and retired-table constraints.
- Replaced generic `course_object_metadata` and its scope with Course Object Reference and Object Checksum. Assignment Export and Course Object Delivery now bind the exact reference directly; the PostgreSQL oracle proves the retired table, generic columns, and required checksum boundary are absent or present as designed.
- Replaced the browser roster-import `RosterImportRowStatus` contract with Course Roster Import Row Result and a direct `result` field. The strict decoder, safe reason pairing, selection guard, review-table label, and fixtures now share the same exact outcome boundary; the complete browser check passes.
- Made Object Delivery Access Event an actual audit decision: it now records the exact delivery, Account, required allowed-or-denied Access Decision, and access time. The ambiguous optional Course field is retired, and the fresh PostgreSQL oracle proves the closed decision constraint.
- Replaced the generic cache table with iMathAS Render Cache Entry. Its durable schema now distinguishes the Entry identity, iMathAS Question Backend Binding, Resource Digest, Payload Digest, encrypted payload, and expiry; the PostgreSQL oracle proves the retired table is absent.
- Completed the iMathAS Result Exchange State model: Verifying, Ready to Commit, Committed, Failed, and Cancelled now have closed state-owned lease, verified-result, terminal-time, and safe failure-code requirements. The Concurrency Contract and PostgreSQL oracle use the same lifecycle.
- Separated iMathAS Result from LMS delivery: an iMathAS Result Exchange now owns its result and checksum, while the then-planned LTI Grade Return bound a closed delivery state to one exact Question Attempt and Assignment Grade. The retired passback table is absent under the PostgreSQL oracle.
- Replaced generic deployment-key columns in private iMathAS Question Backend Session and iMathAS Result Exchange records with the typed iMathAS Question Backend Binding. The PostgreSQL oracle rejects the retired private column while iMathAS protocol keys remain scoped to the adapter.
- Replaced mutable Course Retention Plans and generic lifecycle evidence with immutable Course Retention Plan Revisions and Course Retention Events. Each event binds its exact action, typed Job result, checksum, and time to the same revision selected by the Job; the PostgreSQL oracle proves the retired tables and direct revision constraint.
- Replaced generic Assignment Export identity, state, and artifact kind with Assignment Export Reference, Assignment Export State, and Assignment Export Format. The typed Job now proves its exact Export, Course, and Assignment relationship, while only Job State carries Ready or Leased execution.
- Replaced the PLE-owned WebAuthn Ceremony record and Rust identity with Passkey Ceremony and `PasskeyCeremonyId`. WebAuthn remains protocol vocabulary only; the focused Rust tests and PostgreSQL oracle prove the clean database boundary.
