"""Data models and propagation spec/plan logic."""

# Standard Library
import os
from dataclasses import dataclass

#============================================
# Orchestration context dataclass
#============================================

@dataclass
class PropagateContext:
	"""
	Context object passed to orchestration helpers.
	Mirrors all args fields that downstream helpers need. Treat as read-only after construction.
	"""
	source_dir: str
	template_root: str
	repo_name: str | None
	dry_run: bool
	initial_setup: bool
	# auto_discover: source-template test discovery for ONE repo. When True, the
	# source template's tests/ is scanned for test_*.py/test_*.mjs files absent
	# from the static spec, and those are added to the files copied INTO the
	# target repo. It never walks other repos; the meaning is single-repo only.
	auto_discover: bool
	write_marker: bool

# Language type constants
LANG_PYTHON = 'python'
LANG_TYPESCRIPT = 'typescript'
LANG_RUST = 'rust'
LANG_SWIFT = 'swift'
LANG_OTHER = 'other'
LANG_ALL = 'all'
LANG_UNKNOWN = 'unknown'

#============================================
# Propagation manifests: loaded from meta/propagation/manifests.yaml
#============================================

# All propagation manifests live in meta/propagation/manifests.yaml, the single
# source of propagation config. They are loaded once at import and assigned to the
# module-level names below so importers (from repolib.model import ROUTING_OVERRIDES)
# and dir() keep working. The loaded values carry the same Python types the literals
# previously had: frozenset for set manifests, tuple for META_TEST_PREFIXES, and dict
# for ROUTING_OVERRIDES and CONDITIONAL_OVERLAYS (with frozenset exclude_repos).
#
# Manifest meanings:
#   ROUTING_OVERRIDES: per-file exceptions (only exclude_repos remains).
#   CONDITIONAL_OVERLAYS: repo_type -> overlay_name -> {when, path, description}.
#   SHARED_OVERLAYS: rule_name -> {paths, repo_types, optional when, optional path};
#     routes templates/shared/ files to a chosen SET of repo types.
#   ROOT_PROPAGATE_ALLOWLIST: root files that MAY ship; UNIVERSAL_NOEXIST refines how.
#   UNIVERSAL_NOEXIST: files that ship only when absent at the consumer.
#   MERGE_FILES: files routed to the set-union @-import merge bucket.
#   META_FILES / META_DIRS: files and dirs that NEVER ship (template-meta).
#   META_FILE_PATTERNS: glob patterns that never ship, e.g. changelog archives.
#   SKIP_WALK_DIRS: dirs skipped during os.walk of the source template.
#   AUTO_DISCOVER_DOCS_EXCLUDE: docs excluded from auto-discovery.
#   META_TEST_PREFIXES: template-meta test filename prefixes.
#   DEFAULT_REPO_SKIP_NAMES: default skip list for repo discovery.
#   KNOWN_REPO_TYPES: recognized consumer marker tokens (from the ordered
#     known_repo_types manifest); REPO_TYPE_ORDER keeps display order, the
#     KNOWN_REPO_TYPES frozenset is for membership only.

#============================================
# Manifest loading
#============================================

def _load_propagation_manifests() -> dict:
	"""
	Resolve the template root and load all propagation manifests once at import.

	repolib.repo is imported lazily here to avoid an import cycle: repolib.repo
	imports repolib.model at its module top, so importing repolib.repo at the top
	of this module would cycle. resolve_source_dir only walks the filesystem and
	does not need repolib.model state, so a deferred import is safe.

	Returns:
		dict: Typed propagation manifests from meta/propagation/manifests.yaml.
	"""
	# Lazy imports break the repolib.repo <-> repolib.model import cycle; both
	# modules are only needed once at load time, not at module scope.
	import repolib.repo
	import repolib.manifests
	# Anchor the template root on the running source checkout (repolib package).
	template_root = repolib.repo.resolve_source_dir(None)
	return repolib.manifests.load_manifests(template_root)


