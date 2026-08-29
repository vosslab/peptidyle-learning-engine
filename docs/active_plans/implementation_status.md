# Implementation status and handoff

Last updated: 2026-08-29

This is the sole mutable registry for the global current-package handoff and shared migration
allocations. The [implementation plan](implementation_plan.md), active
[Instructor capability plan](active/instructor_capability_architecture_plan.md), and active
[release completion plan](active/release_completion_plan.md) own architecture, scope, dependency
order, validation, and acceptance. Durable product decisions remain in
[Human Guidance](../HUMAN_GUIDANCE.md); package history and detailed receipts remain in the
[changelog](../CHANGELOG.md).

Work-package labels such as `WP-INST-G2` are temporary plan coordinates. They identify the current
handoff while the plan is active and retire with the planning layer; product contracts and durable
data use domain identifiers.

## Current handoff

- **Current package:** `WP-INST-WN1-SR4-browser-direct-clients` - converge the ledger's exact
  browser contracts, strict decoders, route builders, presentation components, progress helpers,
  and direct consumers from role alias `Learner` to canonical `Student` without an alias layer.
  WN1-OPS1 through WN1-OPS10, WN1-B1 through B5, WN1-GO1, WN1-MG, and WN1-SR1 through SR3 are
  accepted.
  **Current pre-WN1:** lower-camel transport remains in
  material source. **Approved target:** Rust Serde owns PLE `snake_case` data-object properties,
  query keys, and portable discriminants while TypeScript functions/locals and registered
  protocols retain owner conventions. C4-IA1 owns the direct item-analysis route/client contract;
  QM-CAPABILITY owns capability-discriminant spelling.
- **Acceptance-open predecessor:** `WP-INST-G2` is implemented and acceptance-open behind
  `WP-INST-WN1` and its remaining G2 visual/documentation close-out. Its approved
  [audited Student-work and calculated Gradebook plan](active/audited_student_work_gradebook_plan.md)
  retains the roster-first `CourseGradebookStore`, atomic-audit `StudentWorkInspectionStore`, and
  migrations `2026081870` through `2026081878`; G2 W5/W6 resume after WN1 acceptance.
  `CourseGradebookStore` owns the roster-first, server-calculated page; a dedicated
  `StudentWorkInspectionStore` owns one explicit, atomic-audit, solution-free detail read. The package reserves
  migrations `2026081870` through `2026081878` as authority foundation, private immutable witness,
  only app-executable broker, query evidence, tenant-bound worker failure, a forward broker
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

`WN1-OPS3-browser-front-doors` is accepted on 2026-08-29. `capture_screenshots.sh` and
`run_playwright_tests.sh` now use lowercase `script_directory` for their private repository path.
Shell syntax and both visible help contracts pass while the canonical screenshot command still
delegates to the shared production-browser owner. The next ordinary browser lane remains the
permanent runtime evidence; exact private-name inspection is one-time.

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
while tenant-bound setup writes carry the required tenant context. Rust formatting, strict Clippy,
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
route-policy authority, and HTTP tests are retired. A compact route-policy assertion keeps both
methods unauthorized, while `ManualGradingStore` and current status branches remain an explicit
internal dependency for later MG children. Independent review returned `ACCEPT`. Format, server
library check, all 16 focused route-policy tests, and full server-library test compilation pass.
Automated operations, accepted-submission processing, normal submission/status, Gradebook, and
`ManualGradeExportStore` remain intact.

`WN1-MG1B1-outcome` is accepted on 2026-08-29. `GradeOutcome::NeedsManualGrading`,
`AnswerKey::FileUpload`, and `SubmissionDisposition::NeedsManualGrading` are retired. Graded file
upload now returns a typed deterministic-grader capability refusal after format and grading-mode
validation and before answer-key lookup. Independent review returned `ACCEPT`. Format and affected
package checks pass; grading, accepted-submission worker, run, and project-tools suites pass 6,
18, 43, and 63 tests. Supported graded and ungraded paths, external committed outcomes, worker
retry/fencing, Gradebook, and the transitional attempt/evaluation/store bridge remain intact.

`WN1-MG1B2-attempt-status` is accepted on 2026-08-29. `AttemptStatus` now has exactly
`InProgress`, `Submitted`, `AutoSubmitted`, `Cleared`, and `Exempt`, with direct Serde-owned
`snake_case` generation and strict PostgreSQL/browser decoding. Memory and PostgreSQL force-submit
atomically close `InProgress` work as answer-free `AutoSubmitted`, retain exact action replay,
timing cleanup, and audited evidence, and fabricate neither a response nor a result. The temporary
manual Store bridge now uses attempt `Submitted` plus its separate manual evaluation record, and
item analysis reads that evaluation state directly. Independent review returned `ACCEPT`. Manager
format, check, strict Clippy, question-model, Memory/PostgreSQL-capable Store, conformance,
project-tools, TypeScript, and decoder gates pass; the connected absence-evidence worker closure
remains explicitly assigned to a later MG child.

`WN1-MG1B3-evaluation-status-contracts` is accepted on 2026-08-29. The public automated
evaluation status now has exactly `automated_pending`, `automated_exception`, `graded`, and
`exempt`; Rust Serde and the generated TypeScript union share that one direct `snake_case`
contract, and the retired lower-camel spelling fails closed. The Memory status aggregate accepts
only coherent receipt, execution, and evaluation tuples and exposes answer-free pending,
Instructor-attention, or completed projections. Independent review returned `ACCEPT`. Manager
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
same-tenant Student denial, Instructor/RLS/privacy oracle, generation fencing, and exact cleanup.
MG1D now owns the automated-only runtime and persistence boundary plus migration `2026081883`;
C4-IA1 retains the later direct route/browser contract.

