# Role Tier 2 Navigation

## Context

The signed-in shell reserves space for the breadcrumb and Task Row. Human
Guidance settles the Instructor destinations and the fixed-row rule. It leaves
Student Tier 2 destinations and order unsettled. The current Student interface
can render route-specific controls on Attempt pages and no controls on other
routes; neither state establishes the intended fixed menu.

The complete Sysadmin Ribbon task layout also remains unresolved. This record
preserves current Sysadmin behavior without treating a route with no controls
as an approved empty menu.

## Decision

- Instructor Task Rows derive from Product Role and Tier 1 and keep their
  Human Guidance order across deeper routes. Course- and Assessment-specific
  navigation stays in page content and breadcrumbs.
- Student Tier 2 destinations and order remain unknown. Preserve current
  Student behavior. The Attempt and Back to Coursework controls serve current
  Attempt navigation; they do not establish a fixed menu for all Student
  routes under Coursework.
- Sysadmin Tier 2 remains outside this implementation. Preserve current
  behavior while the complete task layout remains undecided.
- Keep the Instructor Assessments row as Assessments Due Soon and My Assessment
  Templates. Human Guidance establishes cross-Course attention for upcoming
  Assessments and a separate reusable Template workflow. It does not establish
  a general all-Assessments collection, its name, or its shape. Browse All is
  a future opportunity for product evidence to evaluate.

## Evidence

- [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) lists the three Student primary
  tabs and says Student tier-two tasks remain unsettled.
- [route_contract.ts](../../../src/route_contract.ts) declares Course,
  Coursework, Grades, Attempt, and Attempt-summary routes. Only Attempt routes
  currently select the route-scoped Student task row.
- The current [Student Attempt screen](../../screenshots/student/laptop/assessment_navigation.png)
  shows Attempt and Back to Coursework. Those links address one active Attempt
  workflow.
- Human Guidance defines Assessments Due Soon as upcoming work across an
  Instructor's Courses. The route and API return that due-soon collection;
  there is no Human-Guidance-backed all-Assessments task or collection route.

## Follow-up

Revisit Student Tier 2 only when product evidence establishes fixed
destinations and their order across the applicable Student Tier 1 areas.
Repeated workflow evidence identifies needs to investigate but does not alone
settle the menu design.
