"""WP-C6 dependency-closure security gates."""

# Standard Library
import pathlib
import tomllib

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())


#============================================
def _read_toml(path: pathlib.Path) -> dict[str, object]:
	"""Read one tracked Cargo manifest."""
	with path.open("rb") as handle:
		return tomllib.load(handle)


#============================================
def _locked_versions(package_name: str) -> list[str]:
	"""Return every resolved version for one Cargo package."""
	lock = _read_toml(REPO_ROOT / "Cargo.lock")
	packages = lock["package"]
	assert isinstance(packages, list)
	versions = []
	for package in packages:
		assert isinstance(package, dict)
		if package["name"] == package_name:
			version = package["version"]
			assert isinstance(version, str)
			versions.append(version)
	return versions


#============================================
def _numeric_version(version: str) -> tuple[int, ...]:
	"""Convert the numeric release portion of a semantic version to a tuple."""
	release = version.split("-", maxsplit=1)[0]
	parsed = tuple(int(part) for part in release.split("."))
	return parsed


#============================================
def _workspace_members() -> dict[str, dict[str, object]]:
	"""Map workspace package names to their parsed manifests."""
	root_manifest = _read_toml(REPO_ROOT / "Cargo.toml")
	workspace = root_manifest["workspace"]
	assert isinstance(workspace, dict)
	patterns = workspace["members"]
	assert isinstance(patterns, list)

	members = {}
	for pattern in patterns:
		assert isinstance(pattern, str)
		for member_path in REPO_ROOT.glob(pattern):
			manifest = _read_toml(member_path / "Cargo.toml")
			package = manifest["package"]
			assert isinstance(package, dict)
			name = package["name"]
			assert isinstance(name, str)
			members[name] = manifest
	return members


#============================================
def _workspace_dependency_aliases() -> dict[str, str]:
	"""Resolve renamed packages inherited through workspace dependencies."""
	root_manifest = _read_toml(REPO_ROOT / "Cargo.toml")
	workspace = root_manifest["workspace"]
	assert isinstance(workspace, dict)
	dependencies = workspace.get("dependencies", {})
	assert isinstance(dependencies, dict)

	aliases = {}
	for alias, specification in dependencies.items():
		package_name = alias
		if isinstance(specification, dict):
			override = specification.get("package")
			if isinstance(override, str):
				package_name = override
		aliases[alias] = package_name
	return aliases


#============================================
def _dependency_tables(manifest: dict[str, object]) -> list[dict[str, object]]:
	"""Collect shipped normal and build dependency tables conservatively."""
	tables = []
	for section_name in ("dependencies", "build-dependencies"):
		section = manifest.get(section_name)
		if isinstance(section, dict):
			tables.append(section)

	targets = manifest.get("target")
	if isinstance(targets, dict):
		for target in targets.values():
			if not isinstance(target, dict):
				continue
			for section_name in ("dependencies", "build-dependencies"):
				section = target.get(section_name)
				if isinstance(section, dict):
					tables.append(section)
	return tables


#============================================
def _local_dependencies(
	manifest: dict[str, object],
	members: dict[str, dict[str, object]],
	workspace_aliases: dict[str, str],
) -> set[str]:
	"""Return every workspace package declared as a shipped dependency."""
	dependencies = set()
	for table in _dependency_tables(manifest):
		for alias, specification in table.items():
			package_name = workspace_aliases.get(alias, alias)
			if isinstance(specification, dict):
				override = specification.get("package")
				if isinstance(override, str):
					package_name = override
			if package_name in members:
				dependencies.add(package_name)
	return dependencies


#============================================
def _workspace_closure(root_package: str) -> set[str]:
	"""Resolve the conservative shipped workspace dependency closure."""
	members = _workspace_members()
	workspace_aliases = _workspace_dependency_aliases()
	closure = set()
	pending = [root_package]
	while pending:
		package = pending.pop()
		if package in closure:
			continue
		closure.add(package)
		pending.extend(
			_local_dependencies(members[package], members, workspace_aliases) - closure
		)
	return closure


#============================================
def test_wasm_dependency_closure_is_key_free() -> None:
	"""Prove the browser module reaches only the two approved workspace crates."""
	closure = _workspace_closure("wasm_bridge")
	assert "grading" not in closure, f"server-only grading reached WASM: {sorted(closure)}"
	assert closure == {"wasm_bridge", "domain", "question_model"}


#============================================
def test_resolved_webpki_includes_all_three_security_fixes() -> None:
	"""Reject the legacy rustls-webpki line reported by GitHub security alerts."""
	versions = _locked_versions("rustls-webpki")
	assert versions and min(map(_numeric_version, versions)) >= (0, 103, 13)
