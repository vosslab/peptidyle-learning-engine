# Terminology contract

This contract defines PLE vocabulary derived from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Human Guidance supersedes this contract
whenever they disagree. Definitions describe product intent, not proof that a
workflow is implemented or accepted. Product terms use Title Case as shown below.

Use [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) for implementation spelling
and casing, and subsystem contracts for transport, storage, and API details.
Those documents must preserve the meanings established by Human Guidance.

## General rules

- Name the product object, not its current table, route, or component.
- Use Account for a global login identity and relationship for scoped access.
- Use Course for either a Blueprint Course or a Course Instance, and Library
  Object for either a Published Question or a Question Pool.
- Use Revision only for Published Questions and Blueprint Courses.
  Assessments, Course Instances, and Draft Questions use current state.
- Use Edit Number only for current-state concurrency; it is not history. When
  needed, it is a monotonic sequential counter, not a stored historical object.
  Name each clock with its domain, such as Assessment Edit Number, Blueprint
  Edit Number, or Draft Question Edit Number.
- Use Revision Number for the numbered Question and Blueprint Revisions,
  named as Question Revision Number or Blueprint Revision Number. Revision
  Numbers start at 1 and increase sequentially within each object.
  A new Revision retains the object's identity; a fork has a new identity and
  starts at Revision 1 when published or created as a revisioned object. Each Revision is a
  complete record; permitted metadata edits preserve its Revision Number.
- Use Assignment only inside the three Assessment Type names.
- A public ID is the one universal, canonical human-facing identifier for a PLE
  object that needs one. Store and use the exact same ID in the database, Rust,
  JSON, URLs, object storage, hashes, logs, and browser UI.
- Use **Id** for one value that is the canonical identity of one object.
- Use **Tuple** for multiple values that together identify one exact object, state, or version.
  Current examples are `PublishedQuestionRevisionTuple { publishedQuestionId, revisionNumber }`,
  `BlueprintRevisionTuple { blueprintCourseId, revisionNumber }`,
  `QuestionImageAssetTuple { questionImageAssetId, checksum }`, and
  `CourseRosterTuple { courseInstanceId, rosterId }`.
- Use **Reference** only for a genuine indirect, scoped, or external locator.
- Preserve the canonical ID exactly across system boundaries. Parsing,
  serialization, API transport, persistence, and display do not add, remove,
  reformat, or translate characters.
- In ID format notation, `X` denotes a cryptographically random Crockford
  Base32 character and `Z` denotes the stored calculated checksum character.
  Public IDs use the Crockford Base32 alphabet and one canonical uppercase
  ASCII form. At a human-input boundary only, lowercase Crockford characters,
  `O` or `o` for `0`, and `I`, `i`, `L`, or `l` for `1` normalize before
  canonical syntax and checksum validation. Question-ID input may also restore
  its canonical hyphen before validation and lookup.
- Public IDs include their embedded checksum character. Entry validation checks
  the ASCII bytes of every other uppercase canonical-ID character, including a
  prefix and excluding only separators and the checksum position. The checksum
  is the high five bits of public unsalted SHA-256 digest byte 0, mapped through
  the Crockford alphabet.
- Do not create product terms from job, event, receipt, snapshot, recovery, or
  compatibility mechanisms unless Human Guidance requires the concept.

### Pre-production implementation vocabulary

PLE is preparing for production launch. Until production, apply current
product vocabulary directly rather than preserving legacy terminology through
compatibility scaffolding. Before production, the initial database design is
edited directly; after production, existing databases are updated without
rebuilding them from scratch. Generic `assignment` names become Assessment;
Assignment remains only in the three
named Assessment Types. The settings surface is **Assessment Properties**.
Exact routes, JSON keys, enum encodings, and database names belong in their
owning implementation contracts rather than defining product meaning here.

## Accounts and roles

**Account** is one global PLE login identity. It has exactly one immutable
**User Role**: **Student**, **Instructor**, or **Sysadmin**. A person needing
more than one User Role uses separate Accounts.

**Instructor** is a user who teaches Courses and can discover,
reuse, create, fork, and publish Questions. All Instructors have the
same product capabilities; Course membership scopes private Course access.
Vetting happens outside PLE before Account creation. There is no separate
Verified Instructor role, status, or permission tier.

**Student** is a user who enrolls in Course Instances and completes Coursework.
Student Accounts persist across Courses and semesters. Institutional email is
required and Student email addresses are immutable.

**Sysadmin** administers PLE, vets Instructors, creates Accounts, and provides
scoped support. Sysadmin Accounts require higher security than other Accounts.
Students and Instructors authenticate with passkeys or email codes, without
passwords. The Live Demo uses its seeded entry unless an SMTP provider is
configured; real inbox acceptance remains an installation check.

**Account State** describes Account access. Instructor Account deactivation
preserves authored content, Course relationships, and history. Reactivation
restores the same Account and User Role. Account deactivation is distinct
from revoking one Course relationship.

**Course relationship** binds one Account to one Course Instance in a scoped
role. Current Student and Instructor relationships are not User Roles.
Instructors may bulk add Students through roster import. They remove Students
individually; PLE has no bulk Student-removal workflow.