`WN1-MG1D-automated-scoring-persistence-retirement` and the parent `WN1-MG` are accepted on
2026-08-29. Runtime composition now has one automated evaluation owner: deterministic completion,
answer-free exception state, bounded retry/recalculation, immutable grading evidence, calculated
Gradebook totals, and roster score export. Migration `2026081883` closes the parallel manual
receipt, binder, policy, table, and catalog values while preserving mature invalidation function
bodies through exact fail-closed catalog rewrites and unchanged identity, owner, ACL, configuration,
and security-mode assertions. The audit found no reachable manual-scoring mutation path; the
bounded `ManualGradeExportStore` remains the score-download/audit seam, and decoder plus route-policy
checks keep retired inputs unavailable.

Permanent evidence includes automated Store/worker behavior, strict status decoding, route-policy
authority, and contactless-Student Gradebook/export coverage in Memory and PostgreSQL. One-time
retirement inventory and clean-volume installation remain outside the permanent suite. Format,
focused check and strict Clippy, 236 learning-data-access tests, 81 conformance tests, 423 server
tests with three intentional ignores, 63 project-tools tests, TypeScript compilation, 117 SQL-line
checks, and the fresh 109-migration PostgreSQL/RLS baseline pass. Six independent review passes
accepted the runtime boundary after canonical lifecycle/plan wording, migration ownership,
domain-only diagnostics, and the contactless export gap were repaired. The final WN1 aggregate and
full Validation suite remain later acceptance gates.

`WN1-SR1-disclosure-statistics` is accepted on 2026-08-29. The complete disclosure and
class-statistics source graph now uses `StudentDisclosureTiming`, `StudentDisclosurePolicy`,
`StudentDisclosureDecision`, `StudentDisclosureInput`, and `StudentClassStatistics`; private Store
methods and PostgreSQL modules use `student_disclosure` and `student_class_statistics`. Effective
Serde, regenerated TypeScript, reusable-curriculum defaults, and strict browser decoders share one
direct `snake_case` contract, including `student_disclosure`, `insufficient_evidence`, and
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
now uses `StudentScoreState`, `StudentAssignmentProgress`, `StudentAssignmentLandingSummary`,
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
optional institutional roster metadata, while roster ID and email remain confined to the audited
CSV projection.

## G1-W2 accepted evidence

`WP-INST-G1 / G1-W2` is accepted on 2026-08-27 for its static/offline implementation and fresh
schema evidence. This acceptance kept `WP-INST-G1` incomplete while W3 stabilized the typed
pending/read boundary. W4 owns 1851 through 1860, W5 owns 1861 through 1865,
W7b owns executable PostgreSQL authority proof, and final `all_test.sh` remains required.

- **Accepted artifacts:** typed `SubmissionPreparation::AcceptedPending` and
  `SubmissionReceiptRead` contracts; answer-free `submission` and `submission_idempotency` parents;
  the composite-FK `accepted_submission_private_response` child; canonical UTF-8 response identity;
  equivalent Memory/PostgreSQL behavior; the dedicated worker-only execution store; separate API and
  worker process logins; and migrations 1849/1850.
- **Rust and focused evidence:** the learning-data-access full suite main target passed 308 tests
  with 1 intentionally ignored test, and auxiliary targets were green. Strict format, check, and
  Clippy gates passed. The focused policy/process/documentation/source set passed 2,008 tests.
- **Repository evidence:** `./check_codebase.sh` passed all 5 gates, including 356 Node tests.
- **Historical database evidence:** fresh PostgreSQL 17 applied all 80 migrations; the second migration pass
  was a no-op; database verification returned `database verify: compatible`. The repaired
  database-baseline
  Rust selector now resolves to exactly one intended test.
- **Independent approvals:** `sql_correctness_post_repair_review.report.md` approved the repaired
  SQL and `w2_security_post_repair_review.report.md` approved the repaired W2 source boundary. Their
  scope explicitly leaves W4 outcome behavior, W7b executable API-denial/worker-lease/RLS proof,
  browser behavior, WP-P2 grant reduction, and final G1 Validation open.

## G1-W3 accepted evidence

`WP-INST-G1 / G1-W3` is accepted on 2026-08-27 for the typed pending/read stabilization and
post-validation outcome classification. This acceptance advances the current stage to G1-W4; it
does not accept `WP-INST-G1`, whose W4-W7 work and final Validation remain required.

- **Accepted artifacts:** exhaustive `SubmissionReceiptRead` pending/read handling; the minimal
  answer-free, no-store `accepted_pending` 202 replay projection; closed deterministic grader
  failure categories and operation-reason mapping; Native, WebWork, QTI, and composite
  post-validation classification; the preserved opaque iMathAS broker boundary; and aligned Memory
  Student-attempt projection for accepted-pending detail reads.
- **Rust evidence:** `server_core` passed 384 tests with 3 intentional connected ignores, and all
  server integration and doctest targets were green. `learning-data-access` passed 308 tests with
  1 intentional ignore in its main target, with auxiliary targets green. Strict Clippy passed for
  both affected crates.
