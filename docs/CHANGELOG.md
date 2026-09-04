# Changelog

## 2026-09-04

### Fixes and Maintenance

- Removed permanent-document dependencies on archived implementation, release, status, and
  wire-naming plans. Durable architecture, contract, roadmap, database, evidence, TODO, and
  changelog documents now own those references; dated reports and audits link to `docs/archive/`.
  Refreshed `CODE_ARCHITECTURE.md`, `FILE_STRUCTURE.md`, and the agent orientation to describe the
  durable/working/archive boundary. The focused Markdown-link gate passes all 196 documents.
- Completed the documentation-set refresh: rewrote the README and Cookbook around the executable
  seeded-session boundary; aligned Install, Usage, FAQ, input-format, roadmap, TODO, development,
  and troubleshooting guidance with the absence of current teaching routes; refreshed release,
  news, and related-project records; and retained a concise `AGENTS.md` that points only to durable
  authorities. Rotated the older 2026-09-02 changelog block to
  [CHANGELOG-2026-09c.md](CHANGELOG-2026-09c.md). Historical screenshots remain managed design
  reference because no live Compose project was running and the local Podman machine reported a
  lockfile-permission warning; no unsupported browser acceptance claim was added.

## 2026-09-03

### Additions and New Features

- Completed vocabulary row 263: Question Asset Reference is now the one complete logical asset/checksum pair at Question-content boundaries. The Question Model, PLE Question JSON source, QTI conversion, presentation codec, generated TypeScript, strict browser decoders, protected delivery routing, and rendering use `questionAsset`; the strict PLE source reader rejects the retired `asset` member. Private QTI worker storage and technical asset-route wording remain distinct. Focused Rust (247) and browser (34) gates, TypeScript, formatting, contextual residual, and diff checks pass without changing a shared fixture. The 2026-09-04 independent [vocabulary final audit](archive/vocabulary_final_audit_candidate_2026-09-04.md) records PASS for all 417 replacement rows after operative plan reconciliation, contextual review, and a fresh final-tree aggregate.
- Completed vocabulary row 273: PLE Question JSON is the sole current internal static-Question format across adapter/compiler, source schema, persistence evidence, generated contracts, strict decoder, authoring/editor, pilot content, fixtures, and tests. The retired numeric/layout names and media type are absent from mutable PLE-owned surfaces; only Human Guidance's read-only descriptive product-intent phrase and superseded changelog history remain. Focused adapter (9 unit, 6 doctest) and authoring/editor (30) tests pass without fixture changes.
- Re-audited open course-accountability row 453: schema and deferred constraints already preserve one Assigned Instructor, while the shared authorization predicate keeps every current Teaching Team Member equal. The missing scope is an authorized Store/Server Route reader; no fictional Course Owner role or browser field was introduced.
- Re-audited open Source Binding rows 262 and 325 against the executable fresh baseline after a reviewer identified a former mixed-registration concern. `question_source_registration`, `source_backend`, and the mixed ownership table are absent from migrations, current source, generated contracts, tests, and operative documentation; retained changelog history is explicitly non-operative. Qualified Draft Question and Question Revision Source Bindings are direct baseline ownership. The rows remain open only for separately unimplemented QSOM1 publication, cleanup, search, route, browser, and final-acceptance scope.
- Completed vocabulary row 489 after a repository-wide audit: no current PLE Retry Token or idempotency-key contract remains. Repeated operations use their existing Question Attempt, Roster Import/revision, iMathAS Session/result, or Blueprint request/Receipt facts; the narrow remaining references are classified technical vocabulary, read-only authority, or superseded history.
- Continued the terminology migration's direct PLE-domain cutover: Correction Generation work, Course Retention object deletion, Assignment Activity contract mapping, database authorization evidence, Question Statistics, and the fresh Account State baseline now name their exact constrained operation or Receipt instead of a generic repeat-operation abstraction. Active, Deactivated, and Closed replace the retired suspended state; Deactivation or Closure revokes current Authenticated Sessions. Standard HTTP/platform terminology and read-only authority remain deliberately classified rather than renamed.

