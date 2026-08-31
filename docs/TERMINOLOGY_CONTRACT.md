# PLE terminology contract

This is the concise semantic contract for PLE-owned database, API, test, and
code terminology. It turns the owner glossary in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) into implementation boundaries; it does
not supersede that owner guidance.

Use [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) after selecting the correct
domain term. Use [VOCABULARY_REPLACEMENTS.md](VOCABULARY_REPLACEMENTS.md) to
complete an in-progress correction: the checklist identifies the wording to
replace, its canonical target, and the required structural change.

## Authority order

1. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) defines intentional product meaning.
2. This document defines the corresponding shared domain vocabulary and
   relationship paths.
3. [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) defines identifier spelling.
4. A focused contract or schema document defines its physical representation.
5. [VOCABULARY_REPLACEMENTS.md](VOCABULARY_REPLACEMENTS.md) tracks convergence
   work as a retained checklist. Checked rows remain through its final audit,
   then the completed checklist retires as one document.

When a term has a narrower meaning at one boundary, name the narrower record or
relationship. A broad context object never substitutes for a stored authority
path.

## Technical boundary vocabulary

Use **HTTP** for the network protocol and **payload** for one bounded unit of
data transferred across a defined boundary. A **decoder** validates and
converts an untrusted representation into an accepted typed value.
**Transport** names the mechanics of exchanging data. **Runtime** names an
actual execution environment or lifecycle. **Type variant** remains
language-level vocabulary for one alternative in a closed type. **Consumer**
names a dependency relationship in which one component reads another
component's contract or artifact. **Message broker** names infrastructure that
actually routes messages between senders and receivers. A **Factory** chooses
among multiple construction strategies and returns an implementation through a
stable interface. Use a direct constructor for one configured result, and name
an injected callable by its action, such as acquire or create.

These technical terms describe mechanics rather than PLE records, product
surfaces, or authority. Name a PLE-owned component by the object or operation
it owns.

## Identity, authentication, and product role

**Account** is one global login identity in the single PLE installation.
Account creation assigns one immutable **Product Role**: **Student**,
**Instructor**, or **Sysadmin**. An Account has an **Account State** derived
from immutable Account State Events: Active, Suspended, or Closed.

**Authenticated Session** is one server-side authentication record for one
Active Account. A successful passkey or email-code authentication creates or
continues an Authenticated Session; suspension or closure revokes its sessions.
`Authenticated Session Reference` identifies that record. A session authenticates
an Account; it grants no course, authoring, Question Library, or FERPA authority
itself.

Each role-distinct login is a separate Account and consequently follows its own
authenticated-session path. For example, a person acting as both Sysadmin and
Instructor uses a Sysadmin Account for system administration and an Instructor
Account for teaching. This separation makes the product role a stable security
boundary while retaining ordinary passwordless authentication for each Account.

**Instructor Approval** is the current result of immutable Instructor Approval
Events. An Instructor Account requires current approval before it may use
Instructor-only Question Library, authoring, or course-creation capabilities.

**Workspace Collaborator** is an Approved Instructor with a current relationship
to one exact Authoring Workspace, derived from immutable start and end Workspace
Collaborator Events. It grants only that private-authoring relationship.

## Course relationships

**Blueprint Course** is a reusable, answer-free course definition. It has no
Students or delivery deadlines. A **Blueprint Revision** is one complete,
immutable authored definition. A **Draft Blueprint Revision** has not yet been
published. **Blueprint Revision Content** is the complete answer-free
definition held by one Blueprint Revision: its structure, defaults, relative
schedules, and exact Question Version References. A **Blueprint Content Digest**
is the SHA-256 value for that versioned content. A **Blueprint Content Check**
compares two complete Blueprint Revision Content values by their digests. A
**Relative Assignment Schedule** is the reusable schedule intent for one
Blueprint Assignment. Each Relative Assignment Schedule Moment stores a signed
calendar-day offset from Course Term start and one local time; it does not store
an absolute delivery instant.

