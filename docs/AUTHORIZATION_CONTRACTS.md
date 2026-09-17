# Authorization contracts

PLE has one installation with global Accounts. This document defines the
authorization predicates implemented by the canonical base schema and its
Store/API boundary. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product
authority; [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the
terms used here; [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md)
describes the PostgreSQL roles and grants that enforce this contract.

Authorization is separate from authentication, identifier syntax, request
validation, edit-number conflicts, and lifecycle validation. A caller's URL,
DTO, queue message, or claimed product role never establishes authority.

## Request and database boundary

The server resolves an opaque authenticated session to an Account before it
opens a protected operation. It sets that trusted Account ID only for the
transaction. The base's `ple_api` authorization functions then derive active
role and exact relationship from durable records. Forced RLS, narrow grants,
and fixed-search-path privileged functions protect the same boundary in
PostgreSQL.

The normal protected-operation sequence is:

```text
resolve session -> derive current durable scope -> validate command and state
-> commit the authorized change
```

An absent, foreign, inactive, or unauthorized protected target uses the same
concealed result where revealing its existence would disclose student or
course data. After a caller has resolved a target they may receive the normal
conflict or validation result: an obsolete Edit Number is a conflict, invalid
resulting content or an incorrect confirmation is validation failure, and a
wrong lifecycle state is a lifecycle conflict.

## Product roles and course scope

Each Account has one immutable Product Role: Student, Instructor, or
Sysadmin. A role is active only while its current Account State is active. A
person requiring more than one role uses separate Accounts.

An active Instructor has global authoring and Question Library capability. A
course operation additionally requires that Account's current Instructor
Course Membership for the exact Course Instance. Course creation establishes
the initial Instructor Membership, and every current Instructor member has the
same teaching authority; no creator-owned authority is retained.

An active Sysadmin may read the answer-free Question and Pool Library support
projections, including exact Revision Bloom Classification metadata. That
read-only support access does not confer Instructor authoring or Library
mutation authority.

An active vetted Instructor with current exact Library read access may correct
the complete Bloom pair on that exact Question or Pool Revision. This is Library
authority, not author, founder, or Pool-owner authority. A Sysadmin has no
Bloom-correction authority; the read projection does not widen into either
correction route.

A Student may act only through that Student's current Course Membership and
Student Record for the exact Course. Student-facing Assessment delivery,
attempt, response, submission, history, and presentation operations recheck
that relationship in the transaction. A revoked membership or inactive
Account therefore stops subsequent access. An Instructor can inspect the
course's authorized teaching projections but does not become a Student; one
Student has no authority over another Student's work.

Sysadmin is a platform Product Role, not ambient teaching or FERPA authority.
Course-instance bootstrap and support operations use their separately bounded,
audited predicates. They do not create a Sysadmin Course Membership or general
access to Student Work.

## Mutable configuration and retained evidence

Course term dates are current Course Instance state. An Assessment is one
current aggregate protected by its Assessment Edit Number. Its status,
authored policy, normalized entries, and exact Question Revision pins are
re-evaluated for a new Attempt. An accepted edit to a Released Assessment
affects later Attempts after release validation; it does not reinterpret an
existing Attempt.

An Attempt retains the effective title, instructions, availability, timing,
policy, and qualified adjustment source needed to interpret its own work. An
Issued Question retains its Assessment position, exact Question Revision,
Question Pool Revision and selection when applicable, point value, and the
backend-owned state needed to resume or interpret the interaction. Readers use
that retained evidence for existing Student Work.

Student Work is rooted in an Assessment Attempt. Complete Question responses
remain editable while the Attempt is open. Whole-Assessment submission makes
the finalized saved responses and immutable backend credit fractions durable
grading evidence. Shared Question Revisions and assets, current Assessment
configuration, and Course Membership are not Student Work owned by an Attempt.

## Assessment lifecycle operations

Release requires current course-Instructor authority, a matching Assessment
Edit Number, and a release-valid current Assessment. Unrelease has the same
course predicate plus Released status, the exact Edit Number when the
implementation uses one, and exact current title confirmation. The database
locks the Assessment first, changes it to Unreleased, and deletes only the
rooted Student Work closure in the same transaction. Shared content, the
Assessment definition, and Course relationships remain. Human Guidance does
not require statistics-rebuild or generic audit machinery for this action.

The dedicated no-login `ple_unrelease_executor` capability performs that
guarded deletion procedure. API, worker, and ordinary application capabilities
do not inherit it. A failed precondition changes neither Assessment state nor
Student Work.

## Published Questions, Question Pools, and Blueprint Courses

Question Revisions, Pool Revisions, and Blueprint Revisions are immutable.
A Published Question may be discoverable or archived.
A Blueprint Course is Private, Public, or Archived. Private is owner-only and
cannot be adopted. Public is shared and adoptable. Archived remains visible to
vetted Instructors through explicit historical discovery, can be forked, and
cannot be adopted. Exact historical Revision References remain resolvable.

New Assessments and Blueprint Revision pins select exact available Question
Revisions. A later Question publication, availability transition, correction,
or worker action never advances an Assessment or retained Student Work pin.

An active Instructor may own Private Blueprint Courses and their content Save and
lifecycle operations. Complete valid creation atomically produces a Private
Blueprint Course with Revision 1.
Save requires the exact current Revision and creates its immutable successor
only for changed canonical content; an unchanged Save returns the current
Revision with `changed: false`. Unsaved browser state is not a server domain object, and this reset
does not define a Blueprint collaborator relationship.

Only the owner changes Blueprint lifecycle state. A Public Blueprint with no
adoptions may return to Private; a Public Blueprint with an adoption remains
Public. The owner may archive a Public Blueprint and restore their Archived
Blueprint to Public. Every vetted Instructor may read Public and Archived
content, but only Public content may be adopted. Every vetted Instructor may
fork Public or Archived content into a new Private Blueprint they own.

Adding a new Blueprint Assessment also triggers the Human-Guidance-required
copy into daughter Course Instances as an Unreleased Assessment. That bounded
system action does not grant the Blueprint owner Course Membership or access to
the daughter Courses. Each daughter Course's current co-Instructors decide
whether to accept offered changes to existing Assessments.

Any vetted Instructor may create a Blueprint Course Change Proposal. Only the
receiving Blueprint owner chooses accepted changes and creates the resulting
Blueprint Revision. Proposal authority never grants Course access or directly
changes daughter Course Instances.

Vetted Instructors may Star or Watch Public and Archived Blueprints. Star
counts and the identities of vetted Instructors who Starred are visible to
vetted Instructors. Watch state and subscription membership are private to the
watcher.

Question authoring workspace relationships remain their own authoring
capability and do not widen course, Assessment, Blueprint Course, or Student
Work authority.

## Workers, objects, and service identities

PLE Accounts are not PostgreSQL roles. API, publisher, renderer,
retention-process, database-owner, and cloud-task identities are technical
service identities with only their direct capability grants. A background
process acts only through the exact operation Human Guidance or an approved
product design requires. A queue payload is input to validate, not authority.
No service identity creates a Student or Instructor grading workflow.

Object delivery uses the current typed relationship and the database's object
binding. Browser-facing data names logical resources rather than storage
locations or service credentials. Course-record and workspace objects retain
their relationship fence; answer keys, private grading input, raw responses,
and signed object addresses are not browser authorization artifacts.

## Audit and durable checks

Where Human Guidance requires an audit, including scoped Sysadmin support and
Forced Question Correction, the record identifies the actor, bounded target,
result, and time while excluding credentials, raw Student responses, grades
where a reference suffices, answer keys, and signed URLs. An audit record
documents an action; it grants no authority and is not a generic product
requirement for every operation.

Permanent tests protect stable authorization and evidence boundaries:
membership and Student ownership, non-enumeration, forced RLS and closed
grants, restricted service capabilities, exact revision provenance, and the
Unrelease closure. Fresh-installation, real service, object-provider, and
worker-race exercises are integration evidence rather than catalog-count
requirements. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).
