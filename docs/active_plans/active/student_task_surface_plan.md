# Plan: Student task surface

## Context

The Student stability-and-density audit identifies bounded task-surface problems: excess response
recovery controls, five visually separate WeBWorK boxes, native/WeBWorK surface drift, fixed-width
Question navigation, phone branding density, and a 444px attempt vertical budget. This is a single
Student interface pass, not a shell redesign.

The current fast gate is red. Stabilization is a prerequisite; no package below begins until it is
green.

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

- Remove Undo and Restore response controls, their seven call sites, and the response-control
  `"restored"` presentation state.
- Flatten the five WeBWorK containers to one surface and align the native Question surface to it.
- Make Question navigation show `...` and allocate narrower Previous/Next controls from measured width.
- Remove the word "Peptidyle" from the narrow-phone product treatment.
- Measure active-attempt vertical use against 444px in the named Student audit scenarios.

## Non-goals

- Do not change Question Backend grading, response persistence, or submission semantics.
- Do not add a new Student workflow, Ribbon destination, preference, or navigation mode.
- Do not solve the current fast-gate failures inside this task-surface plan.

## Current state summary

`question_response_controls/common.tsx` owns the response reset presentation and the `restored`
phase. The audit names five WeBWorK boxes and identifies the current Question navigator's fixed
five-slot logic as unable to show a useful nearby range on phone. SUI-04 supplies the acceptance
examples; SUI-08 supplies the attempt vertical-budget rule. SUI-06 separately requires one
cross-surface Student terminology review; it is a deferred acceptance item, not M6 or WP-H2 work.

## Approach

1. After stabilization, remove the reset/restore surface and state as one response-control package;
   preserve normal response issue, save, and submit behavior.
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
- **Deferred SUI-06 acceptance:** before this plan closes, review Ribbon labels, breadcrumbs,
  headings, buttons, and status messages across the Student Attempt journey. Keep the Assessment
  Type for the item, `Attempt` for the saved or submitted work unit, and `Coursework` for the
  collection; keep the type and Attempt context adjacent to a submission action. This does not
  authorize a general terminology rewrite or change the current implementation scope.
- Fast checks, focused Student browser evidence, rendered PNG review, and `git diff --check` pass.

## Risk register

| Risk                                        | Impact                        | Trigger                                                          | Owner                  | Mitigation                                                       |
| ------------------------------------------- | ----------------------------- | ---------------------------------------------------------------- | ---------------------- | ---------------------------------------------------------------- |
| Removing reset changes a response contract  | Student work behavior changes | A focused response-control check fails                           | response-control owner | Stop and restore the narrow package; do not redesign persistence |
| Native and WeBWorK flattening diverge       | Inconsistent task hierarchy   | One surface needs an extra visual wrapper                        | task-surface owner     | Compare rendered task states before accepting either             |
| Width calculation hides reachable Questions | Navigation regression         | A width scenario omits current or adjacent context unnecessarily | navigation owner       | Use measured candidates and test each listed viewport            |

## Verification

The current red fast gate blocks work. Once green, run the fast gate plus focused response-control
and Student navigation checks, then inspect native PNGs at the four widths. A failing rendered
ellipsis, keyboard path, or response contract blocks the relevant package.
