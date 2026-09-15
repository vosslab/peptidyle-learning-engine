# Design decisions

This file records the durable rationale that supports
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Human Guidance is the authority for
current product intent. Code, schemas, screenshots, tests, changelogs, and old
plans are evidence about implementation or history; they do not override it.

## How to use this document

- Use the decision and consequence together.
- Follow linked owner documents for implementation detail.
- Treat a current source identifier that uses obsolete vocabulary as a migration
  gap, not as a competing product decision.
- Do not infer a feature from a generic table, enum, worker, capability, or
  mockup.
- If Human Guidance does not resolve a choice, record it as unresolved instead
  of expanding this file by inference.

## Product model

### Assessment is the generic activity object

**Decision.** PLE calls the generic object an Assessment. Assignment appears
only inside the Assessment Type names Regular Assignment, Practice Question
Assignment, and Bonus Assignment. Quiz and Exam complete the five current
Types.

**Why.** A single generic noun keeps course content, attempts, navigation, and
data relationships understandable while Types communicate teaching purpose.

**Consequence.** Current `assignment` code, schema, route, and DTO names are
implementation gaps. They must not cause new documentation or UI to reintroduce
Assignment as the generic object.

### Reusable and delivered course content are different objects

**Decision.** A Blueprint Course contains Blueprint Assessments and no Students
or dates. A Course Instance contains Course Instance Assessments and Student
relationships. An Instructor-owned Assessment Template is reusable Assessment
settings outside either Course kind and contains no Questions or Pools.

**Why.** Reuse, teaching delivery, and personal templates have different
ownership and privacy boundaries.

**Consequence.** Adoption copies Blueprint content into the Course Instance and
retains exact Blueprint Revision provenance. New Blueprint Revisions are
offered to daughter Courses for Instructor review; existing Assessment changes
are never applied silently. A newly added Blueprint Assessment is automatically
copied as an Unreleased Course Instance Assessment.

### Current state is not a hidden revision family

**Decision.** Published Questions, published Question Pools, and Blueprint
Courses have immutable Revision families. Human Guidance's general history
summary names Questions and Blueprints while its Pool section explicitly
requires Pool Revisions; this document preserves the explicit Pool rule. Draft
Questions, Course Instances, Assessments, Attempts, Student Work, names, and
lifecycle metadata use current state. Edit Numbers are concurrency controls,
not historical content.

**Why.** History is valuable only where the product needs exact reusable or
submitted evidence. Universal snapshots create cost and a misleading object
model.

**Consequence.** New revision, snapshot, receipt, event, or replay types require
a specific Human Guidance-compatible need. Generic auditability is not enough.

### Course Instances keep their own identity and may become new Blueprints

**Decision.** A Course Instance has deliberately entered short and long names,
not names derived from a parent Blueprint. It starts empty or from a Public
Blueprint, always has at least one assigned Instructor, and may be deliberately
published as a new Blueprint Course.

**Why.** A teaching Course needs its own identity and delivery context, while a
Blueprint contains reusable Course structure without Student records or dates.

**Consequence.** Publishing reusable Course structure creates a new Blueprint
lineage rather than converting the teaching Course or exposing its Students,
dates, Student Work, or other delivery state.

## Accounts, roles, and authorization

### Product roles are global and exclusive

**Decision.** Each global Account has exactly one immutable Product Role:
Student, Instructor, or Sysadmin. A person needing multiple roles uses separate
Accounts.

**Why.** Role-specific interfaces and access rules remain explicit.

**Consequence.** Course relationships do not change the Account's Product Role.
Future Course Observer, Student Observer, and Grader roles are separate Course
relationships, not Product Roles. Grader is not currently needed because
grading is automatic.

### Course authority comes from relationships

**Decision.** All current co-Instructors in a Course Instance have equal
teaching and FERPA authority. The creator or first Instructor has no greater
authority. Student access is limited to active Course relationships and the
Student's own record.

**Why.** A privileged Course owner would contradict ordinary co-teaching and
make staff changes unsafe.

**Consequence.** A route ID or visible Course reference never grants access.
The server and database rederive the exact Account, Course relationship,
Student record, and operation predicate.

### Sysadmin is platform administration, not ambient FERPA access

**Decision.** Sysadmins manage platform configuration and operations but do not
automatically read Course Student records. Support access is deliberate,
scoped, and recorded.

