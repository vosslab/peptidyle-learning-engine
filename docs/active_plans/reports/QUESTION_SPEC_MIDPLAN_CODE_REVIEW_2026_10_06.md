# Question-spec midplan code review

## Result and scope

Six fresh reviewers completed focused plan, test, style, documentation, legacy-code, and comment
reviews of the current working diff (about 685 changed files). The review found concrete corrections
and no additional architectural mismatch
in the reviewed Pool, Revision, Draft, scoring, import, or Sysadmin paths. This was a focused risk
review, not full runtime acceptance or blanket certification of the diff.

The implementation was reviewed by `midplan_plan_luna`, `midplan_test_luna`,
`midplan_style_luna`, `midplan_docs_luna`, `midplan_legacy_luna`, and `midplan_comment_luna`.

## Findings and disposition

### Withdrawn: Library bulk metadata workflow was treated as removal work

The reviewer interpreted "deferred" as an instruction to remove the existing workflow. Human
clarified that deferred means leave the implementation alone. The removal finding is withdrawn;
the existing Library bulk-edit implementation remains in place, and no product specification change
was needed.

### Low: JavaScript key order made two test expectations brittle

The test reviewer found two tests depending on `Object.keys` order. `midplan_test_order_fix_luna`
changed the expectations to avoid treating property enumeration order as product behavior; the
focused set of eight tests passed. The reviewer also proposed a broader fork oracle. The current
user-directed compaction retains the score invariants as the acceptance focus; that disagreement is
recorded without expanding this task.

### Documentation: TODO overstated missing source implementation

The documentation reviewer found that the FIB item described regex as absent, and the Draft item
described the authoring server as Native JSON only. Current source includes regex handling for
FIB/MULTI-FIB and WebWork Draft preview, response testing, and publication paths. The TODO now
tracks connected acceptance for those paths; this wording records source implementation without
claiming connected database, Assessment-score, or browser acceptance. The linked M19 report records
focused source tests, while M16 dependency and end-to-end acceptance remain pending.

### Low: Dead import CLI state after M28 route removal

The legacy review found `ValidatedPilotQuestionImport` and `validated_question_imports` left behind
after the M28 CLI removal. `midplan_low_code_fixes_luna` owns the cleanup. The connected M28 import
acceptance remains pending; removing dead CLI state does not change that status.

### Low: Stale Pool and Sysadmin comments/labels

The comment review found an outdated immutable-fork Pool comment in
`crates/question_model/src/assessment_workspace.rs` and permanent Sysadmin scenario labels that
misstated their M26/M29 coverage. `midplan_low_code_fixes_luna` owns these corrections. The labels
must describe the actual scenarios and do not establish broader milestone acceptance.

### Style: no findings

`midplan_style_luna` reported no style findings in the reviewed scope.

## Acceptance boundary

This review does not establish full test-suite, database, browser, or Live Demo acceptance. It also
does not resolve the known numeric shared-measure limitation caused by the current three-Student
fixture being below the five-contributor privacy floor. Keep source implementation, focused review,
and connected runtime acceptance as separate statuses. The root integration pass owns final status
reconciliation after the assigned corrections land.
