# local repo modules
import bump_version.contracts
import bump_version.parsing

def update_pyproject(text: str, sections: list[str], new_version: str) -> tuple[str, bool]:
	"""Update version lines in a pyproject.toml string.

	Args:
		text (str): File contents.
		sections (list[str]): Sections to update.
		new_version (str): New version.

	Returns:
		tuple[str, bool]: Updated text and changed flag.
	"""
	lines = text.splitlines(keepends=True)
	changed = False

	active_section = None
	for index, line in enumerate(lines):
		match = bump_version.contracts.SECTION_HEADER_PATTERN.match(line.strip())
		if match:
			active_section = match.group("section")
			continue

		if active_section not in sections:
			continue

		match = bump_version.contracts.VERSION_LINE_PATTERN.match(line)
		if not match:
			continue

		indent = match.group("indent")
		quote = match.group("quote")
		rest = match.group("rest")
		newline = "\n" if line.endswith("\n") else ""
		lines[index] = f"{indent}version = {quote}{new_version}{quote}{rest}{newline}"
		changed = True

	return "".join(lines), changed

#============================================

def normalize_cargo_version(version: str) -> str:
	"""Convert a supported repo version to Cargo's SemVer representation.

	Args:
		version (str): Repo version string.

	Returns:
		str: Three-part SemVer without leading zeroes.
	"""
	details = bump_version.parsing.parse_version_details(version)
	base = f"{details['major']}.{details['minor']}.{details['patch']}"
	if not details["pre_tag"]:
		return base

	tag = bump_version.contracts.CARGO_PRE_TAG_NAMES[details["pre_tag"]]
	pre_num = details["pre_num"] if details["pre_num"] is not None else 0
	cargo_version = f"{base}-{tag}.{pre_num}"
	return cargo_version

#============================================

def normalize_target_version(entry: dict, new_version: str) -> str:
	"""Normalize the target version for entries without a patch segment.

	Args:
		entry (dict): Version entry metadata.
		new_version (str): Target version.

	Returns:
		str: Adjusted version string.
	"""
	if entry["kind"] in ("cargo_toml", "cargo_lock"):
		return normalize_cargo_version(new_version)
	if entry.get("patch_optional") and new_version.endswith(".0"):
		short_version = new_version.replace(".0", "", 1)
		if bump_version.contracts.SHORT_PEP440_PATTERN.match(short_version):
			return short_version
	return new_version

#============================================

def entry_matches_target(entry: dict, new_version: str) -> bool:
	"""Check whether an entry already contains its normalized target version.

	Args:
		entry (dict): Version entry metadata.
		new_version (str): Repo target version.

	Returns:
		bool: True when no update is needed.
	"""
	target_version = normalize_target_version(entry, new_version)
	matches = entry["version"] == target_version
	return matches

#============================================

def update_simple_version(text: str, new_version: str, force_update: bool=False) -> tuple[str, bool]:
	"""Update a simple version file.

	Args:
		text (str): File contents.
		new_version (str): New version.
		force_update (bool): Update first non-empty line even if not a version.

	Returns:
		tuple[str, bool]: Updated text and changed flag.
	"""
	lines = text.splitlines(keepends=True)
	for index, line in enumerate(lines):
		strip_line = line.strip()
		if not strip_line or strip_line.startswith("#"):
			continue
		if not bump_version.parsing.is_version_candidate(strip_line) and not force_update:
			break
		newline = "\n" if line.endswith("\n") else ""
		lines[index] = f"{new_version}{newline}"
		return "".join(lines), True

	if force_update:
		return f"{new_version}\n", True

	return text, False

#============================================

def update_version_py(text: str, new_version: str) -> tuple[str, bool]:
	"""Update version assignments in version.py.

	Args:
		text (str): File contents.
		new_version (str): New version.

	Returns:
		tuple[str, bool]: Updated text and changed flag.
	"""
	lines = text.splitlines(keepends=True)
	changed = False
	for index, line in enumerate(lines):
		match = bump_version.contracts.ASSIGNMENT_PATTERN.match(line)
		if not match:
			continue
		indent = match.group("indent")
		name = match.group("name")
		quote = match.group("quote")
		rest = match.group("rest")
		newline = "\n" if line.endswith("\n") else ""
		lines[index] = f"{indent}{name} = {quote}{new_version}{quote}{rest}{newline}"
		changed = True

	return "".join(lines), changed

#============================================

def update_cargo_lock(text: str, package_index: int, new_version: str) -> tuple[str, bool]:
	"""Update one local package version in Cargo.lock.

	Args:
		text (str): File contents.
		package_index (int): Zero-based index of the package stanza to update.
		new_version (str): New version.

	Returns:
		tuple[str, bool]: Updated text and changed flag.
	"""
	lines = text.splitlines(keepends=True)
	current_package_index = -1
	for index, line in enumerate(lines):
		if bump_version.contracts.CARGO_PACKAGE_HEADER_PATTERN.match(line.strip()):
			current_package_index += 1
			continue
		if current_package_index != package_index:
			continue
		match = bump_version.contracts.VERSION_LINE_PATTERN.match(line)
		if not match:
			continue
		indent = match.group("indent")
		quote = match.group("quote")
		rest = match.group("rest")
		newline = "\n" if line.endswith("\n") else ""
		lines[index] = f"{indent}version = {quote}{new_version}{quote}{rest}{newline}"
		return "".join(lines), True

	return text, False

#============================================

def update_entry(entry: dict, new_version: str, apply: bool) -> dict:
	"""Update a version entry.

	Args:
		entry (dict): Version entry.
		new_version (str): New version.
		apply (bool): Whether to write changes.

	Returns:
		dict: Result summary.
	"""
	path = entry["path"]
	if entry.get("create"):
		text = ""
	else:
		with open(path, "r", encoding="utf-8") as handle:
			text = handle.read()

	version_value = normalize_target_version(entry, new_version)
	if entry["kind"] == "pyproject":
		updated_text, changed = update_pyproject(text, entry["sections"], version_value)
	elif entry["kind"] == "cargo_toml":
		updated_text, changed = update_pyproject(text, entry["sections"], version_value)
	elif entry["kind"] == "cargo_lock":
		updated_text, changed = update_cargo_lock(
			text,
			entry["package_index"],
			version_value,
		)
	elif entry["kind"] == "version_py":
		updated_text, changed = update_version_py(text, version_value)
	else:
		updated_text, changed = update_simple_version(
			text,
			version_value,
			force_update=entry.get("force_update", False),
		)

	if changed and apply:
		with open(path, "w", encoding="utf-8") as handle:
			handle.write(updated_text)

	result = {
		"path": path,
		"changed": changed,
	}
	return result

#============================================