**Resolved Assignment Schedule** is the target-term result of resolving one
Relative Assignment Schedule. Each Resolved Assignment Schedule Moment pairs
the resulting Course Local Date and Time with its exact absolute timestamp.
It belongs to one exact Assignment Revision and that revision's Course Schedule
Revision.
**Blueprint Collaborator** is an Approved Instructor with an
explicit, time-bounded contribution relationship to one exact Draft Blueprint
Revision; it grants neither Authoring Workspace nor Course Instance authority.
A **Blueprint Publication Event** makes one reviewed revision reusable by
Approved Instructors and closes its Draft Blueprint Revision collaboration.
**Blueprint Revision Availability** is the current Available or Archived state
derived from immutable Blueprint Revision Availability Events for one published
revision. It determines ordinary new selection without changing historical
references to that revision.

**Fork Blueprint Course** creates a new Blueprint Course from one exact
Blueprint Revision. **Create Course from Blueprint** creates a new Course
Instance from one exact Blueprint Revision. **Copy Assignment from Blueprint**
creates one Course Instance-owned Assignment from one exact Blueprint
Assignment and Blueprint Revision. **Apply Blueprint Update** applies one
reviewed Blueprint change through successor Course Instance records. **Copy
Course for New Term** creates a new Course Instance from an existing Course
Instance while retaining teaching content and excluding Student Work Records.
**Shift Course Dates** creates the next Course Schedule Revision and successor
Assignment Revisions for one changed Course Term.

**Course Schedule Revision** is one immutable Course Instance-owned snapshot of
its Course Term: start date, end date, Course Time Zone, and revision number.
Each Assignment Revision belongs to one exact Course Schedule Revision and
stores its resolved available, due, and close instants. A **Course Schedule
Revision Reference** pairs the exact Course Instance with its positive revision
number; it identifies that immutable record without treating the number alone
as course-wide authority.

Each operation owns its exact Readiness result, Retry Token, Manifest when one
is needed, and Receipt. The operation name remains consistent across interface,
API, schema, and code boundaries.

**Course Instance** is live teaching created from an exact Blueprint Revision.
It owns enrollment, deadlines, releases, accommodations, grades, and other
delivery-specific facts. Course Instance Creation atomically records its source
and an initial Instructor Course Membership.

**Course Origin** is immutable source history for one Course Instance. It
retains the exact Blueprint Revision and, for a rollover, the exact source
Course Instance. It is distinct from the mutable operation precondition and
does not grant authority.

**Course Rollover Manifest** is the closed copied-and-excluded state for one
Course Instance rollover. It retains bounded reusable Assignment sources and
resolved schedules, while its one exclusion policy excludes all Student and
delivery records.

**Course Instance Operation Receipt** is one exact immutable receipt for Copy
Course for New Term, Shift Course Dates, Apply Blueprint Update, Copy Assignment
from Blueprint, or reconciliation. The server-only reconciliation selection
holds one closed receipt variant; public callers receive the exact receipt for
the operation they requested.

**Assignment Source Snapshot** is the immutable operation precondition for one
Assignment import. It binds the exact Blueprint Assignment Revision source,
destination Assignment Revision, and import revision before an update or
selected copy can proceed.

**Assignment Source Record** is immutable server-held evidence that an
Assignment Revision came from one exact Blueprint Assignment Revision. It
retains the checked Question Version substitutions, Blueprint Content Digest,
destination Assignment Revision, and import revision after the operation
commits. It is distinct from the browser-safe Assignment Source projection and
from the pre-mutation Assignment Source Snapshot.

**Copy Assignment from Blueprint Receipt** is the immutable server-held
completion receipt for one copied Blueprint Assignment. It binds the exact
Assignment Source Record, resolved Assignment Schedule, command binding, and
resulting Course Instance Snapshot.

**Course Instance Snapshot** is the immutable operation precondition for one
exact Course Instance. It contains the exact Course Schedule Revision Reference
and ordered Assignment Revision References observed by the server; it grants no
authority and is distinct from a Course Instance Creation Reservation.

