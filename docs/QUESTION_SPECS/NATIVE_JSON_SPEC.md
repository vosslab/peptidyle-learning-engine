# Native JSON Question specification

## Purpose and boundary

Native JSON is PLE's private `pleQuestionJson` source format for the `ple` Question
Backend. It is not QTI, a browser payload, or a public interchange format. PLE validates this
source and freezes it into a Published Question Revision; Students receive an answer-free view.

Question metadata belongs on the Question record, not in Native JSON, for both Draft and Published
Questions. Native JSON contains the Question content needed to display and grade it.
Metadata corrections update the Question record under its existing edit and Revision rules. See
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md) and
[QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md).

## Backend content fields

The fields and validation rules below describe complete source for publication. A Native JSON
Draft can be empty, incomplete, or broken and still be saved. Preview and testing report errors
so the Instructor can continue editing; publication requires valid complete source.

The Backend content has a closed shape. Unknown or duplicate members are invalid. The table
describes Backend-owned fields; the existing decoder also carries duplicated shared metadata,
as documented under Current implementation evidence below.

| Field | Type and rule |
| --- | --- |
| `format` | Required literal `pleQuestionJson`. |
| `prompt` | Required student-facing prompt. |
| `response` | Required one of the eight response objects below. |
| `questionHint` | Optional text, or `null`; distinct from outcome feedback. |
| `feedback` | Optional object with nullable `correct` and `incorrect` text; omission defaults to both `null`. |
| `externalResources` | Optional list of declared external resources; omission defaults to `[]`. |
| `authorScript` | Optional isolated-author-script declaration; omission defaults to `null`. |

Points, availability, Attempt limits, timing, owner, author, public Question ID, and Question
Revision are not Native JSON fields. Their owners are the Assessment, lifecycle, or metadata specs.

## Current implementation evidence: metadata

