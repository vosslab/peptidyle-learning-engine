#!/usr/bin/env python3
"""
reset_repo.py - bootstrap a fresh clone of starter-repo-template.

Interactive bootstrap tool. Prompts for project type, SPDX licenses, PyPI
publishing (python only), staging, and commit. Writes the REPO_TYPE marker,
installs selected LICENSE files, optionally seeds pyproject.toml, calls
repolib directly to lay down type-dispatched files in bootstrap mode,
truncates README + CHANGELOG, and removes itself. Answers come from either an
interactive interview or a json config (--config); the only other CLI flag is
--dry-run, which previews actions without changing files.
"""

# Standard Library
import os
import sys
import glob
import json
import argparse
import datetime
import tempfile
import subprocess
import dataclasses

# local repo modules
import repolib.repo
import repolib.model
import repolib.console
import repolib.process

# Try to import detect_repo_type from tools/; if not available, prediction is skipped.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), 'tools'))
try:
	import detect_repo_type
except ImportError:
	detect_repo_type = None

CODE_LICENSES = ["MIT", "Apache-2.0", "LGPL-3.0", "GPL-3.0", "AGPL-3.0", "MPL-2.0"]
DOCS_LICENSES = ["CC-BY-4.0", "CC-BY-SA-4.0", "none"]

CODE_ALIASES = {
	"m": "MIT",
	"a": "Apache-2.0",
	"l": "LGPL-3.0",
	"g": "GPL-3.0",
	"ag": "AGPL-3.0",
	"mp": "MPL-2.0",
}

DOCS_ALIASES = {
	"cb": "CC-BY-4.0",
	"cs": "CC-BY-SA-4.0",
	"n": "none",
}

#============================================
def resolve_license(user_input: str, canonical: list, aliases: dict, default: str | None = None) -> str:
	"""Resolve license token via alias or unique prefix."""
	token = user_input.strip().lower()
	if token == "":
		if default is None:
			raise ValueError("empty license input; no default available")
		return default
	if token in aliases:
		return aliases[token]
	matches = [name for name in canonical if name.lower().startswith(token)]
	if len(matches) == 1:
		return matches[0]
	raise ValueError(f"ambiguous or unknown license: {user_input!r}")


#============================================
def get_repo_root() -> str:
	"""Return the repository root path via git rev-parse.

	Fails with a clear message rather than an obscure subprocess traceback when
	run outside a git repository (or when git is not installed).

	Returns:
		str: Absolute path to the repository root.
	"""
	# Resolve the repo root via git; a non-repo cwd makes git exit non-zero.
	result = subprocess.run(
		["git", "rev-parse", "--show-toplevel"],
		capture_output=True,
		text=True,
		check=False,
	)
	repo_root = result.stdout.strip()
	if result.returncode != 0 or repo_root == "":
		sys.exit(
			"Error: reset_repo must run inside a git repository. "
			"Clone the template, then run reset from the clone root."
		)
	return repo_root


#============================================
def preflight_check(repo_root: str, code_license: str, docs_license: str) -> None:
	"""Verify that license files exist in LICENSES/ before proceeding."""
	code_path = os.path.join(repo_root, f"LICENSES/LICENSE.{code_license}.md")
	if not os.path.isfile(code_path):
		sys.exit(f"license file missing: {code_path}")
	if docs_license != "none":
		docs_path = os.path.join(repo_root, f"LICENSES/LICENSE.{docs_license}.md")
		if not os.path.isfile(docs_path):
			sys.exit(f"license file missing: {docs_path}")


#============================================
def parse_args() -> argparse.Namespace:
	"""Parse command-line arguments and return the populated namespace."""
	parser = argparse.ArgumentParser(
		description="Reset a cloned starter-repo-template to base configuration"
	)
	parser.add_argument(
		"--config",
		dest="config",
		default=None,
		help="Path to a json answers file (non-interactive mode)",
	)
	parser.add_argument(
		"--dry-run",
		dest="dry_run",
		action="store_true",
		help="Print actions without executing",
	)
	return parser.parse_args()


#============================================
# Module-level helpers (extracted from main)
#============================================

def dry_run_print(msg: str, dry_run: bool) -> None:
	"""Print DRY-RUN prefixed message if dry_run is True."""
	if dry_run:
		print(f"DRY-RUN: {msg}")


def write_marker(repo_root: str, project_type: str, dry_run: bool) -> int:
	"""Write REPO_TYPE marker atomically via temp + replace.

	The marker is written in canonical form: lowercase types, comma separated,
	no spaces, declaration order preserved, and "all" left as the literal "all"
	rather than expanded. Every caller supplies a project_type that already came
	through normalize_project_type, which produces exactly that form, so this
	writes the value verbatim instead of re-normalizing it here.
	"""
	marker_path = os.path.join(repo_root, "REPO_TYPE")
	content = f"{project_type}\n"
	if dry_run:
		escaped_content = content.replace('"', '\\"').replace('\n', '\\n')
		dry_run_print(f'write REPO_TYPE ("{escaped_content}")', dry_run)
	else:
		with tempfile.NamedTemporaryFile(
			mode="w", dir=repo_root, delete=False
		) as tmp:
			tmp.write(content)
			tmp_name = tmp.name
		os.replace(tmp_name, marker_path)
	return 1


