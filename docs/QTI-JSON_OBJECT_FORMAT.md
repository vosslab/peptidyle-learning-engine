# PLE flat-question JSON

Status: accepted v1 source contract; native parsing/compilation, atomic
workspace persistence, immutable publication, and isolated runtime grading are
implemented. The instructor editor, bounded Canvas/Blackboard profile import,
profile-to-native conversion, live PostgreSQL/RLS acceptance, and independent
documentation review are also complete.

## Decision

Peptidyle uses its own small, versioned JSON source format for ordinary static
questions. The contract is named **PLE flat-question JSON**, not QTI JSON. QTI
is an import/export adapter and archival interchange format; it does not define
the internal model.

Version 1 deliberately supports one exactly-one-choice question. A new question
shape or incompatible contract change receives a new explicit schema version.
PLE does not add optional QTI expression trees, arbitrary response processing,
or vendor extension containers to version 1.

## Required family roadmap

The complete product must support at least these eight flat-question families:

- multiple choice (MC), implemented by version 1 as `singleChoice`;
- multiple answer (MA);
- fill-in-the-blank (FIB);
- multiple fill-in-the-blank (MULTI-FIB);
- numerical entry (NUM);
- matching (MATCH);
- ordered list (ORDER); and
- image hot spot (HOTSPOT).

Version 1 remains the closed MC contract described below. WP-FQ-0 owns the QTI-JSONL specification,
reference engine, all-family example, and contract/round-trip tests in QTI Package Maker. PLE adopts
that accepted specification
through the integration boundary in
`docs/active_plans/active/flat_question_family_evolution_plan.md`
rather than freeze a competing version 2 source shape. MATCH is the first implementation vertical
because each initial Chapter 1 assignment calls for
one WeBWorK MATCH and one flat MATCH alongside their MC counterparts.

The local QTI Package Maker item model covers MC, MA, MATCH, NUM, FIB, MULTI-FIB, and ORDER; it does
not define HOTSPOT. Its print-oriented `exam_yaml` writer demonstrates a concise `statement`, list,
and table presentation but intentionally loses several answer keys. WP-FQ-0's QTI-JSONL specification
retain the lossless item semantics while keeping that readable spirit. `BaseItem` supplies optional
`feedback_correct` and `feedback_incorrect`, so missing outcome feedback must remain valid in PLE
too. A wholesale Rust port of QTI Package Maker is out of scope; PLE ports only the bounded
parser/compiler/export behavior required by WP-RC4 through WP-RC6.

WP-FQ-0 defines portable image, binary-reference, and HOTSPOT source semantics with normative fixtures.
PLE will resolve accepted references through an adapter-owned binding step while object storage owns
the bytes, checksums, media types, lifecycle, and authorization. WP-FQ-6 and WP-FQ-7 implement those
accepted fields and geometry without adding a second vocabulary.

## Example

```json
{
  "format": "pleFlatQuestion",
  "version": 1,
  "kind": "singleChoice",
  "title": "Favorite color",
  "prompt": "What is my favorite color?",
  "choices": [
    {
      "id": "blue",
      "text": "Blue",
      "feedback": "Blue is a calm choice."
    },
    {
      "id": "red",
      "text": "Red",
      "feedback": "Red is not my favorite."
    },
    {
      "id": "yellow",
      "text": "Yellow",
      "feedback": "Yellow is bright."
    }
  ],
  "correctChoice": "blue",
  "feedback": {
    "correct": "Exactly right.",
    "incorrect": "Try thinking of a cool color."
  },
  "points": 1.0,
  "attemptPolicy": {
    "maxAttempts": null,
    "feedback": "immediateFull"
  },
  "timingPolicy": {
    "kind": "untimed"
  },
  "tags": ["example"],
  "taxonomy": [],
  "license": {
    "kind": "ccBySa"
  },
  "language": "en-US"
}
```

Choice IDs are semantic stable identifiers, not display labels such as `A`,
`B`, and `C`. The renderer may label or reorder choices without changing the
answer key, feedback binding, or response-distribution identity.

Prompt, choice, and feedback content is Markdown. It passes through the normal
sanitized content renderer; the format does not accept raw browser HTML or
executable content.

