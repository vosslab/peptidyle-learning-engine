# M03 Question metadata schema and SQL report

Status: Schema and operations implemented; generated schema documentation and style checks pass.
Fresh-database behavior and independent specification/quality reviews remain pending.

## Implementation

- Replaced lineage-keyed `published_question_metadata` with
  `question_revision_metadata`, keyed by `(published_question_id, revision_number)` and constrained
  to its parent `question_revision`.
- Added the ordinary owner/Sysadmin metadata replacement function. It locks the stable lineage and
  exact metadata child, checks the current Revision Number and metadata Edit Number, and updates
  Title, Description, Tags, classification, and optional trusted language input in place.
- Changed successor publication to insert metadata for the newly inserted Revision from the locked
  parent metadata. It also carries citation and the current Bloom classification, alongside the
  existing authorship/license copy. Fork publication carries the pinned source citation and Bloom
  classification into Revision 1.
- Kept the deferred bulk endpoint separate. Its database operation now reads and locks latest
  Revision metadata, updates only that row, and permits only the current owner or a Sysadmin.
- Updated row-security policies, grants, metadata search/classification indexes, and generated
  schema documentation for the new child.

## Evidence

- `source source_me.sh && python3 devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py`:
  passed (`clean`). The generator refreshed `docs/SCHEMA_TABLES.md` and
  `schemas/catalog_snapshot.json`.
- `git diff --check`: passed across the shared working tree.
- Manager review caught possible PL/pgSQL name collisions between `RETURNS TABLE` output columns and
  unqualified table columns; the current save function qualifies the lineage, latest Revision, and
  metadata reads and updates explicitly.
- Fresh install and behavior proof for metadata CAS, stale tuple refusal, and publication carry-forward:
  pending. The currently leased M01 baseline started before this SQL handoff, so it cannot certify
  these edits; the next fresh baseline run must use the coherent post-M03 schema tree.

## Product boundary observed

The existing Native JSON editor still exposes Title, Description, and Tags in
`src/features/ple_question_json_authoring/question_json_editor_workspace.tsx`, and its publication
request submits the current source. M03 successor publication therefore copies metadata from the
locked parent record. M04 owns removal of this legacy metadata duplication from Native JSON; a
metadata correction must use the ordinary record save and keep its Revision Number.

The broader M03 acceptance remains pending until the SQL is compiled and exercised against a fresh
database, readers and API are aligned, and independent reviews pass.