**Course Instance Creation Reservation** is server-held pre-creation evidence
for one Course Instance. It binds the exact Blueprint or rollover source, target
Course Term, authorizing Account, request digest, Retry Token, and reserved
Course Instance Reference; it creates no authority of its own.

**Blueprint Fork Reservation** is server-held pre-creation evidence for one
Blueprint Course fork. It binds the exact source Blueprint Revision, authorizing
Account, request digest, Retry Token, and reserved Blueprint Course Reference;
it creates no authority of its own.

**Course Time Zone** is the one exact IANA time-zone name owned by a Course
Term. It gives every Instructor-facing Course Local Date and Time its meaning;
validation accepts only a case-sensitive IANA database member.

**Course Date** is one exact proleptic-Gregorian `YYYY-MM-DD` calendar value
inside a Course Term. It is never an instant, UTC offset, or local date-time;
Course Schedule Revisions store Course Dates in database `date` columns.

**Course Local Date and Time** is one Instructor-facing wall-clock value with
millisecond precision. It is resolved only through its Course Term's Course
Time Zone; nonexistent or ambiguous local values are refused rather than
silently converted to an absolute time.

**Course Membership** is one Account's participation episode in one Course
Instance. Its **Course Membership Role** is Instructor or Student; its state is
derived from Course Membership Events as Active or Ended. A current Instructor
Course Membership makes an Account a **Teaching Team Member**. One Teaching
Team Member is the **Assigned Instructor**, the required accountable instructor;
all current Teaching Team Members have equal teaching authority.

**Course Invitation** is an Instructor-issued, target-bound invitation to one
Course Instance with one Course Membership Role. Its **Course Invitation State**
is Pending, Accepted, Declined, Revoked, or Expired. One immutable Course
Invitation Event records the accepted, declined, or revoked terminal transition;
the absence of that event derives Pending or Expired from the exact deadline.

**Course Invitation Email Rule** is the revisioned set of normalized email
domains applied only when an Instructor issues a Course Invitation. It does not
provide self-enrollment or Account Creation authority.

**Student Record** is the stable educational record for one Student Account in
one Course Instance. A Student Course Membership binds to that Student Record.
Re-enrollment starts another membership episode while retaining the same
Student Record and course history.

**Course Observer Relationship** is a separately governed, answer-free,
identity-free, read-only relationship to one Course Instance. It is not a
Course Membership and it is mutually exclusive with an Instructor Course
Membership in that Course Instance. Its state derives from immutable Course
Observer Relationship Events. **Student Observer** remains a future,
separately approved design, requiring a verified disclosure basis, exact field
scope, expiry, revocation, and access history. **Grader** is a future course
relationship for a manual-grading workflow and has no present implementation.

## Content and delivery relationships

**Draft Question** is one private Question lineage inside an Authoring Workspace.
A **Draft Question Revision** is its complete immutable accepted state. Question
Source and Question Grading Material bind to one exact Draft Question Revision.

**Source Object Reference** is the exact private Object ID and SHA-256 checksum
for immutable authored or imported source bytes. It is reproduction evidence
owned by one exact Draft Question Revision, Question Version, Workspace Import,
or Question Attempt Source Record. It identifies bytes; it is not object
delivery authority and never supplies a Student-visible object URL.

**Question Content Block** is one renderable Text, Math, Image, Code, or Table
unit used within a Question. It carries presentation content and accessibility
descriptions rather than correctness. **Question Prompt** is the learner-visible
task and contains ordered Question Content Blocks. Choice bodies, Matching
Prompts, Matching Choices, and released feedback may use the same presentation
primitive while their containing records retain their exact meaning.

**Question Format** identifies the authored or imported representation of a
Question. **PLE Flat Question JSON v2** is the canonical Question Format for
simple static Questions. Native algorithmic source, WeBWorK PG, QTI, H5P, and
iMathAS source snapshots are other registered Question Formats at their exact
adapter boundaries. Question Format remains independent of educational
interaction, execution, and browser presentation.