# Load once at import and bind each manifest to its module-level public name.
_MANIFESTS = _load_propagation_manifests()
ROUTING_OVERRIDES = _MANIFESTS['routing_overrides']
CONDITIONAL_OVERLAYS = _MANIFESTS['conditional_overlays']
SHARED_OVERLAYS = _MANIFESTS['shared_overlays']
ROOT_PROPAGATE_ALLOWLIST = _MANIFESTS['root_propagate_allowlist']
UNIVERSAL_NOEXIST = _MANIFESTS['universal_noexist']
MERGE_FILES = _MANIFESTS['merge_files']
META_FILES = _MANIFESTS['meta_files']
# Glob patterns (template-root-relative) that never ship, e.g. changelog archives.
META_FILE_PATTERNS = _MANIFESTS['meta_file_patterns']
META_DIRS = _MANIFESTS['meta_dirs']
SKIP_WALK_DIRS = _MANIFESTS['skip_walk_dirs']
AUTO_DISCOVER_DOCS_EXCLUDE = _MANIFESTS['auto_discover_docs_exclude']
META_TEST_PREFIXES = _MANIFESTS['meta_test_prefixes']
DEFAULT_REPO_SKIP_NAMES = _MANIFESTS['default_repo_skip_names']
# Recognized consumer marker tokens. REPO_TYPE_ORDER is the ordered tuple
# (canonical display order for prompts and docs); KNOWN_REPO_TYPES is the derived
# frozenset used only for membership checks. Internal routing pseudo-types
# (universal, unknown) are NOT included here; they are added at their own guard
# in repolib.files so they never leak into UI, docs, or marker validation.
REPO_TYPE_ORDER = _MANIFESTS['known_repo_types']
KNOWN_REPO_TYPES = frozenset(REPO_TYPE_ORDER)
# Child -> parent single-token inheritance map (repo_type_inherits manifest).
# REPO_TYPE_PARENTS.get(child) returns the parent token, or None for a root type
# (scripted, website, compiled, other have no entry). effective_type_chain()
# walks this map so a concrete type inherits its ancestors' overlays.
REPO_TYPE_PARENTS = _MANIFESTS['repo_type_inherits']


#============================================
# REPO_TYPE inheritance chain
#============================================

def ancestors(repo_type: str) -> list[str]:
	"""
	Return the ancestor chain for a repo_type, nearest parent first.

	Walks REPO_TYPE_PARENTS from repo_type upward. A root type (no parent entry)
	returns an empty list. A cycle in the inheritance map raises loudly instead of
	looping forever; the loader also validates acyclicity, so this is a second guard.

	Args:
		repo_type (str): Consumer repository type token.

	Returns:
		list[str]: Ancestor tokens from nearest parent to furthest root.

	Raises:
		ValueError: The inheritance map contains a cycle reachable from repo_type.
	"""
	# Track visited nodes so a malformed cyclic map raises instead of looping.
	chain: list[str] = []
	seen = {repo_type}
	parent = REPO_TYPE_PARENTS.get(repo_type)
	while parent is not None:
		if parent in seen:
			raise ValueError(f"cycle in repo_type_inherits at {parent!r}")
		chain.append(parent)
		seen.add(parent)
		parent = REPO_TYPE_PARENTS.get(parent)
	return chain