- Completed `WP-SD1-A-TERM-01-QVAR1` and row 179: retired combined Variation Policy state is absent across current code, schema, API, generated contracts, interfaces, fixtures, and tests. Question Pool Reuse Rule and Assignment Question Variation Rule independently cross Assignment, released/issued snapshots, PostgreSQL evidence, strict browser controls, Student presentation, and all four focused combinations. No Instructor-selected exact-variation feature exists. Completed row 510: Question Statistics Eligibility is the server-derived, frozen Issued Question fact through Rust, fresh PostgreSQL schema/procedures/catalog, strict browser wire, and the existing serialized fixture; focused contracts and the PostgreSQL 17 acceptance lane pass without an alias, new fixture, or feature. Corrected the unsupported global Question Statistics release shape: global difficulty/discrimination/duration/attempt metrics, availability filter/facet, aggregate machinery, and browser presentation are removed; exact accepted-grade counts remain private, unavailable is the only current public state, and the prior row 327 checkbox is reopened. Completed row 501: `dropped_assignment_grades` / `droppedAssignmentGrades` replaces the generic ID wording through the pure Course Grade calculator, wire, generated contract, strict decoder, and existing focused test; no Course Grade persistence, route, schema, fixture, or feature was invented.
- Completed `WP-SD1-A-TERM-01-BA1` and vocabulary row 565: Blueprint Assignment replaces all seven current reusable-assignment phrases, and the Assignment Editor's separate saved-Course-Assignment source now owns the exact `RetainedAssignmentQuestionSource` type instead of masquerading as a `BlueprintAssignment`. Focused and aggregate gates pass; the three tracked fixtures are unchanged, with no schema, API wire, route, test behavior, compatibility path, or feature added. Implemented acceptance-open `WP-SD1-A-TERM-01-BMOD1`: direct Blueprint Module vocabulary cutover replaces `BlueprintCourseModuleView`, `BlueprintModuleId`, `module_id`, and both edit-handle types with Blueprint Module View/Reference/Edit Choice and Blueprint Assignment Edit Choice across the Question Model, generated contracts, strict browser decoder, editor, and focused tests. Rows 566, 568, and 569 remain open because the complete immutable Blueprint Revision Content Store/Server Route does not exist; no fixture or compatibility path was added. Completed `WP-SD1-A-TERM-01-BAREF1` and vocabulary row 460: the direct `BlueprintAssignmentReference` / `blueprint_assignment_reference` cutover now covers the Question Model, Blueprint-operation records, generated contract, strict decoder, editor/picker consumers, and focused tests; ordinary Course Assignment IDs remain separate private record identities, and no fixture or compatibility path was added. Completed `WP-SD1-A-TERM-01-AAR1` and vocabulary row 457: removed the dead browser-only `StartedAssignmentAttemptId` alias; current state, private record ID, and the `R-` Assignment Attempt Reference remain separate. `WP-SD1-A-TERM-01-NOI1` removes the unsupported Question Submission, Course Roster, and iMathAS Result Exchange Retry Token expansion: repeated requests now use the existing Question Attempt, Roster Import/revision, and iMathAS Session/result identities, returning an existing result or a conflict. Focused browser (52), LDA (33), adapter (8), TypeScript, documentation/SQL (1,216), and PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS-service (3) gates pass; no fixture changed. Completed `WP-SD1-A-TERM-01-RRT2` and vocabulary row 483: Blueprint `RequestRetryToken` and `RequestRetryBinding` are removed across reservations, apply records, commands, receipts, browser intent/decoder/client, and generated contracts. The exact Account, Request Checksum, reservation/target, revision/source, and Receipt facts remain; no Blueprint Store/Server Route demonstrates a separate-token need. Focused Question Model (146), browser (3), TypeScript, generation, residual, formatting, and diff gates pass. Completed `WP-SD1-A-TERM-01-RRT3` and rows 481--482: the unsupported Instructor Grading Retry Token, replay registry, test-only Store, receipt field, generated contract, strict transport/header, and page UUID are removed. Exact operation/action/revision/Request Checksum/Receipt facts remain; no Store or Server Route demonstrates a dedicated-token need. Focused Rust, browser API, LDA, browser (8), TypeScript, generation, residual, formatting, and diff gates pass. Completed `WP-SD1-A-TERM-01-QVAR1` and row 474: the retired selected-problem-variant aggregate is absent across current code, schema, API, generated contracts, interfaces, fixtures, and tests. Question Pool Reuse Rule and Assignment Question Variation Rule separately control later-Attempt selection and issued variations; there is no Instructor-selected exact-variation feature. Focused model/browser, TypeScript, residual, documentation, formatting, and diff gates pass.
- Completed `WP-SD1-A-TERM-01-ICI1` and vocabulary row 451: the sole current live-delivery plan now uses Instructor Course Invitation, and the Terminology Contract distinguishes it from a general Course Invitation by exact target, Instructor Course Membership Role, acceptance-only Teaching Team outcome, and non-membership-change boundary. No fixture, schema, API, route, or behavior changed.
- Completed `WP-SD1-A-TERM-01-AWO1` and vocabulary row 444: the fresh schema column, current-session predicates, RLS policies, dependent private-authoring operations, documentation, catalog oracle, and SQL fixture now use Authoring Workspace Owner while retaining Workspace Collaborator as a separate relationship. PostgreSQL 17 and aggregate gates pass; authorization behavior is unchanged and no compatibility path, fixture, route, Browser Surface, or feature was added.
- Completed `WP-SD1-A-TERM-01-ALPHA1` and vocabulary row 573: Blueprint Course is the sole PLE reusable source-course identity across code, schema, contracts, Browser Surfaces, fixtures, tests, and active plans. Alpha Course remains only as attributed LibreTexts ADAPT prior-art vocabulary; focused and full aggregate gates pass with no behavior, fixture, schema, route, or feature change.
- Completed `WP-SD1-A-TERM-01-QO1` and vocabulary row 442: `owning instructor` is now exactly Question Owner. Immutable Question Ownership Events form a repeatable ordered chain; only the current owner records an accepted transfer to an Active Instructor Account.
  Question Authorship remains separate, new-lineage publication derives its owner server-side, and no browser contract exposes owner identity.
  The Question Library rechecks Account State and stays visible to every Active Instructor Account regardless of ownership.
  PostgreSQL 17 proves invalid-recorder/inactive-target refusal, two transfers, current-owner derivation, shared visibility, and non-active exclusion.
  Final aggregate: 422 contracts, 3 fixtures, Rust/Wasm, 288 Node, 4,850 Python, PostgreSQL 17, and PostgreSQL-plus-MinIO pass; no route or Browser Surface was added.
- Completed `WP-SD1-A-TERM-01-QSB1`: the fresh schema directly creates qualified Draft Question and Question Revision Source Bindings; RLS, Object Record validation, Bind Question Source, publication validation, and iMathAS resolution use them without a mixed-table copy/drop bridge. The metadata-only 2026090301 migration was renamed, retired-name inventory assertions were deleted, no fixture was added or changed, and full aggregate acceptance passes. Rows 262 and 325 remain open for remaining QSOM1 work.
- Completed `WP-SD1-A-TERM-01-SLWS1` and vocabulary row 186: Question Model now solely owns `StudentLateWorkStatus` (On Time, Accepted Late, Marked Late), Domain re-exports it, and `student_late_work_status` crosses the decision, delivery, generated contract, decoder, and Student presentation. Late Work Refused remains a separate access denial; focused and full aggregate gates pass with no fixture, schema, route, or feature added.
- Completed `WP-SD1-A-TERM-01-QANS1` and vocabulary row 286: the trusted PLE grader now names its `QuestionAnswer` builder and uses accepted-response wording for display-ready content; the authoring HCI brief names its local preview as an Answer Key and Question Feedback check. Exact QTI and private grading/correctness language remains; focused and full aggregate gates pass with no fixture, schema, route, or feature added.
- Completed `WP-SD1-A-TERM-01-SAV2`: reopened vocabulary row 707 after application-availability mount jargon reappeared, then corrected current source, product docs, contracts, architecture, security prose, plans, and workstreams. The fresh current-state detector retains seven physical-storage or direct-verification matches only; focused and full aggregate gates pass and row 707 is checked again.
- Completed `WP-SD1-A-TERM-01-RFM1` and vocabulary row 288: browser-local `responseFormatMessage` functions now derive the visible correctness-neutral message from the exact Student Response Format Check and Issues. Question Hint and Question Feedback remain distinct; focused and full aggregate gates pass with no fixture, schema, route, or feature added.
- Completed `WP-SD1-A-TERM-01-SRI1` and vocabulary row 285: the retained future inspection browser contract now uses exact `studentResponseInspection` and `studentResponseInspectionFeedback` members; Domain, generated contracts, strict decoding, tests, and visible privacy copy explicitly separate Student Response, permitted correctness/score, Question Answer, Question Answer Explanation, Answer Key, and Question Grading Input. Focused and full aggregate gates pass; no fixture, schema, Server Route, Browser Surface, or feature was added.
- Completed `WP-SD1-A-TERM-01-BCO1` and vocabulary row 443, replacing the residual generic `owning instructor` meaning with the exact Blueprint Course Owner relationship. The sole durable
  owner field is `blueprint_course.blueprint_course_owner_account_id`; publication, availability, and Draft Blueprint Revision collaboration transitions authorize that relationship. The
  PostgreSQL 17 oracle proves that another Instructor cannot act as owner while the exact Blueprint Course Owner can complete each transition. The database diagnostic no longer carries the retired
  Instructor Approval model. Rust and generated TypeScript expose `BlueprintCourseReadAccess`; the strict decoder and Blueprint Course workspace use `blueprint_course_owner` or
  `active_instructor`, and the existing hostile fixture rejects generic `access: "owner"`.
  The final aggregate generated 422 contracts, validated 3 tracked fixtures, passed Rust
  formatting/checks/strict Clippy/tests/doctests/Wasm, 288 Node tests, 4,850 Python tests,
  PostgreSQL 17 fresh/no-op/catalog/restricted-login with 3 iMathAS Store tests, and the
  PostgreSQL-plus-MinIO course-appearance oracle. No route, Store, schema relationship,
  compatibility alias, Browser Surface, fixture family, or feature was added.
