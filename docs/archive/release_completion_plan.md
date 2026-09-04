# Plan: Peptidyle Learning Engine release completion

## Status

**Vocabulary-ledger synchronization (2026-09-04).** `VOCABULARY_REPLACEMENTS.md` has no unchecked rows. Historical package receipts below may retain the row state that applied when that package was recorded; they do not allocate current vocabulary work. Product capabilities described as still open remain with their named package or plan, not with the vocabulary ledger.

**Current prerequisite: WP-SD1-A is pending independent ACCEPT.** PLE is one installation with global accounts, one Instructor-visible Question Library of Published Questions, private drafts, equal active Instructors, equal Teaching Team Members, and exact CourseInstance/Student authorization. Available Question Revisions are ordinarily selectable; Archived Question Revisions remain resolvable for history and evidence. The current SD1 registry owns the pre-production cutover before release work resumes.

The authoritative current-package and migration-allocation state is [implementation_status.md](implementation_status.md). WP-RC1, WP-RC2, WP-RC3, WP-RC3R, WP-ARCH1, WP-UI1, WP-HG1, WP-R0, WP-R1, WP-R2, and WP-PY-L1 remain accepted where their recorded evidence says so. WP-RC4 through WP-RC12 stay open until their named gates and independent review pass.

`WP-SD1-A-TERM-01-RQB1` is accepted historical evidence. Its prior server-boundary terminology is
superseded by RQB2's exact iMathAS ownership while its least-privilege, PostgreSQL, and aggregate
receipts remain valid evidence.

`WP-SD1-A-TERM-01-RQB2` is accepted after RQB1. It owned one direct pre-production
naming cutover: the remaining generic backend-session names become exact iMathAS Session,
Challenge, Authentication, Grading Context, Result Token, Result Exchange, Result, and Question
Backend Transport ownership; response/control/Student Response marker names become
`ImathasQuestionBackend`/`imathasQuestionBackend`. Question Model owns marker and
generated-boundary renames; LDA and `2026090102` own durable/schema/procedure renames; the iMathAS
adapter owns transport, launch, and result translation; browser copy uses Question Backend only when
backend-agnostic. It preserves RQB1's Session lifecycle, Question Submission marker flow, tables and
relationships, procedures, browser launch flow, security invariants, and focused test categories.
Generated 467 TypeScript declarations; focused adapter, LDA, Node, TypeScript, formatting, and strict
Clippy gates; fresh-apply/no-op/catalog/restricted-login PostgreSQL and both iMathAS PostgreSQL tests;
least-privilege review; and the required aggregate suite all passed on the final tree. This package closes
only the exact terminology cutover; the overall terminology program remains open.

`WP-SD1-A-TERM-01-ALPHA1` is completed. Blueprint Course is the sole PLE reusable source-course
identity across code, schema, contracts, Browser Surfaces, fixtures, tests, and active plans.
Alpha Course remains only as attributed LibreTexts ADAPT prior-art vocabulary. Focused and full
aggregate gates pass; vocabulary row 573 is checked without changing behavior or adding a feature.

`WP-SD1-A-TERM-01-AWO1` is completed. The fresh database and every dependent private-authoring
boundary name Authoring Workspace Owner while preserving Workspace Collaborator as the separate
contributor relationship. PostgreSQL 17 and aggregate gates pass; vocabulary row 444 is checked
without changing authorization behavior or adding a compatibility path.

`WP-SD1-A-TERM-01-BA1` is completed. Blueprint Assignment now names the Blueprint Course-owned
content everywhere, while saved Course Assignment question reuse owns the separate
RetainedAssignmentQuestionSource browser projection. Focused and aggregate gates pass; vocabulary
row 565 is checked without changing wire behavior, adding a compatibility path, or editing fixtures.

`WP-SD1-A-TERM-01-BAREF1` is completed. The exact Blueprint Assignment Reference replaces the
retired internal ID across its Question Model lineage type, Blueprint-operation source record,
generated contract, strict browser decoder, editor/picker consumers, and focused tests. It remains
scoped to an exact Blueprint Course and Blueprint Revision; it is not a Course Assignment record ID
or an authority grant.

`WP-SD1-A-TERM-01-SRF1` is accepted. This no-schema terminology slice aligns the answer-free Student
Response Format Check and its thirteen exact Student Response Format Issues across domain, Wasm,
strict browser decoding, and visible Response Format Messages. Its direct cutover, focused gates,
independent audit with repaired findings, and complete aggregate acceptance passed. The planned
key-free server format-validation fallback remains a separately allocated future server-boundary
correction; that Server Route does not exist.

