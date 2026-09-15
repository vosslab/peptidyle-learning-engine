# Terminology contract

This is the canonical implementation vocabulary derived from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Human Guidance controls when this file
and another document disagree. Product terms use Title Case as shown below.

## General rules

- Name the product object, not its current table, route, or component.
- Use Account for a global login identity and relationship for scoped access.
- Use Revision only for Published Questions, published Question Pools, and
  Blueprint Courses.
- Use Edit Number only for current-state concurrency; it is not history.
- Use Assignment only inside the three Assessment Type names.
- Do not create product terms from job, event, receipt, snapshot, recovery, or
  compatibility mechanisms unless Human Guidance requires the concept.

## Accounts and roles

**Account** is one global PLE login identity. It has exactly one immutable
**Product Role**: **Student**, **Instructor**, or **Sysadmin**. A person needing
more than one Product Role uses separate Accounts.

**Account State** describes whether the Account can authenticate. Deactivation
blocks access but preserves authorship, relationships, Student Work, and
history. Reactivation restores the same Account and Product Role. No permanent
Account-closure workflow is currently defined.

**Course relationship** binds one Account to one Course Instance in a scoped
role. Current Student and Instructor relationships are not Product Roles.
Instructors may bulk add Students through roster import. They remove Students
individually; PLE has no bulk Student-removal workflow.

**Co-Instructor** is any current Instructor relationship in a Course Instance.
All co-Instructors are equal. Do not use Course Owner, primary Instructor, or
creator privilege for the current Course model.

**Course Observer**, **Student Observer**, and **Grader** are possible future
Course roles. They have no current authority unless their separate
relationship, capability, and privacy contracts are implemented. Grader is not
currently needed because grading is automatic.

**Scoped Support Access** is deliberate Sysadmin access to an exact support
need involving FERPA-protected records. It is recorded. The Sysadmin Product
Role alone provides no ambient FERPA access.

## Questions

**Question Backend** is the component that owns Question rendering,
interaction, response interpretation, grading, feedback, and backend-specific
state. PLE treats backend presentation and state as opaque. When PLE requests a
grading outcome, the Question Backend returns it without a deferred grading
state.

**Question Format** identifies the source/adapter contract, such as native PLE
Question JSON or WeBWorK PG/PGML.

**Question Type** is author-declared educational metadata used for discovery and
labels. It is not inferred from backend controls.

**Draft Question** is private, mutable, unpublished, and unversioned. Its **Edit
Number** may protect concurrent saves but does not create Draft Revisions.

**Published Question** is a stable reusable Question lineage in the Question
Library. Its public **Question ID** displays as `AAAA-ZBBB` and has an eight-
character compact form. Seven characters are random identity; the first
character after the hyphen is an HMAC-derived check character.

**Question Revision** is one immutable source-bearing version within a
Published Question lineage. A compatible source change creates a Revision.
Lineage metadata changes do not. A substantive fork creates a new Published
Question and Question ID while preserving attribution.

**Question Archive** is the high-consequence owner action that removes a
Published Question from ordinary discovery/new selection while preserving
exact Revision evidence already used by Assessments and Student Work.

**Question Library** is the vetted-Instructor discovery and reuse surface for
Published Questions. Students receive only Questions selected for authorized
Coursework.

**Starred Question** and **Watched Question** are Instructor curation concepts.
Watch state is private to the watcher. These concepts are not Student Work.

**Question Statistics** is a privacy-safe aggregate only after it cannot
identify or link back to a Student. Course-local or small-cell analysis remains
FERPA-protected.

## Native PLE Question JSON

**PLE Question JSON** is the native private, unpublished, unversioned, static,
strictly validated Question source format. Its `format` value is
`pleQuestionJson`; there is no format-version negotiation.

It supports exactly these native Question types: multiple choice, multiple
answer, fill in the blank, multiple blank, numerical, matching, ordering, and
hotspot.

**Author JavaScript** is optional Question-authored browser behavior executed in
an isolated untrusted environment. It is never authorization or grading
authority. Native grading remains server-side.

**QTI interchange** covers import, export, and archival exchange. Supported
input becomes native Draft Questions. QTI is not PLE's internal source or
another runtime Question model.

## Question Pools

