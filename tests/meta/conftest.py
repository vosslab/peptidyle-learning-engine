"""Path setup for the meta test suite.

Meta tests import repo modules (repolib.*, file_utils, conftest,
detect_repo_type, commit_changelog) through these path entries instead of
each test inserting paths itself.

DUAL-CONFTEST MAP -- this repo carries two conftest.py files with distinct
roles. Read this before touching either one.

  tests/conftest.py (the SHIPPING seed)
    - Handled by repolib.files.merge_conftest, which additively appends its
      three managed blocks (collect_ignore, REPO_HYGIENE_FILTERS,
      OPTIONAL_HELPERS_MENU) into every consumer repo's tests/conftest.py.
    - Because its literal text is the consumer seed, it must hold NO
      repo-specific data. In particular REPO_HYGIENE_FILTERS stays {} here:
      a populated registry would bake this template's glob paths into every
      freshly bootstrapped consumer. Cross-overlay doc references are fixed
      by the backticked-name convention (see docs/MARKDOWN_STYLE.md usage in
      templates/), never by adding exclusions to that registry.

  tests/meta/conftest.py (THIS file, template-meta, never ships)
    - The whole tests/meta/ subtree is excluded from propagation ('meta' is
      a META_DIRS path component), so template-local pytest config belongs
      here, not in the shipping seed.
    - This file must NOT define REPO_HYGIENE_FILTERS. The Layer-2 registry
      loader (file_utils._load_repo_hygiene_filters) reads tests/conftest.py
      by explicit file path precisely because this same-basename file once
      shadowed sys.modules["conftest"] under full-suite collection order
      ("meta" sorts before "test_") and silently emptied the registry.
"""
import os
import sys

# Exclude the template-meta E2E harness from the template's own pytest run.
# This lives here (template-meta, never propagated) rather than in the root
# tests/conftest.py, whose collect_ignore block ships to consumer repos that
# have no tests/meta/ tree.
collect_ignore = ["e2e"]

# Bootstrap: add tests/ so the shared file_utils helper imports. Everything
# else derives from file_utils.get_repo_root() (git rev-parse), not manual walks.
TESTS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if TESTS_DIR not in sys.path:
	sys.path.insert(0, TESTS_DIR)

import file_utils

REPO_ROOT = file_utils.get_repo_root()
search_paths = (
	REPO_ROOT,
	TESTS_DIR,
	os.path.join(REPO_ROOT, "tools"),
	os.path.join(REPO_ROOT, "devel"),
)
for path in search_paths:
	if path not in sys.path:
		sys.path.insert(0, path)
