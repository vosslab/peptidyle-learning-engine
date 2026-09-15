# Student-record retention policy

This document applies the current retention decisions in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). It distinguishes reusable teaching
content from FERPA-protected Student records and does not add lifecycle states
or recovery machinery beyond that guidance.

## Durable boundary

Published Questions and their immutable Question Revisions, Question Pools and
their revisions, Blueprint Courses and their revisions, Draft Questions, and
Instructor-owned authoring content are not Student records owned by a Course
Instance. Archiving or deleting Student records must not delete this reusable
teaching content.

A Student Account is global and is separate from the Student's data in a Course
Instance. Removing or deactivating the Student's Course access does not delete
the Account or Course work. Instructor Account deactivation and permanent
Account closure likewise remain separate from Course retention.

## Retention clock

The final Assessment deadline in a Course Instance starts the retention clock.
Later Student activity in that Course resets the clock. The retention duration
and notice interval are deployment policy; Human Guidance does not assign
numeric values, so this document does not invent them.

Before FERPA-protected Student records leave normal product interfaces, the
system notifies the Course's Instructors. At the archive point:

- Student Work and other FERPA-protected Course records leave normal Instructor
  and Student interfaces.
- The records remain recoverable during the configured retention period.
- Course metadata, Assessment definitions, Questions, and Course settings
  remain available to authorized Instructors.
- The Course becomes inactive after its FERPA-protected Student records are
  permanently deleted.

At the end of the configured retention period, the archived FERPA-protected
Student records are permanently deleted. Backup, deployment-log, and
operational-log retention are infrastructure policies and must not become an
undeclared product archive of Student records.

## Processing boundary

A background process checks stored deadlines and later Student-activity dates,
sends the required Instructor notice, performs the archive transition, and
permanently deletes records whose retention period has expired. Repeating the
same pass is idempotent: it must not duplicate notices, shorten a clock, revive
deleted records, or partially reapply a completed transition.

The exact table, job, event, receipt, and worker shapes are implementation
details, not product concepts. They may be documented only when implementation
evidence requires them. Human Guidance does not require a general-purpose
recovery-state machine, historical snapshot service, or compatibility layer for
this policy.

Draft Question cleanup and generic object-storage cleanup are separate technical
concerns. Neither is authority to delete FERPA-protected Student records or to
change a Course Instance's retention clock.

See [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md), and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) for the access and vocabulary
boundaries.