**Why.** Operational privilege and educational-record access have different
purposes.

**Consequence.** A Sysadmin-created Course gains an ordinary Instructor
relationship for its teaching staff; the Sysadmin does not acquire Course
membership merely by creating or supporting it.

### Account state preserves history

**Decision.** Deactivation blocks new access but preserves authorship, Course
relationships, Student Work, and history. Reactivation restores eligible
relationships. Permanent closure is a separate process.

**Why.** Authentication state must not become accidental content or record
deletion.

## Questions and Pools

### Draft and Published Questions are separate

**Decision.** A Draft Question is private, mutable, unpublished, and
unversioned. Publication creates or advances a stable Published Question
lineage with immutable Question Revisions.

**Why.** Private authoring and public reuse have different access, storage, and
evidence needs.

**Consequence.** Draft cleanup cannot damage published content. Metadata edits
that do not change Question source do not create Revisions. A substantive fork
creates a new Question ID with attribution.

### Public Question and Pool IDs are checked human references

**Decision.** The display form is `AAAA-ZBBB`; the compact form is eight
Crockford Base32 characters. Seven characters are random identity and the
first character after the hyphen is an HMAC-derived check character.

**Why.** A short copyable Reference benefits from typo detection without
becoming sequential or authorization-bearing.

**Consequence.** See [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) for generation
and validation. UUIDs remain internal.

### Question Pools are published revisioned content

**Decision.** A Question Pool has a stable public identity and immutable Pool
Revisions. An Assessment records the exact Pool Revision and selected Question
evidence used for an Attempt.

**Why.** Reuse and random selection must remain explainable after later edits.

**Consequence.** A Pool change never silently rewrites an Assessment or
existing Student Work.

### Question Backends own Question behavior

**Decision.** A Question Backend owns rendering, interaction, response
interpretation, grading, feedback, and backend state. PLE owns authorization,
Assessment and Attempt workflow, persistence of immutable credit fractions,
score calculation, and disclosure.

**Why.** Parsing an external backend's controls inside PLE duplicates semantics
and inevitably drifts.

**Consequence.** Backend presentation and state are opaque. PLE does not infer
Question Type from controls. A backend outage never becomes an incorrect
response. See [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md).

### Native PLE Question JSON stays deliberately small

**Decision.** Native PLE Question JSON is private, unpublished, unversioned,
strictly validated, and static. It supports the eight named native Question
types in Human Guidance. Author JavaScript is isolated and untrusted; grading
is server-side.

**Why.** A closed source shape is easier to validate and teach than a public
extension ecosystem or compatibility framework.

**Consequence.** Native-format changes do not add version negotiation. New
behavior requires an explicit product decision and coordinated strict-shape
change.

## Blueprint Courses

### Blueprints use Private, Public, and Archived lifecycle states

**Decision.** Creation and forks start Private. Private is owner-only and
cannot be adopted. Public is visible to vetted Instructors and adoptable.
Archived is read-only, excluded from ordinary discovery and new adoption,
visible only through explicit archived inclusion, and forkable.

**Why.** Visibility, reuse, and retirement need clear author-controlled states.

**Consequence.** Only the owner changes lifecycle state. Public may return to
Private only before any adoption. Once adopted, it remains Public unless
Archived. Archived restores to Public.

### Blueprint Saves create content Revisions only when content changes

**Decision.** A Blueprint is created Private with Revision 1. Explicit Save
creates the next immutable Revision only after a meaningful canonical content
change. A no-op save creates nothing. Name and lifecycle metadata changes do
not create Revisions.

**Why.** Each Revision should identify an actual reusable course-content state.

**Consequence.** Relative schedules, Course dates, Students, and time zones do
not belong in a Blueprint. Course Instance creation supplies real dates and
local settings.

### Adoption, updates, forks, and Change Proposals preserve provenance

**Decision.** A Course Instance may adopt a Public Blueprint or start empty.
Adoption records the exact Blueprint Revision. New Revisions are offered to
daughter Courses for review and approval. A fork starts a new Private Blueprint
lineage with ancestry and may selectively bring in later source changes.
Instructors may propose changes to another Blueprint through a Blueprint Course
Change Proposal; accepted changes create a new receiving Blueprint Revision.

**Why.** Instructors need both reproducible adoption and independent control.

