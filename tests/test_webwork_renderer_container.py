"""Policy tests for the private, source-pinned upstream WeBWorK stack."""

# Standard Library
import pathlib
import re

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
COMPOSE_PATH = REPO_ROOT / "containers" / "compose.yaml"
ENV_EXAMPLE_PATH = REPO_ROOT / "containers" / "env.example"
LAUNCHER_PATH = REPO_ROOT / "launch_local_stack.sh"
WEBWORK_DIR = REPO_ROOT / "containers" / "webwork"


#============================================
def _service_block(compose: str, service: str) -> str:
	"""Return one top-level Compose service block."""
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
def test_webwork_services_are_private_and_separate_from_ple_storage() -> None:
	"""The renderer reaches only its upstream database, never PLE data services."""
	compose = COMPOSE_PATH.read_text()
	override = (REPO_ROOT / "containers" / "compose.webwork.yaml").read_text()
	renderer = _service_block(compose, "webwork-renderer")
	database = _service_block(compose, "webwork-db")
	active_renderer = "\n".join(line.split("#", 1)[0] for line in renderer.splitlines())
	active_database = "\n".join(line.split("#", 1)[0] for line in database.splitlines())

	assert 'profiles: ["webwork"]' in renderer
	assert 'profiles: ["webwork"]' in database
	assert not re.search(r"^    ports:", active_renderer, re.MULTILINE)
	assert not re.search(r"^    ports:", active_database, re.MULTILINE)
	assert re.search(r"^    volumes:\n(?:.*\n)*?      - ple_webwork_courses:/opt/webwork/courses$", renderer, re.MULTILINE)
	assert active_renderer.count("type: bind") == 2
	for forbidden in ("POSTGRES_", "MINIO_", "AWS_", "PLE_S3_", "DATABASE_URL"):
		assert forbidden not in active_renderer
		assert forbidden not in active_database
	assert "- renderer_private" in renderer
	assert "- webwork_db_private" in renderer
	assert "ple_webwork_courses:/opt/webwork/courses" in renderer
	assert "networks:\n      - webwork_db_private" in database
	assert re.search(r"^  renderer_private:\n    internal: true$", compose, re.MULTILINE)
	assert re.search(r"^  webwork_db_private:\n    internal: true$", compose, re.MULTILINE)
	for service in (renderer, database):
		assert "cap_drop:\n      - ALL" in service
		for capability in ("CHOWN", "SETUID", "SETGID", "DAC_OVERRIDE"):
			assert f"      - {capability}" in service


#============================================
def test_container_build_uses_verified_immutable_public_source_revisions() -> None:
	"""The image must verify exact upstream commits rather than accept mutable tags."""
	renderer = _service_block(COMPOSE_PATH.read_text(), "webwork-renderer")
	containerfile = (WEBWORK_DIR / "Containerfile").read_text()
	env_example = ENV_EXAMPLE_PATH.read_text()
	for token in (
		"PLE_WEBWORK_BASE_IMAGE_SHA256",
		"PLE_WEBWORK2_GIT_URL",
		"PLE_WEBWORK2_GIT_SHA",
		"PLE_WEBWORK_PG_GIT_URL",
		"PLE_WEBWORK_PG_GIT_SHA",
		"PLE_WEBWORK_MARIADB_IMAGE_SHA256",
	):
		assert token in renderer or token in env_example
		assert f"{token}=" in env_example
	assert 'git -C webwork2 fetch --depth 1 origin "$WEBWORK2_GIT_SHA"' in containerfile
	assert 'git -C pg fetch --depth 1 origin "$PG_GIT_SHA"' in containerfile
	assert 'grep -Ec \'^[0-9a-f]{40}$\'' in containerfile
	assert 'test "$(git -C webwork2 rev-parse HEAD)" = "$WEBWORK2_GIT_SHA"' in containerfile
	assert 'test "$(git -C pg rev-parse HEAD)" = "$PG_GIT_SHA"' in containerfile
	assert "test \"$WEBWORK2_GIT_URL\" = 'https://github.com/openwebwork/webwork2.git'" in containerfile
	assert "test \"$PG_GIT_URL\" = 'https://github.com/openwebwork/pg.git'" in containerfile
	assert "lthub/webwork" not in renderer
	assert "COPY --from=source /source/webwork2" in containerfile
	assert "COPY --from=source /source/pg" in containerfile
	assert "FROM docker.io/alpine/git@sha256:" in containerfile
	assert "FROM docker.io/library/node@sha256:" in containerfile
	assert "libemail-stuffer-perl" in containerfile
	assert "libtest-xml-perl" in containerfile
	assert "texlive-lang-arabic" in containerfile
	assert "texlive-lang-other" in containerfile
	assert "Perl::Tidy@20260204 Archive::Zip::SimpleZip Net::SAML2" in containerfile
	assert "npm install" in containerfile
	assert "locale-gen" in containerfile
	assert "RadioButtons" in (WEBWORK_DIR / "probe_render_rpc.sh").read_text()
	assert '"score"[[:space:]]*:[[:space:]]*100' in (WEBWORK_DIR / "probe_render_rpc.sh").read_text()


