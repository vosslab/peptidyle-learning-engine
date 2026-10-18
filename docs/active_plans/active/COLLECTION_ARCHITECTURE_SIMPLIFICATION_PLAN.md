# Collection architecture simplification plan

## Goal

Reduce the permanent effort needed to implement and modify ordinary collections while preserving
the reusable `PageFrame`, `SearchBlock`, `DataSheetControls`, `DataSheetBlock`, `DataSheetTable`,
`HeaderRow`, and `RecordItem` components. The named folders already exist. Complete worthwhile,
demonstrated simplifications during preproduction; do not add compatibility layers.

## Original findings

- `SearchSession` and `createSearchState` hold duplicate values. `SearchBlock` is the sole caller of
  `SearchResults`, which also adapts search state.
- A broad controls callback exposes table state. Field and query setup repeats across consumers.
- `RecordListQuery.kind` is unused, as is the index-based reorder branch.
- `Library` collection setup repeats across `LibraryPage`, `AssessmentEditor` available questions, and
  `QuestionPoolAvailableQuestions`.

## Milestones

| Milestone | Owner | Outcome | Check | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| M1 | Sol | Consolidate `SearchSession` and `createSearchState` into one reactive search controller and lifecycle authority; the surviving name is an implementation choice. Initialize effective `initialQuery` once; preserve latest-wins requests, exact retry, retained rows, disposal, meaningful-edit guards, selection, criteria-clear, and page/sort/mode preservation. Adapters provide source lifecycle without a competing controller. | Exercise those transitions through representative collection consumers. | Accepted | Independent SPEC and distinct QUALITY assessments passed. Canonical fast and full checks passed; final maintained UI replays passed. |
| M2 | Sol | Have `DataSheetBlock` render standard `DataSheetControls` before the record region, receiving pagination binding from the query controller. Remove only the controls render callback that exposes table state. Require fields and query. Keep four modes; operation bindings express Assessment filtering and no display sort, while the editor owns membership and order. `DataSheet` does not execute queries. | Check the four modes, filtering, appropriate pagination and sorting capabilities, and editor-owned membership/order. | Accepted | Independent SPEC and distinct QUALITY assessments passed. Canonical fast and full checks passed; final maintained UI replays passed for Compact, List, Visual Boxes, and Cards. |
| M3 | Sol | Fold the `SearchResults` adapter into `SearchBlock`: adapt search lifecycle there while the controller remains the sole state owner. `DataSheetBlock` owns presentation, including paging and empty state; preserve drag invalidation and support page-supplied controls above or beside it. | Confirm controller state ownership, lifecycle adaptation, presentation ownership, and both control placements. | Accepted | Independent SPEC and distinct QUALITY assessments passed. Canonical fast and full checks passed; final maintained UI replays passed. |
| M4 | Luna | Change the existing `createLocalCompleteSearch` to return `{state, collection: {fields, query}}`; migrate consumers so fields are declared once. Compare `StudentCourses`, `InstructorCourses`, and local editor contents. | Compare each consumer's fields, query, and resulting collection behavior. | Accepted | All 48 local consumers declare fields once. Independent SPEC and distinct QUALITY assessments passed; canonical fast and full checks passed; final maintained UI replays passed. |
| M5 | Sol | Add one domain `Library` assembly function combining the existing definition, controller, fields, query, and bindings. Migrate `LibraryPage`, AssessmentEditor available questions, and `QuestionPoolAvailableQuestions`. Keep caller definitions and callbacks responsible for their scopes, eligibility, permissions, actions, guards, and classification. Keep Starred as a separate sort adapter; add no page-mode framework. | Compare all three consumers' scopes and actions; verify Starred ordering. | Accepted | All three Library consumers use the helper for fields and query; domain actions and scope remain in their callers. Independent SPEC and distinct QUALITY assessments passed; canonical fast and full checks passed; final maintained UI replays passed. |
| M6 | Luna | Remove unused `RecordListQuery.kind`, rename the binding `DataSheetQuery`, and remove the unused index reorder provider branch/context property after confirming callers. Retain active ID-based saved ordering and the array reorder helper. | Inspect callers and verify saved Assessment order still maps by stable IDs. | Accepted | Independent SPEC and distinct QUALITY assessments passed. Canonical fast and full checks passed. A one-time probe confirmed two-entry Assessment order persists by stable ID after reload; final maintained Blueprint replay passed. |
| M7 | Luna | Use existing `fieldId` for genuinely equivalent Library text facts. List mode suppresses a fact only when its corresponding column is visible; hiding the column restores the fact. Keep richer links and details in Cards/VisualBoxes. | Check visible/hidden column behavior and richer mode-specific content. | Accepted | Details visibility repair passed independent SPEC and distinct QUALITY assessments. Canonical fast and full checks passed. A one-time probe confirmed hiding the List column restores Details; the maintained scenario verified the Cards switch, then captured Compact, List, and Visual Boxes. |
| M8 | Manager | Independently review whether integration is simpler in actual use, not merely passing tests. Update existing architecture docs and changelog; remove temporary code. | Review ordinary field/query declarations, caller boundaries, canonical checks, and temporary-code cleanup. | Accepted | Final independent integration review accepted: no material source/composition gaps; ledger and cleanup evidence confirmed. Verification is complete: canonical fast checks exited 0 (9,423 Python tests plus Rust, TypeScript, lint, and format checks); full suite exited 0 (9,423 Python tests); disposable PostgreSQL authority/persistence checks passed. Corrected-source capture built and imported 490/490. The combined wrapper stopped at an obsolete count-readiness assertion; separate maintained Library and Blueprint replays exited 0 against the same stack. Mixed-result screenshots are in `tests/_temp/collection_verification/final_mixed/`; the course search screenshot is `tests/_temp/collection_verification/courses-search-filtered_results.webp`. Executable probes were removed; supported Demo stop exited 0. Logs: `/tmp/ple_collection_final_fast.log`, `/tmp/ple_collection_final_all.log`, `tests/_temp/collection_verification/final_mixed_capture.log`, and `tests/_temp/collection_verification/final_blueprint_capture.log`. |

