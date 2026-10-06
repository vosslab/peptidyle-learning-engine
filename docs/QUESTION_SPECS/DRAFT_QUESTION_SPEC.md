# Draft Question specification

## Purpose and scope

A **Draft Question** is where an Instructor writes and tests a Question before publication.
The workflow is: import or write a Draft, preview it, test it, refine it, add the metadata, then
publish. These are the same Question operations regardless of Backend. Forking a Published
Question provides another starting point.

This document defines the lifecycle before publication. It does not define Question source
formats, Library metadata fields, ownership, or the publication API; see
[QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md),
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md),
[QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md),
and [PUBLISHED_QUESTION_SPEC.md](PUBLISHED_QUESTION_SPEC.md).

This specification follows [Human Guidance](../HUMAN_GUIDANCE.md#draft-question-specifications).

## State and visibility

- A Draft Question is private, mutable, unpublished, and unversioned.
- It is not a Library Object and does not appear in Question Library search or browse results.
- Drafts autosave with a visible saved status so Instructors can return to unfinished work.
- Saving replaces its current working state. A save does not create a Question Revision.
- A Draft Question may use an Edit Number to reject a stale save. That number is a concurrency
  check, not a Draft Revision or a history record.
- A Draft has no content or metadata requirements. It can be empty, incomplete, or broken.
- Create, import, and save preserve that working content. Complete source and required metadata
  are checked at publication.

## Operations

| Operation | Actor and starting state | Result |
| --- | --- | --- |
| Create or autosave | The Draft's authorized Instructor edits private working content. | Autosave the current working content with a visible saved status, including empty, incomplete, or broken content, using the ordinary Edit Number check where applicable. |
| Preview and test | The Instructor tries the Question while authoring it. | Rendering, interaction, and grading can be checked before publication; the Question remains a private Draft. |
| Delete | An Instructor deletes an unneeded Draft. | The private Draft is removed. Published Question Revisions copied from it remain unaffected. |
| Publish a new Question | An authorized Instructor submits a new Question Draft that satisfies Question Publication Validation. | A new Published Question and Revision 1 are created; see [PUBLISHED_QUESTION_SPEC.md](PUBLISHED_QUESTION_SPEC.md). |
| Publish a new Revision | The owning Instructor or a Sysadmin publishes changes to an existing Question after validation. | Keep the Question ID and create the next complete Revision; see [QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md). |
| Publish a fork | An Instructor starts from a fork Draft. | Publication makes the fork available under its own Question ID at Revision 1; see [QUESTION_FORK_SPEC.md](QUESTION_FORK_SPEC.md). |

Question Publication Validation requires Discipline, Subject, and every other required Question
Library metadata field. In particular, Question Type and other required values cannot be absent.
Bloom classification may remain absent while its deferred initial assignment has not run; this is
not permission to omit required publication metadata.

## Preview and testing

An Instructor can render the Draft, interact with the Question, try responses, and inspect the
grading result before publication. Errors must be visible while the Instructor can still edit
the Draft. A picture of the prompt alone does not provide this authoring test.
Preview or test errors leave the Draft available to save and continue editing.

The preview uses the selected Question Backend through the common interface. For WeBWorK, that
means running PG or PGML through its renderer and evaluator. A local browser preview is only one
possible implementation; lacking a local preview does not remove the authoring requirement.
Previewing and testing leave the Question as a private Draft and create no Student Work.

## Abandoned Drafts

Human Guidance permits cleanup of abandoned Drafts after an appropriate warning and recovery
period. Automated cleanup remains deferred, and no expiration period is set.
Cleanup must never damage content already copied into a Published Question Revision.
See the [question specification open questions](../active_plans/decisions/question_specs_open_questions.md).

## Examples

- An Instructor saves an incomplete Native JSON Question with no Subject. It remains a private
  Draft; publication must refuse it until the required metadata is complete.
- An Instructor changes a Draft's answer key three times. Autosave preserves the current working
  state each time; these saves do not create a Revision history.
- An Instructor forks Published Question `ABCD-EFGH`. The resulting private Draft is not
  discoverable through the Question Library until it passes publication validation.

## Related behavior and deferred work

A Draft has no Student Work and cannot be selected for an Assessment. Question source, feedback,
and grading behavior are specified elsewhere. Question Feedback is optional, but its disclosure
timing is deferred; this Draft specification creates no timing rule.
