# Terminology boundary review

## Scope

This review classifies a focused set of recurring words by the boundary where
each one is meaningful. It uses
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) as its authority and records
migration work in
[VOCABULARY_REPLACEMENTS.md](../../VOCABULARY_REPLACEMENTS.md).

The target is to replace PLE-owned meaning with exact canonical terms. Raw word
counts remain useful for finding entries, while sentence and identifier
context decides whether a match needs correction.

Every reviewed meaning receives one explicit disposition: document the useful
term and its narrow boundary, or add a context-specific replacement row. A word
may therefore stay in one precise context while another use migrates.

## Findings

| Word             | Decision                               | Boundary rule                                                                                                                                                                                              |
| ---------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Consumer         | Narrow use                             | Keep for a dependency relationship. Name a component Disposable Stack Adapter or by its exact operation.                                                                                                   |
| Adoption         | Replace as a PLE operation umbrella    | Name Fork Blueprint Course, Create Course from Blueprint, Copy Assignment from Blueprint, Apply Blueprint Update, Copy Course for New Term, or Shift Course Dates.                                         |
| Content Block    | Qualify and document                   | Use Question Content Block for the shared Text, Math, Image, Code, or Table presentation primitive. Let Question Prompt, Choice, Matching Prompt, Matching Choice, or Feedback containment supply meaning. |
| Fixture Corpus   | Replace for PLE-authored data          | Use Stored Question Fixture Set, Pilot Question Set, or an exact purpose-specific set name.                                                                                                                |
| Activity         | Narrow use                             | Use Student activity as collective prose or a derived view. Persist exact Attempts, Submissions, Events, and Receipts.                                                                                     |
| Transport        | Keep at its technical boundary         | Use for exchange mechanics such as HTTP, cookies, scored embeds, and network failures. Name the product operation separately.                                                                              |
| Broker           | Replace as a PLE component umbrella    | Name the authorization operation, Question Backend, or exact Job claim and lease operation.                                                                                                                |
| Factory          | Narrow construction pattern            | Use for a component that chooses among multiple construction strategies. Name injected callables with acquire or create actions, and use direct constructors for one configured result.                    |
| Chapter          | Narrow use                             | Keep literal source-book metadata. Use Pilot Assignment and subject/topic metadata for the current pilot, or Module for a real course section.                                                             |
| Type variant     | Keep at its language boundary          | Use for an alternative in a Rust, TypeScript, or generated closed type. Question Variation remains the generated Question concept.                                                                         |
| Runtime          | Narrow use                             | Keep for an actual execution environment or lifecycle. Rename the browser API client container to Application API.                                                                                         |
| Curriculum       | Replace in PLE-owned contracts         | Use Blueprint Course, Blueprint Revision, Blueprint Assignment, or the exact live Course Instance operation.                                                                                               |
| Decoder          | Keep at its technical boundary         | Use for validation and conversion from an untrusted representation to an accepted typed value.                                                                                                             |
| Curation         | Keep as a workflow                     | Question Curation may name the Instructor workflow or surface. Split stored and exported contracts into Question Folder, Saved Question Search, Star, Watch, and Change Proposal boundaries.               |
| Private Question | Replace when it means editable content | Use Draft Question and Draft Question Revision. Qualify exact source, grading, answer, or storage material as private only when access is the point.                                                       |
| Payload          | Keep at its technical boundary         | Use for one bounded unit of data crossing a defined boundary. Name durable PLE records by their exact record type.                                                                                         |
| HTTP             | Keep at its protocol boundary          | Use for routes, methods, headers, status codes, cookies, and protocol behavior.                                                                                                                            |

## Existing coverage

The replacement checklist already covers Question Content Block,
`ActivityTimestamp`, Question Backend, protected database operations,
Blueprint Course vocabulary, Stored Question Fixture Set, and exact Question
Variation. This review adds the missing context rows for Disposable Stack Adapter,
Application API Runtime, generic Activity, generic Broker, PLE-owned Chapter,
Question Curation, Private Question, the Adoption umbrella, and Factory used
for direct construction or injected operations.

## Active directory vocabulary

The current nonempty `src/` and `crates/` tree contains three kinds of names.
The terminology contract governs only the PLE-owned meanings.

Canonical PLE paths include `question_json`, `presentation`,
`effective_assignment_policy`, `course_appearance`, `question_curation`,
`question_picker`, `assignment_access`, `assignment_workspace`, and
`teaching_operations`. Their canonical meanings are Question Format, Question
Presentation, Effective Assignment Policy, Course Appearance, Question
Curation, Question Picker, Assignment Access, Assignment Workspace, and
Teaching Operations.

Architecture paths include `src`, `crates`, `lib`, `tests`, `fixtures`,
`support`, `examples`, `api`, `decoders`, `http_client`, `auth`, `components`,
`domain`, `contracts`, `learning-data-access`, `postgres`, `objects`,
`project-tools`, `tsgen`, `server`, `export`, `pdf`, `wasm`, and `styles`.
Registered integration paths include `h5p`, `imathas`, `ple`, `qti`,
`blackboard`, `canvas`, and `webwork`. These names belong to architecture or
their external specifications and need no PLE glossary entry.