def copy_license(
	repo_root: str, source_path: str, target_filename: str, dry_run: bool,
) -> int:
	"""Copy a license file to the repo root under the given target_filename.

	The caller chooses target_filename (the reset uses LICENSE.<spdx>.md). The
	source file is guaranteed to exist by preflight_check, and a failed read or
	write raises loudly on its own, so no post-copy verification gate is needed.

	Returns:
		int: 1 (one action taken or announced), for the main() action counter.
	"""
	target_path = os.path.join(repo_root, target_filename)
	if dry_run:
		dry_run_print(f"copy {source_path} -> {target_path}", dry_run)
	else:
		with open(source_path, "r", encoding="utf-8") as src:
			content = src.read()
		with open(target_path, "w", encoding="utf-8") as dst:
			dst.write(content)
	return 1


def path_has_tracked_entry(path: str, repo_root: str) -> bool:
	"""Return True when git tracks path (a file) or any file under it (a directory).

	`git rm` on an untracked pathspec exits 128. Checking first lets git_rm and
	git_rm_recursive skip an already-removed path instead of aborting the reset
	mid-run -- the failure mode when reset is re-run on a partially reset repo.

	Args:
		path (str): Repo-relative file or directory pathspec.
		repo_root (str): Repository root the pathspec is anchored to.

	Returns:
		bool: True if at least one tracked entry matches path.
	"""
	# git ls-files exits 0 with empty output when nothing matches; "--" keeps a
	# path that looks like an option from being misread as one.
	result = subprocess.run(
		["git", "ls-files", "--", path],
		check=True, capture_output=True, text=True, cwd=repo_root,
	)
	return bool(result.stdout.strip())


def git_rm(path: str, repo_root: str, dry_run: bool) -> int:
	"""Remove a tracked file via git rm, anchored at repo_root.

	Idempotent: an untracked or already-removed path is skipped, not an error, so
	a reset never aborts mid-run on a path that is simply already gone.
	"""
	if not path_has_tracked_entry(path, repo_root):
		print(f"{path} not tracked -- skipping git rm")
		return 0
	if dry_run:
		dry_run_print(f"git rm {path}", dry_run)
	else:
		# cwd=repo_root so the relative pathspec resolves against the resolved
		# repo root, not the caller's working directory.
		subprocess.run(["git", "rm", path], check=True, capture_output=True, cwd=repo_root)
	return 1


def git_rm_recursive(path: str, repo_root: str, dry_run: bool) -> int:
	"""Remove a tracked directory recursively via git rm -r, anchored at repo_root.

	Idempotent: an untracked or already-removed directory is skipped, not an
	error, so a reset never aborts mid-run on a path that is simply already gone.
	"""
	if not path_has_tracked_entry(path, repo_root):
		print(f"{path} not tracked -- skipping git rm -r")
		return 0
	if dry_run:
		dry_run_print(f"git rm -r {path}", dry_run)
	else:
		# cwd=repo_root so the relative pathspec resolves against the resolved
		# repo root, not the caller's working directory.
		subprocess.run(["git", "rm", "-r", path], check=True, capture_output=True, cwd=repo_root)
	return 1


def substitute_typescript_package_json(repo_root: str, dry_run: bool) -> int:
	"""Substitute __REPO_NAME__ and __REPO_VERSION__ in package.json in-place."""
	package_json_path = os.path.join(repo_root, "package.json")
	if not os.path.isfile(package_json_path):
		return 0
	with open(package_json_path, "r") as f:
		content = f.read()
	# Guard: only substitute when placeholders are present, so an existing
	# consumer-customized package.json is left untouched (noexist bucket
	# already protects against overwrite at copy time; this is belt-and-braces).
	if "__REPO_NAME__" not in content:
		return 0
	repo_name = os.path.basename(repo_root)
	# CalVer: zero-padded month per docs/REPO_STYLE.md (0Y.0M), e.g. 2026.06.0
	now = datetime.datetime.now()
	repo_version = f"{now.year}.{now.month:02d}.0"
	if dry_run:
		dry_run_print(
			f"substitute __REPO_NAME__ -> {repo_name}, __REPO_VERSION__ -> {repo_version} in {package_json_path}", dry_run
		)
		return 1
	content = content.replace("__REPO_NAME__", repo_name)
	content = content.replace("__REPO_VERSION__", repo_version)
	with open(package_json_path, "w") as f:
		f.write(content)
	return 1


def run_propagate(repo_root: str, dry_run: bool) -> int:
	"""Lay down type-dispatched template files into repo_root via repolib.

	In dry-run, process_repo previews actions without writing.

	Raises:
		RuntimeError: When process_repo returns None (propagation was skipped).
			This means initial-setup propagation silently no-opped, which must
			never happen during reset.
	"""
	# Build a initial-setup context and run the propagator directly.
	# process_repo honors context.dry_run: it logs planned actions and skips
	# all file mutations when dry_run is True.
	context = repolib.process.build_context_for_repo(
		repo_path=repo_root,
		dry_run=dry_run,
		initial_setup=True,
		auto_discover=False,
		write_marker=False,
	)
	counters = repolib.console.init_counters()
	result = repolib.process.process_repo(repo_root, context, counters, emit_per_repo_summary=False)
	# None return means process_repo intentionally skipped this repo (self-skip guard
	# or not a repo dir). During reset, propagation must always run to completion.
	if result is None:
		raise RuntimeError(
			f"initial-setup propagation was skipped for repo: {repo_root}\n"
			"process_repo returned None -- the self-skip guard may have fired. "
			"Ensure repolib is configured with initial_setup=True (initial-setup)."
		)
	return 1


