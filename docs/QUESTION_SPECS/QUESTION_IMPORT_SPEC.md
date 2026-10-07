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
metadata remains separate. PLE reuses its existing Draft image storage for imported assets. QTI is
an interchange path for supported content; PLE does not build a second converter. See
[QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md).

Preserve HTML with inline CSS in imported Native JSON display content. For an ordinary `<img
src="...">` whose relative path refers to a file supplied with the import, locate those bytes,
verify and hash them, store them through the existing Draft asset path, and bind the reference to the
existing Question Image Asset tuple. The import path is temporary input used to find bytes; it is not
a persisted asset identity. Do not require the converter to know PLE asset IDs. Display the image in
Draft preview, published Questions, and response renderers. The final stored HTML `src` syntax
remains unspecified.

Direct SQL and store calls can serve focused schema fixtures. They do not prove import behavior.
Import verification exercises the supported PLE API operations and checks the resulting objects.
The test transport and harness are implementation choices, not additional product requirements.

## Current implementation evidence

The ordinary private Draft API and browser creation page currently support Native JSON, WebWork PG,
and PGML. The API stores the selected Backend, format, source, and registered PG path as the Draft
binding. The creation client test covers all three formats. First publication returns a Question ID;
the importer obtains its exact Revision before using it elsewhere. See
[Draft creation](../../crates/server/src/authoring/create.rs),
[the creation page](../../src/pages/question_drafts_page.tsx), and
[the creation client test](../../tests/test_question_draft_creation.mjs).

The current content-loading tools call Draft storage and the shared Question publisher with
generated IDs. That exercises supported loader paths, not the browser creation route or the
deferred converter handoff. Keep those facts separate when assessing import coverage. Source saves,
preview, and testing use the stored Backend and format binding; focused client and server evidence
is in [Draft source tests](../../tests/test_draft_source_client.mjs) and
[the preview routes](../../crates/server/src/draft_preview.rs).

The accepted M01-M29 implementation and runtime status is recorded in the
[implementation ledger](../active_plans/reports/question_spec_implementation_ledger.md). That
acceptance does not include the later Native JSON converter integration or HTML/image rendering
work recorded in [TODO.md](../TODO.md#future-product-capabilities).

Current source evidence: [authoring.rs](../../crates/server/src/authoring.rs),
[parameterized_publication.rs](../../crates/project-tools/src/curriculum_content/parameterized_publication.rs),
and [publication.rs](../../crates/project-tools/src/pilot_content/publication.rs).

The evidence owner is
[question_backend_spec_handoff.md](../active_plans/reports/question_backend_spec_handoff.md).
The focused Native JSON handoff documentation audit is recorded in
[NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md](../active_plans/reports/NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md).
HTTP error handling, retries, and new endpoint design are outside this documentation-drift pass.