**Student Record** is the Course-scoped record for a global Student Account.
It is FERPA-internal. Recovery and public APIs identify the Student through
**Course Roster** identity instead of `StudentRecordId`. Roster import finds
or creates the Account by institutional email, then uses the Course's Student
Record and enrollment. Ending enrollment or deactivating Course access revokes
future access without deleting the Account or Student Work. Course access may
be restored; retained work follows Course retention.

**Course Roster** is the Course-local Student identity used for Gradebook
labels and Student Work Recovery. **Course Roster Tuple** is Course Instance
ID plus Course Roster ID.

**Co-Instructor** is any current Instructor relationship in a Course Instance.
All co-Instructors are equal. Do not use Course Owner, primary Instructor, or
creator privilege for the current Course model.

**Course Observer**, **Student Observer**, and **Grader** are possible future
Course relationships rather than additional current User Roles. A Course
Observer would have read-only Course content and non-FERPA aggregate access;
a Student Observer would have authorized read-only access to a particular
Student's Course information. These are future capabilities. Graders are not
currently needed because grading is automatic.

**Sysadmin access** is full administrative access to PLE, including Course and Student
records. Before accessing FERPA-sensitive Student data, a Sysadmin confirms that access is needed
for administrative work. That access is recorded for audit.

## Human-facing identifiers

Public IDs are the canonical human-facing identifiers for PLE objects that
need them. Human Guidance names them **IDs**, not references:

| Object | Product name | Canonical form |
| --- | --- | --- |
| Account | Account ID | `UXXXXXXXZ` |
| Course Instance | Course Instance ID | `CIXXXXXXXZ` |
| Assessment | Assessment ID | `AXXXXXXXZ` |
| Blueprint Course | Blueprint Course ID | `BPXXXXXXXZ` |
| Published Question | Question ID | `XXXX-ZXXX` |
| Question Pool | Pool ID | `XXXX-ZXXX` |

Prefixed IDs use seven cryptographically random Crockford Base32 characters
plus embedded checksum `Z`; `Z` is a calculated placeholder, not a literal
character. Account IDs are Sysadmin support IDs and are not automatically
exposed to Students or Instructors.

Store and use the exact same ID in the database, Rust, JSON, URLs, object
storage, hashes, logs, and browser UI. An object with a public ID uses that
ID as its primary key and as the target of every foreign key to it. Do not
keep a second UUID identity beside the public ID. Objects without a public ID
use a native UUID primary key, or a composite natural key when owned by a
parent.

Give an internal object a public ID when a useful human-facing workflow needs
to display, search, communicate, or support it.

Published Questions and Question Pools share one `XXXX-ZXXX` namespace. A
value identifies either a Published Question or a Question Pool, never both.
Human-facing IDs reveal no creation order, counts, database keys, ownership,
or metadata. Generation enforces global uniqueness across every public-ID
object type and retries random collisions. Once issued, a public ID
permanently identifies that object and is never reassigned, including after
deletion or archival. UUIDs do not appear in visible content, navigation URLs,
or copyable links. An opaque identifier remains FERPA-sensitive when it links
a Student to activity.

## Content classification

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#content-classification).

**Content classification** is one shared global vocabulary for Courses and
Library Objects, following **Discipline -> Subject -> Topic -> Subtopic**.

**Discipline** is the broad academic field, such as Biology or Chemistry.
Sysadmins exclusively create and manage Disciplines and their lifecycle.
Instructors select from that vocabulary and can request a missing Discipline.

**Subject** is a globally named area, such as Genetics or Biochemistry, with
one or more Discipline associations. Subject names are unique across PLE.
Instructors may create Subjects within a selected Discipline. If the name
already exists, PLE offers the existing Subject and requires explicit
Instructor acceptance before associating it with the selected Discipline.

**Topic** is a major area within exactly one Subject. **Subtopic** is a narrower
area within exactly one Topic. Instructors may create both within their parents.
Subject, Topic, and Subtopic names are trimmed before consistent formatting
and length validation; length allowances increase as classification narrows.

Every Course has exactly one Discipline and optionally one Subject, Topic,
and Subtopic. Every Library Object has exactly one Discipline and one Subject,
with optional Topic and Subtopic. A selected Subject must be associated with
the selected Discipline; selected Topic and Subtopic must follow their parents.
A Course Instance may differ in classification from its Blueprint Course.

**Tag** is an optional label outside the hierarchy. Courses and Library Objects
may have any number of Tags, including none. Classification selection and
browsing start with Discipline; search may explicitly include a Subject's
content across its other Disciplines.

## Questions

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-specifications).

**Question** is a PLE object containing the content, metadata, and Backend information needed
to present and grade one automatically graded item. It has one canonical title and its own
internal Question record. Answer-choice randomization belongs to the Question, not the Assessment.

Use **Draft Question**, **Published Question**, or **Question Revision** where that distinction
matters. Use **Library Object** when a rule applies equally to Published Questions and Question
Pools. Name the individual kinds when defining Library Object or when the distinction matters.
Use **Question** for general content, display, responses, and grading.