def expand_marker_types(marker: str) -> list[str]:
	"""
	Expand a REPO_TYPE marker string into its ordered list of declared types.

	A marker may declare several types, comma separated (e.g. 'python,rust'), and the
	repo receives the additive union of every declared type's overlays. The alias 'all'
	expands in place to every type in REPO_TYPE_ORDER except 'all' itself, so 'all'
	rides the same multi-type path as any other marker rather than a special case.

	Pure function: no warnings, no filesystem access, no fallback. It is safe to call
	repeatedly from routing hot paths. Unrecognized types pass through untouched;
	warning and degrade belong to validate_marker, not here.

	Args:
		marker (str): Raw REPO_TYPE marker text, one or more comma-separated types.

	Returns:
		list[str]: Declared types in declaration order, deduplicated by first
			occurrence. An empty or all-empty marker returns [].
	"""
	# Collect declared types in declaration order, deduping by first occurrence.
	declared_types: list[str] = []
	for piece in marker.split(','):
		# Tolerate surrounding whitespace and stray spaces after commas.
		declared_type = piece.strip()
		if not declared_type:
			continue
		# 'all' expands in place to every concrete type, preserving marker order.
		if declared_type == LANG_ALL:
			for known_type in REPO_TYPE_ORDER:
				if known_type == LANG_ALL:
					continue
				if known_type not in declared_types:
					declared_types.append(known_type)
			continue
		if declared_type not in declared_types:
			declared_types.append(declared_type)
	return declared_types


def partition_known_types(marker: str) -> tuple[list[str], list[str]]:
	"""
	Split a marker's declared types into the recognized and unrecognized halves.

	Membership in KNOWN_REPO_TYPES is the single validity test for a declared type,
	and this is the one place that applies it. Callers that need only one half take
	only that half, so the rule lives here rather than being restated at each site
	where a marker is validated or gated.

	The internal pseudo-types ('universal', LANG_UNKNOWN) are deliberately absent
	from KNOWN_REPO_TYPES so they never surface in prompts, docs, or marker
	validation. They are therefore reported as unrecognized here, and the callers
	that accept them screen for them before calling.

	Args:
		marker (str): Raw REPO_TYPE marker text, one or more comma-separated types.

	Returns:
		tuple[list[str], list[str]]: (known_types, unknown_types), each in
			declaration order.
	"""
	# One pass over the expansion; membership is the only validity test.
	known_types: list[str] = []
	unknown_types: list[str] = []
	for declared_type in expand_marker_types(marker):
		if declared_type in KNOWN_REPO_TYPES:
			known_types.append(declared_type)
		else:
			unknown_types.append(declared_type)
	return known_types, unknown_types


def effective_type_chain(marker: str) -> list[str]:
	"""
	Return every declared type followed by its ancestors, nearest-first.

	This is the one canonical expansion every routing path consumes, so overlay
	selection, source resolution, and shared-overlay routing all agree on which
	base types a marker inherits. Each type from expand_marker_types contributes
	itself and then its ancestors, deduped nearest-first across the whole marker:
	'python,rust' yields ['python', 'scripted', 'rust', 'compiled'].

	A single-type marker returns exactly [repo_type, *ancestors(repo_type)],
	unchanged from the single-type behavior every existing caller relies on. An
	unrecognized type contributes only itself; since no templates/<unknown>/ root
	exists, it routes to nothing rather than misrouting.

	Args:
		marker (str): Consumer REPO_TYPE marker (one or more comma-separated types).

	Returns:
		list[str]: Declared types and their ancestors, nearest-first, deduplicated.
	"""
	# Union each declared type's own chain, keeping declaration order across types.
	chain: list[str] = []
	for declared_type in expand_marker_types(marker):
		for chain_type in [declared_type, *ancestors(declared_type)]:
			# Dedup while preserving nearest-first order.
			if chain_type not in chain:
				chain.append(chain_type)
	return chain


