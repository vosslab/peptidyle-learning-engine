"""Launcher contract for the explicit disposable canonical teaching mode."""

import pathlib

import file_utils


LAUNCHER_PATH = pathlib.Path(file_utils.get_repo_root()) / "launch_local_stack.sh"


#============================================
def test_canonical_walkthrough_mode_is_explicit_and_uses_the_selected_env_parent() -> None:
	"""The runner mode owns local teaching artifacts without treating a path as a mode switch."""
	launcher = LAUNCHER_PATH.read_text(encoding="ascii")

	assert '--canonical-walkthrough)' in launcher
	assert 'LOCAL_RUNTIME_DIRECTORY="$(dirname "$ENV_FILE")"' in launcher
	assert '"$ENV_FILE" = "$LOCAL_ENV_FILE" ] || [ "$CANONICAL_WALKTHROUGH" -eq 1' in launcher
	assert 'canonical walkthrough gateway port ${configured_gateway_port} is occupied' in launcher
