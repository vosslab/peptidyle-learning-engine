# Standard Library
import ast
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils

FILES = file_utils.discover_files(
	extensions=(".py",), test_key="support_dirs_not_imported"
)

REPORT_NAME = file_utils.report_name(__file__)

HEADER = "Support-directory import violations"

SUPPORT_ROOTS = frozenset({"tools", "devel", "tests"})

# Module-level dict of repo-relative POSIX key -> list of violation lines.
# Populated by the autouse collect_report fixture before any test runs.
VIOLATIONS_BY_FILE: dict[str, list[str]] = {}


#============================================
def module_root(module_name: str) -> str:
	"""Return the first component of an absolute dotted module name."""
	return module_name.partition(".")[0]


#============================================
def tool_sibling_modules(repo_root: str, importer_rel: str) -> set[str]:
	"""Return importable flat siblings of a tools script from live tree contents."""
	tools_dir = pathlib.Path(repo_root, "tools")
	if not tools_dir.is_dir():
		return set()
	modules = {
		child.stem
		for child in tools_dir.glob("*.py")
		if child.name != "__init__.py"
	}
	for child in tools_dir.iterdir():
		if child.is_dir() and (child / "__init__.py").is_file():
			modules.add(child.name)
	importer = pathlib.PurePosixPath(importer_rel)
	if importer.parent == pathlib.PurePosixPath("tools"):
		modules.discard(importer.stem)
	return modules


#============================================
def format_issue(rule: str, rel: str, line_no: int, module_name: str) -> str:
	"""Format one support-directory import violation with migration guidance."""
	return (
		f"{rule}: {rel}:{line_no} imports {module_name}; "
		"see tools/TOOLS_README.md for the migration direction"
	)


#============================================
def imported_modules(node: ast.Import | ast.ImportFrom) -> list[str]:
	"""Return absolute module paths named by one import statement."""
	if isinstance(node, ast.Import):
		return [alias.name for alias in node.names]
	if node.level != 0 or node.module is None:
		return []
	return [node.module]


#============================================
def check_tree_file(rel: str, tree: ast.Module, repo_root: str) -> list[str]:
	"""Return R1/R2 violations for one parsed file under repo_root."""
	issues = []
	siblings = tool_sibling_modules(repo_root, rel)
	for node in file_utils.iter_imports(tree):
		line_no = getattr(node, "lineno", 0) or 0
		for module_name in imported_modules(node):
			root = module_root(module_name)
			if root in SUPPORT_ROOTS:
				issues.append(format_issue("R1", rel, line_no, module_name))
				continue
			if rel.startswith("tools/") and root in siblings:
				issues.append(format_issue("R2", rel, line_no, module_name))
	return sorted(set(issues))


#============================================
def check_file(rel: str, tree: ast.Module) -> list[str]:
	"""Return support-directory import violations in one repository file."""
	return check_tree_file(rel, tree, file_utils.get_repo_root())


#============================================
@pytest.fixture(scope="module", autouse=True)
def collect_report() -> None:
	"""Populate violations and write the standard report only when needed."""
	file_utils.clear_stale_reports()
	VIOLATIONS_BY_FILE.clear()
	VIOLATIONS_BY_FILE.update(file_utils.collect_python_violations(FILES, check_file))
	lines = file_utils.format_violation_report(HEADER, VIOLATIONS_BY_FILE)
	if lines:
		file_utils.write_report_lines(REPORT_NAME, lines)


#============================================
@pytest.mark.parametrize("path", FILES, ids=file_utils.rel_id)
def test_support_dirs_not_imported(path: str) -> None:
	"""Enforce the support-directory import boundary across the repository."""
	rel = file_utils.rel_to_root(path)
	assert rel not in VIOLATIONS_BY_FILE, file_utils.format_violation_assert_message(
		rel, VIOLATIONS_BY_FILE.get(rel, []), REPORT_NAME
	)


#============================================
def write_module(tmp_path: pathlib.Path, rel: str, source: str) -> pathlib.Path:
	"""Write one synthetic Python module and return its filesystem path."""
	path = tmp_path / rel
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text(source, encoding="utf-8")
	return path