- Completed `WP-SD1-A-TERM-01-BRI1` and vocabulary row 567 with a direct Blueprint Revision
  identity cutover. The private UUID remains only on the stable Blueprint Course record, which now
  also owns its bounded `BP-` reference number. PostgreSQL identifies each immutable Blueprint
  Revision only by the composite Blueprint Course Reference number and positive Blueprint Revision
  Number. Course Instance, Course Origin, Assignment source, publication, availability, and
  collaboration records carry that same foreign-key pair. The catalog oracle requires the exact
  columns and primary-key order and rejects the retired revision UUID/parallel Blueprint Course UUID
  identity, an invalid Blueprint Course Reference number, and a nonexistent Blueprint Revision
  Number. Rust, generated TypeScript, strict browser decoding, fixtures, and interfaces were already
  canonical. The final-tree aggregate generated 422 contracts, validated 3 tracked fixtures, passed
  Rust formatting/checks/strict Clippy/tests/doctests/Wasm, 288 Node tests, 4,850 Python tests,
  PostgreSQL 17 fresh/no-op/catalog/restricted-login with 3 iMathAS Store tests, and the
  PostgreSQL-plus-MinIO course-appearance oracle. No compatibility column, backfill, route, Store
  operation, Browser Surface, or feature was added.
- Completed `WP-SD1-A-TERM-01-BRR1` as a Blueprint result/Receipt-boundary prerequisite. The
  Blueprint operations HTTP client now strictly reconstructs all six generated completion-result
  variants with canonical public-reference validators and rejects unknown nested fields, including
  an invented `replay` product state. The valid browser fixture agrees with the Rust-owned generated
  contract, while operation-specific server-held Receipts remain non-Serde. Focused Blueprint
  Question Model (20), frontend (288 Node), TypeScript, formatting, and documentation/source gates
  pass. Final aggregate acceptance generated 422 contracts, validated 3 fixtures, and passed
  Rust/Wasm, 288 Node, 4,850 Python, PostgreSQL 17, and PostgreSQL-plus-MinIO gates. Vocabulary row
  596 remains open until an implemented durable Blueprint operation Store and Server Route return the
  same accepted exact Receipt for the same Account, Request Checksum, reservation, and revision/source
  facts. No schema, wire member,
  compatibility reader, Store, Server Route, or feature was added.
- Completed `WP-SD1-A-TERM-01-QD1`, enforcing the canonical required, non-whitespace Question Title
  and Question Description boundaries in the shared strict browser decoder. The decoder consumes
  the generated Question Model's 512- and 4,000-Unicode-scalar limits rather than duplicating them,
  and a permanent browser test rejects oversized values in a real Published Question page contract. The
  Rust model, PLE Question JSON adapter, generated contract, browser boundary, fixtures, and current
  schema remain aligned. Focused frontend (287 Node tests), Question Model (146), PLE adapter (8),
  and documentation/source (2,432) gates pass. Vocabulary row 301 remains open because mounted
  Question Publication Validation and post-publication metadata editing without a new Question
  Revision are not implemented.
- Completed `WP-SD1-A-TERM-01-QPV1`, converging Question Publication Requirements,
  calculated Question Publication Validation, and Question Publication Issues without creating a
  validated lifecycle state. Generic `PublicationViolation` and `PublicationValidationReport`
  types are absent from current source and contracts. Append-only migration `2026090304` directly
  renames the remaining Question Change Proposal Revision `publication_validation` column and its
  check constraint to exact Question Publication Validation ownership. The PostgreSQL 17 oracle
  requires the canonical non-null JSONB column and bound constraint and rejects the predecessor.
  The unimplemented publication operation and Browser Surface remain QSOM work. Focused
  documentation (2,490), schema, PostgreSQL, aggregate, residual, and diff gates pass. Vocabulary
  row 339 is checked.
- Completed `WP-SD1-A-TERM-01-QVR1`, rejecting the redundant content-level
  `Question Variation Rule: Static` model after current-owner review. Static PLE Question JSON is
  one complete Question Source, and QTI's named static profiles convert accepted items to it without
  inventing a runtime rule. `AssignmentQuestionVariationRule` remains the distinct Assignment-owned
  Reuse Variation or New Variation decision for later Assignment Attempts. The Question Model guide
  now correctly states that this rule changes Question Variations and never redraws Question Pool
  Selections. Retired static-definition names have no active implementation or wire occurrence.
  Focused QTI (95), Question Model (146), documentation (2,432), aggregate, residual, and diff gates
  pass. Row 296 remains checked; later audit clarified that generators do not own variation rules.
