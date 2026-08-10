# Standard Library
import os
import re
import tomllib

# local repo modules
import bump_version.contracts
import bump_version.parsing

def normalize_base_dir(base_dir: str) -> str:
	"""Normalize a base directory path.

	Args:
		base_dir (str): Base directory.

	Returns:
		str: Normalized absolute path.
	"""
	resolved = os.path.abspath(os.path.expanduser(base_dir))
	if not os.path.isdir(resolved):
		raise FileNotFoundError(f"Base directory not found: {resolved}")
	return resolved

#============================================

def normalize_base_version_override(value: str) -> str:
	"""Normalize a base version override string.

	Args:
		value (str): Base version override.

	Returns:
		str: Normalized base version.
	"""
	candidate = value.strip()
	if re.fullmatch(r"\d{2}\.\d{2}", candidate):
		return f"{candidate}.0"
	return candidate

#============================================

def iter_candidate_files(base_dir: str, max_depth: int) -> list[str]:
	"""Find candidate version files.

	Args:
		base_dir (str): Base directory.
		max_depth (int): Max depth to search.

	Returns:
		list[str]: Candidate file paths.
	"""
	base_depth = base_dir.rstrip(os.sep).count(os.sep)
	matches = []
	for root, dirs, files in os.walk(base_dir):
		depth = root.rstrip(os.sep).count(os.sep) - base_depth
		if depth > max_depth:
			dirs[:] = []
			continue

		# ROOT_SKIP_DIRS applies only to the top-level scan directory
		skip_names = (
			bump_version.contracts.SKIP_DIRS | bump_version.contracts.ROOT_SKIP_DIRS
			if depth == 0 else bump_version.contracts.SKIP_DIRS
		)
		dirs[:] = [
			d for d in dirs
			if d not in skip_names and not d.startswith(".")
		]

		for filename in files:
			if filename in bump_version.contracts.CANDIDATE_FILENAMES:
				matches.append(os.path.join(root, filename))

	matches.sort()
	return matches

#============================================

def parse_toml_version(
	data: dict,
	path: str,
	kind: str,
	section_paths: dict,
) -> dict | None:
	"""Parse version metadata from named TOML sections.

	Args:
		data (dict): Parsed TOML data.
		path (str): TOML file path.
		kind (str): Version-entry kind.
		section_paths (dict): Entry section names mapped to TOML key paths.

	Returns:
		dict | None: Entry describing the version, or None if missing.
	"""
	versions = []
	sections = []
	for section_name, section_path in section_paths.items():
		section_data = data
		for key in section_path:
			section_data = section_data.get(key, {})
		version = section_data.get("version")
		# Inherited versions parse as tables (Cargo's version.workspace = true).
		# They are not literal versions and must not be stringified or rewritten.
		if isinstance(version, str) and version:
			versions.append(version)
			sections.append(section_name)

	if not versions:
		return None

	unique_versions = sorted(set(versions))
	if len(unique_versions) > 1:
		raise ValueError(
			f"Conflicting versions in {path}: {', '.join(unique_versions)}"
		)

	entry = {
		"path": path,
		"kind": kind,
		"version": unique_versions[0],
		"sections": sections,
	}

	return entry

#============================================

def parse_pyproject(path: str) -> dict | None:
	"""Parse a pyproject.toml version."""
	with open(path, "rb") as handle:
		data = tomllib.load(handle)
	return parse_toml_version(
		data,
		path,
		"pyproject",
		{"project": ("project",), "tool.poetry": ("tool", "poetry")},
	)

#============================================

