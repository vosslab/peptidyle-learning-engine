"""Permanent trust-boundary checks for the private external PG renderer."""

# Standard Library
import pathlib
import re

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
COMPOSE_PATH = REPO_ROOT / "containers" / "compose.yaml"
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
def test_renderer_is_external_private_and_stateless() -> None:
	"""The PG engine remains replaceable and cannot become a second LMS."""
	compose = COMPOSE_PATH.read_text()
	renderer = _service_block(compose, "webwork-renderer")
	active = "\n".join(line.split("#", 1)[0] for line in renderer.splitlines())
	violations = [
		label
		for label, present in (
			("local renderer build", "build:" in active),
			("persistent renderer volume", "volumes:" in active),
			("host-published renderer port", "ports:" in active),
			("optional renderer profile", "profiles:" in active),
			("SQL-backed renderer", bool(re.search(r"mariadb|webwork-db|database", active, re.I))),
		)
		if present
	]

	assert violations == []
	assert "- renderer_private" in renderer and "renderer_private:\n    internal: true" in compose


#============================================
def test_api_receives_renderer_origin_but_not_renderer_signing_secrets() -> None:
	"""Only the renderer process receives its JWT signing configuration."""
	compose = COMPOSE_PATH.read_text()
	api = _service_block(compose, "api")
	renderer = _service_block(compose, "webwork-renderer")

	assert "PLE_WEBWORK_RENDERER_BASE_URL: http://webwork-renderer:3000/" in api
	assert "PLE_WEBWORK_PROBLEM_JWT_SECRET" not in api and "problemJWTsecret" in renderer


#============================================
def test_renderer_implementation_is_owned_by_the_external_project() -> None:
	"""PLE carries only its integration probe, not a copied renderer service."""
	compose = COMPOSE_PATH.read_text()
	renderer = _service_block(compose, "webwork-renderer")

	assert "image: ${PLE_WEBWORK_RENDERER_IMAGE" in renderer and "build:" not in renderer
	assert not (WEBWORK_DIR / "Containerfile").exists() and (WEBWORK_DIR / "probe_render_api.sh").is_file()