**Question Backend** is the component that owns Question rendering,
interaction, response interpretation, grading, feedback, and backend-specific
state. PLE owns authorization, Question identity, Revisions, persistence,
lifecycle, and stored outcomes. Backend adapters retain their own interaction
knowledge. When PLE requests a grading outcome, the Question Backend returns
it without a deferred grading state. Initial primary Backends are PLE-native
JSON and WeBWorK; iMathAS and H5P are secondary Backends in the product design.
This designation does not establish current runtime support.

**Question Format** identifies the source/adapter contract, such as native PLE
Question JSON, WeBWorK PG, or WeBWorK PGML. Preserve PG and PGML as distinct
source formats; PGML labels require fully PGML-compliant source.

**Algorithmic Question** is one Published Question whose Backend generates
variants from parameterized source. Prefer canonical algorithmic PG or PGML
to generated static variants. Variants do not become separate Published
Questions or a Pool. Pool selection among distinct Questions and backend-native
randomization are separate forms of variation.

**Question Type** identifies the interaction for discovery, filtering, labels, and presentation.
Native JSON has a built-in Type. Other Backends use Type as editable classification metadata;
the author or importer supplies it, with source detection where reliable. Type labels are
MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. Backend rendering and grading remain
owned by the Backend.

**Draft Question** is private, mutable, unpublished, and unversioned. Its **Edit
Number** may protect concurrent saves but does not create Draft Revisions.

**Question Publication Validation** checks Discipline, Subject, and all other
required Library metadata before a Draft becomes a Published Question. Drafts
are outside the Question Library. Instructors may delete them; abandoned-Draft
cleanup requires an appropriate warning and recovery period.

**Published Question** is a stable reusable Question lineage in the Question
Library. Its **Question ID** has the canonical form `XXXX-ZXXX`.
The hyphen is part of the canonical form and makes the value immediately
recognizable as a Question ID. Human input may omit the hyphen, but PLE always
stores, transmits, and displays the canonical hyphenated form. Seven Crockford Base32
characters are cryptographically random identity; the first character after
the hyphen is an embedded checksum character. Checksum input for `XXXX-ZXXX`
is the ASCII bytes of `XXXXXXX`: calculation excludes only the hyphen and `Z`,
while the canonical value retains both. The checksum detects malformed IDs.

**Question Revision** is a complete Question record. Publishing a new Revision creates another
complete record with the same Question ID and the next Revision Number. Permitted metadata edits
change fields on the current record in place. Changes to source, answers, grading rules, Hints,
Question Feedback, Worked Solutions, or assets create a new Revision. Existing Assessments and
Student Work retain their references and fixed source and grading content. See
[QUESTION_REVISION_SPEC.md](QUESTION_SPECS/QUESTION_REVISION_SPEC.md).

**Question fork** begins as a private Draft Question owned by the Instructor who creates it.
Any Instructor may fork a Published Question. Publication requires Question Publication Validation
and establishes a new public Question ID at Revision 1. A fork starts with the source Question's
license, authors, metadata, and Question content, and records the source Question as its parent.
It retains the exact source Question Revision Tuple. Sysadmins retain their full administrative editing
authority. Access to FERPA-sensitive Student data follows the confirmation and audit rule above.