#============================================
def check_synthetic(tmp_path: pathlib.Path, rel: str, source: str) -> list[str]:
	"""Write and parse a synthetic importer before applying the real checker."""
	path = write_module(tmp_path, rel, source)
	tree, error = file_utils.parse_source(str(path))
	assert error is None
	assert tree is not None
	return check_tree_file(rel, tree, str(tmp_path))


#============================================
@pytest.mark.parametrize(
	("importer_rel", "source", "module_name"),
	[
		("app/main.py", "import tools\n", "tools"),
		("tools/a.py", "import tools.runner\n", "tools.runner"),
		("devel/a.py", "from tools import runner\n", "tools"),
		("tests/test_a.py", "from tools.runner import main\n", "tools.runner"),
		("app/main.py", "import devel\n", "devel"),
		("tools/a.py", "import devel.runner\n", "devel.runner"),
		("devel/a.py", "from devel import runner\n", "devel"),
		("tests/test_a.py", "from devel.runner import main\n", "devel.runner"),
		("app/main.py", "import tests\n", "tests"),
		("tools/a.py", "import tests.file_utils\n", "tests.file_utils"),
		("devel/a.py", "from tests import file_utils\n", "tests"),
		("tests/test_a.py", "from tests.file_utils import helper\n", "tests.file_utils"),
	],
)
def test_r1_rejects_every_support_package_import(
	tmp_path: pathlib.Path,
	importer_rel: str,
	source: str,
	module_name: str,
) -> None:
	"""R1 rejects every absolute import form for every support package root."""
	issues = check_synthetic(tmp_path, importer_rel, source)
	assert any("R1" in issue and module_name in issue for issue in issues)


#============================================
@pytest.mark.parametrize("root", sorted(SUPPORT_ROOTS))
@pytest.mark.parametrize(
	"source",
	["import {root}.nested.helper\n", "from {root}.nested.helper import value\n"],
)
def test_r1_rejects_nested_support_package_imports(
	tmp_path: pathlib.Path,
	root: str,
	source: str,
) -> None:
	"""R1 keeps rejecting support-package imports below the first submodule."""
	issues = check_synthetic(tmp_path, "app/main.py", source.format(root=root))
	assert any("R1" in issue and f"{root}.nested.helper" in issue for issue in issues)


#============================================
@pytest.mark.parametrize("source", ["import b\n", "from b import helper\n"])
def test_r2_rejects_actual_tools_siblings(
	tmp_path: pathlib.Path,
	source: str,
) -> None:
	"""R2 rejects both bare import forms when the sibling exists in tools/."""
	write_module(tmp_path, "tools/b.py", "VALUE = 1\n")
	issues = check_synthetic(tmp_path, "tools/a.py", source)
	assert any("R2" in issue and "imports b" in issue for issue in issues)


#============================================
def test_r2_allows_missing_tools_sibling(tmp_path: pathlib.Path) -> None:
	"""R2 permits a bare import when no matching tools sibling exists."""
	issues = check_synthetic(tmp_path, "tools/a.py", "import b\n")
	assert issues == []


#============================================
def test_r2_ignores_non_sibling_imports(tmp_path: pathlib.Path) -> None:
	"""R2 leaves stdlib, declared dependency, and package imports outside tools alone."""
	write_module(
		tmp_path,
		"pyproject.toml",
		"[project]\ndependencies = [\"declared-dependency\"]\n",
	)
	write_module(tmp_path, "package_elsewhere/__init__.py", "")
	issues = check_synthetic(
		tmp_path,
		"tools/a.py",
		"import os\nimport declared_dependency\nimport package_elsewhere\n",
	)
	assert issues == []


#============================================
def test_r3_allows_devel_flat_sibling_helpers(tmp_path: pathlib.Path) -> None:
	"""R3 preserves devel's documented flat sibling-helper exception."""
	write_module(tmp_path, "devel/changelog_lib.py", "VALUE = 1\n")
	issues = check_synthetic(tmp_path, "devel/a.py", "import changelog_lib\n")
	assert issues == []


#============================================
def test_r4_allows_flat_test_helpers(tmp_path: pathlib.Path) -> None:
	"""R4 preserves pytest's flat same-directory test-helper import pattern."""
	write_module(tmp_path, "tests/file_utils.py", "VALUE = 1\n")
	issues = check_synthetic(tmp_path, "tests/test_a.py", "import file_utils\n")
	assert issues == []
# Vendored pytest file. Local changes can and will be overwritten.
