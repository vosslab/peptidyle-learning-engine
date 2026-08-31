# PLE terminology contract

This is the concise semantic contract for PLE-owned database, API, test, and
code terminology. It turns the owner glossary in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) into implementation boundaries; it does
not supersede that owner guidance.

Use [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) after selecting the correct
domain term. Use [VOCABULARY_REPLACEMENTS.md](VOCABULARY_REPLACEMENTS.md) to
complete an in-progress correction: the map identifies the old boundary, its
replacement, and the required structural change.

## Authority order

1. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) defines intentional product meaning.
2. This document defines the corresponding shared domain vocabulary and
   relationship paths.
3. [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) defines identifier spelling.
4. A focused contract or schema document defines its physical representation.
5. [VOCABULARY_REPLACEMENTS.md](VOCABULARY_REPLACEMENTS.md) records remaining
   convergence work and is removed as each correction closes.

When a term has a narrower meaning at one boundary, name the narrower record or
relationship. A broad context object never substitutes for a stored authority
path.

## Identity, authentication, and product role

**Account** is one global login identity in the single PLE installation.
Account creation assigns one immutable **Product Role**: **Student**,
**Instructor**, or **Sysadmin**. An Account has an **Account State** derived
from immutable Account State Events: Active, Suspended, or Closed.

**Authenticated Session** is one server-side authentication record for one
Active Account. A successful passkey or email-code authentication creates or
continues an Authenticated Session; suspension or closure revokes its sessions.
`Authenticated Session Reference` identifies that record. A session authenticates
an Account; it grants no course, authoring, catalog, or FERPA authority itself.

Each role-distinct login is a separate Account and consequently follows its own
authenticated-session path. For example, a person acting as both Sysadmin and
Instructor uses a Sysadmin Account for system administration and an Instructor
Account for teaching. This separation makes the product role a stable security
boundary while retaining ordinary passwordless authentication for each Account.

**Instructor Approval** is the current result of immutable Instructor Approval
Events. An Instructor Account requires current approval before it may use
Instructor-only shared-catalog, authoring, or course-creation capabilities.

**Workspace Collaborator** is an Approved Instructor with a current relationship
to one exact Authoring Workspace, derived from immutable start and end Workspace
Collaborator Events. It grants only that private-authoring relationship.

## Course relationships

**Blueprint Course** is a reusable, answer-free course definition. It has no
Students or delivery deadlines. A **Blueprint Revision** is one complete,
immutable authored definition. A **Draft Blueprint Revision** has not yet been
published. A **Blueprint Collaborator** is an Approved Instructor with an
explicit, time-bounded contribution relationship to one exact Draft Blueprint
Revision; it grants neither Authoring Workspace nor Course Instance authority.
A **Blueprint Publication Event** makes one reviewed revision reusable by
Approved Instructors and closes its Draft Blueprint Revision collaboration.
**Blueprint Revision Availability** is the current Available or Archived state
derived from immutable Blueprint Revision Availability Events for one published
revision. It determines ordinary new selection without changing historical
references to that revision.

**Course Instance** is live teaching created from an exact Blueprint Revision.
It owns enrollment, deadlines, releases, accommodations, grades, and other
delivery-specific facts. Course Instance Creation atomically records its source
and an initial Instructor Course Membership.

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

**Question Collection** is an Account-owned ordered organization of shared
Question lineages. A **Question Collection Entry** records one lineage in that
collection. A **Saved Question Search** is an Account-owned normalized Question
Catalog filter rerun against the current catalog; its Edit Number only detects
competing accepted edits.

**Draft Question** is private Instructor-authored material. A **Draft Question
Revision** is its complete immutable revision. **Question Publication
Readiness** is the calculated complete blocking-issue set for one exact Draft
Question Revision; it is not a lifecycle state.

**Published Question** is a validated Question lineage in the shared Question
Corpus, available to every Approved Instructor. **Question Version** is an
immutable published version identified by the exact `(question_id,
version_number)` pair. A Question Publication Event records entry into the
Corpus; a separate Question Version Availability Event records whether a
published version is Available or Archived for selection.

**Question Change Proposal** is one Instructor-owned improvement thread against
a Published Question. A **Question Change Proposal Revision** is one complete,
immutable, numbered proposed change with its exact base Question Version,
publication-validation evidence, semantic impact, and grading impact. A
**Question Change Event** is immutable evidence that opens, merges, or closes
one exact Proposal Revision; it derives the Proposal's Open, Merged, or Closed
state. A merge records the new same-lineage Question Version and succeeds only
when the exact base remains current. A Forced Question Correction has its own
immutable manifest and one corresponding Question Change Event.

**Assignment** is the stable Course Instance-owned delivery record. An
**Assignment Revision** is one complete immutable teaching definition. Accepted
editing creates the next Assignment Revision; publication changes future work
only. Existing work stays pinned to its exact revision and question versions.

**Assignment Attempt** is one Student Record's pass through one Assignment.
It contains **Issued Questions**. A **Question Attempt** is one Student's work
on an Issued Question. A **Question Submission Receipt** records accepted
response submission; an **Automated Grading Receipt** and **Grading Result**
record the later automated decision. This activity spine keeps server-only
answers, grading material, and FERPA records out of Student-visible data.

**Assignment Access** is the server-calculated decision whether one Student
Record may use one Assignment at a given time. It derives from the exact
Student Record, active Course Membership, Assignment, effective policy, and
lifecycle facts. It returns an exact denial reason when access is absent.

## Authority and inheritance paths

Authority is derived through exact stored relationships. These paths name the
ordinary sources of PLE authority:

| Capability | Required path |
| --- | --- |
| Authenticate | Active Account -> Authenticated Session |
| Shared Question Corpus | Authenticated Session -> Active Approved Instructor Account -> Published Question |
| Private authoring | Authenticated Session -> Active Approved Instructor Account -> exact Authoring Workspace ownership or Workspace Collaborator relationship |
| Draft Blueprint Revision contribution | Authenticated Session -> Active Approved Instructor Account -> current Blueprint Collaborator relationship -> exact Draft Blueprint Revision |
| Teach a Course Instance | Authenticated Session -> Active Approved Instructor Account -> active Instructor Course Membership -> Course Instance |
| Student course work | Authenticated Session -> Active Student Account -> active Student Course Membership -> Student Record -> Assignment Attempt / Question Attempt |
| Student FERPA information | exact Student Record and Course Instance relationship, limited to the approved viewer and requested record scope |
| Course observation | Authenticated Session -> Active Approved Instructor Account -> current Course Observer Relationship -> Course Instance, within its closed read scope |
| System administration | Authenticated Session -> Active Sysadmin Account -> exact audited support operation; general Sysadmin status does not provide general FERPA access |

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
