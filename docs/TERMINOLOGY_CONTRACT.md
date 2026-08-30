# PLE terminology contract

> When in doubt choose precise instructor language over coder or student language choices.

This is PLE's canonical semantic vocabulary. It defines the meaning and
distinctions of PLE-owned terms used in documentation, schema, code, contracts,
APIs, tests, and UI.

[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remains higher authority for owner
product decisions and plain-language intent. [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md)
remains higher authority for identifier spelling, casing, and boundary-specific
form. Subject to those two authorities, this contract supersedes every other
file under `docs/` for the meaning of PLE-owned terms.

Other documents own implementation detail within this vocabulary. For example,
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) owns physical schema,
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) owns PostgreSQL
authorization predicates, and [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)
owns sensitivity, disclosure, and retention. The active plans and
[implementation_status.md](active_plans/implementation_status.md) own scope,
implementation state, and acceptance evidence. All specialized documents use
the meanings defined here.

## Authority rules

- Use the defined canonical term directly.
- A specialized document may add a local implementation name, but it must link
  to this contract and preserve the term's meaning.
- Registered external protocol terms retain their owner spelling inside their
  narrow adapter or protocol context.
- Prefer precise Instructor assessment language when a product term and a
  programming shorthand differ. Use implementation jargon where a distinct
  technical fact requires it.

## Identity and authority

### Account

An **Account** is one global PLE login identity with exactly one immutable
product role: **Student**, **Instructor**, or **Sysadmin**. A person who needs
more than one product role uses separate Accounts.

An Account establishes identity and Product Role classification. Exact stored
relationships establish eligibility and authorize course, Student-record,
workspace, support, worker, and object access.

**Account Creation** is the Sysadmin-owned lifecycle action that creates one
Account and fixes its Product Role. Initial platform setup establishes the first
Sysadmin Account. Email-code and passkey authentication open a session for an
existing Account.

### Product Role and Course Membership Role

A **Product Role** is the Account's immutable global classification: Student,
Instructor, or Sysadmin. A **Course Membership Role** is the Student or
Instructor relationship that one Account has inside one Course Instance. The
two roles must agree, but they answer different questions: Product Role says
which kind of Account this is, while Course Membership Role says how that
Account participates in one exact course.

Use the qualified terms at cross-layer boundaries. Bare **role** is permitted
only where the enclosing Account or Course Membership type makes the meaning
unambiguous. A **Database Role** is a server capability principal with a
separate technical meaning.

### Instructor Approval

An **Instructor Approval** is the revocable record that an Instructor Account
has passed real-person review and currently may use shared Instructor
capabilities. Product Role states that the Account is an Instructor Account;
Instructor Approval supplies current global Instructor eligibility; direct
Instructor Course Membership supplies teaching authority for one Course
Instance.

### Authenticated Session

An **Authenticated Session** is the server-side record of one current opaque,
revocable login to one Account. Session resolution supplies trusted Account and
session facts. The exact stored relationship for an operation supplies
protected-resource access.

### Authentication Email

An **Authentication Email** is the private, verified, mutable email credential
for one existing Account's passwordless sign-in. Its sole role is helping the
server resolve that Account during an email-code ceremony. A server-owned
verified-email change replaces it.

### Email Authentication Challenge and Passkey

An **Email Authentication Challenge** is one short-lived, single-purpose proof
that the current browser can receive a code at an Authentication Email. A
successful challenge creates an Authenticated Session or authorizes the
explicit email-change flow according to its recorded purpose.

A **Passkey** is a revocable credential registered to one Account and used to
authenticate a new session. **WebAuthn ceremony** is the registered protocol
term for its bounded registration or authentication exchange; it stays inside
the authentication adapter. Product authority continues to come from exact
stored relationships.

### Course Membership, Course Enrollment, and Student Record

A **Course Membership** is an Account's direct relationship to one
**Course Instance**. Current memberships are Student or Instructor memberships.
Current approved-Instructor state plus direct Instructor membership gives equal
teaching authority for that course.

