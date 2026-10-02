# Standard Library
import ast

# local repo modules
import file_utils


#============================================
def future_import_paths() -> list[str]:
	"""Return tracked Python files that import from __future__."""
	paths = []
	for path in file_utils.discover_files(extensions=[".py"]):
		relative_path = file_utils.rel_to_root(path)
		tree, error = file_utils.parse_source(path)
		if error is not None:
			paths.append(relative_path)
			continue
		for node in file_utils.iter_imports(tree):
			if isinstance(node, ast.ImportFrom) and node.module == "__future__":
				paths.append(relative_path)
				break
	return paths


#============================================
def test_python_sources_do_not_import_from_future() -> None:
	"""Human Guidance keeps Python on the current interpreter.

	Failure means a tracked Python file imports from __future__. Remove that
	import. Python 3.12 evaluates the annotations without it.
	"""
	paths = future_import_paths()
	assert paths == [], "Remove from __future__ import from: " + ", ".join(paths)
