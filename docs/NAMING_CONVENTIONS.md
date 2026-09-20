# Naming conventions

This file defines cross-language naming after product vocabulary has been
chosen. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) and
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) own the product terms.
Current legacy identifiers are migration evidence, not naming precedent.

## Core rule

Name an identifier for the boundary that owns it and convert it once at that
boundary. Preserve registered external names exactly. For PLE-owned names, use
the canonical product noun even when the current table, route, or component has
an old name.

Use Student, Instructor, and Sysadmin for people and Product Roles. Use Account
for the global authenticated identity. Use the exact Course relationship,
Student record, Blueprint owner, or workspace relationship for scoped
authority. Avoid generic `user`, `owner`, or `admin` when the precise PLE role
is known.

Assessment is the generic activity object. Assignment is not an object,
category, or parent Type; it appears only in Regular Assignment, Practice
Question Assignment, and Bonus Assignment. New identifiers use `assessment`,
not generic `assignment`.

## Language matrix

| Boundary | Convention | Example |
| --- | --- | --- |
| Rust modules, functions, fields, locals | `snake_case` | `assessment_attempt` |
| Rust types, traits, enum variants | `UpperCamelCase` | `AssessmentAttempt` |
| TypeScript functions, locals, signals, ordinary props | `lowerCamelCase` | `saveAssessment` |
| PLE-owned JSON identity and Tuple members | camelCase | `accountId`, `courseInstanceId`, `{questionId, revisionNumber}`, `{blueprintCourseId, revisionNumber}` |
| Other PLE-owned serialized fields | language-native at the owning boundary | SQL `assessment_type`; JSON may still use `snake_case` on non-identity Blueprint views |
| TypeScript types and components | `UpperCamelCase` | `AssessmentPropertiesEditor` |
| Python modules, functions, locals | `snake_case` | `assessment_id` |
| Python classes | `UpperCamelCase` | `ScenarioReceipt` |
| PostgreSQL identifiers | unquoted `snake_case` | `assessment_id` |
| Static URL segments and CSS classes | lowercase kebab case | `assessment-templates` |
| Constants and environment variables | `SCREAMING_SNAKE_CASE` | `MAX_ASSESSMENT_ATTEMPTS` |
| Public IDs | canonical product format | `BPXXXXXXXZ`, `XXXX-ZXXX` |

Framework, DOM, HTTP, database-provider, and registered protocol names retain
their owner's spelling.

## Identity suffixes

| Meaning | Suffix or form | Example |
| --- | --- | --- |
| Domain aggregate | Domain noun | `Account`, `CourseInstance`, `Assessment` |
| Typed internal UUID | `Uuid` / `_uuid` | `AssessmentAttemptUuid`, `assessment_attempt_uuid` |
| Public product ID | reviewed `Id` term | `CourseInstanceId` |
| Immutable Revision number | `RevisionNumber` | `QuestionRevisionNumber`, `BlueprintRevisionNumber` |
| Composite exact identity | `Tuple` | `QuestionRevisionTuple`, `BlueprintRevisionTuple`, `QuestionAssetTuple`, `CourseRosterTuple`; JSON fields `questionRevisionTuple` / `blueprintRevisionTuple` / `questionAssetTuple` / `courseRosterTuple` |
| Genuine indirect, scoped, or external locator | `Reference` | Use only when a simpler Id, Tuple, path, key, handle, or token is inaccurate |
| Current-state concurrency | `EditNumber` | `AssessmentEditNumber`, `BlueprintEditNumber`, `DraftQuestionEditNumber`; JSON `assessmentEditNumber`, `draftQuestionEditNumber`, `expectedAssessmentEditNumber` |
| Integrity value | `Checksum` | `ObjectChecksum` |
| Cardinality | `Count` | `activeInstructorCount` |
| Ordinal placement | `Position` | `authoredPosition` |
| Bounded bearer value | `Token` | `WorkerLeaseToken` |
| One-time correspondence value | `Nonce` | `PresentationNonce` |

Use `Revision` only for Published Questions and Blueprint Courses. Question
Pools, Assessments, and Course Instances are current state. An Edit Number,
event, receipt, snapshot, job generation, or current state is not a Revision.

A Count is not a version number. A Position is neither an identity nor a
version number.

## Nested identity vs resource-root `id`

`id` is permitted only as the immediate identity of a resource at its own JSON
root, typed as that resource's specific `...Id`. Nested identities in a
relationship, selection, provenance, command, route, or DTO use the precise
`...Id`, `...Tuple`, or `...Number`. Do not use shorthand such as `source`,
`target`, `course`, `assessment`, `entry`, `attempt`, `revision`, or
`currentRevision` when the value is an identity or version clock.

HTTP `ETag` and `If-Match` remain standard header spellings only. Domain and
browser values use the exact qualified Edit Number or Revision Number, such as
`assessmentEditNumber` or `BlueprintRevisionNumber`; a quoted header is only
the HTTP encoding of that number.

An exact immutable Blueprint or Question revision is one named Tuple, not
sibling ID-plus-number fields. Course Instance adoption uses
`blueprintRevisionTuple`. Course Instance provenance uses
`adoptedBlueprintRevisionTuple` and `currentBlueprintRevisionTuple`.
Assessment Blueprint Update and known forks use named Blueprint Revision
Tuples such as `expectedSourceBlueprintRevisionTuple`.

