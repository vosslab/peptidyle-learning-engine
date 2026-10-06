# Identity contracts

This document maps PLE identities to the current product model in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Generic teaching objects are
Assessments. Assignment appears only in the three Assessment Type names.

## Rules that apply everywhere

- A durable ID names one thing.
- One canonical identity value is an Id. Multiple values that together identify
  one exact object, state, or version are a Tuple, including Question and
  Blueprint Course Revision Tuples. Reference is only a genuine indirect, scoped, or
  external locator; do not name an Id or a Tuple as a Reference.
- A checksum detects disagreement in otherwise valid data.
- A public ID is the one universal, canonical human-facing identifier for a PLE
  object that needs one.
- A public ID remains its exact canonical string in PostgreSQL, Rust, JSON,
  URLs, object storage, hashes, logs, and browser UI; no boundary translates,
  reformats, strips, reconstructs, or derives it.
- Internal UUIDs remain internal. An object with a public ID uses that public
  ID as its primary key. There is no parallel UUID key beside a public ID.
  JSON for those objects is `id` (or nested `courseInstanceId` / `assessmentId`); there
  is no parallel `reference` property.
- Published Questions and Blueprint Courses use numbered Revisions for saved content.
  Each Revision is a complete record; permitted metadata edits preserve its Revision Number.
  Saving changes to a Question Pool advances its Edit Number.

## Account and relationship identities

PLE is one installation with global Accounts. Each Account has exactly one
immutable user role: Student, Instructor, or Sysadmin. A person who needs
more than one role uses separate Accounts.

| Identity or relationship | Scope and meaning |
| --- | --- |
| Account ID | One global login Account; distinct from Course membership and Student Work |
| Student email | The immutable university or institutional address for one Student Account |
| Session ID | One server-tracked login session; not the browser credential itself |
| User role | The Account's one immutable Student, Instructor, or Sysadmin role |
| Course Instance ID | One delivered Course |
| Course relationship | One Account's Student or Instructor relationship to one Course |
| Student record ID | The global Student Account's FERPA-protected record in one Course Instance. Internal only; recovery and public APIs use Course Roster identity instead |
| Course Roster ID | `CourseRosterId`: Course-local Student roster identifier inside one Course Instance |
| Course Roster Tuple | `CourseRosterTuple { courseInstanceId, rosterId }` |
| Blueprint Course ID | One reusable Blueprint lineage and current lifecycle state |
| Blueprint owner relationship | The one Instructor who owns that Blueprint lineage |
| Authoring workspace ID | One private Draft Question workspace |

Every current co-Instructor has equal Course authority. The creator or first
Instructor has no special ownership. A Sysadmin Account does not gain Course
membership from its global role. Sysadmins have full administrative authority. Before accessing
FERPA-sensitive Student data, they confirm that access is needed for administrative work; that
access is recorded for audit.

Removing a relationship or deactivating an Account does not erase the related
Course history or Student Work. Retention is a separate Course process.

Students and Instructors authenticate through passkeys or email codes. A
Student may have multiple passkeys, but the Student's institutional email does
not change. Authentication factors prove Account access; they do not establish
Course membership.

## Content identities

| Identity | Scope and meaning |
| --- | --- |
| Draft Question ID | One private, mutable, unpublished Draft Question |
| Question ID | One stable Published Question lineage, in canonical form `XXXX-ZXXX` |
| Question Revision Tuple | One exact Question Revision: Question ID plus Question Revision Number |
| Question Pool ID | One published Pool in the shared `XXXX-ZXXX` namespace; contains an unordered set of Question Revision Tuples |
| Blueprint Module ID | Stable UUID lineage identity for one retained Blueprint Module |
| Blueprint Assessment ID | Stable UUID lineage identity for one retained Blueprint Assessment |
| Blueprint Course Revision Tuple | Blueprint Course ID plus Blueprint Revision Number; identifies one exact Blueprint Revision |
| Assessment ID | One current Blueprint or Course Instance Assessment; not a revision family |
| Assessment Attempt ID | One Student's occurrence of one Course Instance Assessment |
| Question Image Asset Tuple | `QuestionImageAssetTuple` with `QuestionImageAssetId` member `questionImageAssetId` plus checksum; every field holding it is `questionImageAssetTuple` |
| Object Address | Typed physical object location. Members use `objectId`, `questionImageAssetId`, `workspaceId`, `workspaceImportId`, `courseBannerId`, and `draftQuestionId` |
| Object ID | One immutable stored object |

In ID format notation, `X` is one cryptographically random Crockford Base32
character and `Z` is the embedded calculated checksum character; both are
stored characters. Every public ID has one canonical uppercase ASCII form.
Human input may normalize lowercase Crockford characters, `O` or `o` to `0`,
and `I`, `i`, `L`, or `l` to `1` before canonical syntax and checksum
validation. The visible Question ID has
seven random characters plus one embedded public SHA-256 checksum character.
Its one canonical form is `XXXX-ZXXX` at every boundary; the hyphen is part of
the form and makes it immediately recognizable. Human Question-ID entry may
omit the hyphen; canonicalization restores it before validation and lookup.
Published Questions and Question Pools share that one namespace: a
value identifies either object, never both. See [QUESTION_ID_SPEC.md](QUESTION_SPECS/QUESTION_ID_SPEC.md).

