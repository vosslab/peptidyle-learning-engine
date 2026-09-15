#!/usr/bin/env python3
"""Fail closed when direct dependencies diverge from the dated registry snapshot."""

import argparse
import datetime
import json
import pathlib
import re
import subprocess
import tomllib


PYPI_MINIMUM_PATTERN = re.compile(
	r"^(?P<name>[A-Za-z0-9_.-]+)(?:\[[A-Za-z0-9_,.-]+\])?>=(?P<version>[0-9][A-Za-z0-9_.+-]*)$"
)
NODE_MINIMUM_PATTERN = re.compile(r"^>=(?P<version>[0-9][0-9A-Za-z.+-]*)$")
SNAPSHOT_NAME = "dependency_freshness_snapshot.json"
REQUIRED_SOURCES = {
	"cargo": "https://crates.io/",
	"node": "https://registry.npmjs.org/",
	"pypi": "https://pypi.org/",
}


#============================================
def parse_args() -> argparse.Namespace:
	"""Parse the one supported audit action."""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("--check", action="store_true", help="validate the checked-in snapshot")
	args = parser.parse_args()
	if not args.check:
		raise ValueError("pass --check to validate dependency freshness")
	return args


#============================================
def read_json(path: pathlib.Path) -> dict[str, object]:
	"""Read one JSON object and reject any other top-level value."""
	value = json.loads(path.read_text(encoding="utf-8"))
	if not isinstance(value, dict):
		raise ValueError(f"{path}: snapshot must be a JSON object")
	return value


#============================================
def read_toml(path: pathlib.Path) -> dict[str, object]:
	"""Read one TOML object and reject any other top-level value."""
	value = tomllib.loads(path.read_text(encoding="utf-8"))
	if not isinstance(value, dict):
		raise ValueError(f"{path}: TOML must be an object")
	return value


#============================================
def require_mapping(value: object, label: str) -> dict[str, object]:
	"""Return a required string-keyed mapping without silently defaulting."""
	if not isinstance(value, dict) or not all(isinstance(key, str) for key in value):
		raise ValueError(f"{label}: expected an object with string keys")
	return value


#============================================
def require_string(value: object, label: str) -> str:
	"""Return a required nonempty string."""
	if not isinstance(value, str) or not value:
		raise ValueError(f"{label}: expected a nonempty string")
	return value


#============================================
def check_snapshot_header(snapshot: dict[str, object]) -> None:
	"""Validate the dated and source-identified registry record."""
	required_keys = {"schema_version", "snapshot_date", "max_snapshot_age_days", "sources", "cargo", "cargo_exceptions", "node", "pypi", "exceptions"}
	if set(snapshot) != required_keys:
		raise ValueError("snapshot: unexpected or missing top-level keys")
	if snapshot["schema_version"] != 1:
		raise ValueError("snapshot: unsupported schema_version")
	snapshot_date = datetime.date.fromisoformat(require_string(snapshot["snapshot_date"], "snapshot_date"))
	max_age = snapshot["max_snapshot_age_days"]
	if not isinstance(max_age, int) or isinstance(max_age, bool) or max_age < 1:
		raise ValueError("max_snapshot_age_days: expected a positive integer")
	age = (datetime.date.today() - snapshot_date).days
	if age < 0 or age > max_age:
		raise ValueError(f"snapshot: {snapshot_date.isoformat()} is outside its {max_age}-day review window")
	sources = require_mapping(snapshot["sources"], "sources")
	if sources != REQUIRED_SOURCES:
		raise ValueError("sources: require the three primary package registries")


#============================================
def lock_versions(lock_data: dict[str, object]) -> dict[str, set[str]]:
	"""Return every registry version named in the checked-in Cargo lockfile."""
	versions: dict[str, set[str]] = {}
	packages = lock_data["package"]
	if not isinstance(packages, list):
		raise ValueError("Cargo.lock: package must be an array")
	for package in packages:
		entry = require_mapping(package, "Cargo.lock package")
		if "source" not in entry or not isinstance(entry["source"], str) or "crates.io-index" not in entry["source"]:
			continue
		name = require_string(entry["name"], "Cargo.lock package name")
		version = require_string(entry["version"], f"Cargo.lock {name} version")
		if name not in versions:
			versions[name] = set()
		versions[name].add(version)
	return versions