**Archive Published Question** makes the ordinary Question read-only and removes it from normal
discovery while preserving it and its existing references. Archived Questions can be restored or
forked. See [PUBLISHED_QUESTION_SPEC.md](QUESTION_SPECS/PUBLISHED_QUESTION_SPEC.md#archive).

**Question Library** is the global collection and Instructor discovery
and reuse surface for Published Questions and Question Pools. **Library Object**
means either of those published objects. Drafts are excluded. Private Course
use does not make a Library Object private. Students access Question content
through authorized Coursework rather than Library discovery.

**Questions in no Pool** is the search filter for Published Questions with no Pool
membership. Both Questions and Pools appear in one combined search. Questions in no Pool plus
Pools is a tentative default candidate, reducing redundant member results; HG retains "probably"
and the final default remains open. Filters can include individual member Questions.
A directly added Assessment Question is a separate concept: it may also belong to a Pool.

**Library result kind** is the search choice **Both**, **Questions**, or **Pools**.
**Question membership** is the search choice **Questions in no Pool** or **All Questions**;
it restricts Question rows and does not restrict Pool rows. These are Library-search terms, not
Question or Pool metadata.

**Starred Library Object** is an Instructor favorite and visible endorsement.
Instructors can see Star counts and who Starred a Question or Pool.
**Watched Library Object** is a subscription to in-app notifications about new
Question Revisions, Pool member-list changes, and forks. Watch lists remain
private. Students and anonymous users receive neither Instructor identity
lists nor Watch information. Stars and Watches are not Student Work.

**Library Object Statistics** are aggregate counts kept separately for each
Published Question Revision and for each Question Pool. They show how often Students received a
Published Question Revision or a Question from a Pool and how much credit they earned: graded-response
count, average stored credit, full-credit percentage, and zero-credit percentage.
Question results describe each Revision separately. Cross-Revision rollups are not
approved intended behavior. Removing names alone does not
make statistics anonymous. Shared statistics must prevent identification or
reconstruction of individual Student activity; Course-specific analysis remains
FERPA-sensitive when Students can be inferred. Privacy-safe aggregates survive
Student-record deletion.

**Bloom Classification** combines independent **Bloom Cognitive Process** and
**Bloom Knowledge Dimension** metadata on Question Revisions and Question
Pools. It
describes the cognitive work needed for full credit, rather than **Question
Difficulty**. A Pool's classification describes its intended cognitive work as
a whole. Bloom Classification may be blank at Library entry; initial AI assignment
is deferred and does not block publication. Pending values have no enforced time limit.
Question Type and other required publication fields remain non-NULL. The owning Instructor may correct either dimension
without a new Revision.
Bloom is ordinary editable metadata on the complete Question record or current Pool record,
like Title. A correction uses the ordinary record save and concurrency checks and preserves the
Question Revision Number. The owning Instructor or a Sysadmin may correct it. A Pool uses its
own values, independently of its members' values.
Teaching interpretation belongs in [BLOOM_TAXONOMY_GUIDE.md](BLOOM_TAXONOMY_GUIDE.md).

**Hints**, **Question Feedback**, and **Worked Solutions** are optional
PLE-managed support content on Questions or Pools, attached where they apply.
Hints and Worked Solutions have their own disclosure settings. Question
Feedback is separate content from correct answers and the Grading Outcome;
its timing and relationship to correct-answer visibility remain deferred. These are distinct from backend-generated
interaction feedback, including when similar material exists in WeBWorK source.

**Backend-generated interaction feedback** is transient unless the Backend
provides a robust preservation method. PLE does not extract or reconstruct it
from source or output. This describes who produces and preserves the content;
optional Question Feedback timing remains deferred in HG.

## Native PLE Question JSON

**PLE Question JSON** is the native private, unpublished, unversioned, static,
strictly validated Question source format. Its `format` value is
`pleQuestionJson`; there is no format-version negotiation.
"Private and unpublished" describes the internal format, not the availability
of a Published Question whose source uses it. Stored native sources may be
upgraded together. Native Questions are static and receive no random seed.

It supports exactly these native Question types: multiple choice, multiple
answer, fill in the blank, multiple blank, numerical, matching, ordering, and
hotspot.

**Author JavaScript** is optional Question-authored browser behavior executed in
an isolated untrusted environment. It is never authorization or grading
authority. Native grading remains server-side.
Author JavaScript is limited to rendering and interaction, isolated from PLE
application state, APIs, credentials, and privileged browser context. Native
HOTSPOT interaction is PLE-owned and uses supported static images or SVG.

**Recorded external resources** include native-source links, images, scripts,
stylesheets, and other URLs, including JavaScript dependencies and CDN domains.
They are explicit and reviewable. Approved dependencies may initially use
recorded CDNs and should eventually be owned and served locally by PLE.

**QTI interchange** covers import, export, and archival exchange. Importers are
transient translators into PLE-managed Question representations. QTI is not
PLE's internal source or another runtime Question model. A QTI ZIP, retained
QTI archive, and extracted QTI image are interchange roles, not Question Image
Assets.

**Question Image Asset** is a still image bound to an exact Question Revision.
Current kinds are PNG, JPEG, and WebP. **Question Image Rendition** is the
authorized delivered form of that image. Course Banner, Profile Image, and
WeBWorK renderer files keep their own identities.

## Question Pools

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-pool-specifications).

**Question Pool** is a published reusable collection with a stable **Pool ID**
(`XXXX-ZXXX`). It contains an unordered set of Question Revision Tuples plus Pool metadata.
A Question Pool cannot contain two Revisions of the same Published Question.
When the set of Question Revision Tuples changes, saving advances the Pool's **Edit Number**.
Changes take effect when the Instructor saves them. There is no undo after saving.
The counter identifies no historical Pool.
Spreadsheet-style sorting changes the editor display, not membership or selection.
Questions and Pools remain distinct objects even though each may occupy an Assessment
position. Members share one Question Type and one Backend; Pools cannot contain Pools.
Creation begins with a Published Question and enters the Library immediately.

**Pool classification** uses the first Question's Discipline and Subject.
Every additional member has that same Discipline and Subject. Member Questions
retain their own Topic, Subtopic, Tags, and other metadata; the Pool also has
its own Title, Description, Topic/Subtopic, Tags, both Bloom dimensions, and optional
support content. Search matches the Pool's own text and metadata, not its members'.
The **Pool owner** is the Account recorded when the Pool is created; a fork records its source
Pool separately. A Pool has no separate Author field.
Members retain their owners and authors. PLE calculates one compatible Pool license
from exact member-Revision licenses and rejects incompatible combinations; that calculated value is
the **calculated Pool license**. Member licenses remain intact.
NC and ND content are deferred.

A Pool must continue to meet its requirements. Show the specific problem, such as a Published
Question with a different Discipline or Subject, duplicate Questions, or another unmet requirement.
If an Assessment requests more Questions than the Pool can provide, show the problem and block
release. The Pool itself may still be valid.
Before release, save incomplete editing state and show the specific problem.
Block affected Assessment release until resolved. An Assessment already released when
a problem develops continues as-is.

