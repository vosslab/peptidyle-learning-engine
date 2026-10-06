# Question fork specification

## Purpose and scope

A **Question fork** lets any Instructor begin an independent Question from one exact Published
Question Revision. A fork is not a Revision of the source. It creates a private Draft Question and
later may become a new Published Question.

Authority: [Human Guidance](../HUMAN_GUIDANCE.md#published-question-specifications).

Forking mostly follows the GitHub model, with PLE's specific rules below. A fork is another object
of the same kind, with its own identity and a link to its source.

## Fork operation

| Starting state | Actor | Result |
| --- | --- | --- |
| A visible Published Question Revision | Any Instructor | A separate private Draft Question owned by that Instructor, keeping the source Question's authors and source link. |

Archived Published Questions can also be forked through this ordinary workflow.
The Draft keeps current working state. Its Question Revision history begins when it is published.

The fork starts from an exact source Revision. Its Draft remains private until it passes Question
Publication Validation and becomes a Published Question at Revision 1. PLE assigns the fork a new
Question ID; HG does not specify whether allocation occurs during Draft creation or publication.
The existing [fork design decision](../DESIGN_DECISIONS.md#published-question-forks-create-a-new-private-draft-through-one-server-command)
reserves that ID when creating the Draft. A reserved identity does not put the Draft in the Library.

The new Published Question ID is distinct from the source ID. The new Question does not receive
later source changes automatically, and later changes to the fork do not change the source.

## Preserved information

A Question fork starts with the source Question's license, authors, metadata, and Question content,
and records the source Question as its parent. It retains the exact source Question Revision Tuple.
The fork has a new Question ID and is owned by the Instructor who creates it. Its own Revision
history starts at Revision 1 when published. The detailed fields belong to
[QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).

## Examples

- An Instructor forks Revision 3 of a Published Question, revises its wording, and publishes it.
  The result is a new Question ID at Revision 1. Assessments that used the source Revision 3 still
  use Revision 3.
- Publishing a wording change under the original Question ID is a new Revision, not a fork.

## Deferred license behavior

Current Pool and Question license support is CC0, CC BY, and CC BY-SA. NC and ND content are
deferred. When ND support is added, PLE must block forks of ND Questions. The separate treatment
of a Pool that merely references an unchanged ND Question remains unresolved. See the
[question specification open questions](../active_plans/decisions/question_specs_open_questions.md).
