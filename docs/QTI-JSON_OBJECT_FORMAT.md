# PLE Question JSON

Status: accepted schema-version-2 source contract. Schema version 2 implements all eight required
PLE Question JSON Question Types through strict parsing, answer-free compilation,
publication validation, student rendering, response validation, and isolated
server grading.

## Decision

Peptidyle uses its own small, versioned JSON source format for ordinary static
questions. The contract is named **PLE Question JSON**, not QTI JSON. QTI
is an import/export adapter and archival interchange format; it does not define
the internal model.

Version 2 uses a closed type-specific `response` object. PLE does not add QTI
expression trees, arbitrary response processing, or vendor extension containers.

## Required Question Type roadmap

The complete product must support at least these eight PLE Question JSON Question Types:

- multiple choice (MC), implemented by `singleChoice`;
- multiple answer (MA);
- fill-in-the-blank (FIB);
- multiple fill-in-the-blank (MULTI-FIB);
- numerical entry (NUM);
- matching (MATCH);
- ordered list (ORDER); and
- image hot spot (HOTSPOT).

Version 2 is based on the lossless semantics of QTI Package Maker's `MC`,
`MA`, `MATCH`, `NUM`, `FIB`, `MULTI_FIB`, and `ORDER` item models: visible content and stable item
identifiers compile separately from accepted answers. PLE adds one bounded
`hotspot` extension because the reviewed QTI Package Maker model does not
define that family. A wholesale Rust port of QTI Package Maker remains out of
scope.

This is an internal PLE source contract, not an implementation of a missing or
future external QTI-JSONL specification. A later QTI-JSONL adapter may map an
accepted external record into these public/private compiler outputs, but it
must not silently reinterpret v2 source bytes.

## Stored example

The complete accepted record lives in
`crates/adapters/ple/tests/fixtures/ple_question_json_single_choice_schema_v2.json`.
Parser and compiler tests load that stored Question data while executable source owns behavior.

Choice IDs are semantic stable identifiers, not display labels such as `A`,
`B`, and `C`. The renderer may label or reorder choices without changing the
answer key, feedback binding, or response-distribution identity.

Prompt, choice, and feedback content is Markdown. It passes through the normal
sanitized content renderer; the format does not accept raw browser HTML or
executable content.

## Teaching support and assets

PLE follows the assessment meanings demonstrated by QTI v3 without importing
QTI's general response-processing language into the PLE Question JSON format. QTI
uses response and outcome processing to select inline, block, or modal feedback.
It represents a requested Hint through a distinct request response even though
the displayed Hint content may live in a QTI feedback block. QTI represents
correct-response facts separately from a model solution, even when displayed
solution content lives in a feedback block, and packages referenced images or
other media as explicit files or resources.

PLE maps those meanings to narrower contracts:

| Reviewed QTI meaning                               | PLE contract                     | PLE timing and ownership                                     |
| -------------------------------------------------- | -------------------------------- | ------------------------------------------------------------ |
| Requested Hint                                     | Question Hint                    | Issued-Question-bound support before response or grading     |
| Feedback selected by a response item               | Choice Feedback                  | Post-grading Question Feedback                               |
| Feedback selected by a correct outcome             | Correct Feedback                 | Post-grading Question Feedback                               |
| Feedback selected by an incorrect outcome          | Incorrect Feedback               | Post-grading Question Feedback                               |
| Correct-response declaration                       | Answer Key, then Question Answer | Private grading facts, then separately released display form |
| Model solution                                     | Question Answer Explanation      | Explanatory content with independent release timing          |
| Item, Hint, Feedback, Answer, or explanation media | Exact Question Asset role        | Checksummed Object bound to the exact Question Revision       |

A QTI Hint request maps to Question Hint even when QTI uses a feedback block as
its display container. A correct-response declaration supplies private Answer
Key facts and a trusted source for the display-ready Question Answer. A QTI
model-solution block maps separately to Question Answer Explanation. A QTI item
whose adaptive response processing cannot preserve these meanings remains in
the QTI Question Format or produces an explicit unsupported-feature result. The
adapter preserves the exact instructional meaning.

The current PLE Question JSON schema-version-2 source implements Choice Feedback through
`response.choices[].feedback`, Correct Feedback through `feedback.correct`, and
Incorrect Feedback through `feedback.incorrect`. Its accepted-answer members
compile into the Answer Key. A dedicated Question Answer output, authored
Question Hint, and authored Question Answer Explanation are open migration
tasks. Unknown members remain invalid; the dedicated runtime Question Hint
capability stays separate from the PLE Question JSON schema-version-2 source contract until that
migration lands.

The current format supports one image-bearing source shape: the HOTSPOT
`surface` binds an exact Question Asset Reference, checksum, and accessible
description. Prompt, choice, Hint, Feedback, Answer, and Answer Explanation
fields currently accept text content rather than file paths or browser URLs.
Their future image or file support uses explicit Question Hint Asset, Question
Feedback Asset, Question Answer Asset, or Question Answer Explanation Asset
relationships and retains the same checksum and accessibility requirements.

