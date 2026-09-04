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




- Completed `WP-SD1-A-TERM-01-PJAF1`: removed the unsupported PLE Question JSON browser file-authoring client, picker, hotspot editor/model, page/field/style/callback wiring, protocol-only client suite, deleted-only editor assertions, and unconsumed `ObjectAddress::WorkspaceQuestionAsset`. The browser has no PLE Question JSON file input, endpoint, Store, schema, SQL, generated declaration, cache/wire field, Object/URL field, fixture, substitute abstraction, or replacement test. `WorkspaceImportAsset`, imported/trusted `QuestionAssetReference` bindings, Object Delivery, parser/compiler, HOTSPOT grading and presentation geometry, and `StudentResponse::Hotspot` remain. Existing object, adapter/model, Question Model, TypeScript, editor, Markdown, formatting, exact-search, and diff gates passed; one-time absence and retained-binding checks passed. Student Upload remains separately accepted under row 213.
- Completed `WP-SD1-A-TERM-01-SU2`: retired the obsolete `WP-FU1`--`WP-FU6` release path before implementation across every declared current owner, including the dated executive snapshot. The snapshot contains no related retirement history; the exact `2026080912` status migration row is the sole history owner. The combined authority detector counts every matching package or Student/learner file-capability occurrence across active plans, implementation plan, status registry, and that snapshot, excluding only that anchored row; both current counts are zero. One-time probes passed `0/1/1/1/1/1` for exact history, planned-before/planned-after package claims, package retirement wording, Student retirement wording, and learner retirement wording. Course Banner Upload remains separate. Adapter-owned Question Asset import remains separate; PLE Question JSON browser file authoring was separately open under row 278 at this record. Independent implementation review passed; independent record review remains required. Documentation checks, shell syntax, focused Prettier, registry cap, and diff gates passed. The shared status registry retains documented pre-existing whole-file Prettier drift; no broad format rewrite was made. No application code, schema, API, generated contract, fixture, test, behavior, or feature changed.

- Completed `WP-SD1-A-TERM-01-PV3`: removed the unowned browser sanitized-markup override and duplicate WeBWorK raw-HTML delivery/cache branch. The browser renders closed Question Content Blocks directly. Bounded strict parsing, protected-value and hostile/malformed-input refusal, source binding, renderer identity, Question Asset routes, private replay/grading, and typed delivery remain. `CACHE_SCHEMA_VERSION` and the deterministic render-key namespace now use v2; retained `ObjectAddress::QuestionRender`, immutable put/`AlreadyExists` recovery, and current decode/validation semantics preserve an immutable rebuild boundary. A one-time cache probe proved separate v1/v2 keys, v1-byte preservation, and v2 rebuild, then was removed. Normal TypeScript generation, focused adapter/renderer/type/format/documentation/diff gates, and independent review passed. No replacement markup record, wire field, compatibility reader, fixture, or permanent test was added. At the PV3 stage, broader View/read-shape terminology remained open; PV6 subsequently closed the broad View row.

- Completed `WP-SD1-A-TERM-01-RQ-CLOSE`: closed the retired generic Reconciliation checklist rows after fresh Graphify-assisted independent classification confirmed no current PLE-owned Reconciliation in source, schema, routes, generated contracts, browser controls, fixtures, test contracts, product documentation, or active plans. The 14 exact queue matches are frozen history/status/audit material, so the temporary current-work queue omits `reconciliation`. The separately scheduled Local Stack Developer Browser Suite lifecycle correction did not describe or block a product Reconciliation boundary. No code, schema, API, behavior, fixture, test, or feature changed.

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
