# Question Pool specification

## Purpose and scope

A **Question Pool** is a reusable Library Object. It contains an unordered set of Question
Revision Tuples identifying Published Questions. Those Questions should be reasonably interchangeable. PLE selects from that set for Student Attempts.
Pools and Published Questions were both part of the original integrated Question Library design.

This document defines which Questions a Pool can contain, its metadata and license, how
Assessments select Questions from it, how problems are handled, and forking.
Authority: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-pool-specifications).
Published Question forks have a separate Draft/Revision lifecycle described in
[QUESTION_FORK_SPEC.md](QUESTION_FORK_SPEC.md).

## Identity, state, and access

- A Pool has its own public `XXXX-ZXXX` ID in the shared Question/Pool namespace. See
  [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md).
- A Pool is created from a Published Question and enters the Question Library immediately.
  It has no Draft Pool state or Pool Revision family.
- The Pool remains owned by its Instructor owner regardless of how many Assessments reference it.
  All Instructors can discover, use, and fork Pools. The owner can edit the Pool; Sysadmin authority
  follows [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).
- Students access selected Questions through Coursework, not Pool discovery.
- A Pool has an owner and, when forked, a parent/source Pool pointer. It has no separate Author
  field. Member Questions retain their owners, authors, attribution, and licenses.
- Changes to a Pool take effect when the Instructor saves them. There is no undo after saving.
  A changed save advances its Edit Number. This counter cannot recover earlier Pool contents.

## Membership

A Question Pool contains an unordered set of Question Revision Tuples. A Question Pool can contain a
Question ID only once. The Question Pool also has its own properties, defined separately from the
properties of the Published Questions it contains.

For example, `(Q, 1)` and `(Q, 2)` are different Question Revision Tuples, but cannot coexist in
one Pool.
Pools contain only Published Questions.

Published Questions offer **Create Pool from Question**. The creation view shows the starting
Question, its Discipline and Subject, and the Pool Title field. Creation includes that Question.
The first member establishes Discipline, Subject, Question Type, and Question Backend; every
additional member matches those four values. The member picker offers eligible Published Questions
through the Question Library. The Questions in a Pool must also satisfy the calculated-license rules below.

Member Topic, Subtopic, Tags, owners, authors, and Bloom values may differ. Pool contents should
represent reasonably interchangeable assessments of the intended learning.

When the set of Question Revision Tuples in a Pool changes, saving
advances the Pool's Edit Number.

Member editing supports spreadsheet-style display sorting. Display order changes neither the set of Question Revision Tuples
nor random selection and does not advance the Pool Edit Number.

| Proposed contents | Result |
| --- | --- |
| Two Native JSON Numeric Questions with the same Discipline and Subject | Common values match; interchangeability and license compatibility still apply. |
| Revision 2 and Revision 4 of one Published Question | A Pool cannot contain two Revisions of the same Published Question. |
| Native JSON Multiple Choice and WeBWorK Multiple Choice | Backend mismatch. |
| Two Matching Questions with different Topics | Topics may differ; the Pool has its own Topic and Subtopic. |

## Metadata and discovery

A Pool has its own Title, Description, Topic, Subtopic, Tags, Bloom Knowledge Dimension, Bloom
Cognitive Process, and optional PLE-managed Hints, Question Feedback, and Worked Solutions.
These describe the Pool, rather than aggregate its members' fields. Discipline, Subject,
Question Type, and Backend are the common member values. PLE calculates the license.

Text search and Topic, Subtopic, Tag, and Bloom filters match the Pool's own text and metadata.
Each member retains its own metadata, source, and rights information. Shared field requirements
are in [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md).
Store calculated Pool metadata on the Pool. Keep these values up to date when the Pool is created or
saved. Search reads the stored values. A periodic backend cron job for recalculating Pool metadata is
deferred.
Optional Question Feedback timing and initial AI Bloom assignment remain deferred.

## License

Initial release supports CC0, CC BY, and CC BY-SA Questions. NC and ND Questions are excluded and
may be reconsidered after release. A Pool cannot use `Mixed` as its license.

PLE calculates the Pool license from its Questions when the Pool is created and whenever
its set of Question Revision Tuples changes. The owner does not choose it manually. The Questions
keep their own licenses and all original rights information.

| Member licenses present | Calculated Pool license |
| --- | --- |
| CC0 only | CC0 |
| CC BY, with or without CC0 | CC BY |
| CC BY-SA, with any CC0 or CC BY | CC BY-SA |

The implementation must use an explicit compatibility table for exact license identifiers; it must
not assume a simple most-restrictive ordering. If the proposed Question Revision Tuples have no
compatible supported license, the save is rejected and the Pool stays unchanged.

