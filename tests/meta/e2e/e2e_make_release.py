#!/usr/bin/env python3
"""
e2e_make_release.py - real-git end-to-end harness for make_release.py.

Drives templates/shared/devel/make_release.py through its full release flow in a
self-built temporary git repo, using REAL git subprocess (git init, git commit,
git archive, git tag). PYTEST_STYLE.md forbids real subprocess CLI round-trips in
pytest, so this closes the one seam the fast pytest tier cannot reach: building
real archives from a committed HEAD and verifying their bundled LICENSE
byte-for-byte. The pure and stdlib-only units live in tests/meta/test_make_release.py.

What this harness builds and exercises, independent of the main repo's state:

  - A temp git repo carrying sibling devel/ helpers (changelog_lib.py,
    commit_changelog.py, make_release.py copied from the local checkout), a
    VERSION file pinned to 26.07 (NOT the live VERSION, to dodge the
    duplicate-version guard on real seeds), a COMMITTED LICENSE, and committed
    docs/RELEASE_HISTORY.md + docs/NEWS.md seeds.
  - ensure_tag_free passes on the clean repo and raises once a v26.07 tag exists.
  - ensure_committed_license passes here and raises in a second repo lacking it.
  - The no-notes path prints the LLM prompt and builds nothing.
  - --write builds output_release/{repo}-v26.07.{zip,tgz} via real git archive,
    and verify_archive_license passes against the REAL built archives.
  - The human-run "git tag" and "gh release create" command strings are printed.
  - OBSERVABILITY: git tag --list is captured before, after the no-notes run, and
    after --write; the script must never tag or mutate git, so it stays unchanged.

Run it directly (outside pytest):

    source source_me.sh && python3 tests/meta/e2e/e2e_make_release.py

Self-contained: builds real git repos under a tempfile.mkdtemp() base that is
removed on exit, may use asserts (it is an E2E test file), and exits non-zero
with clear stderr on the first failure.
Template-meta: lives under tests/meta/e2e/; never propagates; removed by reset.
"""

# Standard Library
import io
import os
import sys
import shutil
import tempfile
import contextlib
import subprocess
import importlib.util

# Resolve the local checkout via git so source helper paths are stable
# regardless of cwd. This file's own repo is the source of the devel helpers.
LOCAL_CHECKOUT = subprocess.run(
	["git", "rev-parse", "--show-toplevel"],
	cwd=os.path.dirname(os.path.abspath(__file__)),
	capture_output=True, text=True, check=True,
).stdout.strip()

# Source helper files copied into each temp repo's devel/ folder. make_release.py
# lives under the shared template overlay; its two siblings live in devel/.
DEVEL_SOURCES = {
	"changelog_lib.py": os.path.join(LOCAL_CHECKOUT, "devel", "changelog_lib.py"),
	"commit_changelog.py": os.path.join(LOCAL_CHECKOUT, "devel", "commit_changelog.py"),
	"make_release.py": os.path.join(
		LOCAL_CHECKOUT, "templates", "shared", "devel", "make_release.py"
	),
}

# Pinned release version: NOT the live VERSION, so the doc writers never collide
# with a real seeded entry and the duplicate-version guard stays dormant.
RELEASE_VERSION = "26.07"

# Fixed LICENSE bytes committed into the repo; the archive check compares these
# byte-for-byte against what git archive bundles.
LICENSE_TEXT = "MIT License\n\nCopyright 2026 E2E Test\n"


#============================================
# Temp git repo construction
#============================================

def git_run(repo_dir: str, args: list[str]) -> None:
	"""Run a git command inside repo_dir, raising on failure.

	Args:
		repo_dir (str): The repository working directory.
		args (list[str]): Git arguments (no leading "git" token).
	"""
	subprocess.run(["git", "-C", repo_dir, *args], check=True, capture_output=True, text=True)


