# Assessment lifecycle

This document applies the Assessment and Attempt decisions in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). It describes the target product even
where current route, table, or Rust identifiers still use `assignment`; those
identifiers are implementation gaps, not alternate product vocabulary.

## Lifecycle at a glance

```text
Draft Question
  -> validate and preview
  -> publish an immutable Question Revision
  -> add the Question or a Question Pool selection to an Assessment
  -> release the Assessment in a Course Instance
  -> start or resume one Assessment Attempt
  -> render one Question at a time
  -> save complete Question responses while the Attempt is open
  -> submit the whole Assessment Attempt
  -> disclose grading and feedback according to Assessment policy
  -> retain, archive, and ultimately delete FERPA-protected Student records
```

The browser can request an answer-free presentation and propose a Student
response. It cannot choose the Student, Course, Assessment, Question Revision,
Pool ID, Pool Edit Number, Question Backend, backend state, score, feedback
policy, or retention action. The server derives those facts from authenticated
context and stored relationships.

## Related identities and owners

| Thing | Owner and lifetime | Important identity |
| --- | --- | --- |
| Draft Question | Instructor authoring workspace; private, mutable, and unpublished | Draft Question UUID and Edit Number |
| Published Question | Shared Question Library lineage | Public Question ID `AAAA-ZBBB` |
| Question Revision | Immutable Question source | Question ID and Revision Number |
| Question Pool | Published reusable collection | Public Pool ID and current membership Edit Number |
| Blueprint Assessment | Blueprint Course content | Blueprint Course and Blueprint Revision |
| Course Instance Assessment | One Course Instance's current configuration | Course and Assessment ID |
| Student Work | The Student's FERPA-protected Course record | Student, Assessment Attempt, and saved response identities |

Published Questions and Blueprint Courses have immutable Revisions. Draft
Questions, Question Pools, Course Instances, Assessments, Attempts, and Student
Work use current state plus concurrency controls; an Edit Number is not
historical content.

## Author and publish Questions

An Instructor edits a private, unversioned Draft Question. Preview is useful
feedback but is not publication authority; the server validates the stored
source at publication. A stale Edit Number conflicts instead of overwriting
newer work.

Publishing a new Question creates a stable public Question identity and its
first immutable Revision. Publishing a compatible source change under the same
lineage creates the next Revision. Changing lineage metadata that Human
Guidance treats as current metadata does not create a Revision. A substantive
fork creates a new Question identity.

Published content is independent of the Draft Question workspace. Draft cleanup
must not make a Published Question or Revision incomplete.

Question Pools are also published reusable objects with stable public identity
and current membership on an Edit Number. An Assessment records the exact
Question Revision, and when it selects from a Pool, the Pool ID and Pool Edit
Number, needed to explain its selection. Importing a Pool into an Assessment
creates Course-local composition; later Pool changes do not silently rewrite
that Assessment.

## Build and release an Assessment

Assessment is the generic product object. Its Assessment Type is one of:

- Regular Assignment
- Practice Question Assignment
- Bonus Assignment
- Quiz
- Exam

Assignment is not an object, category, or parent Type; the word is used only
inside those three Type names.

A Blueprint Assessment belongs to a Blueprint Course and contains no Student
work. Adoption copies it into a Course Instance as a Course Instance Assessment.
An Assessment Template is an Instructor-owned reusable set of settings outside
a Course or Blueprint. It contains no Questions or Question Pools.

A Course Instance Assessment starts Unreleased. The Instructor configures its
Questions, Pool selections, point values, instructions, timing, Attempt limit,
late behavior, and feedback behavior, then runs the automated, interactive
Assessment Release Validation. Validation explains missing, invalid, or
unreasonable values; it checks Questions, point values, Attempt/time limits,
date order, a due date at least 24 hours in the future, and a due date no later
than the Course Instance's six-month Active-lifetime boundary. Only a passing
Assessment can be released. A released
Assessment can be unreleased only through the high-consequence Unrelease action;
the typed confirmation title is the Assessment title, and the action deletes
all Student Work for that Assessment.

Assessment is current state. Human Guidance does not define Assessment
Revisions, Closed or Archived Assessment lifecycle states, a Grade Category,
or a Course Grade Scheme. A deadline can make a released Assessment no longer
available for new work without creating another stored lifecycle state.

Assessment deadlines are absolute UTC instants. Instructor input is interpreted
in the Instructor's IANA time zone, and each Student sees dates in the Student's
own IANA time zone. Changing a display zone never shifts a stored deadline.

## Order Assessment entries by Bloom

An Instructor may order the current Assessment Entries by their Bloom
Classification. A fixed Question Entry uses its exact pinned Question
Revision's pair. A Question Pool Entry uses its Assessment-owned fork's current
Bloom pair; it does not use the reusable source Pool's current pair or
derive a pair from Pool members.

The order is Cognitive Process first (Remember, Understand, Apply, Analyze,
Evaluate, Create), Knowledge Dimension second (Factual, Conceptual,
Procedural, Metacognitive), then the Entry's immediately prior position. The
last key makes equal classifications stable. Sorting changes only the pending
Entry sequence. It neither changes an Entry's identity, pin, Pool membership,
selection count, points, or other settings nor creates a new Question Revision or Pool membership Edit.

The sort control remains unavailable when the complete exact pair for any
Entry cannot be read. It explains whether the missing pair is fixed or
Assessment-owned Pool content and leaves the existing order unchanged. A
successful sort remains an unsaved current-Assessment edit: the ordinary whole
Assessment Save, with its existing Edit Number CAS, is the only persistence
boundary.

## Start or resume an Attempt

The server starts or resumes an Assessment Attempt only for an active Student
Course relationship with access to the released Assessment. It assigns server
timestamps and any fixed deadline. Browser clocks are display aids; stored
server time controls access and automatic submission.

