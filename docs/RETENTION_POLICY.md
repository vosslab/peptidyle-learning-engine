# Student-record retention policy

Peptidyle distinguishes reusable teaching content from the records owned by a
Course Instance. Published Questions, their immutable Question Revisions,
Question Sources, Question Library records, Blueprint Courses, private
authoring workspaces, and Instructor drafts are not Course-owned Student
records. A future Course lifecycle action therefore cannot use a Course as
authority to delete shared teaching content.

Course work, Assignment Attempts, submissions, grades, and their exact
interpretive evidence belong to the Course lifecycle independently of the
Student Account's lifetime. Assignment definitions remain current mutable
configuration; an Attempt and its Issued Questions retain the facts required
to interpret that Student Work directly. Retained Attempt and Issued Question
evidence preserves that interpretation through later configuration changes.

## Current boundary

The base schema contains no Course-retention configuration, plan, revision,
event, job target, receipt, API, Store operation, worker, or browser route.
It consequently does not claim to archive, strip, delete, or extend the
lifecycle of a Course's Student records. The notice, archive, and deletion
timing in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remains product policy to be
implemented by a complete future capability, rather than a hidden database
default.

Draft Question cleanup is a separate authoring concern. A configured
Authoring Workspace cleanup may remove expired draft rows, editable metadata,
and draft source objects after its recovery period. Publication has already
copied the accepted content and metadata into immutable Published Question
storage, so this cleanup does not make a Published Question incomplete.

## Technical object cleanup

Generic object-storage cleanup is implemented independently of Course
retention. An exact storage check identifies one immutable storage anchor;
`ple_private.object_cleanup_manifest` records the permitted disposition for
that checked object; and `ple_audit.object_cleanup_receipt` records the
result. The model supports the owners of course media and profile media. It
does not make a bucket prefix, object listing, Course, or browser request
authority to delete data.

Object cleanup remains technical work: its manifest and receipt say what an
authorized object owner may remove and what happened. They do not express a
Student-record policy or stand in for a Course-wide retention outcome.

## Future Course retention capability

[TODO.md](TODO.md) tracks Active and Inactive Courses as a future vertical
capability. When approved, that work must define the product lifecycle,
authorized actors, exact Student-record scope, durable outcome evidence,
Store and PostgreSQL transaction, worker behavior where needed, API and
browser contract, and connected acceptance together. Its design must preserve
shared content and the retained evidence needed to interpret surviving
Student Work.

Backup, deployment, and operational-log retention are infrastructure policies.
They are separate from product-record lifecycle and must not become an
undeclared Student-record archive.

See [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md#structure-and-installation-data),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) for the current boundaries.