- **Repository evidence:** 3,643 documentation and source-policy checks passed. The permanent
  local route tests cover answer-free submitted projections and the no-store provider-free pending
  replay without services, timing, or fixture data.
- **Independent approvals:** the architecture and security/privacy reviews both approved the final
  W3 boundary. They confirm that W3 preserves answer-free Student data, generic deterministic
  failure handling, and the separate iMathAS broker while creating no acceptance, claim, outcome,
  job, or Student-client effect.
- **Handoff:** W4 consumes the sealed W3 pending/read and deterministic-category contracts before
  dispatching its paired first-effect, worker, and Student-status work. It owns allocated migration
  1851 schema/roles layer plus integrity, public-function authority, table authority, acquisition,
  read, load, completion-lock, commit, and fail capabilities through 1860. The aggregate
  `all_test.sh` remains the manager-owned final gate; a subagent aggregate invocation has no
  retained terminal result and is intentionally unverified.

## G1-W4 stable implementation handoff

`WP-INST-G1 / G1-W4` reached its stable implementation handoff on 2026-08-27. It advanced source
work to W5 while W7b prepared the executable PostgreSQL oracle and W7 prepared final
material-tree Validation.

- **Implemented boundary:** one immutable accepted-submission effect; split exact-fast-path and
  generic-recovery claims; type-distinct eagerly connected pools and service logins; one shared
  leased grading handler; canonical source/digest/projection evidence; atomic tuple-fenced
  load/lock/commit/fail; route-bound verified completed reads; and answer-free pending, attention,
  and completed Student projections.
- **Focused evidence:** `learning-data-access` passed 332 tests with 1 intentional connected ignore;
  `server_core` passed 413 tests with 3 intentional connected ignores; the five process-login tests
  and 1,754 source-length checks passed; strict Clippy, formatting, and diff hygiene were green.
- **Connected stabilization:** a fresh PostgreSQL 17 baseline applied all 90 tracked migrations,
  repeated migration as a no-op, passed compatibility and every registered connected phase, and
  left no disposable resources. The production-shaped headless stack then started the API, worker,
  and HTTPS gateway through the eager private-pool login, membership, and function-surface
  preflights; API and gateway were healthy, the worker remained running, and exact stop cleanup left
  no labelled container, network, or volume.
- **Independent review:** the initial review found lazy private-pool startup; the durable repair
  made typed factories eagerly connect and preflight their exact allowed and denied function
  surfaces.
  Re-review approved the resulting fail-closed composition with no remaining blocker in the W4
  source handoff.
- **Follow-on evidence:** W7b supplied
  `postgres_automated_grading_operations_live`, its database-baseline registration, exhaustive
  role/RLS/function proof, outcome and immutable-evidence behavior, ordinary-versus-worker parity,
  and the 1830-to-1831 score-publication sequence. G1-W7 completed the fresh HCI review and final
  `all_test.sh` material-tree Validation during G1 closeout.

## G1 accepted evidence

`WP-INST-G1` was accepted on 2026-08-28 after W5 through W7b, forward reconciliation, independent
review, and final material-tree Validation completed.

- **Implemented operation boundary:** the course-scoped Instructor list, retry, and recalculation
  routes use revision and idempotency fences. The immutable operation receipts and canonical
  scoring-invalidation capability keep the original Student receipt stable while the ordinary
  worker publishes only the current generation's total.
- **Student and Instructor journey:** the production HTTPS scenario submits Student work once,
  clears the browser answer buffer on `acceptedPending`, exposes **Check grading status**, routes a
  deterministic grader exception to Instructor attention, completes one visible retry, and shows
  the resulting total in the Instructor Gradebook. The focused
  `automated_grading_recovery` browser journey passed against the real stack.
- **Historical pre-reconciliation connected evidence:**
  `source source_me.sh && .venv/bin/python local_stack.py acceptance`
  passed against the historical pre-reconciliation 95-migration material tree, with the
  production browser suite, PostgreSQL baseline and oracles, isolated WebWork grading, API-replica
  restart and durable replay, and exact disposable resource cleanup.
- **Screenshot publication:** `source source_me.sh && ./capture_screenshots.sh` atomically
  published the current 63-artifact corpus after PNG, privacy, provenance, single-origin, and
  cleanup checks. The two G1 Instructor artifacts use the required 1280 by 800 desktop viewport;
  the operation artifact visibly confirms the canonical Question ID copy action.
- **Independent review:** architecture, security, and fresh G1 HCI rereviews returned ACCEPT. The
  HCI closeout found no P0/P1/P2 issue in the one-submit Student status flow, title-first copyable
  Question ID, target-specific retry, focused accepted confirmation, Student completion, or
  Gradebook propagation.
- **Forward reconciliation evidence:** accepted migration restoration and implementation of the
  four allocated forward migrations `2026081866` through `2026081869` are complete in order,
  beginning with the clean-volume fail-closed receipt preflight and ending with the V2 retry
  transition, public V1 retirement, and `DROP ... RESTRICT`. The fresh/no-op/checksum run applied
  and verified all 99 migrations; the connected G1 PostgreSQL oracle, forced-RLS inventory and role
  denials, deterministic browser recovery, isolated WebWork grading, and replica restart/durable
  replay passed with exact cleanup.
- **Final Validation:** `source source_me.sh && ./all_test.sh` passed on the final material tree.
  The exact aggregate passed Rust checks, tests, doctests, strict Clippy, and browser Wasm; all five
  frontend gates with 369 Node tests; 7,978 pytest checks; every canonical production-browser
  scenario; all 99 migrations and connected PostgreSQL/RLS/worker oracles; isolated WebWork;
  replica restart and durable replay; and exact disposable cleanup.