## Version 2 contract

Version 2 keeps the same top-level metadata and policies but places family data
inside one closed `response` object. The common top-level members are
`format`, `version`, `title`, `prompt`, `response`, optional `feedback`,
`points`, `questionAttemptLimit`, `questionAttemptTimeLimit`, optional `tags`, optional
`classifications`, `license`, and `language`. Unknown and duplicate members are
refused at every level.

`questionAttemptLimit` is closed and contains only `maxAttempts`, which controls the
retry bound. It does not disclose results, feedback, or answers. Student
Feedback Release is assignment-owned through the target independent six-field
`StudentFeedbackReleaseRule`: score, per-item correctness, feedback text,
Question Answer, Question Answer Explanation, and class statistics. The current
combined `solution` field remains an open migration item.
The server derives Student Feedback for the authorized read. The Assignment
Revision retains the rule, while the Grading Result and exact Question records
retain the durable facts.

The eight exact response shapes are:

| `response.kind`  | Answer-bearing source members                                              | Browser-safe compiled shape                                        |
| ---------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `singleChoice`   | `choices`, `correctChoice`                                                 | radio choices with stable IDs                                      |
| `multipleAnswer` | `choices`, `correctChoices`                                                | checkbox choices; the correct count is not disclosed               |
| `fillIn`         | `answers`, `matchMode`, `maxLength`                                        | one bounded text field                                             |
| `multiFillIn`    | `blanks`, each with `id`, `label`, `answers`, `matchMode`, and `maxLength` | named bounded text fields                                          |
| `numeric`        | `answer`, `tolerance`, optional `unit`                                     | numeric field, public tolerance rule, and unit                     |
| `matching`       | `prompts`, `choices`, `matches`                                            | one accessible radio group per prompt                              |
| `ordering`       | `items`, `correctOrder`                                                    | one reorderable list                                               |
| `hotspot`        | `surface`, `regions`, `correctRegions`                                     | immutable image reference plus keyboard-operable Hotspot Regions   |

Choice, prompt, blank, ordering-item, and region identifiers use the same
stable identifier grammar. They identify semantics, not
display positions. The server may later project attempt-specific rendered item
IDs at the student wire boundary without changing these durable source IDs.

For example, a matching question is:

```json
{
  "format": "pleQuestionJson",
  "version": 2,
  "title": "Nucleic-acid sugars",
  "prompt": "Match each nucleic acid with its sugar.",
  "response": {
    "kind": "matching",
    "prompts": [
      { "id": "dna", "text": "DNA" },
      { "id": "rna", "text": "RNA" }
    ],
    "choices": [
      { "id": "deoxy", "text": "Deoxyribose" },
      { "id": "ribose", "text": "Ribose" }
    ],
    "matches": [
      { "prompt": "dna", "choice": "deoxy" },
      { "prompt": "rna", "choice": "ribose" }
    ]
  },
  "points": 2.0,
  "questionAttemptLimit": {
    "maxAttempts": null
  },
  "questionAttemptTimeLimit": { "kind": "unlimited" },
  "tags": ["nucleic-acids"],
  "classifications": [],
  "questionLicense": "CC-BY-SA-4.0",
  "language": "en-US"
}
```

Text matching is one of `exact`, `caseInsensitive`, or `normalized`. Numeric
tolerance is one of `exact`, `absolute` with nonnegative finite `epsilon`,
`relative` with nonnegative finite `fraction`, or `significantFigures` with a
positive `digits` count.

Hotspot surfaces name an existing immutable asset UUID, its lowercase SHA-256
checksum, and a nonblank description. Current Hotspot Regions are rectangles
with integer coordinates from 0 through 10,000, independent of browser pixels.
Each rectangle must be nonempty, contained by that normalized surface, and
nonoverlapping. Hotspot Region labels are the primary no-mouse response path;
the image itself is not the only way to identify a region.

## Compilation and security boundary

The authored document contains answers. It is never a student, public,
ordinary-browser-contract, or Wasm payload. The one narrow exception is an
authenticated author-role instructor requesting that instructor's own private
workspace source through the dedicated canonical-source `GET`/`PUT` route;
that route is `no-store`, uses a strong ETag, and does not expose a signed
object URL or checksum. The PLE Question Backend parses it once and produces two
independently checksummed values:

```text
answer-bearing PLE JSON
          |
          v
 strict native compiler
       /        \
      v          v
public question  private answer and feedback records
model            answer key + three feedback forms
```