### Meaning of the Pool license

The Pool license is a clear license for the Pool as a reusable Library Object. It does not relabel,
replace, or grant rights beyond any member's original license. The Pool's calculated license is the
value used when filtering Pool results by license; member license filters continue to apply to
member Question results.

The initial release excludes NC and ND Questions. Reconsider them after release; no future Pool
compatibility or fork behavior is established here.

## Assessment references and selection

Adding a Pool to an Assessment references the existing Pool directly. The Assessment specifies
how many Questions to select from it. That number belongs to the Assessment, not the Pool.
An Assessment does not own the Pool. One Pool may be referenced by one Assessment or hundreds.

Saved Pool changes affect future selections in every Assessment that uses the Pool. Existing Attempts retain the Questions already selected for them.
Starting a new Attempt makes fresh selections; resuming an Attempt preserves its selections.

PLE selects the requested member Questions. Each selected Question Backend controls rendering,
interaction, response interpretation, grading, feedback, and backend-native variation. Pool
selection is separate from randomization within a selected algorithmic Question and works the
same way for every Backend.

For example, an Assessment uses a Pool with five valid Published Questions and requests two.
Attempt A receives Question X Revision 3 and Question Y Revision 1. Returning to
Attempt A keeps those Questions. A new Attempt selects again from the current Pool.

## Forking

Forking is an explicit Instructor action to create an independent Pool. It creates:

- a new public Pool ID;
- the Instructor who created the fork as owner;
- the current set of Question Revision Tuples and Pool metadata copied from the parent Pool;
- Edit Number 1; and
- a parent/source Pool pointer.

The fork is another ordinary reusable Pool. The parent pointer identifies its source, not a stored
historical parent state. Changing either Pool leaves the other unchanged.

There is no Assessment Pool subtype, Assessment ownership of Pools, automatic fork-on-add,
copy-on-write behavior, or hidden Pool snapshot. These boundaries correct specific previous
misinterpretations of the regular Pool model.

## Mismatch and release

A Pool must continue to meet its requirements. Show the specific problem, such as a Published
Question with a different Discipline, Subject, Type, or Backend, duplicate Questions, or
incompatible licenses.

If an Assessment requests more Questions than the Pool can provide, show the problem and block
release. A Pool with two valid Questions may itself be valid while an Assessment requesting three
cannot be released. Each Assessment specifies how many Questions to select.

Before release, Instructor editing may leave an Assessment temporarily incomplete or inconsistent.
Save the current work and show the specific problem. Release is the validation boundary for
unreleased Assessments. An insufficient count blocks release of the affected Assessment; it does
not reject the Pool Save merely because an unreleased Assessment requests too many
Questions. The Instructor can add Published Questions, lower the number selected, replace the Pool,
or remove it from the Assessment.

If a Pool develops a problem after an Assessment is already released, allow that Assessment to
continue as-is.

After an Assessment has been issued to Students, the established post-issue editing restrictions
apply. Individual removal is limited to Questions not issued to any Student in that Assessment.
Removing the whole Pool from an Assessment excludes its earned and possible points from every
Attempt for fairness. Assessment Unrelease deletes Student Work and permits normal editing.
See [ASSESSMENT_LIFECYCLE.md](../ASSESSMENT_LIFECYCLE.md).

| Situation | Result |
| --- | --- |
| Before release, a removal leaves two valid members while an Assessment entry requests three. | Save; show that the Assessment requests more Questions than the Pool can provide and block release. |
| A member's Subject changes and no longer matches the Pool. | Show the Subject mismatch and block affected release until resolved. |
| A problem develops after an Assessment is released. | The released Assessment continues as-is; established post-issue edit restrictions remain. |
| Two Revisions of one Published Question are proposed for the same Pool. | A Pool can contain at most one Revision of each Published Question. |

## Student Work evidence

Student Work for a Question selected from a Pool keeps:

- the selected Question Revision Tuple;
- the Pool ID; and
- the Pool Edit Number at selection.

The exact Question Revision preserves the delivered source for interpretation and grading. The
Pool Edit Number is only a counter; it cannot recover an earlier set of Question Revision Tuples. Existing Attempts
remain interpretable after later Pool edits without historical Pool snapshots or Pool Revisions.

The Question Backend returns the credit fraction for the response, and PLE stores that outcome.
Pool edits do not rewrite delivered Questions, saved responses, or Backend grading outcomes.
Assessment point and partial-credit settings determine awarded points from the stored credit;
see [QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md).