**Question Pool** is a published reusable collection with a stable public ID
and immutable **Pool Revisions**. Questions and Pools remain distinct objects
even though each may occupy an Assessment position.

**Pool selection evidence** is the exact Pool Revision and Published Question
Revision selected for an Assessment Attempt. Later Pool changes do not rewrite
an Assessment or Student Work.

## Courses

**Blueprint Course** is reusable Course content. It has Blueprint Assessments
and no Students, Student Work, dates, time zones, or relative schedules.

The Blueprint lifecycle is:

- **Private**: owner-only and not adoptable;
- **Public**: visible to vetted Instructors and adoptable; or
- **Archived**: read-only, excluded from ordinary discovery and new adoption,
  available through explicit archived inclusion, and forkable.

Only the **Blueprint Course Owner** changes content or lifecycle. A Public
Blueprint can return to Private only before any adoption. Once adopted, it
remains Public unless Archived. Archived restores to Public.

**Blueprint Revision** is one immutable saved Blueprint content state. Creation
produces a Private Blueprint at Revision 1. A meaningful explicit Save creates
the next Revision; a no-op creates none. Name and lifecycle changes do not
create Revisions.

**Blueprint fork** creates a new Private lineage with ancestry.

**Blueprint adoption** creates a Course Instance from one exact Public
Blueprint Revision and copies its Blueprint Assessments. Each copied Assessment
is independent current Course state. New Blueprint Revisions are offered to
daughter Course Instances for Instructor review and approval; changes to
existing Assessments are never silently applied. A newly added Blueprint
Assessment is copied automatically as an Unreleased Course Instance Assessment.

**Blueprint update** is the Instructor-reviewed path for bringing a newer
Blueprint Revision into a daughter Course Instance. It is distinct from the
automatic addition of a newly added Blueprint Assessment.

**Blueprint Course Change Proposal** is an Instructor proposal to change
another Blueprint Course. The receiving owner chooses what to accept, and
accepted changes create a new receiving Blueprint Revision. A proposal never
changes daughter Course Instances directly.

**Canonical Blueprint JSON** is the complete comparison, import, export, and
exchange representation for Blueprint Course content. It is not the primary
persistence model and contains no Student or Course Instance delivery data.

**Starred Blueprint Course** is a visible Instructor endorsement; vetted
Instructors can see the count and who Starred it. **Watched Blueprint Course**
is a private Instructor subscription to Revision and important-change
notifications. Both belong to the Blueprint lineage across Revisions, and
neither is created automatically by adoption or forking.

**Course Instance** is delivered teaching with current Course settings,
Students, equal co-Instructors, Course Instance Assessments, Attempts, and
FERPA-protected records. It may adopt a Public Blueprint or start empty, must
always have at least one assigned Instructor, and may be deliberately published
as a new Blueprint Course. It represents one teaching period and remains Active
for at most six months from creation. A new academic term uses a new Course
Instance; rollover is not a separate product model.

Each Blueprint Course and Course Instance has its own deliberately entered
**Short Name** and **Long Name**. The short name should remain under about 16
characters when practical. Course Instance names are not derived from the
parent Blueprint names.

**Active Course** and **Inactive Course** describe whether the Course is in
current teaching or past-Course interfaces. A Course Instance becomes Inactive
six months after creation. Inactivity is separate from FERPA archival or
deletion and is not a Blueprint lifecycle state.

## Dates and time zones

Assessment deadlines are stored as absolute UTC instants. Instructor-entered
dates use that Instructor's IANA time zone; Student displays use the Student's
IANA time zone. A new Student defaults once to the inviting Instructor's zone.
Changing either display zone changes presentation only and never moves a stored
deadline. Blueprints contain neither instants nor relative schedules.

## Assessments

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

The product-defined Type icons are `pen-to-square` for Regular Assignment,
`arrows-spin` for Practice Question Assignment, `sparkles` for Bonus
Assignment, `square-q` for Quiz, and `file-signature` for Exam. Labels and icons
remain sufficient without Type color.

**Assessment Question** is one ordered Question position with a current point
value. **Randomize question order** is the setting name for Question-order
randomization.

**Blueprint Assessment** is reusable Assessment content and teaching settings
inside a Blueprint Course. It has no Students, Student Work, due/release dates,
or other Course Instance delivery settings.