| Value                        | Storage and readers                                                                       | Contents                                                                                                          |
| ---------------------------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Authoring source             | Private workspace source; authenticated author-source route and server-side compiler only | The complete PLE document, including accepted answers, pairings, regions, order, and feedback                     |
| Published source             | Immutable private Question Source object                                                   | The canonical PLE JSON promoted at publication for source recovery and exact re-import                             |
| Public compiled model        | Checksummed public Question Revision projection                                            | Prompt, choices, policies, points, Question Classifications, license, and language; no answer or private feedback |
| Private compiled records     | Checksummed grader-only `answer_key` JSONB                                                | Answer Key, Choice Feedback, Correct Feedback, Incorrect Feedback, schema version, and exact public-model binding |
| Search and identity metadata | Normal relational columns                                                                 | IDs, title, lifecycle, visibility, and indexed browse fields                                                      |

The private Answer Key and Question Feedback record carries the SHA-256 binding
of the public model. Grading
refuses a different prompt, choice set, policy, metadata record, source family,
or Question Grading Rule. Authored and published source objects are private source
records and cannot receive signed delivery URLs. Publication IDs are minted
only after both compiled halves validate successfully.

This split is more important than the physical JSON representation: a single
combined JSONB row readable by the student path would violate the grading
boundary even if the application promised not to serialize certain members.

## Why JSON rather than YAML

Canonical JSON is the machine contract because it has one relevant data model
across Rust `serde`, PostgreSQL JSONB, and browser tooling. The codec can emit
compact deterministic bytes for checksums, reject duplicate or unknown members,
and avoid YAML-specific implicit types, aliases, tags, and merge behavior.

Parsing speed is a minor benefit, not the architectural reason. These documents
are small and are compiled at authoring/publication time rather than reparsed on
every student request.

YAML may be added later as a human-editing input or export. If it is added, a
bounded YAML reader must translate it once into this exact typed contract. The
canonical JSON and its checksum remain authoritative; YAML never becomes a
second persisted interpretation of the same question.

## Validation

The native codec currently enforces these bounds:

- the complete source is at most 256 KiB;
- the exact format and schema version are required;
- unknown and duplicate members are rejected, including nested policies,
  Question Classifications, and Question Licenses;
- a choice question has 2 through 100 choices; `singleChoice` has exactly one correct choice;
- choice IDs start with a lowercase ASCII letter, use only lowercase letters,
  digits, `_`, or `-`, are unique, and are at most 64 bytes;
- prompt, choice, title, tag, classification system, classification code, classification name, language, and
  license text is nonblank and bounded;
- Choice, Correct, and Incorrect Feedback is optional; when present, it is nonblank and bounded;
- points are finite and nonnegative, using the shared `f64` score model; and
- `maxAttempts` is positive or `null` for unlimited attempts.

The v2 contract additionally enforces exact Question-Type-specific bindings: accepted text
answers are nonempty and unique; multi-blank IDs and answers are complete;
numeric answers and tolerance parameters are finite; matching binds every
prompt once to one unique available choice; ordering names every item exactly
once; and hotspot assets, checksums, rectangles, accessible labels, and correct
region subsets are complete and internally consistent.

Choice Feedback is selected for the submitted choice. Correct or Incorrect
Feedback is appended according to the server-derived grade. The
assignment-owned Student Feedback Release Rule decides whether and when the
Student receives that teaching content.

Canonicalization preserves choice order because order is authored behavior.
Whitespace and JSON object-member order do not change the canonical checksum.

## Evolution and QTI adapters

Version 2 is the only current PLE Question Source and reader. Its closed shape is
parsed exactly: no legacy PLE Question JSON schema-version-1 reader, upcaster, source-byte
fallback, or republishing path is retained. Additive optional members require
review against the v2 contract; incompatible future semantics use a new explicit
version with its own reader and migration plan rather than reinterpreting v2 bytes.

Canvas QTI and Blackboard QTI remain separate import/export profiles. Each
adapter may map the supported PLE Question JSON-supported subset into the same public/private compiler
outputs, retain the original package for provenance, and record unsupported
features. Vendor-specific XML is not copied into the PLE Question JSON schema
merely because one exporter emits it.

The native parser/compiler facade is
`crates/adapters/ple/src/question_json.rs`; schema-version-2 shapes and
compilation live in `crates/adapters/ple/src/question_json/schema_v2.rs`.
The persistence boundary is `crates/learning-data-access/src/question_json.rs`
with focused in-memory and PostgreSQL implementations, and the server owner is
`crates/server/src/question_json_publication.rs`. The private source saves
atomically with its typed draft, publication copies its exact canonical bytes
to an immutable non-signable source object, and the runtime obtains private
Answer Keys and Question Grading Input only through an injected grading
capability. The instructor editor is
complete; bounded Canvas/Blackboard QTI profile mappings, profile-to-PLE conversion, and their
live and independent-review gates are accepted. The remaining visual authoring,
external QTI-JSONL, pilot-content, and hotspot pointer-overlay work is tracked
without weakening this accepted internal source and runtime contract.