**Question Type** classifies the educational interaction with the short values
MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. In MATCH, a
**Matching Prompt** is an item to be matched and a **Matching Choice** is a
possible matching response. **Question Generator** is the exact deterministic
definition that derives parameters from a Question Seed. A **Question
Variation** is the resulting parameterized Question state for one exact
Question Version and seed. It retains the exact Question Version, seed, and,
when seeded, the Generator Reference and ordered declared parameters that
produced the presentation in server and cache evidence. The browser receives
only the answer-free Question Version-and-Seed projection required to bind its
presentation. A **Question Presentation** is the answer-free
title, prompt, Question Assets, and Question Response Format derived from one
Question Variation and bound through the issued-presentation record to one
Question Attempt.

**Question Backend** is the server-side adapter selected by the published
Question Source. It declares its capabilities and performs its exact validation,
issue, reproduction, and automated-grading operations. A **Native Question
Implementation** is one versioned first-party implementation registered for an
exact Question Format, Question Type, and optional Question Generator. An
**External Question Provider** is a configured external system used through a
Question Backend for its exact launch, exchange, render, or grade operations.

**Question Presentation** is the answer-free state issued for one exact
Question Attempt. It contains the Question Prompt, presented Question Assets,
**Question Response Format**, **Question Response Control**, and temporary
Response Item References needed by that presentation. Question Response Format
defines the correctness-neutral shape and constraints of an accepted **Student
Response**. Question Response Control names the browser interaction used to
collect that response. The Question Presentation declares the control, keeping
the Student interface independent of Question Type and Question Format.

**Text Response Match Rule** is the public Exact, Case Insensitive, or
Normalized comparison rule declared by a text-bearing Question Response Format.
**Numeric Response Tolerance** is the public Exact, Absolute, Relative, or
Significant-Figure comparison rule declared by a numeric Question Response
Format. Both rules describe accepted-response behavior only; the Answer Key
retains the correct text or numeric value on the server.

**Question Submission** is the immutable acceptance event for one Student
Response on one Question Attempt. It owns that accepted response. A **Grading
Result** is a distinct immutable evaluation of one Question Submission and is
bound through the containing Question Attempt to its exact source record.
**Answer**, **Answer Key**, and correctness material name server-held grading
facts. **Assignment Submission** is an explicit finalization of one whole
Assignment Attempt and references its accepted Question Submissions instead of
repeating their Student Responses.

**Question Library** is the single shared, authoritative set of Published
Questions available to every Approved Instructor. **My Questions** is the
current Account's view of Published Questions for which it has the Question
Owner relationship. **My Question Drafts** is its view of Draft Questions it
may edit through an exact Authoring Workspace Owner or Workspace Collaborator
relationship.

**Question Owner** is the Approved Instructor relationship accountable for one
Published Question lineage and its ordinary same-lineage revisions. Question
Library visibility remains shared with every Approved Instructor.

**Starred Questions** is the current Account's view of Published Questions to
which it added a **Question Star**. A Question Star is a visible endorsement;
Approved Instructors may see its Account relationship. Use Question Star and
Starred Questions consistently at UI, API, schema, and code boundaries.
Private organization uses Question Folders, while stored search criteria use
Saved Question Searches. **Watched Questions** is the current Account's private
view of Published Questions to which it added a **Question Watch**. A Question
Watch subscribes that Account to permitted version, fork, improvement, and
impact notices. Question Stars carry visible endorsement; Question Watches
carry notification intent. Existing Question, workspace, course, and Student
relationships continue to supply authority.

**Question Folder** is an Account-owned named organization of references to
Published Questions. A Question may appear in more than one Folder, and Folder
membership supplies organization rather than Question access or ownership. A
**Question Folder Share** is one owner-issued recipient relationship for
answer-free Folder inspection and copying. A
**Saved Question Search** stores normalized Question Search criteria rerun
against the current Question Library; its Edit Number only detects competing
accepted edits.

