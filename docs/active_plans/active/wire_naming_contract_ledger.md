# WN1 contract migration ledger

Status: `WP-INST-WN1-A` allocation accepted 2026-08-28 after a fresh independent v3 review. A
2026-08-29 current-source follow-up added the previously unallocated generated-output and shell
operation children below. This ledger records source-verified ownership for the naming migration;
it remains planning evidence, not implementation or product acceptance.

## Contract and role boundary

Rust Serde owns PLE data-object properties, PLE query keys, and portable discriminants. They use
`snake_case`. `tsgen` generates one direct `Foo` from effective Serde, with the Rust type name and
exact effective field names. Route-only values belong to pure `crates/browser-api-contract`; Axum
projects domain or Store state to that value once. TypeScript functions, locals, signals,
components, and UI-only state retain `lowerCamelCase`. DOM, framework, header, static URL, and
registered external protocol spelling remains with its owner.

PLE role-bound product identifiers converge to **Student**, **Instructor**, and **Sysadmin**.
Generic authenticated identity remains `user`; generic educational-system concepts retain
`learning`. Each implementation closure changes its Serde producer, generated contract, client,
decoder, Store, durable reader, and behavior gate together.

Source inventory, generated-output review, clean-volume rebuild, connected browser/provider run,
screenshot capture, migration inspection, digest comparison, and visual review are one-time
evidence. Permanent tests assert observable deterministic behavior.

## Dependency order

```text
accepted WN1-A allocation
  -> WN1-OPS1 root-script naming
  -> WN1-B direct generator and contract root
  -> WN1-GO1 orphaned generated-output retirement after WN1-B5
  -> WN1-MG automated-only grading closure
  -> dependency-ready C and QM rows
  -> WA and D rows
  -> WN1-OPS2..OPS10 PLE-owned shell-family closures before final acceptance
  -> WN1-SR4A Student authority source closure
  -> WN1-SR5 PostgreSQL vocabulary
  -> WN1-SR6 product-document review and WN1-FD filename disposition
  -> WN1-F naming review and full material-tree acceptance
  -> G2 W5/W6, then WP-INST-G3-IA1
```

All rows below are local WN1 subtasks under the globally unique `WP-INST-WN1` package identity.

### WN1-OPS1 root-script naming

`WN1-OPS1` owns the script-private variable conversion in `run_live_demo.sh` and `build.sh`.
It changes `SCRIPT_DIRECTORY` to `script_directory`, `COMMAND` to `command`, and `HEADLESS` to
`headless` in `run_live_demo.sh`; it changes `SCRIPT_DIRECTORY` to `script_directory`, `PROFILE`
to `profile`, `STAGE_NAMES` to `stage_names`, `STAGE_TIMES` to `stage_times`, `BUILD_START` to
`build_start`, `CARGO_PROFILE_FLAG` to `cargo_profile_flag`, `WASM_PROFILE_FLAG` to
`wasm_profile_flag`, `BUILD_END` to `build_end`, and `TOTAL` to `total` in `build.sh`. Exported
process configuration remains `SCREAMING_SNAKE_CASE`. Shell syntax and each script's visible
usage/build behavior are permanent gates. The current root-script variable inventory is one-time
evidence. `WN1-OPS1` depends on accepted WN1-A allocation and completes before WN1-F.

### WN1-OPS2 through WN1-OPS10 shell-family naming

Each child changes one PLE-owned script family from private `SCREAMING_SNAKE_CASE` state to
lowercase `snake_case`, preserving exported process names and explicit immutable test constants.
The children are independently completable after WN1-A and all complete before WN1-F.

| Child                            | Owned scripts                                                     | Private names                                                                                                                          | Focused behavior evidence                                                |
| -------------------------------- | ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `WN1-OPS2-root-aggregate`        | `all_test.sh`                                                     | `SCRIPT_DIRECTORY`                                                                                                                     | Shell syntax now; the required WN1-F aggregate exercises the front door. |
| `WN1-OPS3-browser-front-doors`   | `capture_screenshots.sh`, `run_playwright_tests.sh`               | `SCRIPT_DIRECTORY`                                                                                                                     | Shell syntax, screenshot help, and the next canonical browser lane.      |
| `WN1-OPS4-rust-front-door`       | `check_rust.sh`                                                   | `SCRIPT_DIRECTORY`                                                                                                                     | Shell syntax, help, and the next ordinary Rust gate.                     |
| `WN1-OPS5-wasm-build`            | `pipeline/build_wasm.sh`                                          | `SCRIPT_DIRECTORY`, `CARGO_PROFILE`, `PROFILE_DIR`, `WASM_INPUT`                                                                       | Shell syntax and the existing debug Wasm build/consumer lane.            |
| `WN1-OPS6-python-setup`          | `devel/setup_python.sh`                                           | `REPO_ROOT`, `VENV_DIRECTORY`, `VENV_PYTHON`, `RECEIPT_PATH`, `PYTHON_312`                                                             | Shell syntax and existing setup receipt reuse/refresh behavior.          |
| `WN1-OPS7-wasm-runner-setup`     | `devel/setup_wasm_tests.sh`                                       | `RUNNER_PACKAGE_ID`, `RUNNER_VERSION`, `RUNNER_ROOT`, `RUNNER`, `ACTUAL_VERSION`                                                       | Shell syntax and the matched-runner setup path.                          |
| `WN1-OPS8-e2e-course-appearance` | `tests/e2e/e2e_course_appearance.sh`                              | `SCRIPT_DIRECTORY`, `COMPOSE_STARTED`, `ENV_FILE`, `MANIFEST_FILE`, `CAPABILITY_FILE`, `PROJECT_NAME`                                  | Shell syntax and the existing connected course-appearance lane.          |
| `WN1-OPS9-e2e-database-baseline` | `tests/e2e/e2e_database_baseline.sh`                              | `SCRIPT_DIRECTORY`, `TEMP_DIR`, `COMPOSE_STARTED`, `GATE_FAILURES`, `PROJECT_NAME`, `POSTGRES_VOLUME_NAME`, `EXPECTED_MIGRATION_COUNT` | Shell syntax and the existing database-baseline owner path.              |
| `WN1-OPS10-e2e-orchestrators`    | `tests/e2e/e2e_run_all.sh`, `tests/e2e/e2e_webwork_render_rpc.sh` | `SCRIPT_DIRECTORY`, `PASSED`, `FAILED`, `FAILED_NAMES`                                                                                 | Shell syntax and the complete eight-lane non-browser aggregate.          |