def parse_cargo_toml(path: str) -> tuple[dict | None, str]:
	"""Parse version metadata from a Cargo.toml manifest.

	A manifest owns a literal version in [package] version, in
	[workspace.package] version, or in neither. Members that inherit with
	version.workspace = true own no version and yield no entry, while their
	package name is still reported so Cargo.lock stanzas can be matched.

	Args:
		path (str): Cargo.toml path.

	Returns:
		tuple[dict | None, str]: Version entry (None when inherited or absent)
		and the manifest's package name ("" when the manifest declares none).
	"""
	with open(path, "rb") as handle:
		data = tomllib.load(handle)

	package_data = data.get("package", {})
	package_name = package_data.get("name", "")
	if not isinstance(package_name, str):
		package_name = ""

	entry = parse_toml_version(
		data,
		path,
		"cargo_toml",
		{
			"package": ("package",),
			"workspace.package": ("workspace", "package"),
		},
	)
	if entry:
		entry["package_name"] = package_name
	return entry, package_name

#============================================

def parse_cargo_lock(path: str, package_names: set[str]) -> list[dict]:
	"""Parse local package versions from a Cargo.lock file.

	Only package stanzas whose names occur in a discovered Cargo.toml manifest
	are returned. This avoids treating dependency versions as project versions.

	Args:
		path (str): Cargo.lock path.
		package_names (set[str]): Names of local Cargo packages.

	Returns:
		list[dict]: Entries describing local package versions.
	"""
	with open(path, "r", encoding="utf-8") as handle:
		lines = handle.read().splitlines()

	entries = []
	package_index = -1
	index = 0
	while index < len(lines):
		if not bump_version.contracts.CARGO_PACKAGE_HEADER_PATTERN.match(lines[index]):
			index += 1
			continue

		package_index += 1
		package_name = ""
		package_version = ""
		index += 1
		while index < len(lines) and not lines[index].startswith("["):
			match = bump_version.contracts.VERSION_LINE_PATTERN.match(lines[index])
			if match:
				package_version = match.group("version")
			name_match = bump_version.contracts.CARGO_NAME_PATTERN.match(lines[index])
			if name_match:
				package_name = name_match.group("name")
			index += 1

		if package_name in package_names and package_version:
			entries.append({
				"path": path,
				"kind": "cargo_lock",
				"version": package_version,
				"package_index": package_index,
				"package_name": package_name,
			})

	return entries

#============================================

def parse_simple_version_file(path: str, force_update: bool=False) -> dict | None:
	"""Parse a simple version file (VERSION, version.txt, version).

	Args:
		path (str): File path.
		force_update (bool): Treat the first non-empty line as a version.

	Returns:
		dict | None: Entry describing the version, or None if missing.
	"""
	with open(path, "r", encoding="utf-8") as handle:
		lines = handle.read().splitlines()

	for line in lines:
		strip_line = line.strip()
		if not strip_line or strip_line.startswith("#"):
			continue
		if force_update or bump_version.parsing.is_version_candidate(strip_line):
			entry = {
				"path": path,
				"kind": "simple",
				"version": strip_line,
				"force_update": force_update,
			}
			return entry
		return None

	if force_update:
		entry = {
			"path": path,
			"kind": "simple",
			"version": "",
			"force_update": True,
			"create": False,
		}
		return entry

	return None

#============================================

def build_version_file_entry(base_dir: str, version: str="", create: bool=True) -> dict:
	"""Build a VERSION-file entry.

	Args:
		base_dir (str): Base directory.
		version (str): Current version value.
		create (bool): Whether the VERSION file needs to be created.

	Returns:
		dict: Version entry.
	"""
	return {
		"path": os.path.join(base_dir, "VERSION"),
		"kind": "simple",
		"version": version,
		"force_update": True,
		"create": create,
	}

#============================================

def parse_version_py(path: str) -> dict | None:
	"""Parse a version.py file with assignment patterns.

	Args:
		path (str): File path.

	Returns:
		dict | None: Entry describing the version, or None if missing.
	"""
	with open(path, "r", encoding="utf-8") as handle:
		lines = handle.read().splitlines()

	versions = []
	for line in lines:
		match = bump_version.contracts.ASSIGNMENT_PATTERN.match(line)
		if not match:
			continue
		versions.append(match.group("version"))

	if not versions:
		return None

	unique_versions = sorted(set(versions))
	if len(unique_versions) > 1:
		raise ValueError(
			f"Conflicting versions in {path}: {', '.join(unique_versions)}"
		)

	entry = {
		"path": path,
		"kind": "version_py",
		"version": unique_versions[0],
	}
	return entry

