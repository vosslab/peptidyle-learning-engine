# Fixed Tier 1 -> Tier 2 Ribbon Contract

> Only the Student Tier 2 investigation in this plan is superseded by the approved
> [Student Progress and Response Stats plan](../../archive/STUDENT_PROGRESS_RESPONSE_STATS_PLAN.md) and
> [Human Guidance](../../HUMAN_GUIDANCE.md).
> Instructor work, including evaluation of the Instructor's Student View,
> remains active.

## Context

Instructor Tier 2 currently changes when an Instructor opens a Course or
Assessment, even though Tier 1 stays the same. Human Guidance settles the
Instructor choices and the fixed-row rule. The Student investigation below has
been superseded by the approved Student contract above.

## Objectives

- Keep each settled role and Tier 1 mapping fixed across deeper routes; route
  state controls selection or no selection.
- Preserve existing Course, Assessment, Student, and Student View workflows as
  this mapping changes.
- Complete the work through repository evidence and automated checks, with no
  human approval milestone.

## Design philosophy

Resolve fixed rows from Product Role and Tier 1. Use hubs, page links, and
breadcrumbs for object-local navigation. Count a destination as reachable when
a user has a reasonable path to it from the relevant workflow, not merely when
its URL works.

## Scope and defaults

Preserve the settled Instructor mappings:

- **Courses:** My Blueprint Courses, My Active Courses, My Inactive Courses,
  Search Public Blueprint Courses.
- **Questions:** My Questions, My Draft Questions, Starred, Watched, Search
  Question Library, Browse Question Library.
- **Assessments:** Assessments Due Soon, My Assessment Templates.

Use the approved Student mapping in Human Guidance and Design Decisions; do not
reopen its destinations, order, or Course context in this Instructor-focused plan.

Review product evidence for a useful cross-Course Assessments destination,
including Browse All. A route does not need to exist already if Human Guidance
or the product model clearly establishes the recurring task. Evidence must
also support the destination's actual name and shape; a manager-suggested name
such as All Assessments is a candidate, not a predetermined choice. When
evidence does not establish the task, name, and shape, keep Assessments Due
Soon and My Assessment Templates, and record Browse All as a future
opportunity.

Preserve the answer-free Student View and Instructor identity contract.
Consider a temporary Student-like presentation if captured workflows show it
would better meet that contract; use the existing preview behavior when
evidence does not establish that improvement.

Sysadmin navigation remains out of scope. Preserve existing shell geometry
work.

## Milestones

1. **Capture baseline evidence.** Put temporary route inventories, route
   transition probes, and before captures under `tests/_temp/`. Keep them
   untracked and remove them after use.

2. **Implement settled Instructor rows.** Make the three Human Guidance
   mappings fixed across all Instructor routes sharing each Tier 1 choice.

3. **Clean up Instructor route metadata.** Stop Course and Assessment object
   routes from selecting Instructor Tier 2 contents. Keep route context for
   breadcrumbs and selected-item behavior.

4. **Preserve Course workflows.** Inventory reasonable paths to Course
   assessments, students, Gradebook, and appearance. Reuse current Course
   content links and breadcrumbs; add a local link only when the inventory
   shows a workflow would otherwise be lost. Prove the paths with temporary
   synthetic transitions.

5. **Preserve Assessment editor workflows.** Inventory Overview, Questions,
   Properties, and Student View paths before changing the row. Reuse the
   existing hub and breadcrumbs, adding a local link only for a demonstrated
   gap. Prove the resulting paths with temporary transitions.

6. **Resolve the Assessment row review.** Assess cross-Course needs using Human
   Guidance, product concepts, existing data sources, and workflow captures.
   Add a cross-Course destination only when the evidence establishes its task,
   name, and shape; then implement the smallest route that serves it. Otherwise
   retain the settled two-item row and record Browse All as a future
   opportunity.

7. **Evaluate Student View.** Capture Instructor -> Student View -> Instructor
   transitions and compare the preview against the intended Student
   experience. Adopt a temporary Student-like presentation only when evidence
   shows it materially improves the answer-free preview while retaining
   Instructor identity and avoiding Student state changes; otherwise keep the
   current preview route.

8. **Superseded: Student Tier 2.** Implement the settled Student destinations
   and route behavior in
   [STUDENT_PROGRESS_RESPONSE_STATS_PLAN.md](../../archive/STUDENT_PROGRESS_RESPONSE_STATS_PLAN.md).
   This plan continues to own the Instructor investigation and implementation.

10. **Check touched accessibility behavior.** Reuse existing accessibility and
    browser coverage for the migrated Ribbon controls, page links, focus order,
    keyboard operation, and selected states. Keep one-time probes temporary.

11. **Diagnose responsive geometry.** Use matched captures and temporary
    measurements to inspect row position and size across equivalent route
    transitions and supported widths. Correct route-caused shifts; keep the
    existing shell geometry and do not require pixel equality.

12. **Update Instructor documentation.** Keep the fixed-row invariant and
    settled Instructor mappings aligned across Human Guidance, the Ribbon
    model, design guide, and changelog. Student documentation follows the
    [completed Student plan](../../archive/STUDENT_PROGRESS_RESPONSE_STATS_PLAN.md).

13. **Integrate and verify.** Remove temporary probes and artifacts. Run
    focused contract and browser checks, capture the final canonical screens,
    and run the repository compliance checks.

## Test policy

Treat tests as liabilities as well as assets. Add permanent coverage only for
behavior that is intentionally stable, important, and likely to regress.
Prefer extending existing contract tests over creating new runners.

The fixed Tier 1 -> Tier 2 invariant earns permanent coverage in the existing
consolidated Ribbon contract test for the settled mappings. Keep route
inventories, migration probes, geometry measurements, screenshot comparisons,
reachability checks, and Student View transition probes temporary under
`tests/_temp/`; remove them after use. Extend existing accessibility or browser
coverage where it naturally owns the behavior. Promote other checks only when
the behavior itself merits long-term protection.

## Interfaces and non-goals

No public API or shared PageFrame API change is planned. Internal Ribbon
derivation and Instructor route metadata may change. Sysadmin redesign, general
accessibility auditing, and shell geometry redesign are outside this work.