`source_me.sh` and command-scoped process configuration retain their registered uppercase
environment spelling. Explicit `readonly` E2E fixture constants retain uppercase constant names.
The propagated `check_codebase.sh`, `devel/clean_build.sh`, and `devel/dist_clean.sh` remain with
their shared template owner. Current-source inventory is one-time evidence; these children add no
permanent source-name test.

### WN1-B direct generator and pure contract foundation

Split `crates/project-tools/src/tsgen.rs` into facade, source-discovery, Serde-resolution, model,
output, and test modules. Add the pure contract root, explicit `question_model` plus
`browser-api-contract` discovery, duplicate public-name refusal before cleanup, and direct per-type
output. The ordered local subtasks are `WN1-B1-contract-root`, `WN1-B2-source-model`,
`WN1-B3-types`, `WN1-B4-render`, and `WN1-B5-runner`. `WN1-B4` preserves exact Rust constant
identity for generated constant-module filenames. Focused project-tools/crate-boundary behavior and
TypeScript compilation are permanent evidence; regeneration and source-size review are one-time.

### WN1-GO1 orphaned generated-output retirement

`WN1-GO1` depends on accepted WN1-B5 and removes exactly the obsolete `ts-rs` outputs
`crates/question_model/bindings/Capability.ts` and
`crates/question_model/bindings/BackendCapabilities.ts`. Current consumers use the direct
`generated/api/` declarations owned by `crates/project-tools`; no production import reaches the
legacy pair. This child removes the competing output owner while `QM-CAPABILITY` separately owns
the capability discriminants' `snake_case` conversion.

Existing project-tools generator behavior and TypeScript compilation are permanent gates.
Regeneration ownership, consumer search, and retired-path inventory are one-time evidence; this
child adds no deletion-count test. `WN1-GO1` completes before WN1-F.

### WN1-MG automated-only scoring and artifact publication

`WN1-MG` owns the following source and persistence closure:

- Retire `run/manual_grading.rs`, the human-credit route and route policy,
  `ManualGradingStore`, `ManualEvaluation*`, `ManualCredit`, `SetManualGradeCommand`, and
  `SubmitPendingManualQuestionAttemptCommand`.
- Retire their Memory/PostgreSQL implementations, manual receipts/tables/functions/policies,
  `NeedsManualGrading` status/outcome variants, and `AnswerKey::FileUpload { rubric }`.
- Publication validation rejects a graded `ResponseDefinition::FileUpload` until FU supplies a
  versioned server-supported deterministic validator and artifact grader with immutable
  accepted-artifact evidence.
- Preserve `GradingOperationStore` retry/recalculation, ordinary worker publication, current
  Gradebook, and roster computed-score CSV. Accepted/auto-submitted work reaches a deterministic
  grade, exemption, or answer-free `AutomatedException`; human-entered credit is unavailable.

Existing retry, Store-conformance, publication, and route-policy behavior tests are permanent.
Retirement inventory and clean-volume installation are one-time. This closure precedes C3,
C4-IA1, affected QM rows, and D3.

## Route closures

Every C row moves its route-only request or response type into `browser-api-contract`, then changes
producer, query parser/builder, direct client/decoder, and narrow gate together. `B` means accepted
WN1-B. Each migrated PLE boundary accepts snake fields and rejects retired camel or unknown PLE
fields.

### C1 calculated Gradebook and roster

| Child   | Producer                                                      | Direct reader and permanent gate                                                                                                                                                                  | Depends                            |
| ------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| `C1-G1` | `course/gradebook/calculated.rs`; Gradebook GET               | `CalculatedGradebookView`; calculated Gradebook client/decoder/page-model and Rust tests cover `page_size`, `assignment_ref`, `membership_ref`, `operation_ref`, answer-free/no-store data.       | B; QM-IDENTITY/COURSE/PRESENTATION |
| `C1-G2` | `course/gradebook/selection.rs`; Student/operation selection  | `GradebookSelectionView`; navigation/route tests cover selection identity, cursor, and no-store return context. Owns target selection/query examples in `audited_student_work_gradebook_plan.md`. | C1-G1                              |
| `C1-G3` | selection submitted-run chooser                               | `SubmittedRunChoicesView`; chooser/client/route tests cover run identity and snake query. Owns target chooser examples in `audited_student_work_gradebook_plan.md`.                               | C1-G2                              |
| `C1-G4` | `course/gradebook/inspection.rs`; audited Student-work detail | `InspectedStudentWorkDetailView`; inspection tests cover solution-free projection, atomic audit, return context. Owns target inspection examples in `audited_student_work_gradebook_plan.md`.     | B; QM-PRESENTATION/ACTIVITY        |
| `C1-R1` | `course/roster.rs`; roster, invitation/member, policy         | Enrollment client/decoder and roster tests cover `roster_id`, `roster_email`, `page_size`, revisions, bounded PII. Owns target roster/invitation/policy examples in `ENROLLMENT_DESIGN.md`.       | B; QM-IDENTITY/COURSE              |
| `C1-R2` | `course/roster/import.rs`; CSV preview/commit                 | Preview/commit DTOs; tests cover row selection, invalid-cell withholding, bounds, external CSV header `email,roster_id`. Owns target CSV examples in `ENROLLMENT_DESIGN.md`.                      | C1-R1                              |
| `C1-R3` | `course/roster/export.rs`; grade-export CSV                   | Computed score export reader; tests preserve `roster_id,email,display_name,score`, CSV media type, safe filename, and protected delivery.                                                         | C1-R1                              |

### C2 account and passkey wrappers

