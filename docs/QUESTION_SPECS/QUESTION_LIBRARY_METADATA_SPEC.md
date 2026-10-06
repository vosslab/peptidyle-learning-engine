# Question Library metadata specification

This document owns the field inventory shared by Published Questions and Question Pools. It
describes fields on complete Question Revision records and current Pool records. Permission to
edit a field in place is separate from whether that field belongs to the record.
Authority: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-library-metadata).

Question metadata belongs on the Question record for every Backend. Native JSON contains the
Question content needed to display and grade it. The sections below define the metadata fields
and their assignment, editing, and Revision rules.

Existing duplicate metadata in source encoding is implementation work recorded in
[TODO.md](../TODO.md#question-spec-implementation-follow-up).

## Required and assigned values

PLE assigns values it can determine and validates the remaining required content before Library
entry. Instructors should not have to re-enter a Pool's common Type, Backend, or calculated
license. Importers preserve usable source metadata and identify missing required values.

Required means a value is present and valid, not merely that the browser shows an input box.
Check Question publication, Pool creation, and edits to Library Objects. Drafts have no content
or metadata requirements; imported Drafts follow that same rule. A missing required field blocks
publication. Bloom is an explicit exception: absence while awaiting AI is permitted indefinitely.
An empty string is not a substitute for absent optional text or a valid required value.

A Question's owner and Sysadmin can edit its Title, Description, Discipline, Subject, Topic,
Subtopic, and Tags. Other Instructors may read, reuse, or fork the Question, but may not edit it.
Automated metadata assignment remains desirable because Instructors will do the bare minimum
on metadata; it does not establish an additional editor role. See
[QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).

## Shared field table

Names in the first column are product terms, not a proposed JSON schema. Concrete request names
belong to [QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md) and
[QUESTION_LIBRARY_BULK_EDIT_SPEC.md](QUESTION_LIBRARY_BULK_EDIT_SPEC.md).

| Field | Type and absence | Published Question location | Pool location and assignment | Change rule |
| --- | --- | --- | --- | --- |
| Question ID | Required canonical `XXXX-ZXXX` string | Stable Question identity | Stable Pool identity | PLE generates it; never changes or reuses it |
| Kind | Required Question or Pool | Published Question | Question Pool | Determined by the created object |
| Owner | Required Instructor identity | Question ownership | Pool ownership | Authority follows ownership rules, not author text |
| Title | Required nonempty text | Question Revision record | Pool's own metadata | Metadata edit, no Question Revision |
| Description | Required nonempty text | Question Revision record | Pool's own metadata | Metadata edit, no Question Revision |
| Discipline | Required vocabulary identity | Question Revision record | First member establishes it | Validate hierarchy and the requirements of affected Pools |
| Subject | Required vocabulary identity | Question Revision record | First member establishes it | Must associate with selected Discipline |
| Topic | Optional vocabulary identity; NULL when absent | Question Revision record | Pool's own Topic | If present, belongs to Subject |
| Subtopic | Optional vocabulary identity; NULL when absent | Question Revision record | Pool's own Subtopic | If present, belongs to selected Topic |
| Tags | Collection of labels; empty collection allowed, not NULL | Question Revision record | Pool's own Tags | Metadata edit; independent of hierarchy |
| Question Type | Required closed Type; never NULL | Question Revision record | Common member Type | Native JSON source supplies Type; other Backends use editable classification |
| Question Backend | Required closed Backend; never NULL | Question Revision record | Common member Backend | Selected execution system; not inferred from rendered controls |
| License | Required supported license | Question Revision record | Calculated from exact members | Preserve original member licenses; reject incompatible combinations |
| Bloom Cognitive Process | Optional until assigned; one of six values when present | Question Revision record | Classification of Pool as a whole | Ordinary in-place metadata edit |
| Bloom Knowledge Dimension | Optional until assigned; one of four values when present | Question Revision record | Classification of Pool as a whole | Independent meaning; ordinary metadata edit |
| Hint | Optional support text; NULL when absent | Question Revision record | Pool's own support text | Question changes create Revision; Pool support remains Pool state |
| Question Feedback | Optional support text; NULL when absent | Question Revision record | Pool's own support text | Timing decision deferred; do not infer timing from presence |
| Worked Solution | Optional support text; NULL when absent | Question Revision record | Pool's own support text | Separate disclosure from Hint and answer key |
| Usage statistics | Counts/sums; privacy-sensitive small groups withheld | Per Question Revision | Per Pool | Computed aggregates; never Student records |

Support text is PLE-managed and remains distinct from content generated by a Question Backend.
Putting support content on a Pool does not copy it into each member's source.

## Kind-specific fields

| Field | Meaning and absence | Assignment and changes |
| --- | --- | --- |
| Question Revision Number | Required positive integer for Published Question content | PLE starts at 1 and advances on accepted content publication |
| Question authors | Attribution entries; each has a display name and optional Account link | Preserve source authors; author and owner may differ |
| Question source and images | Backend-specific private source and exact Revision assets | Draft editing then immutable publication |
| Question fork source | Optional exact source reference, present for a published fork | PLE retains origin; not a live update link |
| Pool members | Unordered set of Question Revision Tuples for distinct Published Questions | When the set of Question Revision Tuples in a Pool changes, saving advances the Pool's Edit Number |
| Pool Edit Number | Required positive integer | PLE advances on changed Pool saves; a counter for saved changes |
| Source Pool | Optional Pool ID, present for a Pool fork | PLE records origin when creating the fork |
| Edit Number where needed | Ordinary record concurrency counter | Detect stale saves; identifies no historical record |

Pools have no separate Author field. A Pool selection count belongs to its Assessment entry,
not to global search metadata. HG establishes no required Question or Pool language field.

## Citation

Citation belongs on the Question record. Its format remains deferred until attribution and import
are worked through. HG does not yet choose a URL, text, DOI, or structured format.

## Concrete text constraints

The established storage/write contracts use the following bounds. These are technical limits,
not new teaching requirements. Values are trimmed and contain no control characters where noted.

| Value | Bounds |
| --- | --- |
| Question or Pool Title | 1-512 characters, trimmed, no control characters |
| Question or Pool Description | 1-4000 characters, trimmed, no control characters |
| Each present Hint, Question Feedback, Worked Solution | 1-4000 characters, trimmed, no control characters |
| Author display name | 1-120 characters, trimmed, no control characters |
| Citation (current implementation) | URL up to 2048 characters and/or text up to 4000; at least one nonempty when supplied |

The schema references are [published_question.sql](../../schemas/base_schema/20_tables/published_question.sql)
and [question_pool.sql](../../schemas/base_schema/20_tables/question_pool.sql). Classification
names and Tags have their own checks in
[QUESTION_CLASSIFICATION_SPEC.md](QUESTION_CLASSIFICATION_SPEC.md). Technical bounds must remain
consistent across API validation and persistence.

## Current implementation evidence: language

The existing [published_question.sql](../../schemas/base_schema/20_tables/published_question.sql)
requires non-NULL language text of 2-35 trimmed characters. This is an existing storage constraint,
not an approved product requirement. Required language was removed from the product field table;
implementation reconciliation is recorded in [TODO.md](../TODO.md).

## Ownership of detailed rules

- [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md): authority and attribution.
- [QUESTION_CLASSIFICATION_SPEC.md](QUESTION_CLASSIFICATION_SPEC.md): hierarchy and vocabulary.
- [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_BLOOM_CLASSIFICATION_SPEC.md): values, pending assignment, corrections.
- [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md): compatible member licenses.
- [QUESTION_TYPE_SPEC.md](QUESTION_TYPE_SPEC.md): Type meanings and declaration.
- [FERPA_DATA_POLICY.md](../FERPA_DATA_POLICY.md): statistics privacy and retention.

For example, a fully validated Question with no Bloom Classification may publish. A Question
with NULL `question_type` may not. A Pool may have Tags different from every member. It may not
use a missing license or a "Mixed" license to avoid checking compatibility.
