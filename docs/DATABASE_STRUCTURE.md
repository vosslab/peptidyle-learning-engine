# Database structure

This document maps the canonical PostgreSQL structure to the target product in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Assessment Type enum labels still use
`regular_assignment`, `practice_question_assignment`, and `bonus_assignment`
because those are the three named Assessment Types. Remaining `available`
lifecycle labels on Published Questions are a migration gap, not a second
product model.

Table shape rules (types, identity, snapshots, constraints, indexes, and the
"is my table well designed" checklist) live in [DATABASE_STYLE.md](DATABASE_STYLE.md).

## One canonical structural build

[schemas/base_schema/install.sql](../schemas/base_schema/install.sql) is the
ordered fresh-install manifest. Source is layered by kind: roles and types,
then [20_tables/](../schemas/base_schema/20_tables/), then constraints,
indexes, [50_functions/](../schemas/base_schema/50_functions/), policies, and
grants. Cross-file foreign keys live in
[30_constraints.sql](../schemas/base_schema/30_constraints.sql).

Before the first approved production deployment, structural corrections change
the owning base module and are verified from a clean database. After that
baseline is frozen, reviewed forward migrations carry later changes. The
installation coordinator owns atomic initialization, compatibility checks, and
failure rollback.

## Schema boundaries

| Schema | Responsibility |
| --- | --- |
| `ple_data` | Shared content, stable lineages, current teaching configuration, and public facts |
| `ple_private` | Account-private data, opaque backend state, and FERPA-protected Student Work |
| `ple_audit` | Only deliberately required bounded audit/support evidence |
| `ple_api` | Narrow authenticated operations and application-safe projections |

Runtime identities are not schema owners and receive no general DDL or RLS
bypass. The authenticated Account and exact stored relationship are installed
and checked within the protected transaction.

## Target product relationships

```text
Global Account (one immutable Product Role)
  +-- Course relationships
  |     +-- Course Instance
  |           +-- equal co-Instructor relationships
  |           +-- Student records
  |           +-- Course Instance Assessments
  |                 +-- Assessment Attempts
  |                       +-- selected Question Revision, Pool ID, and Pool Edit Number
  |                       +-- saved responses
  |                       +-- whole-Attempt submission state
  |                       +-- immutable credit fractions
  +-- private Draft Question workspaces
  +-- Blueprint Course owner relationship

Published Question
  +-- immutable Question Revisions

Published Question Pool
  +-- current member list with Edit Number CAS

Blueprint Course (Private, Public, or Archived)
  +-- immutable changed-content Blueprint Revisions
        +-- Blueprint Assessments
  +-- adoption relationships used to copy newly added Blueprint Assessments
      into daughter Course Instances as Unreleased Assessments
```

Published Questions and Blueprint Courses have immutable Revision families.
Question Pools are current state: members live on the Pool row's Edit Number.
Accounts, Course Instances, Assessments, and Blueprint Courses use the public
ID as primary key. Edit Numbers on current aggregates are concurrency
controls. HTTP `ETag`/`If-Match` encode those integers as quoted decimal
strings; the domain value is the Edit Number or Revision Number.

## Questions and Pools

A Published Question has one stable public `XXXX-ZXXX` ID. The exact uppercase
ASCII value, including its hyphen and embedded checksum, is stored unchanged;
there is no compact persistence form or presentation translation. Seven `X`
characters are cryptographically random and `Z` is the public unsalted
SHA-256 checksum character. Published Questions and Question Pools share this
one global namespace, while every public ID is globally unique across all
public-ID object types and is never reused.

Question source changes create immutable Question Revisions. Current metadata
changes do not. Draft Questions remain private, mutable, unpublished, and
unversioned.

Question Pools use stable public identity plus a current member list. Saves
CAS the Pool Edit Number; a no-op identical ordered list does not increment
it. Assessment pool entries pin `question_pool_id` only. Student Work
(`question_pool_selection`) stores Pool ID plus the Pool Edit Number at
issue, and selected items keep exact Question Revision pins so later Pool
edits do not silently change already-issued work.

`question_pool.created_in_transaction` is an internal `xid8` marker with
default `pg_current_xact_id()`. It replaces a timestamp-based heuristic when
a protected construction transaction must distinguish newly created Pools
from prior or concurrent committed rows. It is not a public Pool, Assessment,
or browser/API field.

Frozen Assessment policy lives in content-addressed
`ple_data.assessment_policy_snapshot`. Frozen Entry facts live in
`ple_private.assessment_entry_snapshot`. Student Work primary keys lead with
`course_instance_id`. Question Attempts reference
`ple_private.delivery_toolchain` rather than copying toolchain strings.
Saved responses finalize in place. Library usage statistics are identity-free
counts on Question Revisions and Pool members; private observation receipts
purge with Student Work.

