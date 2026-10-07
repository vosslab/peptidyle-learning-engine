# Question Revision specification

## Purpose and scope

A Question Revision is a complete Question record, including its source and metadata. Publishing
another Revision creates another complete record under the same Question ID. Some fields on the
current record can be edited in place without creating another Revision.

Authority: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#common-revision-and-history-specifications).
Be conservative about creating Revisions. Saving changes to a Question Pool advances its Edit Number;
see [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).

## Identity and complete records

- Question ID plus Revision Number identifies the complete Question record.
- Revision Numbers start at 1 and increase sequentially for that Question.
- A new Revision retains the Question ID and starts from the preceding record, carrying forward
  its attributes except for the changes being published.
- A fork creates a separate Question with a new public ID; publication starts its Revision 1.
- A Revision includes Title, Description, classification, Tags, Bloom, Type, source, and the other
  Question fields. The permission to edit a field in place does not move it outside that record.

## Changes and publication

Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked
Solutions, or Question Image Assets creates a new Question Revision, as HG specifies. Published
source and grading content remain fixed in the record delivered to a Student.

Title, Description, Discipline, Subject, Topic, Subtopic, Tags, and Bloom are ordinary metadata
fields on the current Question record. Permitted edits update that record in place, preserving
its Revision Number. Use ordinary record concurrency when needed. Bloom follows the same editing
model as Title.

Question Type is built into Native JSON's source format. Other Backends use it as classification
metadata. Correcting that classification in place follows ordinary metadata editing; changing
Native JSON's interaction requires a source edit and therefore a new Revision. See
[QUESTION_TYPE_SPEC.md](QUESTION_TYPE_SPEC.md).

The owning Instructor or a Sysadmin may make these edits. Another Instructor can fork the
Question. Saving metadata validates the Question's required fields. If the edit causes a Pool
mismatch, show that mismatch and apply the Pool release rules in
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md#mismatch-and-release).

## Assessment and Student Work references

Assessments and Pools identify Published Questions by their Question Revision Tuples. Publishing
a new Revision preserves those existing tuples. Student Work retains the delivered Question
Revision Tuple, saved response, and Backend grading outcome. Editing metadata on the referenced
record preserves its source and grading content; the Revision Number is not a snapshot of every
past metadata value.

## Examples

| Change | Result |
| --- | --- |
| Correct an answer key and publish | Create the next complete Question Revision; existing Student Work retains its delivered reference. |
| Correct a Title or Tag typo | Edit that field on the current Question record, preserving its Revision Number. |
| Correct Bloom | Edit ordinary metadata on the current Question record, preserving its Revision Number. |
| Correct a WeBWorK Type classification | Edit classification metadata; check affected Pool requirements. |
| Change a Native JSON interaction | Publish the changed source as a new complete Revision. |
| Fork another Instructor's Question | Create a separate Draft; publication creates its own ID and Revision 1. |

## Deferred evaluation after a correction

Evaluating previously submitted Native JSON responses again after an answer-key or grading-rule
correction remains deferred. If implemented, the new grading result replaces the previous result.
Metadata edits leave grading results unchanged. Assessment point-value and partial-credit changes
recalculate awarded points from stored Backend credit without another Backend evaluation; see
[QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md).
Implementation alignment belongs in [TODO.md](../TODO.md#question-spec-implementation-follow-up).
