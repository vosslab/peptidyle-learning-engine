"""Tests for the block-aware merge helper merge_conftest.

merge_conftest manages three blocks in a consumer tests/conftest.py: the
collect_ignore block, the REPO_HYGIENE_FILTERS registry block, and the optional
OPTIONAL_HELPERS_MENU block. All three ship additively. These tests exercise the
real shipped template tests/conftest.py as the canonical source so they track the
actual content, and cover: missing dest (full template), one block present (others
appended, existing block untouched), all present (no change), a consumer-set custom
filters value (preserved), menu block edited by the consumer (never overwritten),
and the template-content guard (every menu line is a comment or blank).
"""

# Standard Library
import os
import pathlib

# local repo modules
import file_utils
import repolib.files


SOURCE_FILE = os.path.join(file_utils.get_repo_root(), "tests", "conftest.py")


#============================================
def _write(path: str, content: str) -> None:
	"""Write content to path as UTF-8 text."""
	with open(path, 'w', encoding='utf-8') as handle:
		handle.write(content)


#============================================
def test_no_dest_returns_full_template(tmp_path: pathlib.Path) -> None:
	"""Missing dest returns the full canonical template with all three managed blocks."""
	dest = tmp_path / "conftest.py"

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	assert "collect_ignore" in result
	assert "REPO_HYGIENE_FILTERS" in result
	assert "OPTIONAL_HELPERS_MENU" in result


#============================================
def test_collect_ignore_present_adds_filters_block(tmp_path: pathlib.Path) -> None:
	"""Consumer with collect_ignore but no filters or menu gets both blocks appended."""
	dest = tmp_path / "conftest.py"
	consumer_text = (
		"import pytest\n"
		"\n"
		"collect_ignore = ['e2e', 'playwright', 'local_only']\n"
	)
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	# Missing block is added.
	assert "REPO_HYGIENE_FILTERS" in result
	# Menu block is also appended.
	assert "OPTIONAL_HELPERS_MENU" in result
	# Existing consumer collect_ignore value survives verbatim.
	assert "collect_ignore = ['e2e', 'playwright', 'local_only']" in result
	# Original consumer import line survives.
	assert "import pytest" in result


#============================================
def test_both_blocks_present_adds_menu(tmp_path: pathlib.Path) -> None:
	"""Consumer carrying only collect_ignore and REPO_HYGIENE_FILTERS gets the menu appended."""
	dest = tmp_path / "conftest.py"
	consumer_text = (
		"collect_ignore = ['e2e', 'playwright']\n"
		"\n"
		"REPO_HYGIENE_FILTERS = {}\n"
	)
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	# Menu block is now appended since the source carries OPTIONAL_HELPERS_MENU.
	assert result is not None
	assert "OPTIONAL_HELPERS_MENU" in result
	# Original content is preserved.
	assert "collect_ignore" in result


#============================================
def test_all_three_blocks_present_returns_none(tmp_path: pathlib.Path) -> None:
	"""Consumer carrying all three managed-block tokens needs no change."""
	dest = tmp_path / "conftest.py"
	consumer_text = (
		"collect_ignore = ['e2e', 'playwright']\n"
		"\n"
		"REPO_HYGIENE_FILTERS = {}\n"
		"\n"
		"# === OPTIONAL_HELPERS_MENU ===\n"
	)
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is None


#============================================
def test_custom_filters_preserved_adds_collect_ignore(tmp_path: pathlib.Path) -> None:
	"""Consumer with a custom filters value and no collect_ignore keeps its value."""
	dest = tmp_path / "conftest.py"
	consumer_text = 'REPO_HYGIENE_FILTERS = {"all": ["foo/**"]}\n'
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	# Missing collect_ignore block is added.
	assert "collect_ignore" in result
	# Custom filters value is preserved verbatim.
	assert "foo/**" in result


#============================================
def test_neither_marker_adds_all_three_blocks(tmp_path: pathlib.Path) -> None:
	"""Consumer with only a fixture import gets all three managed blocks appended."""
	dest = tmp_path / "conftest.py"
	_write(str(dest), "import pytest\n")

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	assert "collect_ignore" in result
	assert "REPO_HYGIENE_FILTERS" in result
	assert "OPTIONAL_HELPERS_MENU" in result
	# Original consumer content survives.
	assert "import pytest" in result


#============================================
def test_empty_dest_returns_full_template(tmp_path: pathlib.Path) -> None:
	"""Dest file that is whitespace-only returns the full canonical template."""
	dest = tmp_path / "conftest.py"
	# Whitespace-only content triggers the empty-dest fast path.
	_write(str(dest), "   \n\n   \n")

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	assert "collect_ignore" in result
	assert "REPO_HYGIENE_FILTERS" in result
	assert "OPTIONAL_HELPERS_MENU" in result


