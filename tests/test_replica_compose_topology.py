"""Permanent Compose policy checks for API restart and replica behavior."""

# Standard Library
import pathlib
import re

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
COMPOSE_PATH = REPO_ROOT / "containers" / "compose.yaml"
CADDYFILE_PATH = REPO_ROOT / "containers" / "Caddyfile"
GATEWAY_CONTAINERFILE_PATH = REPO_ROOT / "containers" / "Containerfile.gateway"
ENV_EXAMPLE_PATH = REPO_ROOT / "containers" / "env.example"
LAUNCHER_PATH = REPO_ROOT / "launch_local_stack.sh"


#============================================
def _service_block(compose: str, service: str) -> str:
	"""Return one top-level Compose service block without parsing dependencies."""
	match = re.search(rf"^  {re.escape(service)}:\n", compose, flags=re.MULTILINE)
	if match is None:
		raise AssertionError(f"compose service {service!r} is missing")
	next_section = re.search(
		r"^(?:  [a-z][a-z0-9_-]*|volumes|networks|configs|secrets):\n",
		compose[match.end():],
		flags=re.MULTILINE,
	)
	end = len(compose) if next_section is None else match.end() + next_section.start()
	return compose[match.start():end]


#============================================
def test_api_replicas_have_no_host_port_and_gateway_is_the_only_listener() -> None:
	"""`--scale api=2` must never contend for a host port."""
	compose = COMPOSE_PATH.read_text()
	api = _service_block(compose, "api")
	gateway = _service_block(compose, "gateway")
	worker = _service_block(compose, "worker")
	renderer = _service_block(compose, "webwork-renderer")

	assert not re.search(r"^    ports:", api, re.MULTILINE), (
		"API replicas must stay internal; publish only the gateway."
	)
	assert not re.search(r"^    ports:", worker, re.MULTILINE), (
		"Workers stay private; a host port is not a monitoring or control interface."
	)
	assert not re.search(r"^    ports:", renderer, re.MULTILINE), (
		"The private renderer is reachable only through its internal service network."
	)
	assert '"127.0.0.1:${PLE_GATEWAY_HOST_PORT:-8080}:8080"' in gateway
	assert ":8080\"" in gateway
	assert "- gateway_api" in api
	assert re.search(
		r"^  gateway_api:\n    internal: true$", compose, re.MULTILINE
	), "Gateway/API traffic must have a dedicated internal network."


#============================================
def test_gateway_uses_digest_pinned_dynamic_dns_without_private_credentials() -> None:
	"""The hardened gateway dynamically discovers every API service address."""
	compose = COMPOSE_PATH.read_text()
	gateway = _service_block(compose, "gateway")
	caddyfile = CADDYFILE_PATH.read_text()
	containerfile = GATEWAY_CONTAINERFILE_PATH.read_text()
	active_gateway = "\n".join(line.split("#", 1)[0] for line in gateway.splitlines())

	for requirement in (
		"build:\n      context: ..",
		"dockerfile: containers/Containerfile.gateway",
		"CADDY_IMAGE: docker.io/library/caddy@sha256:${PLE_GATEWAY_IMAGE_SHA256:?",
		"${PLE_GATEWAY_IMAGE_SHA256:?",
		"source: ./Caddyfile",
		"target: /etc/caddy/Caddyfile",
		"source: ../dist",
		"target: /srv/peptidyle",
		"read_only: true",
		'user: "1000:1000"',
		"cap_drop:\n      - ALL",
		"security_opt:\n      - no-new-privileges:true",
		"cpus:",
		"mem_limit:",
		"pids_limit:",
		"http://127.0.0.1:8080/health",
		'"curl",',
		'"--fail",',
		'"--silent",',
	):
		assert requirement in gateway, f"gateway policy is missing {requirement!r}"
	for requirement in (
		"ARG CADDY_IMAGE",
		"FROM ${CADDY_IMAGE}",
		"RUN setcap -r /usr/bin/caddy",
		"USER 1000:1000",
	):
		assert requirement in containerfile, (
			f"gateway Containerfile policy is missing {requirement!r}"
		)
	for forbidden in (
		"DATABASE_URL",
		"POSTGRES_",
		"MINIO_",
		"AWS_",
		"PLE_AUTH_",
		"PLE_LOCAL_AUTH_",
		"PLE_WEBWORK_",
	):
		assert forbidden not in active_gateway, (
			f"gateway must not receive private service configuration: {forbidden}"
		)
	assert "dynamic a api 3000" in caddyfile
	assert "@api path /api /api/* /health" in caddyfile
	assert "root * /srv/peptidyle" in caddyfile
	assert "try_files {path} /index.html" in caddyfile
	assert "file_server" in caddyfile
	assert "refresh 2s" in caddyfile
	assert "lb_policy round_robin" in caddyfile
	assert "fail_duration 10s" in caddyfile
	assert "max_fails 1" in caddyfile
	assert "health_uri /health" in caddyfile
	assert "health_interval 5s" in caddyfile
	assert "health_timeout 2s" in caddyfile
	assert "health_status 200" in caddyfile
	assert "unhealthy_status 503" not in caddyfile, (
		"A feature-local 503 must not evict a healthy API replica from rotation."
	)
	assert "reverse_proxy api:3000" not in caddyfile
	assert "network_proxy none" in caddyfile
	assert "caddy\", \"validate\"" not in gateway, (
		"Gateway health must prove a proxied API is ready, not merely parse config."
	)