#============================================
def cargo_manifest_provenance(repo_root: pathlib.Path) -> dict[str, set[str]]:
	"""Return every direct registry dependency and each workspace manifest that declares it."""
	command = ["cargo", "metadata", "--no-deps", "--format-version", "1"]
	result = subprocess.run(command, cwd=repo_root, check=True, capture_output=True, text=True)
	metadata = require_mapping(json.loads(result.stdout), "cargo metadata")
	packages = metadata["packages"]
	if not isinstance(packages, list):
		raise ValueError("cargo metadata: packages must be an array")
	provenance: dict[str, set[str]] = {}
	for package in packages:
		entry = require_mapping(package, "cargo metadata package")
		manifest_path = pathlib.Path(require_string(entry["manifest_path"], "cargo manifest path"))
		if not manifest_path.is_relative_to(repo_root):
			raise ValueError(f"cargo metadata: manifest outside repository: {manifest_path}")
		relative_manifest = manifest_path.relative_to(repo_root).as_posix()
		dependencies = entry["dependencies"]
		if not isinstance(dependencies, list):
			raise ValueError(f"cargo metadata: {relative_manifest} dependencies must be an array")
		for dependency in dependencies:
			item = require_mapping(dependency, "cargo metadata dependency")
			source = item["source"]
			if source is None:
				continue
			if not isinstance(source, str) or "crates.io-index" not in source:
				raise ValueError(f"cargo metadata: unsupported direct dependency source {source!r}")
			name = require_string(item["name"], "cargo metadata dependency name")
			if item["req"] != "*":
				raise ValueError(f"{relative_manifest}: {name} must keep its open latest-first '*' requirement")
			if name not in provenance:
				provenance[name] = set()
			provenance[name].add(relative_manifest)
	return provenance


#============================================
def check_cargo(repo_root: pathlib.Path, snapshot: dict[str, object]) -> None:
	"""Require every member's latest-first registry dependency and its snapshot lock version."""
	expected = require_mapping(snapshot["cargo"], "cargo")
	actual = cargo_manifest_provenance(repo_root)
	if set(expected) != set(actual):
		raise ValueError("cargo: snapshot dependencies do not match every workspace member manifest")
	resolved = lock_versions(read_toml(repo_root / "Cargo.lock"))
	for name, record in expected.items():
		entry = require_mapping(record, f"cargo {name}")
		if set(entry) != {"version", "manifests"}:
			raise ValueError(f"cargo {name}: require exactly version and manifests")
		expected_version = require_string(entry["version"], f"cargo {name} version")
		manifest_list = entry["manifests"]
		if not isinstance(manifest_list, list) or not all(isinstance(path, str) for path in manifest_list):
			raise ValueError(f"cargo {name}: manifests must be a string array")
		if set(manifest_list) != actual[name] or len(manifest_list) != len(set(manifest_list)):
			raise ValueError(f"cargo {name}: manifest provenance differs from Cargo metadata")
		if name not in resolved or expected_version not in resolved[name]:
			raise ValueError(
				f"Cargo.lock: {name} does not include current registry version {expected_version}; "
				"the latest-first refresh remains unresolved"
			)
	cargo_exceptions = require_mapping(snapshot["cargo_exceptions"], "cargo_exceptions")
	if cargo_exceptions:
		raise ValueError("cargo_exceptions: latest-first direct dependencies do not permit frozen releases")

#============================================
def node_manifest_dependencies(package_data: dict[str, object]) -> dict[str, str]:
	"""Return all direct Node dependency specifications."""
	dependencies: dict[str, str] = {}
	for section in ("dependencies", "devDependencies"):
		for name, requirement in require_mapping(package_data[section], f"package.json {section}").items():
			dependencies[name] = require_string(requirement, f"package.json {name}")
	return dependencies