## Compilation and security boundary

The authored document contains answers. It is never a learner, public,
ordinary-browser-contract, or Wasm payload. The one narrow exception is an
authenticated author-role instructor requesting that instructor's own private
workspace source through the dedicated canonical-source `GET`/`PUT` route;
that route is `no-store`, uses a strong ETag, and does not expose a signed
object URL or checksum. The native adapter parses it once and produces two
independently checksummed values:

```text
answer-bearing PLE JSON
          |
          v
 strict native compiler
       /        \
      v          v
public question  private grader material
model            answer key + feedback
```

| Value                        | Storage and readers                                                                       | Contents                                                                                           |
| ---------------------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Authoring source             | Private workspace source; authenticated author-source route and server-side compiler only | The complete PLE document, including `correctChoice` and feedback                                  |
| Published source             | Immutable private `ProblemSource` object                                                  | The canonical PLE JSON promoted at publication for provenance, recovery, and exact re-import       |
| Public compiled model        | Checksummed `problem_version_payload` JSONB                                               | Prompt, choices, policies, points, taxonomy, license, and language; no answer or private feedback  |
| Private compiled material    | Checksummed grader-only `answer_key` JSONB                                                | Answer key, per-choice and outcome feedback, schema version, and binding to the exact public model |
| Search and identity metadata | Normal relational columns                                                                 | IDs, title, lifecycle, visibility, and indexed browse fields                                       |

The private material carries the SHA-256 binding of the public model. Grading
refuses a different prompt, choice set, policy, metadata record, source family,
or grading definition. Authored and published source objects are private source
records and cannot receive signed delivery URLs. Publication IDs are minted
only after both compiled halves validate successfully.

This split is more important than the physical JSON representation: a single
combined JSONB row readable by the learner path would violate the grading
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

## Version 1 validation

The native codec currently enforces these bounds:

- the complete source is at most 256 KiB;
- the exact format and schema version are required;
- unknown and duplicate members are rejected, including nested policies,
  taxonomy terms, and licenses;
- a question has 2 through 100 choices and exactly one correct choice;
- choice IDs start with a lowercase ASCII letter, use only lowercase letters,
  digits, `_`, or `-`, are unique, and are at most 64 bytes;
- prompt, choice, title, tag, taxonomy, language, and license text is nonblank and bounded;
- per-choice and correct/incorrect feedback is optional; when present, it is nonblank and bounded;
- points are finite and nonnegative, using the shared `f64` score model; and
- `maxAttempts` is positive or `null` for unlimited attempts.

Per-choice feedback is selected for the submitted choice. Correct or incorrect
outcome feedback is appended according to the server-derived grade. The normal
feedback-disclosure policy still decides whether and when the learner receives
that teaching content.

Canonicalization preserves choice order because order is authored behavior.
Whitespace and JSON object-member order do not change the canonical checksum.

## Evolution and QTI adapters

Version 1 is never silently reinterpreted. Additive optional fields require a
review of old-reader behavior; incompatible changes use version 2 with an
explicit upcaster or republishing path and a committed historical fixture.

Canvas QTI and Blackboard QTI remain separate import/export profiles. Each
adapter may map the supported flat subset into the same public/private compiler
outputs, retain the original package for provenance, and record unsupported
features. Vendor-specific XML is not copied into the PLE flat-question schema
merely because one exporter emits it.

The parser/compiler owner is `crates/adapters/native/src/flat_question.rs`.
The persistence boundary is `crates/learning-data-access/src/flat_question.rs`
with focused in-memory and PostgreSQL implementations, and the server owner is
`crates/server/src/flat_question_publication.rs`. The private source saves
atomically with its typed draft, publication copies its exact canonical bytes
to an immutable non-signable source object, and the runtime obtains private
material only through an injected grader capability. The instructor editor is
complete; bounded Canvas/Blackboard QTI profile mappings, profile-to-native conversion, and their
live and independent-review gates are accepted. The eight-family integration plan is recorded in
`docs/active_plans/active/flat_question_family_evolution_plan.md`.
WP-FQ-0 owns the QTI-JSONL source contract and reference implementation; PLE consumes that accepted
contract without silently widening version 1.
