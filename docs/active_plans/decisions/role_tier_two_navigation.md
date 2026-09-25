# Role Tier 2 Navigation

## Current roles

Instructor Tier 2 rows are fixed by Product Role and Tier 1. Routes select the current destination
when it matches; deeper context does not change the row.

Student Tier 2 keeps a stable row while the Student moves within a Tier 1 area. The current grouping
and Active Attempt selection evidence are in
[DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md#student-tier-2-groups-follow-tier-1-purposes) and
[STUDENT_PROGRESS_RESPONSE_STATS_PLAN.md](../../archive/STUDENT_PROGRESS_RESPONSE_STATS_PLAN.md).
The Student Course-context decision is settled: Coursework and Grades stay global, and opening a
Course from Courses enters explicit Course-specific context without a persistent pin. See
[Human Guidance](../../HUMAN_GUIDANCE.md#student-course-and-coursework-interface).

Instructor Tier 2 stays Courses, Questions, and Assessments with the ordered choices in
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). Course- and Assessment-specific actions remain in page
content and breadcrumbs. The Assessments row stays Assessments Due Soon and My Assessment Templates;
Browse All remains a future opportunity.

## Separate future product choices

Human Guidance settles Courses Tier 2 as the Student's current Course short names. Whether to add a
separate Course overview destination is a future product choice outside this navigation model. The
complete Sysadmin Tier 2 task layout also remains unspecified.