`WP-SD1-A-TERM-01-PI2` is accepted as the direct Assignment Question Analysis domain/schema/typed-Job
cutover in fresh migration `2026082923`. `AssignmentQuestionAnalysis` and
`assignment_question_analysis` own the Course Instance-and-Assignment-scoped, Scoring
Generation-bound analysis for one source Assignment Entry and exact Question Revision; its four
Question Outcome Categories remain separate from Unscored Attempt Count. The production repair
constrains the typed Job target, while two fixture/oracle composition repairs complete the existing
durable SD1 evidence. `assignment_analysis_course_assignment_matches` proves the composite Course
Instance-plus-Assignment relationship and rejects a cross-Course Assignment. Focused model/schema
gates, independent inventory, and the live SD1 least-privilege PostgreSQL lane pass. No Store,
route, browser, generated contract, worker, new test file, or fixture suite was added; one compact
reciprocal case was added to the existing durable SD1 oracle. This package precedes but does not
accept the existing `WP-SD1-A` independent architecture/privacy final SD1 gate.

`WP-SD1-A-TERM-01-IAA1` is allocated after the accepted global Account/Authenticated Session baseline
and before `WP-RC8`. It directly replaces the generic executable PostgreSQL account creator with the
exact Sysadmin Create Instructor Account operation and gives the shared private Authentication Email
relation role-qualified integrity rules. It owns no roster route, Store, DTO, invitation claim, or
Instructor email-replacement implementation; `WP-RC8` remains the sole future Course Roster Import
transaction owner for Student Account resolution and creation.

`WP-SD1-A-TERM-01-AEM1` completed removal of the unimplemented Assignment Export persistence/Job stub and its
named current-service documentation inventory before any export feature is admitted. That
inventory covers the security, identity, database-authorization, multi-server, object-storage,
data-classification, authorization-contract, component-consumer, implementation-plan, release-
plan, and customer-spec claims recorded in [implementation_status.md](implementation_status.md). No
current Assignment Export persistence, Job kind or target, Store, route, worker, delivery path, browser
contract, or service remains.
Its residual audit retains ordinary export wording, Course Grade CSV export, QTI interchange, and
the answer-key-free DOCX/PDF renderer, while a future Assignment Export Manifest remains a
complete authorized service prerequisite rather than a preparatory schema or release claim. Fresh
PostgreSQL catalog, print-renderer, documentation, Rust, residual, and independent-review evidence pass.

`WP-SD1-A-QSOM1-S2A` is the accepted Assignment Entry control move. Fixed and Question Pool
Assignment Entries plus their immutable Assignment Revision Entry snapshots own
QuestionAttemptLimit and QuestionAttemptTimeLimit; BaseAssignmentPolicy attempt/time controls remain
assignment-wide. The later S2B source-model cut removes the retired generic content and grading
records rather than retaining them as current QSOM1 work. The current-package status and migration
allocation remain solely in [implementation_status.md](implementation_status.md).

`WP-SD1-A-QSOM1-S2B1A` is accepted and completed. Server-only, non-Serde
`QuestionEvaluation { correct, normalized_credit }` is Question Backend evaluation while
Assignment-owned GradingResult remains the scoring record. The direct iMathAS issued-score cut keeps
QuestionAttemptId and authentication/Result lifecycle facts on the Session; atomic commit locks the
selected IssuedQuestion and resolves its point_value and scoring_rule. Two independent reviewers
PASS, and the manager's final `source source_me.sh && ./all_test.sh` exits 0 with 424 generated
types, 3 fixtures, Rust workspace/Clippy/tests/doctests/Wasm, 315 Node, 4,908 pytest, fresh/no-op/
catalog/restricted PostgreSQL, 3/3 iMathAS, Course Appearance PostgreSQL-plus-MinIO, and
`PASS: complete live acceptance is green.` The durable blueprint_course schedule, student_work
grading, and iMathAS catalog oracle splits are line-gate organization only, not product behavior.
S2B2 (PLE Question JSON adapter), S2B3 (WeBWorK adapter), S2B4 (iMathAS source/session input
removal), S2B5 (QTI import mapping and minimal H5P preservation), S2B6 (unbacked generic Draft
Question editor and PLE Question JSON fallback removal), and S2B7 (final generic-root deletion) are
accepted and completed after independent review. QTI is import/export/archive interchange, not a
runtime dispatch path. `/workspace` is an Instructor-gated, truthful planned **My Question Drafts**
destination; no authoring or publication Server Route or Browser Surface exists. P1 now implements
the server-only new-lineage publication Store transaction after trusted bytes-first source copying;
new-lineage object-copy coordination is P2; same-lineage publication, cleanup, and browser delivery remain open. The final manager
`source source_me.sh && ./all_test.sh` exits 0 with 421 generated types, 3 tracked fixtures, Rust
format/check/all-feature strict Clippy/tests/doctests/Wasm, 286 Node tests, 4,831 pytest tests,
PostgreSQL 17 fresh/no-op/catalog/restricted plus 3 iMathAS tests, Course Appearance
PostgreSQL-plus-MinIO, and `PASS: complete live acceptance is green.` Parent QSOM1 remains open only
for separately owned publication, persistence, and cleanup work. Published Question Title and
Description remain mutable lineage facts. [implementation_status.md](implementation_status.md)
remains the sole allocation registry.