#============================================
def test_collect_ignore_only_appends_filters_and_menu(tmp_path: pathlib.Path) -> None:
	"""Consumer with collect_ignore only gets both hygiene block and menu block appended."""
	dest = tmp_path / "conftest.py"
	consumer_text = "collect_ignore = ['e2e', 'playwright']\n"
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	# Both missing blocks are appended.
	assert "REPO_HYGIENE_FILTERS" in result
	assert "OPTIONAL_HELPERS_MENU" in result
	# Existing collect_ignore value is preserved.
	assert "collect_ignore = ['e2e', 'playwright']" in result


#============================================
def test_filters_only_appends_collect_ignore_and_menu(tmp_path: pathlib.Path) -> None:
	"""Consumer with REPO_HYGIENE_FILTERS only gets collect_ignore and menu appended."""
	dest = tmp_path / "conftest.py"
	consumer_text = 'REPO_HYGIENE_FILTERS = {"all": ["scratch/**"]}\n'
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	# Both missing blocks are appended.
	assert "collect_ignore" in result
	assert "OPTIONAL_HELPERS_MENU" in result
	# Consumer custom value survives verbatim.
	assert "scratch/**" in result


#============================================
def test_menu_edited_returns_none_when_all_blocks_present(tmp_path: pathlib.Path) -> None:
	"""Consumer with all three blocks, including an uncommented menu helper, needs no change."""
	dest = tmp_path / "conftest.py"
	# Simulate a consumer who has enabled Recipe 1 by uncommenting sys.path insertion.
	# The long subprocess.check_output line is split to stay under 100 chars.
	git_cmd = "['git', 'rev-parse', '--show-toplevel']"
	consumer_text = (
		"collect_ignore = ['e2e', 'playwright']\n"
		"\n"
		"REPO_HYGIENE_FILTERS = {}\n"
		"\n"
		"# === OPTIONAL_HELPERS_MENU ===\n"
		"import sys\n"
		"import subprocess\n"
		f"_repo_root = subprocess.check_output({git_cmd}, text=True).strip()\n"
		"if _repo_root not in sys.path:\n"
		"    sys.path.insert(0, _repo_root)\n"
	)
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	# Nothing is missing; edited menu must not be rewritten.
	assert result is None


#============================================
def test_menu_edited_appends_only_missing_non_menu_block(tmp_path: pathlib.Path) -> None:
	"""Consumer with edited menu but missing collect_ignore gets collect_ignore appended only."""
	dest = tmp_path / "conftest.py"
	# Has an edited menu (uncommented helper) and REPO_HYGIENE_FILTERS, but no collect_ignore.
	consumer_text = (
		"REPO_HYGIENE_FILTERS = {}\n"
		"\n"
		"# === OPTIONAL_HELPERS_MENU ===\n"
		"import sys  # enabled by consumer\n"
	)
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	assert result is not None
	# Missing block was appended.
	assert "collect_ignore" in result
	# Consumer menu content survives verbatim (never overwritten).
	assert "import sys  # enabled by consumer" in result
	# The original REPO_HYGIENE_FILTERS value survives.
	assert "REPO_HYGIENE_FILTERS = {}" in result


#============================================
def test_template_menu_block_contains_only_comments(tmp_path: pathlib.Path) -> None:
	"""Template-content guard: every non-blank line after the menu header starts with #."""
	source_text = repolib.files.read_text(SOURCE_FILE)
	source_lines = source_text.split('\n')

	# Locate the menu header line.
	menu_start = None
	for idx, line in enumerate(source_lines):
		if 'OPTIONAL_HELPERS_MENU' in line:
			menu_start = idx
			break

	assert menu_start is not None, "OPTIONAL_HELPERS_MENU marker not found in template"

	# Every non-blank line after the header must be a comment.
	active_lines = [
		line for line in source_lines[menu_start + 1:]
		if line.strip() != ''
	]
	non_comment_lines = [line for line in active_lines if not line.lstrip().startswith('#')]
	assert non_comment_lines == [], (
		f"Template menu block contains active (non-comment) lines: {non_comment_lines}"
	)


#============================================
def test_marker2_only_no_menu_appended(tmp_path: pathlib.Path) -> None:
	"""Graceful-degradation: source with marker2 but no marker3 produces two-block behavior.

	Writes a synthetic source that contains the REPO_HYGIENE_FILTERS marker
	(which must start with '# REPO_HYGIENE_FILTERS') but not OPTIONAL_HELPERS_MENU.
	The consumer dest has collect_ignore only, so merge_conftest should append the
	filters block but NOT append a menu block.
	"""
	# Build a synthetic source file with only collect_ignore + REPO_HYGIENE_FILTERS.
	# The marker line must start with '# REPO_HYGIENE_FILTERS' per engine detection.
	synthetic_source = tmp_path / "synthetic_source.py"
	source_content = (
		"collect_ignore = ['e2e', 'playwright']\n"
		"\n"
		"# REPO_HYGIENE_FILTERS is the registry.\n"
		"REPO_HYGIENE_FILTERS = {}\n"
	)
	_write(str(synthetic_source), source_content)

	dest = tmp_path / "conftest.py"
	_write(str(dest), "collect_ignore = ['e2e']\n")

	result = repolib.files.merge_conftest(str(synthetic_source), str(dest))

	# Filters block was appended; menu block should NOT appear (source lacks it).
	assert result is not None
	assert "REPO_HYGIENE_FILTERS" in result
	assert "OPTIONAL_HELPERS_MENU" not in result


