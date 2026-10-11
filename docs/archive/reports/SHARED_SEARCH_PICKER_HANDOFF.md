# Shared Search Picker Handoff

Historical preparation for M13-M14, now implemented and verified. The future-tense analysis
below records the starting seams; current behavior and completion evidence are in
[the architecture guide](../../SEARCH_PAGE_ARCHITECTURE.md) and
[the implementation ledger](SHARED_SEARCH_IMPLEMENTATION.md).

## Later Pool model correction

The October 5 decision in [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md)
supersedes the automatic Assessment-fork and Assessment-owned Pool workflow below. Adding a Pool
references its existing ID; the Assessment entry stores its selection count. Explicit forking
creates a regular Instructor-owned Pool. The paths below are retained implementation evidence,
not authority to preserve that behavior. Reconciliation is in [TODO.md](../../TODO.md).

## Scope and authority

This handoff prepares M13 and M14 in
`read-docs-active-plans-active-shared-sea-peaceful-mountain.md`.  It follows
`docs/HUMAN_GUIDANCE.md` and `docs/QUESTION_SPECS/QUESTION_POOL_SPEC.md`: Pool members are
Published Questions, retain exact Revisions, share Discipline, Subject, Type,
and Backend, and use a compatible supported license.  The approved plan keeps
the separate Pool-validation work out of these milestones.  M8-M9 remain the
authoritative server enforcement for every membership mutation.

M13 should use the existing shared search session, controls, results, and
selection bar inside the existing dialog.  M14 should use those same frozen
shared primitives for an Assessment-content search; it must not add
Library-specific imports to `src/features/search/`.

## Current Question picker and Pool paths

`src/features/question_picker/question_picker.tsx` owns a dialog, source
selector, local ordered selection tray, inspection, paging, and confirmation.
Its model has a separate `QuestionPickerSession` and
`QuestionPickerSourceRepository` in `question_picker_model.ts`.  Library and
My Questions sources call the current Question Library repository through
`questionLibraryPickerRepository`; Blueprint Assessment sources have a local
adapter.  The selected value is `QuestionPickerSelection`, which keeps the
canonical Question ID, display row, and exact published Revision tuple.

M13 replaces that picker session and its duplicated controls/results state
with a picker-owned search definition and `createSearchState`.  Keep the
dialog, source choice, inspection, ordered tray, maximum selection, and
`onConfirm` contract in the picker.  The source remains part of the
picker-definition query, so changing a source makes one replacement request
and clears its selection through the shared session.  The picker definition
continues to use the caller-owned `QuestionPickerSourceRepository`; it does
not make a Library request from shared code.

Current Question-picker callers are:

| Caller | Current action that must remain intact |
| --- | --- |
| `components/question_pool_create_dialog.tsx` | Creates a reusable Pool after explicit interchangeability attestation. |
| `features/blueprint_course/blueprint_pool_members_editor.tsx` | Adds exact Revisions to a retained Blueprint Pool draft. |
| `pages/assessment_workspace/assessment_pool_entry_editor.tsx` | Appends one exact Revision to an Assessment-owned Pool fork after attestation. |
| `features/blueprint_course/blueprint_assessment_content_editor.tsx` | Adds fixed Questions to a Blueprint Assessment draft. |
| `pages/assessment_workspace/assessment_workspace_questions_view.tsx` | Adds fixed Questions to a Course Assessment draft. |
| Blueprint Course creation/detail compositions | Keep their existing Library/My Questions source bindings. |

`QuestionPoolCreateDialog` has two modes.  A source-bound creation receives a
`QuestionPoolStartingQuestion`; `questionPoolMemberTuples` pins its exact
starting Revision first, resolves each added Question to its current exact
Revision, and rejects a duplicate starting Question.  `questionPoolSourcePickerRepository`
currently sends the starting Subject, filters page rows by starting Discipline,
and skips empty pages.  It checks Discipline/Subject again while resolving
details.  General Pool creation uses the regular Library source.

The current client-side source binding does **not** carry Type or Backend, and
does not express a license compatibility envelope.  M13 needs a typed
Pool-member eligibility input obtained from the fixed first member or the
existing Pool: Discipline identity, Subject identity, Question Type, Backend,
and supported-license policy.  Its definition must constrain the request
before display and retain a narrow defensive row check.  Do not infer Type or
Backend from Question format.  The current supported licenses are CC0, CC BY,
and CC BY-SA; the approved compatibility table permits every combination of
those three, so this release has no additional per-license exclusion among
them.  A future unsupported license must be excluded by the authoritative
server rule, rather than guessed from a browser row.