`WP-SD1-A-QSOM1-P1` is implemented and acceptance-open after M1. Its server-only new-lineage
Question Publication Store atomically turns one exact current Draft Question state and a trusted
bytes-first target Object Record into the complete first immutable Question Revision aggregate.
Focused PostgreSQL and final-tree aggregate evidence pass; independent review remains required. No publication Server Route or Browser
Surface exists, and same-lineage publication, cleanup, Question Search,
and parent QSOM1 acceptance remain open.

`WP-SD1-A-QSOM1-P2` is implemented and acceptance-open after P1. Its server-only
coordinator resolves the exact authorized current Draft Question Source Object Record, verifies and
copies those immutable bytes to a fresh Question Revision address, issues the HMAC-validated
Question ID, and invokes P1. Focused Rust, strict Clippy, source hygiene, PostgreSQL 17
fresh/no-op/catalog/restricted/iMathAS, and final-tree aggregate evidence pass; independent review
remains required. No publication Server Route or Browser Surface exists. Same-lineage publication,
secret-file composition, orphan cleanup, Draft Question expiration, Question Search, and parent
QSOM1 acceptance remain open. The typed Question Revision Reason is only part of vocabulary row
312, which remains unchecked pending durable reason history and comparison Views.

`WP-SD1-A-TERM-01-SAV1` is completed after corrective re-review reopened row 707 and repaired active
plans, status receipts, contract prose, checklist evidence, and the route-contract comment. Current
meanings now state directly whether a Server Route exists, a Service is implemented, or a Browser
Surface is available. Its accepted inventory classified actual filesystem, volume, and container
attachment operations separately from immutable history/audit/archive evidence and the ledger's
required legacy phrase. The full aggregate,
2,488 documentation/source checks, contextual residual, formatting, and diff gates pass.

`WP-SD1-A-TERM-01-QSLR1` is completed. Question Summary consistently names a stable Published
Question lineage and carries the exact Question Revision Reference for the accepted revision with
the greatest Question Revision Number. PostgreSQL, Rust, generated TypeScript, strict browser,
fixture, test, and current-documentation owners agree, and Question Revision Availability remains
separate. Focused Rust, generation, browser, PostgreSQL 17 migration/catalog/restricted-login,
formatting, and diff gates pass; vocabulary row 317 is checked.

`WP-SD1-A-TERM-01-QT1` is completed. The direct pre-production Question Title cutover uses
`question_title` in Rust and `questionTitle` at serialized and browser boundaries across authored
PLE Question JSON, Draft Question and Question Summary views, presentation and delivery, adapters,
generated declarations, strict decoders, interfaces, fixtures, tests, and current documentation.
No legacy reader remains. Question Prompt, external QTI/XML `title`, and exact titles owned by other
domain concepts stay distinct. Final aggregate and contextual residual evidence pass; vocabulary
row 321 is checked.

`WP-SD1-A-TERM-01-QVR1` is completed. Static PLE Question JSON and QTI-imported static Questions
carry no redundant content-level `Static` variation-rule field. QTI profiles own import-shape
admission; the Assignment-owned Question Variation Rule independently owns later-Attempt Reuse
Variation or New Variation behavior and never Question Pool Selection. A future seeded Question
Generator remains open until its complete source-to-reproduction path exists. Focused QTI, Question
Model, documentation, aggregate, contextual residual, and diff gates pass; vocabulary row 296 is
checked.

`WP-SD1-A-TERM-01-QPV1` is completed. Current contracts distinguish closed Question Publication
Requirements, calculated Question Publication Validation at one exact Draft Question Edit Number,
and its complete Question Publication Issues without making validation a lifecycle state. Generic
report/violation types are absent. Append-only migration `2026090304` gives the remaining Question
Change Proposal Revision schema field and check constraint exact Question Publication Validation
ownership; the PostgreSQL 17 catalog oracle requires them and rejects the predecessor. The actual
publication operation and Browser Surface remain unimplemented QSOM work. Focused schema,
documentation, PostgreSQL, aggregate, residual, and diff gates pass; vocabulary row 339 is checked.