| Child   | Producer                                                                              | Direct reader and permanent gate                                                                                                                                                                                                | Depends               |
| ------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| `C2-A1` | `auth.rs`; session/logout                                                             | `AuthSessionResponse`, `AuthUserResponse`, `SignedOutResponse`; auth client/decoder/Rust tests cover `display_name`, closed roles, cookie/origin/no-store.                                                                      | B; QM-IDENTITY        |
| `C2-A2` | `auth/passwordless.rs`, email change; passwordless/email/invitation/account selection | Account/passwordless DTOs; enrollment/passwordless tests cover snake request/query/result, pagination/revision, generic-error/rate-limit/provider isolation. Owns target account/invitation examples in `ENROLLMENT_DESIGN.md`. | B; QM-IDENTITY/COURSE |
| `C2-A3` | seeded account selector                                                               | `SelectSeededAccountRequest` and selector results; live-demo client/UI/Rust tests cover persona literals, closed body, origin, rate-limit, and unavailable concealment.                                                         | B                     |
| `C2-A4` | WebAuthn wrappers/list/revoke                                                         | PLE wrapper DTOs; enrollment/WebAuthn tests cover `ceremony_id`, PLE timestamps/no-store; nested W3C data remains registered protocol data.                                                                                     | B; QM-IDENTITY        |

### C3 run, tool, preview, validation

| Child         | Producer                                                     | Direct reader and permanent gate                                                                                                                                                                                                              | Depends                                                   |
| ------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `C3-R1`       | `run/{routes,queries,prefetch,submission,support}.rs`        | Run DTO closure; request/run/submission/question/presentation decoders and run/attempt/recovery/secrecy tests cover pending replay, disclosure/lifecycle, bounds, and answer secrecy. Owns qualifying-run examples in `ENROLLMENT_DESIGN.md`. | B; WN1-MG; QM-ACTIVITY/CONTENT/PRESENTATION; WN1-SR1..SR4 |
| `C3-R2`       | `run/external_tool.rs`                                       | `ExternalToolLaunch` and PLE envelope; response/request/question delivery/iMathAS projection tests cover `launch_url`, same-origin, and safe outage/replay.                                                                                   | C3-R1                                                     |
| `C3-P1`       | `author_preview.rs`                                          | Preview query/response/reason; page decoder/Rust tests cover revision/ETag, availability, no-store, answer-key exclusion.                                                                                                                     | B; QM-CONTENT                                             |
| `C3-V1/V2/V3` | response-format, timer, and assignment-capability validation | Direct request/report/verdict values; widget/Wasm/Rust tests cover bounded key-free violations, parity, authenticated fallback, capability/reason/bounds.                                                                                     | B; QM-CONTENT/CAPABILITY/ACTIVITY/ASSIGNMENT              |

### C4 catalog, authoring, item analysis, and curation

| Child                         | Producer                                                                                          | Direct reader and permanent gate                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Depends                                     |
| ----------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `C4-CAT-Q1/Q2/Q3/Q4/P1`       | catalog browse/search/public-reference/detail/publication                                         | Direct catalog values; narrow tests cover `page_size`, `next_cursor`, repeated arrays/duplicate refusal, public identity, redaction, bounded evidence, revision/ETag/no-store.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | B; QM-CATALOG/CONTENT/LIFECYCLE/IDENTITY    |
| `C4-NAV1`                     | `navigation.rs` public-reference resolution                                                       | `NavigationResolution`; tests cover reference-kind binding, authorization, and safe authority boundary.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | B; QM-IDENTITY                              |
| `C4-WS-Q1/Q2/M1/M2/P1/P2`     | workspace list/detail/save/delete/publication validation/diff                                     | Direct workspace values; existing narrow tests cover bounded identity, revision/ETag, closed nested keys, redaction, and no-store.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | B; QM-CONTENT/CAPABILITY/LIFECYCLE          |
| `C4-FQA1/FQA2/FQP1/FQP2/FQP3` | flat asset list/upload and native source GET/PUT/publication                                      | Asset descriptors/receipts and native values; existing asset/Store/source tests cover checksums, opaque bytes, revisions, redaction, and external native-source seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | B; QM-CONTENT/IDENTITY/LIFECYCLE; WA-NATIVE |
| `C4-IA1`                      | `item_analysis.rs`; assignment analysis GET; domain and Memory/PostgreSQL report readers          | Direct `CourseItemAnalysisResponse`, `AssignmentItemAnalysisResponse`, and `ItemAnalysisResponseDistribution`; client/decoder/API capability and server/Node behavior cover finite/range/redaction/no-store. Replace `incomplete_manual_grading` with `incomplete_scoring` and `pending_manual_attempt_count` with `unscored_attempt_count`. Retire `PendingManual` and `pending_manual`: the distribution contains only `Correct`, `Partial`, `Incorrect`, and deterministic `Unanswered`, and its total equals `graded_attempt_count`. `unscored_attempt_count` is outside the distribution; when it makes the aggregate incomplete, `incomplete_scoring` is true and score-derived metrics are null. | B; WN1-MG; QM-STATS/ACTIVITY                |
| `C4-PC-Q1/Q2/M1/M2/S1/S2/S3`  | curation collection/list/member/create/replace/delete and saved-search list/create/replace/delete | Direct collection/search values; existing narrow tests cover access, public Question IDs, ETag/revision, bounds, unique ordering, and current-catalog execution.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | B; QM-CATALOG                               |

`WP-INST-G3-IA1` follows WN1-F, accepted G2, and D1. It owns an Instructor Analysis task that
links aggregate item results to assignment titles/public Question IDs and existing Gradebook
inspection, Library/source, and Questions replacement. It consumes C4-IA1; it adds no score
mutation, artifact access, report SQL, or G4 decision record.

### C5 curriculum and QTI