**Question Pool fork** is a regular Pool with a new ID, its own Instructor owner, and a
parent-Pool pointer. It starts at Edit Number 1 with the same Question Revision Tuples.
It can be reused in many Assessments. Saving changes to the fork leaves the parent Pool unchanged.

**Pool selection** is PLE selecting the number of Questions the Assessment requests from a Pool.
That number belongs to the Assessment, not the Pool. A resumed Attempt
retains its selections; a new Attempt makes fresh selections. The selected
Question Backend owns the resulting interaction.

**Pool selection evidence** pins four values for every Question served from a
Pool: the Published Question ID, its Revision Number, the Question Pool ID,
and the Pool's Edit Number at selection time. The pinned Published Question
Revision is what later interpretation and grading need. Later Pool changes do
not rewrite an Assessment or Student Work.

## Courses

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#course-specifications).

**Course** is the general term covering Blueprint Courses and Course Instances.

**Blueprint Course** is reusable Course content. It has Blueprint Assessments
and no Students, Student Work, dates, time zones, or relative schedules.

The Blueprint lifecycle is:

- **Private**: owner-only and not adoptable;
- **Public**: visible to Instructors and adoptable; or
- **Archived**: read-only, excluded from ordinary discovery and new adoption,
  available through explicit archived inclusion, and forkable.

The **Blueprint Course Owner** is the owning Instructor with ordinary editing
and lifecycle authority. Visibility grants reading, not editing. A Public
Blueprint can return to Private only before any adoption. Once adopted, it
remains Public unless Archived. Archived restores to Public.

**Blueprint Revision** keeps saved reusable content fixed. Permitted metadata changes keep the
Revision Number. Creation
produces a Private Blueprint at Revision 1. A meaningful explicit Save creates
the next Revision; a no-op creates none. Blueprint metadata changes, including
names, do not create Revisions. Lifecycle changes are separate from content saves.

**Blueprint fork** creates a new Private lineage with ancestry.
An Instructor may fork a visible Public or Archived Blueprint and owns the
fork. The fork records the source Blueprint Course Revision Tuple, creates new
Blueprint Assessments, retains Question Revision Tuples,
and retains existing Pool references. Explicit Pool forks provide independent customization.
Source changes are not automatically applied to forks.

**Adoption** connects a Blueprint Course and a Course Instance. Creating a new
Course Instance from a Public Blueprint Course establishes Adoption and copies
every Assessment, its Questions, existing Pool references, and reusable settings from the
selected exact Blueprint Revision. The daughter records its parent Blueprint Course Revision Tuple; copied Assessments are independent current Course state,
Unreleased, with dates unset. A Blueprint Course tracks its Adoption count.

**Create Blueprint from Course Instance** creates a new Blueprint Course from
an existing Course Instance's reusable structure and establishes Adoption. The
new Blueprint Course records the existing Course Instance as its source and
counts it as the new Blueprint Course's first Adoption. The originating Course
Instance remains the same teaching instance. This source relationship is distinct
from the parent/adopted-Revision provenance of a daughter Course Instance.

**Daughter Course Instance** is a Course Instance created from a parent Blueprint
Course through Adoption.
New Blueprint Revisions are offered to daughters for Instructor review and
approval; changes to
existing Assessments are never silently applied. A newly added Blueprint
Assessment is copied automatically as an Unreleased Course Instance Assessment.

**Blueprint update** is the Instructor-reviewed path for bringing a newer
Blueprint Revision's selected changes into a daughter Course Instance. Also
called **Blueprint incorporation**, it is distinct from the
automatic addition of a newly added Blueprint Assessment.

**Blueprint fork comparison** compares visible related Blueprints in the same
fork lineage, normally their newest Revisions, on Instructor request through
canonical JSON. Shared Published Question IDs supply durable relationships;
comparison does not require Blueprint Assessment identity or history across
forks. It shows shared, added, removed, and changed Assessments, Questions, and
Pools despite name, order, or structure changes. Visibility governs comparison;
fork ownership governs incorporating selected source changes into that fork.
Recorded origin is provenance, rather than the required normal comparison pair.

**Blueprint Course Change Proposal** is an Instructor proposal to change
another Blueprint Course. The receiving owner chooses what to accept, and
accepted changes create a new receiving Blueprint Revision. A proposal never
changes daughter Course Instances directly.
It records exact source and target Blueprint Revisions and uses canonical JSON
to describe proposed changes in Instructor-readable terms. It may cover Course
metadata, Assessment settings or structure, and Question membership; Question
source changes belong to the Published Question. The receiving Instructor may
accept all or selected changes into the current target. The proposal retains
what was proposed and accepted. A newer target must be apparent, and changes
that no longer apply cleanly must not be silently applied against it.

**Canonical Blueprint JSON** is the complete comparison, import, export, and
exchange representation for Blueprint Course content. It is not the primary
persistence model and contains no Student or Course Instance delivery data.
It includes Blueprint metadata and ordered Assessments, their reusable settings,
and ordered Published Question and Question Pool entries. Export/import must reproduce
the same complete Course content and structure.

