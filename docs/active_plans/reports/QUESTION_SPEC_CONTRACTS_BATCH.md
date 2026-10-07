# Current Rust-to-TypeScript Contract Batch: M05, M06, M09, M19, M27

Date: 2026-10-06

## Generation

- Command: `source source_me.sh && cargo tsgen`
- Result: success; 415 declarations written.
- A pre-run process query found no matching `tsgen` or repository check process; this task ran one generation pass.
- Generator-owned source roots: `crates/question_model/src/` and `crates/browser-api-contract/src/`.
- Output root: `generated/api/`, as in `question_spec_m04_m17_contracts.md`.
- The output directory had 415 TypeScript files before generation and 416 after. The existing dedicated `QuestionIdSyntaxContract.ts` remained unchanged. Comparing file hashes found 5 added files, 4 removed files, and 16 changed files.

Added: `BlueprintThemeUpdate.ts`, `CurrentQuestionPoolMetadata.ts`, `QuestionPoolMetadataReplacement.ts`, `SaveQuestionPoolMetadataRequest.ts`, `SavedQuestionPoolMetadata.ts`.

Removed: `BloomClassificationCorrectionRequest.ts`, `BloomClassificationEditNumber.ts`, `QuestionBloomCorrectionReceipt.ts`, `QuestionPoolBloomCorrectionReceipt.ts`.

Changed: `BloomClassificationView.ts`, `BlueprintChangeProposalSideView.ts`, `BlueprintCourseSummaryView.ts`, `BlueprintCourseView.ts`, `BlueprintHistoryEntryView.ts`, `BlueprintMetadataState.ts`, `BlueprintPoolInputChoice.ts`, `CanonicalBlueprintMetadata.ts`, `CreateBlueprintCourseInput.ts`, `PublishedQuestionSharedMetadata.ts`, `QuestionMetadataReplacement.ts`, `QuestionPoolMemberView.ts`, `QuestionPoolMetadata.ts`, `QuestionPoolView.ts`, `QuestionSearchFilter.ts`, `TextResponseMatchRule.ts`.

## Contracts consumed in this batch

- **M05:** ordinary Question and Pool metadata carry nullable `bloomCognitiveProcess` and `bloomKnowledgeDimension`. The dedicated Bloom correction request, edit number, and receipts are absent from the generated output.
- **M06:** `PublishedQuestionSharedMetadata` and `QuestionMetadataReplacement` carry `questionType: QuestionType`.
- **M09:** the generated output contains no `memberPosition` or `member_position`; `QuestionPoolMemberView` identifies a member with `publishedQuestionRevisionTuple`.
- **M19:** `TextResponseMatchRule` includes the `"regex"` case.
- **M27:** theme is present in `BlueprintCourseView`, `BlueprintCourseSummaryView`, `CreateBlueprintCourseInput`, `BlueprintMetadataState`, `BlueprintHistoryEntryView`, `CanonicalBlueprintMetadata`, and `BlueprintChangeProposalSideView`. `BlueprintThemeUpdate` is generated.

This is a generation from the shared repository state at the time of the command. M14 Watch and M24 search contracts reflect their current source definitions where present in these roots; they are not claimed to be source-frozen. A later source change to either contract needs a justified follow-up generation. The M09 and M27 model checkpoints were treated as ready, and the M06 duplicate import was already resolved.

## Compilation and TypeScript checks

- `source source_me.sh && cargo check -p question_model -p browser-api-contract`: passed. No Rust source compilation failure was found in the two contract crates.
- `npx tsc --noEmit -p tsconfig.json`: failed with 43 diagnostics.
- `npx tsc --noEmit -p tsconfig.lint.json`: failed with 46 diagnostics: the same 43 source diagnostics plus 3 test-fixture diagnostics.

The adjacent browser/API tree already contains in-flight edits. These diagnostics are recorded and routed below; no browser or Rust source was changed in this task.

### Owner routes for the current diagnostics