## Approved follow-on: Blueprint terminology correction

The accepted Blueprint terminology correction is included in this plan's closeout. Public visibility
applies to every Blueprint, including Archived ones; lifecycle is Active or Archived, while immutable
Revisions remain separate. Existing behavior and authorization are preserved. Source, docs, schema,
API, and generated contracts were updated; `activeOnly`, `activeCount`, and active-scope behavior
passed independent SPEC and distinct QUALITY reviews and focused checks. The final maintained
Blueprint replay exited 0 against the same stack as the Library replay. Archived Public visibility is
supported by source, API, schema, and authority checks; the screenshots demonstrate active discovery
rendering and display modes.

A one-time probe confirmed List column hide/Details restore and saving and reloading a two-entry
Assessment order; its evidence predates the Blueprint terminology correction. The initial combined
wrapper stopped only at an obsolete count-readiness assertion; separate maintained Library and
Blueprint replays passed. The probe executable was removed. Final integration review accepted the work with no material
source/composition gaps; all canonical checks, runtime acceptance, and temporary executable cleanup
are complete.

## Bounded collection continuation

The following justified adjacent simplifications continue this plan after the terminology work.
M1-M8 and the Blueprint terminology correction remain accepted; these tasks do not reopen them.

| Task | Owner | Outcome | Check | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| C1 | Sol | Remove `Promise<unknown>` and page revalidation from the Library repository. Keep strict HTTP decoding at the boundary, remove the `api` to `pages` model dependency, and map successful Library fields once. Preserve typed caller scopes. | Check decoder rejection of malformed responses, one-time field mapping, and all existing Library scopes and behavior. | DONE | Typed Library projection probe passed; implementation and focused source review accepted. Probe: `/tmp/ple_typed_library_projection.log`. |
| C2 | Luna | Move Draft list and delete HTTP decoding into the existing Draft API owner. Keep page calls typed and preserve current errors and delete behavior. | Check malformed response handling, typed page calls, surfaced errors, and delete behavior. | DONE | Draft API owner probe passed; implementation and focused source review accepted. Probe: `/tmp/ple_draft_api_owner.log`. |
| C3 | Fresh SPEC reviewer, then distinct QUALITY reviewer | Review the integrated C1-C2 result after implementation. Run canonical fast checks; run the full suite only if integration impact requires it. | Record separate SPEC and QUALITY decisions and the exact canonical checks. Repair failures at their owner and repeat the affected review or check. | DONE | Fresh SPEC and distinct QUALITY reviews ACCEPT. Canonical fast checks passed: 9,423 Python tests plus Rust, TypeScript, lint, and format checks (`/tmp/ple_collection_continuation_fast.log`, exit 0). Full-suite and browser receipts predate this continuation; reviewers found no UI behavior change requiring a repeat. |
| C4 | Manager | Integrate accepted changes and update the relevant docs and changelog. Close this continuation with source, review, and check evidence. | Confirm no temporary implementation machinery remains and record the final status here and in `current_work.md`. | DONE | Final Luna integration ACCEPT. Temporary probes were removed; changelog updated. Full-suite and browser evidence remain limited to the earlier accepted plan work. |

Keep the continuation within these Library and Draft API ownership boundaries. The Rust transaction
helper remains deferred; reopen it only through a separately justified scope decision.

## Scope and sequencing

Work in the existing SearchBlock/DataSheet and affected consumer folders, with related docs and tests.
Make shared-interface changes before consumer migrations; serialize edits that overlap. Adjacent
changes fit when representative callers demonstrate lower maintenance, clearer ownership, or less
configuration. Keep backend, schema, and thumbnail work unchanged unless evidence shows a related
architecture need.

The manager and subagents complete the milestones and bounded continuation autonomously. Automated
checks and independent agent assessments provide acceptance evidence; no task requires human
interaction. When a check fails, diagnose the cause, repair it at its owner, and rerun the affected
check. Report a genuine unrecoverable blocker with its evidence instead of waiting passively. Keep
each task open until its required evidence is recorded.

Preserve full-query sorting and filtering, selection, columns, the four display modes, direct
add/remove, Included behavior, saved Assessment order, and unsaved-edit protection. Compare source
declarations to confirm ordinary fields and domain setup are declared once and shared algorithms do
not move into callers.

## Verification

Inspect existing tests first. Keep permanent tests only for meaningful missing stable contracts;
place disposable checks in `tests/_temp/` and remove them at closeout. Run the canonical fast checks
for the continuation. Run the full suite only if implementation or integration impact requires it:

```bash
source ./source_me.sh && ./launchers/run_fast_checks.sh
```

Record focused checks separately from the fast checks and any full-suite run. C3 records independent
SPEC and QUALITY reviews; C4 records integration and documentation closeout.

For UI changes, capture screenshots with the repository's canonical workflow:

```bash
source ./source_me.sh && ./devel/capture_screenshots.sh
```

Report focused checks separately from full-suite and rendered-artifact acceptance. Use `git mv` for
tracked moves. Unrelated Git staging and commits are outside this plan's scope and do not gate
completion.