**Starred Blueprint Course** is a visible Instructor endorsement;
Instructors can see the count and who Starred it. **Watched Blueprint Course**
is a private Instructor subscription to Revision and important-change
notifications. Both belong to the Blueprint lineage across Revisions, and
neither is created automatically by adoption or forking.
Instructors may Star or Watch Public and Archived Blueprints.

**Promoted** is a searchable boolean Blueprint Course flag controlled
exclusively by Sysadmins. It is distinct from Stars, Watches, and lifecycle.

**Course Instance** is delivered teaching with current Course settings,
Students, equal co-Instructors, Course Instance Assessments, Attempts, and
FERPA-protected records. It may adopt a Public Blueprint or start empty, must
always have at least one assigned Instructor, and may supply the reusable
structure for a new Blueprint Course. The originating Course Instance remains
the same teaching instance. It represents one teaching period and remains
Active for at most six months from creation. A new academic term uses a new
Course Instance; rollover is not a separate product model.

Each Blueprint Course and Course Instance has its own deliberately entered
**Short Name** and **Long Name**. The short name should remain under about 16
characters when practical. Course Instance names are not derived from the
parent Blueprint names.

**Active Course** and **Inactive Course** describe whether the Course is in
current teaching or past-Course interfaces. A Course Instance becomes Inactive
six months after creation. Inactivity is separate from FERPA archival or
deletion and is not a Blueprint lifecycle state.
PLE warns Instructors before the six-month transition. The latest Assessment
deadline ends normal teaching and starts the separate FERPA retention clock.

## Dates and time zones

Assessment deadlines are stored as absolute UTC instants. Instructor-entered
dates use that Instructor's IANA time zone; Student displays use the Student's
IANA time zone. A new Student defaults once to the inviting Instructor's zone.
Changing either display zone changes presentation only and never moves a stored
deadline. Blueprints contain neither instants nor relative schedules.

## Assessments

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#assessment-specifications).

**Assessment** is the generic object that organizes Questions and Question
Pools into graded or practice work.

**Assignment** is not an object, category, or parent Type. It appears only in
the names Weekly Assignment, Unit Review Assignment, and Bonus
Assignment.

**Assessment Type** is one of:

- **Weekly Assignment**;
- **Unit Review Assignment**;
- **Bonus Assignment**;
- **Quiz**; or
- **Exam**.

Type supplies defaults. An Instructor may change settings without changing the
Type. Instructors cannot create new Types.

**Theme** is one of the 15 closed, durable visual identities. Each Theme has a
coordinated Light appearance and Dark appearance. A **Look** is one Theme in
one Display Mode, so PLE has 30 Looks. A **Palette Color** is one of a Look's
five source colors: Canvas, Surface, Secondary, Accent, or Highlight.

**Personal Theme** is an Instructor's Theme for global Instructor pages.
**Course Theme** is the Theme an Instructor assigns to a Course. Course pages
use their Course Theme; global Instructor pages use the Personal Theme; other
global pages use the default `grass` Theme.

**Display Mode** is Light or Dark. **Display Mode Preference** is an Account's
nullable explicit Light or Dark choice. When it is unset, the document follows
the browser preference live. Theme selection and Display Mode Preference are
independent: neither changes the other. There is no System or Auto Display
Mode, Theme inheritance, Theme-strength setting, or per-Course Display Mode.

The product-defined Font Awesome Type icons are `pen-to-square` for Weekly
Assignment, `arrows-spin` for Unit Review Assignment, `star` for Bonus
Assignment, `circle-question` for Quiz, and `file-signature` for Exam. Themes
define Type colors while preserving Type meaning and icons. Labels and icons
remain sufficient without color.

**Assessment Question** is one ordered Question position with a current point
value. **Randomize question order** is the setting name for Question-order
randomization.
Assessment content is an ordered sequence of Published Questions and Pools.
Questions retain the same public ID and exact Revision. Adding a Pool references its existing ID
and the Assessment specifies how many Questions to select. Pool forking is an explicit action.
Both Assessment forms use the same underlying model
and are ordered within their Course.

**Blueprint Assessment** is reusable Assessment content and teaching settings
inside a Blueprint Course. It has no Students, Student Work, due/release dates,
or other Course Instance delivery settings.

**Course Instance Assessment** is an Assessment delivered to Students. It has
current Questions/Pools, point values, timing, access, Attempt, and disclosure
settings.

**Assessment Template** is an Instructor-owned reusable set of Assessment
settings for creating Course Instance Assessments. It has one Assessment Type
and contains no Questions or Pools. Creating a Course Instance Assessment
copies its settings; later Template changes do not change existing Assessments.
Blueprint Assessments do not use Templates.

**Assessment Question Editor** is the Instructor composition surface.
**Assessment Properties Editor** is the settings surface.

**Assessment Release Validation** checks Questions, point values, timing order,
reasonable dates, Attempt/time limits, and other required values. A due date is
reasonable only when it is at least 24 hours in the future and no later than
the Course Instance's six-month Active limit. Validation is automated and
interactive, explains each correction, and can be rerun. A Course Instance
Assessment can become **Released** only after validation passes. New Course
Instance Assessments begin **Unreleased**.

Unreleased and Released are the only stored Assessment lifecycle states defined
here. Do not use Closed or Archived as Assessment states; date-derived access
does not require another state.