- **M09 tuple shape:** route the stale `memberPosition` reads/writes to the owners of `src/api/decoders/assessment_pool_fork.ts`, `src/api/decoders/question_pool_detail.ts`, and `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx`.
- **M05 nullable metadata and retired correction API:** route Pool summary/detail decoders, metadata decoder imports, Bloom decoder cleanup, nullable Bloom handling, and fixtures to the owners of `src/api/decoders/question_pool_detail.ts`, `src/api/decoders/question_pool_metadata.ts`, `src/api/decoders/question_pool_summary.ts`, `src/api/decoders/bloom_classification.ts`, `src/pages/assessment_workspace/assessment_workspace_questions_model.ts`, `tests/playwright/e2e/ui_backbone_parity.spec.ts`, and `tests/support/ribbon_shell_harness.tsx`.
- **M27 create input:** route the missing `theme` value to the owner of `src/features/blueprint_course/blueprint_course_model.ts`.
- **Current Pool-view decoder drift:** route the missing `canEditMetadata` projection in `src/api/decoders/question_pool_detail.ts` to that Pool detail API owner.
- **Other current browser diagnostics:** route the unsupported `toSorted` calls and their type cascades to the owners of `src/api/http_client/question_pool_creation.ts`, `src/components/question_pool_create_dialog.tsx`, `src/features/blueprint_course/blueprint_pool_members_editor.tsx`, and `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx`. Route the `NoticeKind` mismatch to `src/features/blueprint_course/blueprint_course_detail_workspace.tsx`; route the unused import to `src/pages/question_pool_detail.tsx`.

Exact compiler output follows. The lint-project output is included in full so the three additional fixture diagnostics remain explicit.

### `tsconfig.json` output (43 diagnostics)

```text
src/api/decoders/assessment_pool_fork.ts(64,16): error TS2339: Property 'memberPosition' does not exist on type 'QuestionPoolMemberView'.
src/api/decoders/bloom_classification.ts(6,10): error TS6133: 'DecodeError' is declared but its value is never read.
src/api/decoders/question_pool_detail.ts(61,5): error TS2353: Object literal may only specify known properties, and 'memberPosition' does not exist in type 'QuestionPoolMemberView'.
src/api/decoders/question_pool_detail.ts(99,16): error TS2339: Property 'memberPosition' does not exist on type 'QuestionPoolMemberView'.
src/api/decoders/question_pool_detail.ts(106,3): error TS2741: Property 'canEditMetadata' is missing in type '{ metadata: QuestionPoolMetadata; questionPoolId: string; ownerAccountId: string; questionType: QuestionType; backend: QuestionBackend; ... 4 more ...; evidence: QuestionStatistics; }' but required in type 'QuestionPoolView'.
src/api/decoders/question_pool_metadata.ts(5,1): error TS6133: 'QuestionPoolId' is declared but its value is never read.
src/api/decoders/question_pool_summary.ts(95,3): error TS2739: Type '{ title: string; description: string; disciplineUuid: string; disciplineName: string; disciplineIsRetired: boolean; subjectUuid: string; topicUuid: string | null; subtopicUuid: string | null; tags: string[]; }' is missing the following properties from type 'QuestionPoolMetadata': bloomCognitiveProcess, bloomKnowledgeDimension
src/api/http_client/question_pool_creation.ts(54,18): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/api/http_client/question_pool_creation.ts(54,28): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/api/http_client/question_pool_creation.ts(54,34): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/components/question_pool_create_dialog.tsx(270,21): error TS2550: Property 'toSorted' does not exist on type '{ title: string; publishedQuestionId: string; revisionNumber: number; revisionIsFixed: boolean; }[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/components/question_pool_create_dialog.tsx(270,31): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/components/question_pool_create_dialog.tsx(270,37): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/features/blueprint_course/blueprint_course_detail_workspace.tsx(777,25): error TS2769: No overload matches this call.
  The last overload gave the following error.
    Type '"error"' is not assignable to type 'NoticeKind'.
src/features/blueprint_course/blueprint_course_model.ts(132,3): error TS2741: Property 'theme' is missing in type '{ classification: CourseClassification; short_name: string; long_name: string; modules: { label: string; assessments: BlueprintAssessmentContentInput[]; }[]; }' but required in type 'CreateBlueprintCourseInput'.
src/features/blueprint_course/blueprint_pool_members_editor.tsx(123,27): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/features/blueprint_course/blueprint_pool_members_editor.tsx(123,37): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/features/blueprint_course/blueprint_pool_members_editor.tsx(123,43): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(97,52): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(97,62): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(97,68): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(106,20): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(106,30): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(106,36): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(144,26): error TS2550: Property 'toSorted' does not exist on type 'QuestionPoolMemberView[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(144,36): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(144,42): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(149,23): error TS7006: Parameter 'member' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(149,31): error TS7006: Parameter 'index' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(151,48): error TS18046: 'record' is of type 'unknown'.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(164,36): error TS2550: Property 'toSorted' does not exist on type 'QuestionPoolMemberView[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(164,46): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(164,52): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(169,44): error TS7006: Parameter '_member' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(169,53): error TS7006: Parameter 'index' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(169,73): error TS18046: 'record' is of type 'unknown'.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(170,33): error TS7006: Parameter 'member' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(176,37): error TS18046: 'record' is of type 'unknown'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(192,31): error TS2345: Argument of type 'BloomCognitiveProcess | null' is not assignable to parameter of type 'NonNullable<BloomCognitiveProcess | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomCognitiveProcess | null>'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(193,31): error TS2345: Argument of type 'BloomCognitiveProcess | null' is not assignable to parameter of type 'NonNullable<BloomCognitiveProcess | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomCognitiveProcess | null>'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(196,33): error TS2345: Argument of type 'BloomKnowledgeDimension | null' is not assignable to parameter of type 'NonNullable<BloomKnowledgeDimension | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomKnowledgeDimension | null>'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(197,33): error TS2345: Argument of type 'BloomKnowledgeDimension | null' is not assignable to parameter of type 'NonNullable<BloomKnowledgeDimension | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomKnowledgeDimension | null>'.
src/pages/question_pool_detail.tsx(4,46): error TS6133: 'on' is declared but its value is never read.
```