#============================================
def test_render_course_and_mojolicious_configuration_keep_authenticated_rpc() -> None:
	"""Provision only the source-render service account and forbid insecure RPC."""
	course = (WEBWORK_DIR / "course.conf").read_text()
	mojo = (WEBWORK_DIR / "webwork2.mojolicious.yml").read_text()
	bootstrap = (WEBWORK_DIR / "init_render_course.sh").read_text()
	entrypoint = (WEBWORK_DIR / "entrypoint.sh").read_text()
	probe = (WEBWORK_DIR / "probe_render_rpc.sh").read_text()

	assert "allow_unsecured_rpc: 0" in mojo
	assert "webservice_render_source" in course
	assert "login_proctor" in course
	assert "addcourse --users=" in bootstrap
	assert ",2\\n'" in bootstrap
	assert "--professors" not in bootstrap
	assert "exec \"$@\"" in entrypoint
	assert "site.conf.dist" in entrypoint
	assert "webwork2.mojolicious.dist.yml" in entrypoint
	assert "^secrets:$" in entrypoint
	assert "^pg_dir: /opt/webwork/pg$" in entrypoint
	assert "PLE_WEBWORK_MOJO_SECRET_FILE" in entrypoint
	assert "allow_unsecured_rpc: 0" in entrypoint
	assert "cat /opt/ple-webwork/site.conf >>" in entrypoint
	assert "sudo" in (WEBWORK_DIR / "Containerfile").read_text()
	assert "/webwork2/render_rpc" in probe
	assert "--request POST" in probe
	assert "courseID=" in probe
	assert "passwd=" in probe
	assert 'render_request "$response_file"' in probe
	assert "cat \"$response_file\"" not in probe


#============================================
def test_launcher_builds_and_probes_webwork_before_starting_ple_api() -> None:
	"""A selected renderer becomes a real prerequisite, not a nominal profile."""
	launcher = LAUNCHER_PATH.read_text()
	for setting in (
		"PLE_WEBWORK2_GIT_SHA",
		"PLE_WEBWORK_PG_GIT_SHA",
		"PLE_WEBWORK_DATABASE_PASSWORD",
		"PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE",
	):
		assert setting in launcher
	assert 'compose up -d --build webwork-db webwork-renderer' in launcher
	assert 'compose exec -T webwork-renderer /usr/local/bin/probe_render_rpc.sh' in launcher
	assert launcher.index("Building and provisioning private upstream WeBWorK") < launcher.index(
		"Building images and starting API, worker, and browser gateway"
	)