**Assessment Unrelease** is the Danger Zone action that requires the exact
Assessment title, returns it to Unreleased, and permanently deletes all Student
Work for it.
Deletion includes Attempts, saved responses, submissions, and grading outcomes.
The Assessment, Questions, settings, and other teaching content remain editable.
A later release requires normal validation and begins without the earlier work.

**Assessment availability** follows release, dates, and access settings rather
than an additional lifecycle state. New Course Instance Assessments default to
allowing new Attempts and submissions only through the due date and rejecting
late work.

## Assessment Types and disclosure

Weekly Assignments support regular learning and default to unlimited Attempts.
They reinforce current learning and may introduce new topics. Unit Review
Assignments provide focused review of covered material and may carry a small
number of points or extra credit. They use the same whole-Attempt submission
boundary as other Assessments and default to showing the correct answer
immediately after submission.
Bonus Assignments are optional extra credit, worth zero points possible, and
add earned points directly to the grade. Quizzes assess recent material; Exams
are individual assessments associated with scheduled exam periods. Both allow
one Assessment Attempt and may use more restrictive settings.

Weekly and Bonus Assignments rarely show the correct answer but show the
Student response and correctness. Quizzes and Exams default to withholding
correct answers until all Students in the Course complete the Assessment;
Instructors can change that setting. Completion means Student
submission or automatic submission on expiration, regardless of correctness
or score. Scores have no withholding or separate posting control. Viewing
submissions and correct answers has separate availability settings. Optional
Question Feedback timing, including its relationship to correct-answer visibility,
remains deferred in HG; this document does not select a timing rule.

## Attempts, responses, and scoring

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#assessment-attempt-specifications).

**Assessment Attempt** is one Student occurrence of one Course Instance
Assessment. Blueprint Assessments have no Attempts.
Student Work begins when a Student starts an Attempt. Saved responses persist
across browser sessions. Instructors control permitted Attempt counts within
the product rules; automatic submission and grading require no Instructor action.

**Saved response** is a complete Question response retained while the Attempt
is open. It is replaceable until whole-Assessment submission. An incomplete
response is unsaved for product purposes and is not graded.
The Question interface may keep unfinished input locally while the Student
works. Response actions describe saving or changing a response, such as **Save
response**. **Assessment submission** remains the whole-Attempt action.

**Assessment submission** is the whole-Attempt transition. It finalizes all
saved responses together as Student Work. A Question without a complete saved
response remains visibly unanswered, receives zero credit, and counts as
incorrect without backend evaluation. An internal row or ID must not be
documented as another Student action or product lifecycle.
The Backend may evaluate before submission when its interaction needs that
evaluation, but the Student sees the Grading Outcome only after whole-Attempt
submission. There is no separate Student or Instructor grading workflow.

**Attempt time limit** is a finite duration for each Attempt. An Assessment
contains at most 250 delivered Questions; a Pool counts by the number selected,
not its total membership. The default is 1.5 minutes per delivered Question,
rounded up to the nearest whole minute. An explicit Instructor override may
be at most 12 hours. Individual Student accommodations apply afterward and
may extend the effective limit to at most 24 hours. The interface shows the
calculated default and the specific override.

**Attempt expiration** uses a server-owned wall-clock deadline. Time continues
while disconnected. Reconnect/resume does not pause or extend it. Expiration
submits the whole Attempt and applies the same saved-response and unanswered-
Question rules as Student submission. Interaction checks expiration, and
background processing ensures submission even after the Student leaves.

**Grading Outcome** is the immutable **credit fraction** returned by the
Question Backend for a complete response it evaluates. PLE stores it unchanged under the current grading model; Native JSON regrading
is deferred in HG.
An unanswered Question's zero contribution requires no Backend evaluation.

**Assessment score** is calculated from stored credit fractions and current
Assessment Question point values. A point change recalculates scores without
backend interaction or regrading. Removing a whole Pool excludes its earned and
possible points from every Attempt. Question scores and the Assessment total
are visible after submission and automatic grading, without a posting step.

When an Assessment has multiple submitted Attempts, the highest Assessment
Attempt score is the Student's Assessment score. PLE uses Question points rather
than separate Question weights, Grade Categories, weighted categories, Course
Grade Schemes, or Course percentage calculations. Pilot grade export is CSV
or TSV only and carries
point-based Assessment scores for Course-level handling in the Instructor's
home LMS.

**Student Work** is the umbrella term for FERPA-sensitive records created by a
Student in a Course Instance, including Attempts, saved Question responses,
submissions, Grading Outcomes, and evidence needed to interpret submitted work.
The underlying records retain their own identities and purposes. Work preserves
the exact Attempt, Published Question Revision delivered, and, when the Question
came from a Pool, the Question Pool ID and Pool Edit Number at selection,
plus the finalized response and grading outcome. Content or settings changes
do not rewrite delivered evidence or completed Attempt history. Retain only
additional historical facts needed to interpret or grade the work correctly.

## Interface vocabulary

Instructor primary Ribbon tabs are **Courses**, **Questions**, and
**Assessments**.

