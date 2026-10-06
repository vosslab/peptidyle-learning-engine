# Blueprint Course import API specification

## Intended behavior

An importer builds Blueprint Courses through ordinary authenticated PLE operations. Imported
content supplies source data and relationships; PLE generates Blueprint Course and Assessment IDs.
The importer uses the Question and Pool references returned by PLE, rather than assigning public
IDs or constructing the expected database state directly.

Course assembly preserves the intended Assessment order, exact Published Question Revisions,
Pool references and selection counts, points, and supported Assessment settings. Reusing a Pool
keeps its identity. Creating an independent Pool follows the ordinary Pool fork rules.

A Blueprint Course supplies the starting Theme for a Course created from it. The Instructor can
then change that Course's Theme independently. This is ordinary use of the Blueprint, not a
separate adoption mode.

The result must be readable through PLE APIs with generated identities and the intended content
relationships intact. That is the evidence a real import system test needs. SQL fixtures remain
useful for lower-level tests; they do not establish the product import workflow.

## Scope and document ownership

[QUESTION_SPECS/QUESTION_IMPORT_SPEC.md](QUESTION_SPECS/QUESTION_IMPORT_SPEC.md)
owns Question creation. This document owns subsequent Blueprint Course assembly.
[BIOLOGY_PROBLEMS_SPECS/BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md](BIOLOGY_PROBLEMS_SPECS/BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md)
owns source-specific grouping into the base Courses. Neil chose one Assessment per website topic
as the starting organization, splitting long topics as needed. Topics do not automatically become
Pools.

The first-delivery choices and source completeness are recorded in
[FALL_2026_PILOT.md](FALL_2026_PILOT.md) and
[BIOLOGY_PROBLEMS_SPECS/README.md](BIOLOGY_PROBLEMS_SPECS/README.md).

## Current implementation differences

- Current Blueprint APIs create and replace a Course tree and return generated identities.
  Their internal Module structure does not prescribe how source problem sets become Assessments.
- Existing direct-store and SQL loaders do not prove the intended API creation and readback path,
  even when those loaders already generate IDs.
- Current Blueprint inputs have no Theme field. Carrying the Blueprint's starting Theme into a
  new Course is an implementation gap, not an unresolved product decision.
- The current feedback-timing request field does not settle the deferred optional feedback decision.
- The current Pool-order request value does not change HG's unordered Pool membership rule.

Detailed source evidence is retained in
[biology_course_spec_handoff.md](active_plans/reports/biology_course_spec_handoff.md).
The [alignment report](active_plans/reports/question_specs_alignment_report.md) separates these
implementation differences from product-specification conflicts. HTTP protocol and replacement
implementation design are outside this documentation-drift pass.