## T6 accepted evidence

`WP-INST-T6` was accepted on 2026-08-27. Its binding plan remains the acceptance authority, and
the completed handoff advanced to the accepted `WP-INST-G1` package.

- **Focused architecture and contracts: passed.** Migration `2026081848`, persisted incomplete
  Drafts, focused Questions and Policies commands, strict shared revisions, publication readiness,
  answer-free Student view, generic unexpected-error mapping, and the fixed-slot replacement route
  pass focused Rust, TypeScript, Node, lint, format, source-size, and static policy gates. The
  focused suite includes 19 Node tests and 375 runnable server tests; future-run replacement
  preserves issued snapshots while changing the authoritative question.
- **Connected live-demo journey: passed.** The production-shaped HTTPS owner passed the complete
  visible scenario selection, including independent Instructor and Sysadmin passkeys, assignment
  workspace authoring, same-assignment Student submission and Instructor gradebook observation,
  fixed-slot replacement, recovery, item pools, discovery, curation, reusable curricula, and
  curriculum adoption. Complete local-stack acceptance passed all 15 browser scenarios, the
  78-migration/DB oracle, WebWork oracle, replica restart, and exact cleanup.
- **Screenshot publication: passed.** The current 61-artifact corpus passed PNG, privacy, provenance,
  single-origin, atomic-publication, exact-cleanup, and human visual review. Instructor and Sysadmin
  evidence remains 1280 by 800 desktop-only; Student evidence retains its declared variable
  profiles.
- **Independent review: passed.** Final architecture/security and HCI/accessibility reviews return
  ACCEPT with no unresolved P0, P1, or P2 finding. The shared browser client enforces `no-store`
  for editor responses, and Questions provides title-bound controls and an accessible replacement
  summary.
- **Final Validation: passed.** `source source_me.sh && ./all_test.sh` passed on the exact final
  material tree, including Rust checks/tests/doctests/Wasm, frontend/codebase/Node, 7,428 pytest
  cases, all 15 production browser scenarios, all 78 migrations and database oracles, isolated
  WebWork, and replica restart/durable replay. The six durable closure paths formed part of that
  material tree:
  `crates/server/src/course/tests/assignment_revision/replacement.rs`,
  `docs/screenshots/instructor/assignment_workspace/01_assignment_policies.png`,
  `docs/screenshots/instructor/assignment_workspace/02_student_view.png`,
  `src/pages/assignment_workspace/assignment_workspace_authoring.css`,
  `tests/test_assignment_workspace_policy_summary.mjs`, and
  `tests/test_assignment_workspace_replacement_client.mjs`.

### Active-system invariants

- Use the canonical disposable production-shaped HTTPS stack and visible UI-created product state.
- Keep grading deterministic and server-owned; browser contracts remain answer-free.
- Preserve tenant isolation, immutable published content, draft-versus-publication identity,
  immutable evidence, and stateless API replicas.
- Keep the learning engine question-agnostic. Biology examples are fixtures rather than policy.
- Retain direct-entry evidence for the five fixed seeded personas. Elena Instructor and Morgan
  Sysadmin each retain an independent generic passkey journey.

## B2 accepted evidence

The B2 implementation and focused evidence are current as of 2026-08-26. The selected Graphify
query identified the README architecture/documentation surface, `migrations.rs`,
`CurriculumAdoptionLivePage`, `createCurriculumAdoptionClient`, and the curriculum-adoption
persistence bridges as the relevant communities; source inspection confirmed those ownership
boundaries and the allocated `2026081838` through `2026081847` migration set.

- **Focused PostgreSQL/RLS oracle: passed.** The ignored
  `postgres_curriculum_adoption_live::postgres_curriculum_adoption_is_brokered_atomic_and_recoverable`
  test passed against the allocated B2 schema, including broker authority, forced RLS, atomic
  adoption and recovery, provenance/receipt persistence, and reconciliation relationships.
- **Connected browser suite: passed.** All 15 production-shaped HTTPS journeys are green, including
  direct Sysadmin and Instructor passkey entry, authorization, authoring, preview, replacement,
  item pools, grading conflicts, Student delivery, discovery evidence, curation, reusable curricula,
  adoption and rollover, WebWork, gateway recovery, and QTI import.
- **Static and deterministic gates: passed.** The five-part codebase gate, 322 Node tests, 7,361
  pytest checks, complete Rust feature/Clippy/test/doctest matrix, browser Wasm target, focused
  scenario contracts, source limits, ASCII, Markdown links, and diff hygiene are green. Independent
  post-fix review returned ACCEPT with no unresolved P0, P1, or P2 finding.
- **Real-service gates: passed.** The 77-migration PostgreSQL/RLS/persistence baseline, isolated
  WebWork scoring and outage oracle, and API replica restart/replay oracle passed with exact cleanup.
- **Screenshot publication: passed.** At B2 acceptance, all 75 declared real-stack artifacts passed
  PNG integrity, privacy, provenance, atomic publication, and human visual review. Instructor and
  Sysadmin evidence used only the 1280 by 800 desktop profile; Student evidence retained the
  declared variable profiles.
- **Final Validation: passed.** `source source_me.sh && ./all_test.sh` completed on the published
  material tree, including the complete Rust, Node, pytest, production-browser, PostgreSQL,
  WebWork, replica-restart, and cleanup gates.