def validate_marker(marker: str, repo_label: str) -> str:
	"""
	Validate a REPO_TYPE marker and return the marker to route with.

	This is the validation layer, kept separate from the pure parsing in
	expand_marker_types so warnings fire once at the marker readers instead of once
	per resolver call. Unrecognized types are dropped with a single warning naming
	each, so 'python,pyhton' still routes as 'python': the valid half is preserved
	and the typo stays visible. Only when NO declared type is recognized does the
	marker degrade to 'other', so a bad marker still never aborts a batch run.

	Args:
		marker (str): Raw REPO_TYPE marker text as read from the consumer repo.
		repo_label (str): Repository name used in warning messages.

	Returns:
		str: Canonical comma-joined marker of the recognized types, or 'other' when
			the marker names no recognized type.
	"""
	# repolib.console is imported here, inside the function body, not at module
	# level. repolib.model loads its manifests at import time through
	# repolib.repo.resolve_source_dir, and repolib.repo imports repolib.model, so
	# this module keeps every repolib import function-scoped to stay clear of that
	# cycle. This import also binds the name `repolib` as a function-local, so it
	# must precede every repolib.* reference in this function.
	import repolib.console
	known_types, unknown_types = partition_known_types(marker)
	unknown_text = ', '.join(repr(declared_type) for declared_type in unknown_types)
	# No recognized type at all: degrade to other rather than routing nothing.
	if not known_types:
		# An empty, whitespace-only, or comma-only marker declares nothing at all,
		# so unknown_text would render blank; warn about the empty marker itself
		# instead of an empty list of types.
		if not unknown_types:
			repolib.console.log_action(
				"warn",
				f"repo={repo_label} REPO_TYPE marker is empty; treating as other"
			)
		else:
			repolib.console.log_action(
				"warn",
				f"repo={repo_label} REPO_TYPE type(s) {unknown_text} not recognized; treating as other"
			)
		return LANG_OTHER
	canonical_marker = ','.join(known_types)
	# Some declared types were bad: drop them, keep the valid half, name typos once.
	if unknown_types:
		repolib.console.log_action(
			"warn",
			f"repo={repo_label} REPO_TYPE type(s) {unknown_text} not recognized; "
			f"routing as {canonical_marker!r}"
		)
	return canonical_marker


#============================================
# Conditional-overlay selection
#============================================

def single_type_overlay_dirs(repo_type: str, repo_dir: str) -> list[str]:
	"""
	Overlay path segments for ONE repo_type (base folder plus conditional overlays).

	This is the pre-inheritance expansion: the base type folder first, then each
	conditional overlay whose marker file exists at the consumer. select_overlay_dirs
	runs this over the full effective_type_chain and unions the results.

	Args:
		repo_type (str): A single repository type token (no inheritance applied).
		repo_dir (str): Consumer repository directory to test marker files against.

	Returns:
		list[str]: Ordered overlay path segments for this one type, e.g.
			['python', 'python/_pypi'] when pyproject.toml exists at repo_dir.
	"""
	# Base repo_type overlay always applies and comes first.
	overlay_dirs = [repo_type]
	# Conditional overlays for this repo_type, if any are configured.
	overlay_config = CONDITIONAL_OVERLAYS.get(repo_type, {})
	for overlay_name, condition in overlay_config.items():
		when_verb = condition['when']
		# Only the has_file verb is understood; anything else is a config error.
		if when_verb != 'has_file':
			raise ValueError(
				f"unknown conditional-overlay 'when' verb {when_verb!r} for "
				f"overlay {overlay_name!r} (repo_type {repo_type!r}); "
				"only 'has_file' is supported"
			)
		# Marker file path is relative to the consumer repo root.
		marker_path = os.path.join(repo_dir, condition['path'])
		if os.path.isfile(marker_path):
			# Overlay segment is the underscore folder under templates/<repo_type>/.
			overlay_dirs.append(f"{repo_type}/{overlay_name}")
	return overlay_dirs


