"""
Behavior tests for the REPO_TYPE inheritance DAG.

Covers repolib.model.ancestors/effective_type_chain ordering and cycle guard,
repolib.manifests.build_repo_type_inherits load-time validation (unknown parent,
cycle), a disjointness guard that fails when two chain members claim the same
file_rel, ancestor-conditional overlay inheritance via a synthetic manifest
(mirroring SYNTHETIC_MANIFEST_YAML in test_manifest_loader.py), and a routing
matrix over the live template tree that checks three anchor assets across every
base and concrete repo_type.
"""

# Standard Library
import os
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils
import repolib.files
import repolib.manifests
import repolib.model


TEMPLATE_ROOT = file_utils.get_repo_root()


#============================================
# ancestors() / effective_type_chain() ordering
#============================================

def test_ancestors_walks_nearest_parent_first(monkeypatch: pytest.MonkeyPatch) -> None:
	"""ancestors() returns the parent chain, nearest first, for a multi-level DAG."""
	synthetic_parents = {'grandchild': 'child', 'child': 'parent'}
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', synthetic_parents)
	assert repolib.model.ancestors('grandchild') == ['child', 'parent']


def test_ancestors_of_root_type_is_empty(monkeypatch: pytest.MonkeyPatch) -> None:
	"""A root type (no entry in REPO_TYPE_PARENTS) has an empty ancestor chain."""
	synthetic_parents = {'child': 'parent'}
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', synthetic_parents)
	assert repolib.model.ancestors('parent') == []


def test_effective_type_chain_prepends_repo_type(monkeypatch: pytest.MonkeyPatch) -> None:
	"""effective_type_chain() returns [repo_type, *ancestors(repo_type)]."""
	synthetic_parents = {'grandchild': 'child', 'child': 'parent'}
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', synthetic_parents)
	assert repolib.model.effective_type_chain('grandchild') == ['grandchild', 'child', 'parent']


def test_live_typescript_chain_reaches_website(monkeypatch: pytest.MonkeyPatch) -> None:
	"""The live manifest chains typescript to website (decision 2 of the plan)."""
	# No monkeypatch here: exercises the real repo_type_inherits manifest.
	assert repolib.model.effective_type_chain('typescript') == ['typescript', 'website']


#============================================
# ancestors() cycle guard (model-level, second guard behind the loader)
#============================================

def test_ancestors_raises_on_cycle(monkeypatch: pytest.MonkeyPatch) -> None:
	"""A cyclic REPO_TYPE_PARENTS map raises instead of looping forever."""
	synthetic_parents = {'a': 'b', 'b': 'a'}
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', synthetic_parents)
	with pytest.raises(ValueError, match='cycle'):
		repolib.model.ancestors('a')


#============================================
# build_repo_type_inherits() load-time validation
#============================================

def test_build_repo_type_inherits_raises_on_unknown_parent() -> None:
	"""A parent token absent from known_repo_types raises ValueError."""
	with pytest.raises(ValueError, match='not a known_repo_types token'):
		repolib.manifests.build_repo_type_inherits(
			{'typescript': 'not_a_real_base'}, ('typescript', 'other'),
		)


def test_build_repo_type_inherits_raises_on_unknown_child() -> None:
	"""A child token absent from known_repo_types raises ValueError."""
	with pytest.raises(ValueError, match='not a known_repo_types token'):
		repolib.manifests.build_repo_type_inherits(
			{'not_a_real_type': 'other'}, ('typescript', 'other'),
		)


def test_build_repo_type_inherits_raises_on_cycle() -> None:
	"""A cyclic child->parent map raises ValueError at load time."""
	with pytest.raises(ValueError, match='cycle'):
		repolib.manifests.build_repo_type_inherits(
			{'a': 'b', 'b': 'a'}, ('a', 'b'),
		)


#============================================
# Disjointness guard: two chain members must never claim the same file_rel
#============================================

def chain_file_rel_sets(template_root: str, repo_type: str) -> dict:
	"""
	Map each chain member token to the file_rel set under its own overlay roots.

	Walks single_type_overlay_roots() for each member of effective_type_chain
	independently (never unioned), so a caller can compare one member's files
	against another's to detect a routing collision.

	Args:
		template_root (str): Template root directory.
		repo_type (str): Repository type to expand into its inheritance chain.

	Returns:
		dict: chain member token -> set of file_rel paths it contributes.
	"""
	rel_sets_by_type = {}
	for chain_type in repolib.model.effective_type_chain(repo_type):
		rels = set()
		for overlay_root in repolib.model.single_type_overlay_roots(template_root, chain_type):
			if not os.path.isdir(overlay_root):
				continue
			for dirpath, _dirs, filenames in os.walk(overlay_root):
				for filename in filenames:
					abs_path = os.path.join(dirpath, filename)
					rels.add(os.path.relpath(abs_path, overlay_root))
		rel_sets_by_type[chain_type] = rels
	return rel_sets_by_type


