# Question Spec Schema Snapshot Checkpoint

The source-only schema documentation checkpoint is current after M03, M04, M08, M17, and M26.

Ran `source ./source_me.sh && python3 devel/generate_schema_tables_doc.py`. The generator read `schemas/base_schema/` and wrote `docs/SCHEMA_TABLES.md` and `schemas/catalog_snapshot.json`. The generated catalog contains 141 tables, 413 indexes, 59 enums, and 6 domains. The Markdown diff reflects the current source: it adds the assessment partial-credit flag and Blueprint parent revision reference, removes the superseded Blueprint fork tables, and documents revision-scoped question metadata.

The refreshed JSON snapshot is byte-for-byte unchanged from the existing root snapshot, so the source-only generation and snapshot agree; the earlier apparent stale-root discrepancy is resolved by regenerating both outputs from the same source tree. `schema_style/check_schema_style.py` then read the snapshot, reported `clean`, and wrote `output/schema_style_findings.txt` with `clean` (zero findings). No production SQL writers or live database were involved.

No production SQL or checker changes were made in this checkpoint.