The existing final write paths remain required defenses: reusable Pool creation
uses `QuestionPoolCreationClient.createQuestionPool`; Blueprint Pool changes
are saved by the Blueprint editor; Assessment-owned Pool changes use
`appendAssessmentQuestionPoolForkMembers`.  Each sends exact tuples and an
explicit interchangeability attestation.  M13 preserves those calls and their
conflict/error behavior.

## Current Assessment Pool picker and fork paths

`src/features/question_pool_picker/question_pool_picker.tsx` is a second,
Pool-only dialog.  It calls `QuestionPoolLibraryClient.listQuestionPools`,
returns `QuestionPoolPickerSelection`, and has its own search/paging UI.

There are two consumers:

| Consumer | Current selection and mutation |
| --- | --- |
| `features/blueprint_course/blueprint_assessment_content_editor.tsx` | `confirmQuestionPool` appends the selected Pool ID and exact Pool edit number to the Blueprint draft. No server fork occurs until the Blueprint workflow that already owns that content. |
| `pages/assessment_workspace/assessment_workspace_questions_page.tsx` through `assessment_workspace_questions_view.tsx` | `choosePool` stages ID/edit number; explicit `Import Question Pool` validates capacity and calls `importAssessmentQuestionPoolFork` with the expected source edit number, then reloads. |

M14 replaces `question_pool_picker/` with one Assessment-content picker that
uses a definition over the M12 combined Library query.  It displays both
Questions and Pools, begins with the Library default and membership filter,
and returns a tagged selection (`question` exact tuple or `pool` ID plus edit
number/member count).  The Blueprint caller retains its draft append behavior.
The Course caller retains the separate staged selection, count/points/order/
scoring controls, and explicit import action; choosing a Pool must not bypass
the existing fork confirmation or expected-edit-number mutation.

## Safe ownership after M12

M13 owns only `src/features/question_picker/**`,
`components/question_pool_create_dialog.tsx`,
`components/question_pool_create_model.ts`,
`features/blueprint_course/blueprint_pool_members_editor.tsx`,
`pages/assessment_workspace/assessment_pool_entry_editor.tsx`, and their
focused tests.  It may make mechanical call-site changes needed to preserve
the stable Question-picker props, but it does not change Assessment Pool
import UI or `question_pool_picker/`.

M14 owns a new `src/features/assessment_content_picker/**`, deletes
`src/features/question_pool_picker/**`, and changes only the two Assessment
Pool selection call sites: `blueprint_assessment_content_editor.tsx` and
`assessment_workspace_questions_{page,view}.tsx`, plus its focused tests.
M14 consumes the frozen M13 Question-picker selection contract and frozen
`src/features/search/` primitives.  It does not edit Pool-member eligibility
or source-bound Pool creation.

## Meaningful acceptance checks

M13:

- Existing `test_question_picker.mjs` still proves source switching, exact
  selection, ordered tray, inspection, paging, retry, and confirmation.
- Extend `test_question_pool_source_binding.mjs` with eligible and ineligible
  Type, Backend, Discipline, Subject, and unsupported-license candidates.  It
  must prove an eligible Question already in another Pool remains offered.
- A browser check opens each Pool-member context, submits text once, selects an
  eligible Question, confirms, and verifies the current exact tuple and
  attestation mutation path receive it.  It must show no incompatible row.

M14:

- A temporary Playwright check adds a Question and a Pool through the one
  picker to a Blueprint Assessment and a Course Assessment.
- The Course path confirms its visible Pool selection is staged first, then
  creates the existing Assessment-owned fork only after the explicit import
  action, with the selected Pool ID and expected edit number.
- The Blueprint path preserves its existing draft semantics and selected Pool
  edit number.  The old `question_pool_picker/` has no imports after migration.


## Scoped Blueprint eligibility refinement

Read-only M13 preparation found that Blueprint Pool membership is returned by the existing
Blueprint-scoped reader, which checks visibility and membership in the requested Assessment.
Its current tuple-only response cannot tell the picker the Pool's authoritative classification.
The generic current-Pool endpoint is not tied to that Blueprint Assessment boundary, so M13
should expand the existing scoped response rather than substitute the generic reader.

Add Pool `disciplineUuid`, `subjectUuid`, `questionType`, and `backend` as an eligibility value
to the SQL/store/server/generated/browser response. Read these from the Pool row. Deriving
classification from the first pinned Question is incorrect: later Question reclassification does
not change the Pool's classification. All three supported licenses fit, so this addition needs
no separate license compatibility matrix. The Assessment fork view already carries these facts.

Verify the scoped reader still conceals unrelated Pool IDs and returns Pool classification even
when a member Question's classification has changed. M13 completed this refinement and its
fresh connected proof; see the implementation ledger.
