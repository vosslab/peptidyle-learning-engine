"""Focused launcher tests for one effective local browser origin."""

# Standard Library
import pathlib
import subprocess
import tempfile

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
LAUNCHER_PATH = REPO_ROOT / "launch_local_stack.sh"


#============================================
def _gateway_functions() -> str:
	"""Extract the launcher helpers without invoking the launcher lifecycle."""
	launcher = LAUNCHER_PATH.read_text(encoding="ascii")
	start = launcher.index("env_value() {")
	end = launcher.index("\nrandom_hex() {")
	return launcher[start:end]


#============================================
def test_inherited_gateway_port_sets_matching_webauthn_origin_without_rewriting_env_file() -> None:
	"""A one-run loopback override reaches Compose and WebAuthn through the same origin."""
	with tempfile.TemporaryDirectory() as temporary_name:
		env_file = pathlib.Path(temporary_name) / "env.local"
		env_file.write_text(
			"PLE_GATEWAY_HOST_PORT=8080\nPLE_WEBAUTHN_ORIGIN=http://localhost:8080\n",
			encoding="ascii",
		)
		driver = _gateway_functions() + """
ENV_FILE="$1"
export PLE_GATEWAY_HOST_PORT=8081
configure_local_webauthn_origin
printf '%s\\n' "$PLE_WEBAUTHN_ORIGIN"
"""
		result = subprocess.run(
			["bash", "-c", driver, "gateway-config-test", str(env_file)],
			check=True,
			capture_output=True,
			text=True,
		)

		assert result.stdout == "http://localhost:8081\n"
		assert env_file.read_text(encoding="ascii") == (
			"PLE_GATEWAY_HOST_PORT=8080\nPLE_WEBAUTHN_ORIGIN=http://localhost:8080\n"
		)


#============================================
def test_launcher_uses_the_same_effective_port_for_collision_and_health() -> None:
	"""Bootstrap collision selection and final readiness cannot independently choose an origin."""
	launcher = LAUNCHER_PATH.read_text(encoding="ascii")

	assert 'configured_gateway_port="$(effective_gateway_port)"' in launcher
	assert "configure_local_webauthn_origin" in launcher
	assert 'gateway_port="$(effective_gateway_port)"' in launcher