**Consequence.** Newly added Blueprint Assessments are automatically copied to
daughter Course Instances as Unreleased Assessments. Changes to existing
Assessments require the daughter Course Instructor's review and approval.
Change Proposals never change daughters directly; accepted changes reach them
through the normal Blueprint update workflow.

### Canonical Blueprint JSON is the comparison and exchange form

**Decision.** Canonical Blueprint JSON contains Blueprint metadata plus ordered
Blueprint Assessments, their reusable settings, Published Questions, and
published Question Pools. It is complete enough for comparison, import,
export, exchange, and recreation, but is not the primary persistence model.

**Why.** Blueprint comparison and exchange need one exact portable form without
turning that interchange representation into the storage architecture.

**Consequence.** Blueprint JSON contains no deadlines, release dates, Student
data, or Course Instance delivery settings. Change Proposals compare Blueprint
Revisions through this canonical representation.

### Blueprint Stars and Watches belong to the lineage

**Decision.** Vetted Instructors may Star or Watch Public and Archived
Blueprint Courses. Stars are visible endorsements; Watch state is private and
drives notifications about Revisions and other important changes. Forking or
adopting does not automatically Star or Watch.

**Why.** Endorsement and notification choices apply to a Blueprint lineage;
adoption and forking are separate Course-creation decisions.

**Consequence.** Stars and Watches follow the Blueprint lineage across all of
its Revisions. They are not copied into a fork or daughter Course.

## Assessments and Student Work

### Assessments have a small release lifecycle

**Decision.** A Course Instance Assessment is Unreleased or Released. Release
is explicit. Date-based availability does not create Closed or Archived
Assessment states.

**Why.** Extra stored states duplicate values already determined by release and
time.

**Consequence.** Unrelease is the high-consequence reversal. It requires the
typed Assessment title and deletes all Student Work for that Assessment while
preserving the Assessment and shared content.

### The whole Assessment Attempt is the submission boundary

**Decision.** Complete Question responses are saved and remain editable while
the Attempt is open. Incomplete responses are not saved as complete or graded.
The Student submits the whole Attempt; the deadline can submit it
automatically. That transition finalizes all saved responses together.

**Why.** Students need reliable navigation and saved work without accidentally
finalizing one Question at a time.

**Consequence.** The product has no separate response-finalization action,
public per-response finalization state, or Question-level grading workflow.
Repeating a whole-Attempt submission converges on the same result.

### Backend credit is immutable; points remain current

**Decision.** The Question Backend returns an immutable credit fraction. PLE
stores it unchanged and calculates the score using the Assessment Question's
current point value.

**Why.** Point corrections should update totals without pretending the Student
gave a different response or requiring regrading.

**Consequence.** PLE has no ordinary regrading, grading Retry, mutable result,
or scoring-freshness lifecycle. A current point-value edit recalculates scores
from stored fractions.

### Attempt policy remains configurable without invented grade selection

**Decision.** Instructors control Attempt limits and Assessment behavior.
Regular Assignment defaults support repeated work toward success. Human
Guidance does not yet choose which Attempt contributes to a Course grade when
several exist.

**Why.** A familiar default should not silently become a universal Gradebook
formula.

**Consequence.** Do not document highest, latest, first, or average Attempt as
the product rule until it is decided. Human Guidance also does not define a
Course Grade Scheme or Grade Category model.

### Evidence is minimal and purpose-bound

**Decision.** Retain exact Question/Pool Revision selection, backend state
needed to interpret the response, the response, immutable credit fraction, and
disclosure state.

**Why.** Student Work must remain explainable without creating an unnecessary
historical surveillance or replay system.

**Consequence.** Human Guidance does not require rendered-page snapshots,
software-version snapshots, generalized receipts, compatibility layers, or
public background-grading machinery.

## Interface

### One stable Ribbon frame serves role-specific work

**Decision.** The shell keeps stable page geometry, separates global context
from page tasks, and preserves required backed destinations even when their
collections are empty. Sign Out is in the Profile menu.

**Why.** Stable geometry and honest empty states reduce cognitive load and
prevent unavailable controls from masquerading as features.

**Consequence.** Instructor primary tabs are Courses, Questions, and
Assessments. Their exact task rows come from Human Guidance. Student work is
collectively Coursework, while a specific item uses its Assessment Type name.

### Role interfaces expose only real capabilities

