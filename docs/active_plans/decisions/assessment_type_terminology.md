# Assessment type terminology

## Status

Open for owner decision. This document proposes terminology and structure. The
canonical terminology contract retains its current Assignment model until the
owner accepts a shared root.

## Teaching meanings

| Visible type | Teaching purpose                                                          | Typical course policy                                                                            |
| ------------ | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Assignment   | Work completed outside class to reinforce learning or explore a new topic | Weekly, low point value, repeated attempts, and collaboration encouraged                         |
| Quiz         | Short check of recent understanding                                       | Moderate point value, short time limit, individual work, and an optional collaborative follow-up |
| Exam         | Cumulative midterm or final assessment                                    | High point value, bounded time, individual work, and limited attempts                            |

The values used in one course, such as 5-15 Assignment points, a 20-point
30-minute Quiz, or two 50-point 75-minute Exam parts, belong in course-owned
templates or records. They are useful examples rather than product constants.

## Recommended model

Use **Course Assessment** as the shared structural root and require one
**Assessment Type**: Assignment, Quiz, or Exam. Keep the three visible names in
the Instructor and Student interfaces. One common model owns revisions,
ordered entries, publication, scheduling, issued Questions, attempts,
submissions, grading, and receipts.

## Current evidence

[ACTIVITY_MODEL.md](../../ACTIVITY_MODEL.md) already separates completion,
grade selection, continued attempts, variation, timing, and Student Feedback
release. [MASTERY_ASSIGNMENT_DESIGN.md](../../MASTERY_ASSIGNMENT_DESIGN.md)
currently presents Mastery, Standard graded assignment, Exam, and Practice as
policy templates. Quiz and collaboration expectations remain the missing
teaching distinctions.

Assessment Type and policy template answer different questions. Assessment
Type says whether the visible course work is an Assignment, Quiz, or Exam.
Mastery, Standard, and Practice are reusable starting policy templates. The
current Exam template becomes the ordinary starting policy for Assessment Type
Exam rather than remaining a peer of Mastery and Practice.

Assessment Type records teaching intent and selects an Instructor-editable
starting template. Explicit policies remain authoritative for behavior:

- availability, due, and close times;
- whole-assessment and per-Question time limits;
- Question Attempt limits and later assessment attempts;
- Question variation and Question Pool selection;
- grade selection and point values;
- Student Feedback release; and
- collaboration expectations.

Add **Collaboration Expectation** as explicit Student-visible teaching
direction. Suggested values are Individual Work, Collaboration Allowed, and
Collaboration Encouraged. The interface displays the selected expectation with
the assessment instructions. An in-class collaborative Quiz can retain
individual Student Attempts and Submissions while selecting Collaboration
Encouraged. Shared submissions require a separately approved ownership design.

Treat Assessment Type as the Instructor's stated teaching meaning. It remains
stable when an Instructor configures an untimed Quiz, a take-home Exam, or
another reasonable policy combination.

## Interface proposal

Use **Assessments** as the Course Instance destination, with Assignments,
Quizzes, and Exams as its views. Creation starts with the visible Assessment
Type and then exposes the explicit policies. Question Type and Question Format
remain independent of Assessment Type.

## Shared record path

If Course Assessment becomes the structural root, the record path becomes:

```text
Course Assessment
  -> Assessment Attempt
    -> Issued Question
      -> Question Attempt
        -> Question Submission
```

This uses one storage and API hierarchy for the shared mechanics.

## Decisions to settle

- Approve Course Assessment as the shared root, or select another plain shared
  term.
- Approve Assessment Type as a required authored field.
- Decide whether an Exam with multiple timed parts is one Exam with explicit
  Exam Parts or several linked Exams.
- Confirm that collaborative Quizzes retain individual Student submissions for
  the first implementation.