def select_overlay_dirs(repo_type: str, repo_dir: str) -> list[str]:
	"""
	Select the ordered template overlay folders that apply to a consumer repo.

	Runs each member of effective_type_chain(repo_type) through the base-plus-
	conditional expansion (single_type_overlay_dirs) and unions the segments
	nearest-first: the concrete type's own overlays come first, then each ancestor
	base's overlays. So a typescript repo picks up its own overlay AND the website
	base overlay it inherits.

	Args:
		repo_type (str): Consumer repository marker: one type token or a comma-separated
			list (python, typescript, rust, swift, other, all, ...).
		repo_dir (str): Consumer repository directory to test marker files against.

	Returns:
		list[str]: Ordered, deduplicated overlay path segments under templates/,
			nearest-first across the inheritance chain.
	"""
	# Union the per-type segments across the inheritance chain, nearest-first.
	overlay_dirs: list[str] = []
	for chain_type in effective_type_chain(repo_type):
		for segment in single_type_overlay_dirs(chain_type, repo_dir):
			# Dedup while preserving nearest-first order.
			if segment not in overlay_dirs:
				overlay_dirs.append(segment)
	return overlay_dirs


def single_type_overlay_roots(template_root: str, repo_type: str) -> list[str]:
	"""
	Candidate overlay root directories for ONE repo_type (no inheritance applied).

	Returns the base templates/<repo_type>/ root plus every configured conditional
	overlay root (templates/<repo_type>/_<name>/). overlay_roots_for_type runs this
	over the full effective_type_chain and unions the results.

	Args:
		template_root (str): Template root directory.
		repo_type (str): A single repository type token.

	Returns:
		list[str]: Absolute candidate overlay root directories for this one type.
	"""
	# Base typed root always comes first.
	roots = [os.path.join(template_root, 'templates', repo_type)]
	# Append each configured conditional overlay root for this repo_type.
	overlay_config = CONDITIONAL_OVERLAYS.get(repo_type, {})
	for overlay_name in overlay_config:
		roots.append(os.path.join(template_root, 'templates', repo_type, overlay_name))
	return roots


def overlay_roots_for_type(template_root: str, repo_type: str) -> list[str]:
	"""
	Yield the candidate overlay root directories for a repo_type and its ancestors.

	Runs each member of effective_type_chain(repo_type) through the single-type
	root expansion and unions the roots nearest-first, so a source resolver for a
	concrete type also searches the roots of every base it inherits (e.g. a
	typescript lookup searches templates/typescript/ and templates/website/). The
	universal template root is handled separately by each resolver.

	Args:
		template_root (str): Template root directory.
		repo_type (str): Repository marker: one type token or a comma-separated
			list (python, typescript, rust, swift, other, all, ...).

	Returns:
		list[str]: Ordered, deduplicated candidate overlay root directories under
			templates/, nearest-first across the inheritance chain.
	"""
	# Union the per-type roots across the inheritance chain, nearest-first.
	roots: list[str] = []
	for chain_type in effective_type_chain(repo_type):
		for root in single_type_overlay_roots(template_root, chain_type):
			# Dedup while preserving nearest-first order.
			if root not in roots:
				roots.append(root)
	return roots


#============================================
# Shared-overlay selection
#============================================

def all_shared_overlay_paths() -> set[str]:
	"""
	Collect every consumer-relative path named by any shared_overlays rule.

	The shared walk uses this union as a coverage guard: a file under
	templates/shared/ that no rule names routes nowhere, which is a config bug.

	Returns:
		set[str]: Consumer-relative paths (e.g. 'devel/make_release.py') that at
			least one shared_overlays rule lists in its 'paths'.
	"""
	# Union the 'paths' list of every configured rule.
	covered_paths = set()
	for rule in SHARED_OVERLAYS.values():
		for path in rule['paths']:
			covered_paths.add(path)
	return covered_paths