**Decision.** Student, Instructor, and Sysadmin interfaces differ by actual
role responsibility. Future controls are not presented as usable. Student View
is an Instructor preview mode, not a second Account role or persistent Student
record.

**Why.** Disabled or speculative controls teach the wrong workflow.

### High-consequence actions are distinct

**Decision.** Assessment Unrelease, Published Question Archive, and Blueprint
Course Archive use a Danger Zone. Unrelease requires typing the Assessment
title. Archive actions explain their effect and require clear confirmation;
Human Guidance does not prescribe typed-title confirmation for them.

**Why.** These actions have meaningfully different consequences from ordinary
editing.

## Data, privacy, and operations

### Student Account and Course data have separate lifetimes

**Decision.** Student Accounts are global. Course removal, account
deactivation, or ordinary account closure does not delete Student Work.

**Why.** Authentication, access, and educational-record retention are separate
legal and product concerns.

### Retention starts from the final Assessment deadline

**Decision.** The final Assessment deadline starts the Course retention clock;
later Student activity resets it. Instructors receive notice before
FERPA-protected records leave normal interfaces. Records remain recoverable
during the retention period, are then permanently deleted, and the Course
becomes inactive. Course metadata, Assessment definitions, Questions, and
settings remain.

**Why.** The product must protect records while providing a predictable end to
ordinary FERPA retention.

**Consequence.** The background check is idempotent. Numeric durations and
table/job shapes are deployment and implementation decisions not specified by
Human Guidance. See [RETENTION_POLICY.md](RETENTION_POLICY.md).

### APIs remain stateless and durable state is shared

**Decision.** Correctness-bearing state belongs in PostgreSQL, typed object
storage, or the responsible Question Backend boundary, not API-process memory
or browser caches.

**Why.** Requests must survive restarts and multiple replicas.

**Consequence.** Caches contain answer-free reusable data only and never become
authorization, response, timing, or grading authority.

### Object storage is typed and server-owned

**Decision.** The database owns logical object identity and scope; the server
constructs storage keys and verifies integrity. Browsers use authorized logical
delivery routes.

**Why.** Raw paths and bucket prefixes are not authorization models.

### Background workers are justified by exact product needs

**Decision.** Expired-Attempt submission and retention checks use idempotent
background processing because they must complete without a connected browser.
Bounded asset preparation may use an operation-specific background mechanism.
A generic worker framework does not authorize grading
queues, recovery states, audit machinery, or compatibility jobs.

**Why.** Infrastructure should implement a decided behavior, not create product
behavior by implication.

## Implementation and evidence

### One canonical database baseline serves fresh installation

**Decision.** Pre-production database structure has one reviewed canonical
fresh-install path. Installation data is separate from structure and is
explicitly selected.

**Why.** Fresh and repeated installation should be understandable and
deterministic.

### Tests prove behavior at the owning boundary

**Decision.** Fast deterministic tests, connected PostgreSQL tests, live
backend probes, and browser acceptance prove different things. A passing lower
layer does not claim acceptance at a higher layer.

**Why.** Mocked or structural evidence cannot prove a real teaching workflow.

**Consequence.** Permanent tests protect stable external behavior, not internal
call order, inventories, current dates, artificial delays, or tunable defaults.
See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

### Current implementation and target product remain distinguishable

**Decision.** Documentation may describe an existing old route, table, or UI
when needed for migration or operations, but must label it as implementation
evidence and state the target Human Guidance term or behavior nearby.

**Why.** Pretending old implementation does not exist is inaccurate; treating
it as product authority perpetuates it.

## Unresolved decisions

This compliance pass intentionally leaves the following for the product owner:

- the precise timing relationship between Practice Question correct-answer
  feedback and the rule that grading outcomes appear only after whole-Attempt
  submission;
- which Attempt contributes to a Course grade when multiple Attempts exist;
- whether Course Grade Schemes or Grade Categories should exist;
- exact Student and Sysadmin Ribbon slot composition beyond Human Guidance's
  stated minimums;
- numeric retention and notice durations;
- whether Course copy/rollover/date-shift workflow is desired;
- whether any real Question Backend needs deferred completion; and
- exact route names for Blueprint lifecycle transitions.

See the temporary
[COMPLIANCE_SUMMARY.md](active_plans/reports/human_guidance_compliance/COMPLIANCE_SUMMARY.md)
for the corpus review.