The existing [source_document.rs](../../crates/adapters/ple/src/question_json/source_document.rs)
includes `questionTitle`, `questionDescription`, `tags`, `questionLicense`, and `questionCitation`
in its source document. These duplicate shared Question metadata. The decoder, authoring,
publication, and read paths need alignment with the Question record's ownership of those fields;
see [TODO.md](../TODO.md#question-spec-implementation-follow-up).

These fields describe the current encoding, not a second authority for metadata. This document
does not introduce a replacement authoring API or database schema.

## Current implementation evidence: language

The existing [source_document.rs](../../crates/adapters/ple/src/question_json/source_document.rs)
decodes `language` as a string and validates it as nonblank metadata text. HG does not require
Question language. This current-code constraint does not establish an Instructor input or
publication requirement; reconciliation belongs in [TODO.md](../TODO.md).

## Response shapes and grading

The Backend always calculates the earned credit fraction below, and PLE stores it regardless of
whether the Assessment awards partial credit. Apply the Assessment setting when calculating
points from that fraction; see
[QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md#stored-credit-and-awarded-points).

| `response.kind` | Required answer-bearing fields | Grading rule |
| --- | --- | --- |
| `singleChoice` | `choices`, `correctChoice`; optional `randomizeChoices` defaults to `false` | all-or-nothing: selected stable ID equals the correct ID. |
| `multipleAnswer` | `choices`, `correctChoices`; optional `randomizeChoices` defaults to `false` | partial-credit scoring rule in [MULTIPLE_ANSWER_SCORING_SPEC.md](MULTIPLE_ANSWER_SCORING_SPEC.md). |
| `fillIn` | `answers`, `matchMode`, `maxLength` | one response matches an accepted answer or author-supplied regular expression. |
| `multiFillIn` | `blanks` with `id`, `label`, `answers`, `matchMode`, `maxLength` | grade each blank as a FIB; credit is correct blanks divided by all blanks. |
| `numeric` | `answer`, `tolerance`; optional `unit` defaults to `null` | finite value meets declared tolerance. |
| `matching` | `prompts`, `choices`, `matches` | correct pairs divided by the total number of prompts; wrong or blank pairs earn zero, without an additional deduction. |
| `ordering` | `items`, `correctOrder` | average the correct-position and correctly ordered-pair fractions; see [ORDER_SCORING_SPEC.md](ORDER_SCORING_SPEC.md). |
| `hotspot` | `surface`, `regions`, `correctRegions` | all-or-nothing: selected region-ID set equals correct set. |

Matching calculates proportional credit: three correct
pairs out of five earn `0.6`. Every prompt
has equal weight, including unanswered prompts in the denominator. Calculate the fraction before
applying Assessment points. Shuffling display order does not change the keyed identities.

The Native JSON Multiple Answer scoring document owns its formula, worked examples, and
scoring principles. Assessment Instructors control whether partial credit is awarded.
Each Question Backend owns its grading calculation.

Multiple Fill-in-the-Blank Questions combine independent FIBs. Each blank uses its own accepted
answers and the same regular-expression support as a single FIB. Blanks have equal weight: three correct out of five earn `0.6`. Wrong or unanswered
blanks earn zero while remaining in the denominator. Valid saved MATCH and MULTI-FIB responses
can contain unanswered parts; preserve those responses for submission and grading.

The current Native JSON implementation still returns only zero or one and rejects incomplete
Matching responses. Matching, Multiple Answer, MULTI-FIB, and ORDER partial-credit grading need
alignment; see [TODO.md](../TODO.md#question-spec-implementation-follow-up). The ORDER formula,
pair-counting explanation, and worked examples belong to
[ORDER_SCORING_SPEC.md](ORDER_SCORING_SPEC.md).
MC and HOTSPOT all-or-nothing grading is explicitly confirmed.

FIB supports lists of accepted answers and author-supplied regular expressions. MULTI-FIB uses
that same FIB behavior for every blank. Current text matching implements `exact`,
`caseInsensitive`, and `normalized`; regex support is an implementation gap. Numeric tolerance is `exact`, `absolute` with nonnegative
finite `epsilon`, `relative` with nonnegative finite `fraction`, or `significantFigures` with
positive `digits`.

Response-item IDs identify meaning rather than display position. Display shuffling preserves
the answer key's identity relationships.

### Current implementation limits

[source_compile.rs](../../crates/adapters/ple/src/question_json/source_compile.rs) currently accepts
2-100 choices, at least two Matching prompts, and at least three Ordering items. Its Matching
mapping binds each prompt to a distinct choice. HOTSPOT regions are nonempty, nonoverlapping
rectangles in a 0--10,000 coordinate space. Multi-blank slots have unique, nonempty IDs.

These describe the current compiler. HG establishes the supported Types and grading rules, not
those minimum counts, a universal one-to-one Matching restriction, or rectangular-only HOTSPOT
content. The ORDER examples covering three through seven items do not establish a size limit.
Supported-input limitations should be reported accurately rather than presented as approved
teaching requirements. The existing compiler limits are unchanged in this documentation pass.

## MATCH response presentation

On laptop and desktop screens, show MATCH prompts with one shared choice bank containing the full
set of choices. Support drag-and-drop and an equally capable keyboard-only way to assign, change,
and clear matches. Keep each prompt's assigned choice easy to recognize and the choice bank
reachable with keyboard, pointer, or touch.

Layouts may adapt to available space, including repeating choices on narrow screens when useful.
They preserve the same content, response meaning, and grading behavior. These requirements come
from [HG's Native Question response presentation](../HUMAN_GUIDANCE.md#native-question-response-presentation).

## Presentation, images, scripts, and randomization

The compiler derives an answer-free presentation. Answer Keys, accepted answers, matching pairs,
correct order, correct regions, and protected feedback never enter its normal browser shape.
Native JSON receives no backend seed. `randomizeChoices: true` changes only authored choice display
using issued presentation state; it does not change stable IDs or grading.

An image uses a Question Image Asset reference tied to the Revision: asset ID, lowercase SHA-256 checksum,
and accessible description. A hotspot surface must reference an uploaded valid asset. Author
JavaScript may support rendering or interaction only in an isolated untrusted environment. It has
no credentials, private source, Answer Key, or grading authority. Its dependency list is closed;
current declared library support is `rdkit`. That current library-name list is separate from the
HG requirement to record and review external JavaScript dependencies and their CDN domains.
Approved dependencies may initially load from recorded CDN sources; supported dependencies should
eventually be owned and served locally by PLE. Recording a resource alone does not approve it.
Native HOTSPOT interactions use PLE-owned code and support still images and SVG.

## Validation, publication, and evolution

Native source is at most 256 KiB and is strictly parsed before publication. The server creates an
answer-free public-content checksum and binds private grading facts to it. A change to source,
Answer Key, feedback, or image assets creates a new Published Question Revision.
Title, Description, classification, Tags, and Bloom follow the ordinary in-place metadata rules
on the Question record. Their current duplication in serialized source is an implementation gap,
not a reason to require another Revision for those edits.

Native JSON has one strictly validated, unversioned source shape. `format` identifies the format,
not a version. A shape change migrates stored native Questions with every reader, writer,
validator, and compiler. QTI conversion is governed by
[QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md).

## Response members and Student responses

`choices` are objects with stable `id`, displayed `text`, and nullable private `feedback`.
Matching prompts, matching choices, and ordering items have stable `id` and displayed `text`.
`matches` are private `{ "prompt": "id", "choice": "id" }` bindings. A multi-blank entry has
`id`, displayed `label`, accepted `answers`, `matchMode`, and `maxLength`. A numeric `unit` is a
string or `null`. A hotspot `surface` has a Question Image Asset ID, checksum, and nonblank
description; each region has `id`, label, `x`, `y`, `width`, and `height`.

The answer-bearing document is never Student-facing. The corresponding accepted Student response
has the matching shape below; the server validates it against the issued answer-free presentation:

| Source kind | Student response |
| --- | --- |
| `singleChoice` | `{ "kind":"multipleChoice", "selected":["choice-id"] }` |
| `multipleAnswer` | `{ "kind":"multipleChoice", "selected":["choice-id"] }` |
| `fillIn` | `{ "kind":"shortText", "text":"answer" }` |
| `multiFillIn` | `{ "kind":"multiBlank", "answers":[{"slot":"blank-id","text":"answer"}] }` |
| `numeric` | `{ "kind":"numeric", "value":1.5 }` |
| `matching` | `{ "kind":"matching", "matches":[{"prompt":"prompt-id","choice":"choice-id"}] }` |
| `ordering` | `{ "kind":"ordering", "order":["item-id"] }` |
| `hotspot` | `{ "kind":"hotspot", "selections":[{"region":"region-id"}] }` |

Matching permits unanswered prompts; they earn zero while answered pairs are graded normally.
The current response validation must be updated to support that rule. A response cannot name an
unavailable ID, duplicate a stable prompt ID, or supply a different response kind. Other Types
retain their applicable response requirements pending any separately approved grading changes.

## Eight minimal source examples

These fragments are the `response` member of a valid document. Common required top-level fields
remain as defined above.

The following Matching example illustrates Backend content with shared metadata on the Question
record. Its Title could be "Nucleic-acid sugars" and its Description "Match nucleic acids to their
characteristic sugars." Publication also validates the record's license and required metadata.
The current decoder still requires the extra fields described under implementation evidence;
this example shows the intended separation, not a currently accepted complete API request:

```json
{"format":"pleQuestionJson","prompt":"Match each nucleic acid with its sugar.","response":{"kind":"matching","prompts":[{"id":"dna","text":"DNA"},{"id":"rna","text":"RNA"}],"choices":[{"id":"deoxy","text":"Deoxyribose"},{"id":"ribose","text":"Ribose"}],"matches":[{"prompt":"dna","choice":"deoxy"},{"prompt":"rna","choice":"ribose"}]}}
```

```json
{"kind":"singleChoice","choices":[{"id":"a","text":"A","feedback":null},{"id":"b","text":"B","feedback":null}],"correctChoice":"a","randomizeChoices":true}
```

```json
{"kind":"multipleAnswer","choices":[{"id":"a","text":"A","feedback":null},{"id":"b","text":"B","feedback":null}],"correctChoices":["a"],"randomizeChoices":true}
```

```json
{"kind":"fillIn","answers":["DNA"],"matchMode":"caseInsensitive","maxLength":32}
```

```json
{"kind":"multiFillIn","blanks":[{"id":"first","label":"First","answers":["DNA"],"matchMode":"exact","maxLength":32}]}
```

```json
{"kind":"numeric","answer":7,"tolerance":{"kind":"absolute","epsilon":0.1},"unit":null}
```

```json
{"kind":"matching","prompts":[{"id":"dna","text":"DNA"},{"id":"rna","text":"RNA"}],"choices":[{"id":"deoxy","text":"Deoxyribose"},{"id":"ribose","text":"Ribose"}],"matches":[{"prompt":"dna","choice":"deoxy"},{"prompt":"rna","choice":"ribose"}]}
```

```json
{"kind":"ordering","items":[{"id":"one","text":"one"},{"id":"two","text":"two"},{"id":"three","text":"three"}],"correctOrder":["one","two","three"]}
```

```json
{"kind":"hotspot","surface":{"questionImageAssetId":"018f2fbb-36ec-7cba-a840-1a2b3c4d5e70","checksum":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","description":"Cell diagram"},"regions":[{"id":"nucleus","label":"Nucleus","x":0,"y":0,"width":100,"height":100}],"correctRegions":["nucleus"]}
```

## Protected compilation boundary

Publication validates complete private source and produces an immutable private source binding for
the Revision. It separately derives answer-free public content and binds private Answer Key and
Question Feedback to that content's checksum. Browser, public preview, and Wasm response code use
only answer-free presentation. The server obtains private source and grading material only through
the selected Backend's trusted capability.

The `externalResources` list is author-declared inventory metadata with `url` and one of `link`,
`image`, `stylesheet`, `script`, or `other`; declaring an entry neither fetches it nor grants
execution. `authorScript` has source text and a closed `libraries` list; it has no dependency URL,
version, or digest fields. It stays in an isolated untrusted runtime and cannot become a grading,
authorization, or private-data authority.

## Source validation detail

The source format field is exactly `pleQuestionJson`; complete source is at most 256 KiB. The
parser rejects duplicate and unknown members at every level. Stable choice, prompt, blank, ordering,
and region IDs begin with a lowercase ASCII letter and contain only lowercase ASCII letters,
digits, `_`, and `-`; they are at most 64 bytes. IDs are unique in their response collection.

Accepted text values are nonempty and unique. Exact matching compares as written; case-insensitive
matching ignores ASCII case; normalized matching trims leading/trailing whitespace, collapses
whitespace runs to one space, and lowercases text. Numeric answers and
tolerance values are finite. Absolute tolerance accepts `abs(actual - expected) <= epsilon`;
relative tolerance accepts `abs(actual - expected) <= abs(expected) * fraction`; significant
figures compares values after rounding to declared positive digits. Assessment points and timing
are outside this source.
