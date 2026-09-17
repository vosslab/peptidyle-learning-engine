# Student-record retention policy

This policy owns the timing and repeatable processing of protected Student
educational records. It applies the product boundary in
[FERPA_DATA_POLICY.md](FERPA_DATA_POLICY.md).

## Related data policies

These policies intentionally overlap where FERPA classification, authorization,
and retention meet. [FERPA_DATA_POLICY.md](FERPA_DATA_POLICY.md) is the primary
FERPA reference and owns the Student educational-record boundary. This policy
owns retention-lifecycle details. [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)
owns the broader data-classification and handling model.

User decisions and [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remain the overall
authority.

## Protected-record boundary

For this lifecycle, protected records are Course-scoped evidence of a Student's
educational activity or outcome: membership history; Attempts and state; saved
and final responses; grades, scores, credit, and outcomes; Student-specific
feedback; Student-linked timing, deadlines, accommodations, and activity;
issued Question or Pool selections; interpretation evidence; analytics; and
linkable opaque or pseudonymous IDs. Removing a name does not remove the
sensitivity.

Reusable Course metadata, Assessment definitions, configuration, settings,
Questions, Pools, Blueprints and Revisions, Instructor authoring, independent
global Account data, and genuinely privacy-safe aggregates are outside this
Student-record lifecycle only when independent of identifiable or linkable
individual Student evidence. Student-linked evidence embedded in any of them
remains protected. A Published Question reused by reference does not become
Student Work; its Student-linked delivery or selection record remains
protected. "Not FERPA" does not mean public: Account data remains personal and
authentication-sensitive, and Answer Keys, private authoring, credentials, and
backend state remain restricted. This is PLE's protective product
classification, not a categorical legal ruling.

## Active and Inactive Courses

A Course Instance represents one teaching period and remains Active for at most
six months from creation. PLE warns its Instructors before that limit and makes
the Course Instance Inactive when the limit is reached. An Assessment deadline
cannot extend beyond the limit; teaching another period requires a new Course
Instance.

The latest Assessment deadline starts the Student-record retention clock.
Creating an Assessment with a later deadline or extending a deadline can move
that clock within the six-month Active lifetime. Active and Inactive are Course
lifecycle states, not retention stages: becoming Inactive does not itself send
notice, archive records, remove ordinary access, or delete records.

## Current operational schedule

The current operational schedule is measured from the latest Assessment
deadline, not from a Student action, Course creation, Course inactivity, or a
previous retention milestone. These are target operational defaults, not
statutory requirements or immutable product constants.

| Time from latest deadline | Required outcome |
| --- | --- |
| Day 0 | Normal Course access continues; starting the clock takes no retention action. |
| Day 30 | Notify each current Course Instructor that records will leave normal access. |
| Day 100 | Archive FERPA-sensitive Course records and begin recovery. |
| Day 365 | Permanently delete FERPA-sensitive Course records. |

Under the normal schedule, recovery is available when `100 <= day < 365`. It
is measured from the latest Assessment deadline, not as an additional period
after archive. If records were archived accidentally or prematurely, an
authorized current Course Instructor may explicitly recover them until that
same original deletion deadline. Early archival does not reset or extend the
deadline, restore ordinary browsing, or create a new lifecycle state. The
access and recovery boundary itself belongs to
[FERPA_DATA_POLICY.md](FERPA_DATA_POLICY.md).

The current schema's operational defaults express this schedule as a 70-day
archive-notice lead, a 100-day archive point, and a 265-day interval from that
same scheduled archive cutoff to deletion. A fresh isolated PostgreSQL
installation verified the derived day-30 notice action, day-100 archive action,
and day-365 deletion cutoff, including deletion at day 365 after a day-200
archive. That database proof does not establish a deployed notification worker,
email delivery, or production scheduling. These are operational configuration,
not statutory requirements or durable product constants.

## Processing boundary

A background process checks stored Assessment deadlines, sends the required
Instructor notice, performs the archive transition, and permanently deletes
records whose policy milestone has passed. Repeating the same pass is
idempotent: it must not duplicate notices, shorten a clock, revive deleted
records, or partially reapply a completed transition.

The exact table, job, event, receipt, and worker shapes are implementation
details, not product concepts. Draft Question cleanup and generic object-storage
cleanup are separate technical concerns; neither changes this retention clock.

## Durable boundary

Published Questions and their immutable Question Revisions, Question Pools and
their revisions, Blueprint Courses and their revisions, Draft Questions, and
Instructor-owned authoring content are not Student records owned by a Course
Instance. Retention processing must not delete this reusable teaching content.

A Student Account is global and remains separate from the Student's Course
records. Removing or deactivating Course access does not delete the Account or
Course work. Instructor Account deactivation likewise remains separate from
Course retention. No permanent Account-closure workflow is currently defined.
