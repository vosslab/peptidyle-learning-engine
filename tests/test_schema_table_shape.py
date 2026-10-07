"""Source tables follow the DATABASE_STYLE table-shape and clock rules.

A closed vocabulary stored as repeated text CHECK lists, or a table that
loses its clock, fails this test. Fix schemas/base_schema/ so the source
checker reports no findings. The catalog snapshot is not the input.
"""

import pathlib

import schema_style.schema_catalog_lib as schema_catalog_lib
import schema_style.schema_style_rules as schema_style_rules


def test_table_shape_and_clocks_follow_database_style() -> None:
	"""Closed vocabularies are enums and every source table keeps a clock."""
	repo_root = pathlib.Path(__file__).resolve().parents[1]
	source_dir = repo_root / "schemas" / "base_schema"
	# The snapshot can omit a current text CHECK. Load the SQL files themselves.
	catalog = schema_catalog_lib.load_from_source(str(source_dir))
	findings, notes, advisory_rules = schema_style_rules.collect_findings(
		catalog, str(source_dir), False,
	)
	assert "Tier 3 skipped (source-only)" in notes
	assert "Tier 2 skipped (no role tags)" not in notes
	assert "rule_14_unindexed_fk" in advisory_rules
	described = [
		item.rule + " " + item.location + " " + item.message
		for item in findings
	]
	assert described == []