`QuestionAssetTuple` members are `{questionAssetId, checksum}`. Every field
holding that Tuple is `questionAssetTuple`. Object Address members are
`objectId`, `questionAssetId`, `workspaceId`, `workspaceImportId`,
`courseBannerId`, and `draftQuestionId`.

Student Work Recovery uses `CourseRosterTuple { courseInstanceId, rosterId }`
with typed Course, Assessment, Attempt, and Question Revision identities. It
never exposes `StudentRecordId`.

PostgreSQL tables and composite foreign keys retain role-qualified physical
pairs such as `(source_blueprint_course_id,
source_blueprint_revision_number)`. That storage shape is the intentional
physical representation of a Tuple. Application and API layers assemble the
named Tuple at the SQL-to-Rust boundary and do not persist a Tuple type in
PostgreSQL.

Complete relationship or comparison-side objects may be named `source`,
`target`, `left`, or `right`. Scalar identities inside them still use the
precise `...Id` or Tuple. Ordinary English such as code source, registered
HTTP, DOM, and vendor names, route syntax before immediate parsing, physical
SQL keys, and archival historical records keep their owner's spelling.

Use `Uuid` only when the physical value is a UUID. A public ID is the one
universal, canonical human-facing identifier for a PLE object that needs one.
Store and use an exact public ID unchanged across all boundaries; it is not a
display form or a translated version of another identifier. The public
Question/Pool ID remains `QuestionId`/`PoolId` because ID is its product name.

## Domain map

| Domain | Canonical naming |
| --- | --- |
| Account/session | `account_id` (Account ID), `session_id` UUID, immutable Product Role |
| Course relationship | `course_membership_id` UUID or precise Student/Instructor relationship name |
| Student record | `student_record_id` UUID under one Course Instance; FERPA-internal, not a recovery or public API field |
| Course Roster | `CourseRosterId` and `CourseRosterTuple { courseInstanceId, rosterId }` |
| Draft Question | `draft_question_id` UUID, optional `draft_question_edit_number` |
| Published Question | `published_question_id` / `QuestionId` plus `QuestionRevisionTuple` |
| Question Pool | `question_pool_id` / `PoolId` plus Pool member list and `question_pool_edit_number` |
| Blueprint Course | `blueprint_course_id` / `BlueprintCourseId`, `BlueprintRevisionTuple`, and current lifecycle |
| Course Instance | `course_instance_id` / `CourseInstanceId` with equal co-Instructor relationships |
| Assessment | `assessment_id` / `AssessmentId`; Blueprint Assessment or Course Instance Assessment where scope matters |
| Assessment Attempt | `assessment_attempt_id` UUID and current whole-submission state |
| Saved response | UUID of the saved-response row; never a Student submission |
| Object | `object_record_id` UUID plus typed owner/scope; Object Address members use `objectId`, `questionAssetId`, `workspaceId`, `workspaceImportId`, `courseBannerId`, and `draftQuestionId` |
| Service work | Exact operation target plus lease only when asynchronous work is required |

Current `assignment_uuid`, `assignment_attempt_uuid`, `QuestionAttempt`, or
`QuestionResponse` identifiers are implementation names. When documentation
must cite them, state the canonical Assessment or saved-response meaning nearby.

## Blueprint and Course names

Use `BlueprintCourse` for reusable content and `CourseInstance` for delivered
teaching. Blueprint lifecycle values are `Private`, `Public`, and `Archived`.
Do not name a Blueprint state Draft, Available, or Published.

A Blueprint contains no dates or relative schedules. A Course Instance may
adopt a Public Blueprint or start empty. Every current co-Instructor is equal;
do not add Course Owner or primary-Instructor identifiers.

Use `BlueprintAssessment` and `CourseInstanceAssessment`. A newly added
Blueprint Assessment is copied to daughter Course Instances as Unreleased.
Use Blueprint Update for the Instructor-reviewed daughter workflow and
Blueprint Course Change Proposal for a proposal to another Blueprint owner.
Do not describe changes to existing Assessments as silently applied.

## Assessment UI names

- `AssessmentQuestionEditor` for composition.
- `AssessmentPropertiesEditor` for settings.
- `AssessmentsDueSoon` and `MyAssessmentTemplates` for Instructor tasks.
- `Coursework` for the Student collection.
- `SubmitAssessment` for whole-Attempt submission.
- `BackToCoursework` for the Student return action.

Assessment Type identifiers correspond to the five fixed Types. Do not permit
Instructor-created Type names.

## PostgreSQL

- Use unquoted lowercase `snake_case`.
- Name timestamps for events or transitions with `_at`.
- Name serialized documents with `_payload` and SHA-256 companions with
  `_payload_sha256`.
- Name foreign keys for the exact parent.
- Do not preserve a legacy table name by adding a compatibility alias unless a
  separately approved migration requires it.

## Browser and TypeScript

Use `lowerCamelCase` for browser-owned runtime state and `UpperCamelCase` for
types/components. PLE data-object fields follow their Rust Serde names. Static
URL segments and CSS classes use lowercase kebab case.

Browser names never claim authority: `selectedCourseRef`, for example, is a
selector until the server resolves the authenticated relationship.

## Change rule

A product rename changes the owning model, generated contracts, direct
consumers, routes/schema where in scope, focused tests, and docs together in a
dedicated implementation task. A docs-only pass records existing code names as
gaps and does not invent compatibility layers.
