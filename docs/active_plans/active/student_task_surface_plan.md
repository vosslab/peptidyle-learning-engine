# Plan: Student task surface

## Current execution and acceptance

Reconcile this older plan with current source before implementing changes. The obsolete response
restore controls and full phone wordmark were removed in the September 24 work recorded in
[CHANGELOG-2026-09p.md](../../CHANGELOG-2026-09p.md). Preserve those fixes and verify the remaining
wrapper, navigation, and usable-space outcomes against current captures.

The manager runs the existing fast checks to establish a fresh baseline and assigns demonstrated
failures to implementation agents. A stale statement that stabilization is pending is not an
external dependency. Fresh test and visual-review agents assess the actual Student journey,
keyboard operation, visible nearby Question navigation, and clipping at supported viewports.
Use local seeded Accounts and automated browser interaction; independent capture review replaces
human visual sign-off. Treat the old 444px figure as a diagnostic baseline, not a pass/fail cap.
Fix observed behavior failures and repeat the affected checks. Finish after independent acceptance,
the applicable aggregate gate, and documentation; no milestone needs a human reply or deployment.

## Context

The Student stability-and-density audit identifies bounded task-surface problems: excess response
recovery controls, five visually separate WeBWorK boxes, native/WeBWorK surface drift, fixed-width
Question navigation, phone branding density, and a 444px attempt vertical budget. This is a single
Student interface pass, not a shell redesign.

Establish the current baseline with the fast gate. The manager assigns any demonstrated regression
to an agent and continues independent checks while that repair proceeds.

## Objectives

- Keep the Question response surface focused on answering, saving, navigating, and submitting.
- Make WeBWorK and native Question surfaces follow one compact visual hierarchy.
- Spend Question-navigation width on nearby Questions before fixed control width.
- Keep the phone Student surface within the audit's measured vertical budget where the task permits.

## Design philosophy

Remove controls and boxes that do not advance the Student's immediate task rather than adding
another mode or workflow. The audit's existing task sequence is the boundary; a general Student
redesign is rejected.

## Scope

- Verify the removal of Undo and Restore response controls and their obsolete presentation state.
- Flatten the five WeBWorK containers to one surface and align the native Question surface to it.
- Make Question navigation show `...` and allocate narrower Previous/Next controls from measured width.
- Remove the word "Peptidyle" from the narrow-phone product treatment.
- Measure active-attempt vertical use against 444px in the named Student audit scenarios.

## Non-goals

- Do not change Question Backend grading, response persistence, or submission semantics.
- Do not add a new Student workflow, Ribbon destination, preference, or navigation mode.
- Keep unrelated feature development outside this task; assign necessary baseline repairs as
  bounded agent tasks with their own checks.

## Current state summary

The historical audit named obsolete response controls, five WeBWorK boxes, and a Question
navigator unable to show a useful nearby range on phone. Reproduce any remaining issue against
current source before changing it. SUI-04 supplies the acceptance
examples; SUI-08 supplies the attempt vertical-budget rule. SUI-06 separately requires one
cross-surface Student terminology review; it is a deferred acceptance item, not M6 or WP-H2 work.

## Approach

1. Verify the existing removal of the reset/restore surface and state; correct only demonstrated
   regressions while preserving normal response issue, save, and submit behavior.
2. Flatten WeBWorK wrappers first, then apply the same resulting surface hierarchy to native
   Question content without changing either backend contract.
3. Measure available navigator width at 393, 600, 800, and 1280px (including enlarged text), then
   select the largest endpoint/current/adjacent-number/ellipsis sequence that fits.
4. Apply the narrow-phone branding and button-width changes, then capture the attempt's first 444px
   for unanswered, saved, and submitted states.

## Critical files

- `src/components/question_response_controls/common.tsx` and its seven current call sites.
- `src/pages/assessment_attempt_page.tsx`, `src/components/student_assessment_attempt_navigation.tsx`,
  and `src/components/student_assessment_attempt_navigation.css`.
- The WeBWorK and native Question presentation wrappers identified by the implementation owner.
- `docs/active_plans/audits/student_ui_stability_and_density_audit_2026-09-21.md`.

## Acceptance criteria and gates

- No Student response surface exposes Undo or Restore, and no response-control state is named
  `restored`.
- One visual container, rather than five nested boxes, frames the WeBWorK task; native Questions
  match its hierarchy without losing backend-owned rendering boundaries.
- Omitted Question ranges visibly render `...`; a fitting row prefers nearby numbers such as
  `1 2 ... 3 [4] 5 ... 8 9` over endpoints alone.
- Previous/Next remain keyboard reachable with long labels and enlarged text.
- The 444px measurement is reported for the audit scenarios rather than treated as a universal CSS cap.
- **SUI-06 acceptance:** an independent agent reviews Ribbon labels, breadcrumbs,
  headings, buttons, and status messages across the Student Attempt journey. Keep the Assessment
  Type for the item, `Attempt` for the saved or submitted work unit, and `Coursework` for the
  collection; keep the type and Attempt context adjacent to a submission action. This does not
  authorize a general terminology rewrite or change the current implementation scope.
- Fast checks, focused automated Student browser evidence, independent capture review, and
  `git diff --check` pass.

## Risk register

| Risk                                        | Impact                        | Trigger                                                          | Owner                  | Mitigation                                                       |
| ------------------------------------------- | ----------------------------- | ---------------------------------------------------------------- | ---------------------- | ---------------------------------------------------------------- |
| Removing reset changes a response contract  | Student work behavior changes | A focused response-control check fails                           | response-control owner | Stop and restore the narrow package; do not redesign persistence |
| Native and WeBWorK flattening diverge       | Inconsistent task hierarchy   | One surface needs an extra visual wrapper                        | task-surface owner     | Compare rendered task states before accepting either             |
| Width calculation hides reachable Questions | Navigation regression         | A width scenario omits current or adjacent context unnecessarily | navigation owner       | Use measured candidates and test each listed viewport            |

## Verification

Run the fast gate plus focused response-control and Student navigation checks. A fresh visual-review
agent inspects current captures at the four widths. A failing ellipsis, keyboard path, clipping, or
response contract returns the relevant package to its implementation agent for repair and recheck.
