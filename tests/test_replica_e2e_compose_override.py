"""Permanent static policy checks for the isolated replica-restart E2E stack."""

# Standard Library
import pathlib

# Third Party
import yaml

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
COMPOSE_PATH = REPO_ROOT / "containers" / "compose.yaml"
OVERRIDE_PATH = REPO_ROOT / "tests" / "e2e" / "compose.replica-e2e.yaml"
CONTAINERFILE_PATH = REPO_ROOT / "containers" / "Containerfile.api"
ENV_EXAMPLE_PATH = REPO_ROOT / "containers" / "env.example"


#============================================
def _compose(path: pathlib.Path) -> dict[str, object]:
	"""Parse a committed Compose file rather than relying on indentation text."""
	parsed = yaml.safe_load(path.read_text())
	assert isinstance(parsed, dict)
	return parsed


#============================================
def test_local_host_ports_are_loopback_parameterized_with_safe_defaults() -> None:
	"""An E2E project can choose isolated ports without exposing data services."""
	compose = COMPOSE_PATH.read_text()
	env_example = ENV_EXAMPLE_PATH.read_text()
	for setting, default, container_port in (
		("PLE_POSTGRES_HOST_PORT", "5432", "5432"),
		("PLE_MINIO_API_HOST_PORT", "9000", "9000"),
		("PLE_MINIO_CONSOLE_HOST_PORT", "9001", "9001"),
		("PLE_GATEWAY_HOST_PORT", "8080", "8080"),
	):
		assert f'"127.0.0.1:${{{setting}:-{default}}}:{container_port}"' in compose
		assert f"{setting}={default}" in env_example


#============================================
def test_replica_override_selects_only_the_explicit_test_build_and_toggle() -> None:
	"""Replica attribution is impossible in the normal image or ordinary stack."""
	override = _compose(OVERRIDE_PATH)
	services = override.get("services")
	assert isinstance(services, dict)
	assert set(services) == {"api"}, "the override must not introduce a seed service"
	api = services["api"]
	assert isinstance(api, dict)
	assert "ports" not in api, "API replicas remain private behind Caddy"
	assert "volumes" not in api, "the existing read-only local-auth bind stays authoritative"
	assert api["build"] == {"target": "e2e-observability"}
	assert api["environment"] == {"PLE_ENABLE_E2E_OBSERVABILITY": "1"}

	containerfile = CONTAINERFILE_PATH.read_text()
	assert "FROM builder AS e2e-observability-builder" in containerfile
	assert "cargo build --release --locked --bin server_core --features e2e-observability" in containerfile
	assert "FROM runtime-base AS e2e-observability" in containerfile
	assert "FROM runtime-base AS production" in containerfile
	assert "RUN cargo build --release --locked --bin server_core\n" in containerfile


#============================================
def test_override_does_not_relax_renderer_or_add_privileged_configuration() -> None:
	"""The native E2E uses the production security boundary unchanged."""
	override_text = OVERRIDE_PATH.read_text()
	for forbidden in (
		"webwork-renderer",
		"POSTGRES_",
		"MINIO_",
		"AWS_",
		"DATABASE_URL",
		"PLE_LOCAL_AUTH_",
	):
		assert forbidden not in override_text, f"override must not contain {forbidden!r}"