def read_stub_version(stub_path: str) -> str:
	"""Read the [project] version string from the pyproject stub.

	The stub is minimal (a [project] table with name + version). Parse the
	version line directly so no toml library dependency is introduced.

	Args:
		stub_path (str): Path to templates/python/_pypi/noexist/pyproject.toml.

	Returns:
		str: The version string, e.g. "26.06".

	Raises:
		RuntimeError: When no version line is found in the stub.
	"""
	with open(stub_path, "r") as f:
		stub_lines = f.read().splitlines()
	for line in stub_lines:
		stripped = line.strip()
		# Match a top-level version assignment: version = "..."
		if stripped.startswith("version") and "=" in stripped:
			# Split on the first '=' and strip surrounding quotes/space.
			value = stripped.split("=", 1)[1].strip()
			version = value.strip('"').strip("'")
			return version
	raise RuntimeError(f"no [project] version found in stub: {stub_path}")


def seed_pyproject(repo_root: str, dry_run: bool) -> int:
	"""Seed pyproject.toml from the _pypi stub and write a synced VERSION file.

	Runs BEFORE propagation so select_overlay_dirs sees pyproject.toml and the
	python/_pypi overlay (submit_to_pypi.py) ships in the same reset. Skips
	seeding when pyproject.toml already exists at the repo root (a consumer file
	is left untouched). VERSION is written to match the stub's [project] version
	per docs/REPO_STYLE.md.

	Args:
		repo_root (str): Repository root path.
		dry_run (bool): When True, log the actions without writing.

	Returns:
		int: Count of actions taken or announced.
	"""
	pyproject_path = os.path.join(repo_root, "pyproject.toml")
	# A consumer-supplied pyproject already selects the overlay; leave it alone.
	if os.path.isfile(pyproject_path):
		dry_run_print("pyproject.toml already present -- skip seeding", dry_run)
		return 0
	# Anchor the stub path on the script location, not a repo_root-relative
	# hardcode, so seeding works regardless of where the script is invoked from.
	script_dir = os.path.dirname(os.path.abspath(__file__))
	stub_path = os.path.join(script_dir, "templates/python/_pypi/noexist/pyproject.toml")
	with open(stub_path, "r") as src:
		stub_content = src.read()
	version = read_stub_version(stub_path)
	version_path = os.path.join(repo_root, "VERSION")
	if dry_run:
		dry_run_print(f"seed pyproject.toml from {stub_path}", dry_run)
		dry_run_print(f"write VERSION ({version})", dry_run)
		return 2
	# Write the seed pyproject and a VERSION file holding the same version string.
	with open(pyproject_path, "w") as dst:
		dst.write(stub_content)
	with open(version_path, "w") as vf:
		vf.write(f"{version}\n")
	return 2


# Template-owned root-level directories that must be absent after reset cleanup.
# Only the specific template convention locations for "meta" are checked:
# root meta/ and tests/meta/. Legitimate consumer meta/ elsewhere is not rejected.
# Note: root tools/ is intentionally NOT listed. The cleanup phase still runs
# `git rm -r tools/` to remove the template's own tracked root tools/ (e.g.
# tools/detect_repo_type.py), but typed overlays may now legitimately ship files
# into a consumer's tools/ (e.g. tools/sync_typescript_package_pins.py for
# typescript). Those freshly propagated, still-untracked files survive `git rm`
# and must not trip the end-state verifier, so tools/ is not an owned prefix.
TEMPLATE_OWNED_PREFIXES = [
	"templates/",
	"repolib/",
	"LICENSES/",
	"meta/",
	"tests/meta/",
]

# Sentinel scaffold paths that must exist after successful propagation, by project type.
# Each entry is (project_type, relative_path). Rust, swift, other, and the base
# types with no dedicated overlay (scripted, compiled) are skipped (no sentinel).
SCAFFOLD_SENTINELS: dict[str, str] = {
	"typescript": "eslint.config.js",
	"python": "docs/PYTHON_STYLE.md",
	"website": "mkdocs.yml",
}


def verify_clean_end_state(repo_root: str, dry_run: bool) -> int:
	"""Verify no template-owned paths remain after cleanup.

	In dry-run, logs the check that would be performed.
	In live mode, checks (a) git ls-files and (b) disk for each TEMPLATE_OWNED_PREFIXES
	entry. Raises RuntimeError listing every leftover path found. Note that root
	tools/ is deliberately excluded from TEMPLATE_OWNED_PREFIXES: typed overlays may
	ship files into a consumer's tools/ (e.g. tools/sync_typescript_package_pins.py),
	so a tools/ directory remaining on disk after `git rm -r tools/` is expected and
	must not fail this verifier.

	Returns:
		int: 1 (action taken or announced).

	Raises:
		RuntimeError: When any template-owned path remains tracked or on disk.
	"""
	if dry_run:
		print("DRY-RUN: verify: would check for leftover template-owned paths")
		return 1

	# (a) Check git ls-files for any tracked path under template-owned prefixes
	ls_result = subprocess.run(
		["git", "ls-files"], check=True, capture_output=True, text=True, cwd=repo_root,
	)
	tracked_paths = ls_result.stdout.splitlines()
	leftover_tracked: list[str] = []
	for tracked_path in tracked_paths:
		for prefix in TEMPLATE_OWNED_PREFIXES:
			if tracked_path.startswith(prefix) or tracked_path == prefix.rstrip("/"):
				leftover_tracked.append(f"tracked: {tracked_path}")
				break

	# (b) Check that root-level template-owned directories are absent on disk.
	# For nested entries like tests/meta/, check the full path.
	leftover_disk: list[str] = []
	for prefix in TEMPLATE_OWNED_PREFIXES:
		# strip trailing slash for os.path.isdir check
		check_path = os.path.join(repo_root, prefix.rstrip("/"))
		if os.path.isdir(check_path):
			leftover_disk.append(f"on disk: {prefix}")

	all_leftovers = leftover_tracked + leftover_disk
	if all_leftovers:
		leftover_list = "\n  ".join(all_leftovers)
		raise RuntimeError(
			f"template-owned paths remain after cleanup:\n  {leftover_list}"
		)
	return 1


