# Student-record retention policy

Peptidyle separates reusable Questions and Blueprint Courses from the records
owned by one Course Instance. Course Retention applies to one exact Course
Instance. It never deletes a shared Published Question, Question Revision,
Question Source, Question Library record, Blueprint Course, Instructor draft,
or private Authoring Workspace.

Student Work Records and Grades follow a Course Retention Plan independently
of the Student Account. The Course-owned Assignment definition and every
released Assignment Revision required to interpret retained Student work also
remain. A future cleanup operation may reclaim only a superseded current
Assignment Object after an exact reference check has placed it in an Object
Cleanup Manifest. It has no broad Assignment-content deletion permission.

## Current foundation

The current database baseline is preparation, not a mounted Course Retention
service:

- `ple_private.course_retention_plan_revision` records an immutable Course
  Retention Plan Revision with its exact Course, positive revision number,
  Course Retention Action, scheduled time, manifest checksum, and creation
  time.
- `ple_private.job` binds a typed Course Retention Job to that exact Plan
  Revision.
- `ple_audit.course_retention_event` binds one immutable Course Retention
  Event to the same Plan Revision, Job result, action, checksum, and time.
- `ple_private.object_cleanup_manifest` and
  `ple_audit.object_cleanup_receipt` supply the separate technical Object
  Cleanup Manifest and Object Cleanup Receipt foundation. The receipt's
  `permitted_disposition` is a technical cleanup result, not a Course
  Retention action or a browser choice.

There is currently no Course Retention Store, PostgreSQL procedure, server
route, worker, browser reader, Course Retention State record, Course Retention
Notice record, Assignment Revision Retention Rule, Course Retention Receipt,
or frozen Course Retention manifest-membership relation. The browser therefore
offers no Course Retention panel, request, or route.

## Required complete boundary

A future Course Retention capability must introduce its durable relationships
as one complete Store-backed package:

- Course Retention State is separate from Job State.
- Each Course Retention Plan Revision records the exact action, scheduled time,
  prior Plan Revision Reference, and Assignment Revision Retention Rule.
- A Course Retention Notice records an archive, purge, or extension intent and
  its creation time; it does not assert delivery.
- The Assignment Revision Retention Rule binds exact Course-owned Assignment
  Revisions. It preserves released revisions and issued facts.
- The Store derives frozen Object Cleanup Manifest membership only after exact
  reference checks. An Object Cleanup Receipt retains its technical outcome.
- A Course Retention Job leases and commits the accepted Plan Revision. A
  Course Retention Event and Course Retention Receipt exist only after commit.

The complete implementation must add exact authorization, Store, PostgreSQL,
route, worker, generated-contract, browser-reader, and connected acceptance
evidence together. A browser request cannot supply an object, cleanup outcome,
Assignment Revision Retention Rule, or Course Retention Action authority.

## Related boundaries

Object cleanup is exact-object work; a bucket prefix or listing never
authorizes deletion. Object Delivery, object checks, cleanup manifests, and
cleanup receipts remain separate from Course Retention policy. Backup,
deployment, and operational-log retention are separate infrastructure policies
and cannot become undeclared Student-record archives.

See [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md#object-grading-and-retention-boundaries),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), and
[CONTRACTS.md](CONTRACTS.md#api-and-service-contracts) for the current
implementation boundary.