A **Course Enrollment** is the act of establishing a new active Student Course
Membership and binding it to that Student's course-scoped Student Record.
Revocation ends that membership episode; re-enrollment creates a new membership
episode and preserves the existing Student Record and its history. Use
**enrolled Student** for the current state. Course Enrollment solely
establishes the course-level membership episode.

A **Student Record** is the course-scoped educational-record relationship for
one Student. It is a logical authorization relationship, independent of one
particular database shape. Student work derives from the exact Course Instance,
active Student Course Membership, and Student Record ownership chain. Product
Role establishes eligibility for that relationship.

### Course Observer and Student Observer

A **Course Observer** is an Account with an explicit read-only relationship to
one Course Instance. The relationship supplies course content, Assignments,
Questions, and Student completion while keeping individual scores private. A
**Student Observer** is an Account with an explicit read-only relationship to
one Student Record after the required FERPA rights are established. Each
observer relationship supplies its stated read projection.

### Sysadmin Support Capability

A **Sysadmin Support Capability** is a purpose-bound, time-bounded, revocable,
and auditable grant for one repair operation and exact target. That capability
supplies the Sysadmin's FERPA access for the operation.

### How identity and access begin

```text
Initial platform setup -> first Sysadmin Account + fixed Product Role

Sysadmin Account Creation -> Account + fixed Product Role

Real-person Instructor review -> Instructor Approval
  -> current global Instructor eligibility

Course Invitation acceptance by an existing Student Account
  -> Course Enrollment -> Student Course Membership + Student Record

Course Instance creation for an Approved Instructor
  -> Instructor Course Membership

Email-code or Passkey authentication for an existing Account
  -> Authenticated Session
```

These paths have different trust origins and converge on the same Account and
Authenticated Session model. Account Creation fixes which kind of Account
exists. Instructor Approval supplies revocable Instructor eligibility. Course
Enrollment and Instructor Course Membership establish exact course
relationships. Authentication creates the session used to resolve those
server-held facts.

## Authority paths

```text
Authenticated Session -> Account identity and immutable role

Account + current approved-Instructor state + Instructor Course Membership
  -> Course Instance teaching authority

Account + active Student Course Membership + Student Record + Course Instance
  -> Student Record authority

Account + Authoring Workspace Owner or Collaborating Instructor relationship
  -> private draft-authoring authority

Sysadmin Account + Sysadmin Support Capability
  -> bounded repair authority
```

An identifier names a candidate record. The schema, Rust contracts, wire
models, routes, and tests preserve the applicable authority path and its
stopping point.

## Assignment activity hierarchy

PLE separates mutable Assignment composition from immutable Student activity:

```text
Assignment
  -> Assignment Entry (Fixed Question or Question Pool)

Student Record + Assignment
  -> Assignment Attempt
       -> Assignment Submission (when explicit finalization applies)
       -> Issued Question
            -> Question Attempt
                 -> Question Submission
                      -> Student Answer
```

### Assignment

An **Assignment** is a Course Instance-owned delivery definition. It selects
exact published question versions and owns teaching policy, release, and
delivery configuration. Student work attaches through Student Assignment
Attempts that bind the Student Record and Assignment directly.

### Assignment Attempt

An **Assignment Attempt** is one Student's complete pass through one Assignment.
It binds one exact Student Record and Assignment in the same Course Instance
and contains the ordered Issued Questions for that pass. Repeated practice
creates another Assignment Attempt and preserves every earlier one. It owns
pass-level variation, completion, and timing facts.

Use **Assignment Attempt** in product language, documentation, UI, contracts,
and implementation.

### Issued Question

An **Issued Question** is one immutable concrete question selected and ordered
for one Assignment Attempt. It binds the source Assignment Entry, exact Question
Version, delivery position, points and scoring treatment, statistics
eligibility, and pool-selection evidence when applicable. It freezes what the
Assignment Attempt contains as the Assignment evolves.

Every Question Attempt references one exact Issued Question. That
relationship carries the complete selection and ordering evidence.

### Question Attempt

A **Question Attempt** is one Student's server-issued try at one Issued
Question. It binds the variation seed, answer-free presentation, timing state,
grading backend, and source evidence needed for that try. Repeating the same
concrete question creates another Question Attempt under the same Issued
Question. A Student supplies an answer through a Question Attempt.