def verify_scaffold_sentinel(repo_root: str, project_type: str) -> None:
	"""Assert that at least one required scaffold path exists after propagation.

	This guards against a "successful but empty" propagation regression, where
	process_repo returns a dict but wrote nothing. Only checked for project types
	that have a known sentinel (typescript, python). Raises RuntimeError on failure.

	Args:
		repo_root (str): Repository root path.
		project_type (str): The project type marker (e.g. 'typescript', 'python,rust').

	Raises:
		RuntimeError: When the sentinel path is absent after propagation.
	"""
	sentinel = SCAFFOLD_SENTINELS.get(project_type)
	if sentinel is None:
		# rust and other have no sentinel defined; skip silently
		return
	sentinel_path = os.path.join(repo_root, sentinel)
	if not os.path.isfile(sentinel_path):
		raise RuntimeError(
			f"propagation completed but required scaffold path is missing: {sentinel}\n"
			f"Expected at: {sentinel_path}\n"
			"process_repo returned success but may have written nothing."
		)


def truncate_file(path: str, repo_root: str, dry_run: bool) -> int:
	"""Truncate file to zero bytes."""
	full_path = os.path.join(repo_root, path)
	if dry_run:
		dry_run_print(f"truncate {path}", dry_run)
	else:
		open(full_path, "w").close()
	return 1


#============================================
# Config resolution helpers
#============================================

def normalize_project_type(raw: str, default: str) -> str:
	"""Normalize a raw project-type answer to a canonical marker.

	Accepts the single-letter menu shortcuts (p/t/r/s/o), the full type names
	(python/typescript/rust/swift/other/all), the base types added for repo-type
	inheritance (scripted/website/compiled, full name only, no single-letter
	alias since their letters are already claimed by concrete descendants), or
	an empty string (which selects the supplied default). Shared by the
	interview and the config producers so the accepted values cannot drift
	between the two paths.

	Several types may be declared at once, either comma separated ("p,r") or as a
	run of single-letter aliases ("pr"); both become "python,rust". The result is
	the canonical written form: lowercase types, comma separated, no spaces,
	declaration order preserved, deduped by first occurrence, and "all" left as the
	literal "all" rather than expanded.

	An empty answer selects the default, and the default is normalized through the
	same path rather than returned verbatim. That matters when the default came
	from an existing REPO_TYPE marker: a stored marker carrying duplicates or stray
	spacing is rewritten in canonical form instead of being perpetuated by someone
	simply pressing Enter.

	Unlike the propagator's marker reader, an unrecognized piece exits rather
	than degrading. Bootstrap validates its input up front, before any mutation,
	so reset never fails partway through a reset. Every piece is checked before
	anything is returned.

	Args:
		raw (str): The raw user answer or config value.
		default (str): Marker to use when raw is empty. Normalized like any answer.

	Returns:
		str: A canonical marker of one or more comma-separated types drawn from
			"python", "typescript", "rust", "swift", "other", "all", "scripted",
			"website", "compiled".
	"""
	answer = raw.strip().lower()
	if answer == "":
		# Normalize the default through the same path instead of returning it as
		# stored, so accepting an existing marker with Enter canonicalizes it.
		answer = default.strip().lower()
	if answer == "":
		return default
	# Answer parsing is shared with the propagator's prompt
	# (repolib.repo.expand_choice_piece and its alias table), so adding or renaming
	# a repo type is a single edit rather than two copies that can silently drift,
	# and both prompts accept the same forms. Only the parsing is shared: this
	# function still exits on an invalid piece, while the propagator's reader drops
	# unknown pieces and keeps the valid half.
	# Validate and collect every piece before returning, so an invalid piece is
	# rejected up front rather than after part of the answer was accepted.
	declared_types: list[str] = []
	for piece in answer.split(","):
		piece_types = repolib.repo.expand_choice_piece(piece)
		if piece_types is None:
			sys.exit(f"Invalid project type: {raw!r}")
		for declared_type in piece_types:
			if declared_type not in declared_types:
				declared_types.append(declared_type)
	return ",".join(declared_types)