def shared_rule_ships_to(rule: dict, rule_name: str, repo_type: str, repo_dir: str) -> bool:
	"""
	Decide whether one shared_overlays rule ships to a given consumer.

	The repo_type must be listed in the rule. The optional 'when' verb then gates
	on a marker file at the consumer: 'lacks_file' ships only when the marker is
	ABSENT (the mirror of the conditional-overlay 'has_file' verb). A rule without
	'when' ships unconditionally to every listed repo_type.

	Args:
		rule (dict): A shared_overlays rule ({paths, repo_types, optional when/path}).
		rule_name (str): Rule name, used only for a clear error message.
		repo_type (str): Consumer repository marker: one type token or a comma-separated
			list (python, typescript, rust, swift, other, all, ...).
		repo_dir (str): Consumer repository directory to test the marker file against.

	Returns:
		bool: True when this rule ships its paths to this consumer.

	Raises:
		ValueError: The rule carries a 'when' verb other than 'lacks_file'.
	"""
	# repo_type gate: a rule applies when the consumer's inheritance chain
	# intersects the rule's listed types, so a rule targeting a base (e.g.
	# scripted) reaches every descendant (python) that inherits it.
	if not (set(effective_type_chain(repo_type)) & set(rule['repo_types'])):
		return False
	# 'when' is genuinely optional: a rule without it ships unconditionally.
	when_verb = rule.get('when')
	if when_verb is None:
		return True
	# Only the lacks_file verb is understood; anything else is a config error.
	if when_verb != 'lacks_file':
		raise ValueError(
			f"unknown shared-overlay 'when' verb {when_verb!r} for rule "
			f"{rule_name!r}; only 'lacks_file' is supported"
		)
	# Marker file path is relative to the consumer repo root.
	marker_path = os.path.join(repo_dir, rule['path'])
	# lacks_file: ship only when the marker file is ABSENT at the consumer.
	return not os.path.isfile(marker_path)


def shared_path_ships(file_rel: str, repo_type: str, repo_dir: str) -> bool:
	"""
	Decide whether a templates/shared/ file ships to a given consumer.

	A shared file ships when at least one rule that NAMES it in its 'paths' applies
	to this consumer (repo_type listed and the optional lacks_file condition holds).

	Args:
		file_rel (str): Consumer-relative path of the shared file (e.g.
			'devel/make_release.py'), relative to templates/shared/.
		repo_type (str): Consumer repository marker: one type token or a comma-separated
			list (python, typescript, rust, swift, other, all, ...).
		repo_dir (str): Consumer repository directory to test marker files against.

	Returns:
		bool: True when some applicable rule ships this file to this consumer.
	"""
	# First applicable rule that names this file wins; the union semantics mean any
	# matching rule is enough to ship the file.
	for rule_name, rule in SHARED_OVERLAYS.items():
		if file_rel not in rule['paths']:
			continue
		if shared_rule_ships_to(rule, rule_name, repo_type, repo_dir):
			return True
	return False


#============================================
# Source/target path resolution
#============================================

def source_path_for_bucket(template_root: str, bucket: str, file_rel: str, repo_type: str = 'universal') -> str:
	"""
	Resolve canonical source path for a file in a bucket.
	Handles universal files at template root and typed files under templates/<repo_type>/.
	Typed lookups also search the repo_type's conditional overlays
	(templates/<repo_type>/_<name>/), so an overlay file such as
	templates/python/_pypi/devel/submit_to_pypi.py resolves for python repos.
	For noexist_files, looks under templates/<repo_type>[/_overlay]/noexist/ as well as root.
	"""
	# Reuse the non-raising resolver, then fail loud if nothing matched.
	source = find_source_for_bucket(template_root, bucket, file_rel, repo_type)
	if source is None:
		raise FileNotFoundError(f"canonical source missing for {bucket} entry {file_rel!r}")
	return source


