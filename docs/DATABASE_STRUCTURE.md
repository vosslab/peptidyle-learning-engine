# Database structure

This document maps the canonical PostgreSQL structure to the target product in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). The checked-in schema still contains
some legacy `assignment`, `available`, response-finalization, and worker
identifiers. Those names describe implementation gaps; they are not alternate
product decisions.

## One canonical structural build

[schemas/base_schema/install.sql](../schemas/base_schema/install.sql) is the
ordered fresh-install manifest. Each included module owns its current tables,
constraints, indexes, functions, row-level-security policies, and grants. A
cross-domain relationship belongs in the cross-domain module rather than a
late corrective layer.

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
  |                       +-- selected Question/Pool Revision evidence
  |                       +-- saved responses
  |                       +-- whole-Attempt submission state
  |                       +-- immutable credit fractions
  +-- private Draft Question workspaces
  +-- Blueprint Course owner relationship

Published Question
  +-- immutable Question Revisions

Published Question Pool
  +-- immutable Pool Revisions

Blueprint Course (Private, Public, or Archived)
  +-- immutable changed-content Blueprint Revisions
        +-- Blueprint Assessments
  +-- adoption relationships used to copy newly added Blueprint Assessments
      into daughter Course Instances as Unreleased Assessments
```

Published Questions, published Question Pools, and Blueprint Courses have
immutable Revision families. Edit Numbers on other current aggregates are
concurrency controls. Human Guidance's general history summary omits Pools,
but its Pool rules explicitly require immutable Pool Revisions.

## Questions and Pools

A Published Question has one stable public ID. Storage uses the eight-character
compact Crockford Base32 form; presentation uses `AAAA-ZBBB`. Seven characters
are random identity and the first character after the hyphen is the
HMAC-derived check character.

Question source changes create immutable Question Revisions. Current metadata
changes do not. Draft Questions remain private, mutable, unpublished, and
unversioned.

Question Pools use stable public identity plus immutable Pool Revisions.
Assessment and Attempt records retain exact Question/Pool Revision evidence so
later publication does not silently change Student Work.

## Blueprint Courses

Creation atomically produces a Private Blueprint and Revision 1. A meaningful
explicit content Save creates one next immutable Revision. A canonical no-op
creates none. Short name, long name, and Private/Public/Archived lifecycle are
current lineage metadata and do not create Revisions.

A Course Instance may be created empty or adopt a Public Blueprint. Adoption
copies Blueprint Assessments into current Course Instance Assessments and
retains exact Blueprint Revision provenance. Blueprints contain no Students,
dates, time zones, or relative schedules.

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

Current schema objects named `assignment*` implement parts of this aggregate
but do not preserve Assignment as the generic product term. They require a
separate code/schema migration.

## Student Work

An Assessment Attempt is the Student Work root. It retains the exact Student,
Course, Assessment, timing, Question or Pool Revision selections, backend state
needed to interpret responses, saved responses, whole-Attempt submission state,
immutable credit fractions, and protected feedback.

A complete response is replaceable while the Attempt is open. Submitting the
whole Assessment finalizes all saved responses together. An internal row named
`question_submission` may currently store that evidence, but its name does not
define another Student action or lifecycle.

Scores are derived from immutable credit fractions and current Assessment
Question point values. The target model requires no regrading, mutable result,
scoring generation, or score-rebuild worker.

## Assessment Unrelease

One protected transaction verifies equal co-Instructor authority, Released
state, current precondition when used, and the exact typed Assessment title. It
changes the Assessment to Unreleased and deletes the complete Assessment-owned
Student Work graph while preserving the Assessment definition, Course
relationships, and shared Published content.

Additional audit, receipt, statistics-rebuild, or correction machinery is not
a product requirement unless independently justified.

## Retention

The final Assessment deadline starts the Course retention clock; later Student
activity resets it. The database must support Instructor notice, archive from
normal interfaces, recoverability during the configured period, permanent
deletion of FERPA-protected Student records, and Course inactivity while
preserving Course metadata, Assessments, Questions, and settings.

Human Guidance does not specify numeric durations or exact job/event/table
shapes. Existing cleanup or job tables do not by themselves satisfy or redefine
this contract.

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