Blueprint Course `BPXXXXXXXZ`, Course Instance `CIXXXXXXXZ`, Assessment
`AXXXXXXXZ`, and Account `UXXXXXXXZ` IDs each contain seven random Crockford
Base32 characters plus final checksum `Z`. Checksum input is the ASCII bytes of
every other uppercase canonical-ID character, including a prefix and excluding
only separators and checksum position. Public unsalted SHA-256 maps the high
five bits of digest byte 0 through the Crockford alphabet. Validation happens
before database lookup or resolution. Generation enforces global uniqueness
across every public-ID object type, retries random collisions, and never
reassigns an issued ID, including after deletion or archival.

A Question Revision is a complete record containing Question content and metadata. A source
change creates a new Revision. Permitted metadata edits update that record without changing its
Revision Number. A fork creates a new Question ID.

A Question Pool is an unordered set of Question Revision Tuples plus its metadata. Saving changes
to that set advances the Pool's Edit Number; no Revision is created. Student Work records the
selected Question Revision Tuple, Pool ID, and Pool Edit Number. Existing Attempts retain their
selected Questions after later Pool edits.

## Blueprint identities and lifecycle

A Blueprint Course is created Private with Revision 1. An explicit Save creates
the next Blueprint Revision only when reusable content changed; a
no-op returns the current Revision and creates nothing. Name and lifecycle
metadata changes do not create Blueprint Revisions.

The lifecycle state belongs to the Blueprint lineage:

- Private: owner-only and unavailable for adoption;
- Public: visible to Instructors and available for adoption; or
- Archived: read-only, excluded from ordinary discovery and new adoption, but
  discoverable through explicit archived inclusion and forkable.

Only the owner changes Blueprint lifecycle state. A Public Blueprint can return
to Private only before any Course Instance has adopted it. Archived restores to
Public. A Revision Tuple remains exact regardless of later lifecycle
changes.

Blueprints contain no Students, dates, time zones, or relative schedules.

Adoption and forks retain the source Blueprint Course Revision Tuple.
Those references support daughter update review and selective fork updates;
they do not authorize a silent mutation. A Blueprint Course Change Proposal
targets one receiving Blueprint and creates a new receiving Revision only for
changes its owner accepts.

## Assessment and Student Work identities

Assessment is the generic product object. Assignment is not an object,
category, or parent Type; the word appears only in the three Assessment Type
names Regular Assignment, Practice Question Assignment, and Bonus Assignment.

An Assessment Attempt owns the Student's saved responses and whole-Assessment
submission state. An implementation may assign internal row IDs to Question
positions or response evidence, but those IDs must not create another Student
action or product Attempt family.

The Question Backend owns opaque render state and response interpretation. PLE
binds that state to the authenticated Student, Course, Assessment Attempt,
Question position, and exact Revision evidence. A browser-supplied Attempt or
position is only a selector.

Archived Student Work Recovery returns typed Course Instance, Assessment,
Assessment Attempt, Question Revision Tuple, and `CourseRosterTuple`
identities. It never exposes `StudentRecordId`. `StudentRecordId` may remain
an internal FERPA record key.

## Future Course relationships

Course Observer, Student Observer, and Grader are future Course roles. They are
not user roles and do not exist merely because a generic capability or row
shape could represent them. Each requires its own Course relationship and
privacy contract before it becomes available. A Grader is not currently needed
because PLE grading is automatic.

## Operational identities

Internal job, lease, delivery, or support IDs name narrow technical operations.
They do not establish user roles or justify new product lifecycle states.
Scoped Sysadmin support access to FERPA-protected records must identify the
support purpose and be recorded, but Human Guidance does not require a general
audit identity for every ordinary action.

## Browser and secret boundaries

- Raw session credentials, signing keys, Answer Keys, private rubrics, backend
  credentials, and provider tokens never enter ordinary browser DTOs, URLs,
  logs, analytics, or examples.
- A signed object URL is a short-lived delivery result, not a durable object
  identity.
- A presentation token or checksum checks correspondence with server-held
  state; it never authenticates the Student.
- Student-visible public IDs and Course/Assessment IDs are resolved only
  after authorization.

## Maintainer checklist

For each new identifier, document what it names, its scope, who mints it, where
it persists, whether it crosses into a browser, and which stored relationship
authorizes its use. If possession conveys authority, use a bounded opaque
capability with expiry and redaction rather than an ordinary ID.

See [USER_ROLES.md](USER_ROLES.md),
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md),
[README.md](QUESTION_SPECS/README.md), and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
