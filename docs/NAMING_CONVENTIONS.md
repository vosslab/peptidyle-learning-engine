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
| PLE-owned serialized fields | Serde-owned `snake_case` | `assessment_type` |
| TypeScript types and components | `UpperCamelCase` | `AssessmentPropertiesEditor` |
| Python modules, functions, locals | `snake_case` | `assessment_reference` |
| Python classes | `UpperCamelCase` | `ScenarioReceipt` |
| PostgreSQL identifiers | unquoted `snake_case` | `assessment_uuid` |
| Static URL segments and CSS classes | lowercase kebab case | `assessment-templates` |
| Constants and environment variables | `SCREAMING_SNAKE_CASE` | `MAX_ASSESSMENT_ATTEMPTS` |
| Public References | reviewed product format | `C-11`, `AAAA-ZBBB` |

Framework, DOM, HTTP, database-provider, and registered protocol names retain
their owner's spelling.

## Identity suffixes

| Meaning | Suffix or form | Example |
| --- | --- | --- |
| Domain aggregate | Domain noun | `Account`, `CourseInstance`, `Assessment` |
| Typed internal UUID | `Uuid` / `_uuid` | `AssessmentAttemptUuid`, `assessment_attempt_uuid` |
| Human locator | reviewed `Reference` term | `CourseInstanceReference` |
| Public product ID | reviewed `Id` term | `QuestionId` |
| Immutable Revision number | `RevisionNumber` | `QuestionRevisionNumber` |
| Immutable Revision reference | `RevisionReference` | `BlueprintRevisionReference` |
| Current-state concurrency | `EditNumber` | `AssessmentEditNumber` |
| Integrity value | `Checksum` | `ObjectChecksum` |
| Bounded bearer value | `Token` | `WorkerLeaseToken` |
| One-time correspondence value | `Nonce` | `PresentationNonce` |

Use `Revision` only for Published Questions, published Question Pools, and
Blueprint Courses. An Edit
Number, event, receipt, snapshot, job generation, or current state is not a
Revision.

Use `Uuid` only when the physical value is a UUID. A public Reference is a
separate human locator and never authorization. The public Question/Pool ID
remains `QuestionId`/`PoolId` because ID is its product name.

## Domain map

| Domain | Canonical naming |
| --- | --- |
| Account/session | `account_uuid`, `session_uuid`, immutable Product Role |
| Course relationship | `course_relationship_uuid` or precise Student/Instructor relationship name |
| Student record | `student_record_uuid` under one Course Instance |
| Draft Question | `draft_question_uuid`, optional `draft_question_edit_number` |
| Published Question | `question_id` plus `QuestionRevisionReference` |
| Question Pool | `pool_id` plus `PoolRevisionReference` |
| Blueprint Course | `blueprint_course_uuid` plus `BlueprintRevisionReference` and current lifecycle |
| Course Instance | `course_instance_uuid` with equal co-Instructor relationships |
| Assessment | `assessment_uuid`; Blueprint Assessment or Course Instance Assessment where scope matters |
| Assessment Attempt | `assessment_attempt_uuid` and current whole-submission state |
| Saved response | `saved_response_uuid` or exact implementation evidence name; never a Student submission |
| Object | `object_record_uuid` plus typed owner/scope |
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
