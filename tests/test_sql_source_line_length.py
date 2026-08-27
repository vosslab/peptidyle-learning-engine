"""Enforce a pathological-line boundary for authored SQL source."""

# Standard Library
import os
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils


PATHOLOGICAL_LINE_LENGTH = 1000
OVERRIDE_LIST = "tests/source_sql_line_length_overrides.txt"
REPORT_NAME = file_utils.report_name(__file__)
HEADER = "SQL source pathological-line violations:"

VIOLATIONS_BY_FILE: dict[str, list[str]] = {}
OVERRIDE_VIOLATIONS: list[str] = []


#============================================
def load_override_paths(repo_root: str | None = None) -> frozenset[str]:
	"""
	Load exact, documented paths exempted from the SQL sentinel check.

	Comments immediately before an entry document why an immutable path is
	approved. Entries themselves remain one exact repo-relative POSIX path per
	line, so the parser cannot express a directory or glob exemption.

	Args:
		repo_root: Repository root. Defaults to the active Git repository root.

	Returns:
		frozenset[str]: Exact repo-relative SQL paths from the optional registry.
	"""
	if repo_root is None:
		repo_root = file_utils.get_repo_root()
	list_path = os.path.join(repo_root, OVERRIDE_LIST)
	if not os.path.isfile(list_path):
		return frozenset()
	overrides = set()
	with open(list_path, "r", encoding="utf-8") as handle:
		for line_number, raw_line in enumerate(handle, start=1):
			entry = raw_line.strip()
			if not entry or entry.startswith("#"):
				continue
			parts = entry.split("/")
			invalid = entry.startswith("/") or "\\" in entry or ".." in parts
			invalid = invalid or "." in parts or any(character in entry for character in "*?[]")
			if invalid:
				raise ValueError(
					f"{OVERRIDE_LIST}:{line_number}: expected an exact repo-relative POSIX path"
				)
			if entry in overrides:
				raise ValueError(
					f"{OVERRIDE_LIST}:{line_number}: duplicate override path {entry}"
				)
			overrides.add(entry)
	return frozenset(overrides)


OVERRIDE_PATHS = load_override_paths()


#============================================
def violations_for_source(rel: str, data: bytes) -> list[str]:
	"""
	Return path, line, and length evidence for pathological source lines.

	SQL source is required to be ASCII by a separate permanent gate. Measuring
	raw bytes here therefore measures the same code-point length without
	decoding or introducing locale-dependent behavior. ``splitlines`` removes
	LF, CRLF, and CR line endings before measuring each physical line.

	Args:
		rel: Repo-relative POSIX path used in violation messages.
		data: Raw SQL source bytes.

	Returns:
		list[str]: One evidence line for each source line at or above the sentinel.
	"""
	violations = []
	for line_number, line in enumerate(data.splitlines(), start=1):
		line_length = len(line)
		if line_length < PATHOLOGICAL_LINE_LENGTH:
			continue
		violations.append(
			f"{rel}:{line_number}: line length {line_length} characters "
			f"(pathological threshold {PATHOLOGICAL_LINE_LENGTH})"
		)
	return violations


#============================================
def check_file(rel: str) -> list[str]:
	"""
	Check one discovered SQL file, honoring only an exact approved override.

	Args:
		rel: Repo-relative POSIX path to a discovered SQL file.

	Returns:
		list[str]: Pathological-line evidence, or an empty list when clean.
	"""
	if rel in OVERRIDE_PATHS:
		return []
	abs_path = os.path.join(file_utils.get_repo_root(), rel)
	with open(abs_path, "rb") as handle:
		data = handle.read()
	return violations_for_source(rel, data)