#============================================
def test_gateway_operator_inputs_are_documented_and_document_two_replica_startup() -> None:
	"""The gateway image is explicit; the loopback port has an E2E-safe default."""
	compose = COMPOSE_PATH.read_text()
	env_example = ENV_EXAMPLE_PATH.read_text()
	assert "${PLE_GATEWAY_IMAGE_SHA256:?" in compose, (
		"Compose must clearly reject a missing immutable gateway image digest."
	)
	assert "PLE_GATEWAY_IMAGE_SHA256=" in env_example
	assert "${PLE_GATEWAY_HOST_PORT:-8080}" in compose
	assert "PLE_GATEWAY_HOST_PORT=8080" in env_example
	assert "--scale api=2" in env_example


#============================================
def test_gateway_uses_the_public_8000_range_when_the_default_port_is_busy() -> None:
	"""The local launcher records a loopback-friendly fallback without exposing private services."""
	launcher = LAUNCHER_PATH.read_text()

	assert 'inherited_gateway_port="${PLE_GATEWAY_HOST_PORT:-}"' in launcher
	assert 'gateway_port="${inherited_gateway_port:-$configured_gateway_port}"' in launcher
	assert 'gateway_port="${gateway_port:-8080}"' in launcher
	assert 'candidate_port=8000' in launcher
	assert 'while [ "$candidate_port" -le 8099 ]; do' in launcher
	assert "no available local gateway port was found from 8000 through 8099" in launcher
	assert 'write_env_value PLE_GATEWAY_HOST_PORT "$available_gateway_port"' in launcher


#============================================
def test_native_infrastructure_uses_required_immutable_digests_and_a_read_only_major_guard() -> None:
	"""Local persistence cannot silently start under a mutable or wrong-major image."""
	compose = COMPOSE_PATH.read_text()
	env_example = ENV_EXAMPLE_PATH.read_text()

	for image, setting in (
		("docker.io/library/postgres", "PLE_POSTGRES_IMAGE_SHA256"),
		("quay.io/minio/minio", "PLE_MINIO_IMAGE_SHA256"),
		("quay.io/minio/mc", "PLE_MINIO_MC_IMAGE_SHA256"),
	):
		assert f"{image}@sha256:${{{setting}:?" in compose
		assert f"{setting}=" in env_example

	for mutable in (
		"docker.io/library/postgres:latest",
		"quay.io/minio/minio:latest",
		"quay.io/minio/mc:latest",
	):
		assert mutable not in compose

	guard = _service_block(compose, "postgres-major-guard")
	assert "ple_pgdata:/var/lib/postgresql/data:ro" in guard
	assert "network_mode: none" in guard
	assert "read_only: true" in guard
	assert '"$${actual_major}" != "17"' in guard