- Completed `WP-SD1-A-TERM-01-QT1`, replacing generic Question `title` fields with the
  canonical Question Title across PLE-owned models, adapters, APIs, generated TypeScript,
  browser interfaces, authored PLE Question JSON, fixtures, tests, and current documentation.
  Rust uses `question_title`; serialized and browser contracts use `questionTitle`, with no
  compatibility reader for the retired wire member. Question Prompt remains the task, while
  external QTI/XML `title` attributes and exact Assignment, Course, Blueprint Course, and Grade
  Category titles remain distinct. The final aggregate generated 422 declarations, validated 3
  tracked fixtures, and passed Rust/Wasm, 286 Node, 4,850 Python, PostgreSQL 17, and
  PostgreSQL-plus-MinIO gates. Vocabulary row 321 is checked.
- Completed `WP-SD1-A-TERM-01-QSLR1`, converging Question Summary and Latest Question
  Revision across the Rust model, PostgreSQL projection, generated TypeScript contract, strict
  browser decoder, fixtures, tests, and current documentation. Question Summary now consistently
  names a stable Published Question lineage and carries the exact Question Revision Reference with
  the greatest accepted Question Revision Number; Question Revision Availability remains separate.
  Focused Rust Question Library tests (10), generated-contract regeneration (422 declarations),
  browser decoder tests (2), PostgreSQL 17 fresh/no-op/catalog/restricted-login acceptance including
  the greatest-accepted-revision oracle and 3 iMathAS Store tests, formatting, and diff validation
  pass. Later audit reopened row 317 for two Current Question Revision residuals.
- Corrected and revalidated `WP-SD1-A-TERM-01-SAV1` after external review reopened row 707. Active
  plans, status receipts, contract prose, checklist evidence, and a route-contract comment now use
  direct Server Route existence, Service implementation, and Browser Surface availability language.
  The final 82-match inventory contains 10 real technical mount operations, 71 immutable history,
  audit, or archive matches, and the ledger's one required legacy phrase. Full aggregate acceptance
  passes with 422 contracts, 3 fixtures, Rust/Wasm, 288 Node tests, 4,850 Python tests, PostgreSQL 17,
  and PostgreSQL-plus-MinIO; 2,488 documentation/source, residual, formatting, and diff gates pass.
  Vocabulary row 707 is checked. This correction changes no API, schema, wire contract, behavior,
  fixture, or feature.
- Implemented `WP-SD1-A-QSOM1-P2` as the server-only new-lineage Question
  Publication coordinator and added append-only
  `2026090303_qsom1_draft_publication_source_resolution.sql`. The exact
  Instructor-session-authorized source read rejects stale Draft Question edits and returns only the
  current complete Workspace Question Source Object Record; `ple_app` cannot execute its private
  implementation. The coordinator requires complete agreement between that record and object
  storage, copies the bytes to a fresh immutable Question Revision address, issues the human-facing
  Question ID from OS-CSPRNG entropy plus an HMAC-SHA-256 validation character, and invokes P1.
  `QuestionRevisionReason` now owns the trimmed, bounded, control-free Question Revision Reason
  invariant. Focused model, Learning Data Access, server, strict Clippy, source-hygiene, and
  PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS gates pass. The final-tree aggregate
  generated 422 Rust-owned TypeScript types, validated 3 tracked fixtures, passed Rust
  formatting/checks/strict Clippy/tests/doctests/Wasm, 286 Node tests, 4,850 Python tests,
  PostgreSQL 17 fresh/no-op/catalog/restricted-login with 3 iMathAS Store tests, and the
  PostgreSQL-plus-MinIO Course Appearance oracle; complete live acceptance is green. Independent
  review remains open, so P2 and parent QSOM1 are not accepted. No Server Route or Browser Surface
  exists; same-lineage publication, secret-file composition, orphan cleanup, Draft Question
  expiration, Question Search, and broader vocabulary convergence remain open. No vocabulary
  checkbox is changed: row 312 still requires durable reason history and comparison Views.
- Implemented `WP-SD1-A-QSOM1-P1` as the server-only new-lineage Question
  Publication Store and append-only `2026090302_qsom1_new_lineage_question_publication.sql`.
  After trusted bytes-first storage, one transaction rechecks the active Instructor, Authoring
  Workspace, exact Draft Question Edit Number, Draft Question Metadata, Source Binding, and source
  bytes before creating the complete first immutable Question Revision, its separate Published
  Question Metadata and Source Binding, credit, ownership, publication, and Available evidence.
  Reviewed author display names create no inferred Account relationship; a later exact Account-aware
  input may establish that optional relation without conflating credit and ownership.
  The PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS lane passes stale-edit,
  cross-workspace, non-string-author, complete-write, and post-Draft-deletion checks. The final-tree
  aggregate generated 421 Rust-owned TypeScript types, validated 3 tracked fixtures, passed Rust
  formatting/checks/strict Clippy/tests/doctests/Wasm, 286 Node tests, 4,850 Python tests, the
  PostgreSQL 17 fresh/no-op/catalog/restricted-login lane with 3 iMathAS Store tests, and the
  PostgreSQL-plus-MinIO Course Appearance oracle; complete live acceptance is green. Independent
  review remains open, so P1 and the parent QSOM1 are not accepted. No publication
  Server Route or Browser Surface exists; object-copy coordination, same-lineage publication,
  cleanup, Question Search, and ledger rows 261, 262, 320, 323-325, and 529 remain open.
- Accepted and completed `WP-SD1-A-QSOM1-M1` with the append-only
  `2026090301_qsom1_metadata_ownership.sql` migration. It separates Draft Question Metadata,
  Published Question Metadata, Draft Question Source Binding, and immutable
  Question Revision Source Binding; uses exact Object Address equality; and
  rewires Published Question projections and publication-event completeness.
  The retired mixed nullable-XOR table and inline mutable metadata are removed.
  Independent re-review passes after fresh PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS
  evidence and focused Learning Data Access Question Source tests (3 passed). Parent QSOM1,
  vocabulary rows, publication, Question Search, cleanup, routes, and browser work remain open.

### Fixes and Maintenance

