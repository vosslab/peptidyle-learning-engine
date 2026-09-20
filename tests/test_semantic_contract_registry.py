"""Durable registry of cross-boundary Id/Tuple/Number contracts.

The pytest checks that each registered surface cites Human Guidance or
Terminology and points at native Rust, TypeScript, and PostgreSQL coverage.
Canonical JSON lives inline in those native tests, not a committed fixture
tree. Behavior is proven by the named tests.
"""

import json
import pathlib
import re

import file_utils

REQUIRED = {
	"id",
	"behavior",
	"citation",
	"rust_test",
	"typescript_test",
	"postgres_api",
	"postgres_test",
}
POSTGRES_API = re.compile(r"^ple_api\.[a-z][a-z0-9_]*$")
AUTHORITY = ("HUMAN_GUIDANCE.md", "TERMINOLOGY_CONTRACT.md")


def test_semantic_contract_registry_points_at_native_coverage() -> None:
	"""Every registered surface cites living authority and existing native tests."""
	repo_root = pathlib.Path(file_utils.get_repo_root())
	registry_path = repo_root / "tests" / "semantic_contracts" / "registry.json"
	registry = json.loads(registry_path.read_text(encoding="utf-8"))
	surfaces = registry["surfaces"]
	assert surfaces, "registry must name at least one durable contract surface"
	seen_ids: set[str] = set()
	for surface in surfaces:
		missing = REQUIRED - set(surface)
		assert not missing, f"{surface.get('id')}: missing {sorted(missing)}"
		surface_id = surface["id"]
		assert surface_id not in seen_ids, f"duplicate registry id {surface_id}"
		seen_ids.add(surface_id)
		citation = surface["citation"]
		assert any(name in citation for name in AUTHORITY), (
			f"{surface_id} must cite Human Guidance or Terminology"
		)
		assert (repo_root / surface["rust_test"]).is_file(), surface["rust_test"]
		assert (repo_root / surface["typescript_test"]).is_file(), surface["typescript_test"]
		assert (repo_root / surface["postgres_test"]).is_file(), surface["postgres_test"]
		assert POSTGRES_API.fullmatch(surface["postgres_api"]), (
			f"{surface_id} postgres_api must be a ple_api function"
		)
