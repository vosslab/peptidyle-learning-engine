# Question Backend specification

## Purpose and authority

A Question Backend turns one exact Published Question Revision and its issued state into a
Question presentation, validates a response, and returns grading facts. This document is the
common rule for Native JSON and WeBWorK. It follows [Human Guidance](../HUMAN_GUIDANCE.md).
A Question Pool selects a Question; it does not run the selected backend.

Questions are backend agnostic. Every Backend serves the same Question model and lifecycle.
Differences in source format, rendering, interaction, and grading belong behind this interface.
Missing support in an implementation is a gap in the common workflow, not a separate product rule.

## Responsibility boundary

Library and lifecycle metadata belong on the Question record for every Backend, rather than in
Backend source. Title, Description, Tags, license, citation, classification, ownership, and identity
are examples, not an exhaustive list. See
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md).

| PLE owns | The selected Question Backend owns |
| --- | --- |
| Account, Course, Assessment, Attempt, and Student Work authorization | complete-source validation |
| exact Question Revision and Pool-selection evidence | answer-free rendering and interaction |
| response saving and whole-Attempt submission | response interpretation |
| Assessment points, Instructor-controlled partial-credit setting, and score calculation | grading and partial-credit calculation |
| storing the returned immutable credit fraction | backend-specific randomization and opaque state |
| access to hints, feedback, answers, and explanations | backend-generated feedback and answer material |

PLE provides only trusted, server-derived inputs: the exact Revision, private source, issued
randomization or retained opaque state, and saved response. The browser cannot select a backend,
Revision, seed, answer, score, or credit fraction. Private source, Answer Keys, provider
credentials, private URLs, server-only state, and raw provider results stay server-side.

## Stored credit and awarded points

New Assessments start with partial credit enabled.

Each Backend calculates the earned credit fraction using its grading rules. PLE stores that
fraction regardless of the Assessment's partial-credit setting. The setting controls awarded
points, not the Backend calculation or stored outcome:

```text
if partial_credit_enabled:
    awarded_credit = stored_credit
else:
    awarded_credit = 1 if stored_credit == 1 else 0
question_points_earned = question_points_possible * awarded_credit
```

Assessment Instructors may change the setting after Students start. Apply the current setting
consistently to every Attempt, including submitted Attempts, and recalculate Assessment scores
from stored fractions. This also updates which Attempt has the highest score. Stored responses
and grading outcomes remain unchanged; the Backend does not regrade them.

## Required operations

| Operation | Required result |
| --- | --- |
| Validate | Check source for publication or report errors during preview and testing. Draft creation, import, and saving preserve unfinished or broken content. |
| Render | Return answer-free presentation and only state needed to resume the issued Question. |
| Save | Retain a complete response while the Attempt is open; saving does not post a result. |
| Evaluate | Return an immutable credit fraction without a deferred grading state. |
| Submit | PLE finalizes saved responses together as Student Work and stores the result. |
| Disclose | PLE posts automatically graded scores immediately on submission; Assessment rules govern answers and deferred optional feedback. |

A backend may evaluate before Attempt submission when its interaction needs it. A later point-value
change recalculates points from stored credit; PLE does not ask the backend to grade again.
A complete response means valid data the Backend can evaluate. For Native JSON MATCH and
MULTI-FIB, a valid saved response can include unanswered parts. Grade those parts as zero while
preserving credit for answered parts. This follows their settled grading rules.
Unanswered Questions are not sent to a backend. Rendering, validation, or grading failure must not
record an incorrect response or a made-up score. There is no generic pending-grade job, retry
protocol, or grading receipt.

Backend feedback is transient unless that backend provides a robust retained form. PLE does not
scrape feedback, answer logic, or controls from source or rendered output. PLE-managed Hints,
Question Feedback, and Worked Solutions are separate teaching material.

Optional feedback timing is not a backend submission rule. The deferred product decision is whether optional
Question feedback appears automatically after submission or follows an Assessment-controlled rule.
Until settled, the backend returns protected feedback only when asked and PLE owns disclosure.

## State and evidence

For each issued Question, retain the exact Revision, Pool selection when applicable, backend
randomization value or opaque state when applicable, saved response, immutable credit fraction,
and protected teaching content needed for Student Work. This is evidence for issued work, not a
general replay service or archive of rendered pages.

## Current backends and later additions

`ple` and `webwork` are the current production backend names. iMathAS is a possible later backend
and H5P remains deferred. A later backend needs a complete source model, bounded response,
server-only secrets, preserved state, immutable credit result, and real authorization and
submission tests. It does not create another Account, Assessment, or grading lifecycle. Future H5P
is limited to Weekly Assignments, Bonus Assignments, and Practice Question activities; Quizzes and
Exams do not use H5P.

Implementation evidence is recorded outside this specification in
[question_backend_spec_handoff.md](../active_plans/reports/question_backend_spec_handoff.md).
Open and deferred decisions are listed in
[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md).