**Question Curation** is the Instructor workflow for finding, reviewing,
organizing, and improving Questions. It is a workflow or surface label. Its
durable records keep their exact names, including Question Folder, Saved
Question Search, Question Star, Question Watch, and Question Change Proposal.

**Question Search** applies normalized criteria to the current Question
Library. A **Question Summary** is one answer-free published-Question listing.
A **Question Search Result** combines that summary with permitted Question
Statistics. **Question Details** is the expanded answer-free view of one exact
Question Version. **Question Statistics** is the privacy-safe, version-specific
aggregate released from accepted graded Question Attempts. A **Question
Picker** is the shared Instructor control that uses Question Search to select
Published Questions for an Assignment or Blueprint Course; selection supplies
no Question ownership or editing authority.

**Question Publication Readiness** is the calculated complete blocking-issue
set for one exact Draft Question Revision; it is not a lifecycle state.

**Published Question** is a validated Question lineage in the Question Library,
available to every Approved Instructor. **Question Version** is an
immutable published version identified by the exact `(question_id,
version_number)` pair. A Question Publication Event records entry into the
Question Library; a separate Question Version Availability Event records
whether a published version is Available or Archived for selection.

**Question Change Proposal** is one Instructor-owned improvement thread against
a Published Question. A **Question Change Proposal Revision** is one complete,
immutable, numbered proposed change with its exact base Question Version,
publication-validation evidence, semantic impact, and grading impact. A
**Question Change Event** is immutable evidence that opens, merges, or closes
one exact Proposal Revision; it derives the Proposal's Open, Merged, or Closed
state. A merge records the new same-lineage Question Version and succeeds only
when the exact base remains current. A Forced Question Correction has its own
immutable manifest and one corresponding Question Change Event.

**Forced Question Correction Manifest** is the closed, immutable Sysadmin-approved
record for one critical Question Version correction. It binds the flawed and
replacement Question Versions, the reason, and direct exact targets for every
affected Assignment, Assignment Attempt, Issued Question, and Assignment Grade.
It supplies the fixed scope for correction work and its evidence; it is not a
generic remediation payload.

**Assignment** is the stable Course Instance-owned delivery record. An
**Assignment Revision** is one complete immutable teaching definition. Its
direct stored definition includes Assignment Title, lifecycle, Assignment
Instructions, resolved schedule, time and attempt limits, late-work rule,
and Assignment Deadline Rule.
An **Assignment Revision Reference** pairs the Assignment Reference with its
positive Assignment Revision Number. Assignment Workspace edit requests carry
that exact reference as their base revision, rather than an unnamed mutable
aggregate counter.
Accepted editing creates the next Assignment Revision; publication changes
future work only. Existing work stays pinned to its exact revision and Question
Versions.
**Base Assignment Policy** is the complete authored timing, attempt, variation,
navigation, scoring, and Student Feedback Release configuration in that
revision. **Effective Assignment Policy** is the server-calculated result for
one exact Student Record, Assignment Revision, and evaluation time after the
applicable Accommodation Revisions and Student Schedule Adjustments are applied.
Each **Effective Assignment Policy Value** pairs one resolved field value with
the exact Assignment Policy Source that supplied it.

**Assignment Policy Source** explains the specific base policy or direct
Student Accommodation that supplied one Instructor-previewed Effective
Assignment Policy value. Its direct Accommodation form carries only the exact
Student Course Membership and a safe display label. **Assignment Policy Source
Kind** is the identity-free equivalent used where a preview must not expose a
membership or person locator.
A **Student Feedback Release Rule** states when score, correctness, feedback,
solutions, and class statistics become Student-visible.
**Assignment Publication Readiness** is the calculated complete blocking-issue
set for one exact Draft Assignment Revision; it is not a lifecycle state.
An **Assignment Entry Availability** belongs to one top-level Fixed Question or
Question Pool in that revision and is Available or Retired for future Assignment
Attempts. A **Question Pool Candidate Availability** belongs only to one candidate
inside its owning Question Pool. Existing Issued Questions retain their exact
historical source regardless of later availability changes.
An **Assignment Entry Scoring Rule** belongs to one top-level Fixed Question or
Question Pool in an Assignment Revision. It is Normal, Full Credit, Extra
Credit, or Excluded and freezes on each Issued Question.
**Question Pool Selection Rule** is the complete reviewed algorithm and output
ordering for one Question Pool. The separate Question Variation Rule determines
whether a later Assignment Attempt reuses or redraws that pool.
It either retains Questions with fresh Question Seeds, uses Instructor-selected
Question Variants, or redraws Question Pools.