- Replaced further generic repeat-operation wording with exact current-generation recalculation,
  already-complete deletion, Instructor support action, Question Source Binding, Question
  Submission/Receipt, and session no-op boundaries across source, schema comments, and contracts.
  The Retry Token audit remains open for its full repository scope.

- Corrected Question Model presentation prose so its authenticated-attempt resolution and
  Presentation Response Item Reference checksum inputs name Question Revision, Question Seed,
  Question Backend, Question Title, and Question Presentation Nonce directly.

- Corrected the PLE-owned cache and prefetch contract to use Question Seed and Question Revision
  Reference throughout, including the actual `QuestionRender { question_revision, question_seed,
object }` address. External fields and code identifiers remain exact.

- Corrected active plan, authorization, and Question Backend prose to use Question Revision and
  Question Revision Reference as product concepts, retaining actual backticked code identifiers
  unchanged. This documentation cutover does not alter publication behavior or add a feature.

- Corrected the QTI parser boundary comment to name the complete browser-visible
  Question Asset Reference rather than separately naming its asset ID and checksum. Internal
  worker/storage and delivery identities remain scoped; no asset service or fixture changed.

- Recorded the Question Metadata audit: the current Rust, generated API, and strict decoder own
  one six-field bounded grouping and keep source, identity, authority, lifecycle, Authorship, and
  Question Owner separate. The broader row remains open for its unimplemented publication/search
  and classification work.

- Recorded the Course Setup Ribbon audit: the UI authority defines Grade Settings and Appearance
  tasks, but the executable Ribbon contract and available Appearance task do not exist. The row
  remains open; no misleading relabel or Browser Surface was added.

- Reopened vocabulary row 489 for a fresh repository-wide Retry Token audit. Corrected current
  reservation, Question Submission, iMathAS Result Exchange, Blueprint client, and frontend
  architecture prose to state the exact identity/Receipt boundary rather than a generic
  idempotency mechanism. The same pass corrected Roster Import, determinism, failure recovery,
  replica, and data-contract documentation. No token, fixture, schema, route, behavior, or feature
  was added.

- Completed `QSLR2` and vocabulary row 317: Question Summary carries its exact Latest Question
  Revision across PostgreSQL, Rust, generated TypeScript, strict decoding, fixtures, tests, active
  plans, and durable documentation. Security and source comments now distinguish that immutable
  revision from current availability and the one-Question-Submission boundary. No fixture, schema,
  route, behavior, compatibility reader, or feature changed.

- Corrected mutable QTI documentation and active-plan status from "flat-question" to its exact
  import/export/archive interchange role. The two remaining occurrences are read-only authority
  references; the PLE Question JSON ledger row remains open for its separate full-scope migration.

- Corrected active Assignment Attempt and iMathAS plan/contract language: a matching repeated
  Question Submission returns its existing Receipt, a changed response conflicts, and an iMathAS
  repeat resolves through its existing Session and result identities. The plans no longer prescribe
  a generic idempotency or retry-token architecture; no implementation, fixture, route, or feature changed.

- Replaced the remaining active generic replay shorthand in Forced Question Correction, iMathAS
  acceptance, and Object Storage Repair planning with their exact operation boundaries. Database
  properties and explicitly superseded history remain documented where they name real evidence.

- Corrected the active assessment-payload contract to use the generated Question Presentation
  `questionRevision` and `question_seed` fields, and to name Question Seed, Question Source,
  Question Source Binding, and the one-Question-Submission boundary precisely. The only retained
  bare `seed` is explicitly the external WeBWorK renderer field; generic idempotency wording is
  absent. This documents current contracts without adding a Retry Token, fixture, route, or feature.

- Renamed the browser-local Assignment Attempt state field from generic `seed` to `questionSeed`.
  Its owner is the exact Question Seed that selects the issued Question Variation; the generated
  and strict-decoder `question_seed` wire spelling remains unchanged. Focused recovery tests and
  TypeScript compilation pass without a fixture, route, or behavior change.

- Renamed the durable `ObjectAddress::QuestionRender.seed` cache-key component to
  `question_seed`. WeBWorK and iMathAS cache constructors now name the same exact Question Seed;
  WeBWorK's PLE-owned cache and issue coordinator parameters now do as well. External protocol
  spellings remain unchanged. The iMathAS PLE-owned render and transport request fields now use
  `question_seed`, while its registered render payload retains `seed`. Workspace compilation and
  focused Object and adapter tests pass without changing object paths, fixtures, routes, or behavior.

- Corrected PLE-owned source comments to distinguish Question Seed from Question Pool Preview
  Nonce and server selection entropy. The Question Model suite (143) and strict Clippy pass;
  the retained bare `seed` assertion is hostile-wire rejection evidence.

- Corrected active assessment, concurrency, adapter, authorization, backend, and contract
  documentation to name Question Seed, Question Revision, Question Attempt, Student Record, and
  Question Backend at their exact boundaries. The grading contract now relies on one Submission per
  Question Attempt rather than a generic idempotency claim. No external protocol field or feature changed.

- Clarified the browser `QuestionSubmissionReceipt` as the receipt for the one accepted Question
  Submission on its exact Question Attempt, rather than an "idempotent" abstraction. Its current
  type and wire shape already use the natural Question Attempt identity.

- Corrected PLE Question JSON hotspot publication so it accepts and substitutes a complete
  `QuestionAssetReference`, replacing the logical asset identity and authored checksum atomically.
  The focused native adapter test proves the compiled Question Response Format carries precisely
  the replacement pair. This does not claim the separately open asset publication, rendering,
  export, or delivery Services, and adds no fixture.

- Corrected the audited terminology drift in rows 321 and 481--483. `CourseQuestionUse.title`
  now uses the distinct Course Title decoder rather than the Question Title validator. Operative
  contracts and active plans now use Question Attempt, Roster Import/revision, iMathAS Session/result,
  and Blueprint request facts instead of `Idempotency-Key`, `submission_idempotency`, or request-retry
  bindings; current source, schema, generated contracts, browser clients, and tests contain no such
  architecture. The Terminology Contract and Design Decisions continue to prefer existing operation
  identity and constraints. Blueprint Assignment Reference is clarified as a stable child-lineage key, while
  Blueprint Assignment Revision Reference binds it to one immutable revision snapshot.

- Rotated the complete 2026-09-01 history into `CHANGELOG-2026-09a.md`; the active changelog keeps
  the two newest date blocks and remains below the enforced 1,000-line source limit.