#============================================
def test_marker3_only_no_crash(tmp_path: pathlib.Path) -> None:
	"""Graceful-degradation: source with marker3 but no marker2 does not crash.

	Writes a synthetic source that carries OPTIONAL_HELPERS_MENU but not
	REPO_HYGIENE_FILTERS. merge_conftest should complete without error and
	whatever it ships should contain what exists in the source.
	"""
	# Synthetic source: collect_ignore block + menu block, no filters block.
	synthetic_source = tmp_path / "synthetic_source.py"
	source_content = (
		"collect_ignore = ['e2e', 'playwright']\n"
		"\n"
		"# === OPTIONAL_HELPERS_MENU ===\n"
		"# # Recipe 1: add repo root to sys.path\n"
	)
	_write(str(synthetic_source), source_content)

	dest = tmp_path / "conftest.py"
	_write(str(dest), "collect_ignore = ['e2e']\n")

	# Should not raise; returned value is whatever was produced.
	result = repolib.files.merge_conftest(str(synthetic_source), str(dest))

	# The call must complete without raising an exception.
	# (result may be None or a string depending on what changed; either is acceptable)
	# If a value is returned it must at minimum preserve collect_ignore.
	if result is not None:
		assert "collect_ignore" in result


#============================================
def test_source_neither_marker_only_collect_ignore(tmp_path: pathlib.Path) -> None:
	"""Graceful-degradation: source with only collect_ignore; full content treated as collect block.

	When the source has no REPO_HYGIENE_FILTERS and no OPTIONAL_HELPERS_MENU marker,
	merge_conftest appends nothing beyond what it can extract. A dest that also
	lacks all markers gets the source content applied.
	"""
	# Synthetic source: only a collect_ignore block, no other managed-block markers.
	synthetic_source = tmp_path / "synthetic_source.py"
	source_content = "collect_ignore = ['e2e', 'playwright']\n"
	_write(str(synthetic_source), source_content)

	dest = tmp_path / "conftest.py"
	_write(str(dest), "import pytest\n")

	result = repolib.files.merge_conftest(str(synthetic_source), str(dest))

	# Should not raise; dest lacked collect_ignore so something is appended.
	if result is not None:
		# Whatever was added, the original import line must survive.
		assert "import pytest" in result


#============================================
def test_collect_ignore_prose_mention_skips_block(tmp_path: pathlib.Path) -> None:
	"""Prose mention of collect_ignore in a comment causes the block to be skipped.

	The engine uses substring detection: 'collect_ignore' not in dest_text.
	A comment line containing the token (e.g. '# collect_ignore handled elsewhere')
	satisfies that check, so the collect_ignore block is NOT appended even though
	no actual collect_ignore assignment exists.

	This is the known substring-detection limitation: the engine cannot distinguish
	a prose mention from a real assignment. This test documents and asserts the
	actual current behavior so a future engine change surfaces here immediately.
	"""
	dest = tmp_path / "conftest.py"
	# Comment mentions collect_ignore but there is no actual assignment.
	consumer_text = (
		"# collect_ignore handled elsewhere\n"
		"import pytest\n"
	)
	_write(str(dest), consumer_text)

	result = repolib.files.merge_conftest(SOURCE_FILE, str(dest))

	# ACTUAL BEHAVIOR: because the token 'collect_ignore' appears in the dest text,
	# the engine treats it as already present and does NOT append the collect_ignore block.
	# The REPO_HYGIENE_FILTERS and OPTIONAL_HELPERS_MENU blocks ARE still missing,
	# so the engine returns a non-None result with those appended.
	assert result is not None
	# The hygiene and menu blocks are appended (they were absent).
	assert "REPO_HYGIENE_FILTERS" in result
	assert "OPTIONAL_HELPERS_MENU" in result
	# collect_ignore block is NOT re-appended as an assignment because the token
	# was found in prose. The result has no 'collect_ignore =' assignment line.
	# (Note: the appended REPO_HYGIENE_FILTERS block comment also mentions the token,
	# so a simple count check is fragile. We assert via assignment-pattern absence.)
	assert "collect_ignore =" not in result