| Child         | Producer                                          | Direct reader and permanent gate                                                                                                                                                                                        | Depends                         |
| ------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| `C5-CA1`      | curriculum adoption/rollover/term shift/reconcile | Curriculum request/preview/completed/inspection DTOs; tests cover preview-command witness, public IDs, idempotent replay/recovery.                                                                                      | B; QM-CURRICULUM/COURSE         |
| `C5-Q1/Q2/Q3` | QTI import/conversion/publication                 | PLE report, conversion, and publication DTOs; tests cover bounded answer-free report/digest/status/no-store, review acknowledgement/revision, and publication failure. QTI ZIP/XML/profile identifiers remain external. | B; WA-QTI; QM-CONTENT/LIFECYCLE |

### C6 course, assignment, and teaching operations

`course/routing.rs` becomes shared nonserializing topology/state/body-limit support: it contains no
Serde derive, manually spelled PLE JSON key, or PLE query object. Static paths retain their
protocol-owned kebab spelling. `course/pagination.rs` owns `CoursePageQuery` and `PageRequest`
conversion; `course/course_listing.rs` owns course list/create/get and term failure mapping;
`course/assignments/listing.rs` owns assignment listing; and
`course/assignments/strict_request.rs` owns typed assignment-request comparison.

| Child            | Producer                                                                                                        | Direct reader and permanent gate                                                                                                                                                                                                                                                                                                             | Depends                                            |
| ---------------- | --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `C6-CR1`         | `course/pagination.rs` `CoursePageQuery`/`PageRequest`, and `course/course_listing.rs` list/create/get course   | Course query/create/page DTOs; tests accept `page_size`, reject `pageSize` and unknown keys, validate cursor/page bounds, preserve Instructor-only creation/no-store, and strictly decode course/term results. Owns target course-term examples in `CONTRACTS.md`, `API_CONTRACTS.md`, and `ENROLLMENT_DESIGN.md`.                           | B; QM-COURSE/IDENTITY                              |
| `C6-CR2`         | grade scheme/totals/export                                                                                      | course grade/CSV reader; tests cover CAS/server totals/no-store/ETag/audited CSV.                                                                                                                                                                                                                                                            | C6-CR1; QM-COURSE                                  |
| `C6-AS1`         | `course/assignments/listing.rs`; `course/assignments/strict_request.rs`; direct `AssignmentEntryRequest` import | assignment editor/request/error/delivery DTOs; tests prove exact snake assignment request, retired-camel/unknown refusal, Student/Instructor projection, public Question IDs/revisions/capability, answer-free Student view, assignment-list `CoursePageQuery`, and no-store decoder/client. Owns assignment examples in `API_CONTRACTS.md`. | C6-CR1; QM-ASSIGNMENT/CONTENT/COURSE; WN1-SR2..SR4 |
| `C6-GO1`         | grading operations list/retry/recalculate                                                                       | operation DTOs; tests cover `group_by=student`, `page_size`, ETag/idempotency, generation-fenced automated retry, and recalculation. Owns automated-operation target examples in `API_CONTRACTS.md` and `automated_grading_operations_plan.md`.                                                                                              | B; WN1-MG; QM-GRADING-OPS                          |
| `C6-TA1/TG1/TP1` | authority/targets/retention, groups/members/purpose/warnings, teaching preview plane/modifiers                  | Safe authority/group/preview values; existing tests cover bounded display, PII concealment, roles/revision/ETag, memberships, pagination/no-store, and synthetic relation/disclosure/denial.                                                                                                                                                 | B; QM-TEACHING/IDENTITY/ASSIGNMENT                 |

## Student-role source closures

Each closure changes effective Serde, generated modules, direct consumers, and its named behavior
together. The existing `StudentAssignmentSummary` aggregate in `activity.rs` keeps its identity;
the course landing type becomes `StudentAssignmentLandingSummary` to avoid a type collision.

