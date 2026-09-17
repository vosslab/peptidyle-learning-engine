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
- Use Revision only for Published Questions, published Question Pools, and
  Blueprint Courses.
- Use Edit Number only for current-state concurrency; it is not history. When
  needed, it is a monotonic sequential counter, not a stored historical object.
- Revision Numbers start at 1 and increase sequentially within each object.
  A new Revision retains the object's identity; a fork has a new identity and
  starts at Revision 1 when published or created as a revisioned object.
- Use Assignment only inside the three Assessment Type names.
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
**Product Role**: **Student**, **Instructor**, or **Sysadmin**. A person needing
more than one Product Role uses separate Accounts.

**Instructor** is a Sysadmin-vetted user who teaches Courses and can discover,
reuse, create, fork, and publish Questions. All vetted Instructors have the
same product capabilities; Course membership scopes private Course access.

**Student** is a user who enrolls in Course Instances and completes Coursework.
Student Accounts persist across Courses and semesters. Institutional email is
required and Student email addresses are immutable.

**Sysadmin** administers PLE, vets Instructors, creates Accounts, and provides
scoped support. Sysadmin Accounts require higher security than other Accounts.
Students and Instructors authenticate with passkeys or email codes, without
passwords. Email authentication is not yet configured for the Live Demo.

**Account State** describes Account access. Instructor Account deactivation
preserves authored content, Course relationships, and history. Reactivation
restores the same Account and Product Role. Account deactivation is distinct
from revoking one Course relationship.

**Course relationship** binds one Account to one Course Instance in a scoped
role. Current Student and Instructor relationships are not Product Roles.
Instructors may bulk add Students through roster import. They remove Students
individually; PLE has no bulk Student-removal workflow.

**Student Record** is the Course-scoped record for a global Student Account.
Roster import finds or creates the Account by institutional email, then uses
the Course's Student Record and enrollment. Ending enrollment or deactivating
Course access revokes future access without deleting the Account or Student
Work. Course access may be restored; retained work follows Course retention.

**Co-Instructor** is any current Instructor relationship in a Course Instance.
All co-Instructors are equal. Do not use Course Owner, primary Instructor, or
creator privilege for the current Course model.

**Course Observer**, **Student Observer**, and **Grader** are possible future
Course relationships rather than additional current Product Roles. A Course
Observer would have read-only Course content and non-FERPA aggregate access;
a Student Observer would have authorized read-only access to a particular
Student's Course information. These are future capabilities. Graders are not
currently needed because grading is automatic.

**Scoped Support Access** is deliberate Sysadmin access to an exact support
need involving FERPA-protected records. It is recorded. The Sysadmin Product
Role alone provides no ambient FERPA access.

## Human-facing identifiers

**Reference ID** is a short, opaque, human-facing reference used when a workflow
needs to display, search, communicate, or support an object. References use
`BP` for Blueprint Courses, `CI` for Course Instances, `A` for Assessments, and
`U` for Accounts, followed directly by the common cryptographically random
Crockford Base32 format. Account references are Sysadmin support references
and are not automatically exposed to Students or Instructors.

Published Questions and Question Pools retain their public `AAAA-ZBBB` IDs.
Human-facing IDs reveal no creation order, counts, database keys, ownership,
or metadata. Generation enforces uniqueness and retries random collisions.
UUIDs do not appear in visible content, navigation URLs, or copyable links.
An opaque identifier remains FERPA-sensitive when it links a Student to activity.

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

**Question** is one automatically evaluated PLE question, regardless of its
Question Backend. It has one canonical title and its own internal Question
record. Answer-choice randomization belongs to the Question, not the Assessment.

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

**Question Type** is immutable author-declared educational metadata on a
Published Question Revision, used for discovery, filtering, labels, and
presentation. It is not inferred from backend controls. Native Type labels are
MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.

**Draft Question** is private, mutable, unpublished, and unversioned. Its **Edit
Number** may protect concurrent saves but does not create Draft Revisions.

**Question Publication Validation** checks Discipline, Subject, and all other
required Library metadata before a Draft becomes a Published Question. Drafts
are outside the Question Library. Instructors may delete them; abandoned-Draft
cleanup requires an appropriate warning and recovery period.

**Published Question** is a stable reusable Question lineage in the Question
Library. Its public **Question ID** displays as `AAAA-ZBBB` and has an eight-
character compact form. Seven Crockford Base32 characters are cryptographically
random identity; the first character after the hyphen is an HMAC-derived check
character. The check character detects malformed IDs and is not a security
boundary.

**Question Revision** is one immutable source-bearing version within a
Published Question lineage. Changes to source, answers, grading rules, Hints,
Question Feedback, Worked Solutions, or assets create a new Revision. Changes
to title, description, classification, Tags, or other search metadata update
the lineage without creating a Revision. Existing Assessments and Student Work
remain pinned to exact Revisions when a successor is published.