def build_release_repo(repo_dir: str, with_license: bool) -> None:
	"""Build a committed git repo that make_release.py can operate on.

	Creates the devel/ helper trio, a VERSION file, optional committed LICENSE,
	and docs seeds, then commits the whole tree so HEAD is non-empty.

	Args:
		repo_dir (str): Destination repo directory (created fresh).
		with_license (bool): When True, commit a LICENSE at the repo root.
	"""
	# Start from a clean directory so a failed earlier run cannot poison this one.
	shutil.rmtree(repo_dir, ignore_errors=True)
	os.makedirs(os.path.join(repo_dir, "devel"))
	os.makedirs(os.path.join(repo_dir, "docs"))

	# Copy the devel helper trio so make_release's bare imports resolve in-repo.
	for name, source_path in DEVEL_SOURCES.items():
		shutil.copyfile(source_path, os.path.join(repo_dir, "devel", name))

	# Pin VERSION to the release version so read_version_file returns it.
	with open(os.path.join(repo_dir, "VERSION"), "w", encoding="utf-8") as version_file:
		version_file.write(RELEASE_VERSION + "\n")

	# Seed the docs targets the --write doc writers prepend to.
	with open(os.path.join(repo_dir, "docs", "RELEASE_HISTORY.md"), "w", encoding="utf-8") as rh:
		rh.write("# Release History\n\n## v26.05 - 2026-05-01\n\nOlder release.\n")
	with open(os.path.join(repo_dir, "docs", "NEWS.md"), "w", encoding="utf-8") as news:
		news.write("# News\n\n## v26.05 - 2026-05-01\n\nOlder news.\n")

	# Optionally commit a LICENSE so the snapshot can ship it inside the archives.
	if with_license:
		with open(os.path.join(repo_dir, "LICENSE"), "w", encoding="utf-8") as license_file:
			license_file.write(LICENSE_TEXT)

	# Initialize, set a local identity, and commit the tree so HEAD exists.
	git_run(repo_dir, ["init", "--quiet"])
	git_run(repo_dir, ["config", "user.email", "e2e@example.com"])
	git_run(repo_dir, ["config", "user.name", "e2e harness"])
	git_run(repo_dir, ["add", "-A"])
	git_run(repo_dir, ["commit", "--quiet", "-m", "initial"])


def load_make_release(repo_dir: str) -> object:
	"""Load make_release from a repo's devel/ folder with its siblings importable.

	The module does bare `import changelog_lib` / `import commit_changelog`, so the
	repo's devel/ directory is inserted on sys.path before exec so those resolve to
	the repo-local copies.

	Args:
		repo_dir (str): The repo whose devel/ holds the helper trio.

	Returns:
		The loaded make_release module object.
	"""
	devel_dir = os.path.join(repo_dir, "devel")
	if devel_dir not in sys.path:
		sys.path.insert(0, devel_dir)
	module_path = os.path.join(devel_dir, "make_release.py")
	spec = importlib.util.spec_from_file_location("make_release", module_path)
	module = importlib.util.module_from_spec(spec)
	spec.loader.exec_module(module)
	return module


#============================================
# Observability helpers
#============================================

def list_tags(repo_dir: str) -> str:
	"""Return the stripped `git tag --list` output for repo_dir."""
	result = subprocess.run(
		["git", "-C", repo_dir, "tag", "--list"],
		capture_output=True, text=True, check=True,
	)
	return result.stdout.strip()


def run_main(make_release: object, argv: list[str]) -> str:
	"""Run make_release.main() with a given argv and return captured stdout.

	Args:
		make_release: The loaded make_release module.
		argv (list[str]): The argv list (including the program name at index 0).

	Returns:
		str: Everything main() printed to stdout.
	"""
	captured = io.StringIO()
	saved_argv = sys.argv
	sys.argv = argv
	try:
		with contextlib.redirect_stdout(captured):
			make_release.main()
	finally:
		sys.argv = saved_argv
	return captured.getvalue()


#============================================
# Checks
#============================================

def check_preconditions_pass(make_release: object, label: str) -> None:
	"""ensure_tag_free and ensure_committed_license pass on the clean repo."""
	# No tag yet, so the tag-free guard must not raise.
	make_release.ensure_tag_free(RELEASE_VERSION)
	# LICENSE is committed, so the committed-license guard must not raise.
	make_release.ensure_committed_license()
	print(f"  PASS [{label}]: ensure_tag_free and ensure_committed_license pass on clean repo")


def check_no_notes_prints_prompt_and_builds_nothing(
	make_release: object, repo_dir: str, label: str,
) -> None:
	"""The no-notes path prints the LLM prompt and builds no archives."""
	output_dir = os.path.join(repo_dir, make_release.OUTPUT_DIR_NAME)
	out = run_main(make_release, ["make_release.py", RELEASE_VERSION])
	# The prompt must name the version so the LLM stamps the right release.
	assert f"v{RELEASE_VERSION}" in out, "no-notes output missing version token"
	# The prompt must ask for release notes (its whole purpose).
	assert "release notes" in out.lower(), "no-notes output missing release-notes guidance"
	# No archives are built on the no-notes path; the output dir must not appear.
	assert not os.path.exists(output_dir), "no-notes path unexpectedly created output_release/"
	print(f"  PASS [{label}]: no-notes path prints the LLM prompt and builds nothing")