Use the qualified **Question Attempt** term wherever Assignment Attempt could
also be meant.

### Question Submission, Assignment Submission, and Student Answer

A **Question Submission** is an immutable, idempotent acceptance event and
evidence record for one Question Attempt. It commits one Student Answer for
server-side grading and may retain the accepted Student Answer privately, its receipt,
and its grading status. Browser drafts remain browser state, and Grading Results
record grades separately.

An **Assignment Submission** is an immutable, idempotent finalization event for
one whole Assignment Attempt. It contains finalization metadata while each
Student Answer remains in its Question Submission. It records the Student's explicit
decision to finish the Assignment Attempt and closes later work according to
the Assignment's policy.

**Assignment Attempt Completion** is the server-derived result of applying the
Assignment Completion Rule. Mastery or practice may complete from Question
Submissions; an exam or quiz may require an explicit Assignment Submission. Store
completion and current Assignment Attempt state separately from an optional
Assignment Submission.

A **Student Answer** is the learner-supplied answer data accepted within a
Question Submission. Server validation establishes acceptance, and stored
relationships establish authority. In Instructor-facing language, say a
Student **submits an answer**. Use Question Submission when referring to the
accepted event, receipt, or evidence record.

Use the qualified **Question Submission** and **Assignment Submission** terms at
every cross-layer boundary. A local Question Attempt context may use
**submission** where its subject remains unambiguous.

Use stored references and referential constraints to close the persistent
relationship chain:

- Assignment Attempt binds one exact Student Record and Assignment in the same
  Course Instance.
- Issued Question belongs to that Assignment Attempt and freezes
  the exact source Assignment Entry or pool candidate plus Question Version.
- Question Attempt belongs to that Issued Question.
- Question Submission belongs to that Question Attempt.
- Assignment Submission, when present, belongs directly to the Assignment
  Attempt; Student Answer data remains under Question Submission.

Duplicated course, Student, or Assignment references used for row-level
authorization or query shape participate in exact composite foreign keys.
The complete stored relationship chain proves that a Question Attempt belongs
to an Assignment.

## Content and course terms

### Question, Question Corpus, Question Catalog, and Library

A **Question** is one stable educational-question lineage in the global
**Question Corpus**. A **Question Version** is one immutable reviewed version
within that Question. An Assignment always pins an exact Question Version; it
changes that pin only through an explicit reviewed Assignment update.

The **Question Corpus** is the complete global collection of Published
Questions available to Approved Instructors. The **Question Catalog** is the
answer-free, searchable and filterable projection of that corpus. **Library**
is the Instructor-facing interface for browsing the Question Catalog. The
Corpus remains the collection, the Catalog remains its discovery projection,
and exact Instructor relationships remain the authorization model.
Use **Question Corpus** when discussing the content set, **Question Catalog**
when discussing discovery data or queries, and **Library** only for the user
interface.

A **Published Question** is validated, answer-free Question Corpus content
available to Approved Instructors. A
**Draft Question** is private unpublished question work visible only through its
exact workspace relationship until publication validation succeeds.

Publication has one visibility contract: every Published Question is available
to every Approved Instructor. Question Catalog authority belongs to Approved
Instructors.

### Question Version Availability

**Question Version Availability** is the Instructor-facing status that controls
ordinary discovery and new selection after publication:

- **Available** versions are discoverable and eligible for ordinary new
  Assignment selection.
- **Archived** versions remain exactly resolvable as historical evidence, with
  a reason, and stay outside ordinary discovery and new Assignment selection.

Existing Assignment pins and historical evidence resolve their exact Question
Version in every status. Availability preserves the immutable version content.
**Published** describes how content entered the Question Corpus;
**Available** describes whether Instructors may select it for new work.

### Question stewardship and curation

A **Question Change Proposal** is an Instructor's proposed change to one exact
Question Version. Its acceptance makes a new Question Version in the same
Question lineage. A **Question Fork** creates a distinct Question lineage for
a different educational objective, task, Question Type, or purpose. A
**Forced Question Correction** is the separate audited Sysadmin path for a
critical security or correctness flaw.

