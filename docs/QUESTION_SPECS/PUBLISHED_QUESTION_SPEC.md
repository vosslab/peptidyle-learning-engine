# Published Question specification

## Purpose and scope

A **Published Question** is a reusable Question with its own revision history available through the Question Library to
every Instructor. Each Question Revision is a complete record with fixed source and grading
content and permitted editable metadata fields. This document
defines publication, availability, and later changes. It links to the specialized specifications
for identity, Revisions, forks, metadata, and Backends.

Authority: [Human Guidance](../HUMAN_GUIDANCE.md#published-question-specifications).

## Required properties

- A Published Question has a canonical public Question ID. Questions and Pools share that
  namespace; see [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md).
- It has a complete first Question Revision record and is a Library Object.
- It is available to all Instructors in the global Question Library. Students receive its content
  only through authorized Coursework.
- Every Instructor can read it, add it to an Assessment, or fork it. Only the owning Instructor
  or a Sysadmin may edit it.
- It has Title and Description, required shared Library metadata, and the owner, authors, source,
  citation, and license information that applies to the Question. See
  [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md) and
  [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).
- It may contain optional PLE-managed Hints, Question Feedback, and Worked Solutions. Absence of
  optional support content does not make the Student workflow incomplete.

A Draft can be saved without a Question Type. Publication requires a supported Question Type
and the other required fields. Bloom dimensions may be absent while their deferred initial AI
assignment is pending; no time limit is specified.

## Publish a new Question

First publication accepts a private Draft Question only after Question Publication Validation succeeds.
The result is a new Published Question with a complete Revision 1 record and ordinary Library
availability. A failed validation leaves the Draft private and does not create a partial Library
Object.

## Change a Published Question

The owning Instructor or a Sysadmin may publish a new Question Revision. See
[QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md) for the exact boundary between a Revision
and current metadata edit.

Any Instructor can fork a Published Question into a private Draft. The Draft starts with the
source Question's license, authors, metadata, and Question content, and records its source
Question Revision Tuple. Publication requires validation and creates Revision 1 of the new
Published Question.
See [QUESTION_FORK_SPEC.md](QUESTION_FORK_SPEC.md).

## Archive

Archive is a state on the ordinary Published Question. Follow the GitHub repository archive
model: Archive makes the Question read-only and removes it from normal discovery while preserving
it and its existing references. Archived Questions can be restored or forked.

Place Archive Published Question in the visually separate Danger Zone. Explain its effect on
shared availability and require clear confirmation, as specified in
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#high-consequence-actions).

## Assessment and Pool use

An Assessment identifies a Published Question by its Question Revision Tuple. A Question Pool may
contain that same tuple. Later publication of a new Revision leaves both existing tuples unchanged.
Pool use is specified in [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).

## Deferred behavior

Feedback disclosure timing is deferred pending review of actual PLE feedback uses. Regrading
already submitted work after an answer-key correction is also deferred. Neither is implied by
publishing a Question Revision. See the
[question specification open questions](../active_plans/decisions/question_specs_open_questions.md).