**Assignment Completion Rule** determines whether one Assignment Attempt is
complete. It requires Answer All, All Correct, or Score At Least with its
explicit threshold; it does not select a grade or permit later practice.

**Assignment Attempt Grade Rule** selects the completed Assignment Attempt that
contributes to the Gradebook. It is First, Latest, Highest, or Instructor
Selected, while every other Assignment Attempt remains retained evidence.

**Course Grade Scheme** is the Course Instance's complete, server-calculated
grade configuration. It uses either total points or weighted Grade Categories,
with one final rounding rule and optional letter bands. A **Grade Category**
has its title, weight, order, and Drop Lowest count in that Scheme; included
Assignments refer to its exact Grade Category identity.

**Assignment Attempt Continuation Rule** decides whether another Assignment
Attempt may start after completion. It is Unlimited, Capped by an explicit
additional-Attempt limit, or Closed.

**Assignment Attempt** is one Student Record's pass through one Assignment.
It contains **Issued Questions**. A **Question Attempt** is one Student's work
on an Issued Question. A **Question Submission Receipt** records accepted
response submission. A **Grading Result** records the later evaluation, and an
**Automated Grading Receipt** binds that result to its exact automated
operation. This record path keeps server-only answers, grading material, and
FERPA records out of Student-visible data.

**Student Work Records** collectively names Assignment Attempts, Issued
Questions, Question Attempts, Question Submissions, Grading Results, Events,
and Receipts. The collective term supports documentation and derived views;
each stored fact keeps its exact record name and owner.

**Active Student Course Membership** is the prerequisite decision that one
Student Record currently belongs to one exact Course Instance. It authorizes
later Assignment evaluation but neither opens an Assignment nor supplies its
schedule or late-work result.

**Assignment Access** is the server-calculated decision whether one Student
Record may use one Assignment at a given time. It applies the Active Student
Course Membership prerequisite, Assignment lifecycle, action authority, and
Effective Assignment Policy. It returns an exact denial reason when access is
absent and an **Assignment Start Decision** when access is otherwise allowed.
An Assignment Start Decision is May Start, Not Yet Available, Closed, Attempt
Limit Reached, or Late Work Refused. **Student Late Work Status** exists only
for work that may start: On Time, Accepted Late, or Marked Late.

**Selected Student** is the real Student selected by an authorized Instructor
from one Course Instance roster. A **Student View Scenario** is the separate,
identity-free input and result used to evaluate a hypothetical or derived
Student view. The scenario carries no Student Record, membership, account, or
other person locator.

**Accommodation Application Rule** states how an authorized direct Student
Accommodation combines with the Base Assignment Policy. **Extend Only** permits
only changes that widen a Student's available time or limits; **Replace**
applies the authorized Accommodation value directly. The rule is applied
consistently to every adjusted Assignment policy field.

**Accommodation Adjustment** is the closed set of specific available, due,
close, time-limit, and attempt-limit values supplied by one Accommodation. An
adjustment records a specific value, Unrestricted, or inheritance for each
field; it never changes Assignment-owned late-work or Assignment Deadline Rule.

**Assignment Workspace** is the Instructor editing surface for an Assignment
and its exact Revisions, policies, readiness, and Student View. It is an
interface name; Assignment and Assignment Revision remain the durable records.

**Course Appearance** is one Course Instance's revisioned Course Theme, Course
Banner, and banner alternative text. A Course Appearance name describes that
exact visual configuration rather than a general theme registry.

## Stored Question data

