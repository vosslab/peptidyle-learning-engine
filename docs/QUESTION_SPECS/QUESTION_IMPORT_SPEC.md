# Question import specification

## Purpose

Import brings external Question content into PLE through ordinary authenticated PLE behavior.
Imported content supplies source data and relationships; PLE assigns public Question IDs, Draft
IDs, and Question Revisions. Source identifiers remain provenance, never preassigned PLE IDs.

Import is one way to supply Question content. Instructors also write and test Questions directly
in Drafts. Both use the same Question lifecycle, independent of Backend; see
[DRAFT_QUESTION_SPEC.md](DRAFT_QUESTION_SPEC.md).

## Workflow

1. Read the source record, license, attribution, backend, Question Type, classification, tags, and assets.
2. Create a private Draft through supported PLE operations. PLE assigns identity and ownership.
3. Supply the source, assets, and available metadata to the Draft.
4. Preview, test, and refine the Draft, then complete the metadata required for publication.
5. Apply the normal publication validation. Report failures; never approximate unsupported content.
6. Publish and capture the Question ID and exact Revision returned by PLE for later Pool and
   Blueprint Assessment requests.
7. Read the result through the API and verify identity, metadata, assets, backend, Type, Revision,
   and relationships against the source mapping.

The importer records the source-to-PLE mapping and verifies the published result through the API.
After launch, PLE is independent of BiologyProblems.org and does not track or apply its later
changes. The initial source mapping preserves attribution; it is not an ongoing connection.

## Required data and paths

Import preserves the available source, assets, and metadata in a Draft, even when incomplete or
broken. Drafts have no content or metadata requirements. Before publication, supply valid backend
source, title, description, classification, license, author/provenance, tags as applicable,
and assets. Native JSON's authored `response.kind`
declares the Question Type; it must map to one supported non-NULL Type. A typed interchange reader
may map a known source interaction to that Type. Publication rejects a missing or unsupported Type.
Bloom values may remain NULL while deferred AI assignment is pending, with no limit. Import bypasses
none of the owner, license, source, asset, backend, or publication checks.

All imported Questions follow the same Draft and publication workflow. Backend source handling
preserves the appropriate representation: Native JSON for static Questions, and canonical PG or
PGML for WeBWorK. One algorithmic source remains one Published Question.
PLE accepts Native JSON directly from `qti-package-maker-rs`, with no intermediate Question
format. The Native JSON specification owns the existing unversioned source format; Question
metadata remains separate. Imported-image binding uses existing Draft image storage. QTI is an
interchange path for supported content; PLE does not build a second converter. See
[QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md).

Preserve authored HTML and inline CSS in imported Native JSON display content. After saving the
source unchanged as a Draft, upload each referenced image's raw bytes through the existing Draft
image endpoint. Supply the current Draft edit number in `If-Match`, the image media type in
`Content-Type`, and the exact relative source filename in
`X-PLE-Source-Relative-Name`. PLE stores the filename-to-`QuestionImageAssetTuple` binding
separately from source; source paths are lookup keys and never persistent asset identity. Uploads
do not increment the Draft edit number. Duplicate filenames are rejected. The filename must be a
nonblank slash-separated relative path no longer than 4096 characters, with no leading slash,
`.` or `..` segment, backslash, colon, query, fragment, or control character. Supported image
media types are PNG, JPEG, WebP, and SVG.

After import, authors can open the complete Draft for preview. Per-question save and upload errors
retain the Draft for repair and do not stop later items in the batch. The browser renders only
images bound to the Draft through PLE's authenticated same-origin image route; `externalResources`
never authorizes loading. HTML and inline CSS use the presentation boundary in
[NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md#display-content). This is the settled contract; see the
[converter handoff](NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md#draft-image-upload-and-binding) for the
request details.

Direct SQL and store calls can serve focused schema fixtures. They do not prove import behavior.
Import verification exercises the supported PLE API operations and checks the resulting objects.
The test transport and harness are implementation choices, not additional product requirements.

## Current implementation evidence

The browser import client loads vendored QPM WASM and converts supported banks
locally to Native JSON. Its implementation saves each source unchanged through the Draft API, then
uploads every referenced image per question with the current edit number, matching media type, and
exact `X-PLE-Source-Relative-Name`. Bindings stay separate from source. Native JSON validation
remains in preview and publication. Results report saved Draft links, rejections, and request
failures; later items continue after an item failure, with no automatic retry. This implementation
description does not claim connected or browser runtime acceptance. HOTSPOT is outside QPM import. See
[the transport contract](NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md#browser-transport).

The ordinary private Draft API currently supports Native JSON, WebWork PG,
and PGML. The API stores the selected Backend, format, source, and registered PG path as the Draft
binding. First publication returns a Question ID; the importer obtains its exact Revision before
using it elsewhere. See [Draft creation](../../crates/server/src/authoring/create.rs) and
[the creation page](../../src/pages/question_drafts/question_drafts_page.tsx). The former creation client test was
retired during test pruning and is historical evidence rather than current coverage.

The current content-loading tools call Draft storage and the shared Question publisher with
generated IDs. That exercises supported loader paths, not the browser creation route or the
browser converter handoff. Keep those facts separate when assessing import coverage. Source saves,
preview, and testing use the stored Backend and format binding; the former Draft source tests were
retired during test pruning. [The preview routes](../../crates/server/src/draft_preview.rs) remain
implementation evidence rather than replacement coverage.

The accepted M01-M29 implementation and runtime status is recorded in the
[implementation ledger](../archive/reports/question_spec_implementation_ledger.md). That
acceptance does not establish Native JSON converter integration or HTML/image rendering runtime
acceptance.

Current source evidence: [authoring.rs](../../crates/server/src/authoring.rs),
and [import_bpw.py](../../launchers/import_bpw.py).

The evidence owner is
[question_backend_spec_handoff.md](../archive/reports/question_backend_spec_handoff.md).
The focused Native JSON handoff documentation audit is recorded in
[NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md](../archive/reports/NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md).
The import flow reuses existing endpoints and adds no retry workflow.