def find_source_for_bucket(template_root: str, bucket: str, file_rel: str, repo_type: str = 'universal') -> str | None:
	"""
	Resolve canonical source path for a file in a bucket, or return None if not found.

	Non-raising variant of source_path_for_bucket() for cleaner predicate-based control flow.

	Args:
		template_root (str): Template root directory.
		bucket (str): Bucket name (overwrite_files, noexist_files, devel_files, test_files).
		file_rel (str): Relative path of the file.
		repo_type (str): Repository marker: one type token or a comma-separated list
			(python, typescript, rust, swift, other, all, ...). Defaults to 'universal'.

	Returns:
		str | None: Canonical source path if found, None otherwise.

	Typed lookups search every declared type's base root (templates/<type>/) AND each
	of its conditional overlay roots (templates/<type>/_<name>/) via
	overlay_roots_for_type(), so overlay-only source files resolve correctly.

	A multi-type marker needs no special case here: overlay_roots_for_type() returns
	the roots of every declared type and its ancestors, nearest-first, so the first
	declared type wins a same-path collision. 'all' is just the marker that declares
	every concrete type, so a typescript-only source still resolves under its own
	family root.
	"""
	# Normalize repo_type alias
	if repo_type == 'universal':
		repo_type = 'python'

	# Typed candidate roots: base templates/<repo_type>/ plus conditional overlays.
	typed_roots = overlay_roots_for_type(template_root, repo_type)

	# Determine candidate paths based on bucket.
	# Shared-overlay root: a templates/shared/ file's canonical source lives there.
	# Checked last in each branch so typed and universal sources shadow it (shared
	# files are the fallback canonical copy, not an override).
	shared_root = os.path.join(template_root, 'templates', 'shared')

	if bucket == 'devel_files':
		# Typed/overlay roots shadow the universal root, mirroring overwrite_files.
		# This ordering matters during single-repo reset: there template_root IS the
		# consumer repo, so a consumer-owned devel/<name> (e.g. a stale, tracked
		# devel/submit_to_pypi.py) sits at the universal devel/ location. Checking
		# the universal root first would resolve the propagation SOURCE to that
		# consumer file, making the copy a self no-op and leaving stale content in
		# place. An overlay-only file like _pypi/devel/submit_to_pypi.py must resolve
		# to its overlay source so the refresh/overwrite actually happens.
		# Typed/overlay roots: templates/<repo_type>[/_overlay]/devel/<name>
		for typed_root in typed_roots:
			candidate = os.path.join(typed_root, 'devel', file_rel)
			if os.path.isfile(candidate):
				return candidate
		# Universal devel files: template_root/devel/<name>
		candidate = os.path.join(template_root, 'devel', file_rel)
		if os.path.isfile(candidate):
			return candidate
		# Shared-overlay devel files: templates/shared/devel/<name>
		candidate = os.path.join(shared_root, 'devel', file_rel)
		if os.path.isfile(candidate):
			return candidate

	elif bucket == 'test_files':
		# test files: file_rel already includes tests/ prefix, so just join directly
		candidate = os.path.join(template_root, file_rel)
		if os.path.isfile(candidate):
			return candidate
		# Typed/overlay roots: templates/<repo_type>[/_overlay]/tests/<name>
		for typed_root in typed_roots:
			candidate = os.path.join(typed_root, file_rel)
			if os.path.isfile(candidate):
				return candidate
		# Shared-overlay test files: templates/shared/<file_rel>
		candidate = os.path.join(shared_root, file_rel)
		if os.path.isfile(candidate):
			return candidate

	elif bucket == 'noexist_files':
		# noexist files: could be at template root (universal) or under typed noexist dirs.
		# Typed/overlay roots shadow the universal root (rule 5: repo-specific wins),
		# mirroring overwrite_files and devel_files. A file present BOTH at root (e.g.
		# universal Brewfile) AND under templates/<type>/noexist/ (e.g. python's
		# python@3.12 Brewfile) resolves to the typed source for that type, while other
		# types fall through to the universal root version.
		# Typed/overlay noexist: templates/<repo_type>[/_overlay]/noexist/<path>
		for typed_root in typed_roots:
			candidate = os.path.join(typed_root, 'noexist', file_rel)
			if os.path.isfile(candidate):
				return candidate
		# Universal root: template_root/<path>
		candidate = os.path.join(template_root, file_rel)
		if os.path.isfile(candidate):
			return candidate
		# Shared-overlay noexist files: templates/shared/noexist/<file_rel>
		candidate = os.path.join(shared_root, 'noexist', file_rel)
		if os.path.isfile(candidate):
			return candidate

	else:
		# overwrite_files (or default): typed under templates/<type>[/_overlay]/ shadows root
		for typed_root in typed_roots:
			candidate = os.path.join(typed_root, file_rel)
			if os.path.isfile(candidate):
				return candidate
		candidate = os.path.join(template_root, file_rel)
		if os.path.isfile(candidate):
			return candidate
		# Shared-overlay overwrite files: templates/shared/<file_rel>
		candidate = os.path.join(shared_root, file_rel)
		if os.path.isfile(candidate):
			return candidate

	return None


