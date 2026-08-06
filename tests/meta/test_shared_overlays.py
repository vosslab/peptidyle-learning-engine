"""
Regression tests for the shared-overlay routing primitive.

Guards `repolib.model` shared-overlay helpers and the two-way sync between
templates/shared/ on disk and the SHARED_OVERLAYS manifest.

Covers:
  (1) shared_rule_ships_to: ship/skip by repo_types, unconditional rules,
      the lacks_file condition, and unknown-verb error.
  (2) shared_path_ships: any-matching-rule ships the file.
  (3) all_shared_overlay_paths: union of every rule's paths list.
  (4) Coverage guard (disk -> manifest): compute_propagation_plan raises on an
      uncovered templates/shared/ file.

Behavioral tests use synthetic rule dicts so they pass independent of the live
manifest. The source_release rule is registered, so SHARED_OVERLAYS is non-empty.
The coverage guard uses tmp_path to build a fake shared tree and confirms the
RuntimeError fires for any uncovered file. This is a folder-based propagation
system: disk drives routing, so there is no manifest -> disk existence check.
"""

# Standard Library
import pathlib

# PIP3 modules
import pytest

# local repo modules
import file_utils
import repolib.files
import repolib.model


TEMPLATE_ROOT = file_utils.get_repo_root()


#============================================
# shared_rule_ships_to: ship/skip by repo_types
#============================================

def test_ships_to_listed_repo_type(tmp_path: pathlib.Path) -> None:
	"""Rule ships when repo_type appears in its repo_types list."""
	rule = {'paths': ['devel/make_release.py'], 'repo_types': ['rust', 'swift']}
	result = repolib.model.shared_rule_ships_to(rule, 'my_rule', 'rust', str(tmp_path))
	assert result is True


def test_skips_unlisted_repo_type(tmp_path: pathlib.Path) -> None:
	"""Rule does not ship when repo_type is absent from its repo_types list."""
	rule = {'paths': ['devel/make_release.py'], 'repo_types': ['rust', 'swift']}
	result = repolib.model.shared_rule_ships_to(rule, 'my_rule', 'python', str(tmp_path))
	assert result is False


def test_unconditional_rule_ships(tmp_path: pathlib.Path) -> None:
	"""Rule without 'when' ships unconditionally to every listed repo_type."""
	# No 'when' key: the rule applies to all repo_types listed, no marker check.
	rule = {'paths': ['docs/RELEASE_HISTORY.md'], 'repo_types': ['python', 'typescript']}
	result = repolib.model.shared_rule_ships_to(rule, 'no_cond', 'python', str(tmp_path))
	assert result is True


#============================================
# shared_rule_ships_to: lacks_file condition
#============================================

def test_lacks_file_ships_when_marker_absent(tmp_path: pathlib.Path) -> None:
	"""lacks_file rule ships when the marker file is absent at the consumer."""
	# pyproject.toml does NOT exist in tmp_path -> rule should ship.
	rule = {
		'paths': ['devel/make_release.py'],
		'repo_types': ['python', 'rust', 'other'],
		'when': 'lacks_file',
		'path': 'pyproject.toml',
	}
	result = repolib.model.shared_rule_ships_to(rule, 'source_release', 'rust', str(tmp_path))
	assert result is True


def test_lacks_file_skips_when_marker_present(tmp_path: pathlib.Path) -> None:
	"""lacks_file rule does NOT ship when the marker file exists at the consumer."""
	# Write the marker so it IS present.
	(tmp_path / 'pyproject.toml').write_text('[project]\nname = "dummy"\n')
	rule = {
		'paths': ['devel/make_release.py'],
		'repo_types': ['python', 'rust', 'other'],
		'when': 'lacks_file',
		'path': 'pyproject.toml',
	}
	result = repolib.model.shared_rule_ships_to(rule, 'source_release', 'python', str(tmp_path))
	assert result is False


def test_lacks_file_skips_unlisted_type_even_without_marker(tmp_path: pathlib.Path) -> None:
	"""lacks_file rule skips a type not in repo_types regardless of marker absence."""
	# typescript is not in repo_types, so the rule never ships to it.
	rule = {
		'paths': ['devel/make_release.py'],
		'repo_types': ['rust', 'other'],
		'when': 'lacks_file',
		'path': 'pyproject.toml',
	}
	result = repolib.model.shared_rule_ships_to(rule, 'source_release', 'typescript', str(tmp_path))
	assert result is False


#============================================
# shared_rule_ships_to: unknown verb raises
#============================================

