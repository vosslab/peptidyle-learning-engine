# Native JSON converter handoff

## Goal and boundary

QPM's `ple_native_json` writer emits PLE's existing, unversioned
`pleQuestionJson` source and the associated files needed by that source. This is a coordinated
private format between the converter and PLE, not a published interchange format or a commitment
to add a version field. The integration covers MC, MA, FIB, MULTI-FIB, NUM, MATCH, and ORDER.
HOTSPOT stays PLE-native: its existing Draft editor supplies image upload, region editing, and
correct-region selection. HOTSPOT is outside the converter contract.

PLE accepts source directly into the ordinary Question Draft flow. QPM converts and validates the
source package; PLE owns storage and its Native JSON acceptance rules. Draft creation means saved.
Preview and publication parse and compile the source through the existing decoder. Question
metadata uses ordinary defaults and editing controls, separately from source content.

## Browser transport

The tracked `assets/qpm_wasm/` distribution contains QPM's wrapper, WASM, license, and `source.json`
with the checkout commit and artifact hashes. PLE's content-addressed build serves those files
locally. Opening Import loads the wrapper and calls `initialize`; ordinary pages load neither
QPM's wrapper nor its WASM. The picker lists only `formats().formats` entries with `canRead`.

`convert` receives the selected file bytes with `outputFormat: "ple_native_json"` and
`allowMixed: true`. Its directory artifact supplies `item_*.json` and optional `media/*` entries.
PLE shows warnings, inspects each item's display fields for images, then saves image-free items
sequentially through `POST /api/authoring/drafts`. It preserves the emitted source HTML unchanged.
Shared media does not make unrelated questions image-bearing; `externalResources` is inventory,
not an image binding. Current inline-image binding is incomplete, so affected items are skipped
before Draft creation with `contains images`.

One result list reports created Draft links, rejected items with available server messages,
skipped items with reasons, and individual request failures. Processing continues after a failed
item, preserves earlier successes, and performs no automatic retry. A failed network response can
leave the save outcome unknown. Check My Draft Questions before reimporting.

Current readers are BBQ, text2qti, Oklahoma, and Blackboard export ZIP. Canvas QTI 1.2 and
Blackboard QTI 2.1 remain writer-only in QPM; adding readers is separate upstream work. See
[INSTALL.md](../INSTALL.md#refresh-the-qpm-distribution) for the refresh procedure.

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

The converter preserves the relative reference and supplies those bytes. Future inline-image
binding in PLE must resolve the reference against the supplied import files, compute or verify the
checksum, store the bytes through existing Draft image storage, and bind the existing tuple.
The path is lookup input, not a
persistent asset identity. `QuestionImageAssetTuple` has no new key or object-store field; PLE
keeps logical asset identity separate from physical object identity.

## Image and HTML responsibilities

The converter preserves authored HTML and inline CSS in all Native JSON display content that
supports HTML, as defined by the canonical [Display content rule](NATIVE_JSON_SPEC.md#display-content),
and supplies each referenced file's bytes. Prompt and choice content are representative examples.
It does not assign PLE asset IDs or object-store IDs. PLE owns checksum verification and assignment
for its asset tuple when image binding is implemented.
Paths such as `assets/image_001.png` are neutral examples, not naming requirements.

Imported inline-image binding is deferred. Current browser import skips image-bearing questions
before Draft creation, as specified in the browser transport above. When binding is implemented,
PLE will resolve each image reference in Native JSON display content, verify the supplied bytes,
store them through the existing Draft asset path, and bind the current `QuestionImageAssetTuple`.
PLE will make the image available in preview and published content. Prompt and choice references
are useful representative coverage. Final stored HTML `src` syntax remains unspecified. The
existing single-surface HOTSPOT path remains a complete PLE-only authoring capability.

HTML may parse and be stored as strings today, but current compilation/rendering escapes or treats
it as text. It therefore does not yet satisfy the settled HTML display requirement. Sanitization
policy remains deferred. Existing isolated `authorScript` rules remain in force and HTML support
does not widen script execution.

## Verification boundary

The former seven-kind browser journey imported a fixed BBQ bank and opened every saved Draft
preview. It was retired during test pruning, so it is historical refresh evidence rather than
current QPM-to-PLE coverage. A discovered conversion failure calls for correcting the QPM Native
JSON engine or PLE decoder against [NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md), then rebuilding and
re-vendoring after engine changes. Inline-image rendering remains separate work.

## Current PLE gaps and evidence

- The Native JSON compiler stores prompt and choice strings, but the current renderer escapes or
  treats them as text. HTML display with inline CSS remains an implementation gap. This is separate
  from the M01-M29 acceptance recorded in the [October 7 ledger closeout](../archive/reports/question_spec_implementation_ledger.md).
- The current publication image path handles a HOTSPOT surface. It does not enumerate and bind an
  arbitrary set of images referenced in prompt and choice strings. Structured image rendering
  already uses the tuple and image URL path; ordinary inline HTML image rendering is incomplete.
- The browser import skips each image-bearing item before creating its Draft; it does not yet bind
  arbitrary inline images to PLE image storage.
- HOTSPOT's existing stored surface requires `questionImageAssetId` and `checksum`. PLE's GUI owns
  that complete authoring path; there is no QPM-side HOTSPOT representation to coordinate.

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
references. The vendored browser distribution uses the existing PLE Native JSON writer.