A **Forced Question Correction Manifest** is the immutable closed correction
scope: flawed and replacement Question Versions, reason, remediation rule,
generation, and exact affected teaching references. It authorizes only its
separately approved correction workflow and preserves original questions,
Student Answers, scores, and Receipts. **Correction Recalculation Evidence** records
the outcome for one affected Course Instance and generation. The Manifest
supplies the closed correction scope.

A **Question Owner** is the Approved Instructor responsible for deciding
same-lineage moderate edits and Question Change Proposals for one Question.
Every Approved Instructor retains visibility of the Published Question.
**Question Authorship** is the reviewed public attribution and contributor
history attached to a Question Version. It supplies evidence and credit, while
exact stored relationships supply authorization.

A **Question Star** is an Approved Instructor's visible endorsement of one
Question lineage. Approved Instructors may see its count and the Approved
Instructor identities behind it. A **Question Watch** is the watching
Instructor's private subscription to version, fork, improvement, and impact
notices for one Question lineage or exact Question Version. Catalog, authoring,
course, and grading authority continue to come from their exact relationships.

**Question Curation** is an Instructor's private organization and discovery of
Published Questions through Stars, Watches, Instructor Collections, and Saved
Catalog Searches. Its scope is private organization and discovery.

A **Question ID** is the stable, copyable identifier for one Question lineage.
It is an established, Question-specific product term. A Question Version has
one server-only identity. Its parent
relationship necessarily identifies the Question lineage, so exact version
selection uses one server-only identity for the immutable version.

### Question source, format, backend, type, and response

A **Question Source** is the private authored or imported material needed to
validate and reproduce a Question Version. It may contain grading-sensitive
content and remains in server-side authoring and grading boundaries. A
**Question Format** is the authored, imported, or exported representation, such
as PLE flat-question JSON, WeBWorK PG/PGML, a QTI package profile, or H5P. A
**Question Backend** is the server-side adapter that issues, renders,
and, when supported, grades that source.

A **Question Type** is the learner interaction and answer structure. The native
PLE flat-question types are:

| Question Type | Meaning |
| ------------- | ------- |
| **MC** | Multiple Choice |
| **MA** | Multiple Answer |
| **FIB** | Fill-in-the-Blank |
| **MULTI-FIB** | Multi-Part Fill-in-the-Blank |
| **NUM** | Numerical Entry |
| **MATCH** | Matching |
| **ORDER** | Ordered List |
| **HOTSPOT** | Hotspot |