#============================================
def test_render_password_is_an_ignored_file_not_an_env_value() -> None:
	"""The launcher keeps the service password out of env.local and command output."""
	launcher = LAUNCHER_PATH.read_text()
	gitignore = (REPO_ROOT / ".gitignore").read_text()
	compose = COMPOSE_PATH.read_text()
	override = (REPO_ROOT / "containers" / "compose.webwork.yaml").read_text()
	assert "LOCAL_WEBWORK_SECRET_FILE=\"containers/.secrets/webwork_render_password\"" in launcher
	assert "containers/.secrets/" in gitignore
	assert "PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE" in launcher
	assert "PLE_WEBWORK_MOJO_SECRET_HOST_FILE" in launcher
	assert "write_env_value PLE_WEBWORK_RENDER_PASSWORD " not in launcher
	assert "openssl rand -base64 48" in launcher
	assert "PLE_WEBWORK_RENDER_PASSWORD_FILE: /run/ple-secrets/webwork_render_password" in override
	assert "PLE_WEBWORK_RENDER_PASSWORD:" not in compose
	assert "PLE_WEBWORK_MOJO_SECRET_FILE: /run/ple-secrets/webwork_mojolicious_secret" in compose
	assert "target: /run/ple-secrets/webwork_render_password" in compose
	assert "target: /run/ple-secrets/webwork_mojolicious_secret" in compose
	assert "secrets:" not in compose
	api = _service_block(override, "api")
	initializer = _service_block(compose, "webwork-api-secret-init")
	assert "webwork-api-secret-init:" in api
	assert "ple_webwork_api_runtime" in api
	assert "chown 10001:10001" in initializer
	assert "chmod 600" in initializer
	assert "network_mode: none" in initializer
	assert "docker.io/library/alpine@sha256:${PLE_SECRET_INIT_IMAGE_SHA256" in initializer
	assert "read_only: true" in initializer
	assert "cap_drop:\n      - ALL" in initializer
	assert "- CHOWN" in initializer and "- DAC_OVERRIDE" in initializer
	assert "no-new-privileges:true" in initializer
	assert "USER peptidyle" in (REPO_ROOT / "containers" / "Containerfile.api").read_text()
	native_api = _service_block(compose, "api")
	assert "PLE_WEBWORK_RENDER_PASSWORD_FILE" not in native_api
	assert "webwork-api-secret-init" not in native_api
	assert "compose.webwork.yaml" in launcher
	assert "compose rm -f webwork-api-secret-init" in launcher
	proof = (REPO_ROOT / "tests" / "e2e" / "e2e_webwork_api_secret_mode.sh").read_text()
	assert "--user 10001:10001" in proof
	assert "--user 10002:10002" in proof
	assert "PLE_SECRET_INIT_IMAGE_SHA256" in proof
	assert "set_default_env_value PLE_SECRET_INIT_IMAGE_SHA256" in launcher
	assert "PLE_SECRET_INIT_IMAGE_SHA256" in launcher
	assert "LOCAL_WEBWORK_MOJO_SECRET_FILE" in launcher
	assert "LOCAL_WEBWORK_PROVENANCE_FILE" in launcher
	assert "podman image inspect --format '{{.Id}}'" in launcher
	assert "tr '+/' '-_'" in launcher
	assert "reconcile_render_account.pl" in (WEBWORK_DIR / "init_render_course.sh").read_text()
	assert "information_schema.tables" in (WEBWORK_DIR / "init_render_course.sh").read_text()
	assert ".orphaned-" in (WEBWORK_DIR / "init_render_course.sh").read_text()
	assert "*[!A-Za-z0-9_-]*" in (WEBWORK_DIR / "init_render_course.sh").read_text()
	assert "must have mode 0600" in launcher
	assert "stat -f '%Lp'" in launcher
	assert "stat -c '%a'" in launcher
	assert "validate_webwork_secret_file()" in launcher
	assert 'validate_webwork_secret_file "$(env_value PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE)"' in launcher
	assert 'validate_webwork_secret_file "$(env_value PLE_WEBWORK_MOJO_SECRET_HOST_FILE)"' in launcher
	e2e = (REPO_ROOT / "tests" / "e2e" / "e2e_webwork_render_rpc.sh").read_text()
	assert "Podman read-only bind changed strict secret mode" in e2e
	assert "stat -c '%a' /run/ple-secrets/secret" in e2e
	assert "WebWork secret $secret_path must be at least 32 random bytes" in launcher
	assert "write_env_value PLE_WEBWORK_RENDERER_VERSION" in launcher
	assert "PLE_WEBWORK_RENDERER_BASE_URL \"http://webwork-renderer:8080/webwork2/\"" in launcher
	assert "PLE_WEBWORK_RENDERER_BASE_URL=http://webwork-renderer:8080/webwork2/" in ENV_EXAMPLE_PATH.read_text()