`WP-SD1-A-TERM-01-QD1` is completed as a prerequisite. The shared strict browser decoder consumes
the generated Question Model limits and enforces a required, non-whitespace Question Title of at
most 512 Unicode scalars and Question Description of at most 4,000. A permanent browser test rejects oversized values in a real
Published Question page contract. Focused frontend, Question Model, PLE adapter,
documentation/source, formatting, and diff gates pass. The vocabulary boundary is complete. The
separately open product capability is a Question Publication Validation Service and
post-publication metadata-editing Server Route proving that Question Title and Question Description
are mutable Published Question lineage facts without creating a Question Revision.

`WP-SD1-A-TERM-01-BRR1` is completed as a prerequisite. The strict Blueprint operations HTTP
decoder reconstructs all six exact generated completion-result variants and rejects unknown nested
fields, including a separate `replay` product state. The valid browser fixture matches the Rust-owned
contract; operation-specific server-held Receipts remain non-Serde. Focused Blueprint Question Model,
frontend, TypeScript, formatting, and documentation/source gates pass. Final aggregate acceptance
generated 422 contracts, validated 3 fixtures, and passed Rust/Wasm, 288 Node, 4,850 Python,
PostgreSQL 17, and PostgreSQL-plus-MinIO gates. The vocabulary boundary is complete. The separately
open product capability is a durable Blueprint operation Store and Server Route returning the same
accepted exact Receipt for the same Account, Request Checksum, source/target, and revision facts.

`WP-SD1-A-TERM-01-BRI1` is completed. The fresh PostgreSQL baseline directly removes the redundant
Blueprint Revision UUID and identifies every Blueprint Revision by the exact Blueprint Course
Reference number and positive Blueprint Revision Number pair already used by Rust and generated
browser contracts. Course Instance, Course Origin, Assignment source, publication, availability,
and collaboration records use that pair consistently. The final-tree aggregate generated 422
contracts, validated 3 tracked fixtures, passed Rust formatting/checks/strict
Clippy/tests/doctests/Wasm, 288 Node tests, 4,850 Python tests, PostgreSQL 17
fresh/no-op/catalog/restricted-login with 3 iMathAS Store tests, and the PostgreSQL-plus-MinIO
course-appearance oracle. Vocabulary row 567 is checked without a compatibility column, backfill,
route, Store operation, Browser Surface, or feature.

`WP-SD1-A-TERM-01-BCO1` is completed. The sole durable owner relationship is
`blueprint_course.blueprint_course_owner_account_id`; publication, availability, and Draft
Blueprint Revision collaboration authorize that exact Blueprint Course Owner. PostgreSQL 17 proves
another Instructor is refused while the owner succeeds. Rust, generated TypeScript, the strict
decoder, the Blueprint Course workspace, and the hostile generic-owner refusal fixture agree on
`BlueprintCourseReadAccess`. The approval-era database diagnostic is removed, and vocabulary row
443 is checked. The final aggregate generated 422 contracts, validated 3 tracked fixtures, passed
Rust formatting/checks/strict Clippy/tests/doctests/Wasm, 288 Node tests, 4,850 Python tests,
PostgreSQL 17 fresh/no-op/catalog/restricted-login with 3 iMathAS Store tests, and the
PostgreSQL-plus-MinIO course-appearance oracle. No route, Store, schema relationship, compatibility
alias, Browser Surface, fixture family, or feature was added.

`WP-SD1-A-TERM-01-QO1` is completed. Each Published Question lineage has one current Question Owner
derived from a repeatable ordered chain of immutable Question Ownership Events. Only the current
owner records an accepted transfer, and the next owner must be an Active Instructor Account.
Question Authorship stays separate; publication derives ownership server-side and browser contracts
expose no owner identity. The Question Library rechecks Account State and remains visible to every
Active Instructor Account regardless of ownership. PostgreSQL 17 proves the owner transition and
shared-visibility boundary. Vocabulary row 442 is checked without a route, Store operation, Browser
Surface, compatibility path, or feature.

`WP-SD1-A-TERM-01-QSB1` is completed. The pre-production fresh baseline now creates and uses
qualified Draft Question and Question Revision Source Bindings directly across RLS, Object Record
validation, Bind Question Source, publication validation, and iMathAS resolution. The metadata-only
2026090301 migration no longer copies from or drops a predecessor table, and retired-name-specific
catalog assertions are deleted. Full aggregate and PostgreSQL 17 gates pass. Rows 262 and 325 remain
open for the remaining QSOM1 cleanup, Question Search, route, browser, and final acceptance scope.

