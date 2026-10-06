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
PLE uses `qti-package-maker-rs` as an external library for all conversion. QTI is an interchange
path for supported static content; PLE does not build a second converter. See
[QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md). HG leaves the representation exchanged with
that external library undecided. Native JSON as an eventual internal source format does not
settle the converter-to-PLE handoff format.

Direct SQL and store calls can serve focused schema fixtures. They do not prove import behavior.
Import verification exercises the supported PLE API operations and checks the resulting objects.
The test transport and harness are implementation choices, not additional product requirements.

## Current implementation evidence

The authoring API currently provides Native JSON Draft creation, image upload, source and metadata
updates, first publication, and publication of a new Revision. First publication returns a
Question ID; the importer must obtain its exact Revision before using it elsewhere.
The browser-facing Draft creation route currently accepts only Native JSON. WeBWorK Draft creation
and publication already exist in the shared services and content-loading tools. PG and PGML use
the same Draft and Question publisher model. Extending the browser-facing route is a specific
implementation follow-up, not a missing WeBWorK lifecycle.

The current content-loading tools call Draft storage and the shared Question publisher with
generated IDs. That is more than raw SQL fixture loading, but it does not exercise the
browser-facing authoring route. Keep those facts separate when assessing import coverage.

Current source evidence: [authoring.rs](../../crates/server/src/authoring.rs),
[parameterized_publication.rs](../../crates/project-tools/src/curriculum_content/parameterized_publication.rs),
and [publication.rs](../../crates/project-tools/src/pilot_content/publication.rs).

The evidence owner is
[question_backend_spec_handoff.md](../active_plans/reports/question_backend_spec_handoff.md).
HTTP error handling, retries, and new endpoint design are outside this documentation-drift pass.
