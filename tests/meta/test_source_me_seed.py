"""Pin the source_me.sh seed invariant so the template does not silently drift.

The shipped seed must stay generic: sourcing it leaves PYTHONPATH empty (the
repo-root extension block ships commented out) while both Python runtime flags
are set. This is a behavior check, not a comment-string match, so it survives
comment rewording but fails if an active PYTHONPATH line is ever shipped.

Template-local: lives under tests/meta/, which the propagation walk skips
(skip_walk_dirs includes 'meta'), so this test does not ship to consumers.

The test assumes this machine's fixed ~/.bashrc as the expected local
environment and asserts only the seed's own guarantees, so unrelated bashrc
content cannot make it flaky.
"""

import os
import subprocess

import file_utils


#============================================
def source_seed_env() -> dict[str, str]:
	"""
	Source the repo's source_me.sh in a fresh bash and capture key env vars.

	Runs a bash subprocess that sources the seed and prints the three variables
	the seed contract governs. Output from sourcing ~/.bashrc is discarded so
	only the labeled marker lines reach stdout.

	Returns:
		dict[str, str]: Keys 'PYTHONPATH', 'PYTHONUNBUFFERED', and
			'PYTHONDONTWRITEBYTECODE' mapped to their post-source values (empty
			string when unset).
	"""
	repo_root = file_utils.get_repo_root()
	seed_path = os.path.join(repo_root, "source_me.sh")
	# Source the seed, silence its own chatter, then print each governed var on
	# its own labeled line so the parent parses exact values.
	script = ""
	script += f"source '{seed_path}' >/dev/null 2>&1\n"
	script += "printf 'PYTHONPATH=%s\\n' \"${PYTHONPATH-}\"\n"
	script += "printf 'PYTHONUNBUFFERED=%s\\n' \"${PYTHONUNBUFFERED-}\"\n"
	script += "printf 'PYTHONDONTWRITEBYTECODE=%s\\n' \"${PYTHONDONTWRITEBYTECODE-}\"\n"
	command = ["bash", "-c", script]
	result = subprocess.run(command, capture_output=True, text=True, cwd=repo_root)
	# Parse the labeled marker lines into a dict of var -> value.
	env = {}
	for line in result.stdout.splitlines():
		if "=" not in line:
			continue
		key, value = line.split("=", 1)
		env[key] = value
	return env


#============================================
def test_seed_leaves_pythonpath_empty() -> None:
	"""Sourcing the generic seed must leave PYTHONPATH empty (no active line)."""
	env = source_seed_env()
	assert env["PYTHONPATH"] == ""


#============================================
def test_seed_sets_python_runtime_flags() -> None:
	"""The seed must export both Python runtime optimization flags as 1."""
	env = source_seed_env()
	assert env["PYTHONUNBUFFERED"] == "1"
	assert env["PYTHONDONTWRITEBYTECODE"] == "1"