Reloading, reconnecting, or using another authenticated browser resumes the
same open Attempt and its saved responses. These actions do not start a new
Attempt or extend its deadline. Attempt limits and whether another Attempt may
start are Assessment policy.

Regular Assignment defaults support repeated work toward success, including an
unlimited-Attempt default. When more than one Attempt is submitted, the highest
Assessment Attempt score is the Student's Assessment score.

## Select, render, and navigate Questions

Starting an Attempt fixes the Question Revision, Pool ID, and Pool Edit Number evidence and the
backend-specific state needed for each Assessment position. Randomization uses
the retained state; resuming an Attempt does not silently choose a newer
Revision or different Question.

Students see one Question at a time and can navigate among all Questions in the
Assessment. Navigation shows saved-status information while keeping page
geometry stable. A render excludes Answer Keys, private feedback, raw private
source, provider credentials, and grader state.

The Question Backend owns rendering, interaction, response interpretation,
grading, feedback, and backend-specific state. PLE stores an opaque
presentation plus opaque backend state and must not build native parsers for a
backend's controls.

## Save responses

A complete response can be saved while the Attempt remains open. A later valid
save replaces that Question's working response. An incomplete response is not
saved as a complete response and is not graded.

Saving changes only the working response. The Student may revisit and edit any
saved response until the whole Attempt is submitted or automatically closes at
its deadline. The only Student submission action applies to the whole Attempt.

The backend may evaluate a complete response before whole-Assessment submission
so that the immutable credit fraction is ready, but PLE does not expose a
Student-visible grading outcome merely because the response was saved.

## Submit the whole Assessment Attempt

The Student submits the entire Assessment Attempt once. The server loads the
authoritative Assessment, Questions, saved responses, backend state, timing,
and disclosure policy. Repeating a successful submission returns the same
submitted state and does not create another result.

At the deadline, the system automatically submits the Attempt using the same
product semantics: saved complete responses are finalized together as Student
Work. Unsaved or incomplete positions remain visibly unanswered, contribute
zero credit, and count as incorrect without being sent to the Question Backend.
A late response cannot replace saved work after the Attempt has closed.

The Question Backend returns an immutable credit fraction for each complete
response it evaluates. PLE stores that fraction without reinterpretation and
calculates points from the Assessment Question's current point value. Changing
point values recalculates scores from the stored fractions; it does not regrade
responses or change the fractions.

When PLE requests a grading outcome, the Question Backend returns it without a
deferred grading state. PLE has no Student- or Instructor-visible asynchronous
grading workflow, grading job state machine, retry button, regrading lifecycle,
deferred-completion state, or generalized grading receipt.

## Disclose results and feedback

After Assessment Attempt submission, authorized readers may see only the result
and correct-answer material allowed by Assessment policy. Practice Question
Assignments use the same submission boundary and show the correct answer
immediately after submission. Optional Question Feedback is shown when the
Question Backend provides it and does not use Assessment correct-answer
disclosure settings. The browser never submits a score, correctness assertion,
Answer Key, or component weight.

## Question Backend boundary

| Pathway | PLE responsibility | Backend responsibility |
| --- | --- | --- |
| Native PLE Question JSON | Store a private, unpublished, unversioned, strictly validated static source; isolate author JavaScript | Render supported native controls, interpret responses, grade on the server, and produce feedback |
| QTI import | Validate supported input and create native Draft Questions | Native PLE backend owns runtime behavior after conversion |
| WeBWorK | Store the exact licensed PG/PGML source and pass opaque state securely | Render, interpret controls, grade, and return opaque presentation/state plus credit fraction |
| Other backend | Enforce the common authorization, Attempt, retention, and disclosure contracts | Own its format-specific render, response, grading, feedback, and state semantics |

No backend may widen the browser payload to include private source, Answer Keys,
credentials, or backend authority tokens.

## Failure semantics

| Boundary | Safe outcome |
| --- | --- |
| Draft validation or publication fails | Keep the Draft; do not create a partial published object. |
| Concurrent start or resume | Return the one authoritative open Attempt. |
| Render fails | Keep the Attempt resumable without changing its selected Question or state. |
| Network loss during save | Let the Student retry the save while the Attempt remains open. |
| Attempt reaches deadline | Automatically submit the whole Attempt, finalize its saved complete responses, and leave other positions visibly unanswered with zero credit and no backend evaluation. |
| Submission replay | Return the already-submitted Attempt without duplicating results. |
| Backend fails | Preserve Student work and expose only a bounded product error; do not invent credit. |
| Retention processing fails | Do not report an archive or permanent deletion that did not complete. |

The exact retry or repair machinery is an implementation decision justified by
the failing boundary. It is not a separate product recovery lifecycle.

## Retention

The Course Instance becomes Inactive six months after creation. That limit
prevents Course reuse or deadline extensions from indefinitely delaying FERPA
retention and deletion, but becoming Inactive does not itself delete Student
records. The latest Assessment deadline starts the FERPA retention clock; it
does not itself archive or remove Student data. The configured policy later
determines notice, removal from normal interfaces, recovery, and permanent
deletion. Course metadata, Assessments, Questions, and settings remain. See
[RETENTION_POLICY.md](RETENTION_POLICY.md).

## Contract map

- [QUESTION_MODEL.md](QUESTION_MODEL.md): Question source, publication, and
  browser-safe boundaries.
- [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md): Attempts, saved responses, timing, and
  score projections.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md): render and
  response payloads.
- [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md): opaque backend
  ownership and immutable credit fractions.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md): role and Course
  access.
- [RETENTION_POLICY.md](RETENTION_POLICY.md): FERPA record lifecycle.