#============================================
def check_node(repo_root: pathlib.Path, snapshot: dict[str, object]) -> None:
	"""Require current Node minima and exact lockfile resolutions, except approved blockers."""
	package_data = read_json(repo_root / "package.json")
	lock_data = read_json(repo_root / "package-lock.json")
	dependencies = node_manifest_dependencies(package_data)
	expected = require_mapping(snapshot["node"], "node")
	exceptions = require_mapping(snapshot["exceptions"], "exceptions")
	if set(expected) != set(dependencies):
		raise ValueError("node: snapshot dependencies do not match package.json")
	root_lock = require_mapping(require_mapping(lock_data["packages"], "package-lock packages")[""], "package-lock root")
	locked_dependencies = node_manifest_dependencies(root_lock)
	if locked_dependencies != dependencies:
		raise ValueError("package-lock.json: root requirements do not match package.json")
	lock_packages = require_mapping(lock_data["packages"], "package-lock packages")
	for name, recorded in expected.items():
		expected_version = require_string(recorded, f"node {name}")
		requirement = dependencies[name]
		if name in exceptions:
			exception = require_mapping(exceptions[name], f"exception {name}")
			if set(exception) != {"snapshot_date", "blocked_latest", "supported_security_release", "blocker", "manifest_requirement"}:
				raise ValueError(f"exception {name}: unexpected or missing fields")
			if exception["manifest_requirement"] != requirement:
				raise ValueError(f"exception {name}: manifest requirement changed")
			if exception["blocked_latest"] != expected_version:
				raise ValueError(f"exception {name}: blocked latest differs from registry snapshot")
			datetime.date.fromisoformat(require_string(exception["snapshot_date"], f"exception {name} date"))
			require_string(exception["blocker"], f"exception {name} blocker")
			allowed_version = require_string(exception["supported_security_release"], f"exception {name} security release")
		else:
			match = NODE_MINIMUM_PATTERN.fullmatch(requirement)
			if match is None or match["version"] != expected_version:
				raise ValueError(f"package.json: {name} must use >={expected_version}")
			allowed_version = expected_version
		lock_entry = require_mapping(lock_packages.get(f"node_modules/{name}"), f"package-lock {name}")
		if lock_entry.get("version") != allowed_version:
			raise ValueError(f"package-lock.json: {name} must resolve {allowed_version}")
	if set(exceptions) != {"typescript"}:
		raise ValueError("exceptions: only the reviewed TypeScript compatibility exception is permitted")


#============================================
def read_python_requirements(repo_root: pathlib.Path) -> dict[str, str]:
	"""Read latest-first PyPI floors from every repository requirements file."""
	requirements: dict[str, str] = {}
	for path in sorted(repo_root.glob("pip_requirements*.txt")):
		for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
			line = raw_line.split("#", maxsplit=1)[0].strip()
			if not line:
				continue
			match = PYPI_MINIMUM_PATTERN.fullmatch(line)
			if match is None:
				raise ValueError(f"{path.name}:{line_number}: require one >= PyPI floor")
			key = f"{path.name}:{match['name'].lower().replace('_', '-')}"
			if key in requirements:
				raise ValueError(f"{path.name}:{line_number}: duplicate requirement {match['name']}")
			requirements[key] = match["version"]
	return requirements


#============================================
def check_pypi(repo_root: pathlib.Path, snapshot: dict[str, object]) -> None:
	"""Require every PyPI floor to equal the dated primary-registry release."""
	expected = require_mapping(snapshot["pypi"], "pypi")
	actual = read_python_requirements(repo_root)
	if actual != expected:
		raise ValueError("pypi: requirements do not match the dated primary-registry snapshot")


#============================================
def main() -> None:
	"""Run the deterministic freshness gate."""
	parse_args()
	repo_root = pathlib.Path(__file__).resolve().parents[1]
	snapshot = read_json(repo_root / "devel" / SNAPSHOT_NAME)
	check_snapshot_header(snapshot)
	check_cargo(repo_root, snapshot)
	check_node(repo_root, snapshot)
	check_pypi(repo_root, snapshot)
	print("dependency freshness snapshot is current and matches every direct manifest")


if __name__ == "__main__":
	main()