- Repaired the QSOM1 M1 PostgreSQL foundation review blockers before independent acceptance. Source Binding
  table constraints now allow only PLE/PLE Question JSON, WeBWorK/WeBWorK PG,
  and iMathAS/iMathAS Backend/Format pairs with their qualified routing facts.
  The retired Question Revision Source Registration helper is absent and the
  catalog oracle enforces that retired shape. The existing record oracle proves
  both Binding owners reject mismatched Backend/Format facts and hostile
  superset Object Addresses with SQLSTATE `23514`. The scoped PostgreSQL
  fresh/no-op/catalog/restricted/iMathAS acceptance lane passes. This repair is M1 evidence;
  it does not accept any remaining parent QSOM1 work.

### Decisions and Failures

- **Final closeout for `WP-SD1-A-QSOM1-S2B2`--`S2B7`.** This entry supersedes the earlier
  same-day current-state wording that left these completed source-model slices acceptance-open.
  The PLE Question JSON and WeBWorK adapters, iMathAS source/session-input removal, QTI import
  mapping with minimal H5P preservation, unbacked-editor and PLE Question JSON fallback removal,
  and final generic-root deletion are accepted and completed after independent review. The final
  `source source_me.sh && ./all_test.sh` exits 0 with 421 generated types, 3 tracked fixtures,
  Rust format/check/all-feature strict Clippy/tests/doctests/Wasm, 286 Node tests, 4,831 pytest
  tests, PostgreSQL 17 fresh/no-op/catalog/restricted plus 3 iMathAS tests, Course Appearance
  PostgreSQL-plus-MinIO, and `PASS: complete live acceptance is green.` Vocabulary rows 181, 182,
  and 275 are accepted for their exact source-model boundaries. Parent QSOM1 remains open only for
  separately owned publication, persistence, and cleanup work; its remaining vocabulary rows are
  not accepted by this closeout.

- **Current-state synchronization for `WP-SD1-A-QSOM1-S2B2`--`S2B7`.** The PLE Question JSON,
  WeBWorK, iMathAS, QTI-import, H5P-preservation, unbacked-editor removal, and final generic-root
  deletion slices are implemented and their independent reviews pass. The retired generic
  `DraftQuestionContent`, source-bearing generic Question Revision, `QuestionGradingRule`, and QTI
  runtime-dispatch claims are superseded. `/workspace` is an Instructor-gated **Planned My Question
  Drafts** destination, not a mounted editor; authorized authoring and publication server workflows
  remain unmounted. QSOM1 remains open for separately owned publication, persistence, and cleanup
  work, and final aggregate acceptance is pending.

- **Historical clarification of Published Question metadata mutability.** A stable Published Question lineage owns
  mutable discovery metadata such as Question Title and Question Description. Editing those values
  creates no Question Revision. A Question Revision instead preserves one immutable complete
  Question Source and its exact historical evidence. The terminology, lifecycle, identity,
  concurrency, retention, caching, data-classification, authorization, and active-plan documents now
  use that boundary consistently. M1 now provides parallel Draft Question and Published Question
  Metadata tables and separate Source Bindings; the parent QSOM1 work remains open for publication,
  Question Search, persistence completion, cleanup, routes, and browser acceptance.

- **Accepted and completed `WP-SD1-A-QSOM1-S2B1A` (backend evaluation and Assignment scoring).**
  Server-only, non-Serde `QuestionEvaluation { correct, normalized_credit }` is Question Backend
  evaluation; Assignment-owned GradingResult remains the scoring record. The direct iMathAS
  issued-score cut keeps authentication/Result lifecycle facts and QuestionAttemptId on the Session,
  while atomic commit locks the selected IssuedQuestion and resolves its point_value and scoring_rule.
  Two independent reviewers PASS, and the manager's final `source source_me.sh && ./all_test.sh`
  exits 0 with 424 generated types, 3 fixtures, Rust workspace/Clippy/tests/doctests/Wasm, 315 Node,
  4,908 pytest, fresh/no-op/catalog/restricted PostgreSQL, 3/3 iMathAS, Course Appearance
  PostgreSQL-plus-MinIO, and `PASS: complete live acceptance is green.` The durable blueprint_course
  schedule, student_work grading, and iMathAS catalog oracle splits are line-gate organization only,
  not product behavior. S2B2--S2B7, generic source structs, and final no-point QuestionGradingRule
  deletion remain open; parent QSOM1 remains open.

- **Accepted and completed `WP-SD1-A-QSOM1-S2A` (Assignment Entry Question
  Attempt controls).** Fixed and Question Pool Assignment Entries now own QuestionAttemptLimit and
  QuestionAttemptTimeLimit. Immutable Assignment Revision Entry snapshots carry nullable positive
  attempt limits/time-limit seconds, nullable nonnegative grace seconds, and paired time/grace.
  Assignment-wide BaseAssignmentPolicy attempt/time controls remain distinct. Backend evaluation
  remains distinct from AssignmentEntryScoringRule and AssignmentPointValue. Generic
  DraftQuestionContent and QuestionRevision still duplicate the prompt/response/attempt-control
  material and await the next source-model cut; QuestionGradingRule remains open for removal. Parent
  QSOM1 and row 528 remain open, with no generic source-model, exact Assignment Entry/policy
  ownership, generated/browser authoring, or publication completion claim. Independent review and
  focused PostgreSQL acceptance pass. The final manager `source source_me.sh && ./all_test.sh` exits
  0 with 424 generated types, 3 fixtures, Rust/workspace/Clippy/doctests/Wasm, 315 Node, 4,908
  pytest, PostgreSQL fresh/no-op/catalog/restricted/iMathAS, Course Appearance PostgreSQL-plus-
  MinIO, and `PASS: complete live acceptance is green.` The existing
  `test_assignment_workspace_content_conflict_client.mjs` and
  `test_assignment_workspace_questions.mjs` Node fixture files state explicit unlimited controls;
  the behavior-preserving Blueprint Course module split satisfies the code line gate.