### B2 seeded course-model correction

The approved live-demo course-model correction defines recognizable ordinary teaching courses with ordinary active
memberships and Student work: `Biochemistry: Protein Structure and Function`, `Genetics: Foundations of Inheritance`,
and `Biochemistry: Molecular Foundations`. Installer diagnostics retain an internal recipe identity, while product
surfaces use the teaching-course title. Morgan and Avery retain their separate ordinary authorization course.
Blueprints are non-enrollable personal reusable assignments, and Alpha curricula are
non-enrollable shared curricula; each name stays exclusive to its corresponding reusable aggregate.

The corrected seed distributes five deterministic Student observations across meaningful ordinary Chapter 1
assignments titled `Molecular Foundations: Charged Functional Groups` in the Genetics and Biochemistry teaching
courses. Existing item-analysis and discovery surfaces present those observations in context through the ordinary course
evidence surfaces. Course navigation presents recognizable teaching courses from active server-owned relationships:
Instructor teaching membership, Student membership, and the Sysadmin's direct teaching membership or audited
support relation under ASVS 8.2.2 and 8.3.1. Seeded memberships provide representative course context.

Before first production deployment, the reviewed clean-cluster baseline reissues `2026081818` with the final visible
Biochemistry teaching title, and disposable live-demo volumes are regenerated from it. The resulting checksum is the
canonical immutable v1 baseline. This is the first shipped baseline, so its coherent title and topology belong in v1;
the general accepted-migration immutability rule governs the forward-only ledger after that reset and after v1 ships.

Validation classification for this correction is explicit: focused permanent relationship tests protect course,
membership, reusable-aggregate, observation, and navigation relationships; a fresh live-stack database and visual
walkthrough supplies one-time package evidence. Screenshot publication and complete Validation are green; B2 was
accepted on 2026-08-26.

## Shared migration ledger and allocation

The release integrator owns migration ordering and this ledger. The reviewed pre-production v1 reset above is the
explicit clean-cluster baseline decision. After v1 ships, accepted files are immutable; future schema packages receive
an allocation before implementation. Non-schema packages do not receive an implicit allocation.