`WP-SD1-A-TERM-01-SLWS1` is completed. Question Model now owns the only Student Late Work Status
enum, Domain re-exports it, and the exact field name continues through Student Assignment Delivery,
the generated browser contract, strict decoding, and Student presentation. Its three accepted-work
results remain distinct from the Late Work Refused Assignment Start Decision denial. Vocabulary
row 186 is checked after focused and full aggregate acceptance; no fixture, schema, route, or
feature was added.

`WP-SD1-A-TERM-01-QANS1` is completed. Released display-ready accepted-response content is named
Question Answer through the trusted grader, Student Feedback, generated contracts, strict decoder,
release policy, and visible headings. The local authoring preview is correctly described as an
Answer Key and Question Feedback check; exact QTI vocabulary and private grading/correctness facts
remain distinct. Vocabulary row 286 is checked after focused and full aggregate acceptance, with no
fixture, schema, route, or feature added.

`WP-SD1-A-TERM-01-SAV2` is completed after a second corrective review reopened row 707. All current
application-availability prose now says directly whether a Server Route exists, a Service is
implemented, a Browser Surface is available, a component renders, or transport begins. The 12
remaining current attachment operations are actual filesystem, volume, or container work. Focused and
full aggregate gates pass; row 707 is checked again without adding a capability.

`WP-SD1-A-TERM-01-RFM1` is completed. The browser derives Response Format Messages from the exact
Student Response Format Check and Issues during key-free local validation. Question Hint and
Question Feedback remain separate pre-response and post-grading teaching content. Vocabulary row
288 is checked after focused and full aggregate acceptance, with no fixture, schema, route, or
feature added.

`WP-SD1-A-TERM-01-SRI1` is completed. The retained future Student Response Inspection browser
contract now names its exact inspection and permitted correctness/score members. Domain, generated
contracts, strict decoding, tests, and visible privacy copy distinguish Student Response,
Question Answer, Question Answer Explanation, Answer Key, and Question Grading Input. Vocabulary
row 285 is checked after focused and full aggregate acceptance; no fixture, schema, Server Route,
Browser Surface, or feature was added.

This is the binding release authority for decisions, objectives, architecture, dependency order, acceptance/evidence, migration policy, risks, rollout, and closeout. The [Current Package Registry](implementation_status.md) records current package status. Update both documents when a release decision, dependency, status, or acceptance condition changes.

### Evidence classification

Apply [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) and the permanent-test checklist to every package. Permanent tests prove stable behavior and security boundaries. Disposable service, cloud, browser, screenshot, migration, timing, and reconstruction checks prove their distinct environmental claims. One-time inventories and probes record a decision, then leave the permanent suite. Fixtures exist only for stable serialized contracts; otherwise use inline builders.

## Decisions

### In-scope Decision Register