These compact labels are product terminology. Source and schema names apply
their language-specific spelling rules independently. An External Question
Provider declares one registered Question Type when its contract supports that
mapping; Question Backend names the integration itself. The
[qti-package-maker taxonomy](https://github.com/vosslab/qti-package-maker)
supplies the shared assessment labels and reserves **format** for file and
package representations.

**Answer Format** is the server-owned shape and validation contract for the
Student Answer accepted by one Issued Question. It carries the permitted rendered
item references, cardinality, numeric constraints, and other answer-shape rules
needed by that exact Question Version and issued presentation. **Answer
Control** is the learner-facing browser interaction that collects a Student Answer
matching that format. The answer-free presentation selects the control after
the server resolves the Issued Question.

Question Format, Question Backend, Question Type, Answer Format, and Answer
Control are independent axes. Use the qualified term that states the intended axis. Registered names
such as WeBWorK, QTI, H5P, WebAuthn, and LTI keep their external spelling only
inside their adapters and documentation.

### Course and assignment structure

A **Blueprint Course** contains reusable answer-free course structure. A
**Course Instance** is a teaching course created from one Blueprint Course. It
owns Students, membership, live deadlines, Student Records, Assignments,
Assignment Attempts, Question Attempts, Question Submissions, Assignment
Submissions, feedback, grades, and delivery settings.

Use the spaced forms **Blueprint Course** and **Course Instance** in product
language, documentation, and UI. Bare **course** means Course Instance where
the teaching context is already explicit. Blueprint Course is the one reusable
course aggregate. One-assignment reuse is a bounded module-and-Assignment
projection of a Blueprint Course.

A **Blueprint Course Owner** is the Approved Instructor responsible for one
Blueprint Course's draft, publication, fork, and update decisions. Published
Blueprint Courses remain reusable by every Approved Instructor.

A **Blueprint Revision** is one immutable ordered version of a Blueprint Course.
A **Blueprint Publication** is the explicit transition that makes one reviewed,
answer-free Blueprint Revision reusable by every Approved Instructor. A draft
Blueprint Course remains private to its exact Blueprint Course Owner and
Collaborating Instructor relationships. Publication shares the reusable
structure with Approved Instructors while Course Membership continues to govern
live teaching.

A **Blueprint Adoption** is an Instructor workflow that deliberately applies
reusable content from one exact Blueprint Revision to a destination. Adoption
is the decision; the resulting Course Origin or Assignment Source is its
durable evidence. It copies reusable answer-free teaching structure and
preserves Student Records, grades, responses, and live delivery state in the
Course Instance.

A **Course Origin** is the immutable evidence of how a Course Instance was
created. It always records the exact parent Blueprint Course and Blueprint
Revision; rollover also records its exact source Course Instance. An
**Assignment Source** is the immutable source evidence for one Course
Instance-owned Assignment, including the exact Blueprint Assignment and
Blueprint Revision from which it was adopted,
updated, or copied. Later Assignment adoption or update may change Assignment
Source while preserving Course Origin.

A **Blueprint Fork** creates a new independently owned Blueprint Course from an
exact source revision and retains immutable source history under independent
ownership. A **Blueprint Instantiation** creates a new Course Instance from an
exact Blueprint Revision. An **Assignment Adoption** materializes one selected
Blueprint Assignment as a new Assignment in an existing Course Instance. A
**Course Rollover** creates a new Course Instance for a later Course Term from
reviewed reusable teaching state. A **Course Term Shift** deliberately resolves
unissued schedules against a different Course Term. A **Controlled Blueprint
Update** applies selected later Blueprint meaning to an existing daughter
Course Instance.

A **Selected Assignment Copy** creates a new Assignment from a deliberately
selected Blueprint Assignment when an existing locally changed Assignment must
remain intact and copies only the selected reusable teaching structure. These
operations preserve Course Memberships, Student Records, attempts, responses,
grades, accommodations, retention state, and the Assignment's existing release
decision.

A **Course Term** is one Course Instance's inclusive start date, inclusive end
date, and authoritative IANA time zone. The three values form one scheduling
contract and supply the live course's scheduling basis.

A **Blueprint Assignment** is one ordered reusable Assignment definition inside
a Blueprint Course. It owns reusable question selection, instructions, policy
defaults, and relative schedule intent. Release, live deadlines, Student
Records, and Student work belong to the Course Instance. Applying a Blueprint
Assignment materializes a distinct Course Instance-owned Assignment with
immutable source history.

Use this complete ownership sequence: the Course Instance-owned Assignment is
the live teaching definition, and an Assignment Attempt binds Student-specific
work directly to its Student Record and Assignment.

An **Assignment Entry** is one ordered composition node in an Assignment. It is
exactly one **Fixed Question** or one **Question Pool**. A Fixed Question pins
one exact Question Version. A Question Pool owns exact-version candidates, a
draw count, and a deterministic ordering rule. Reusing the same Question
Version in multiple entries preserves separate entry identities.

At Assignment Attempt start, the server expands the Assignment Entries into
immutable Issued Questions. A Fixed Question contributes itself; a
Question Pool contributes its selected candidates and pool-selection evidence.
This keeps mutable authoring structure, issue-time selection evidence, and
repeated Question Attempts at three distinct levels.

An **Assignment Lifecycle** is the Instructor-controlled transition model among
Draft, Published, Closed, and Archived. **Assignment State** is one Assignment's
current value on that lifecycle. Only a Published Assignment may offer
Assignment Access; a Course Instance's release and schedule rules can still
withhold that access.

An **Assignment Release** is the explicit Course Instance decision that makes a
Published Assignment eligible for Student delivery, subject to its audience,
schedule, and accommodations. **Assignment Delivery** means the Assignment's
release, resolved schedule, and explicit local divergence responsibility; it is
one responsibility within the Assignment. Publication, release, and current
Assignment Access remain three distinct decisions.

An Assignment Attempt binds Student work directly to its Student Record and
Assignment. An Assignment Grade binds the same pair and selects the contributing
Assignment Attempt according to the Assignment Attempt Grade Rule.

### Assignment access and policy

**Assignment Access** is the server's current decision that a Student may view,
start, resume, or submit work for an Assignment. It depends on exact Student
Record ownership, Assignment audience, lifecycle, schedule, and applicable
accommodations. Access is a server decision evaluated from those facts.

An **Assignment Audience** identifies the course-wide or Course Group-defined
Students to whom an Assignment is offered. A **Course Group** is a named
course-local grouping for one documented teaching purpose; exact access rules
continue to authorize Student work. An **Accommodation** is an authorized group or
individual adjustment to a Student's Assignment Access window or limit. It
preserves the Assignment's question and grading rule and every other Student's
record.

An **Assignment Access Window** is the Assignment's available, due, and close
schedule. An **Assignment Attempt Limit** is the maximum number of whole
Assignment Attempts a Student may start. A **Question Attempt Limit** is the
maximum number of Question Attempts permitted for one Issued Question within
an Assignment Attempt.
Use the qualified limit term at every shared boundary.

An **Assignment Completion Rule** states what makes an Assignment Attempt
complete. An **Assignment Finalization Rule** states whether the Student must
make an explicit Assignment Submission and what work becomes immutable when it
is accepted. Completion and finalization are separate decisions: a practice
Assignment may complete from Question Submissions alone, while an exam may
require whole-Assignment finalization. An **Assignment Attempt Grade Rule**
selects which completed Assignment Attempt contributes to the Gradebook. An
**Assignment Attempt
Continuation Rule** states whether and how later practice Assignment Attempts
may begin. A **Question Variation Rule** states what changes in those later
Assignment Attempts. A **Student Feedback Release Rule** states when score,
correctness, feedback, solutions, or class statistics become visible.

An **Assignment Activity** is an Instructor-facing named teaching configuration
such as Mastery, Practice, or Exam. It expands to explicit completion,
Assignment Attempt grading, continuation, variation, timing, and Student
Feedback Release Rules. An **Assignment Activity** configures the Assignment;
each Assignment Attempt stores the resulting activity facts. A **Mastery
Assignment** uses repeated practice, fresh variation in later Assignment
Attempts, immediate educational feedback, and a
highest-score Gradebook rule. Continued-practice rules keep later practice
available after a perfect Assignment Attempt.

### Presentation, grading, and evidence

A **Question Presentation** is the exact answer-free question state issued for
one Question Attempt. It includes only the prompt, permitted assets, answer-free
Answer Format, Answer Control description, and presentation binding needed
to answer that Question Attempt. It is
an answer-free projection bound to that exact Question Attempt.

A **Question Submission Receipt** is the immutable evidence returned for an
accepted Question Submission. An **Assignment Submission Receipt** is the
corresponding evidence for accepted whole-attempt finalization. A **Grading
Result** is the server-generated correctness and score outcome for a Question
Submission. **Feedback** is the policy-released teaching explanation or result
projection. Answer keys and Grades remain separately governed records.

A **Student Feedback Release** is the immutable record of the feedback
projection made visible for one accepted Question Submission at one time. It
records the result of applying the Student Feedback Release Rule. The Rule
governs later releases.

A **Score** is a numeric outcome for one Question Attempt or Assignment Attempt.
An **Assignment Grade** is the Gradebook's selected course-record result for one
Assignment, determined by its Assignment Attempt Grade Rule. Use **Score** for
the numeric outcome and **Grade** for the selected course record.

A **Gradebook** is the Instructor-facing course record of Assignment grades.
It uses the Assignment Attempt Grade Rule and server-generated Grading Results;
server-generated Grading Results supply its scores.

A **Gradebook Snapshot** is one immutable, calculated point-in-time projection
for one Student Record, Assignment, and scoring generation. It is evidence used
to explain or reproduce the current Gradebook result from the underlying
Question Submissions and Grading Results.

### Analysis and statistics

An **Assignment Analysis** is one Instructor-facing, calculated analysis of an
Assignment for one scoring generation. It summarizes Assignment Attempt
completion and outcomes. The Gradebook remains the course-record owner for
Assignment Grades.

An **Assignment Item Analysis** is the per-question part of an Assignment
Analysis. It analyzes Student Answers and outcomes by source Assignment Entry and the
exact Question Versions selected into Issued Questions. **Item
analysis** is the established assessment-analysis term.

**Question Statistics** are identity-free global aggregates for one exact
Question Version, such as accepted graded Question Attempt count, correct count,
and eligible response-choice counts. Privacy-safe Question-lineage rollups may
combine versions only when labeled and after the disclosure threshold is met.
Question Statistics contain only identity-free aggregates and remain separate
from Assignment Item Analysis.

**Class Statistics** are the bounded Student-visible projection of current
course-local analysis permitted by the Assignment's Student Feedback Release
Rule. They contain only the policy-approved course-local aggregate.

### Authoring and teaching relationships

An **Authoring Workspace** is the private root for Draft Questions, imports,
and authoring assets. An **Authoring Workspace Owner** is the Approved Instructor
responsible for that workspace. A **Collaborating Instructor** is an Approved
Instructor with an explicit relationship to contribute within that workspace.
An **Instructor Collection** is one Instructor's private organization of shared
Questions. Question Catalog relationships continue to govern visibility. A
**Saved Catalog Search** stores one Instructor's search definition and reruns it
against the current Catalog.

A **Workspace Import** is a private staged external-content import inside one
Authoring Workspace. It remains private source evidence until separately
validated and published as a Question. Publication creates the Question and its
Catalog visibility.

A **Course Roster Import** is a staged batch of proposed Student Course
Enrollments or Course Invitations for one exact Course Instance. Commit uses
the canonical Account, Student Record, Course Membership, and invitation
pathways.

An **Approved Instructor** is an Instructor Account with current Instructor
Approval. Instructor Approval supplies global eligibility, while direct
Course Membership supplies teaching authority for a Course Instance. A
**Teaching Team** is the set of equal current
Instructor Course Memberships for one Course Instance. An **Assigned
Instructor** is the required accountable current Instructor member named when a
Course Instance is created; every current Teaching Team member retains equal
teaching authority. A **Course Invitation** is the revocable offer whose
accepted claim may create one Course Membership.

A **Student View** is the Instructor's answer-free preview of Student-visible
Assignment content and policy outcomes. A preview for a selected Student,
Course Group, or time uses explicit hypothetical inputs and records preview
evidence separately from Student Records, Assignment Attempts, Submissions, and
statistics.

A **Grader** is the reserved human Course Membership Role for future
manual-grading workflows. Its activation belongs to the future manual-grading
design.
A **Database Grading Role** is a server-only technical principal for protected
grading material.

### Operational terms

An **Instructor Grading Operation** is a bounded Instructor-requested server
operation for recalculation, recovery, or inspection. An **Automated Grading
Operation** is the server execution that grades one accepted Question
Submission and commits an immutable Automated Grading Receipt. Use bare
**Grading Operation** only as an explicit umbrella; the two operations have
different initiators, parents, authorization, and receipts.

A **Recalculation** recomputes a derived score, Assignment Grade, Gradebook
Snapshot, or analysis under a new positive **Scoring Generation**. It appends
new result and control evidence while preserving the original Question
Submission, Student Answer, prior Grading Result, and prior Receipt. **Regrade** is
permitted Instructor-facing action text only when the actual operation reruns
grading for accepted work. Use **Recalculation** for other derived aggregate
work.

A **Job** is one server-owned unit of background work. A **Worker Lease** is the
time-bounded exclusive claim that lets one worker execute a Job. These
operational terms refer exclusively to background work.

An **External Question Provider** is a configured server-side integration that
renders or grades a supported Question Version behind a Question Backend. An
**External Tool Launch Session** is its short-lived, Account-, course-, Student
Record-, Assignment-, Question Attempt-, and exact-version-bound launch state. An
**External Tool Exchange** is the server-held verification and commit state for
one provider interaction. Committing the Exchange through the ordinary PLE
boundaries creates any resulting Question Submission, Grading Result, Score, or
Grade. Registered provider and LTI terms such as **score passback** remain
adapter vocabulary.

An **Assignment Export Request** is one Account-attributed request to generate
an exact Assignment export from a frozen Manifest. An **Assignment Export
Artifact** is one generated format output of that request, such as DOCX, PDF,
QTI, or an authorized answer key. A request, Job, Manifest, and Artifact are
separate records. Each Receipt and state transition proves the completed step.

A **Course Retention Plan** is the revision- and generation-bound schedule for
one Course Instance's archive, private-artifact deletion, or Student-record
purge stage. **Archive** withholds ordinary activity while retaining protected
records. **Purge** permanently removes the exact manifested Student records and
private Objects while preserving shared Published Questions, Blueprint Courses,
private Draft Questions, and identity-free Question Statistics. A Retention
Plan supplies the closed targets and timing before Job execution.

### Object storage terms

An **Object** is immutable stored bytes plus the server-created metadata that
binds their storage address, checksum, media type, length, and ownership scope.
The owning database relationship grants access to an Object.

An **Asset** is an Object used as referenced presentation media for a Question,
Course Instance, or Assignment. A **Source Object** retains authored or imported
source material for validation, reproduction, or source history and may be private
or answer-bearing. An **Artifact** is a generated output such as an export,
archive, report, or rendered deliverable. Source, Asset, and Artifact describe
three distinct semantic uses of an Object.

An **Object Reference** is authoritative database state that makes one exact
Object relevant to a Question, workspace, Course Instance, Student Record, or
operation. An **Object Delivery** is the separately authorized mapping that
makes one exact referenced Object retrievable through a bounded route. Object
Delivery is distinct from Assignment Delivery. An **Object Storage Check**
compares referenced Object metadata with the exact stored bytes and records a
verified, missing, or mismatched result. An **Object Cleanup** is a separate,
explicitly authorized operation that deletes or retains the exact Objects in a
closed cleanup scope and records its outcome. Cleanup authority comes from that
separate closed scope.

### State, status, and lifecycle

An entity's **State** is its authoritative current value on one closed
transition axis. A **Lifecycle** is the permitted states and transitions across
that entity's existence. A **Status** is a read projection that explains
current state or progress to a caller. A specialized mutation contract supplies
transition authority.

Qualify these terms by their subject, such as **Assignment Attempt State**,
**Worker Job State**, and **Question Version Availability**. Bare **State**,
**Status**, or **Lifecycle** is acceptable only in a narrow context where the
subject remains unambiguous. Use separate state concepts for publication,
selection availability, execution progress, completion, and visibility.

### Evidence-record terms

Use these terms according to their distinct meanings:

- A **Command** is validated intent to perform one mutation. A later Event or
  Receipt records its accepted result.
- An **Operation** is one bounded workflow that may move through execution
  states and may produce Receipts or Evidence.
- An **Event** is an immutable fact that a named transition or action occurred.
- A **Receipt** is the immutable acknowledged result and binding of an accepted
  Command or committed operation step.
- A **Snapshot** is an immutable point-in-time projection calculated from
  retained source facts.
- A **Manifest** is an immutable, closed list of exact inputs or targets for a
  later operation. It proves the operation's scope; its Receipt proves
  completion.
- **Evidence** is durable data retained to verify, audit, replay, or explain a
  fact. Evidence may include an Event, Receipt, Snapshot, or Manifest, and an
  exact stored relationship supplies its authority scope.

Choose each word from its definition: **Request** for requested intent,
**Receipt** for acknowledged acceptance, **Snapshot** for an immutable
point-in-time projection, **Event** for an occurred fact, and **Manifest** for a
closed work scope.

## Agent check

Before adding a relation, type, contract, route, or test, identify:

1. The canonical term it represents.
2. Its exact parent and complete authority path, if it authorizes anything.
3. Whether it is an Assignment Entry, Assignment Attempt, Issued Question,
   Question Attempt, Question Submission,
   Assignment Submission, or Student Answer.
4. The boundary where that path and meaning stop.

Use the canonical relationship and level as the answer before implementing it.