#============================================
def validate_override_paths(
	override_paths: frozenset[str],
	files: list[str],
	repo_root: str | None = None,
) -> list[str]:
	"""
	Reject stale, non-SQL, untracked, or unnecessary override entries.

	Args:
		override_paths: Exact entries loaded from the override registry.
		files: Absolute SQL paths returned by ``file_utils.discover_files``.
		repo_root: Repository root used to resolve paths.

	Returns:
		list[str]: Registry errors, empty when every entry is justified.
	"""
	if repo_root is None:
		repo_root = file_utils.get_repo_root()
	discovered = {file_utils.rel_to_root(path, repo_root) for path in files}
	errors = []
	for rel in sorted(override_paths):
		abs_path = os.path.join(repo_root, rel)
		if not os.path.isfile(abs_path):
			errors.append(f"{OVERRIDE_LIST}: missing override path {rel}")
			continue
		if os.path.splitext(rel)[1].lower() != ".sql":
			errors.append(f"{OVERRIDE_LIST}: override is not SQL: {rel}")
			continue
		if rel not in discovered:
			errors.append(f"{OVERRIDE_LIST}: path is not a tracked discovered SQL file: {rel}")
			continue
		with open(abs_path, "rb") as handle:
			data = handle.read()
		if not violations_for_source(rel, data):
			errors.append(f"{OVERRIDE_LIST}: override is no longer needed: {rel}")
	return errors


FILES = file_utils.discover_files(
	extensions=(".sql",),
	test_key="sql_source_line_length",
)


#============================================
@pytest.fixture(scope="module", autouse=True)
def collect_report() -> None:
	"""Collect SQL sentinel violations and write the standard report when dirty."""
	file_utils.clear_stale_reports()
	VIOLATIONS_BY_FILE.clear()
	OVERRIDE_VIOLATIONS.clear()
	OVERRIDE_VIOLATIONS.extend(validate_override_paths(OVERRIDE_PATHS, FILES))
	if OVERRIDE_VIOLATIONS:
		VIOLATIONS_BY_FILE[OVERRIDE_LIST] = list(OVERRIDE_VIOLATIONS)
	VIOLATIONS_BY_FILE.update(file_utils.collect_file_violations(FILES, check_file))
	lines = file_utils.format_violation_report(HEADER, VIOLATIONS_BY_FILE)
	if lines:
		file_utils.write_report_lines(REPORT_NAME, lines)


#============================================
@pytest.mark.parametrize(
	("data", "expected"),
	(
		(b"x" * 999, []),
		(
			b"x" * 1000,
			["sample.sql:1: line length 1000 characters (pathological threshold 1000)"],
		),
		(
			b"x" * 1001,
			["sample.sql:1: line length 1001 characters (pathological threshold 1000)"],
		),
	),
	ids=("999-characters-ok", "1000-characters-fails", "1001-characters-fails"),
)
def test_sql_source_line_length_boundary(data: bytes, expected: list[str]) -> None:
	"""Accept ordinary long SQL and reject the inclusive pathological sentinel."""
	violations = violations_for_source("sample.sql", data)
	assert violations == expected


#============================================
def test_sql_override_registry_is_current() -> None:
	"""Require every exact override to be an active, justified SQL exemption."""
	assert not OVERRIDE_VIOLATIONS, "\n".join(OVERRIDE_VIOLATIONS)


#============================================
def test_sql_override_loader_rejects_broad_paths(tmp_path: pathlib.Path) -> None:
	"""Keep the override parser limited to exact repo-relative paths."""
	tests_dir = tmp_path / "tests"
	tests_dir.mkdir()
	list_path = tests_dir / "source_sql_line_length_overrides.txt"
	list_path.write_text("schemas/migrations/*.sql\n", encoding="utf-8")
	with pytest.raises(ValueError, match="exact repo-relative POSIX path"):
		load_override_paths(str(tmp_path))


#============================================
@pytest.mark.parametrize("path", FILES, ids=file_utils.rel_id)
def test_sql_source_line_length(path: str) -> None:
	"""Fail when a tracked authored SQL line reaches 1000 characters."""
	rel = file_utils.rel_to_root(path)
	assert rel not in VIOLATIONS_BY_FILE, file_utils.format_violation_assert_message(
		rel, VIOLATIONS_BY_FILE.get(rel, []), REPORT_NAME
	)
