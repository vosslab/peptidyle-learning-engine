"""Enforce a maintainable line-count limit for tracked source files."""

# Standard Library
import os
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils


LINE_LIMIT = 1000
OVERRIDE_LIST = "tests/source_file_line_limit_overrides.txt"

# This gate deliberately owns its own exclusions. Shared hygiene exclusions
# would hide maintained source outside this package's reviewed policy.
EXCLUDED_PREFIXES = (
	".git/", ".pytest_cache/", ".venv/", "OTHER_REPOS/", "coverage/", "dist/",
	"dist_wasm/", "generated/", "node_modules/", "playwright-report/", "target/",
	"test-results/", "docs/archive/", "tests/fixtures/", "tests/artifacts/",
	"tests/e2e/fixtures/", "tests/playwright/fixtures/",
)

# Authored code, templates, queries, and documentation. Generic text/data,
# generated artifacts, configuration, notebooks, and binary formats stay out.
SOURCE_EXTENSIONS = frozenset({
	".ac", ".adoc", ".am", ".asm", ".asciidoc",
	".bash", ".bat", ".bnf",
	".c", ".cc", ".cgi", ".cjs", ".clj", ".cljs", ".cljc", ".cmake",
	".cmd", ".coffee", ".cpp", ".cs", ".css", ".cts", ".cxx",
	".dart", ".dockerfile",
	".el", ".ep", ".erl", ".ex", ".exs",
	".f", ".f03", ".f08", ".f90", ".f95", ".fish", ".fs", ".fsx",
	".go", ".gradle", ".groovy",
	".h", ".hpp", ".hrl", ".hs", ".htm", ".html", ".hxx",
	".i", ".inc",
	".java", ".jl", ".js", ".jst", ".jsx",
	".kt", ".kts",
	".less", ".lhs", ".lisp", ".lua",
	".m", ".markdown", ".maxima", ".md", ".mjs", ".ml", ".mli", ".mm", ".mts",
	".nim", ".nix",
	".pas", ".pg", ".pgml", ".php", ".pl", ".pm", ".pod", ".proto", ".ps1", ".py",
	".qmd", ".qml",
	".r", ".rb", ".rkt", ".rmd", ".rs", ".rst",
	".sass", ".scala", ".scm", ".scss", ".sh", ".sol", ".sql", ".svelte", ".swift",
	".t", ".tcl", ".tcss", ".tex", ".tf", ".ts", ".tsx",
	".v", ".vb", ".vhd", ".vhdl", ".vue",
	".zig", ".zsh",
})

# Common source/build filenames without a useful source extension.
SOURCE_FILENAMES = frozenset({
	"brewfile", "cmakelists.txt", "dockerfile", "gemfile", "jenkinsfile", "justfile",
	"makefile", "meson.build", "pkgbuild", "rakefile", "sconscript", "sconstruct",
	"vagrantfile",
})

DOCUMENTATION_EXTENSIONS = frozenset({".adoc", ".asciidoc", ".markdown", ".md", ".rst", ".txt"})
REPORT_NAME = file_utils.report_name(__file__)
HEADER = "Source file line-limit violations:"
VIOLATIONS_BY_FILE: dict[str, list[str]] = {}


#============================================
def is_allowed_override_path(entry: str) -> bool:
	"""Return whether an override is an allowed immutable or documentation artifact.

	Args:
		entry: Exact repo-relative POSIX path from the override file.

	Returns:
		bool: True only for documentation/history or immutable migration SQL.
	"""
	extension = pathlib.PurePosixPath(entry).suffix.lower()
	is_document = entry.startswith(("docs/", "history/")) and extension in DOCUMENTATION_EXTENSIONS
	is_migration = entry.startswith("schemas/migrations/") and extension == ".sql"
	allowed = is_document or is_migration
	return allowed


