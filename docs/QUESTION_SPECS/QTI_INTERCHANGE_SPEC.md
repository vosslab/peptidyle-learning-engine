# QTI interchange specification

## Purpose and ownership

QTI is for import, export, and archival interchange of supported content. It is separate from
PLE's runtime Question Backends and private Native JSON source format. Import uses the ordinary
Draft, preview, testing, and publication workflow in
[QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md).

PLE uses [qti-package-maker-rs](https://github.com/vosslab/qti-package-maker-rs) as an external
library for conversion and packaging. PLE accepts Native JSON directly from the library, with no
intermediate Question format. PLE's [Native JSON specification](NATIVE_JSON_SPEC.md) owns the
existing unversioned source format. Question metadata remains separate and enters through the
ordinary Draft metadata path.

## Draft import and image assets

The QTI adapter verifies image bytes before binding them to the existing
`QuestionImageAssetTuple` (`questionImageAssetId` and SHA-256 `checksum`) in Question content. Its
current adapter derives the logical asset ID from the verified bytes at bind time. The PLE Draft
upload path can mint logical asset IDs independently; publication preserves a logical asset ID
while assigning physical object-store identity. These are existing implementation paths, not a
universal ID-minting rule.

The external converter owns conversion. PLE follows ordinary Draft validation, preview, testing,
and publication, and reports unsupported content or assets according to the existing import rules.
The QTI parser/worker handoff exists, but server integration of converter output with Draft storage
and workflows remains implementation work. That work reuses the existing tuple and image storage;
transport details remain implementation work.

Converter output preserves HTML with inline CSS in Native JSON display content. Ordinary HTML
`img src` references that are relative to supplied import files locate the provided bytes. For
example, `assets/image_001.png` identifies the matching supplied file at that relative path. Verify
and hash those bytes, store them through the existing Draft asset path, and bind them through the
existing `QuestionImageAssetTuple`. The relative import path is not a persisted asset identity, and
the converter need not know PLE asset IDs. Display the images in prompts, choices, and other
applicable response content through Draft and published renderers. Choosing the final stored HTML
`src` syntax and transport details remains implementation work. This follows the package-resource
model described in the external [QTI v3 specification](https://www.imsglobal.org/spec/qti/v3p0/impl/).

## Content and identity

Preserve supported Question content, assets, attribution, and the meaning of answers and feedback.
Report conversion limitations accurately. Source IDs identify source records; PLE assigns its
own identities. Available metadata enters the Draft, and required publication checks apply when
publishing. Vendor classification needs a defined mapping to PLE classification.

Assessment disclosure is separate from conversion of Question content. Optional Question Feedback
timing remains deferred. Preserve one algorithmic source as one Question; a rendered static
instance does not preserve its algorithmic behavior. Report unsupported adaptive logic, scripts,
vendor extensions, ambiguous scoring, or inaccessible hotspot geometry rather than inventing a
simpler equivalent. Evidence format and retention remain implementation choices.

## Current implementation evidence

Ordinary inline HTML images in Native JSON prompt and choice strings are not currently compiled
into image blocks. The Native JSON document accepts prompt and choice strings and compiles them
through `markdown_blocks` ([source_document.rs](../../crates/adapters/ple/src/question_json/source_document.rs#L38),
[source_compile.rs](../../crates/adapters/ple/src/question_json/source_compile.rs#L190)); the browser's
text block renderer displays that string as text and does not parse HTML or Markdown
([question_renderer.tsx](../../src/components/question_renderer.tsx#L345)). A structured image block
does use the existing `QuestionImageAssetTuple` and Draft or published image URL resolver and renders
an `<img>` ([question_renderer.tsx](../../src/components/question_renderer.tsx#L355)). The QTI parser
can produce such image blocks from package `<img>` elements
([parser.rs](../../crates/adapters/qti/src/parser.rs#L456)), but the multiple-choice response
renderer reduces image choice blocks to their description text
([common.tsx](../../src/components/question_response_controls/common.tsx#L172),
[multiple_choice.tsx](../../src/components/question_response_controls/multiple_choice.tsx#L155)).
The current publication image path
([question_publication_images.rs](../../crates/server/src/question_publication_images.rs)) handles
one HOTSPOT surface image. It does not enumerate and bind an arbitrary set of prompt and choice
image references. Therefore the current QTI and Native JSON evidence does not establish that
ordinary inline prompt and choice images are displayed through import and response rendering.
Future integration must resolve each supplied file, bind it through the existing tuple and asset
storage paths, and verify prompt and choice rendering. HOTSPOT's required stored tuple is known;
its converter-side pre-binding representation remains a shared implementation question.
The `externalResources` inventory remains source metadata. Supplied-file resolution follows the
relative paths in the imported content and package; the inventory does not define that lookup.
The focused Native JSON handoff documentation audit is recorded in
[NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md](../active_plans/reports/NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md).

The previously recorded compiled engine inventory has ten writers and four readers, with different
coverage in each direction. This is historical implementation evidence, not fresh converter
testing and does not establish that a PLE Native JSON writer is implemented. A future Rust/WASM
build is not current PLE capability.

| Direction | Current engine inventory |
| --- | --- |
| Readers | `bbq_text_upload`, `text2qti`, `okla_chrst_bqgen`, `blackboard_export_zip` |
| QTI writers | `blackboard_qti_v2_1`, `canvas_qti_v1_2` |
| Other writers | `bbq_text_upload`, `blackboard_export_zip`, `exam_yaml`, `html_selftest`, `human_readable`, `moodle_aiken`, `okla_chrst_bqgen`, `text2qti` |

There is no listed QTI ZIP reader for Blackboard QTI 2.1 or Canvas QTI 1.2. PLE must not claim
that either package is an import path until a reader is added and verified. A selected writer can
also return no output when no selected item is representable; inspect its result and warnings.

## Package rules

Archives are bounded and structurally checked before content is trusted. Asset paths stay inside
the archive and media types are validated before upload. Reuse the external library's package
handling. Supported formats and conversion limitations must be stated accurately.
