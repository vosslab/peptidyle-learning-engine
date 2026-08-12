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
def _compose_service_lookup() -> str:
	"""Extract the project-aware service lookup without invoking the lifecycle."""
	launcher = LAUNCHER_PATH.read_text(encoding="ascii")
	start = launcher.index("compose_service_container_id() {")
	end = launcher.index("\nrequire_env_value() {")
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


#============================================
def test_renderer_probe_uses_the_selected_compose_project_service() -> None:
	"""The renderer probe receives the one container selected by this Compose invocation."""
	driver = _compose_service_lookup() + """
die() { printf '%s\\n' "$*" >&2; exit 1; }
compose() { [ "$1" = ps ] && [ "$2" = -q ] || exit 97; printf '%s\\n' api renderer; }
podman() {
  [ "$1" = container ] && [ "$2" = inspect ] || exit 98
  case "$5" in
    api) printf '%s\\n' api ;;
    renderer) printf '%s\\n' webwork-renderer ;;
    *) exit 99 ;;
  esac
}
compose_service_container_id webwork-renderer
"""
	result = subprocess.run(
		["bash", "-c", driver],
		check=True,
		capture_output=True,
		text=True,
	)

	assert result.stdout == "renderer\n"


#============================================
def test_renderer_probe_accepts_docker_compose_service_label() -> None:
	"""Docker Compose labels identify the service when they are the provider's labels."""
	driver = _compose_service_lookup() + """
die() { printf '%s\\n' "$*" >&2; exit 1; }
compose() { printf '%s\\n' docker_renderer; }
podman() {
  [ "$1" = container ] && [ "$2" = inspect ] || exit 98
  case "$4" in
    *io.podman.compose.service*) printf '%s\\n' '<no value>' ;;
    *com.docker.compose.service*) printf '%s\\n' webwork-renderer ;;
    *) exit 99 ;;
  esac
}
compose_service_container_id webwork-renderer
"""
	result = subprocess.run(
		["bash", "-c", driver],
		check=True,
		capture_output=True,
		text=True,
	)

	assert result.stdout == "docker_renderer\n"


#============================================
def test_renderer_probe_rejects_an_ambiguous_compose_service_lookup() -> None:
	"""A probe never chooses between multiple renderer containers."""
	driver = _compose_service_lookup() + """
die() { printf '%s\\n' "$*" >&2; exit 1; }
compose() { printf '%s\\n' first_renderer second_renderer; }
podman() { printf '%s\\n' webwork-renderer; }
compose_service_container_id webwork-renderer
"""
	result = subprocess.run(
		["bash", "-c", driver],
		check=False,
		capture_output=True,
		text=True,
	)

	assert result.returncode != 0
	assert "expected exactly one running PLE webwork-renderer container" in result.stderr