| Topic                         | Binding decision                                                                                                                                                                                                                                                                                                                                                                                                                                    | Owner                                                                                           |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Installation and roles        | One PLE installation has global accounts. Each account has one immutable Student, Instructor, or Sysadmin role; people needing multiple roles use separate accounts. Course authority is matching exact membership and Student ownership. Sysadmin Create Instructor Account creates one active Account with the fixed Instructor Product Role from a normalized email address and creates no Sysadmin membership; support is explicit and audited. | WP-SD1                                                                                          |
| Reusable courses              | A revisioned `BlueprintCourse` owns reusable ordered structure. Every `CourseInstance` has one immutable Blueprint parent and applied revision; it alone owns Students, deadlines, releases, accommodations, grades, and delivery state.                                                                                                                                                                                                            | WP-SD1-B--G                                                                                     |
| Published questions           | Stable `AAA-BBBB` `QuestionId` identifies a lineage; immutable `QuestionRevision` records hold reviewed revisions. Assignments and evidence pin exact versions and never move automatically.                                                                                                                                                                                                                                                        | WP-R2, WP-SD1                                                                                   |
| Draft questions               | One mutable Draft Question belongs to one Authoring Workspace. Its private UUID is server-only and its positive Draft Question Edit Number is the save/publication concurrency token. Publication creates an immutable `QuestionRevisionReference { question_id, revision_number }`; Draft Question revision history is not retained.                                                                                                               | DQM1 implemented pending independent review; QSRC2 implemented pending joint independent review |
| Question stewardship          | Moderate owner edits, validated exact-base Change Proposals, full private-draft forks, and audited Sysadmin ForcedQuestionCorrections preserve attribution, compatible CC licensing, history, and exact pins. UI label: **Suggest an improvement**.                                                                                                                                                                                                 | WP-R2, WP-SD1                                                                                   |
| PLE questions                 | PLE Question JSON version 3 is the sole PLE Question JSON reader and PLE Question Source for MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. Public Question Presentations are answer-free; grading remains server-owned.                                                                                                                                                                                                                   | WP-RC4, WP-RC5                                                                                  |
| Question variation and grades | New Assignment Attempts independently use `reuseSelection` for Question Pool membership and `newVariation` for Question Variations; resumed Issued Questions retain their exact issued variation. The grade default is `highest`.                                                                                                                                                                                                                   | WP-RC0, WP-PROF-T5, WP-PROF-G1--G5                                                              |
| Retention                     | CourseInstance Student records notify at 30 days, archive at 100, delete at 365; course-owned Assignment Content remains. Aggregate publication requires k >= 5.                                                                                                                                                                                                                                                                                    | WP-RC0, WP-SD1                                                                                  |
| Question technologies         | One shared Draft Question, publication, Assignment, issuance, presentation, submission, evaluation, feedback-release, and Gradebook pipeline resolves the registered Question Backend for format-specific work. QTI profiles map accepted flat items into PLE Question JSON or refuse; QTI remains import/export/archive interchange. H5P retains its distinct package source and current ungraded-practice behavior behind the shared operations.  | WP-RC6                                                                                          |
| Objects                       | Database records define intended bytes; inventory proves storage. Object Storage Check uses two observations and reference rechecks; Object Storage Repair acts only on that evidence. A dedicated publisher alone activates immutable public copies.                                                                                                                                                                                               | WP-RC7                                                                                          |
| Identity and enrollment       | Email-code sign-in is the primary authentication method; passkeys are optional convenience credentials on the same global account. Invitations create exact course membership and Student records atomically.                                                                                                                                                                                                                                       | WP-RC8                                                                                          |
| LTI                           | LTI 1.3 launch and AGS passback use verified server credentials and summary-derived grades only.                                                                                                                                                                                                                                                                                                                                                    | WP-RC9                                                                                          |
| Deployment and traffic        | OpenTofu owns disposable AWS infrastructure. Anonymous landing traffic terminates at static edge storage; authenticated requests have bounded cost and no client analytics.                                                                                                                                                                                                                                                                         | WP-RC10, WP-RC11                                                                                |

### Out-of-scope decisions

Version 1 focuses H5P Package content on ungraded practice. Content-addressed byte deduplication, a TypeScript API server, server-graded H5P, local passwords, mandatory institutional SSO, client analytics, Kubernetes/Redis/Kafka/sharding, unreviewed rich-media QTI mappings, a Rust QTI Package Maker port, actual institutional credentials, and a real 10,000-Student cohort remain future possibilities. These future possibilities do not relax release acceptance.

## Objectives and scope

Deliver one coherent automated-grading platform and the required production-stack journey. Grading, answer keys, correctness decisions, object authorization, and course selection remain server-owned. Browser contracts remain answer-free. Issued work and grading evidence are immutable, and Instructor inspection is audited.

The release scope is the dependency-ordered [Current Package Registry](implementation_status.md): WP-RC1--WP-RC12, WP-ARCH1, and their current-package prerequisites. It includes live delivery convergence, variation, discovery, sharing, reusable curricula, Blueprint updates, automated grading operations, PLE Question implementations, adapters, QTI interchange, DOCX/PDF print rendering, Object Storage Check and Repair, identity/enrollment, LTI, artifacts, deployment, cost controls, and final closure.

No package may turn an unresolved product decision into an implicit compatibility path. If evidence invalidates a decision, update the In-scope Decision Register, every affected package entry, and acceptance evidence in one reviewed planning change before code continues.

### BlueprintCourse and CourseInstance cutover

`BlueprintCourse` is the only reusable course-level aggregate. Its revision holds ordered modules, assignments, relative schedule defaults, and exact published-question pins. A one-assignment reusable unit is a one-module `BlueprintCourseView`, not another type.

Each `CourseInstance` binds to one immutable Blueprint parent and applied source revision. Instantiation copies reusable meaning and resolved defaults, never Student records. Students, deadlines, releases, accommodations, grades, attempts, and delivery settings are private CourseInstance state. A Blueprint revision can make a new assignment available to descendants as unreleased; release requires an explicit Instructor decision and preserves local delivery edits.

