"""Permanent deployment-policy checks for local file-backed development auth."""

# Standard Library
import json
import pathlib
import re

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
COMPOSE_PATH = REPO_ROOT / "containers" / "compose.yaml"
ENV_EXAMPLE_PATH = REPO_ROOT / "containers" / "env.example"
IDENTITIES_EXAMPLE_PATH = REPO_ROOT / "containers" / "local-identities.example.json"
GITIGNORE_PATH = REPO_ROOT / ".gitignore"


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
def test_api_explicitly_uses_opt_in_local_file_authentication() -> None:
	"""Compose selects only the guarded local provider and its internal path."""
	api = _service_block(COMPOSE_PATH.read_text(), "api")
	for requirement in (
		"PLE_AUTH_PROVIDER: local-file",
		'PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH: "1"',
		"PLE_LOCAL_AUTH_FILE: /run/ple/local-identities.json",
		"source: ${PLE_LOCAL_AUTH_HOST_FILE:?",
		"target: /run/ple/local-identities.json",
		"read_only: true",
		"bind:\n          create_host_path: false",
	):
		assert requirement in api, f"API local-auth contract is missing {requirement!r}"
	api_environment = api.split("    volumes:", 1)[0]
	assert "PLE_LOCAL_AUTH_HOST_FILE:" not in api_environment, (
		"The API must receive its fixed internal identity-file path, not an operator host path."
	)


#============================================
def test_local_identity_file_is_required_read_only_and_never_a_tracked_identity() -> None:
	"""A missing local file fails compose clearly; the committed example is inert."""
	compose = COMPOSE_PATH.read_text()
	env_example = ENV_EXAMPLE_PATH.read_text()
	assert "PLE_LOCAL_AUTH_HOST_FILE:?set PLE_LOCAL_AUTH_HOST_FILE" in compose
	assert "PLE_LOCAL_AUTH_HOST_FILE=" in env_example
	assert "credential_sha256" not in env_example
	assert json.loads(IDENTITIES_EXAMPLE_PATH.read_text()) == {"credentials": []}
	assert "containers/local-identities.json" in GITIGNORE_PATH.read_text()


#============================================
def test_renderer_receives_no_local_authentication_configuration() -> None:
	"""The private renderer remains unrelated to local API authentication."""
	renderer = _service_block(COMPOSE_PATH.read_text(), "webwork-renderer")
	assert "PLE_AUTH_" not in renderer
	assert "PLE_LOCAL_AUTH_" not in renderer