| Allocation                | Package               | Current disposition                                                                                                                                                                                 |
| ------------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `2026080801`-`2026080806` | Foundational baseline | Accepted six-file baseline                                                                                                                                                                          |
| `2026080907`              | `WP-RC1`              | Accepted course appearance                                                                                                                                                                          |
| `2026080908`              | `WP-P2`               | Allocated secure question-grading payloads and the post-G1-W2 legacy-consumer/grant-reduction transition                                                                                            |
| `2026080909`              | `WP-RC8`              | Allocated passwordless identity and enrollment                                                                                                                                                      |
| `2026080910`              | `WP-RC7`              | Reserved object reconciliation                                                                                                                                                                      |
| `2026080911`              | `WP-RC9`              | Reserved LTI Advantage                                                                                                                                                                              |
| `2026080912`              | `WP-FU1`-`WP-FU6`     | Reserved secure Student uploads                                                                                                                                                                     |
| `2026080914`-`2026080935` | Release packages      | Existing forward allocations                                                                                                                                                                        |
| `2026081401`              | `WP-R0`               | Existing ranked-catalog allocation                                                                                                                                                                  |
| `2026081501`-`2026081504` | `WP-RC8` repairs      | Existing forward allocations                                                                                                                                                                        |
| `2026081801`              | `WP-INST-S2`          | Accepted term and time zone                                                                                                                                                                         |
| `2026081802`              | `WP-INST-S7`          | Accepted typed references and bylines                                                                                                                                                               |
| `2026081803`              | `WP-INST-S5`          | Accepted entitlement and materialization                                                                                                                                                            |
| `2026081804`              | `WP-INST-S3`          | Accepted effective-policy resolver                                                                                                                                                                  |
| `2026081805`              | `WP-INST-S4`          | Accepted disclosure policy                                                                                                                                                                          |
| `2026081806`              | `WP-INST-S6`          | Accepted course grade scheme                                                                                                                                                                        |
| `2026081807`              | `WP-INST-T2`          | Accepted teaching operations                                                                                                                                                                        |
| `2026081808`              | `WP-INST-LD1`         | Accepted live-demo installation state                                                                                                                                                               |
| `2026081809`              | `WP-INST-LD2`         | Accepted Sysadmin candidate and completed-install brokers                                                                                                                                           |
| `2026081810`              | `WP-INST-LD2`         | Accepted Student pre-tenant context repair                                                                                                                                                          |
| `2026081811`              | Reserved              | Reserved numeric identity                                                                                                                                                                           |
| `2026081812`              | `WP-INST-LD3`         | Accepted ordinary assignment mutation authority                                                                                                                                                     |
| `2026081813`              | Reserved              | Reserved numeric identity                                                                                                                                                                           |
| `2026081814`              | `WP-INST-LD3`         | Accepted assignment-definition capability                                                                                                                                                           |
| `2026081815`              | Reserved              | Reserved numeric identity                                                                                                                                                                           |
| `2026081816`              | `WP-INST-LD3`         | Accepted course-group mutation brokers                                                                                                                                                              |
| `2026081817`              | `WP-INST-LD3`         | Accepted Student-work source and execution snapshots                                                                                                                                                |
| `2026081818`              | `WP-INST-LD3`         | Canonical v1 course provisioning and installed-course attestation                                                                                                                                   |
| `2026081819`              | `WP-INST-LD3`         | Accepted grade control and export audit                                                                                                                                                             |
| `2026081820`              | `WP-INST-LD3`         | Accepted scoring preparation and finalization                                                                                                                                                       |
| `2026081821`-`2026081822` | Reserved              | Reserved numeric identities                                                                                                                                                                         |
| `2026081823`              | `WP-INST-LD3`         | Accepted teaching-invitation mutation authority                                                                                                                                                     |
| `2026081824`              | `WP-INST-LD3`         | Accepted roster procedure ambiguity repair                                                                                                                                                          |
| `2026081825`              | `WP-INST-LD3`         | Accepted inactive-Student materialization decision                                                                                                                                                  |
| `2026081826`              | `WP-INST-T5`          | Accepted pre-issue assignment-definition replacement                                                                                                                                                |
| `2026081827`              | `WP-INST-D1`          | Accepted discovery evidence and response-family projection                                                                                                                                          |
| `2026081828`              | `WP-INST-D1`          | Accepted actor usage snapshots and Library facets                                                                                                                                                   |
| `2026081829`              | `WP-INST-LD3`         | Reserved Student-work broker capability                                                                                                                                                             |
| `2026081830`              | `WP-INST-G1`          | Reserved assignment recalculation enqueue capability                                                                                                                                                |
| `2026081831`              | `WP-INST-G1`          | Reserved scoring-generation publication                                                                                                                                                             |
| `2026081832`              | `WP-INST-G3`          | Reserved item-analysis publication and cleanup                                                                                                                                                      |
| `2026081833`              | `WP-INST-T5`          | Reserved assignment-definition scratch isolation                                                                                                                                                    |
| `2026081834`              | `WP-INST-LD3`         | Reserved course-group policy broker repair                                                                                                                                                          |
| `2026081835`              | `WP-INST-LD1`         | Reserved catalog-derived Base Course freshness authority                                                                                                                                            |
| `2026081836`              | `WP-INST-D2`          | Accepted problem curation capabilities                                                                                                                                                              |
| `2026081837`              | `WP-INST-B1`          | Accepted blueprint and public Alpha capabilities                                                                                                                                                    |
| `2026081838`              | `WP-INST-B2`          | Accepted curriculum-adoption schema, lineage, schedule, provenance, receipt, integrity, and forced RLS foundation                                                                                   |
| `2026081839`              | `WP-INST-B2`          | Accepted curriculum-adoption common broker authority, retention integration, and shared capability boundary                                                                                         |
| `2026081840`              | `WP-INST-B2`          | Accepted curriculum-adoption relational snapshots, locked preparation, inspection, and reconciliation helpers                                                                                       |
| `2026081841`              | `WP-INST-B2`          | Accepted canonical ordinary-course topology, issued-work fencing, and topology capability assertions                                                                                                |
| `2026081842`              | `WP-INST-B2`          | Accepted curriculum-adoption source authorization, closed request validation, and source snapshot facts                                                                                             |
| `2026081843`              | `WP-INST-B2`          | Accepted teaching-course, import, inspection, reconciliation, and controlled schedule snapshot facts                                                                                                |
| `2026081844`              | `WP-INST-B2`          | Accepted curriculum-adoption shared materializer validation, idempotency, receipt, and evidence helpers                                                                                             |
| `2026081845`              | `WP-INST-B2`          | Accepted fork, assignment adoption, fast-forward, and reconciliation materializers                                                                                                                  |
| `2026081846`              | `WP-INST-B2`          | Accepted whole-course instantiation, rollover, and term-shift materializers                                                                                                                         |
| `2026081847`              | `WP-INST-B2`          | Accepted canonical public bridge completion and final broker catalog assertions                                                                                                                     |
| `2026081848`              | `WP-INST-T6`          | Allocated assignment-workspace capability migration: empty Draft/Archived definitions and Published readiness                                                                                       |
| `2026081849`              | `WP-INST-G1`          | Accepted W2 operation/evaluation/execution schema prerequisite and receipts                                                                                                                         |
| `2026081850`              | `WP-INST-G1`          | Accepted W2 private accepted-response, acceptance/replay, retention/RLS, and lease-fenced execution boundary                                                                                        |
| `2026081851`              | `WP-INST-G1 / G1-W4`  | Schema and roles; proof: fresh schema/role shape query                                                                                                                                              |
| `2026081852`              | `WP-INST-G1 / G1-W4`  | Integrity guards and triggers; proof: immutable-write rejection                                                                                                                                     |
| `2026081853`              | `WP-INST-G1 / G1-W4`  | Public function authority; proof: effective catalog closes PUBLIC/default EXECUTE and legacy v1 load                                                                                                |
| `2026081854`              | `WP-INST-G1 / G1-W4`  | Witness/RLS/table authority and receipt version SELECT; proof: exact authority and ACL matrix                                                                                                       |
| `2026081855`              | `WP-INST-G1 / G1-W4`  | Split generic/exact claim and ready/max convergence; proof: one winner and sibling denial                                                                                                           |
| `2026081856`              | `WP-INST-G1 / G1-W4`  | Four-key structural verified read; proof: entitled route succeeds and changed key fails                                                                                                             |
| `2026081857`              | `WP-INST-G1 / G1-W4`  | Exact private execution load; proof: exact claim loads once and rejects mismatches                                                                                                                  |
| `2026081858`              | `WP-INST-G1 / G1-W4`  | Completion lock; proof: lock fences stale or duplicate completion                                                                                                                                   |
| `2026081859`              | `WP-INST-G1 / G1-W4`  | Commit-v2; proof: full 36-input signature commits one immutable aggregate                                                                                                                           |
| `2026081860`              | `WP-INST-G1 / G1-W4`  | Fail; proof: NULL-safe closed failure validation preserves invalid-call state                                                                                                                       |
| `2026081861`              | `WP-INST-G1 / G1-W5`  | W5 Instructor grading-operation capability and broker surface                                                                                                                                       |
| `2026081862`              | `WP-INST-G1 / G1-W5`  | W5 worker-authoritative grading-operation lifecycle projection                                                                                                                                      |
| `2026081863`              | `WP-INST-G1 / G1-W5`  | W5 immutable scoring-invalidation origin evidence                                                                                                                                                   |
| `2026081864`              | `WP-INST-G1 / G1-W5`  | W5 canonical generation, job, operation, and supersession capability                                                                                                                                |
| `2026081865`              | `WP-INST-G1 / G1-W5`  | W5 source-specific invalidation witnesses and least-privilege adapters                                                                                                                              |
| `2026081866`              | `WP-INST-G1 / G1-W7`  | `2026081866_g1_receipt_provenance_schema.sql`: clean-volume preflight; execution and operation receipt provenance/category schema and closed constraints                                            |
| `2026081867`              | `WP-INST-G1 / G1-W7`  | `2026081867_g1_execution_receipt_writers.sql`: acceptance, claim, and failure writer bodies; exact ACL/catalog proof                                                                                |
| `2026081868`              | `WP-INST-G1 / G1-W7`  | `2026081868_g1_completion_receipt_writer.sql`: 36-input commit-v2 body and narrow ACL/catalog proof                                                                                                 |
| `2026081869`              | `WP-INST-G1 / G1-W7`  | `2026081869_g1_instructor_receipt_writers.sql`: V2 retry transition, public retry routing/V1 retirement, and final (1865) recalculation body with broker ACL proof                                  |
| `2026081870`              | `WP-INST-G2 / G2-W3B` | `2026081870_student_work_inspection_authority.sql`: dedicated inspection owner, fixed search path, baseline revocations, narrow grants, and catalog proof                                           |
| `2026081871`              | `WP-INST-G2 / G2-W3B` | `2026081871_student_work_inspection_witness.sql`: private immutable receipt/presentation/response witness, integrity boundary, and catalog proof                                                    |
| `2026081872`              | `WP-INST-G2 / G2-W3B` | `2026081872_student_work_inspection_capability.sql`: only app-executable inspection broker, parameter-bound composite resolution, atomic audits, and closed ACL proof                               |
| `2026081873`              | `WP-INST-G2 / G2-W3B` | `2026081873_student_work_inspection_indexes.sql`: query-demonstrated inspection/audit indexes with retained closed broker authority                                                                 |
| `2026081874`              | `WP-INST-G2 / G2-W3A` | `2026081874_tenant_bound_worker_failure.sql`: tenant-bound queue failure capability and publisher adapter with the unscoped V1 surface retired                                                      |
| `2026081875`              | `WP-INST-G2 / G2-W3B` | `2026081875_student_work_inspection_rowset_contract.sql`: forward repair aligning the broker's transient JSON rowset with exact PostgreSQL field names                                              |
| `2026081876`              | `WP-INST-G2 / G2-W3B` | `2026081876_student_work_inspection_safe_labels.sql`: server-owned validated Student display label and assignment title returned by the existing audited inspection broker, without entering audits |
| `2026081877`              | `WP-INST-G2 / G2-W5`  | `2026081877_base_course_accepted_submission_completion.sql`: host-only fast-path identity, exact typed queue claim, and accepted-private-response-aware Base Course completion verification         |
| `2026081878`              | `WP-INST-G2 / G2-W5`  | `2026081878_gradebook_operation_selection.sql`: execute-only Instructor broker for exact public grading-operation Gradebook selection without direct application-table access                       |
| `2026081879`              | `WP-INST-WN1 / WN1-D` | Course-authority broker ownership, narrow RLS policies, explicit ACLs, and forced RLS                                                                                                               |
| `2026081880`              | `WP-INST-WN1 / WN1-D` | Exact authority-function argument rebinding and dependent recreation with unchanged authorization behavior                                                                                          |
| `2026081881`              | `WP-INST-WN1 / WN1-D` | Student-role schema vocabulary for effective relations, columns, constraints, indexes, and catalog fingerprints                                                                                     |
| `2026081882`              | `WP-INST-WN1 / WN1-D` | Student-work broker, policy, trusted-function, fence, grant, ownership, and SQLx vocabulary                                                                                                         |
| `2026081883`              | `WP-INST-WN1 / WN1-MG` | Automated-only scoring constraints and manual-grade persistence retirement                                                                                                                         |
| `2026081884`              | `WP-INST-WN1 / WN1-D` | Direct Student-work payload contracts for current run, attempt, submission, feedback, and summary records                                                                                           |
| `2026081885`              | `WP-INST-WN1 / WN1-D` | Canonical receipt payload V2 for new immutable evidence while retaining V1 bytes and readers                                                                                                        |
| `2026081886`              | `WP-INST-WN1 / WN1-D` | Catalog, workspace, publication, and flat-asset payload contracts                                                                                                                                   |
| `2026081887`              | `WP-INST-WN1 / WN1-D` | Curriculum-adoption request, inspection, and reconciliation payload contracts                                                                                                                       |
| `2026081888`              | `WP-INST-WN1 / WN1-D` | Operational worker, retention, delivery, roster/account, provider-cache, and export payload contracts                                                                                               |

