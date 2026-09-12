# Student Work Records

This document describes the current Assignment and Student Work model. The
canonical terms and boundaries are in
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md); the human product authority
is [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Database ownership and privilege
boundaries are described in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

PLE separates current teaching configuration from immutable Student Work.
An Assignment is one current Course Instance-owned aggregate. An Assignment
Attempt is one retained occurrence of that Assignment for one Student Record.
Only Question Revision and Blueprint Revision are Revision concepts.

## Assignment current state

An Assignment has a stable identity within its Course Instance and one current
set of teaching settings:

- title and instructions;
- availability, due, and close timestamps;
- whole-Attempt limits and late-work rule;
- completion, grade, continuation, reuse, variation, resume, display,
  navigation, and ordering policies;
- feedback-release settings; and
- Assignment Status.

`Assignment Edit Number` is the qualified optimistic-concurrency value for the
current aggregate. An accepted meaningful save advances it once. An unchanged
save keeps it. A caller supplies the expected value for a competing save,
release, or Unrelease operation.

`Assignment Status` is current lifecycle state: `Unreleased`, `Released`,
`Closed`, or `Archived`. Release is a validated state transition. A Released
Assignment remains editable when its resulting current configuration passes
release validation. Accepted Released Assignment edits govern future Attempts;
they do not reinterpret an existing Attempt.

An Assignment created from reusable content retains `BlueprintAssignmentSource`:
the exact Blueprint Revision Reference and the stable Blueprint Assignment
Reference from which it was copied. That provenance explains creation and does
not create another Assignment Revision or restrict later current edits.

## Assignment composition

Current composition uses ordered Assignment Entries. Each entry is either:

- a Fixed Question with one exact Question Revision pin; or
- a Question Pool with a selection count, per-item points, selection-order
  policy, and eligible Question Pool Items.

Every Question Pool Item also pins one exact Question Revision. A newer
Question Revision never advances an Assignment Entry or pool item implicitly.
Current entry and pool configuration is the source for a future Attempt. Its
availability supports ordinary current authoring without changing evidence
already issued to Students.

## Student Work hierarchy

| Record | Owns or retains |
| --- | --- |
| Student Record | One Student Account's educational record in one Course Instance |
| Assignment Attempt | One effective occurrence of one Assignment for that Student Record |
| Question Pool Selection | The exact selected Pool Items for one Attempt when a pool is used |
| Issued Question | One position, Assignment Entry, exact Question Revision, seed, and per-question evidence |
| Question Attempt | One delivered attempt at an Issued Question |
| Saved Response / Question Submission | The Student's retained response state and accepted submission |
| Grading evidence | Forward-only grading state, result, receipt, and statistics observation evidence |

The Course Instance and Assignment establish scope. Every child is constrained
to the same Assignment Attempt, Student Record, Course Instance, and exact
published Question Revision relationship. PostgreSQL maintains these ownership
relationships and row-level authorization; browser identifiers do not establish
authority.

## Retained Attempt evidence

Starting an Attempt copies the effective Assignment facts required to interpret
that occurrence. This includes its title and instructions; availability, due,
and close timestamps; whole-Attempt time limit; late-work, completion,
feedback-release, reuse, variation, ordering, navigation, display, and resume
policies; and effective student-specific values with their accommodation source
and Edit Number when an accommodation changed them.

This is evidence, not an Assignment snapshot family. Current Assignment state
continues to decide eligibility for a future Attempt. Retained Attempt facts
decide how an existing Attempt, its timing, disclosure, scoring, and history
are interpreted after a later current Assignment edit.

An Issued Question retains the facts specific to one issued position:

- its Assignment Entry identity and issue position;
- its exact Question Revision;
- its Question Seed;
- its point value, scoring rule, and statistics eligibility;
- its pool-selection source when applicable; and
- its presentation and reproduction binding.

The presentation binding retains the source and ready asset rendition selected
for delivery. Resume and replay read that retained binding so a later asset or
current Assignment change cannot alter an already-issued question.

Question Pool Selection retains the exact selected items. The issued questions
for that selection must match its retained item set. Reuse and variation policy
therefore remain interpretable without consulting later current composition.

## Delivery and grading evidence

`Question Attempt` records the delivery occurrence for an Issued Question,
including server timing and its operational state. A saved response remains
associated with that Question Attempt. An accepted Question Submission is
immutable Student evidence. Forward-only grading records connect the accepted
submission to its job, grading result, receipt, and eligible anonymous
statistics observations.

Completion and grade selection use retained Attempt policy and Issued Question
facts. Assignment Progress and the Gradebook are derived views over this
evidence; they do not replace it. Authorized readers receive only the fields
permitted by the retained feedback-release policy and current authorization.

The trusted server, Store, and PostgreSQL issue timestamps, reference numbers,
seeds, selection results, and grading transitions. The browser supplies
responses and current-state preconditions, never authoritative Student Work
facts or teaching authority.

## Authorization and concurrency

A Student works only through the exact active Student Course Membership and
Student Record. A Teaching Team Member acts only through a current Instructor
Course Membership. Each protected operation verifies the relationship at the
trusted boundary and applies the same scope inside PostgreSQL.

Attempt start, response save, submission, grading commit, and Unrelease take
the Assignment root lock before changing Attempt-rooted Student Work. The lock
order gives exactly one operation the next state: an in-flight worker either
commits before Unrelease or finds no remaining target after it.

Student Work evidence is immutable after its accepted transition. Private
tables deny ordinary update and delete paths; narrowly owned database routines
perform the allowed forward transitions.

## Assignment Unrelease

Assignment Unrelease is the high-consequence Instructor operation that returns
a Released Assignment to `Unreleased` current state. The Instructor confirms
the exact title and supplies the current Assignment Edit Number. PostgreSQL
verifies current Teaching Team authority and Released status while holding the
Assignment root lock.

One transaction then:

1. counts affected Assignment Attempts, Question Submissions, Assignment
   Submissions, and Grading Results for the result and audit evidence;
2. changes Assignment Status to `Unreleased` and advances the Assignment Edit
   Number;
3. deletes the Assignment Attempt roots; their owned Student Work closure
   follows constrained cascades;
4. rebuilds statistics for the affected exact Question Revisions from surviving
   observation receipts; and
5. records one immutable, redacted audit event with the actor, Assignment,
   aggregate counts, outcome, and time.

The closure includes Attempt-rooted issued questions, pool selections, question
attempts, saved responses, submissions, delivery and presentation bindings,
grading jobs and results, receipts, correction links, and statistics
observations. It preserves the Assignment's current entries, shared Question
Revisions, shared assets, Course relationships, and the redacted Unrelease
event. The dedicated no-login database executor owns the guarded destructive
routine; application and worker roles do not receive general deletion
authority.

An Unrelease precondition failure leaves Assignment Status, Assignment Edit
Number, Student Work, statistics, and audit evidence unchanged.

## Related boundaries

- [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) maps the end-to-end
  assessment path.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines the
  answer-safe Student transport boundary.
- [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) explains the
  teaching rationale for configurable completion and practice.
- [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) describes the implemented
  PostgreSQL catalog and installation lifecycle.