**Question fork** begins as a private Draft Question with its own authorship
and lineage. Any Instructor may fork a Published Question. Publication requires
Question Publication Validation and establishes a new public Question ID at
Revision 1. Forks and Revisions preserve attribution, contributor credit,
history, and compatible CC licensing. Forced corrections are audited Sysadmin
actions reserved for critical flaws.

**Archive Published Question** is the high-consequence action that removes a
Published Question from ordinary discovery/new selection while preserving
exact Revision evidence already used by Assessments and Student Work.

**Question Library** is the global collection and vetted-Instructor discovery
and reuse surface for Published Questions and Question Pools. **Library Object**
means either of those published objects. Drafts are excluded. Private Course
use does not make a Library Object private. Students access Question content
through authorized Coursework rather than Library discovery.

**Starred Library Object** is an Instructor favorite and visible endorsement.
Vetted Instructors can see Star counts and who Starred a Question or Pool.
**Watched Library Object** is a subscription to in-app notifications about new
Revisions, forks, improvement threads, and impact notices. Watch lists remain
private. Students and anonymous users receive neither Instructor identity
lists nor Watch information. Stars and Watches are not Student Work.

**Library Object Statistics** are aggregate counts kept separately for each
Question Revision and Pool Revision. Question statistics include accepted
graded Attempt and correct counts, and may include incorrect, partial-credit,
unanswered, and eligible answer-choice counts. Pool statistics may include use
and selection counts. Rollups across Revisions must be clearly labeled and meet
privacy thresholds. Removing names alone does not make statistics anonymous.
Shared statistics must prevent identification or reconstruction of individual
Student activity; Course-specific analysis remains FERPA-sensitive when Students
can be inferred. Privacy-safe aggregates survive Student-record deletion.

**Bloom Classification** combines independent **Bloom Cognitive Process** and
**Bloom Knowledge Dimension** metadata on Question and Pool Revisions. It
describes the cognitive work needed for full credit, rather than **Question
Difficulty**. A Pool's classification describes its intended cognitive work as
a whole. Bloom Classification is required for Library entry; AI assigns it
initially and an Instructor may correct either dimension without a new Revision.
Teaching interpretation belongs in [BLOOM_TAXONOMY_GUIDE.md](BLOOM_TAXONOMY_GUIDE.md).

**Hints**, **Question Feedback**, and **Worked Solutions** are optional
PLE-managed support content on Questions or Pools, attached where they apply.
Hints and Worked Solutions have their own disclosure settings. Question
Feedback follows its disclosure rules and is separate from correct-answer
disclosure and the Grading Outcome. These are distinct from backend-generated
interaction feedback, including when similar material exists in WeBWorK source.

**Backend-generated interaction feedback** is transient unless the Backend
provides a robust preservation method. PLE does not extract or reconstruct it
from source or output. Optional feedback is shown when the Backend provides it,
without its own delayed-release state or use of correct-answer disclosure settings.

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
PLE's internal source or another runtime Question model.

## Question Pools

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#question-pool-specifications).

**Question Pool** is a published reusable collection with a stable public ID
and immutable **Pool Revisions**. Questions and Pools remain distinct objects
even though each may occupy an Assessment position.
Pools contain interchangeable Published Questions, may span Question Backends,
and cannot contain other Pools. Creation begins with a Published Question and
enters the Library immediately. A Pool has its own public `AAAA-ZBBB` ID.

**Pool classification** uses the first Question's Discipline and Subject.
Every additional member has that same Discipline and Subject. Member Questions
retain their own Topic, Subtopic, Tags, and other metadata; the Pool also has
its own Title, Description, and applicable Library metadata and support content.

**Question Pool fork** creates a new Pool ID at Revision 1, initially retaining
the same member Question IDs and exact Revisions. Adding a Pool to another
Assessment automatically forks it into an independently editable Pool belonging
to that Assessment; the source is unchanged.

**Pool selection** is PLE's selection of the Instructor-specified number of
Questions from Instructor-chosen interchangeable contents. A resumed Attempt
retains its selections; a new Attempt makes fresh selections. The selected
Question Backend owns the resulting interaction.

**Pool selection evidence** is the exact Pool Revision and Published Question
Revision selected for an Assessment Attempt. Later Pool changes do not rewrite
an Assessment or Student Work.

## Courses

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#course-specifications).

**Course** is the general term covering Blueprint Courses and Course Instances.

**Blueprint Course** is reusable Course content. It has Blueprint Assessments
and no Students, Student Work, dates, time zones, or relative schedules.

The Blueprint lifecycle is:

- **Private**: owner-only and not adoptable;
- **Public**: visible to vetted Instructors and adoptable; or
- **Archived**: read-only, excluded from ordinary discovery and new adoption,
  available through explicit archived inclusion, and forkable.

The **Blueprint Course Owner** is the owning Instructor with ordinary editing
and lifecycle authority. Visibility grants reading, not editing. A Public
Blueprint can return to Private only before any adoption. Once adopted, it
remains Public unless Archived. Archived restores to Public.