#============================================

def parse_versions(base_dir: str, max_depth: int) -> list[dict]:
	"""Scan the repo for version sources.

	Args:
		base_dir (str): Base directory.
		max_depth (int): Max depth to scan.

	Returns:
		list[dict]: List of version entries.
	"""
	entries = []
	candidate_paths = iter_candidate_files(base_dir, max_depth)
	cargo_package_names = set()
	for path in candidate_paths:
		filename = os.path.basename(path)
		if filename == "pyproject.toml":
			entry = parse_pyproject(path)
		elif filename == "Cargo.toml":
			entry, package_name = parse_cargo_toml(path)
			# Collect the name even when the manifest inherits its version, so
			# inheriting members are still matched in Cargo.lock.
			if package_name:
				cargo_package_names.add(package_name)
		elif filename == "version.py":
			entry = parse_version_py(path)
		elif filename == "Cargo.lock":
			continue
		else:
			force_update = filename == "VERSION"
			entry = parse_simple_version_file(path, force_update=force_update)
		if entry:
			entries.append(entry)

	for path in candidate_paths:
		if os.path.basename(path) == "Cargo.lock":
			entries.extend(parse_cargo_lock(path, cargo_package_names))

	return entries

#============================================

def ensure_version_file_entry(entries: list[dict], base_dir: str) -> list[dict]:
	"""Ensure the root VERSION file is represented.

	Args:
		entries (list[dict]): Discovered version entries.
		base_dir (str): Base directory.

	Returns:
		list[dict]: Entries with root VERSION appended when missing.
	"""
	version_path = os.path.join(base_dir, "VERSION")
	for entry in entries:
		if os.path.abspath(entry["path"]) == os.path.abspath(version_path):
			return entries
	if os.path.exists(version_path):
		return entries
	return entries + [build_version_file_entry(base_dir)]

#============================================

def format_entry_label(entry: dict, base_dir: str) -> str:
	"""Return an unambiguous repository-relative entry label.

	Cargo.lock contributes one entry per local package stanza, so the package
	name is included to keep those repeated paths distinguishable.

	Args:
		entry (dict): Version entry.
		base_dir (str): Repository root.

	Returns:
		str: Path, including a Cargo package name when the path repeats.
	"""
	rel_path = os.path.relpath(entry["path"], base_dir)
	if entry["kind"] == "cargo_lock":
		label = f"{rel_path} [{entry['package_name']}]"
	else:
		label = rel_path
	return label

#============================================

def resolve_source_entry(entries: list[dict], source: str) -> dict:
	"""Resolve a source entry by path.

	Args:
		entries (list[dict]): Version entries.
		source (str): Source path.

	Returns:
		dict: Matching entry.
	"""
	if not source:
		raise ValueError("Source path is empty.")

	normalized = os.path.abspath(os.path.expanduser(source))
	for entry in entries:
		if os.path.abspath(entry["path"]) == normalized:
			return entry

	paths = "\n".join(sorted(entry["path"] for entry in entries))
	raise ValueError(f"Source path not found. Known paths:\n{paths}")

#============================================

def choose_base_version(entries: list[dict], source: str) -> str:
	"""Choose a base version.

	Args:
		entries (list[dict]): Version entries.
		source (str): Optional source path.

	Returns:
		str: Base version string.
	"""
	if source:
		source_entry = resolve_source_entry(entries, source)
		return source_entry["version"]

	versions = sorted(set(entry["version"] for entry in entries))
	if len(versions) == 1:
		return versions[0]

	joined = ", ".join(versions)
	raise ValueError(f"Multiple versions found: {joined}. Use --source or --set-version.")

#============================================