**Course Instance Assessment** is an Assessment delivered to Students. It has
current Questions/Pools, point values, timing, access, Attempt, and disclosure
settings.

**Assessment Template** is an Instructor-owned reusable set of Assessment
settings. It has one Assessment Type and contains no Questions or Pools.
Creating an Assessment copies its settings; later Template changes do not
change existing Assessments. Blueprint Assessments do not use Templates.

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

## Assessment Types and disclosure

Regular Assignments support regular learning and default to unlimited Attempts.
Practice Question Assignments use the same whole-Attempt submission boundary as
other Assessments and show the correct answer immediately after submission.
Bonus Assignments are worth zero points possible and add earned points directly
to the grade. Quizzes and Exams may have more restrictive settings.

Regular and Bonus Assignments rarely show the correct answer but show the
Student response and correctness. Quizzes and Exams withhold correct answers
until all Students complete the Assessment. Optional Question Feedback is
separate from the correct answer and grading outcome. It is shown when provided
without its own delayed-release state.

## Attempts, responses, and scoring

**Assessment Attempt** is one Student occurrence of one Course Instance
Assessment. Blueprint Assessments have no Attempts.

**Saved response** is a complete Question response retained while the Attempt
is open. It is replaceable until whole-Assessment submission. An incomplete
response is unsaved for product purposes and is not graded.

**Assessment submission** is the whole-Attempt transition. It finalizes all
saved responses together as Student Work. A Question without a complete saved
response remains visibly unanswered, receives zero credit, and counts as
incorrect without backend evaluation. An internal row or ID must not be
documented as another Student action or product lifecycle.

**Attempt expiration** uses a server-owned wall-clock deadline. Time continues
while disconnected. Reconnect/resume does not pause or extend it. Expiration
submits the whole Attempt and applies the same saved-response and unanswered-
Question rules as Student submission. Interaction checks expiration, and
background processing ensures submission even after the Student leaves.

**Credit fraction** is the immutable grading outcome returned by the Question
Backend for a complete response it evaluates. PLE stores it unchanged.

**Assessment score** is calculated from stored credit fractions and current
Assessment Question point values. A point change recalculates scores without
backend interaction or regrading.

When an Assessment has multiple submitted Attempts, the highest Assessment
Attempt score is the Student's Assessment score. PLE uses Question points rather
than Grade Categories, weighted categories, Course Grade Schemes, or Course
percentage calculations. Pilot grade export is CSV or TSV only and carries
point-based Assessment scores for Course-level handling in the Instructor's
home LMS.

**Student Work** is the FERPA-protected Course evidence needed to identify the
exact Question/Pool Revision delivered, finalized saved response, backend
credit outcome, and other minimum facts needed to interpret the work.

## Interface vocabulary

Instructor primary Ribbon tabs are **Courses**, **Questions**, and
**Assessments**.

Course tasks are **My Blueprint Courses**, **My Active Courses**, **My Inactive
Courses**, and **Search Public Blueprint Courses**.

Question tasks are **My Questions**, **My Draft Questions**, **Starred**,
**Watched**, **Search Question Library**, and **Browse Question Library**.

Assessment tasks are **Assessments Due Soon** and **My Assessment Templates**.

Student work is collectively **Coursework**. A particular item uses its
Assessment Type label. The return action is **Back to Coursework** and the
whole-Attempt action is **Submit Assessment**.

**Student View** is an Instructor answer-free preview and creates no Student
Work. **Profile menu** owns Sign Out.

Required backed destinations remain visible with honest empty states.
Unimplemented future capabilities are not shown as usable controls.
The complete Student and Sysadmin Ribbon task layouts do not have locked-in
designs yet.

## Retention

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

The background check is idempotent. Human Guidance does not define numeric
FERPA retention durations or exact job, event, receipt, or table shapes.

## Implementation-only vocabulary

Current source may contain `AssignmentId`, `QuestionAttemptId`,
`QuestionSubmission`, `Available`, grading jobs, generations, receipts, or
other old/internal names. Use them only when pointing precisely to current
implementation evidence. State the current product term or gap nearby. Do not
promote them into product language, UI copy, new architecture, or additional
lifecycle states.