def test_unknown_verb_raises_for_listed_type(tmp_path: pathlib.Path) -> None:
	"""Unknown 'when' verb raises ValueError when repo_type is in the rule's list.

	The repo_type gate passes first; the bad verb is only evaluated for listed
	types.  Target a type the rule lists to reach the verb check.
	"""
	rule = {
		'paths': ['devel/make_release.py'],
		'repo_types': ['python', 'rust'],
		'when': 'bad_verb',
		'path': 'pyproject.toml',
	}
	with pytest.raises(ValueError, match='bad_verb'):
		repolib.model.shared_rule_ships_to(rule, 'broken_rule', 'python', str(tmp_path))


def test_unknown_verb_returns_false_for_unlisted_type(tmp_path: pathlib.Path) -> None:
	"""Unknown 'when' verb does not raise when repo_type is not in the rule's list.

	The repo_type gate returns False before the verb check is reached, so a
	bad verb in a rule that does not target this type is silently ignored.
	"""
	rule = {
		'paths': ['devel/make_release.py'],
		'repo_types': ['rust'],
		'when': 'bad_verb',
		'path': 'pyproject.toml',
	}
	# python is not in repo_types -> False with no exception
	result = repolib.model.shared_rule_ships_to(rule, 'broken_rule', 'python', str(tmp_path))
	assert result is False


#============================================
# shared_path_ships: aggregate any-rule logic
#============================================

