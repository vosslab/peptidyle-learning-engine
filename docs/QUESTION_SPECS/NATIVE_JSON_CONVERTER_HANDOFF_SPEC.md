# Native JSON converter handoff

## Goal and boundary

Implement a qti-package-maker-rs writer that emits PLE's existing, unversioned
`pleQuestionJson` source and the associated files needed by that source. This is a coordinated
private format between the converter and PLE, not a published interchange format or a commitment
to add a version field. The writer can begin with response kinds that need no unresolved media
binding. HOTSPOT remains a coordinated follow-up because the converter-side surface reference
before PLE binding has no settled representation.

PLE accepts the source directly into the ordinary Question Draft flow. The converter emits source
content, available Question metadata, and bytes for referenced files. PLE validates the source,
resolves file references, computes or verifies the checksum used in its asset tuple, stores image
bytes, and binds them through the existing `QuestionImageAssetTuple` (`questionImageAssetId` and
`checksum`). PLE saves metadata through the ordinary Question record path. The handoff's exact
transport and the final `img src` value after PLE binding are implementation choices still to make.

Send this handoff with [NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md),
[MULTIPLE_ANSWER_SCORING_SPEC.md](MULTIPLE_ANSWER_SCORING_SPEC.md),
[ORDER_SCORING_SPEC.md](ORDER_SCORING_SPEC.md),
[QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md),
[QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md), and the primary
[Human Guidance import and Native JSON decisions](../HUMAN_GUIDANCE.md#question-formats-and-type-specifications).
The Native JSON spec owns exact source and type validation; the import spec owns Draft flow and
metadata transfer; QTI interchange supplies conversion and file-resource context.

## Existing source contract

The source decoder is [source_document.rs](../../crates/adapters/ple/src/question_json/source_document.rs);
the choice shape is in [question_json.rs](../../crates/adapters/ple/src/question_json.rs), and the
complete content and grading rules live in [NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md). JSON member
names use camelCase. The decoder rejects unknown members.

| Member | JSON type | Writer requirement |
| --- | --- | --- |
| `format` | string | Emit the literal `pleQuestionJson`. |
| `prompt` | string | Emit display content as HTML, with inline CSS when useful. Relative image references may point to associated supplied files. |
| `response` | object | Emit one of the supported kinds below, retaining stable IDs and answer meaning. |
| `questionHint` | string or `null` | Optional; displayed hint, separate from outcome feedback. |
| `feedback` | object | Optional `correct` and `incorrect` display strings, each nullable. |
| `externalResources` | array | Optional declared inventory with `url` and `kind`; this records resources and does not fetch or authorize them. |
| `authorScript` | object or `null` | Optional existing isolated-author-script declaration; preserve its current shape and restrictions. |

The minimal required source consists of `format`, `prompt`, and `response`. Omitted optional
members use the decoder defaults. Question title, description, authors, license, citation, Type
classification for non-Native backends, tags, Bloom, and other Library metadata belong to the
separate Question record and ordinary Draft metadata path. Citation is plain text. Do not add
metadata members to this closed source object.

## Response kinds and minimum content

This is a writer checklist, not a replacement for the full [response specification](NATIVE_JSON_SPEC.md#response-shapes-and-grading).

| `response.kind` | Minimum answer-bearing members |
| --- | --- |
| `singleChoice` | `choices` (`id`, `text`; optional per-choice `feedback`) and `correctChoice`; optional `randomizeChoices`. |
| `multipleAnswer` | `choices` and `correctChoices`; optional `randomizeChoices`. |
| `fillIn` | `answers`, `matchMode`, and `maxLength`. |
| `multiFillIn` | `blanks`, each with `id`, `label`, `answers`, `matchMode`, and `maxLength`. |
| `numeric` | `answer` and `tolerance`; optional `unit`. |
| `matching` | `prompts` and `choices` as `{id,text}`, plus `matches` as `{prompt,choice}` ID pairs. |
| `ordering` | `items` as `{id,text}` and `correctOrder` as stable IDs. |
| `hotspot` | `surface`, `regions`, and `correctRegions`. The current stored form uses a PLE image ID and checksum; a converter must not invent or mint those PLE identities. Agree on its pre-binding input representation with PLE before implementing this kind. |

For `fillIn` and each multi-fill blank, `matchMode` is one of `exact`, `caseInsensitive`,
`normalized`, or `regex`. Numeric `tolerance` is one of `exact`, `absolute` with `epsilon`,
`relative` with `fraction`, or `significantFigures` with `digits`. Apply the detailed requirements,
grading rules, and supported source constraints in the canonical spec rather than deriving new
rules from this summary.

## Minimal complete source and associated file

This valid single-choice source demonstrates HTML and a content-relative image reference. The
relative path is an example only; it does not establish a required filename or directory policy.

```json
{
  "format": "pleQuestionJson",
  "prompt": "<p><strong>Which structure is shown?</strong></p><img src=\"assets/image_001.png\" alt=\"A simple cell diagram\" style=\"max-width: 20rem\">",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "cell", "text": "<em>Cell</em>"},
      {"id": "tissue", "text": "Tissue"}
    ],
    "correctChoice": "cell"
  }
}
```

The associated file is supplied separately at the matching relative path:

```text
assets/image_001.png  ->  image bytes
```

The converter preserves the relative reference and supplies those bytes. PLE resolves the
reference against the supplied import files, computes or verifies the checksum it binds to the
asset tuple, stores the bytes through
existing Draft image storage, and binds the existing tuple. The path is lookup input, not a
persistent asset identity. `QuestionImageAssetTuple` has no new key or object-store field; PLE
keeps logical asset identity separate from physical object identity.

## Image and HTML responsibilities

The converter preserves authored HTML and inline CSS in all Native JSON display content that
supports HTML, as defined by the canonical [Display content rule](NATIVE_JSON_SPEC.md#display-content),
and supplies each referenced file's bytes. Prompt and choice content are representative examples.
It does not assign PLE asset IDs or object-store IDs. PLE computes or verifies the checksum used in
its asset tuple.
Paths such as `assets/image_001.png` are neutral examples, not naming requirements.

PLE resolves each image reference in Native JSON display content that supports HTML, verifies the
bytes, stores them through the existing Draft asset path, and binds the resulting bytes to the
current `QuestionImageAssetTuple`. PLE then makes the image available in preview and published
content. Prompt and choice references are useful representative coverage. Account for the existing
single-surface HOTSPOT source path. Current source only handles one HOTSPOT surface; the converter's
pre-binding HOTSPOT input shape still needs agreement. Exact transport and final stored HTML `src`
syntax remain open implementation details.

HTML may parse and be stored as strings today, but current compilation/rendering escapes or treats
it as text. It therefore does not yet satisfy the settled HTML display requirement. Sanitization
policy remains deferred. Existing isolated `authorScript` rules remain in force and HTML support
does not widen script execution.

## Small implementation milestones

1. **Non-media writer slice:** emit valid existing Native JSON for singleChoice, multipleAnswer,
   fillIn, multiFillIn, numeric, matching, and ordering from suitable source items. Keep metadata
   separate. Do not block this work on HOTSPOT's unresolved surface binding.
2. **Writer contract smoke:** round-trip representative emitted JSON through PLE's current source
   decoder; confirm member names, defaults, response kind, stable answer IDs, and display strings.
   Treat decoder success as shape compatibility, not proof of rendering or import acceptance.
3. **Associated-file emission:** preserve relative references and emit the referenced bytes
   alongside JSON. A focused smoke confirms each emitted reference identifies one supplied file;
   naming remains writer policy.
4. **PLE import binding:** resolve supplied files, verify/hash/store image bytes, bind the existing
   tuple, and persist metadata through ordinary Draft paths. Cover more than the current one-image
   HOTSPOT publication helper, including prompt and choice references.
5. **Rendered import smoke:** inspect a Draft preview with an inline prompt image and choice image,
   then verify the same content after publication. This exposes the current HTML escaping gap and
   image lifecycle gaps; it is distinct from source-decoder smoke.
6. **HOTSPOT coordination:** agree the pre-binding source/reference shape between the writer and
   PLE, then bind it to the existing surface tuple. This decision must not make the converter assign
   a PLE ID or object-store identity.

## Current PLE gaps and evidence

- The Native JSON compiler stores prompt and choice strings, but the current renderer escapes or
  treats them as text. HTML display with inline CSS remains an implementation gap. This is separate
  from the M01-M29 acceptance recorded in the [October 7 ledger closeout](../active_plans/reports/question_spec_implementation_ledger.md).
- The current publication image path handles a HOTSPOT surface. It does not enumerate and bind an
  arbitrary set of images referenced in prompt and choice strings. Structured image rendering
  already uses the tuple and image URL path; ordinary inline HTML image rendering is incomplete.
- Converter output has no server Draft-storage integration yet. The QTI parser/worker handoff and
  external Rust converter do not establish a PLE Native JSON writer or complete import workflow.
- The stored HOTSPOT surface requires `questionImageAssetId` and `checksum`, but source inspection
  does not establish the converter's pre-binding representation. Keep that question open for
  coordination; implement the other seven response kinds independently.

Relevant implementation anchors: [Native JSON decoder](../../crates/adapters/ple/src/question_json/source_document.rs),
[existing image tuple](../../crates/question_model/src/question_content.rs),
[HOTSPOT publication image path](../../crates/server/src/question_publication_images.rs),
[QTI import evidence](QTI_INTERCHANGE_SPEC.md#current-implementation-evidence), and
[the existing future import TODO](../TODO.md#future-product-capabilities).

In the qti-package-maker-rs checkout, useful existing extension points are
`crates/qti-engines/src/traits.rs` and `crates/qti-engines/src/registry.rs` for writer registration,
`crates/qti-core/src/item.rs` for item views, and `crates/qti-core/src/media/resolve.rs` plus
`crates/qti-core/src/media/rewrite.rs` for discovering supplied media and rewriting HTML `src`
references. `html_selftest` is a nearby writer with shared media handling. These are orientation
references, not a required architecture or a claim that a PLE Native JSON writer already exists.