## Blueprint Courses

Creation atomically produces a Private Blueprint and Revision 1. A meaningful
explicit content Save creates one next immutable Revision. A canonical no-op
creates none. Short name, long name, and Private/Public/Archived lifecycle are
current lineage metadata and do not create Revisions.

A Course Instance may be created empty or adopt a Public Blueprint. Adoption
copies Blueprint Assessments into current Course Instance Assessments and
retains exact Blueprint Revision provenance. Blueprints contain no Students,
dates, time zones, or relative schedules.

The reverse creation workflow does not convert or mutate its source Course
Instance. It atomically creates a distinct actor-owned Private Blueprint at
Revision 1 and records one immutable Blueprint-to-source-Course relation. That
source Course counts as the Blueprint's first Adoption and contributes its
distinct students-ever-enrolled count, but it is not a daughter Course and is
never included in automatic daughter-update operations. Course-owned Pools are
forked into fresh Blueprint-owned Pools with identical ordered exact member
pins.

The Blueprint-to-daughter relationship exposes newer Blueprint Revisions for
Instructor review and approval. Existing Assessment changes are not applied
silently, while a newly added Blueprint Assessment is copied automatically as
an Unreleased Course Instance Assessment. Fork ancestry supports discovery and
selective application of later source changes. Blueprint Course Change
Proposals target a receiving Blueprint owner and create a new receiving
Revision only for accepted changes. Their exact storage and concurrency shapes
remain implementation choices rather than additional Revision families.

Any current schema column or enum that represents `Available` instead of
Private/Public/Archived is a migration gap.

## Course Instances and Assessments

A Course Instance stores current Course identity, term settings, and Course
relationships. Every co-Instructor relationship grants equal Course authority.
There is no privileged Course-owner row in the target model.

A Course Instance Assessment stores one current teaching configuration:
Assessment Type, instructions, timing, Attempt policy, feedback policy, ordered
Questions/Pool selections, and point values. Its lifecycle is Unreleased or
Released. Closed and Archived Assessment rows are not target lifecycle states.

The generic teaching object is stored as `ple_data.assessment`. Assignment
appears only inside the three Assessment Type names.

## Student Work

An Assessment Attempt is the Student Work root. It retains the exact Student,
Course, Assessment, timing, Question Revision and Pool Edit Number selections, backend state
needed to interpret responses, saved responses, whole-Attempt submission state,
immutable credit fractions, and protected feedback.

A complete response is replaceable while the Attempt is open. Submitting the
whole Assessment Attempt finalizes all saved responses together. Positions
without a complete saved response remain visibly unanswered, contribute zero,
and count as incorrect without backend evaluation. Saved responses finalize in place on
`assessment_attempt_saved_response`; there is no separate response-copy table.

Scores are derived from immutable credit fractions and current Assessment
Question point values. The highest submitted Assessment Attempt score is used.
The target model requires no regrading, mutable result, scoring generation,
separate weighting model, or score-rebuild worker.

## Assessment Unrelease

One protected transaction verifies equal co-Instructor authority, Released
state, current precondition when used, and the exact typed Assessment title. It
changes the Assessment to Unreleased and deletes the complete Assessment-owned
Student Work graph while preserving the Assessment definition, Course
relationships, and shared Published content.

Additional audit, receipt, statistics-rebuild, or correction machinery is not
a product requirement unless independently justified.

## Retention

The Course Instance creation time anchors a six-month maximum Active lifetime.
That limit caps Assessment deadline movement so Course reuse cannot indefinitely
delay FERPA retention and deletion, but becoming Inactive does not itself delete
Student records. The latest Assessment deadline starts the FERPA retention
clock. The database must support later Instructor notice, FERPA archive,
recoverability during the configured period, and permanent deletion of
FERPA-protected Student records while preserving Course metadata, Assessments,
Questions, and settings.

Human Guidance does not specify numeric FERPA retention durations or exact
job/event/table shapes. Existing cleanup or job tables do not by themselves
satisfy or redefine this contract.

## Structure and installation data

The base manifest is structural DDL. Ordinary installation data is separate and
convergent. Live Demo data uses ordinary product records, never demo-only
lifecycles or authority shortcuts. The Genetics Question bundle remains normal
installation content even when the optional Live Demo teaching graph is
omitted.

## Verification boundary

Verify fresh atomic installation, compatible replay, forced RLS, least
privilege, equal co-Instructor relationships, Blueprint lifecycle and Saves,
whole-Assessment submission, Unrelease closure, and retention fences at the
connected PostgreSQL boundary. Catalog inspection alone does not prove the
browser product.