def resolve_project_type(repo_root: str) -> str:
	"""Resolve project type interactively, seeded by detection or existing marker.

	The existing REPO_TYPE marker (when present) is offered as the prompt
	default; an empty answer accepts it. Bootstrap overwrites the marker without
	a force guard.

	Args:
		repo_root (str): Repository root path.

	Returns:
		str: The resolved project type marker, one or more comma-separated types.
	"""
	marker_path = os.path.join(repo_root, "REPO_TYPE")
	existing_marker = None
	if os.path.isfile(marker_path):
		with open(marker_path, "r") as f:
			marker_text = f.read().strip()
		# Readers tolerate stray whitespace around commas, but an accepted default is
		# rewritten to the marker file, so reduce it to the canonical no-spaces form.
		# normalize_project_type canonicalizes the default again on the way out, so a
		# stored marker with duplicates is not perpetuated by pressing Enter.
		marker_types = [piece.strip() for piece in marker_text.split(",") if piece.strip()]
		existing_marker = ",".join(marker_types)

	# Choose the default offered at the prompt: an existing marker wins, then
	# detection, then python.
	if existing_marker:
		default_type = existing_marker
	elif detect_repo_type:
		# Try to predict repo type when the detector module is available.
		detected_type, confidence, _ = detect_repo_type.detect_repo_type(repo_root)
		if confidence == 'high' and detected_type != 'ambiguous':
			default_type = detected_type
		elif confidence == 'medium':
			default_type = detected_type
		else:
			default_type = "python"
	else:
		default_type = "python"

	# Always prompt; an empty answer accepts the default. The default is shown in
	# full rather than as its first letter, since it may be a comma-separated marker.
	user_input = input(
		"Project type? [p]ython / [t]ypescript / [r]ust / [s]wift / [o]ther / "
		"[a]ll / scripted / website / compiled "
		f"(list allowed, e.g. python,rust or pr) [{default_type}]: "
	).strip()
	return normalize_project_type(user_input, default_type)


def resolve_pypi(project_type: str) -> bool:
	"""Resolve whether a python project publishes to PyPI.

	Only meaningful for python repos. Non-python types never seed pyproject.toml.
	For python repos, prompt the user; default is no.

	Args:
		project_type (str): Resolved project type marker.

	Returns:
		bool: True when pyproject.toml should be seeded before propagation.
	"""
	# PyPI seeding only applies to python repos.
	if project_type != "python":
		return False
	# Interactive python run: ask, default no.
	user_input = input("Will this Python project be published as a pypi package? [y/N]: ").strip()
	return user_input.lower() == "y"


def resolve_licenses() -> tuple:
	"""Prompt for code and docs licenses, resolving via alias or unique prefix."""
	# Code license: prompt until a valid choice is entered (no default).
	while True:
		user_input = input(
			"Code license?\n  [m] MIT\n  [a] Apache-2.0\n  [l] LGPL-3.0\n  [g] GPL-3.0\n  [ag] AGPL-3.0\n  [mp] MPL-2.0\nChoice: "
		).strip()
		try:
			code_license = resolve_license(
				user_input, CODE_LICENSES, CODE_ALIASES, default=None
			)
			break
		except ValueError as e:
			print(f"Error: {e}. Please try again.")

	# Docs license: empty answer accepts the none default.
	user_input = input(
		"Docs license?\n  [cb] CC-BY-4.0\n  [cs] CC-BY-SA-4.0\n  [n] none\nChoice [n]: "
	).strip()
	try:
		docs_license = resolve_license(
			user_input, DOCS_LICENSES, DOCS_ALIASES, default="none"
		)
	except ValueError as e:
		sys.exit(f"Invalid docs license: {e}")

	return code_license, docs_license


def resolve_stage() -> bool:
	"""Prompt whether to stage changes; default yes."""
	user_input = input("Stage changes? [Y/n]: ").strip()
	# Empty answer accepts the yes default; only an explicit 'n' declines.
	return user_input.lower() != "n"


def resolve_commit() -> bool:
	"""Prompt whether to create a commit; default no."""
	user_input = input("Create a commit? [y/N]: ").strip()
	return user_input.lower() == "y"


#============================================
# Answers seam: interview and config producers
#============================================

@dataclasses.dataclass
class ResetAnswers:
	"""Resolved bootstrap answers, from interview or config.

	Attributes:
		project_type (str): Canonical project type marker.
		code_license (str): Resolved SPDX code license id.
		docs_license (str): Resolved SPDX docs license id, or "none".
		pypi (bool): Whether to seed pyproject.toml for PyPI publishing.
		stage (bool): Whether to stage changes with git add -A.
		commit (bool): Whether to create a commit after staging.
	"""
	project_type: str
	code_license: str
	docs_license: str
	pypi: bool
	stage: bool
	commit: bool


def answers_from_interview(repo_root: str) -> ResetAnswers:
	"""Collect answers via the interactive prompts.

	Asks, in order: project type, code license, docs license, PyPI (python
	only), stage changes, create a commit. Returns the resolved answers.

	Args:
		repo_root (str): Repository root path.

	Returns:
		ResetAnswers: The resolved answers.
	"""
	project_type = resolve_project_type(repo_root)
	code_license, docs_license = resolve_licenses()
	pypi = resolve_pypi(project_type)
	stage = resolve_stage()
	commit = resolve_commit()
	answers = ResetAnswers(
		project_type=project_type,
		code_license=code_license,
		docs_license=docs_license,
		pypi=pypi,
		stage=stage,
		commit=commit,
	)
	return answers


def load_config(path: str) -> dict:
	"""Load and validate a json answers file.

	Parses the file with the stdlib json reader and raises a clear error
	(rather than leaking a bare FileNotFoundError, JSONDecodeError, or type
	confusion) when the file is missing, is not valid json, or does not parse
	to a json object (dict) at the top level.

	Args:
		path (str): Path to the json answers file.

	Returns:
		dict: The parsed top-level json object.
	"""
	if not os.path.isfile(path):
		sys.exit(f"Error: config file not found: {path}")
	with open(path, "r") as f:
		raw_text = f.read()
	# Parse json; convert a decode failure into a clear, named error.
	try:
		data = json.loads(raw_text)
	except json.JSONDecodeError as exc:
		sys.exit(f"Error: config file is not valid json: {path} ({exc})")
	if not isinstance(data, dict):
		sys.exit(
			f"Error: config file must be a json object at the top level: {path}"
		)
	return data