| Local closure                              | Exact retained target names and scope                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Permanent behavior gate                                                                                           |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| `WN1-SR1 disclosure and statistics`        | `LearnerDisclosureTiming`, `LearnerDisclosurePolicy`, `LearnerDisclosureDecision`, and `LearnerDisclosureInput` become `StudentDisclosureTiming`, `StudentDisclosurePolicy`, `StudentDisclosureDecision`, and `StudentDisclosureInput`; `learner_disclosure` becomes `student_disclosure`. `LearnerClassStatistics` and `completed_learner_cohort_size` become `StudentClassStatistics` and `completed_student_cohort_size`. Owns `run_policy.rs`, `statistics.rs`, `domain::disclosure_policy`, feedback Store inputs, the `student_class_statistics` Store method and Memory/PostgreSQL implementations, PostgreSQL `student_disclosure.rs` and `student_class_statistics.rs` modules, generated modules, and direct policy/statistics decoders. Effective Serde uses `snake_case`: policy fields, timing values, statistics states and fields, and reusable-curriculum `student_disclosure`.                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Disclosure timing, k-anonymity suppression, answer-free feedback, and generated-client typecheck.                 |
| `WN1-SR2 Student assignment projection`    | `LearnerScoreState`, `LearnerAssignmentProgress`, `LearnerAssignmentSummary`, `LearnerLateStatus`, `LearnerAssignmentDelivery`, and `LearnerAssignmentDetail` become `StudentScoreState`, `StudentAssignmentProgress`, `StudentAssignmentLandingSummary`, `StudentLateStatus`, `StudentAssignmentDelivery`, and `StudentAssignmentDetail`. `LearnerAssignmentSummarySnapshot` becomes `StudentAssignmentSummarySnapshot`; `LearnerNotActiveCourseStudent` becomes `StudentNotActiveCourse`. Effective Serde uses direct `snake_case` for every renamed projection field and discriminant owned by these six public model types. Owns those model definitions and generated modules, the private Store snapshot identity plus every direct Store/server run/course typed use of that snapshot, the domain entitlement variant and its PostgreSQL/test consumers, server course-assignment construction, direct TypeScript type consumers, and strict decoder wire-field updates. The existing `StudentAssignmentSummary` aggregate retains its identity and present wire contract until `QM-ACTIVITY`; the renamed snapshot carries it without transferring that aggregate's Serde ownership into SR2. SR3 retains run-path function/Store-capability vocabulary; SR4 retains decoder function, component, progress-helper, and filename renames. | Student list/detail/progress tests preserve score disclosure and answer-free Instructor Student view.             |
| `WN1-SR3 Student run and Store capability` | The exact target map is the normative `WN1-SR3 exact run and Store register` below: every named public and `_impl` method, module, projection, local, and function changes once, with no alias.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Run issuance, prefetch, submission replay/recovery, cross-Student denial, and external-tool handoff.              |
| `WN1-SR4 browser direct clients`           | The exact target map is the normative `WN1-SR4 exact browser register` below: all registered contracts, presentation names, components, converters, decoder exports, and client/runtime members change once.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Strict decoder and Student browser journey; browser projection remains answer-free.                               |
| `WN1-SR4A Student authority source`        | Close the non-serialized Rust authority vocabulary discovered by the post-SR4 whole-tree review: entitlement grants/facts/denials, entitlement materialization commands, the Student-visible assignment-list Store capability, Memory identity indexes, PostgreSQL read helpers, direct server callers, and their behavior tests. The exact target map is below.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Entitlement, enrollment, list visibility, materialization, and cross-Student denial behavior.                     |
| `WN1-SR5 PostgreSQL vocabulary`            | The exact target map is the normative `WN1-SR5 exact PostgreSQL register` below: each role-bearing schema name, broker policy, authority function, fence, and direct SQLx caller changes once.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Clean-volume migration, Store conformance, RLS authorization, and unchanged Student/Instructor/Sysadmin behavior. |
| `WN1-SR6 product documentation`            | Convert human-role uses to Student in `LIVE_DEMO_SPEC.md`, `STUDENT_GUIDE.md`, `INSTRUCTOR_GUIDE.md`, `COOKBOOK.md`, `FAQ.md`, `API_CONTRACTS.md`, `ENROLLMENT_DESIGN.md`, `CODE_ARCHITECTURE.md`, `FILE_STRUCTURE.md`, `FRONTEND_ARCHITECTURE.md`, `MASTERY_ASSIGNMENT_DESIGN.md`, `DATA_CLASSIFICATION.md`, `RETENTION_POLICY.md`, `CACHING_AND_PREFETCH.md`, `INSTRUCTOR_PAGE_VISUALS.md`, `STUDENT_PAGE_VISUALS.md`, `UI_DESIGN_REVIEW.md`, and `LOCAL_STACK_OPERATIONS.md`. Retain generic learning/teaching prose, `learning-*` system vocabulary, registered external terms, frozen history, and a clearly labeled `Current pre-WN1` evidence boundary where present-source spelling remains useful.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | One-time independent documentation/material-tree review; it adds no permanent documentation inventory test.       |

Manual-only `ManualGradingStore`, `ManualEvaluation*`, manual commands, manual status/outcome variants,
manual routes, manual receipts/tables/functions, `AnswerKey::FileUpload { rubric }`,
`incomplete_manual_grading`, `pending_manual_attempt_count`, `PendingManual`, and
`pending_manual` have no Student target; WN1-MG/C4-IA1 retire them.

### WN1-SR3 exact run and Store register

`WN1-SR3` owns the following exact current-name to target-name mappings. Public methods are in
`crates/learning-data-access/src/contracts/store.rs`; `_impl` capability methods are in
`contracts/store_capabilities.rs`; `in_memory/{activity,runs}.rs` and
`postgres/{activity,runs}.rs` implement each applicable target. `learner_submission_status` is the
capability-trait method on `LearnerSubmissionStatusStore`, not a `Store` forwarding method; its
trait, implementations, and direct callers still change atomically with this register.

| Current method                                                                        | Target method                                                                         |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `learner_assignment_run_items`, `learner_assignment_run_items_impl`                   | `student_assignment_run_items`, `student_assignment_run_items_impl`                   |
| `learner_get_prefetched_question`, `learner_get_prefetched_question_impl`             | `student_get_prefetched_question`, `student_get_prefetched_question_impl`             |
| `learner_pending_submission_for_run`, `learner_pending_submission_for_run_impl`       | `student_pending_submission_for_run`, `student_pending_submission_for_run_impl`       |
| `learner_list_question_attempts`, `learner_list_question_attempts_impl`               | `student_list_question_attempts`, `student_list_question_attempts_impl`               |
| `learner_get_enrollment`, `learner_get_enrollment_impl`                               | `student_get_enrollment`, `student_get_enrollment_impl`                               |
| `learner_get_enrollment_for_assignment`, `learner_get_enrollment_for_assignment_impl` | `student_get_enrollment_for_assignment`, `student_get_enrollment_for_assignment_impl` |
| `learner_get_run`, `learner_get_run_impl`                                             | `student_get_run`, `student_get_run_impl`                                             |
| `learner_list_runs`, `learner_list_runs_impl`                                         | `student_list_runs`, `student_list_runs_impl`                                         |
| `learner_get_question_attempt`, `learner_get_question_attempt_impl`                   | `student_get_question_attempt`, `student_get_question_attempt_impl`                   |
| `learner_get_summary`, `learner_get_summary_impl`                                     | `student_get_summary`, `student_get_summary_impl`                                     |
| `learner_submission_status`                                                           | `student_submission_status`                                                           |

