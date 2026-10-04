# Architecture and implementation changes

Implementation findings for schema, identity storage, and install behavior. Open
readings are recorded once in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md).

## Schema and identity

- Source schema style reports no findings, including clock rules. The catalog
  snapshot is not the proof. `tests/test_schema_table_shape.py` loads
  `schemas/base_schema` from source.
- Support repair resource class is `ple_data.support_repair_resource_class`. A
  disposable database installed the schema, rejected an invalid class, and was
  removed.
- Canonical public IDs are stored and reread unchanged. Mint placeholders are
  replaced. Lowercase IDs and internal UUIDs are rejected.
- Object paths and the retention failure log keep the canonical public ID. Changing
  that ID changes the object hash.
- Profile displays and copies the canonical Account ID and hides a lowercase value
  and a UUID.

## Install

- The default `installation-data provision` command includes the Live Demo and the
  Genetics Blueprint. A disposable proof created Course BCHM 301 and public
  Blueprint Genetics, with 9 topics and 41 Questions, then removed Postgres, object
  storage, and the API. The Live Demo launcher was not started.

## Not refreshed here

Screenshot corpus regeneration, the Ribbon destination ledger, and Graphify output
were not rerun. The checklist summary in
[compliance_summary.md](compliance_summary.md) is the count record for this audit.

Data and history: 159 verified, 2 open, 0 not applicable. The open rows are the
Theme and avatar key question and the simplest-term question.