**Stored Question Fixture Set** is an explicit data-file set of authored
Questions used to validate Question behavior. **Pilot Question Set** is the
corresponding explicit data-file set for one named pilot workflow. Product
operation retrieves authored Question records through their owning PostgreSQL
and object-storage boundaries. Executable source owns behavior: it loads,
transforms, and validates the stored records.

## Interface surfaces and ribbon navigation

This section defines canonical surface names and their semantic ownership.
[UI_DESIGN_GUIDE.md](UI_DESIGN_GUIDE.md) owns placement, geometry, rendering,
and interaction behavior.

**Application Shell** is the persistent frame around the current PLE content
region. It owns the **Ribbon**, presentation settings, and the content origin.
Route content renders inside that frame.

**Ribbon** is the Application Shell-owned navigation surface. It persists
while route content changes and has one stable **Ribbon Schema** for each
combination of **Ribbon Scope** and **Product Role**. Every Product Role uses
the same Ribbon architecture with its own distinct menu. A page supplies its
task heading and workflow content inside the content region.

**Ribbon Schema** is the predefined ordered set of Ribbon Slots and Ribbon
Tasks selected by one Ribbon Scope and Product Role pair. **Ribbon Scope** is
the exact product context. The closed scopes are:

- **Product Ribbon Scope** for navigation across PLE surfaces without one
  selected Course Instance or Assignment Attempt.
- **Course Instance Ribbon Scope** for one live Course Instance.
- **Assignment Attempt Ribbon Scope** for one Student's exact Assignment
  Attempt.

Ribbon Scope and Product Role select the Ribbon Schema. Exact domain
relationships supply presentation availability. The current route supplies
selection. Loaded records supply labels and course appearance. Server and
Store boundaries continue to authorize every protected operation. Because
Product Role is immutable, one Account uses one Ribbon Schema for each Ribbon
Scope throughout its Authenticated Session.

**Ribbon Context Row** is the fixed row that identifies PLE, the current
Course Instance or Assignment Attempt when present, and the current Account
and Profile controls. A **Ribbon Context Control** is a utility destination
owned by the Context Row rather than a Ribbon Slot. Account and Profile are
Ribbon Context Controls. Context labels remain separate from the page's task
heading.

**Ribbon Tab Row** contains the primary **Ribbon Tabs** for the current Ribbon
Schema. A Ribbon Tab is a navigation link to one primary destination. The
**Selected Ribbon Tab** is the tab whose destination matches the current route.
A route reached through a Ribbon Context Control may have **No Selected Ribbon
Tab**; the selected Ribbon Schema remains present with no Tab selected. Account
Security, Instructor Course Invitations, and Sign In are Context Control routes
that use this state.

**Ribbon Task Row** contains secondary **Ribbon Tasks** for the Selected
Ribbon Tab. A Ribbon Task is a navigation link to one task-specific
destination, such as Overview, Questions, Policies, Grading Operations, or
Student View for an Assignment. A **Ribbon Task Area** is a presentation-only
heading for adjacent Ribbon Tasks with one shared purpose.

**Page Action** is a control that performs an operation on the current
content, such as Create Assignment, Save, Publish, or Submit. Page Actions live
with the content they affect. Ribbon Tabs and Ribbon Tasks navigate; Page
Actions perform operations.

**Ribbon Slot** is one stable ordered position in a predefined Ribbon Schema.
Its **Ribbon Availability** is one of:

- **Available** when current presentation facts make the destination
  appropriate to show as a live link.
- **Checking** while the exact relationship facts needed for presentation are
  loading.
- **Unavailable** when the known relationship excludes that destination from
  the current Ribbon.

Selection and loading are separate from Ribbon Availability. **Selected**
means the control's destination is the current route. **Loading** means a
navigation to that destination is still in progress. **Active** remains a
domain-state term for records such as Accounts and Course Memberships.

**Content Layout** is the route-selected composition below the Ribbon.
**Reading Layout** uses a bounded line length for prose. **Full-width Layout**
uses the available content width for the Question Library, teaching workspaces,
and dense records.