def answers_from_config(path: str) -> ResetAnswers:
	"""Build ResetAnswers from a json config file.

	Required keys (project_type, code_license) are read so a missing one raises
	a clear message naming the key. Optional keys use defaults that match the
	interview defaults exactly: docs_license=none, pypi=False, stage=True,
	commit=False. License values reuse resolve_license so accepted values cannot
	drift from the interview path.

	Args:
		path (str): Path to the json answers file.

	Returns:
		ResetAnswers: The resolved answers.
	"""
	config = load_config(path)
	# Required keys: name the missing key clearly instead of leaking KeyError.
	if "project_type" not in config:
		sys.exit(f"Error: config missing required key 'project_type': {path}")
	if "code_license" not in config:
		sys.exit(f"Error: config missing required key 'code_license': {path}")
	# Normalize project type through the shared helper; no default fallback is
	# needed since the key is required, but pass python as a harmless default.
	project_type = normalize_project_type(str(config["project_type"]), "python")
	# Resolve licenses through resolve_license so config tokens accept the same
	# aliases and prefixes as the interview.
	try:
		code_license = resolve_license(
			str(config["code_license"]), CODE_LICENSES, CODE_ALIASES, default=None
		)
	except ValueError as exc:
		sys.exit(f"Error: invalid code_license in config: {exc}")
	# Optional docs_license defaults to none (matches interview default).
	docs_raw = config.get("docs_license", "none")
	try:
		docs_license = resolve_license(
			str(docs_raw), DOCS_LICENSES, DOCS_ALIASES, default="none"
		)
	except ValueError as exc:
		sys.exit(f"Error: invalid docs_license in config: {exc}")
	# Optional behavior flags default to the interview defaults.
	pypi = config.get("pypi", False)
	stage = config.get("stage", True)
	commit = config.get("commit", False)
	# PyPI seeding only applies to python repos, mirroring resolve_pypi.
	if project_type != "python":
		pypi = False
	answers = ResetAnswers(
		project_type=project_type,
		code_license=code_license,
		docs_license=docs_license,
		pypi=bool(pypi),
		stage=bool(stage),
		commit=bool(commit),
	)
	return answers


def is_template_source_dir(repo_root: str) -> bool:
	"""Return True when repo_root is the template source checkout.

	Detects the template by folder name only (no remote/origin inspection) so
	the refuse-guard is deterministic and unit-testable.

	Args:
		repo_root (str): Repository root path.

	Returns:
		bool: True when the basename is "starter-repo-template".
	"""
	# normpath strips a trailing slash so basename cannot return "" and bypass the guard
	return os.path.basename(os.path.normpath(repo_root)) == "starter-repo-template"


def confirm_plan(answers: ResetAnswers, dry_run: bool, skip_confirm: bool) -> None:
	"""Print the plan summary and prompt the user to confirm before applying.

	Args:
		answers: The resolved bootstrap answers describing what will be applied.
		dry_run: When True, prefix the mode label with DRY-RUN for clarity.
		skip_confirm: When True, skip printing and the Proceed prompt entirely.
			Set to True in config mode (bool(args.config)) because config runs
			are non-interactive; interactive mode passes False so the user sees
			the summary and must type 'y' to proceed.
	"""
	if not skip_confirm:
		mode = "DRY-RUN" if dry_run else "LIVE"
		print("")
		print("Summary:")
		print(f"  type:         {answers.project_type}")
		print(f"  code license: {answers.code_license}")
		print(f"  docs license: {answers.docs_license}")
		print(f"  pypi:         {'yes' if answers.pypi else 'no'}")
		print(f"  stage:        {'yes' if answers.stage else 'no'}")
		print(f"  commit:       {'yes' if answers.commit else 'no'}")
		print(f"  mode:         {mode}")
		confirm_input = input("Proceed? [y/N]: ").strip()
		if not confirm_input or confirm_input.lower() != "y":
			sys.exit("Aborted")