- **Accepted and completed `WP-SD1-A-QSOM1-S1` (Question Source authority
  documentation).** The current object-backed boundary records Question Source as complete
  immutable format-specific bytes in object storage; Source Object Reference identifies those bytes
  and Source Object Checksum verifies them. Question Source Registration binds a Draft Question or
  Question Revision owner with exact Backend/Format and backend-specific routing. The fresh baseline
  and `2026082940` Object Record validation own the active authority directly. The generic
  registration Question Type/content checksum and universal generic Answer
  Key, Question Feedback, Question Answer Explanation, Question Grading Input, and Workspace Import
  Grading Input sidecars are absent. The terms remain legitimate backend-produced or policy-released
  runtime contracts, while private artifacts remain backend-specific. Vocabulary row 528 is reopened:
  generic split sidecars are not canonical. QSOM1 remains open for generic DraftQuestionContent and
  QuestionRevision prompt/response/QuestionGradingRule/QuestionAttemptLimit/QuestionAttemptTimeLimit
  material and its exact Assignment Entry/policy, consumer, generated-contract, browser, and
  publication migration. This documentation package makes no mounted publication, editor, Store,
  route, browser, generated-contract, or QSOM1-acceptance claim. Independent review passes and the
  manager terminal `source source_me.sh && ./all_test.sh` exits 0 with 424 generated types, 4,908
  pytest checks, Rust/workspace/browser/static gates, PostgreSQL fresh/no-op/catalog/restricted/
  iMathAS acceptance, and Course Appearance PostgreSQL-plus-MinIO acceptance.

- **Accepted `WP-SD1-A-DQM1` (mutable Draft Question cut).** One
  mutable Draft Question belongs to one Authoring Workspace, with a server-private Draft Question
  UUID and positive Draft Question Edit Number concurrency token. The active fresh schema, LDA
  boundary, and PostgreSQL oracle directly bind current Question Source Registration to that Draft
  Question and reject retained draft-revision ownership. Publication is still unmounted: its future
  operation must validate an exact Edit Number and create an immutable Question Revision identified
  by `QuestionRevisionReference { question_id, revision_number }`. Draft retention remains a
  separate policy decision. Independent schema/LDA and documentation/contracts reviews pass; the
  shared terminal gate below validates the final tree.

- **Accepted `WP-SD1-A-TERM-01-QSRC2` (Question Source Registration cut).** Question Source
  Registration binds immutable Question Source evidence, exact Question Backend/Format routing,
  and bounded metadata to either the current Draft Question or an immutable Question Revision. It
  has no surrogate UUID. Draft ownership is `draft_question_uuid` with a
  positive expected Draft Question Edit Number; published ownership is `(question_id,
revision_number)`; XOR and unique-owner constraints remain. The registrar returns void, accepts
  identical facts idempotently, and rejects stale, changed, or unauthorized facts. A future
  authorized publication operation atomically creates the complete Question Revision-owned Question
  Source Registration and aggregate; it remains unmounted. Generic Answer Key, Feedback, Hint,
  Explanation, and Grading Input
  records are not universal requirements. Focused Rust, PostgreSQL fresh/no-op/catalog/restricted
  acceptance, contextual residual, formatting, and hygiene evidence pass. Independent schema/LDA
  and documentation/contracts reviews pass. The manager terminal `source source_me.sh &&
./all_test.sh` passes Rust/generated/browser/documentation gates, 4,912 pytest checks,
  PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS, and Course Appearance
  PostgreSQL-plus-MinIO acceptance. QSOM1 remains open for the opaque-source, publication, and
  browser work not claimed by this persistence correction.

- Accepted and completed `WN1-TERM-LEARNING-REFERENCE-H0` as a documentation-only correction of
  vocabulary rows 447, 448, and 453. The proposed `AccountId`, Course,
  membership, Student Record, Assignment, Assignment Attempt, Issued Question,
  and Question Attempt `*Id` to `*Reference` migrations are rejected: opaque
  UUID-backed private record IDs and existing SQL `*_id` keys remain IDs.
  Reviewed prefixed public locators remain separate, scoped route values, and
  no Reference is invented for Student Record, Issued Question, or Question
  Attempt. No source, schema, generated output, route, test, detector, or
  migration allocation changes. Independent acceptance passes; rows 447, 448,
  and 453 remain checked as the resolved ID-versus-Reference ledger claims.

### Behavior or Interface Changes

- Completed `WP-SD1-A-TERM-01-604-CR2` and vocabulary row 604 with a direct Course Roster
  change-number browser cut. Course-bound local contracts now use generated
  `CourseRosterChangeNumber` as `rosterChangeNumber`: a canonical positive PostgreSQL-BIGINT
  decimal string. Course Roster pagination and aggregate actions, invitation email-rule and
  roster-import results, calculated Gradebook pages, submitted Assignment Attempt chooser pages,
  and Gradebook continuation checks use that exact field. `If-Match` and strong ETag validation
  use the exact quoted decimal. The strict readers refuse `rosterRevision`, noncanonical, and
  out-of-range values. Independent completion review PASS confirms the final focused
  strict-decoder evidence rejects noncanonical `"092"` and the max-plus-one
  `"9223372036854775808"`. Course Invitation state preconditions, `importRevision`,
  scheme/scoring counters, CR1 generated `rosterChangeNumber`, routes, Store/schema behavior,
  and generated scalar ownership remain unchanged. Regeneration (424 declarations), focused Node
  (26), TypeScript, Rust, codebase, residual, and diff gates pass; no route, Store, schema,
  migration, fixture family, browser scenario, compatibility alias, or permanent test machinery
  was added.

- Completed `WP-SD1-A-TERM-01-603-CI1` as the final ordered row-603 child after independent CI1
  review passed and row 603 became closure eligible.
  `CourseInvitationStatePrecondition` replaces the remaining generic Course Invitation transport
  through Rust/Serde, generated declarations, strict decoder, teaching-operation client, API
  facade, and the Instructor/Pending invitation callers. Both invitation views serialize
  `state_precondition`; revoke, accept, and decline require that exact type for `If-Match`, while
  create retains its Location and strong-ETag validation. The retired generic Rust export and
  generated declaration are removed. AE1 `AssignmentEditNumber`, AC1R deletion and Scenario
  modifiers, CR1 `CourseRosterChangeNumber`, Product Role, route, Store, schema, migration,
  fixture, browser scenario, compatibility behavior, and permanent tests are unchanged. Focused
  Rust/Node/TypeScript, generation, formatting, strict Clippy, aggregate Rust/codebase,
  generic/residual, and diff gates pass. The final generic detector is zero and retired. Row 603
  now closes only the stable generic `TeachingOperationRevision` replacement with four exact
  outcomes: AE1 `AssignmentEditNumber`; rejected AC1 followed by AC1R mutation-surface removal
  with value-only Scenario modifiers; CR1 `CourseRosterChangeNumber`; and CI1
  `CourseInvitationStatePrecondition`. Product Role remains separately allocated and pending.
  Nonblocking coverage observation: the existing focused invitation Node suite covers creation
  decoding but has no dedicated HTTP transport assertion for invitation list/revoke/respond; this
  documentation-only closure adds no test.