`2026081803` (`S5`), `2026081804` (`S3`), and `2026081805` (`S4`) reflect the accepted
pre-file allocation reorder. Allocations `2026081811`, `1813`, `1815`, `1821`, and `1822` retain
their numeric identities. T6 owns `2026081848`; G1 accepted `2026081849` and `2026081850` in
addition to reserved enqueue/publication capabilities `2026081830` and `2026081831`. G3 retains
`2026081832`. G1-W4 owns ordered forward allocations `2026081851` through `2026081860`: schema/roles,
integrity, public-function authority, table authority, claim, read, load, completion lock, commit,
then fail. G1-W5 owns `2026081861` through `2026081865`: Instructor operations, lifecycle
projection, immutable invalidation origins, the canonical invalidation capability, and
source-specific least-privilege witnesses. The seven accepted migrations are restored
byte-for-byte. The four allocated closeout migrations are implemented in order: migration 1866
fails closed when either `grading_execution_receipt` or `grading_operation_receipt` is nonempty
before adding provenance/category fields; it preserves immutable receipt history. Migration 1869
creates the five-input actor-bound retry V2 capability, routes the unchanged public retry caller
through it, revokes V1 execute, and drops the four-input V1 with `RESTRICT`. The 99-migration live
database, RLS, worker, browser, WebWork, and replica-restart evidence is green. These rows remain
allocated, and G1-W7 plus `WP-INST-G1` are accepted on the final 99-migration material tree. The
Instructor plan owns dependencies among reserved capabilities.

