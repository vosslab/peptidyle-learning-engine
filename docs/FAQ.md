# Frequently asked questions

These answers follow [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Current routes or
screens with older names are implementation gaps, not a different product.

## What is a Blueprint Course?

A Blueprint Course is reusable Course content with no Students, dates, time
zones, or relative schedules. It is created Private with Revision 1. A changed
explicit Save creates the next immutable Revision; a no-op creates none.

Private is owner-only and cannot be adopted. Public is visible to vetted
Instructors and adoptable. Archived is read-only, omitted from ordinary
discovery and new adoption, available only through explicit archived inclusion,
and forkable.

## What is a Course Instance?

A Course Instance is delivered teaching with Students, equal co-Instructors,
Course dates, Assessments, Attempts, and FERPA-protected records. It may adopt
one exact Public Blueprint Revision or start empty. Adoption copies Blueprint
Assessments into editable Course Instance Assessments.

## Do Blueprint changes update daughter Courses?

Newly added Blueprint Assessments are automatically copied into daughter Course
Instances as unreleased Assessments. New Blueprint Revisions are offered for
the daughter Course Instructor's review and approval, and changes to existing
Assessments are never applied silently.

## Can Instructors propose Blueprint changes?

Yes. An Instructor may create a Blueprint Course Change Proposal. The receiving
owner decides what to accept; acceptance creates a new Revision of the receiving
Blueprint. A proposal never directly changes daughter Course Instances.

## Can I fork a Blueprint?

Yes. A fork starts a new Private Blueprint lineage with ancestry. A Private
Blueprint is owner-only until its owner makes it Public.

## What is an Assessment?

Assessment is the generic object that organizes Questions and Question Pools
into graded or practice work. The five Assessment Types are Regular Assignment,
Practice Question Assignment, Bonus Assignment, Quiz, and Exam. Assignment is
not an object, category, or parent Type; the word appears only inside those
three Type names.

Blueprint Assessments contain reusable content and teaching settings but no
Students or delivery dates. Course Instance Assessments deliver work to
Students. Assessment Templates contain reusable settings but no Questions or
Pools.

## Does PLE support repeated practice?

Yes. Instructors control the allowed number of Assessment Attempts. Regular
Assignments default to unlimited Attempts, and Students may practice toward a
perfect score when settings allow it. Each Attempt retains its own exact
Question/Pool Revision evidence and Student Work.

When several Attempts are submitted, the highest Assessment Attempt score is
the Student's Assessment score.

## How does submission work?

The Student navigates all Questions and saves complete responses while the
Assessment Attempt is open. Incomplete responses are not saved as complete or
graded. Saving changes only the working response.

The whole Assessment Attempt is submitted at once, either by the Student or
automatically at its deadline. That action finalizes all saved responses
together. Other positions remain visibly unanswered, receive zero credit, and
count as incorrect without being sent to the Question Backend. The server-owned
wall clock continues while the browser is closed or disconnected.

## Who grades a Question?

The selected Question Backend owns rendering, interaction, response
interpretation, grading, feedback, and opaque backend state. It returns an
immutable credit fraction. PLE stores that fraction and calculates points from
the Assessment Question's current point value.

Changing point values recalculates scores without regrading. PLE has no ordinary
Instructor grading, Retry, regrading, or mutable-result workflow.

## Is PLE tied to one Question format?

No. Native PLE Question JSON supports eight strictly validated static Question
types. WeBWorK owns its opaque document, controls, response interpretation, and
grading. Other backends use the same ownership boundary. QTI is an import path;
accepted items become native Draft Questions instead of remaining a second
runtime model.

## Is PLE Question JSON QTI?

No. Native PLE Question JSON is private, unpublished, unversioned, static, and
strictly validated. QTI is a hostile-input import/interchange format.

## Can a Student browser contact WeBWorK directly?

No. PLE is the private renderer's only client. The browser receives an
authorized opaque document through PLE and sends a bounded opaque response back
to PLE. It receives no renderer credential, private source, Answer Key, or raw
grading result.

## Why is grading server-only?

Browser code may validate response shape, but it has no Answer Key, private
grading input, or authority to assign credit. The backend grades from trusted
server state. This also keeps author JavaScript in native Questions untrusted.

## What is Student View?

Student View is an Instructor's answer-free preview. It keeps the Instructor
Account and Course authority and creates no Student relationship, Assessment
Attempt, response, credit, or Student Work.

## How are roles separated?

Each global Account has exactly one immutable Product Role: Student,
Instructor, or Sysadmin. All co-Instructors in a Course have equal authority.
A Sysadmin has no ambient Course membership or FERPA access; support access is
deliberate, scoped, and recorded.

## What happens when access or an Account is deactivated?

New access stops, but authorship, Course relationships, Student Work, and
history remain. Reactivation restores access through still-valid
relationships. Course-record retention remains separate, and no permanent
Account-closure workflow is currently defined.

## What happens to old Student records?

A Course Instance becomes Inactive six months after creation. That limit
prevents Course reuse or deadline extensions from indefinitely delaying FERPA
retention and deletion, but becoming Inactive does not itself delete Student
records. The latest Assessment deadline starts the FERPA retention clock; it
does not itself archive or remove Student data. The configured policy later
determines notice, removal from normal interfaces, recovery, and permanent
deletion. Course metadata, Assessments, Questions, and settings remain.

## What runs in Solid and Wasm?

Solid renders the role-specific interface and calls the same-origin Rust API.
The browser-safe Wasm boundary can format or validate answer-free response
shapes. It never owns authorization, Answer Keys, grading, or Student Work.

## Is the Live Demo a separate product?

No. It is a disposable installation with seeded fictional Accounts and ordinary
product records. Its Account selector replaces identity verification only. The
server still derives Product Role, Course relationships, and authorization.

## Is PLE ready for production?

Consult [ROADMAP.md](ROADMAP.md), [TODO.md](TODO.md), and current release
evidence. A functional Live Demo or passing lower-layer test is not by itself a
production-readiness claim.

## Where should a contributor record a decision?

Product intent belongs in Human Guidance. Cross-module implementation contracts
belong in [CONTRACTS.md](CONTRACTS.md) and focused owner documents. Unresolved
work belongs in [TODO.md](TODO.md) or an active plan only after the product
choice is settled. Accepted implementation evidence belongs in
[CHANGELOG.md](CHANGELOG.md).