- Completed `WP-SD1-A-TERM-01-603-CR1` with a direct pre-production Course Roster
  change-number cut. `CourseRosterChangeNumber` now owns the Instructor Memberships page
  `roster_change_number` and direct-Instructor-removal `If-Match` through Rust/Serde, generated
  declarations, strict decoder, teaching-operation client, API facade, and Teaching Team panel.
  The existing generic `TeachingOperationRevision` remains only for deferred Course Invitation
  flows; AE1 `AssignmentEditNumber`, AC1R deletion/Scenario modifiers, and the distinct local
  roster import/list client remain unchanged. A private shared canonical-positive-decimal validator
  preserves the PostgreSQL-BIGINT boundary without adding public generic transport vocabulary.
  No route, Store, schema, migration, fixture, browser scenario, compatibility alias, or permanent
  test was added. Focused Rust/Node/TypeScript, generation, formatting, strict Clippy, residual,
  and diff checks pass. Independent CR1 review passes; only the one-time scoped CR1 detector is
  retired. Row 603 remains open for CI1 and final generic retirement.

- Accepted and completed `WP-SD1-A-TERM-01-PR1` and vocabulary row 446. `ProductRole`/`product_role`/`productRole` directly replace the immutable global Account and Authenticated Session classification through fresh schema, Rust, generated contract, strict browser decoder, route gate, Live Demo selector, direct PostgreSQL oracle, and current documentation. Course Membership Role remains distinct, Authentication Email retains its local role-qualified integrity meaning, and authorization/RLS behavior is unchanged. Independent review and the exact-owner PR1 detector pass; the detector is retired. Final exact-tree acceptance generated 424 TypeScript declarations, validated 3 fixtures, passed Rust/TypeScript, 315 Node and 4,912 Python tests, PostgreSQL 17 fresh/no-op/catalog/restricted/iMathAS (3/3), and PostgreSQL-plus-MinIO (1/1) cleanup. No compatibility alias, new feature, fixture family, or permanent test was added.

- Rejected and superseded `WP-SD1-A-TERM-01-603-AC1`: its proposed Accommodation revision
  reference had no producer and incorrectly received an Assignment edit number at the unmounted
  Assignment Access gate. Completed `WP-SD1-A-TERM-01-603-AC1R` with the direct deletion of that
  unsupported mutation/preview route, browser surface, client/decoder, and AC1-only Question
  Model/generated plumbing. The value-only Student View Scenario modifier model and AE1
  `AssignmentEditNumber` remain; deferred roster/invitation generic uses remain. No route, Store,
  schema, migration, fixture, alias, replacement browser surface, or permanent test is added.
  Focused Rust/Node/TypeScript, formatting, strict Clippy, codebase, residual, and diff gates plus
  independent AC1R review pass. Only AC1R's removed-surface detectors are retired. Row 603 remains
  open; CR1 and CI1 are pending.

- Completed `WP-SD1-A-TERM-01-603-AE1` as the first sequential row-603 Assignment edit-number
  transport cut. It replaces the generic teaching-operation value only in Assignment workspace/edit,
  Instructor Preview Schedule, Question Pool Preview, and hypothetical/selected/returned Student
  View Scenario generated contracts, strict readers/writers, and direct consumers with existing
  `AssignmentEditNumber`. The existing Question Pool lower-camel field is `editNumber`; preview
  plane fields are `edit_number`. Accommodation, roster, invitation, and generic response uses
  remain for AC1, CR1, and CI1. This direct pre-production cut adds no route, Store, schema,
  migration, fixture family, compatibility alias, or permanent test. Focused Question Model
  (11 + 2 + 1), Node (7), TypeScript, formatting, strict Clippy, scoped residual, diff, and
  independent review pass. The AE1 direct-consumer detector is retired. Row 603 remains open
  pending AC1, CR1, CI1, and final retirement evidence.

- Completed `WP-SD1-A-TERM-01-QLB1` and vocabulary row 318: generated `QuestionSearchRequest`, `QuestionSearchResult`, and `QuestionSearchPage` remain the sole Question Model transport vocabulary, while the intentionally flattened browser contract directly uses the complete `QuestionLibraryBrowse*` family in the Library, Question Picker, Assignment Editor, and sole API adapter. `QuestionSearchAuthorship` remains generated vocabulary in the browse query and `questionSearchRequest()` remains the server-request constructor; no alias remains. The one-time QLB1 residual detector is zero and retired after independent review marked closure eligible. Focused Node (18), both TypeScript checks, QLB1 scoped Prettier, Markdown-link (194), and diff checks pass; fresh `check_codebase.sh` passes all five gates, including 316 Node tests. No Store, route, schema, generated transport source, fixture, test, feature, or behavior changed.

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
  browser boundary, fixtures, and durable documentation. A post-closure
  documentation-integrity repair removes the obsolete deterministic-generator
  vector links and regeneration command: current static PLE evidence is the
  existing server-owned issue/reproduce and Question Presentation descriptor
  checks, while future source-owned generator work remains open. Direct static PLE
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

- Directly unmounted the unbacked PLE Question JSON authoring route composition. `/workspace`
  now presents only the Instructor-gated planned My Question Drafts contract destination, and
  `/workspace/:workspaceRef` fails closed rather than creating browser UUIDs or calling unhandled
  PLE Question JSON read/write/publication paths. The format-specific PLE Question JSON client and
  editor remain non-mounted source exercised by their existing focused Node tests; this change adds
  no server endpoint, compatibility path, fixture, or test family. Focused Node, TypeScript,
  ESLint, Prettier, and diff checks pass. QSOM1 remains open pending the registered Draft Question
  UUID/Edit Number authoring boundary and independent review.

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
