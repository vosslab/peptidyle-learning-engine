# Course relationships and Student records

This document applies the Account, Course, role, and FERPA decisions in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Existing enrollment, invitation,
receipt, or `assignment` names describe current implementation only.

## Core distinction

PLE keeps four concepts separate:

| Concept | Meaning |
| --- | --- |
| Global Account | One login identity with one immutable Product Role |
| Course relationship | A Student or Instructor's current access to one Course Instance |
| Student record | That Student Account's FERPA-protected data in one Course Instance |
| Assessment Attempt | Student Work for one Course Instance Assessment |

Removing Course access or deactivating an Account changes authorization. It
does not erase Student Work, Course history, authorship, or Instructor
relationships. Course retention remains separate from Account deactivation.
No permanent Account-closure workflow is currently defined.

## Product roles

Each Account is exactly one of Student, Instructor, or Sysadmin. A person who
needs multiple roles uses separate Accounts.

Course relationships do not change Product Role. Every current co-Instructor
has equal Course authority; the creator or first Instructor is not a Course
owner. A Sysadmin has no implicit Course relationship or ambient FERPA access.

Future Course Observer, Student Observer, and Grader roles require separate
Course-scoped relationship and privacy designs. They are not current Product
Roles or inferred capabilities. Grader is not currently needed because Question
grading is automatic.

## Creating Course relationships

An authorized Course workflow may invite or add an Account whose Product Role
matches the Course relationship. Acceptance rechecks the Account, role, Course,
invitation state, and current Course policy. Invitation possession alone is not
permanent Course authority.

Adding an Instructor creates an equal co-Instructor relationship. Adding a
Student creates or reconnects the Course relationship to the Student record for
that global Student Account. Rejoining must not create a second Student identity
or orphan earlier Course work.

Instructors may bulk add Students through roster import. They remove Students
individually. PLE does not provide bulk Student removal from a Course Instance.
Roster import supports efficient enrollment of a class; it is not a workflow
for replacing the roster of an earlier teaching period.

Students and Instructors authenticate with a passkey or email code. Student
Accounts use the required university or institutional email address (`.edu` in
the United States), and a Student email address is immutable. The exact
invitation token, challenge, or LMS ceremony is an implementation
contract; Human Guidance does not require a particular invitation receipt
model.

An Instructor may reset a Student's login access for that Course and send a new
signup code. This restores an authentication path; it does not create a new
Student Account, rewrite the immutable email, or delete Student Work.

## Student authorization

A Student can:

- discover only Courses with an active Student relationship;
- open only the Student's own Coursework in those Courses;
- start or resume only an allowed Assessment Attempt;
- save responses only in that Student's open Attempt; and
- read only that Student's submitted work and permitted feedback.

The server derives the Student record from the authenticated Account and exact
Course relationship. A Course reference, Student-record ID, Attempt ID, or
browser role claim never grants access.

## Instructor authorization

An active Instructor relationship authorizes the current co-Instructor to
manage Course teaching content, roster relationships, Course settings, and
authorized FERPA records. All current co-Instructors use the same predicate.

Blueprint ownership is separate. A Course Instructor does not thereby own a
Blueprint, and a Blueprint owner does not thereby gain access to a daughter
Course Instance.

## Student Work

An Assessment Attempt and its selected Question Revision, Pool ID, and Pool Edit Number evidence, saved
responses, whole-Assessment submission state, credit fractions, scores, and
feedback belong to the Student record in the Course.

Question responses save while the Attempt is open. The Student submits the
whole Assessment Attempt, which finalizes all saved responses together. Account
or Course relationship changes do not rewrite this evidence.

## Removal, deactivation, and reactivation

- Ending a Student Course relationship blocks new Student access and new
  Attempts but preserves the Student record and existing work.
- Restoring a valid relationship can restore access while retention permits it.
- Deactivating an Instructor Account blocks authentication while preserving
  Course relationships, authorship, and history; reactivation restores the
  same Account and Product Role.
- Deactivating a Student's Course access blocks that Course relationship while
  preserving the global Student Account and Student Work; it may be restored
  while retention permits it.
- FERPA record deletion follows Course retention and does not delete the global
  Account.

## Course lifetime and FERPA retention

A Course Instance represents one teaching period and becomes Inactive six
months after creation, after an Instructor warning. Assessment deadlines can
move the end of normal teaching only within that Active lifetime, preventing
Course reuse or deadline extensions from delaying FERPA retention indefinitely.
The latest Assessment deadline starts the FERPA retention clock; it does not
itself archive or remove Student data. The configured policy later determines
notice, removal from normal interfaces, recovery, and permanent deletion.
Becoming Inactive does not itself delete Student records. Course metadata,
Assessment definitions, Questions, and settings remain.

Course-specific responses, scores, timing, and even small-cell aggregates are
protected educational records. Anonymous statistics may survive only when they
cannot identify or link back to a Student.

See [RETENTION_POLICY.md](RETENTION_POLICY.md) and
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Student View and support

Instructor Student View is an answer-free preview that keeps the Instructor's
identity and creates no Student relationship, Student record, Attempt, response,
or grade.

Sysadmin support access to FERPA-protected records is deliberate, narrowly
scoped, and recorded. Platform role alone does not authorize Course data.

## Concurrency

Relationship changes and Student Work operations recheck current Account and
Course facts in the same protected transaction. If removal races with an
Attempt save or submission, one operation commits first and the other observes
the resulting authority/state; neither creates work after access has ended.

## Implementation gap rule

Current tables may distinguish membership episodes, invitation states,
enrollment rows, access receipts, or nested Question-attempt records. Preserve
them only when they enforce the current boundary or retain necessary evidence.
Do not treat their existence as authority for new product states, grade
selection, audit machinery, or response-level finalization actions.