def check_write_builds_and_verifies_archives(
	make_release: object, repo_dir: str, label: str,
) -> None:
	"""--write builds real archives, verifies their LICENSE, and prints commands."""
	repo_name = os.path.basename(repo_dir)
	notes_path = os.path.join(repo_dir, "_notes.md")
	with open(notes_path, "w", encoding="utf-8") as notes_file:
		notes_file.write("Added the release flow.\n")
	out = run_main(make_release, [
		"make_release.py", RELEASE_VERSION,
		"--notes-file", notes_path,
		"--write",
	])
	# Both archives must exist after a real git archive build.
	prefix = f"{repo_name}-v{RELEASE_VERSION}/"
	output_dir = os.path.join(repo_dir, make_release.OUTPUT_DIR_NAME)
	zip_path = os.path.join(output_dir, f"{repo_name}-v{RELEASE_VERSION}.zip")
	tgz_path = os.path.join(output_dir, f"{repo_name}-v{RELEASE_VERSION}.tgz")
	assert os.path.isfile(zip_path), f"--write did not build {zip_path}"
	assert os.path.isfile(tgz_path), f"--write did not build {tgz_path}"
	# verify_archive_license raises on mismatch; pass means the bundled LICENSE
	# matches HEAD byte-for-byte in BOTH real archives.
	expected_bytes = make_release.read_head_license_bytes()
	license_member = f"{prefix}LICENSE"
	make_release.verify_archive_license(zip_path, license_member, expected_bytes)
	make_release.verify_archive_license(tgz_path, license_member, expected_bytes)
	# The release and tag steps must surface only as printed command strings.
	assert "git tag" in out, "--write output missing the git tag command"
	assert "gh release create" in out, "--write output missing the gh release create command"
	# The doc writers must have prepended the new entry above the older one.
	rh_text = open(os.path.join(repo_dir, "docs", "RELEASE_HISTORY.md"), encoding="utf-8").read()
	assert rh_text.index("v26.07") < rh_text.index("v26.05"), "release history not prepended"
	print(f"  PASS [{label}]: --write built and verified real zip and tgz archives")


def check_tag_guard_raises_when_tag_exists(
	make_release: object, repo_dir: str, label: str,
) -> None:
	"""ensure_tag_free raises once a v{version} tag exists in the repo."""
	tag_name = f"v{RELEASE_VERSION}"
	git_run(repo_dir, ["tag", tag_name])
	raised = False
	try:
		make_release.ensure_tag_free(RELEASE_VERSION)
	except RuntimeError:
		raised = True
	# Remove the tag so it does not linger past this check.
	git_run(repo_dir, ["tag", "-d", tag_name])
	assert raised, "ensure_tag_free did not raise when the tag existed"
	print(f"  PASS [{label}]: ensure_tag_free raises when the v{RELEASE_VERSION} tag exists")


def check_license_guard_raises_without_license(
	make_release: object, no_license_dir: str, label: str,
) -> None:
	"""ensure_committed_license raises in a committed repo lacking a LICENSE."""
	build_release_repo(no_license_dir, with_license=False)
	saved_cwd = os.getcwd()
	os.chdir(no_license_dir)
	raised = False
	try:
		make_release.ensure_committed_license()
	except RuntimeError:
		raised = True
	finally:
		os.chdir(saved_cwd)
	assert raised, "ensure_committed_license did not raise without a committed LICENSE"
	print(f"  PASS [{label}]: ensure_committed_license raises when HEAD has no LICENSE")


#============================================
# Main
#============================================

def main() -> None:
	"""Build a temp repo and run every make_release check end to end."""
	# tempfile.mkdtemp gives a unique, self-cleaning base (no hardcoded /tmp).
	base_dir = tempfile.mkdtemp(prefix="make_release_e2e_")
	repo_dir = os.path.join(base_dir, "repo")
	no_license_dir = os.path.join(base_dir, "nolicense")
	saved_cwd = os.getcwd()
	print("=== make_release E2E ===")
	print(f"local checkout: {LOCAL_CHECKOUT}")
	print(f"temp repo:      {repo_dir}")
	try:
		build_release_repo(repo_dir, with_license=True)
		make_release = load_make_release(repo_dir)
		os.chdir(repo_dir)

		# Tags must start empty and stay empty across the no-mutation flows.
		tags_before = list_tags(repo_dir)

		check_preconditions_pass(make_release, "clean")
		check_no_notes_prints_prompt_and_builds_nothing(make_release, repo_dir, "no-notes")

		tags_after_no_notes = list_tags(repo_dir)
		assert tags_after_no_notes == tags_before, "no-notes run changed git tags"

		check_write_builds_and_verifies_archives(make_release, repo_dir, "write")

		tags_after_write = list_tags(repo_dir)
		assert tags_after_write == tags_before, "--write run changed git tags"
		print("  PASS [observability]: git tag --list unchanged across dry-run and --write")

		# License guard runs in its own repo; restore cwd to the primary repo after.
		check_license_guard_raises_without_license(make_release, no_license_dir, "no-license")
		os.chdir(repo_dir)
		# Tag guard mutates tags intentionally, so run it last and clean up its tag.
		check_tag_guard_raises_when_tag_exists(make_release, repo_dir, "tag-exists")
	finally:
		os.chdir(saved_cwd)
		shutil.rmtree(base_dir, ignore_errors=True)

	print("\n=== SUMMARY ===")
	print("PASS: all make_release E2E checks succeeded.")


if __name__ == "__main__":
	main()
