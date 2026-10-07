# Question Library specification

The Question Library is one global collection of Published Questions and Question Pools.
Search and Browse are two entry paths into the same collection. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-library-specifications).

## Access and scope

Active Instructors discover and reuse both kinds. Draft Questions remain private working content
outside the Library. Using content in a private Course Instance does not make the Library Object
private. Students access Question content through authorized Coursework, not Library search.
Sysadmin support access follows [AUTHORIZATION_CONTRACTS.md](../AUTHORIZATION_CONTRACTS.md).

An archived Question is read-only and leaves normal discovery while its existing references
remain intact. It can be restored or forked; see
[PUBLISHED_QUESTION_SPEC.md](PUBLISHED_QUESTION_SPEC.md#archive).
Pool availability follows [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).

## Question Library entry points

The Questions Ribbon includes My Questions, My Draft Questions, Starred, Watched, Search Question
Library, and Browse Question Library. My Questions supports finding and managing the Instructor's
Published Questions; My Draft Questions emphasizes unfinished private work. Starred provides a
quick personal collection, and Watched follows relevant changes. These views retain their distinct
tasks while using the same Question and Pool meanings.

## Common search result

Every result represents a Library Object. Shared result facts have the same meaning for both
kinds, even when internal storage differs. Backend source and correct answers are not needed
for ordinary discovery.

| Common fact | Required meaning |
| --- | --- |
| Kind and Question ID | Identify which Library Object the result opens |
| Title and Description | The object's own text, using one canonical Title |
| Owner | The object's owner, distinct from Question authors |
| Discipline, Subject, optional Topic/Subtopic | The object's own classification |
| Tags | The object's own labels, possibly empty |
| Question Type and Backend | Exact Question values or the Pool's common member values |
| License | Question license or the Pool's calculated compatible license |
| Bloom | Exact Question Revision's pair or Pool's pair; NULL is legitimate |
| Available actions | Only actions the current Instructor may perform |

Question-specific facts include authors and the exact Question Revision represented. Pool-specific
facts include member count, source Pool where present, and Pool Edit Number. Preserve those
distinctions. Carry shared facts through API decoding to display instead of discarding them and
reconstructing them differently by kind. The field definitions are in
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md).

The table defines available result facts, not a demand to show every fact in every compact row.
Display enough information to judge relevance without opening each object. IDs remain visually
subordinate to Titles. Owner and Author labels keep their distinct meanings.

## Shared display and selection

The reusable spreadsheet-style interface supports exploration before entering search text and
supports refinement after simple search. It offers Compact, List, and Visual boxes, with the same
results, filters, selection meanings, and paging. Visual boxes help with image-based Questions.
Exact display names remain flexible where HG describes them as examples.

List items open in a new tab or window. Tooltips provide brief text on hover and keyboard focus;
expensive hover previews are not part of the required design. Navigation and search lifetime are
owned by [QUESTION_LIBRARY_SEARCH_SPEC.md](QUESTION_LIBRARY_SEARCH_SPEC.md).

Assessment editors reuse combined discovery to choose Library Objects. Committing a Question
selection retains its exact Revision. Adding a Pool stores its existing Pool ID and the Assessment
entry's selection count. An Instructor explicitly forks a Pool when independent customization is
wanted; see [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).
The picker for adding Questions to a Pool offers Published Questions only. Assessment
editors provide direct Search and Browse paths, inspection before adding, and bulk addition by
Question ID. The shared interface must support these workflows for both Library Object kinds.
Current picker limitations are implementation evidence; they do not establish a product rule
that requires Questions and Pools to be added separately.

## Stars, Watches, and proposals

Library Objects can be Starred and Watched. Star means favorite and visible endorsement: an Instructor
can Star or unstar, see the count, and see which Instructors Starred accessible content. Use
"who Starred," not "endorsement list." Instructor identity lists and Watch information are not
exposed to Students or anonymous users.

Watch means subscription. A Question Watch covers new Revisions and forks.
Instructors watching a Pool receive notifications when its Questions change or when it is forked.
The Instructor's Watch list is private.

An Instructor who finds a problem in another Instructor's Library Object can fork it and fix it.
Use Change Proposals as the Instructor-facing term for proposed content changes. Owner and
authors remain defined in
[QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md).

## Usage statistics

Published Questions retain privacy-safe aggregate statistics per Revision; Pools retain their
own aggregate statistics. They show how often Students received each Published Question Revision
or Questions from each Pool and how much credit they earned. Count a delivery even when the
Attempt has not been submitted. Credit
statistics use graded responses; delivery alone is not a graded response. Pool statistics
accumulate from Questions delivered through that Pool, including across changes to its set of
Question Revision Tuples.

For graded responses, show:

| Measure | Meaning |
| --- | --- |
| Graded responses | Number of responses contributing to the credit statistics |
| Average credit | Sum of stored credit fractions divided by the graded-response count |
| Full-credit percentage | Percentage of those graded responses with stored credit equal to one |
| Zero-credit percentage | Percentage of those graded responses with stored credit equal to zero |

These measures use the Question Backend's stored credit fraction. Assessment point values and
partial-credit settings determine awarded Assessment points separately. Responses between zero
and full credit form the remainder; the agreed display uses the four measures above.

The average can hide different outcomes: everyone earning half credit and half earning full
credit while half earn zero have the same average. The full-credit and zero-credit percentages
make that difference visible.

Aggregates contain counts and sums, not Student Attempts or identifiable records. Removing names
alone is insufficient. Show shared statistics only when individual Students cannot reasonably
be inferred. Course-local and small-group analysis remains protected by
[FERPA_DATA_POLICY.md](../FERPA_DATA_POLICY.md). Retention must satisfy that policy before
aggregates survive deletion of Student records.

Instructor bulk metadata editing is deferred. That does not defer ordinary individual metadata
edits or selecting content for Assessment assembly.

## Related workflows

- [QUESTION_LIBRARY_SEARCH_SPEC.md](QUESTION_LIBRARY_SEARCH_SPEC.md): Search and Browse, syntax, sorting, paging, navigation.
- [QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md): filter meanings, including Questions in no Pool.
- [QUESTION_LIBRARY_BULK_EDIT_SPEC.md](QUESTION_LIBRARY_BULK_EDIT_SPEC.md): cleanup of many Library Objects.