Course tasks are **My Blueprint Courses**, **My Active Courses**, **My Inactive
Courses**, and **Search Public Blueprint Courses**.

Question tasks are **My Questions**, **My Draft Questions**, **Starred**,
**Watched**, **Search Question Library**, and **Browse Question Library**.

Assessment tasks are **Assessments Due Soon** and **My Assessment Templates**.

Student work is collectively **Coursework**. A particular item uses its
Assessment Type label. Student navigation and actions use familiar Student
language and describe their effect; the submission action submits the whole
Attempt rather than an individual Question.

**Student View** is a clearly labeled Instructor answer-free preview without
changing identity. It creates no Student Work, Attempts, submissions, or grades.

**Ribbon** is persistent role-specific navigation. **Breadcrumbs** form the
permanent row below it, showing human-readable names and preserving Course
context. **Profile menu** contains Profile settings and Sign Out.
The Profile avatar appears at the upper right.

**Avatar Gallery** is the PLE-provided avatar collection available to all
User Roles. Every Account receives a random gallery avatar at creation.
Students may change their gallery selection but cannot upload Profile images.
Instructors and Sysadmins share Profile functionality and may select a gallery
avatar or upload and crop their own image.

**Danger Zone** separates high-consequence actions from ordinary editing:
Assessment Unrelease, Archive Published Question, and Archive Blueprint Course.
Unrelease requires the typed title and explains Student Work deletion; Archive
requires clear confirmation of shared-availability effects. Restore uses
ordinary availability controls.

Required destinations remain visible when their collections are empty, with
honest empty states and an obvious first action where applicable. Required
Instructor destinations remain visible but unavailable while their target page
is incomplete. Other unimplemented future capabilities are not shown as usable
controls.
HG defines the Student Ribbon task layout. The complete Sysadmin Ribbon layout
remains unsettled.

## Retention

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#course-retention-and-lifecycle).

**FERPA retention clock** begins at the latest Assessment deadline. Creating an
Assessment with a later deadline or extending a deadline can move the clock,
but not beyond the Course Instance's six-month Active limit. Starting the clock
does not itself notify, archive, hide, or delete Student data. Installation-wide FERPA retention intervals, as operational
configuration, determine those later transitions. The six-month limit keeps
Course reuse or deadline extensions from indefinitely delaying FERPA retention
and deletion. Becoming Inactive does not itself delete Student records.

**FERPA archive** removes FERPA-protected Student records from normal Instructor
and Student interfaces after Instructor notice while keeping the records
recoverable during the configured retention period.

**Permanent FERPA deletion** removes those archived records at the end of the
retention period. It is separate from the Course Instance's six-month
Active-to-Inactive transition. Course metadata, Assessment definitions,
Questions, and settings remain.

The background check executes policy from stored Course dates and creation
time. It is idempotent; running late produces the same retention decision as
running on schedule. Retention intervals are operational configuration, not
separate product decisions. Human Guidance does not prescribe their numeric
durations or exact job, event, receipt, or table shapes. Student Accounts and
privacy-safe aggregate Library statistics persist independently of deletion
of identifiable Course records.

## Implementation-only vocabulary

Implementation identifiers such as a saved-response row name underlying
records; they do not imply a per-Question Student submission action. Legacy
names such as `AssignmentId`, `Id`, `<retired-term-replace-me>`, or `Available`
must not define current product meaning. Technical terms such as jobs,
generations, and receipts belong to their implementation boundaries. Use
precise identifiers when documenting source evidence, with the product meaning
or gap nearby; internal names create no additional product workflow or
lifecycle state.

Rust, TypeScript, SQL, and JSON name the same public IDs. Each language
uses its ordinary casing: SQL `account_id`, Rust/TypeScript type
`AccountId`, Rust field `account_id`, JSON `id` or nested `accountId`.
Do not keep a parallel UUID primary key, a `reference` JSON field for a
public ID, or a `public_reference` SQL alias beside a public ID. Composite
Published Question Revision Tuple JSON is the field `publishedQuestionRevisionTuple` with members
`{publishedQuestionId, revisionNumber}`. Blueprint Course Revision Tuple JSON is the field
`blueprintRevisionTuple` with members `{blueprintCourseId, revisionNumber}`.
Question Image Asset Tuple JSON is the field `questionImageAssetTuple` with members
`{questionImageAssetId, checksum}`. Course Roster Tuple JSON is the field
`courseRosterTuple` with members `{courseInstanceId, rosterId}`. Domain clocks
use qualified names such as `assessmentEditNumber`, `blueprintEditNumber`,
`draftQuestionEditNumber`, and `expectedAssessmentEditNumber`.

JSON `id` is only the immediate identity of a resource at its own root.
Nested identities use the precise `...Id`, `...Tuple`, or `...Number`. An
exact immutable Blueprint or Published Question revision is the named Tuple
`blueprintRevisionTuple` or `publishedQuestionRevisionTuple`, including Course
Instance adoption (`blueprintRevisionTuple`) and provenance
(`adoptedBlueprintRevisionTuple`, `currentBlueprintRevisionTuple`). An
HTTP `ETag` is only a quoted encoding of an explicitly named Edit Number or
Revision Number; domain fields do not store ETag-shaped values.