def main() -> None:
	"""Run the interactive bootstrap flow: gather answers, preflight, then apply phases."""
	args = parse_args()
	repo_root = get_repo_root()

	# === phase: source-repo refuse guard (SAFETY CRITICAL) ===
	# Run FIRST, before any phase and regardless of --dry-run/--config: refuse to
	# reset the template source checkout itself.
	if is_template_source_dir(repo_root):
		sys.exit(
			"This repo is named starter-repo-template. Clone or rename it to "
			"the consumer project name before running reset."
		)

	# === phase: gather answers (config or interview) ===
	# Config mode (--config) is non-interactive; the interview asks, in order:
	# project type, code license, docs license, PyPI (python only), stage
	# changes, create a commit. Staging and commit are driven by these answers.
	if args.config:
		answers = answers_from_config(args.config)
	else:
		answers = answers_from_interview(repo_root)

	# Pull the resolved answers into locals for the phase bodies below.
	project_type = answers.project_type
	code_license = answers.code_license
	docs_license = answers.docs_license
	publish_pypi = answers.pypi
	stage = answers.stage
	commit = answers.commit

	preflight_check(repo_root, code_license, docs_license)

	# === phase: summary and confirmation ===
	# Config mode auto-skips the Proceed prompt; interactive mode keeps it.
	skip_confirm = bool(args.config)
	confirm_plan(answers, args.dry_run, skip_confirm)

	action_count = 0

	# === phase: marker write ===
	action_count += write_marker(repo_root, project_type, args.dry_run)

	# === phase: license install ===
	code_source = os.path.join(repo_root, f"LICENSES/LICENSE.{code_license}.md")
	action_count += copy_license(repo_root, code_source, f"LICENSE.{code_license}.md", args.dry_run)

	if docs_license != "none":
		docs_source = os.path.join(repo_root, f"LICENSES/LICENSE.{docs_license}.md")
		action_count += copy_license(repo_root, docs_source, f"LICENSE.{docs_license}.md", args.dry_run)

	# === phase: cleanup LICENSES/ ===
	action_count += git_rm_recursive("LICENSES/", repo_root, args.dry_run)

	# === phase: seed pyproject (BEFORE propagate) ===
	# On a PyPI-publishing python repo, seed pyproject.toml from the _pypi stub
	# (plus a synced VERSION file) so select_overlay_dirs includes python/_pypi
	# during the propagate phase below and submit_to_pypi.py ships in this reset.
	# This MUST run before run_propagate so the overlay selection sees the file.
	if publish_pypi:
		action_count += seed_pyproject(repo_root, args.dry_run)

	# === phase: propagate (direct repolib call) ===
	action_count += run_propagate(repo_root, args.dry_run)

	# === phase: scaffold sentinel check ===
	# After propagation completes (live only), assert that the required per-type
	# scaffold path exists. Guards against "successful but empty" propagation.
	if not args.dry_run:
		verify_scaffold_sentinel(repo_root, project_type)

	# === phase: typescript-specific work ===
	# Must run AFTER propagate so the noexist bucket has placed package.json at repo root.
	if project_type == "typescript":
		action_count += substitute_typescript_package_json(repo_root, args.dry_run)

	# === phase: truncate boilerplate ===
	action_count += truncate_file("README.md", repo_root, args.dry_run)
	action_count += truncate_file("docs/CHANGELOG.md", repo_root, args.dry_run)

	# === phase: remove changelog archives ===
	# Rotation archives (docs/CHANGELOG-*.md) are template-meta: they record the
	# template's own changelog history and must not linger in a consumer clone.
	# The active docs/CHANGELOG.md is truncated above; the archives are matched by
	# META_FILE_PATTERNS glob and removed here. META_FILE_PATTERNS is read before
	# repolib/ is git-rm'd below, so this source remains valid.
	for pattern in repolib.model.META_FILE_PATTERNS:
		for archive_path in sorted(glob.glob(os.path.join(repo_root, pattern))):
			archive_rel = os.path.relpath(archive_path, repo_root)
			action_count += git_rm(archive_rel, repo_root, args.dry_run)

	# === phase: remove templates/ ===
	# templates/ must be removed AFTER propagation has read from it and AFTER
	# gitignore merge completes. Untracked or absent templates/ is not an error
	# (supports partially repaired clones).
	templates_dir = os.path.join(repo_root, "templates")
	if args.dry_run:
		if os.path.isdir(templates_dir):
			dry_run_print("git rm -r templates/", args.dry_run)
			action_count += 1
		else:
			dry_run_print("templates/ absent -- skip removal", args.dry_run)
	else:
		ls_templates = subprocess.run(
			["git", "ls-files", "templates/"],
			check=True, capture_output=True, text=True, cwd=repo_root,
		)
		if ls_templates.stdout.strip():
			# templates/ has tracked files; remove them via git rm -r
			action_count += git_rm_recursive("templates/", repo_root, args.dry_run)
		elif os.path.isdir(templates_dir):
			# untracked templates/ directory present; log and skip (no git state to touch)
			print("templates/ is untracked -- skipping git rm (directory left on disk)")
		else:
			# completely absent; nothing to do
			print("templates/ absent -- nothing to remove")

	# === phase: git rm cleanup ===
	# Remove the template-only propagation infrastructure from the consumer:
	# entry script and the repolib package (renamed from propagate/ in the template).
	action_count += git_rm("propagate_style_guides.py", repo_root, args.dry_run)
	action_count += git_rm_recursive("repolib/", repo_root, args.dry_run)
	# pip_requirements-meta.txt holds deps for the template's own meta tooling
	# (propagate/reset/tests). It is a meta_files entry (never ships) and must not
	# linger in a consumer clone; remove it like the propagation infrastructure.
	action_count += git_rm("pip_requirements-meta.txt", repo_root, args.dry_run)
	# Remove the template's own tracked root tools/ (e.g. tools/detect_repo_type.py).
	# `git rm -r tools/` removes tracked entries only; freshly propagated untracked
	# files (e.g. tools/sync_typescript_package_pins.py for typescript consumers)
	# survive and stay on disk. Guard on tracked content so the case where tools/
	# holds only untracked propagated files (no tracked entry) is logged and skipped
	# instead of failing on a no-match pathspec, mirroring the templates/ handling.
	if args.dry_run:
		dry_run_print("git rm -r tools/", args.dry_run)
		action_count += 1
	else:
		ls_tools = subprocess.run(
			["git", "ls-files", "tools/"],
			check=True, capture_output=True, text=True, cwd=repo_root,
		)
		if ls_tools.stdout.strip():
			action_count += git_rm_recursive("tools/", repo_root, args.dry_run)
		else:
			print("tools/ has no tracked files -- skipping git rm (any propagated files left on disk)")
	# Strip every directory named "meta/" anywhere in the tree (template-only
	# trees: top-level meta/, tests/meta/, any future subtree/meta/). Walk the
	# git index so only tracked dirs are touched; pick the shallowest "meta"
	# in each path so a nested case like a/meta/sub/meta/ collapses to a/meta/
	# and `git rm -r` is not asked to remove the same subtree twice.
	ls_result = subprocess.run(
		["git", "ls-files"], check=True, capture_output=True, text=True, cwd=repo_root
	)
	tracked = ls_result.stdout.splitlines()
	meta_dirs: list[str] = []
	seen = set()
	for tracked_path in tracked:
		parts = tracked_path.split("/")
		for idx, part in enumerate(parts):
			if part == "meta":
				meta_dir = "/".join(parts[: idx + 1]) + "/"
				if meta_dir not in seen:
					seen.add(meta_dir)
					meta_dirs.append(meta_dir)
				break
	# Drop entries whose ancestor is already in the set (sibling dirs like
	# meta/ and tests/meta/ are not ancestor-nested and both survive).
	meta_dirs.sort(key=len)
	pruned: list[str] = []
	for candidate in meta_dirs:
		covered = any(candidate.startswith(ancestor) and candidate != ancestor for ancestor in pruned)
		if not covered:
			pruned.append(candidate)
	for meta_dir in pruned:
		action_count += git_rm_recursive(meta_dir, repo_root, args.dry_run)

	# Keep submit_to_pypi.py only when the python/_pypi overlay applies to this
	# repo. Compute a single pypi_applies boolean so the dry-run path does not
	# mislog a cleanup it would not perform: select_overlay_dirs sees pyproject.toml
	# only when it exists on disk, which in dry-run is never (seed_pyproject skipped
	# writing it). OR in the chosen PyPI answer so a dry-run with PyPI=yes does NOT
	# log a misleading "git rm devel/submit_to_pypi.py".
	overlay_dirs = repolib.model.select_overlay_dirs(project_type, repo_root)
	pypi_applies = publish_pypi or (f"{project_type}/_pypi" in overlay_dirs)
	if not pypi_applies:
		# Guard on tracked content so a fresh repo that never received the PyPI
		# overlay (PyPI=no, or a non-python type, or python-without-pyproject)
		# does not fail on a no-match pathspec. `git rm` aborts with exit 128 when
		# the path is untracked, so only remove devel/submit_to_pypi.py when the
		# git index actually tracks it, mirroring the tools/ handling above.
		if args.dry_run:
			dry_run_print("git rm -f devel/submit_to_pypi.py", args.dry_run)
			action_count += 1
		else:
			ls_pypi = subprocess.run(
				["git", "ls-files", "devel/submit_to_pypi.py"],
				check=True, capture_output=True, text=True, cwd=repo_root,
			)
			if ls_pypi.stdout.strip():
				# Force-remove: when the overlay does not apply, this file must not
				# exist. The propagate phase may have refreshed it on disk from the
				# _pypi overlay source (a consumer with a stale, tracked
				# devel/submit_to_pypi.py), leaving the working tree differing from
				# the index. Plain `git rm` aborts on that difference; `-f` removes
				# it unconditionally, which is the intended PyPI=no end state.
				subprocess.run(
					["git", "rm", "-f", "devel/submit_to_pypi.py"],
					check=True, capture_output=True, cwd=repo_root,
				)
				action_count += 1
			else:
				print("devel/submit_to_pypi.py untracked -- skipping git rm")

	action_count += git_rm("reset_repo.py", repo_root, args.dry_run)

	# === phase: end-state verification ===
	# Verify no template-owned paths remain (git index + disk). In dry-run, logs
	# the check that would happen. In live mode, raises on any leftover.
	action_count += verify_clean_end_state(repo_root, args.dry_run)

	# === phase: stage changes ===
	if stage:
		action_count += 1
		if args.dry_run:
			dry_run_print("git add -A", args.dry_run)
		else:
			subprocess.run(["git", "add", "-A"], check=True, capture_output=True, cwd=repo_root)

	# === phase: commit ===
	if commit:
		action_count += 1
		commit_msg = f"initial commit: reset repo to base template ({project_type})"
		if args.dry_run:
			dry_run_print(f"git commit -m {repr(commit_msg)}", args.dry_run)
		else:
			subprocess.run(
				["git", "commit", "-m", commit_msg], check=True, capture_output=True, cwd=repo_root
			)

	# === phase: summary print ===
	if args.dry_run:
		print(f"DRY-RUN: {action_count} actions planned. No files changed.")
	else:
		if commit:
			print("Committed.")
		elif not stage:
			print("Working tree modified. Run 'git add -A && git commit' when ready.")
		else:
			print("Staged. Run 'git commit' when ready.")

		subprocess.run(["git", "status", "--short"], check=False, cwd=repo_root)

	# Next-step hint lists the dependency files actually present for this repo type.
	# python ships both pip_requirements.txt (python-only) and the universal
	# pip_requirements-dev.txt; every other type ships only the universal dev file.
	if project_type == "python":
		print("\nNext steps:")
		print("  pip install -r pip_requirements.txt && pip install -r pip_requirements-dev.txt")
	elif project_type == "typescript":
		print("\nNext steps:")
		print("  npm install && bash devel/setup_playwright.sh")
		print("  pip install -r pip_requirements-dev.txt")
	elif project_type == "rust":
		print("\nNext steps:")
		print("  cargo build")
		print("  pip install -r pip_requirements-dev.txt")
	else:
		print("\nNext steps:")
		print("  pip install -r pip_requirements-dev.txt")


if __name__ == "__main__":
	main()
