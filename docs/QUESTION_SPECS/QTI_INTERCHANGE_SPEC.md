# QTI interchange specification

## Purpose and ownership

QTI is for import, export, and archival interchange of supported content. It is separate from
PLE's runtime Question Backends and private Native JSON source format. Imports use the ordinary
Draft, preview, testing, and publication workflow in
[QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md).

PLE uses [qti-package-maker-rs](https://github.com/vosslab/qti-package-maker-rs) as an external
library for conversion and packaging. Its browser WASM distribution is vendored into PLE. QPM
parses and validates the source bank locally, and PLE accepts emitted Native JSON directly through
its existing Draft API. PLE's [Native JSON specification](NATIVE_JSON_SPEC.md) owns the unversioned
source format. Question metadata remains separate and uses ordinary Draft defaults and controls.

## Current readable formats

The import picker derives its inventory from QPM's `formats()` and includes only `canRead` entries.
The current distribution provides these readers:

| Reader | Input |
| --- | --- |
| `bbq_text_upload` | Blackboard text-upload bank, including seven supported Question Types |
| `text2qti` | text2qti text source |
| `okla_chrst_bqgen` | Oklahoma question-generator text |
| `blackboard_export_zip` | Blackboard export ZIP |

QPM's `canvas_qti_v1_2` and `blackboard_qti_v2_1` engines are writers only. Neither is currently a
QTI import format. Adding readers requires upstream work outside the Native JSON engine scope.
Blackboard export ZIP is a distinct format from Blackboard QTI 2.1. The former PLE QTI adapter
and its unused imported-source helper have been removed; PLE does not maintain a second converter.

## Draft results and images

QPM returns conversion warnings and a directory artifact containing Native JSON question files
and optional media. PLE inspects each emitted question's display fields for image references.
Shared package media does not make unrelated questions image-bearing. Until inline-image binding
is complete, PLE skips affected items before Draft creation and reports `contains images`.
Image-free items save sequentially through the existing Draft API.

One list reports created Draft links, rejected requests with available server messages, skipped
items with reasons, and individual request failures. Processing continues after each failure,
preserves earlier successes, and does not retry automatically. Created means saved; preview and
publication apply the existing Native JSON decoder and compiler.

Future inline-image binding must resolve content-relative references against supplied files,
verify and hash the bytes, and store them through the existing Draft image path. The resulting
`QuestionImageAssetTuple` keeps logical image identity and checksum separate from physical storage.
The converter knows relative paths, not PLE asset IDs or object-store IDs. `externalResources`
remains inventory metadata; it does not control image lookup, fetching, or authorization.

HOTSPOT remains PLE-native. PLE's existing editor owns image upload, region editing, and
correct-region selection. It is outside QPM integration; no converter-side HOTSPOT representation
is required. See [DESIGN_DECISIONS.md](../DESIGN_DECISIONS.md#hotspot-authoring-stays-in-ple).

## Content and identity

Preserve supported Question content and answer meaning, and report conversion limitations
accurately. Source IDs identify source records; PLE assigns its own identities. Publication
requires the ordinary metadata, source, and asset checks. Vendor classification mapping and
automatic metadata transfer are separate work.

Assessment disclosure is separate from conversion. Optional Question Feedback timing remains
deferred. One algorithmic source remains one Question; a static instance does not preserve its
algorithmic behavior. Report unsupported adaptive logic, scripts, vendor extensions, or ambiguous
scoring rather than inventing a simpler equivalent.

## Rendering and verification

Import preserves emitted HTML with inline CSS. The current Native JSON compiler and renderer
still treat relevant display strings as text. Existing structured image rendering uses PLE's image
tuple, while the publication image path handles a single HOTSPOT surface. Arbitrary inline prompt
and choice images are not yet bound or rendered; those limitations remain in
[TODO.md](../TODO.md#future-product-capabilities).

The former import journey loaded a fixed seven-kind BBQ bank and opened every saved Draft preview.
It and its focused implementation checks were retired during test pruning. They are historical
evidence, not current import coverage. A writer-only QTI engine is a reported limitation, not a
passing import result. See [NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md](NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md)
for transport, acceptance, and refresh details.

Archives are bounded and structurally checked by QPM before conversion. PLE reuses QPM's package
handling rather than implementing another ZIP/XML path or adding import endpoints.