Canonical Product destination names are **Courses**, **Question Library**,
**Blueprint Courses**, and **Instructor Approvals**. Courses is the current
Account's Course Instance surface. Instructor Approvals is the Sysadmin surface
for Instructor vetting and Account approval. Account and Profile remain Ribbon
Context Controls.

Canonical Question Library view names are **All Questions**, **My Questions**,
**My Question Drafts**, **Starred**, and **Watched**. All Questions means every
Published Question available through the Question Library. My means ownership,
Draft means publication state, Starred means a Question Star relationship, and
Watched means a Question Watch relationship. Folders, tags, classifications,
Saved Question Searches, and search facets are organizational mechanisms within
these views.

Canonical Course Instance destination names are **Assignments**, **Students**,
**Gradebook**, **Teaching Operations**, **Blueprint Updates**, and **Course
Setup**. Teaching Operations names the teaching and course-lifecycle surface.
Blueprint Updates names reviewed changes from the parent Blueprint Course.
Course Setup names Course Instance configuration; **Grade Settings** names its
grade-calculation configuration and **Appearance** names Course Appearance.
**Create Assignment** is a Page Action.

Canonical Assignment Attempt labels are **Attempt**, **Back to Assignments**,
and **Assignment Attempt Progress**. Attempt names the Student's current
Assignment Attempt surface. Back to Assignments names its course-Assignment
navigation destination. Assignment Attempt Progress names the current Question
position. Question positions are Attempt content rather than Ribbon navigation.

## Authority and inheritance paths

Authority is derived through exact stored relationships. These paths name the
ordinary sources of PLE authority:

| Capability                            | Required path                                                                                                                                        |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Authenticate                          | Active Account -> Authenticated Session                                                                                                              |
| Question Library                      | Authenticated Session -> Active Approved Instructor Account -> Published Question                                                                    |
| Private authoring                     | Authenticated Session -> Active Approved Instructor Account -> exact Authoring Workspace ownership or Workspace Collaborator relationship            |
| Draft Blueprint Revision contribution | Authenticated Session -> Active Approved Instructor Account -> current Blueprint Collaborator relationship -> exact Draft Blueprint Revision         |
| Teach a Course Instance               | Authenticated Session -> Active Approved Instructor Account -> active Instructor Course Membership -> Course Instance                                |
| Student course work                   | Authenticated Session -> Active Student Account -> active Student Course Membership -> Student Record -> Assignment Attempt -> exact Published Assignment Revision -> Issued Question -> Question Attempt |
| Student FERPA information             | exact Student Record and Course Instance relationship, limited to the approved viewer and requested record scope                                     |
| Course observation                    | Authenticated Session -> Active Approved Instructor Account -> current Course Observer Relationship -> Course Instance, within its closed read scope |
| System administration                 | Authenticated Session -> Active Sysadmin Account -> exact audited support operation; general Sysadmin status does not provide general FERPA access   |

The arrows show inheritance, not merely convenient joins. A caller may receive
only the records and fields supported by the complete path. A direct
relationship is required whenever an operation crosses into Student work,
private authoring, or a specific Course Instance.

## Distinctions that preserve the model

- Product Role classifies a global Account; Course Membership Role describes
  participation in one Course Instance. They never substitute for one another.
- Authentication identifies an Account; authorization follows the exact domain
  relationship from that Account.
- A Course Instance inherits reusable structure from an exact Blueprint
  Revision, then owns its own delivery facts and Student records.
- An Assignment owns delivery definition through immutable Assignment
  Revisions; an Assignment Attempt owns one Student Record's activity.
- Publication is historical entry into shared availability; current selection
  availability is a separate fact.
- A human-readable Reference or product ID locates a record. The exact stored
  relationship, state, and scope authorize an operation.

## Applying the contract

For each change, first identify the product noun, then the owning record, then
the exact relationship that supplies authority. Use the resulting term across
schema, API, code, tests, and documentation together. Keep evidence records
specific to the operation they prove. Record settled implementation decisions
in [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md), while owner choices remain in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).