def test_shared_path_ships_when_rule_matches(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""shared_path_ships returns True when a matching rule names the file."""
	# Inject a synthetic rule covering 'devel/make_release.py' for rust.
	fake_overlays = {
		'release': {
			'paths': ['devel/make_release.py'],
			'repo_types': ['rust', 'swift', 'other'],
		},
	}
	monkeypatch.setattr(repolib.model, 'SHARED_OVERLAYS', fake_overlays)
	result = repolib.model.shared_path_ships('devel/make_release.py', 'rust', str(tmp_path))
	assert result is True


def test_shared_path_ships_skips_unmatched_type(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""shared_path_ships returns False when no rule ships the file to this type."""
	# Rule exists but only for rust; python must not receive the file.
	fake_overlays = {
		'release': {
			'paths': ['devel/make_release.py'],
			'repo_types': ['rust'],
		},
	}
	monkeypatch.setattr(repolib.model, 'SHARED_OVERLAYS', fake_overlays)
	result = repolib.model.shared_path_ships('devel/make_release.py', 'python', str(tmp_path))
	assert result is False


def test_shared_path_ships_skips_unnamed_file(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""shared_path_ships returns False when no rule names the requested file."""
	# Only 'docs/NEWS.md' is covered; asking about 'devel/make_release.py' returns False.
	fake_overlays = {
		'release': {
			'paths': ['docs/NEWS.md'],
			'repo_types': ['python', 'rust'],
		},
	}
	monkeypatch.setattr(repolib.model, 'SHARED_OVERLAYS', fake_overlays)
	result = repolib.model.shared_path_ships('devel/make_release.py', 'python', str(tmp_path))
	assert result is False


#============================================
# all_shared_overlay_paths: union coverage set
#============================================

def test_all_shared_overlay_paths_collects_union(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""all_shared_overlay_paths returns the union of every rule's paths list."""
	fake_overlays = {
		'rule_a': {'paths': ['docs/NEWS.md', 'devel/make_release.py'], 'repo_types': ['rust']},
		'rule_b': {'paths': ['docs/RELEASE_HISTORY.md'], 'repo_types': ['python']},
	}
	monkeypatch.setattr(repolib.model, 'SHARED_OVERLAYS', fake_overlays)
	result = repolib.model.all_shared_overlay_paths()
	# Every path from both rules must appear in the union; behavioral property only.
	assert 'docs/NEWS.md' in result
	assert 'devel/make_release.py' in result
	assert 'docs/RELEASE_HISTORY.md' in result


def test_all_shared_overlay_paths_empty_when_no_rules(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""all_shared_overlay_paths returns an empty set when SHARED_OVERLAYS is empty."""
	monkeypatch.setattr(repolib.model, 'SHARED_OVERLAYS', {})
	result = repolib.model.all_shared_overlay_paths()
	assert result == set()


#============================================
# Coverage guard (disk -> manifest)
# An uncovered templates/shared/ file must raise RuntimeError.
#============================================

def test_coverage_guard_raises_on_uncovered_shared_file(tmp_path: pathlib.Path) -> None:
	"""compute_propagation_plan raises RuntimeError on an uncovered templates/shared/ file.

	No registered rule names orphan.txt, so the file placed under a tmp
	templates/shared/ tree is uncovered.  The RuntimeError protects against orphaned
	shared files that route nowhere.
	"""
	# Build a minimal fake template tree with one uncovered shared file.
	shared_dir = tmp_path / 'templates' / 'shared'
	shared_dir.mkdir(parents=True)
	(shared_dir / 'orphan.txt').write_text('uncovered')
	# No rule names orphan.txt, so it is uncovered regardless of the registered
	# source_release rule; the walk must raise.
	with pytest.raises(RuntimeError, match='shared overlay leak'):
		repolib.files.compute_propagation_plan(str(tmp_path), 'rust')


#============================================
# Live-manifest routing: source_release rule
# Verifies the registered source_release rule ships/skips correctly.
# Uses the live SHARED_OVERLAYS so these tests exercise real registered
# config, not synthetic monkeypatched rules.
#============================================

# Derive the routing constants from the live manifest so the tests stay in sync
# with repolib.model.SHARED_OVERLAYS instead of hardcoding the registered values.
# source_release ships make_release.py UNCONDITIONALLY to its listed types
# (python incl. PyPI, rust, swift, other); typescript is excluded because those
# repos are GitHub Pages based and do not cut releases.
_SOURCE_RELEASE_RULE = repolib.model.SHARED_OVERLAYS['source_release']
SOURCE_RELEASE_PATHS = _SOURCE_RELEASE_RULE['paths']

# Repo types that should receive the source_release files.
SOURCE_RELEASE_RECIPIENT_TYPES = _SOURCE_RELEASE_RULE['repo_types']


@pytest.mark.parametrize('repo_type', SOURCE_RELEASE_RECIPIENT_TYPES)
@pytest.mark.parametrize('shared_path', SOURCE_RELEASE_PATHS)
def test_source_release_ships_to_recipient_types(
	repo_type: str,
	shared_path: str,
	tmp_path: pathlib.Path,
) -> None:
	"""source_release files ship to every listed type (unconditionally)."""
	result = repolib.model.shared_path_ships(shared_path, repo_type, str(tmp_path))
	assert result is True


@pytest.mark.parametrize('shared_path', SOURCE_RELEASE_PATHS)
def test_source_release_ships_to_python_with_pyproject(
	shared_path: str,
	tmp_path: pathlib.Path,
) -> None:
	"""source_release ships to python repos even when they carry pyproject.toml (PyPI)."""
	# A PyPI python repo gets make_release.py (source snapshot) alongside its
	# _pypi submit_to_pypi.py; the rule no longer gates on pyproject.toml.
	(tmp_path / 'pyproject.toml').write_text('[project]\nname = "dummy"\n')
	result = repolib.model.shared_path_ships(shared_path, 'python', str(tmp_path))
	assert result is True


@pytest.mark.parametrize('shared_path', SOURCE_RELEASE_PATHS)
def test_source_release_skips_typescript(
	shared_path: str,
	tmp_path: pathlib.Path,
) -> None:
	"""source_release files do NOT ship to typescript repos (not in the rule's repo_types)."""
	# typescript is not listed in the source_release rule's repo_types, and its
	# inherited chain (typescript, website) does not intersect the rule's types.
	result = repolib.model.shared_path_ships(shared_path, 'typescript', str(tmp_path))
	assert result is False


def test_source_release_targets_the_base_set() -> None:
	"""source_release.repo_types is exactly the base set [scripted, compiled, other].

	Base-targeted routing (decision 6 of the inheritance-DAG plan): any future
	scripted or compiled language inherits release tooling with no further
	manifest edit, while the website family (website, typescript) routes
	releases elsewhere by staying outside this set.
	"""
	assert set(SOURCE_RELEASE_RECIPIENT_TYPES) == {'scripted', 'compiled', 'other'}


#============================================
# Ancestor inheritance: typescript receives templates/website/ content
#============================================

def test_typescript_receives_playwright_style_through_website_overlay() -> None:
	"""typescript inherits docs/PLAYWRIGHT_TEST_STYLE.md from the website overlay.

	The file lives under templates/website/docs/ (moved there from
	templates/shared/); typescript ships it only because effective_type_chain
	('typescript') includes 'website', not through any shared_overlays rule.
	"""
	plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, 'typescript')
	assert 'docs/PLAYWRIGHT_TEST_STYLE.md' in plan['overwrite_files']