| Current type, file, projection, or local/function                                                                                                                                                                                                                                                                                                                                  | Exact target                                                                                                                                                                                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LearnerAttemptDescriptorV1`, `LearnerRunScreenScopeV1`, `LearnerRunScreenRunV1`, `LearnerRunScreenV1`                                                                                                                                                                                                                                                                             | `StudentAttemptDescriptorV1`, `StudentRunScreenScopeV1`, `StudentRunScreenRunV1`, `StudentRunScreenV1`                                                                                                                                              |
| `LearnerWorkRoutingBinding`, `LearnerSubmissionStatusRead`, `LearnerSubmissionStatusStore`, `LearnerAttemptProjection`                                                                                                                                                                                                                                                             | `StudentWorkRoutingBinding`, `StudentSubmissionStatusRead`, `StudentSubmissionStatusStore`, `StudentAttemptProjection`                                                                                                                              |
| `in_memory/runs/learner_reads.rs`, `in_memory/runs/learner_submission_status.rs`, `in_memory/runs/learner_submission_status_tests.rs`, including every module declaration, test path, import, and qualified reference                                                                                                                                                              | `in_memory/runs/student_reads.rs`, `in_memory/runs/student_submission_status.rs`, `in_memory/runs/student_submission_status_tests.rs`, with their complete declaration/import graph                                                                 |
| `postgres/learner_work_preparation.rs`, `postgres/runs/learner_projections.rs`, `active_learner_run_for_read`, `postgres/runs/learner_transition.rs`, and `lock_prepared_predecessor_for_learner_run`, including every `mod`, `use`, and qualified reference in `postgres.rs`, `postgres/{entitlement,submission_preparation,student_run_preparation,runs}.rs`, and run submodules | `postgres/student_work_preparation.rs`, `postgres/runs/student_projections.rs`, `active_student_run_for_read`, `postgres/runs/student_transition.rs`, and `lock_prepared_predecessor_for_student_run`, with their complete declaration/import graph |
| `server/course/assignments/learner.rs`, its adjacent `course/tests/assignment_revision/learner.rs` behavior module, `RunSummaryEnrollmentAccess::Learner`, and the shared Rust `learner_assignment_progress` run helper                                                                                                                                                            | `server/course/assignments/student.rs`, `course/tests/assignment_revision/student.rs`, `RunSummaryEnrollmentAccess::Student`, and `student_assignment_progress`                                                                                     |
| `learner_work_binding` across `learning-data-access` external-tool contracts, Memory/PostgreSQL implementations and their conformance tests, plus `server/{composite_backend,imathas_backend,run/external_tool}` modules and IMathAS run tests; this includes every direct field/local/parameter carrying `LearnerWorkRoutingBinding`                                              | `student_work_binding` across that complete PLE-owned routing-binding graph                                                                                                                                                                         |
| `apply_learner_disclosure` in `server/run/submission.rs`, `learner_submission_status_projection` in `server/run/submission_status.rs`, `learner_scoring_status` in `server/run/support.rs`, and `redact_learner_run_score` in `server/run/queries.rs`, including their direct run callers; `GroupPurposeCapabilities::learner_visible` in `question_model/src/entitlement.rs`      | `apply_student_disclosure`, `student_submission_status_projection`, `student_scoring_status`, `redact_student_run_score`; `GroupPurposeCapabilities::student_visible`                                                                               |
| `ScoringInvalidationOriginKind::LearnerSupport`, `ScoringInvalidationOrigin::learner_support`                                                                                                                                                                                                                                                                                      | `ScoringInvalidationOriginKind::StudentSupport`, `ScoringInvalidationOrigin::student_support`                                                                                                                                                       |
| `learner_name` in the course/statistics projection, generated fixture, Memory/PostgreSQL decode aliases, and `tests/e2e/postgres_partition_pruning.sql` evidence query                                                                                                                                                                                                             | `student_name`; every fallback role label becomes `Student`                                                                                                                                                                                         |

### WN1-SR4 exact browser register

| Current browser contract, component, converter, or member                                                                                 | Exact target                                                                                                                                                             |
| ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `LearnerQuestionAttempt`, `LearnerSubmissionStatus` in `src/api/contracts.ts`                                                             | `StudentQuestionAttempt`, `StudentSubmissionStatus`                                                                                                                      |
| `LearnerWorkRouteScope`, `learnerWorkRoute`                                                                                               | `StudentWorkRouteScope`, `studentWorkRoute`                                                                                                                              |
| `LearnerAssignmentPresentationDelivery`, `LearnerAssignmentPresentationData`, `LearnerAssignmentPresentationProps`                        | `StudentAssignmentPresentationDelivery`, `StudentAssignmentPresentationData`, `StudentAssignmentPresentationProps`                                                       |
| `toLearnerAssignmentPresentationData`, `LearnerAssignmentPresentation`                                                                    | `toStudentAssignmentPresentationData`, `StudentAssignmentPresentation`                                                                                                   |
| `src/components/learner_assignment_presentation.tsx`, `src/components/learner_assignment_presentation.css`                                | `src/components/student_assignment_presentation.tsx`, `src/components/student_assignment_presentation.css`                                                               |
| `src/learner_progress.ts`, `learnerProgressSummary`, `learnerScoreValue`                                                                  | `src/student_progress.ts`, `studentProgressSummary`, `studentScoreValue`                                                                                                 |
| `src/features/attempt/learner_response.ts`, `projectLearnerResponse`, and the feedback-panel `learnerResponse` presentation prop          | `src/features/attempt/student_response.ts`, `projectStudentResponse`, and `studentResponse`                                                                              |
| `learnerAttemptPath`, `verifyLearnerSubmissionStatus` in `src/api/http_client/request.ts`; imported `learnerAttemptPath` in `response.ts` | `studentAttemptPath`, `verifyStudentSubmissionStatus`; imported `studentAttemptPath`                                                                                     |
| `/api/assignments/{assignment}/learner`, `get_learner_assignment`, `learner_assignment_detail_response`                                   | `/api/assignments/{assignment}/student`, `get_student_assignment`, `student_assignment_detail_response`, including router, route policy, browser client, and route tests |
| `isLearnerSubmissionPost`, `isLearnerStatusGet` in the recovery browser helper                                                            | `isStudentSubmissionPost`, `isStudentStatusGet`                                                                                                                          |

| Decoder file                      | Current decoder export or local                                                                  | Exact target                                                                                                                                                                                                              |
| --------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `assignment_teaching_delivery.ts` | `decodeLearnerAssignmentSummary`, `decodeLearnerAssignmentDetail`                                | `decodeStudentAssignmentLandingSummary`, `decodeStudentAssignmentDetail`; the landing decoder follows `StudentAssignmentLandingSummary` and stays distinct from the activity aggregate's `decodeStudentAssignmentSummary` |
| `run.ts`                          | `decodeLearnerQuestionAttempt`, `decodeLearnerAssignmentProgress`, `decodeLearnerAssignmentPage` | `decodeStudentQuestionAttempt`, `decodeStudentAssignmentProgress`, `decodeStudentAssignmentPage`                                                                                                                          |
| `submission_status.ts`            | `decodeLearnerSubmissionStatus`                                                                  | `decodeStudentSubmissionStatus`                                                                                                                                                                                           |

### WN1-SR4A exact Student authority source register

This closure owns PLE-internal Rust authority and identity vocabulary that is neither a serialized
question-model contract nor a PostgreSQL identifier. It completes before SR5 so the forward SQL
rename can bind to already canonical Store and domain owners.

| Current authority name                                                                                                      | Exact target                                                                  |
| --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `list_learner_entitled_assignments`, `list_learner_entitled_assignments_impl`                                               | `list_student_entitled_assignments`, `list_student_entitled_assignments_impl` |
| `MaterializeAssignmentEntitlementCommand::for_learner_action`, its `learner` field/accessor/parameters                      | `for_student_action`, with `student_user` for the role-bound `UserId`         |
| `EntitlementGrant::learner`, `EntitlementFacts::learner`                                                                    | `student_user`; the distinct `student` value remains the `StudentId`          |
| `EntitlementDenial::AudienceExcludesLearner`, `IndividualPatch::Learner`                                                    | `AudienceExcludesStudent`, `IndividualPatch::Student`                         |
| `learner_identity`, `learner_by_user`, `learner_by_student` in Memory roster state                                          | `student_identity`, `student_by_user`, `student_user_by_student`              |
| `learner_enrollment_for_read`, `learner_enrollment_for_assignment_read`                                                     | `student_enrollment_for_read`, `student_enrollment_for_assignment_read`       |
| Role-bound Rust locals such as `learner_self` and `learner_inputs` in direct authority/feedback/grade calculation consumers | `student_self`, `student_inputs`                                              |

Question-model wire names, grading-operation grouping, catalog-statistics contracts, and current
PostgreSQL columns/functions retain their separately registered QM/C6/SR5 owners. Focused behavior
tests are permanent; the identifier inventory is one-time evidence.

### WN1-SR5 PostgreSQL closure

`2026081881` owns schema vocabulary. `2026081882` owns roles, policies, functions, fences,
grants, ownership, and direct SQLx callers. The closure uses globally unique account, Student,
Course, assignment, run, attempt, workspace, and catalog identities. It provides no backfill or
compatibility alias.

The resulting schema names Student identity, Student-work broker predicates, Student-owned
fingerprint receipts, Student support, and Student-record fences directly. Every policy and
function derives authority from its exact protected Course, Student, membership, actor, or durable
capability relation. Memory helpers, direct Rust callers, tests, catalog assertions, and database
decision/error text use the same canonical Student vocabulary. Constraint lookup uses relation
identity plus constrained columns, so generated PostgreSQL abbreviations never become durable
contract names.

### WN1-SR6 live product-document register

`WN1-SR6` changes a human role/person or Student-owned path, account, identity, record,
submission, or work from `learner` to `Student` in each current product document below. It retains
generic learning and teaching prose, `learning-*` system terms, registered external names, frozen
historical material, and explicitly labeled `Current pre-WN1` source evidence. This register is a
one-time documentation/material-tree review and does not create a permanent text-inventory test.

| Current product document            | Exact role conversion                           | Retained boundary                                          |
| ----------------------------------- | ----------------------------------------------- | ---------------------------------------------------------- |
| `docs/LIVE_DEMO_SPEC.md`            | Person/role and role-owned flow -> Student      | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/STUDENT_GUIDE.md`             | Person/role and Student work -> Student         | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/INSTRUCTOR_GUIDE.md`          | Instructor-facing Student references -> Student | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/COOKBOOK.md`                  | Instructor action on a person -> Student        | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/FAQ.md`                       | Person/role and Student path -> Student         | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/API_CONTRACTS.md`             | PLE role-bound contract prose -> Student        | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/ENROLLMENT_DESIGN.md`         | Membership person and record -> Student         | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/CODE_ARCHITECTURE.md`         | PLE role-bound component/projection -> Student  | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/FILE_STRUCTURE.md`            | Current PLE role-bound path -> Student          | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/FRONTEND_ARCHITECTURE.md`     | Browser role-bound projection -> Student        | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/MASTERY_ASSIGNMENT_DESIGN.md` | Assignment person/work -> Student               | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/DATA_CLASSIFICATION.md`       | Student identity and record -> Student          | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/RETENTION_POLICY.md`          | Student record and authority -> Student         | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/CACHING_AND_PREFETCH.md`      | Student cache/projection -> Student             | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/INSTRUCTOR_PAGE_VISUALS.md`   | Page labels and role-bound evidence -> Student  | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/STUDENT_PAGE_VISUALS.md`      | Page labels and Student flow -> Student         | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/UI_DESIGN_REVIEW.md`          | Role-bound UI/person reference -> Student       | Generic learning, historical, and Current pre-WN1 evidence |
| `docs/LOCAL_STACK_OPERATIONS.md`    | Live-stack person/role operation -> Student     | Generic learning, historical, and Current pre-WN1 evidence |

## Shared model, adapter, and external seams

`QM-IDENTITY`, `QM-CAPABILITY`, `QM-LIFECYCLE`, `QM-CATALOG`, `QM-COURSE`, `QM-CONTENT`,
`QM-ACTIVITY`, `QM-STATS`, `QM-PRESENTATION`, `QM-ASSIGNMENT`, `QM-CURRICULUM`, `QM-TEACHING`,
and `QM-GRADING-OPS` migrate their complete source/type graph after direct route projections land.
`QM-ACTIVITY` owns Student assignment/status vocabulary; `QM-PRESENTATION` owns Student
disclosure; `QM-STATS` owns aggregate disclosure; `QM-GRADING-OPS` owns automated
status/reason/state/action. Their permanent gates are focused effective-Serde and semantic behavior
tests, not inventories. `WA1-WASM`, `WA2-NATIVE`, `WA3-QTI`, `WA4-H5P`, and
`WA5-PROVIDER-CACHE` own their PLE parse/stringify wrappers and retain raw wasm-bindgen, native
bytes, QTI/H5P, and provider/PGML/HTML owner spellings.

## Durable closure

Accepted migrations `2026080801` through `2026081878` and checksums are frozen historical evidence.
All WN1 migrations use clean-volume installation, zero row backfill, and canonical producer/seed
rebuild. Current mutable rows are disposable. Immutable records retain their named version, digest,
and reader; a changed immutable representation receives a forward version.

| Allocation | Exact filename and owner                                                                                                  | Atomic outcome and permanent behavior gate                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ---------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1879       | `2026081879_course_authority_broker_ownership.sql` -- PostgreSQL authority owner                                          | Creates `ple_course_authority_broker` with explicit `NOLOGIN`, `NOINHERIT`, `NOBYPASSRLS` ownership, narrow RLS reads/policies, explicit ACLs, and `FORCE ROW LEVEL SECURITY` for course appearance/roster predicate ownership. Existing authority suites preserve behavior.                                                                                                                                                                                                                                                                                                                                                                                           |
| 1880       | `2026081880_authority_function_argument_rebinding.sql` -- PostgreSQL authority plus learning-data-access PostgreSQL owner | Rebinds `ple_retention_authorize`, `ple_course_appearance_actor`, `ple_course_appearance_authorize`, and `ple_course_roster_actor` to exact `p_sysadmin_only`/`p_instructor_only` signatures. Recreates retention action dependents, effective appearance authorization, and `ple_inspect_student_work_v1`; preserves `ple_retention_broker`, `ple_course_authority_broker`, inspection broker owners, `PUBLIC` revoke, narrow `ple_app`/inspection grants, `SECURITY DEFINER`, search path, typed signatures, and current authorization behavior. Existing role-matrix suites are permanent; `pg_proc`, dependency, owner, ACL, and clean-volume checks are one-time. |
| 1881       | `2026081881_student_role_schema_vocabulary.sql` -- Student-role persistence owner                                         | Renames effective role-bearing relations, columns, constraints, indexes, catalog fingerprint names, and `learner_support` to the exact SR5 `student_*` targets; generic `learning_*` remains stable. Store/RLS behavior is permanent evidence.                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 1882       | `2026081882_student_work_broker_vocabulary.sql` -- Student-work authority owner                                           | Renames `ple_learner_work_broker`, trusted functions, policies, fence, and callers to SR5 `student_*` targets; recreates security-definer bindings with explicit owner/ACL/forced-RLS policy. Student-work authority behavior is permanent evidence.                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 1883       | `2026081883_automated_scoring_only.sql` -- WN1-MG persistence owner                                                       | Retires manual-grade relations, constraints, policies, grants, and status checks; installs automated-only checks for `automated_pending`, `automated_exception`, `graded`, and `exempt`; recreates exact dependents. Deterministic score/exception/retry behavior is permanent evidence.                                                                                                                                                                                                                                                                                                                                                                               |
| 1884       | `2026081884_student_work_payload_contracts.sql` -- run/submission persistence owner                                       | Changes current mutable Student-work JSONB readers/writers/checks/indexes/functions for run, attempt, submission, idempotency, evaluation, feedback, and summary projections to direct snake Serde. Existing V1 immutable evidence readers remain. Reader/writer and recovery behavior is permanent evidence.                                                                                                                                                                                                                                                                                                                                                          |
| 1885       | `2026081885_canonical_receipt_payload_v2.sql` -- canonical-evidence/grading-receipt owner                                 | Adds V2 writer/reader/checksum/validation only when immutable receipt representation changes; preserves V1 bytes and readers. Canonical digest/replay behavior is permanent evidence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 1886       | `2026081886_catalog_workspace_payload_contracts.sql` -- catalog/authoring persistence owner                               | Changes catalog/publication/workspace/flat-asset checks/readers/writers; immutable source/provenance receives a named forward version when needed. QTI/H5P/native/provider seams retain owner spelling.                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 1887       | `2026081887_curriculum_payload_contracts.sql` -- curriculum-adoption persistence owner                                    | Changes curriculum adoption request/inspection/reconciliation constraints/readers/writers; preserves digest-bound immutable request/semantic identity.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 1888       | `2026081888_operational_payload_contracts.sql` -- jobs/retention/asset-delivery/export persistence owner                  | Changes worker, retention, delivery, account/roster projection, provider-cache, and export current operational records; retained object identity uses a named forward version.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

Migrations 1886-1888 retain existing deterministic parser/import/replay/Store behavior as permanent
coverage. Clean-volume rebuild, migration catalog inspection, regeneration, and digest inspection are
one-time evidence. A child uses its allocated migration only when its stored contract changes.

## Final WN1-F acceptance

`WN1-FD` is the bounded documentation/files owner. Before final review, it registers frozen
historical filenames, including historical release material, and allocates every current active
filename identified by the scoped naming-document audit to canonical naming. Its exact active
scope is `docs/how-to-reduce-impact-of-bot-traffic.md`, `docs/QTI-JSON_OBJECT_FORMAT.md`,
`docs/active_plans/Rust_SQLx_and_PostgreSQL_implementation.md`, `customer-spec.md`,
`m0-results.md`, `m0-review.md`, `peptidyle-security-audit.md`, `peptidyle-walkthrough-plan.md`,
and dated active reports under `docs/active_plans/reports/`. The implementation owner follows the
repository move policy when applying the recorded disposition: canonical rename with in-tree links
updated, archive/history move, or explicit frozen-filename registration. Material-tree link
validation is one-time evidence; this child adds no broad inventory test.

WN1-F requires `WN1-OPS1` through `WN1-OPS10`, `WN1-GO1`, `WN1-SR6`, `WN1-FD`, every applicable
C/QM/WA/D closure, independent review against `docs/NAMING_CONVENTIONS.md`, material-tree
validation, and:

```bash
source source_me.sh && ./all_test.sh
```

Any required unrun, skipped, or failed gate keeps WN1 and the G2 handback acceptance-open.