def assert_chain_disjoint(template_root: str, repo_type: str) -> None:
	"""
	Raise AssertionError if two members of repo_type's chain claim one file_rel.

	Args:
		template_root (str): Template root directory.
		repo_type (str): Repository type to expand into its inheritance chain.
	"""
	rel_sets_by_type = chain_file_rel_sets(template_root, repo_type)
	chain = list(rel_sets_by_type.keys())
	for i in range(len(chain)):
		for j in range(i + 1, len(chain)):
			overlap = rel_sets_by_type[chain[i]] & rel_sets_by_type[chain[j]]
			assert not overlap, (
				f"disjointness violated for {repo_type!r}: {chain[i]!r} and "
				f"{chain[j]!r} both claim {sorted(overlap)!r}"
			)


@pytest.mark.parametrize('repo_type', ['python', 'rust', 'swift', 'typescript'])
def test_live_chain_is_disjoint(repo_type: str) -> None:
	"""Every concrete type's live overlay chain ships distinct consumer paths."""
	assert_chain_disjoint(TEMPLATE_ROOT, repo_type)


def test_disjointness_guard_fails_on_synthetic_collision(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The guard fails loudly when a child and its parent both ship the same file_rel."""
	# Build a synthetic template tree where child and parent both claim dup.txt.
	child_dir = tmp_path / 'templates' / 'child'
	parent_dir = tmp_path / 'templates' / 'parent'
	child_dir.mkdir(parents=True)
	parent_dir.mkdir(parents=True)
	(child_dir / 'dup.txt').write_text('from child')
	(parent_dir / 'dup.txt').write_text('from parent')
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', {'child': 'parent'})
	monkeypatch.setattr(repolib.model, 'CONDITIONAL_OVERLAYS', {})
	with pytest.raises(AssertionError, match='dup.txt'):
		assert_chain_disjoint(str(tmp_path), 'child')


#============================================
# Ancestor-conditional overlay inheritance (synthetic manifest)
#
# Mirrors the SYNTHETIC_MANIFEST_YAML pattern in test_manifest_loader.py: write a
# minimal-but-complete manifests.yaml, load it through the real loader, then
# apply the loaded sections to repolib.model so select_overlay_dirs exercises
# the real inheritance-plus-conditional-overlay expansion end to end.
#============================================

SYNTHETIC_INHERITANCE_YAML = """\
routing_overrides: {}
conditional_overlays:
  website:
    _cond:
      when: has_file
      path: WEBSITE_MARKER.txt
      description: synthetic ancestor-conditional overlay
shared_overlays: {}
root_propagate_allowlist: []
universal_noexist: []
merge_files: []
meta_files: []
meta_file_patterns: []
meta_dirs: []
skip_walk_dirs: []
auto_discover_docs_exclude: []
default_repo_skip_names: []
meta_test_prefixes: []
known_repo_types:
  - typescript
  - website
repo_type_inherits:
  typescript: website
"""


def write_synthetic_manifest(template_root: pathlib.Path, body: str) -> str:
	"""
	Write a manifests.yaml under template_root/meta/propagation/.

	Args:
		template_root (pathlib.Path): Synthetic template root directory.
		body (str): Raw YAML text to write.

	Returns:
		str: The template root as a string for passing to load_manifests.
	"""
	manifest_path = template_root / repolib.manifests.MANIFESTS_REL_PATH
	manifest_path.parent.mkdir(parents=True, exist_ok=True)
	manifest_path.write_text(body, encoding='utf-8')
	return str(template_root)


def test_ancestor_conditional_overlay_reaches_child_when_marker_present(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""typescript reaches the website/_cond overlay when its marker file exists."""
	manifest_root = write_synthetic_manifest(tmp_path / 'template', SYNTHETIC_INHERITANCE_YAML)
	manifests = repolib.manifests.load_manifests(manifest_root)
	monkeypatch.setattr(repolib.model, 'CONDITIONAL_OVERLAYS', manifests['conditional_overlays'])
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', manifests['repo_type_inherits'])

	consumer_dir = tmp_path / 'consumer'
	consumer_dir.mkdir()
	(consumer_dir / 'WEBSITE_MARKER.txt').write_text('marker present')

	overlay_dirs = repolib.model.select_overlay_dirs('typescript', str(consumer_dir))
	assert 'website/_cond' in overlay_dirs


def test_ancestor_conditional_overlay_absent_when_marker_missing(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""typescript does NOT receive website/_cond when the marker file is absent."""
	manifest_root = write_synthetic_manifest(tmp_path / 'template', SYNTHETIC_INHERITANCE_YAML)
	manifests = repolib.manifests.load_manifests(manifest_root)
	monkeypatch.setattr(repolib.model, 'CONDITIONAL_OVERLAYS', manifests['conditional_overlays'])
	monkeypatch.setattr(repolib.model, 'REPO_TYPE_PARENTS', manifests['repo_type_inherits'])

	consumer_dir = tmp_path / 'consumer'
	consumer_dir.mkdir()
	# No WEBSITE_MARKER.txt written this time.

	overlay_dirs = repolib.model.select_overlay_dirs('typescript', str(consumer_dir))
	assert 'website/_cond' not in overlay_dirs


#============================================
# Routing matrix: three anchor assets across every base and concrete type
#============================================

# Anchor assets and the bucket compute_propagation_plan sorts each into.
# PLAYWRIGHT_TEST_STYLE_MD lives under templates/website/docs/ -> overwrite_files.
# TSCONFIG_JSON lives under templates/typescript/noexist/ -> noexist_files.
# MAKE_RELEASE_PY is a shared_overlays devel file, stored as a bare basename in
# devel_files (the devel_files bucket drops the 'devel/' prefix).
PLAYWRIGHT_TEST_STYLE_MD = 'docs/PLAYWRIGHT_TEST_STYLE.md'
TSCONFIG_JSON = 'tsconfig.json'
MAKE_RELEASE_PY = 'make_release.py'

# (repo_type, playwright_style_present, tsconfig_present, make_release_present)
ROUTING_MATRIX = [
	('python', False, False, True),
	('rust', False, False, True),
	('swift', False, False, True),
	('typescript', True, True, False),
	('website', True, False, False),
	('other', False, False, True),
	('scripted', False, False, True),
	('compiled', False, False, True),
]


@pytest.mark.parametrize(
	'repo_type, expect_playwright_style, expect_tsconfig, expect_make_release',
	ROUTING_MATRIX,
)
def test_routing_matrix_anchor_assets(
	repo_type: str,
	expect_playwright_style: bool,
	expect_tsconfig: bool,
	expect_make_release: bool,
) -> None:
	"""Each repo_type ships (or omits) the three anchor assets per the DAG design."""
	plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, repo_type)
	assert (PLAYWRIGHT_TEST_STYLE_MD in plan['overwrite_files']) == expect_playwright_style
	assert (TSCONFIG_JSON in plan['noexist_files']) == expect_tsconfig
	assert (MAKE_RELEASE_PY in plan['devel_files']) == expect_make_release


#============================================
# 'all' regression: new base tokens must not change the aggregate shipped set
#============================================

# The concrete types that made up the 'all' aggregate before this version added
# scripted/website/compiled as directly selectable base tokens. Comparing the
# live 'all' plan against the union of just these legacy types proves the new
# base tokens add no new files (they only route existing website/typescript
# content, already reachable via typescript's inheritance).
LEGACY_CONCRETE_TYPES = ('python', 'typescript', 'rust', 'swift', 'other')


def _union_plan(repo_types: tuple) -> dict:
	"""
	Union the buckets of compute_propagation_plan across several repo_types.

	Args:
		repo_types (tuple): Repository type tokens to aggregate.

	Returns:
		dict: Bucket name -> sorted list of the union of entries across types.
	"""
	union = {
		'overwrite_files': set(),
		'noexist_files': set(),
		'merge_files': set(),
		'devel_files': set(),
		'test_files': set(),
	}
	for repo_type in repo_types:
		plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, repo_type)
		for bucket in union:
			union[bucket].update(plan[bucket])
	return union


def test_all_set_equals_legacy_concrete_union() -> None:
	"""REPO_TYPE=all ships the same file set as the union of the pre-change types."""
	all_plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, 'all')
	legacy_union = _union_plan(LEGACY_CONCRETE_TYPES)
	for bucket in legacy_union:
		assert set(all_plan[bucket]) == legacy_union[bucket], (
			f"bucket {bucket!r} differs between REPO_TYPE=all and the legacy union"
		)


def test_all_includes_playwright_test_style() -> None:
	"""REPO_TYPE=all ships docs/PLAYWRIGHT_TEST_STYLE.md (via the website overlay)."""
	all_plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, 'all')
	assert PLAYWRIGHT_TEST_STYLE_MD in all_plan['overwrite_files']


def test_website_only_asset_absent_from_non_website_type() -> None:
	"""A website-only asset ships to website but not to a type outside its chain."""
	website_plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, 'website')
	rust_plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, 'rust')
	assert PLAYWRIGHT_TEST_STYLE_MD in website_plan['overwrite_files']
	assert PLAYWRIGHT_TEST_STYLE_MD not in rust_plan['overwrite_files']
