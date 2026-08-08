"""Permanent deployment-policy checks for the private WeBWorK renderer."""

# Standard Library
import pathlib
import re

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
COMPOSE_PATH = REPO_ROOT / "containers" / "compose.yaml"
ENV_EXAMPLE_PATH = REPO_ROOT / "containers" / "env.example"


#============================================
def _service_block(compose: str, service: str) -> str:
	"""Return one top-level compose service block without parsing a dependency."""
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
def test_renderer_is_private_and_cannot_receive_learning_data_access() -> None:
	"""Keep PG execution isolated from public, database, and object-store surfaces."""
	renderer = _service_block(COMPOSE_PATH.read_text(), "webwork-renderer")
	active_configuration = "\n".join(line.split("#", 1)[0] for line in renderer.splitlines())
	forbidden = ("DATABASE_URL", "POSTGRES_", "MINIO_", "AWS_")
	assert not re.search(r"^    (?:ports|volumes):", active_configuration, re.MULTILINE), (
		"The private renderer must not have host ports or persistent mounts."
	)
	assert all(token not in active_configuration for token in forbidden), (
		"The private renderer must receive source bytes only over its internal HTTP "
		f"contract, not host ports, mounts, database access, or object credentials: {renderer}"
	)
	assert re.search(r"^    networks:\n      - renderer_private$", renderer, re.MULTILINE), (
		"The renderer must join only the internal renderer_private network."
	)
	for requirement in ("cap_drop:\n      - ALL", "security_opt:\n      - no-new-privileges:true"):
		assert requirement in renderer, (
			"The renderer must drop Linux capabilities and prohibit privilege escalation: "
			f"missing {requirement!r}"
		)


#============================================
def test_renderer_requires_a_pinned_contract_and_enforced_runtime_bounds() -> None:
	"""Require an immutable shipped image plus enforced resource and request bounds."""
	renderer = _service_block(COMPOSE_PATH.read_text(), "webwork-renderer")
	for requirement in (
		"${PLE_WEBWORK_RENDERER_IMAGE_REPOSITORY:?",
		"${PLE_WEBWORK_RENDERER_IMAGE_SHA256:?",
		"@sha256:",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS:",
		"read_only: true",
		"user: \"65532:65532\"",
		"cpus:",
		"mem_limit:",
		"pids_limit:",
		"healthcheck:",
		"timeout:",
		"${PLE_WEBWORK_RENDERER_HEALTHCHECK:?",
	):
		assert requirement in renderer, f"renderer policy is missing {requirement!r}"
	user = re.search(r'^    user: (.+)$', renderer, re.MULTILINE)
	assert user is not None, "renderer must declare an explicit non-root user"
	assert "${" not in user.group(1), "renderer user must not be environment-configurable"
	assert user.group(1).strip('"') not in {"0", "0:0", "root"}, (
		"renderer must never run as root"
	)
	assert re.search(
		r"^  renderer_private:\n    internal: true$", COMPOSE_PATH.read_text(), re.MULTILINE
	), "renderer network must be compose-internal"
	assert "PLE_WEBWORK_RENDERER_IMAGE_REPOSITORY=" in ENV_EXAMPLE_PATH.read_text(), (
		"Operators need an explicit required image input rather than a hidden latest tag."
	)


#============================================
def test_api_receives_only_the_private_renderer_client_contract() -> None:
	"""Wire API-to-renderer HTTP privately without coupling API availability."""
	compose = COMPOSE_PATH.read_text()
	api = _service_block(compose, "api")
	assert "webwork-renderer:" not in api.split("    environment:", 1)[0], (
		"A renderer outage must not block API startup or native-question routes."
	)
	for requirement in (
		"PLE_WEBWORK_RENDERER_BASE_URL:",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS:",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES:",
		"- renderer_private",
	):
		assert requirement in api, f"API renderer client wiring is missing {requirement!r}"
	assert "http://webwork-renderer:" in ENV_EXAMPLE_PATH.read_text(), (
		"The documented renderer URL must resolve through private Compose DNS."
	)
	for setting in (
		"PLE_WEBWORK_RENDERER_BASE_URL=",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS=",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES=",
	):
		assert setting in ENV_EXAMPLE_PATH.read_text(), (
			f"Operators need documented server-only renderer client setting {setting!r}."
		)
	assert "PLE_WEBWORK_CONNECT_TIMEOUT_SECONDS" not in compose
	assert "PLE_WEBWORK_CONNECT_TIMEOUT_SECONDS" not in ENV_EXAMPLE_PATH.read_text()


#============================================
def test_api_has_all_three_storage_buckets_and_a_public_asset_origin() -> None:
	"""Keep content, student records, and temporary processing explicit at runtime."""
	api = _service_block(COMPOSE_PATH.read_text(), "api")
	for setting in (
		"PLE_CONTENT_BUCKET: content",
		"PLE_STUDENT_RECORDS_BUCKET: student-records",
		"PLE_TEMP_PROCESSING_BUCKET: temp-processing",
		"PLE_PUBLIC_ASSET_BASE_URL:",
	):
		assert setting in api, f"API storage wiring is missing {setting!r}"
	assert "PLE_PUBLIC_ASSET_BASE_URL=" in ENV_EXAMPLE_PATH.read_text(), (
		"Operators need a documented public immutable-asset origin."
	)