`acceptance-runtime` names an actual execution environment for connected
acceptance work. `sd1_staged_database` is a plan-namespaced implementation area
inside that runtime and follows the active SD1 package record. Neither name is
a product object, interface surface, or stored record.

The active paths below carry PLE meaning that the replacement checklist now
maps to a canonical owner.

| Task                      | Implementation owner                                                                                                   | Required correction                                                                                                                                                                              | Success condition                                                                                                                                                                                                | Validation                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TERM-PATH-1 (open)        | `crates/adapters/imathas/imathas_question_backend.rs`                                                                  | iMathAS Question Backend is the canonical iMathAS integration boundary.                                                                                                                          | Every PLE-owned iMathAS adapter path and symbol names iMathAS or an exact Session, Launch, Result Exchange, Result, render, or grade operation. Message broker remains only infrastructure that routes messages. | Open pending the RQB2 direct cutover.                                                                                                                                                         |
| TERM-PATH-2 (complete)    | `src/components/question_response_controls/`                                                                           | Question Response Control is the canonical component boundary.                                                                                                                                   | The component boundary names Question Response Control, while `StudentResponse` remains the learner data and `QuestionResponseFormat` remains its accepted shape.                                                | 2026-08-31: focused response-control, Student presentation, and Student Response projection tests, TypeScript compilation, source-path inspection, and `git diff --check` pass.               |
| TERM-PATH-3 (complete)    | `src/features/question_attempt/`                                                                                       | Question Attempt state is the canonical browser feature boundary.                                                                                                                                | The feature owns one Issued Question's interaction and submission lifecycle; Assignment Attempt remains its containing pass and completion owner.                                                                | 2026-08-31: focused Question Attempt behavior tests, TypeScript compilation, source-path inspection, and `git diff --check` pass.                                                             |
| TERM-PATH-4 (complete)    | `crates/domain/src/student_feedback_release.rs` and its consumers                                                      | Student Feedback Release and `student_feedback_release` are the canonical boundary.                                                                                                              | Public types, source paths, generated contracts, and browser copy state when score, correctness, feedback, solutions, and class statistics become visible.                                                       | 2026-08-31: focused domain and browser policy suites, TypeScript compilation, source search, Markdown links, and `git diff --check` pass.                                                     |
| TERM-PATH-5 (complete)    | `crates/question_model/src/blueprint_course/`, `src/features/blueprint_course/`, and their Store/API consumers         | Rename the aggregate and module paths to Blueprint Course and `blueprint_course`.                                                                                                                | Every reusable course aggregate is a Blueprint Course; every immutable state is a Blueprint Revision; stores and interfaces name the exact record they own.                                                      | 2026-08-31: Question Model, focused Blueprint Course browser tests, TypeScript compilation, Markdown links, source-path inspection, and `git diff --check` pass.                              |
| TERM-PATH-6               | `crates/question_model/src/blueprint_operations/`, `src/features/blueprint_operations/`, and their Store/API consumers | Split the transport boundary into Fork Blueprint Course, Create Course from Blueprint, Copy Assignment from Blueprint, Apply Blueprint Update, Copy Course for New Term, and Shift Course Dates. | Each operation owns its command, readiness result, retry token, manifest when needed, receipt, Store method, route, decoder, interface state, and tests. Shared code sits behind those exact contracts.          | Run the focused Question Model, Store, server, and browser operation suites; run `npx tsc --noEmit`; inspect `blueprint_operations` and unqualified adoption matches; run `git diff --check`. |
| TERM-FACTORY-1 (complete) | `local_stack_control/acceptance_profile_owner.py` and its three callers                                                | Rename the two injected callables to `acquire_browser_suite_lease` and `create_command_runner`.                                                                                                  | Every Local Stack callable states its actual action; Factory remains reserved for multi-strategy construction.                                                                                                   | 2026-08-31: focused Local Stack owner, lease, reset, developer, and CLI tests pass. Current source, schema, generated contracts, and tests contain no Factory occurrence.                     |
| TERM-FACTORY-2 (complete) | `crates/adapters/imathas/src/test_support.rs` and its adapter/server callers                                           | Replace the one-mode Factory wrappers with direct recorded-provider and recorded-transport construction.                                                                                         | `RecordedImathasProvider` constructs directly from its mode; exact functions construct the contracted transport, provider, or provider-plus-transport pair; test-support callers use those operations.           | 2026-08-31: iMathAS adapter test-support suite and Clippy pass. Current source, schema, generated contracts, and tests contain no Factory occurrence.                                         |
| TERM-FACTORY-3 (complete) | `crates/adapters/ple/src/question_json/imported.rs`                                                                    | Trusted PLE Question JSON import construction is the canonical direct boundary.                                                                                                                  | Documentation names `ImportedPleQuestionJson::from_imported` and `ImportedPleQuestionJsonError` directly and describes direct construction as the complete boundary.                                             | 2026-09-01: PLE Question Backend tests and Clippy, source inspection, and `git diff --check` pass.                                                                                            |

## Assessment naming question

Assignments, Quizzes, and Exams share delivery mechanics but communicate
different teaching intent. The open design is recorded in
[assessment_type_terminology.md](../decisions/assessment_type_terminology.md).
The terminology contract remains unchanged until that decision is accepted.