`2026082911` owns minimal-Blueprint construction; the direct course-creation capability invokes
it while atomically creating the bound CourseInstance and initial Instructor membership.
`2026082913` owns immutable CourseInstance adoption records and their exact record identity;
`2026082929` owns the only executable Blueprint-operation application capability over those
records, including Assignment Import Repair; `2026082930` owns forced RLS for CourseInstance roots
and dependent private state.
`2026082906` owns the shared Rust
account-transaction installer. The Blueprint operation boundary has exactly six operations and never creates a
blank CourseInstance. An apply receives scope only from session-derived `AuthenticatedSession`; adapters
and protected database operations receive no client-supplied installation scope.

No current product type, route, Store capability, PostgreSQL table/function/policy, generated contract, live-demo resource, or screenshot may use Alpha as a Peptidyle product concept. Historical migrations, changelogs, and ADAPT comparison material remain evidence rather than compatibility contracts. Fresh SD1-C allocations belong only in [implementation_status.md](implementation_status.md).

### Cutover evidence boundary

| Claim                                                                                                                 | Evidence                                                                      |
| --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Aggregate normalization, revision races, propagation, and private delivery state                                      | Permanent focused Rust/Memory/Node behavior and contract tests                |
| Global Question Library visibility, CourseInstance isolation, protected database authority, and fresh/no-op migration | Disposable PostgreSQL/RLS acceptance                                          |
| Create, revise, publish, select, instantiate, update, and release workflows                                           | Production HTTPS browser acceptance through `run_playwright_tests.sh --build` |
| Hierarchy, release state, recovery, focus, contrast, and product vocabulary                                           | Rendered screenshots plus independent visual review                           |
| Complete material tree                                                                                                | `source source_me.sh && ./all_test.sh`, with every required lane passing      |

## Architecture and ownership

| Boundary                  | Owner                                                  | Rule                                                                                                                                               |
| ------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product decisions         | `docs/HUMAN_GUIDANCE.md` and this plan                 | Human Guidance remains terse owner intent; settled engineering interpretation belongs here or in [DESIGN_DECISIONS.md](../DESIGN_DECISIONS.md). |
| Public question contracts | `crates/question_model`                                | Generate answer-free TypeScript contracts.                                                                                                         |
| Source adapters           | `crates/adapters/{ple,qti,webwork,h5p,imathas}`        | One strict versioned adapter per format.                                                                                                           |
| Grading                   | `crates/grading` plus server-only adapter capabilities | No browser, generated TypeScript, or Wasm grading authority.                                                                                       |
| Persistence               | `crates/learning-data-access`, `schemas/migrations`    | Memory/PostgreSQL parity; PostgreSQL is production authority.                                                                                      |
| Objects                   | `crates/objects`                                       | Typed keys, checksums, role-based delivery, inventory, Object Storage Check and Repair.                                                            |
| HTTP and workers          | `crates/server`                                        | Bounded same-origin requests; durable jobs carry explicit least authority.                                                                         |
| Browser                   | `src/`                                                 | Strict decoders, accessible visible workflows, no source archive parsing.                                                                          |
| Local stack               | `local_stack_control/`, `containers/`                  | Python owns complex orchestration; shell entry points are direct facades.                                                                          |
| Deployment                | `deploy/opentofu/`                                     | Declarative, reviewable, disposable before activation.                                                                                             |

## Dependency order

```text
WP-SD1-A independent ACCEPT
  -> WP-SD1-B -> WP-SD1-C -> WP-SD1-D/E -> WP-SD1-F -> WP-SD1-G
  -> WP-RC4 -> WP-P1..WP-P6 -> WP-RC5 -> WP-RC6
  -> WP-P2 -> WP-RC7
  -> WP-SD1-A-TERM-01-IAA1 -> WP-RC8 -> WP-RC9 -> WP-RC10 -> WP-RC11 -> WP-RC12

Accepted foundations: WP-RC1 -> WP-RC2 -> WP-RC3 -> WP-ARCH1 -> WP-RC3R.
Accepted orchestration: WP-R1 -> WP-PY-L1. Accepted stewardship: WP-R2.
```

WP-SD1 is the release prerequisite. `WP-SD1-A-decisions-and-impact-contract` stays pending until its independent architecture/privacy review accepts it. B--G proceed in order without an Alpha bridge or parallel product path. WP-P1 may progress beside RC4 closeout, but all WP-P1--WP-P6 requirements accept before RC5. WP-P2 preserves migration allocation and transitions legacy consumers before RC7 schema work. `WP-SD1-A-TERM-01-IAA1` supplies exact current-baseline Instructor creation and role-qualified private-email integrity before the complete `WP-RC8` roster/import delivery transaction.

## Release acceptance criteria