**Blueprint Revision** is one immutable saved Blueprint content state. Creation
produces a Private Blueprint at Revision 1. A meaningful explicit Save creates
the next Revision; a no-op creates none. Blueprint metadata changes, including
names, do not create Revisions. Lifecycle changes are separate from content saves.

**Blueprint fork** creates a new Private lineage with ancestry.
An Instructor may fork a visible Public or Archived Blueprint and owns the
fork. The fork records the exact source Blueprint and Revision, creates new
Blueprint Assessments, retains Published Question IDs and exact Revisions,
and forks Pools into new Pool IDs with the same exact initial membership.
Source changes are not automatically applied to forks.

**Adoption** connects a Blueprint Course and a Course Instance. Creating a new
Course Instance from a Public Blueprint Course establishes Adoption and copies
every Assessment, its Questions, forked Pools, and reusable settings from the
selected exact Blueprint Revision. The daughter records its parent Blueprint
and adopted Revision; copied Assessments are independent current Course state,
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
and ordered Published Question and Pool references. Export/import must reproduce
the same complete Course content and structure.

**Starred Blueprint Course** is a visible Instructor endorsement; vetted
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
the names Regular Assignment, Practice Question Assignment, and Bonus
Assignment.

**Assessment Type** is one of:

- **Regular Assignment**;
- **Practice Question Assignment**;
- **Bonus Assignment**;
- **Quiz**; or
- **Exam**.

Type supplies defaults. An Instructor may change settings without changing the
Type. Instructors cannot create new Types.

The product-defined Font Awesome Type icons are `pen-to-square` for Regular
Assignment, `arrows-spin` for Practice Question Assignment, `star` for Bonus
Assignment, `circle-question` for Quiz, and `file-signature` for Exam. Themes
define Type colors while preserving Type meaning and icons. Labels and icons
remain sufficient without color.

**Assessment Question** is one ordered Question position with a current point
value. **Randomize question order** is the setting name for Question-order
randomization.
Assessment content is an ordered sequence of Published Questions and Pools.
Questions retain the same public ID and exact Revision; added Pools are forked
and independently editable. Both Assessment forms use the same underlying model
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

Regular Assignments support regular learning and default to unlimited Attempts.
They reinforce current learning and may introduce new topics. Practice Question
Assignments provide focused review of covered material and may carry a small
number of points or extra credit. They use the same whole-Attempt submission
boundary as other Assessments and show the correct answer immediately after
submission.
Bonus Assignments are optional extra credit, worth zero points possible, and
add earned points directly to the grade. Quizzes assess recent material; Exams
are individual assessments associated with scheduled exam periods. Both allow
one Assessment Attempt and may use more restrictive settings.

Regular and Bonus Assignments rarely show the correct answer but show the
Student response and correctness. Quizzes and Exams withhold correct answers
until all Students in the Course complete it. Completion means Student
submission or automatic submission on expiration, regardless of correctness
or score. Response, correctness, correct-answer, and support-content disclosure
remain distinct settings. Optional Question Feedback is separate from the
correct answer and Grading Outcome; backend-provided feedback does not use
the Assessment's correct-answer gate or its own delayed-release state.

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
response** and **Restore initial response**. **Assessment submission** remains
the whole-Attempt action.

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
Question Backend for a complete response it evaluates. PLE stores it unchanged.
An unanswered Question's zero contribution requires no Backend evaluation.

**Assessment score** is calculated from stored credit fractions and current
Assessment Question point values. A point change recalculates scores without
backend interaction or regrading.

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
the exact Attempt, Question Revision, and Pool Revision/selection delivered,
finalized response, and grading outcome. Content or settings changes do not
rewrite delivered evidence or completed Attempt history. Retain only additional
historical facts needed to interpret or grade the work correctly.

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
context. **Profile menu** owns Profile settings, account settings, and Sign Out.
The Profile avatar appears at the upper right.

**Avatar Gallery** is the PLE-provided avatar collection available to all
Product Roles. Every Account receives a random gallery avatar at creation.
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
The complete Student and Sysadmin Ribbon task layouts do not have locked-in
designs yet.

## Retention

Authority: [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#course-retention-and-lifecycle).

**FERPA retention clock** begins at the latest Assessment deadline. Creating an
Assessment with a later deadline or extending a deadline can move the clock,
but not beyond the Course Instance's six-month Active limit. Starting the clock
does not itself notify, archive, hide, or delete Student data. The configured
retention policy determines those later transitions. The six-month limit keeps
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

Implementation identifiers such as `QuestionResponse` name underlying records;
they do not imply a per-Question Student submission action. Legacy names such
as `AssignmentId`, `QuestionAttemptId`, or `Available` must not define current
product meaning. Technical terms such as jobs, generations, and receipts belong
to their implementation boundaries. Use precise identifiers when documenting
source evidence, with the product meaning or gap nearby; internal names create
no additional product workflow or lifecycle state.