def override_key_source(template_root: str, file_rel: str) -> str | None:
	"""
	Resolve a ROUTING_OVERRIDES key to its existing template source path.

	An override key is a repo-root-relative path (e.g. 'docs/CLAUDE_HOOK_USAGE_GUIDE.md').
	Its source may live in any bucket layout, so this probes find_source_for_bucket()
	across the relevant buckets (overwrite, noexist, devel, test) and returns the first
	existing source path. The probe uses the python repo_type so that typed and
	conditional-overlay roots are searched in addition to the universal root.

	Args:
		template_root (str): Template root directory.
		file_rel (str): ROUTING_OVERRIDES key (repo-root-relative path).

	Returns:
		str | None: Existing template source path, or None if it resolves nowhere.
	"""
	# devel keys carry a 'devel/' prefix at the override layer, but find_source_for_bucket
	# expects the bare name under devel_files; strip the prefix for that probe.
	devel_rel = file_rel
	if devel_rel.startswith('devel' + os.sep):
		devel_rel = devel_rel[len('devel' + os.sep):]
	# Probe each plausible bucket; first hit wins. python repo_type pulls in typed
	# and conditional-overlay roots via overlay_roots_for_type().
	for bucket, probe_rel in (
		('overwrite_files', file_rel),
		('noexist_files', file_rel),
		('devel_files', devel_rel),
		('test_files', file_rel),
	):
		source = find_source_for_bucket(template_root, bucket, probe_rel, LANG_PYTHON)
		if source is not None:
			return source
	return None


def target_path_for_bucket(repo_dir: str, bucket: str, file_rel: str) -> str:
	"""
	Resolve target path at consumer repo.
	Note: for test_files, file_rel includes 'tests/' prefix (e.g., 'tests/test_foo.py').
	For devel_files, file_rel is a bare name (e.g., 'submit_to_pypi.py').
	"""
	if bucket == 'devel_files':
		return os.path.join(repo_dir, 'devel', file_rel)
	# test_files: file_rel already includes 'tests/' prefix; other buckets are repo-root-relative.
	return os.path.join(repo_dir, file_rel)


def format_path_pair(source_file: str, dest_file: str, repo_dir: str, context: 'PropagateContext') -> str:
	"""
	Format a source-dest file pair for logging using repo-relative paths.

	  - If src relative path == dst relative path, show only the dst relative path
	  - Otherwise, show both as "src_rel -> dst_rel"

	Args:
		source_file (str): Absolute source file path.
		dest_file (str): Absolute destination file path.
		repo_dir (str): Repository directory path.
		context (PropagateContext): Context with source_dir.

	Returns:
		str: Formatted path string for logging.
	"""
	# Compute relative paths
	src_relative = os.path.relpath(source_file, context.source_dir)
	dst_relative = os.path.relpath(dest_file, repo_dir)

	# If relative paths are the same, show only one
	if src_relative == dst_relative:
		return dst_relative

	# Otherwise show both
	return f"{src_relative} -> {dst_relative}"