- Every package has a named owner, actual artifact boundary, behavior, evidence class, and independent review.
- Focused mocks support code work but cannot solely accept a production route, Store, object, identity, worker, or deployment claim.
- Answers, keys, Question Backend credentials, object credentials, and authority selection remain server-owned.
- Student workflows are keyboard complete; authoring has visible focus, status, recovery, reflow, and semantic labels; HOTSPOT has a non-pointer alternative.
- Migrations are forward-only and demonstrate fresh, no-op, checksum, RLS/grant, and live behavior evidence.
- Required skipped, unavailable, stale, or human-only evidence remains open.

## Test and verification strategy

Run the narrowest owner check, then the package gate, then release evidence. The aggregate front door:

```bash
source source_me.sh && ./all_test.sh
```

`all_test.sh` owns `check_rust.sh`, `check_codebase.sh`, repository pytest, and `local_stack.py acceptance`. The current controller runs its declared database/object acceptance lanes. The former `run_playwright_tests.sh --build` and screenshot-corpus browser lanes have no executable owner after their configuration was retired; browser restoration is required before release acceptance. `tests/e2e/e2e_run_all.sh` is an explicit non-browser bulk E2E owner, never a second aggregate. Development SKIP output names a missing prerequisite; release evidence requires PASS.

| Evidence class          | Owner                              | Claim                                                                                     |
| ----------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------- |
| Permanent offline       | `all_test.sh`                      | Stable Rust, TypeScript, Node, Python, generated-contract, security, and hygiene behavior |
| Real service acceptance | `local_stack.py acceptance`        | Its current declared PostgreSQL and object-service lanes                                  |
| Production browser      | Restoration required               | Built bundle through HTTPS gateway and real UI-created state                              |
| Visual evidence         | Restoration required               | Fresh rendered states and human visual review for UI or viewport changes                  |
| One-time evidence       | Graphify and direct probes         | Narrow decision/migration/config disposition, not permanent tests                         |
| Human acceptance        | Independent review and walkthrough | Teaching workflow, accessibility, visual sense-making, legal/activation decisions         |

The final handoff records command, date, material-tree state, environment, receipt path, and limitations for every required evidence class.

## Migration policy

The shared [Migration Allocation Registry](implementation_status.md#migration-allocation-registry) is the only allocation registry. New schema packages receive an allocation before implementation; accepted migrations are never inserted or renamed. PLE Question JSON identity stays in its versioned source payload and immutable object/checksum binding; no generic catchall table is added. Current source and disposable test data use PLE Question JSON version 3 only. `QuestionPresentationBinding`, QTI profile v1, and `AAA-BBBB` Question IDs are current contracts, not compatibility shims.

## Risk register

| Risk                                                  | Owner                 | Control                                                                                                                    |
| ----------------------------------------------------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Documentation substitutes for product evidence        | Release integrator    | Package acceptance requires working behavior and evidence.                                                                 |
| Adapter output leaks answers or unsafe markup         | Adapter owner         | Strict translation, sanitization, private network, browser trace.                                                          |
| New protected data boundary exposes an Answer Key     | Boundary owner        | Public/private compilation, DTO scans, server-only grading.                                                                |
| Object Storage Repair deletes valid concurrent bytes  | Object owner          | Two observations, quarantine, reference recheck, and a repeat-safe exact repair result.                                    |
| Role/membership disagreement selects course authority | Auth owner            | One immutable account/session role, matching Student/Instructor membership, no Sysadmin membership, and origin validation. |
| Published bytes escape before commit                  | Object owner          | Transactional pending registry and dedicated publisher.                                                                    |
| iMathAS Question Backend dispatch outcome is unknown  | iMathAS adapter owner | Durable lease-bound marker and explicit operator resolution.                                                               |
| Deployment exposes secrets or broad destroy           | Deployment owner      | Secret references, unique tags, reviewed plan, bounded destroy.                                                            |
| Bot protection harms legitimate users                 | Edge owner            | Count mode, accessible recovery, versioned legitimate corpus, rollback.                                                    |
| Pilot begins before activation evidence               | Product owner         | Separate signed production-activation checklist.                                                                           |

## Rollout and closeout

Working-codebase release proves reproducible repository-owned artifacts without institutional secrets. Production activation supplies operator credentials, applies deployment, runs named live gates, completes legal review, and enrolls the pilot. Neither milestone substitutes for the other.

WP-RC12 closes only after every package in [implementation_status.md](implementation_status.md) has required PASS evidence and independent review. It updates release evidence, documentation, implementation status, changelog, and release notes with exact receipts. Source inventories, scratch probes, and temporary diagnostics remain documented one-time evidence rather than fragile permanent tests.

Each package handoff records package ID, owner, changed files, visible/security behavior, focused/package/release checks, evidence paths, governing decisions, and independent findings.