## Accepted package pointers

| Package                     | Current durable result                                                    | Owning evidence                                                                                    |
| --------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `WP-INST-LD1`               | Base Course installation lifecycle and retained-state rules               | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-LD2`               | Seeded entry and connected live authoring boundary                        | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-LD3`               | Ordinary live assignment, Student work, and immutable evidence path       | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-T5`                | Fixed-or-pool assignment editing and deterministic issued draws           | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-T6`                | Accepted assignment workspace, focused replacement, and live Student view | [T6 plan](active/instructor_assignment_workspace_plan.md), [changelog](../CHANGELOG.md)            |
| `WP-INST-D1`                | Canonical Library discovery and evidence-backed question detail           | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-D2`                | Live curation and shared problem selection                                | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-B1`                | Revisioned Blueprints, public Alpha curricula, and shared reuse           | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-B2`                | Curriculum adoption, rollover, term shifting, and controlled update       | [Instructor plan](active/instructor_capability_architecture_plan.md), [changelog](../CHANGELOG.md) |
| `WP-INST-G1`                | Automated-grading exception routing, retry, and recalculation             | [G1 plan](active/automated_grading_operations_plan.md), [changelog](../CHANGELOG.md)               |
| `WP-INST-G2`                | Calculated Gradebook and audited Student-work inspection                  | [G2 plan](active/audited_student_work_gradebook_plan.md), [changelog](../CHANGELOG.md)             |
| `WP-R0`-`WP-R2`, `WP-PY-L1` | Accepted cross-roadmap capabilities                                       | [Release plan](active/release_completion_plan.md), [changelog](../CHANGELOG.md)                    |

## Dependency-ordered queue

The authoritative package sequence is in the [release completion plan](active/release_completion_plan.md)
and [Instructor capability plan](active/instructor_capability_architecture_plan.md). The current
handoff is:

1. Complete current `WN1-SR4-browser-direct-clients`. `WN1-OPS1` through `WN1-OPS10`, WN1-B1
   through B5, WN1-GO1, WN1-MG, and WN1-SR1 through SR3 are accepted.
   Atomic C1-C6 and
   source/type-level QM children may run in parallel only where the ledger proves disjoint ownership;
   WN1-WA and WN1-D follow their affected producer dependencies and feed WN1-F. The approved
   [wire naming contract migration plan](active/wire_naming_contract_migration_plan.md) owns these
   dependencies; WN1-F runs `source source_me.sh && ./all_test.sh` and the exact
   `NAMING_CONVENTIONS.md` review before acceptance.
2. Resume `WP-INST-G2` W5/W6 visual and documentation close-out only after WN1 acceptance; G2 remains
   acceptance-open until its own named gates pass. Then implement `WP-INST-G3-IA1` as the visible
   Instructor item-analysis successor before continuing G3 through G5 and E1/E2 in the Instructor
   plan's declared dependency order.
3. Resume the release queue at `WP-RC8`, then follow the release plan through native-family,
   Student-payload, reconciliation, LTI, upload, deployment, cost-control, and release closure
   packages.
4. Run the complete final-material-tree Validation suite before declaring the goal complete.

## Operational references

- [LIVE_DEMO_SPEC.md](../LIVE_DEMO_SPEC.md) defines the live demo behavior.
- [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md) defines required Validation evidence.
- [DEVELOPMENT.md](../DEVELOPMENT.md), [INSTALL.md](../INSTALL.md), [USAGE.md](../USAGE.md), and
  [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) own operational instructions.
- The dated comparison snapshot is
  [project_status_report_2026-08-10.md](reports/project_status_report_2026-08-10.md); older status
  notes and `partial_commit_status.md` are historical references.
