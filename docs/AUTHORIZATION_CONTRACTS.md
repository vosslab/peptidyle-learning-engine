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

A Student may act only through that Student's current Course Membership and
Student Record for the exact course. Student-facing assignment delivery,
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

Course term dates are current Course Instance state. An Assignment is one
current aggregate protected by its Assignment Edit Number. Its status,
authored policy, normalized entries, and exact Question Revision pins are
re-evaluated for a new Attempt. An accepted edit to a Released Assignment
affects later Attempts after release validation; it does not reinterpret an
existing Attempt.

An Attempt retains the effective title, instructions, availability, timing,
policy, and qualified adjustment source needed to interpret its own work. An
Issued Question retains its Assignment Entry identity and position, exact
Question Revision, seed and presentation/reproduction binding, point value,
scoring rule, statistics eligibility, and pool-selection source. Readers use
that retained evidence for existing Student Work.

Student Work is immutable to ordinary runtime capabilities. Its root is an
Assignment Attempt and its dependent issued questions, responses,
presentations, submissions, grading evidence, backend exchanges, pool
selection, and statistics observations are database-owned records. Shared
Question Revisions and assets, current Assignment configuration, and Course
Membership are not Student Work owned by an Attempt.

## Assignment lifecycle operations

Release requires current course-Instructor authority, a matching Assignment
Edit Number, and a release-valid current Assignment. Unrelease has the same
course predicate plus Released status, the exact Edit Number, and exact current
title confirmation. The database locks the Assignment first, changes it to
Unreleased, deletes only the rooted Student Work closure, rebuilds affected
Question Revision statistics, and writes one redacted audit event in the same
transaction. The event records actor, Assignment, aggregate deletion counts,
outcome, and time; it does not contain Student identities, responses, or
grades.

The dedicated no-login `ple_unrelease_executor` capability performs that
guarded deletion procedure. API, worker, and ordinary application capabilities
do not inherit it. A failed precondition changes neither Assignment state,
Student Work, statistics, nor audit state.

## Published content and Blueprint Courses

Question Revision and Blueprint Revision are the only product Revision
concepts. Each is immutable. A Question or Blueprint Course lineage carries
current `available` or `archived` state and an Availability Edit Number, with
append-only availability events. Archive removes content from ordinary
browsing and new selection; it preserves resolution of exact historical
revision references. Restore is the corresponding current-state transition.

New Assignments and Blueprint Draft pins select exact available Question
Revisions. A later Question publication, availability transition, correction,
or worker action never advances an Assignment or retained Student Work pin.

An active Instructor owns one private Blueprint Draft for a Blueprint Course.
Blueprint Draft saves require its Edit Number and increment it only when the
content changes. An explicit publication copies the complete Draft into a new
immutable Blueprint Revision and retains an accepted-request receipt for safe
replay. A Blueprint Draft is single-owner state; this reset does not define a
Blueprint collaborator relationship.

Question authoring workspace relationships remain their own authoring
capability and do not widen course, Assignment, Blueprint Draft, or Student
Work authority.

## Workers, objects, and service identities

PLE Accounts are not PostgreSQL roles. API, publisher, grader, renderer,
worker, database-owner, and cloud-task identities are service identities with
only their direct capability grants. A worker acts through its typed claim,
lease, and commit path; a queue payload is an input to validate, not an
authority token. Worker capabilities do not inherit application or Unrelease
authority.

Object delivery uses the current typed relationship and the database's object
binding. Browser-facing data names logical resources rather than storage
locations or service credentials. Course-record and workspace objects retain
their relationship fence; answer keys, private grading input, raw responses,
and signed object addresses are not browser authorization artifacts.

## Audit and durable checks

Audit records identify the actor, bounded target, result, and time required by
their operation while excluding credentials, raw Student responses, grades
where an audit reference suffices, answer keys, and signed URLs. An audit row
records an action; it grants no authority.

Permanent tests protect stable authorization and evidence boundaries:
membership and Student ownership, non-enumeration, forced RLS and closed
grants, restricted service capabilities, exact revision provenance, and the
Unrelease closure. Fresh-installation, real service, object-provider, and
worker-race exercises are integration evidence rather than catalog-count
requirements. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).
