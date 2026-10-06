# QTI interchange specification

## Purpose and ownership

QTI is for import, export, and archival interchange of supported content. It is separate from
PLE's runtime Question Backends and private Native JSON source format. Import uses the ordinary
Draft, preview, testing, and publication workflow in
[QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md).

PLE uses [qti-package-maker-rs](https://github.com/vosslab/qti-package-maker-rs) as an external
library for conversion and packaging. PLE supplies selected content and uses the library's
supported readers and writers. Native JSON is an intended internal destination for suitable
static content; that does not determine the representation exchanged with the converter.

## Unsettled handoff format

[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#deferred-content-tools-and-public-apis) explicitly leaves
that format undecided. Its examples include BBQ text, a QTI-style JSON or editable YAML format,
and something closer to Native JSON. They are alternatives, not approved wire formats.

The previous source-interaction-to-Native-JSON table was an unsupported mapping contract and has
been removed. Exact field mappings depend on the chosen handoff and verified converter support.
Record that unresolved choice in
[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md).

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

The previously recorded compiled engine inventory has ten writers and four readers, with different
coverage in each direction. This is retained implementation evidence, not fresh converter testing
or a settled PLE handoff contract. A future Rust/WASM build is not current PLE capability.

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