### `tsconfig.lint.json` output (46 diagnostics)

```text
src/api/decoders/assessment_pool_fork.ts(64,16): error TS2339: Property 'memberPosition' does not exist on type 'QuestionPoolMemberView'.
src/api/decoders/bloom_classification.ts(6,10): error TS6133: 'DecodeError' is declared but its value is never read.
src/api/decoders/question_pool_detail.ts(61,5): error TS2353: Object literal may only specify known properties, and 'memberPosition' does not exist in type 'QuestionPoolMemberView'.
src/api/decoders/question_pool_detail.ts(99,16): error TS2339: Property 'memberPosition' does not exist on type 'QuestionPoolMemberView'.
src/api/decoders/question_pool_detail.ts(106,3): error TS2741: Property 'canEditMetadata' is missing in type '{ metadata: QuestionPoolMetadata; questionPoolId: string; ownerAccountId: string; questionType: QuestionType; backend: QuestionBackend; ... 4 more ...; evidence: QuestionStatistics; }' but required in type 'QuestionPoolView'.
src/api/decoders/question_pool_metadata.ts(5,1): error TS6133: 'QuestionPoolId' is declared but its value is never read.
src/api/decoders/question_pool_summary.ts(95,3): error TS2739: Type '{ title: string; description: string; disciplineUuid: string; disciplineName: string; disciplineIsRetired: boolean; subjectUuid: string; topicUuid: string | null; subtopicUuid: string | null; tags: string[]; }' is missing the following properties from type 'QuestionPoolMetadata': bloomCognitiveProcess, bloomKnowledgeDimension
src/api/http_client/question_pool_creation.ts(54,18): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/api/http_client/question_pool_creation.ts(54,28): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/api/http_client/question_pool_creation.ts(54,34): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/components/question_pool_create_dialog.tsx(270,21): error TS2550: Property 'toSorted' does not exist on type '{ title: string; publishedQuestionId: string; revisionNumber: number; revisionIsFixed: boolean; }[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/components/question_pool_create_dialog.tsx(270,31): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/components/question_pool_create_dialog.tsx(270,37): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/features/blueprint_course/blueprint_course_detail_workspace.tsx(777,25): error TS2769: No overload matches this call.
  The last overload gave the following error.
    Type '"error"' is not assignable to type 'NoticeKind'.
src/features/blueprint_course/blueprint_course_model.ts(132,3): error TS2741: Property 'theme' is missing in type '{ classification: CourseClassification; short_name: string; long_name: string; modules: { label: string; assessments: BlueprintAssessmentContentInput[]; }[]; }' but required in type 'CreateBlueprintCourseInput'.
src/features/blueprint_course/blueprint_pool_members_editor.tsx(123,27): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/features/blueprint_course/blueprint_pool_members_editor.tsx(123,37): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/features/blueprint_course/blueprint_pool_members_editor.tsx(123,43): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(97,52): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(97,62): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(97,68): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(106,20): error TS2550: Property 'toSorted' does not exist on type 'PublishedQuestionRevisionTuple[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(106,30): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(106,36): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(144,26): error TS2550: Property 'toSorted' does not exist on type 'QuestionPoolMemberView[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(144,36): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(144,42): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(149,23): error TS7006: Parameter 'member' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(149,31): error TS7006: Parameter 'index' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(151,48): error TS18046: 'record' is of type 'unknown'.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(164,36): error TS2550: Property 'toSorted' does not exist on type 'QuestionPoolMemberView[]'. Do you need to change your target library? Try changing the 'lib' compiler option to 'es2023' or later.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(164,46): error TS7006: Parameter 'left' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(164,52): error TS7006: Parameter 'right' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(169,44): error TS7006: Parameter '_member' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(169,53): error TS7006: Parameter 'index' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(169,73): error TS18046: 'record' is of type 'unknown'.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(170,33): error TS7006: Parameter 'member' implicitly has an 'any' type.
src/pages/assessment_workspace/assessment_pool_entry_editor.tsx(176,37): error TS18046: 'record' is of type 'unknown'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(192,31): error TS2345: Argument of type 'BloomCognitiveProcess | null' is not assignable to parameter of type 'NonNullable<BloomCognitiveProcess | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomCognitiveProcess | null>'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(193,31): error TS2345: Argument of type 'BloomCognitiveProcess | null' is not assignable to parameter of type 'NonNullable<BloomCognitiveProcess | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomCognitiveProcess | null>'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(196,33): error TS2345: Argument of type 'BloomKnowledgeDimension | null' is not assignable to parameter of type 'NonNullable<BloomKnowledgeDimension | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomKnowledgeDimension | null>'.
src/pages/assessment_workspace/assessment_workspace_questions_model.ts(197,33): error TS2345: Argument of type 'BloomKnowledgeDimension | null' is not assignable to parameter of type 'NonNullable<BloomKnowledgeDimension | null>'.
  Type 'null' is not assignable to type 'NonNullable<BloomKnowledgeDimension | null>'.
src/pages/question_pool_detail.tsx(4,46): error TS6133: 'on' is declared but its value is never read.
tests/playwright/e2e/ui_backbone_parity.spec.ts(74,11): error TS2739: Type '{ title: string; description: string; disciplineUuid: string; disciplineName: string; disciplineIsRetired: false; subjectUuid: string; topicUuid: null; subtopicUuid: null; tags: string[]; }' is missing the following properties from type 'QuestionPoolMetadata': bloomCognitiveProcess, bloomKnowledgeDimension
tests/support/ribbon_shell_harness.tsx(260,7): error TS2739: Type '{ title: string; description: string; disciplineUuid: string; disciplineName: string; disciplineIsRetired: false; subjectUuid: string; topicUuid: null; subtopicUuid: null; tags: string[]; }' is missing the following properties from type 'QuestionPoolMetadata': bloomCognitiveProcess, bloomKnowledgeDimension
tests/support/ribbon_shell_harness.tsx(611,7): error TS2322: Type 'Promise<{ questionId: string; metadataEditNumber: number; questionTitle: string; questionDescription: string; questionType: QuestionType; tags: string[]; disciplineUuid: string; subjectUuid: string; topicUuid: null; subtopicUuid: null; }[]>' is not assignable to type 'Promise<readonly PublishedQuestionSharedMetadata[]>'.
  Type '{ questionId: string; metadataEditNumber: number; questionTitle: string; questionDescription: string; questionType: QuestionType; tags: string[]; disciplineUuid: string; subjectUuid: string; topicUuid: null; subtopicUuid: null; }[]' is not assignable to type 'readonly PublishedQuestionSharedMetadata[]'.
    Type '{ questionId: string; metadataEditNumber: number; questionTitle: string; questionDescription: string; questionType: QuestionType; tags: string[]; disciplineUuid: string; subjectUuid: string; topicUuid: null; subtopicUuid: null; }' is missing the following properties from type 'PublishedQuestionSharedMetadata': bloomCognitiveProcess, bloomKnowledgeDimension
```

## Formatting and diff

The before/after SHA-256 comparison is the generated-output diff summary above; no second generation pass was needed. `npx prettier --check docs/active_plans/reports/QUESTION_SPEC_CONTRACTS_BATCH.md` passed. Checking all generated files with `npx prettier --check --ignore-path /dev/null generated/api/*.ts docs/active_plans/reports/QUESTION_SPEC_CONTRACTS_BATCH.md` flagged these 9 files:

- `AssessmentEntry.ts`
- `AssessmentEntrySummary.ts`
- `AssessmentType.ts`
- `BlueprintAssessmentEntryInput.ts`
- `BlueprintAssessmentEntryView.ts`
- `LibraryStewardshipEvent.ts`
- `PreviewFutureSeam.ts`
- `QuestionIdSyntaxContract.ts`
- `Theme.ts`

All 9 hashes match the pre-generation snapshot; these formatter findings were not introduced by this batch. Generated files were left as emitted by their owning generators.

## Scope and acceptance

Only `generated/api/` and this report were in scope. No database or container was used. The TypeScript contracts are generated, but integrated acceptance remains pending the owners' browser/API updates and the repository's integrated checks.
