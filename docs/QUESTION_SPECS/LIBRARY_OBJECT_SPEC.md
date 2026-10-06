# Library Object specification

A Library Object is either a Published Question or a Question Pool. Both are first-class objects
in one Question Library and were part of PLE's original design. A Library search result represents
a Library Object: shared fields plus the fields specific to its kind.

Authority: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-specifications). This document owns
the common model; the [README.md](README.md) identifies the detailed rule owners.

## The two kinds

| Property | Published Question | Question Pool |
| --- | --- | --- |
| Meaning | One reusable Question with immutable source Revisions | A reusable set of interchangeable Published Questions |
| Public identity | Question ID | Question ID in the same namespace |
| Source content | One exact Question Revision | Unordered set of Question Revision Tuples |
| Change counter | Question Revision Number for the complete record; ordinary Edit Number where needed | Pool Edit Number for saved changes |
| Working state | Private Draft before publication | Created from a Published Question; immediately in the Library |
| Execution | Selected Question Backend renders and grades | PLE selects members; each selected Question Backend renders and grades |
| Attribution | Owner and authors are distinct | Owner and optional source-Pool link; each member retains its own authors |
| Assessment use | Exact Question Revision at an ordered position | Pool reference and Assessment selection count at an ordered position |

A Pool is not a Question Backend or a Question Type. It has one common Type and Backend because
its members must agree. A Draft Question is private working content, not a third Library Object kind.

## Question behavior

Questions are backend agnostic. Drafts, publication, identity, ownership, Revisions, Library
discovery, and Assessment use follow the same Question rules. The selected Backend handles its
source and execution through the common Question Backend interface; it does not create a
different kind of Question or lifecycle.

Every stored Question has its own internal Question record and one canonical Title; compact
interfaces may truncate the displayed Title. Questions are subject agnostic and graded strictly
and deterministically by their Backend, without requiring an Instructor to grade. Answer-choice
randomization belongs to the Question, including its Native JSON setting, not to the Assessment.

## Shared responsibilities

Library Objects have a canonical Question ID, an owner, Title, Description, required Discipline and
Subject, optional Topic/Subtopic and Tags, a required license, and their own discovery metadata.
Bloom may be absent while initial AI assignment is deferred. The authoritative field table is in
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md).

Library Objects participate in Search, Browse, filtering, and sorting. Instructor bulk metadata
editing remains desired but is deferred. Shared
fields retain their meanings through storage, API responses, browser data, and result display.
Kind-specific fields remain explicit. A Pool result does not need dummy Question source, an
invented Author, or a fabricated Question Revision Number.

Both kinds are available to Instructors regardless of use in a private Course Instance. Students
receive authorized content through Coursework. Knowing an ID grants no access by itself.
See [AUTHORIZATION_CONTRACTS.md](../AUTHORIZATION_CONTRACTS.md).

## Boundaries and examples

- Searching for a Topic matches the Question's own Topic or the Pool's own Topic. A matching
  member does not make its Pool match.
- A Pool of Numeric WeBWorK Questions matches the Numeric and WeBWorK filters through its common
  required values. It is not excluded merely because it is a Pool.
- An Author filter applies to Question authors. Pool ownership is a different field.
- Changing a Question Title changes shared search metadata. Changing its answer creates a
  Question Revision. Saving changes to the set of Question Revision Tuples advances the Pool's
  Edit Number.
- An Assessment can use either kind. Retained Student Work identifies the actual Question
  Revision delivered, including Pool selection evidence where applicable.

These distinctions are owned by [QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md),
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md), and
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md). Shared identity is defined
once in [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md).