#============================================
def load_override_paths(repo_root: str | None = None) -> frozenset[str]:
	"""Load exact manager-approved immutable/documentation exclusions.

	Args:
		repo_root: Repository root. Defaults to the active Git repository root.

	Returns:
		frozenset[str]: Exact repo-relative paths approved for exclusion.
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
			invalid = invalid or any(character in entry for character in "*?[]")
			if invalid or not is_allowed_override_path(entry):
				raise ValueError(
					f"{OVERRIDE_LIST}:{line_number}: override must be an exact documentation/history "
					"artifact or immutable schemas/migrations SQL path"
				)
			overrides.add(entry)
	result = frozenset(overrides)
	return result


OVERRIDE_PATHS = load_override_paths()


#============================================
def is_source_file(rel: str, override_paths: frozenset[str] | None = None) -> bool:
	"""Select authored source files by extension or conventional filename.

	Args:
		rel: Repo-relative POSIX path.
		override_paths: Exact manager-approved paths. Defaults to the active list.

	Returns:
		bool: True when the path is an authored source file covered by the gate.
	"""
	if override_paths is None:
		override_paths = OVERRIDE_PATHS
	if rel.startswith(EXCLUDED_PREFIXES) or rel in override_paths:
		return False
	basename = pathlib.PurePosixPath(rel).name.lower()
	extension = pathlib.PurePosixPath(basename).suffix.lower()
	is_source = basename in SOURCE_FILENAMES or extension in SOURCE_EXTENSIONS
	return is_source


#============================================
def has_symlink_directory(repo_root: str, rel: str) -> bool:
	"""Return whether a tracked path would traverse a directory symlink.

	Args:
		repo_root: Repository root from which the path is resolved.
		rel: Repo-relative POSIX path to inspect.

	Returns:
		bool: True when any parent directory is a symlink.
	"""
	directory = repo_root
	for part in pathlib.PurePosixPath(rel).parts[:-1]:
		directory = os.path.join(directory, part)
		if os.path.islink(directory):
			return True
	return False


#============================================
def discover_source_files(repo_root: str, tracked_paths: list[str] | None = None) -> list[str]:
	"""Discover tracked source paths without shared hygiene exclusions.

	Args:
		repo_root: Repository root from which tracked paths are resolved.
		tracked_paths: Test-only tracked relative path injection.

	Returns:
		list[str]: Sorted repo-relative source paths.
	"""
	if tracked_paths is None:
		tracked_paths = file_utils.list_tracked_files(repo_root)
	paths = []
	for rel in tracked_paths:
		if has_symlink_directory(repo_root, rel):
			continue
		abs_path = os.path.join(repo_root, rel)
		# The index may still name a path deleted in the working tree. A line
		# limit applies only to source bytes that exist; Git review owns deletion
		# detection. Preserve file symlinks here so count_file_lines rejects them.
		if not os.path.lexists(abs_path):
			continue
		if is_source_file(rel):
			paths.append(rel)
	paths.sort()
	return paths


FILES = discover_source_files(file_utils.get_repo_root())


#============================================
def count_file_lines(path: str, rel: str) -> int:
	"""Validate one source file and count its physical lines from bytes.

	Args:
		path: Absolute path to the source file.
		rel: Repo-relative POSIX path for policy failures.

	Returns:
		int: Zero for empty input; otherwise LF count plus final unterminated line.
	"""
	if os.path.islink(path):
		raise ValueError(f"{rel}: maintained source file must not be a symlink")
	with open(path, "rb") as handle:
		contents = handle.read()
	if b"\0" in contents:
		raise ValueError(f"{rel}: maintained source file contains NUL bytes")
	try:
		contents.decode("utf-8")
	except UnicodeDecodeError as error:
		raise ValueError(f"{rel}: maintained source file is not valid UTF-8") from error
	if not contents:
		return 0
	line_count = contents.count(b"\n")
	if not contents.endswith(b"\n"):
		line_count += 1
	return line_count


#============================================
def violations_for_line_count(rel: str, line_count: int) -> list[str]:
	"""Return a violation when a source file reaches the exclusive limit.

	Args:
		rel: Repo-relative POSIX path used in the violation message.
		line_count: Physical line count for the file.

	Returns:
		list[str]: One violation at 1000 or more lines, otherwise an empty list.
	"""
	if line_count < LINE_LIMIT:
		return []
	message = f"{rel}: {line_count} lines; source files must contain fewer than {LINE_LIMIT} lines"
	return [message]


#============================================
def check_file(rel: str) -> list[str]:
	"""Check one tracked source file against the exclusive line limit.

	Args:
		rel: Repo-relative POSIX path to check.

	Returns:
		list[str]: One formatted violation when the file is too long, otherwise empty.
	"""
	abs_path = os.path.join(file_utils.get_repo_root(), rel)
	line_count = count_file_lines(abs_path, rel)
	violations = violations_for_line_count(rel, line_count)
	return violations


#============================================
@pytest.fixture(scope="module", autouse=True)
def collect_report() -> None:
	"""Collect all line-limit violations and write the complete report when dirty."""
	file_utils.clear_stale_reports()
	VIOLATIONS_BY_FILE.clear()
	for rel in FILES:
		violations = check_file(rel)
		if violations:
			VIOLATIONS_BY_FILE[rel] = violations
	lines = file_utils.format_violation_report(HEADER, VIOLATIONS_BY_FILE)
	if lines:
		file_utils.write_report_lines(REPORT_NAME, lines)


#============================================
@pytest.mark.parametrize(
	("line_count", "should_fail"),
	((999, False), (1000, True)),
	ids=("999-lines-ok", "1000-lines-fails"),
)
def test_source_file_line_limit_boundary(line_count: int, should_fail: bool) -> None:
	"""Pin the requested exclusive boundary: 999 passes and 1000 fails."""
	violations = violations_for_line_count("sample.py", line_count)
	assert bool(violations) is should_fail


#============================================
def test_source_file_line_limit_accepts_only_allowed_override_classes(tmp_path: pathlib.Path) -> None:
	"""Allow a documentation override and reject maintained source overrides."""
	tests_dir = tmp_path / "tests"
	tests_dir.mkdir()
	list_path = tests_dir / "source_file_line_limit_overrides.txt"
	list_path.write_text("docs/history.md\n", encoding="utf-8")
	assert load_override_paths(str(tmp_path)) == frozenset({"docs/history.md"})
	list_path.write_text("src/maintained.py\n", encoding="utf-8")
	with pytest.raises(ValueError, match="documentation/history"):
		load_override_paths(str(tmp_path))


#============================================
def test_source_file_line_limit_uses_only_its_reviewed_prefixes(tmp_path: pathlib.Path) -> None:
	"""Keep source under a shared-hygiene exclusion not reviewed by this gate."""
	paths = ["legacy/kept.py", "target/generated.rs", "src/kept.rs"]
	(tmp_path / "legacy").mkdir()
	(tmp_path / "legacy" / "kept.py").write_text("source\n", encoding="utf-8")
	(tmp_path / "target").mkdir()
	(tmp_path / "target" / "generated.rs").write_text("source\n", encoding="utf-8")
	(tmp_path / "src").mkdir()
	(tmp_path / "src" / "kept.rs").write_text("source\n", encoding="utf-8")
	discovered = discover_source_files(str(tmp_path), paths)
	assert discovered == ["legacy/kept.py", "src/kept.rs"]


#============================================
def test_source_file_line_limit_does_not_follow_directory_symlinks(tmp_path: pathlib.Path) -> None:
	"""Skip a tracked path that would resolve through a directory symlink."""
	real_dir = tmp_path / "real"
	real_dir.mkdir()
	(real_dir / "hidden.py").write_text("source\n", encoding="utf-8")
	(tmp_path / "linked").symlink_to(real_dir, target_is_directory=True)
	(tmp_path / "kept.py").write_text("source\n", encoding="utf-8")
	discovered = discover_source_files(str(tmp_path), ["linked/hidden.py", "kept.py"])
	assert discovered == ["kept.py"]


#============================================
def test_source_file_line_limit_ignores_a_tracked_path_deleted_from_worktree(
	tmp_path: pathlib.Path,
) -> None:
	"""Leave deleted-path detection to Git rather than opening absent bytes."""
	(tmp_path / "kept.py").write_text("source\n", encoding="utf-8")
	discovered = discover_source_files(str(tmp_path), ["deleted.py", "kept.py"])
	assert discovered == ["kept.py"]


#============================================
@pytest.mark.parametrize(
	("contents", "expected"),
	((b"", 0), (b"one\n", 1), (b"one", 1), (b"one\r\ntwo\r\n", 2)),
	ids=("empty", "lf", "final-no-lf", "crlf"),
)
def test_source_file_line_limit_counts_physical_lines(
	tmp_path: pathlib.Path,
	contents: bytes,
	expected: int,
) -> None:
	"""Count empty, final-unterminated, and CRLF source files physically."""
	path = tmp_path / "sample.py"
	path.write_bytes(contents)
	assert count_file_lines(str(path), "sample.py") == expected


#============================================
@pytest.mark.parametrize(
	("contents", "message"),
	((b"text\0", "NUL"), (b"\xff", "UTF-8")),
	ids=("nul", "invalid-utf8"),
)
def test_source_file_line_limit_rejects_invalid_source_bytes(
	tmp_path: pathlib.Path,
	contents: bytes,
	message: str,
) -> None:
	"""Reject source bytes that cannot safely be treated as text."""
	path = tmp_path / "sample.py"
	path.write_bytes(contents)
	with pytest.raises(ValueError, match=message):
		count_file_lines(str(path), "sample.py")


#============================================
def test_source_file_line_limit_rejects_source_symlink(tmp_path: pathlib.Path) -> None:
	"""Reject a tracked source file that resolves through a symlink."""
	target = tmp_path / "target.py"
	target.write_text("source\n", encoding="utf-8")
	link = tmp_path / "linked.py"
	link.symlink_to(target)
	with pytest.raises(ValueError, match="linked.py"):
		count_file_lines(str(link), "linked.py")


#============================================
@pytest.mark.parametrize("rel", FILES, ids=lambda rel: rel)
def test_source_file_line_limit(rel: str) -> None:
	"""Fail when a tracked authored source file contains 1000 or more lines."""
	assert rel not in VIOLATIONS_BY_FILE, file_utils.format_violation_assert_message(
		rel, VIOLATIONS_BY_FILE.get(rel, []), REPORT_NAME
	)
