"""Durable registry of cross-boundary Id/Tuple/Number contracts.

The pytest only checks that each registered surface names existing native
test files. Behavior is proven by those Rust, TypeScript, and PostgreSQL tests.
"""

import json
import pathlib

import file_utils


def test_semantic_contract_registry_points_at_native_coverage() -> None:
	"""Every registered surface has native test files that exist."""
	repo_root = pathlib.Path(file_utils.get_repo_root())
	registry_path = repo_root / "tests" / "semantic_contracts" / "registry.json"
	registry = json.loads(registry_path.read_text(encoding="utf-8"))
	surfaces = registry["surfaces"]
	assert surfaces, "registry must name at least one durable contract surface"
	required = {"id", "behavior", "rust_test", "typescript_test", "postgres_api"}
	seen_ids: set[str] = set()
	for surface in surfaces:
		missing = required - set(surface)
		assert not missing, f"{surface.get('id')}: missing {sorted(missing)}"
		surface_id = surface["id"]
		assert surface_id not in seen_ids, f"duplicate registry id {surface_id}"
		seen_ids.add(surface_id)
		assert (repo_root / surface["rust_test"]).is_file(), surface["rust_test"]
		assert (repo_root / surface["typescript_test"]).is_file(), surface["typescript_test"]
		assert surface["postgres_api"], f"{surface_id} must name a PostgreSQL API"
