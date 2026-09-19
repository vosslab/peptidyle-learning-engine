# Repeated practice and mastery

The filename is historical. "Mastery Assignment" is not a current Assessment
Type or separate product object. The current model uses one of the five
Assessment Types plus independently configurable Assessment settings.
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is authoritative.

## Teaching intent

PLE treats repeated work as continued learning rather than an exception. A
Regular Assignment defaults to unlimited Attempts, and an Instructor may
configure other Assessment Types to allow repetition. A Student can practice
toward a perfect score when the Assessment settings allow it.

Each Assessment Attempt remains separate FERPA-protected Student Work. Starting
a new Attempt does not overwrite earlier responses or grading outcomes.
Resuming an open Attempt keeps its existing Question selections, backend state,
saved responses, and wall-clock deadline.

## Current object model

| Object | Responsibility |
| --- | --- |
| Course Instance Assessment | Current Type, Questions/Pools, point values, Attempt limit, timing, and disclosure settings |
| Assessment Attempt | One Student's occurrence of that Assessment |
| Selected Question evidence | Exact Question Revision, Pool ID and Pool Edit Number when applicable, and randomization/backend state |
| Saved response | Replaceable complete response while the Attempt is open |
| Finalized response evidence | Saved response retained with the submitted whole Assessment Attempt |
| Credit outcome | Immutable fraction returned by the Question Backend |

The Student navigates all Questions, saves complete responses, and submits the
whole Assessment Attempt. The saved responses are finalized together in that
transition; no nested product lifecycle is introduced.

## Defaults and independent settings

Assessment Type provides defaults but does not lock the settings. Instructors
can change settings without changing Type.

- Regular Assignments are practice for current learning and default to unlimited
  Attempts.
- Practice Question Assignments provide focused review, use the same submission
  boundary as other Assessments, and show the correct answer immediately after
  the whole Assessment Attempt is submitted.
- Bonus Assignments provide optional extra credit, are worth zero points
  possible, and add earned points directly to the grade.
- Quizzes may use more restrictive Attempt and collaboration settings.
- Exams may use more restrictive Attempt, timing, availability, and feedback
  settings.

New Course Instance Assessments default to accepting submissions and starting
Attempts only through the due date, with late work rejected. Disclosure
settings remain independent.

## Attempts and Questions

Each Attempt has a server-owned wall-clock time limit. Reconnect, reload, or a
different authenticated browser resumes the same open Attempt without pausing
or extending the clock. At expiration, PLE submits the whole Attempt, finalizes
saved complete responses, and leaves other Questions visibly unanswered. Each
unanswered Question contributes zero credit and counts as incorrect without
being sent to the Question Backend.

A new Attempt may choose new randomized variants or Pool selections according
to Assessment settings. An existing Attempt never changes its delivered
Question Revision or Pool Edit Number evidence.

## Scoring

The Question Backend owns response interpretation and returns an immutable
credit fraction. PLE calculates the Assessment score from that fraction and
the current Question point value. A point-value change recalculates scores; it
does not regrade or replace the credit outcome.

When several Attempts are submitted, the highest Assessment Attempt score is
the Student's Assessment score. PLE uses Question point values directly and
does not add separate Question weights, Grade Categories, weighted categories,
a Course Grade Scheme, or Course percentage calculations. Pilot grade export
uses CSV or TSV point data.

## Feedback boundary

Saving a complete response changes only the working response and does not
disclose a grading outcome. Whole-Assessment submission is the ordinary
grading-disclosure boundary.

Practice Question Assignments show the correct answer immediately after the
whole Assessment Attempt is submitted. Optional Question Feedback is shown
when the Question Backend provides it and does not use Assessment correct-answer
disclosure settings.

## Student View

Instructor Student View is answer-free preview using the Instructor's own
identity and Course authority. It creates no Assessment Attempt, response,
credit, or Student record. Only an enrolled Student creates Student Work.

## Implementation gap rule

Current source may still expose mastery-specific policies, generic Assignment
names, nested Question Attempts, grade-selection enums, or response-level
finalization records. Those are implementation evidence for a later code task,
not requirements to preserve in new documentation or UI.
