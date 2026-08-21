#!/usr/bin/env python3
"""Drive the connected ordinary-site live-demo browser journey in one disposable stack."""

import base64
import hashlib
import json
import os
import pathlib
import secrets
import sys
from collections.abc import Mapping, Sequence

SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.consumer
import local_stack_control.env_file
import local_stack_control.live_demo_claim_context
import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process


POSTGRES_USER = "ple_live_demo_browser"
POSTGRES_DATABASE = "ple_live_demo_browser"
PRIVATE_STATE_RELATIVE_DIRECTORY = pathlib.Path("target") / "live-demo-browser"
PRIVATE_STATE_DIRECTORY_PREFIX = "run-"
LOCAL_TENANT_ID = "00000000-0000-0000-0000-000000000100"
LOCAL_INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
LOCAL_MARY_ID = "00000000-0000-0000-0000-000000000102"
LOCAL_JACK_ID = "00000000-0000-0000-0000-000000000103"
LOCAL_AVERY_ID = "00000000-0000-0000-0000-000000000104"
LOCAL_SYSADMIN_ID = "00000000-0000-0000-0000-000000000105"


class LiveDemoBrowserError(local_stack_control.models.ControllerError):
	"""A concise connected-browser infrastructure failure."""


#============================================
def repo_root() -> pathlib.Path:
	"""Return the checkout owning this disposable E2E lane."""
	result = pathlib.Path(__file__).resolve().parents[2]
	return result


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one exact mode-0600 private ASCII file."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		if isinstance(content, str):
			output.write(content.encode("ascii"))
		else:
			output.write(content)


#============================================
def random_port(base: int) -> int:
	"""Select one bounded owner-local loopback port."""
	result = base + secrets.randbelow(400)
	return result


#============================================
def canonical_secret32() -> str:
	"""Return one unpadded base64url encoding of exactly 32 random bytes."""
	encoded = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode("ascii")
	result = encoded.rstrip("=")
	return result


#============================================
def adapter_argv(action: str, manifest_path: pathlib.Path, arguments: Sequence[str] = ()) -> list[str]:
	"""Form one closed lifecycle-adapter invocation."""
	result = [sys.executable, "-m", "local_stack_control._consumer_cli", action, "--manifest", str(manifest_path)]
	result.extend(arguments)
	return result


#============================================
def run(runner: local_stack_control.process.CommandRunner, argv: list[str], root: pathlib.Path, environment: dict[str, str] | None = None) -> None:
	"""Stream one external boundary without reinterpreting its status."""
	returncode = runner.stream(argv, environment, root)
	if returncode != 0:
		raised = "live-demo browser E2E command failed: " + " ".join(argv)
		raise LiveDemoBrowserError(raised)


#============================================
def write_private_target(directory: pathlib.Path, postgres_port: int, minio_port: int, minio_console_port: int, gateway_port: int, selections: Mapping[str, str]) -> tuple[str, pathlib.Path, pathlib.Path]:
	"""Create the complete ordinary-stack environment with no test-double auth."""
	project = "ple-live-demo-browser-" + secrets.token_hex(6)
	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	private_file(capability_path, capability)
	capability_digest = hashlib.sha256(capability).hexdigest()
	invitation_path = directory / "invitation-secret"
	question_path = directory / "question-id-secret"
	private_file(invitation_path, canonical_secret32())
	private_file(question_path, canonical_secret32())
	renderer_provenance_path = directory / "webwork-renderer.provenance"
	claim_context_path = directory / "live-demo-sysadmin-claim-context.json"
	env_path = directory / "env.local"
	env_content = (
		f"POSTGRES_USER={POSTGRES_USER}\nPOSTGRES_PASSWORD={secrets.token_hex(24)}\n"
		f"POSTGRES_DB={POSTGRES_DATABASE}\nPLE_POSTGRES_HOST_PORT={postgres_port}\n"
		"MINIO_ROOT_USER=ple-live-demo-browser\n"
		f"MINIO_ROOT_PASSWORD={secrets.token_hex(24)}\nPLE_MINIO_API_HOST_PORT={minio_port}\n"
		f"PLE_MINIO_CONSOLE_HOST_PORT={minio_console_port}\nPLE_GATEWAY_HOST_PORT={gateway_port}\n"
		f"PLE_LOCAL_GRADER_PASSWORD={secrets.token_hex(24)}\n"
		f"PLE_PUBLIC_ASSET_BASE_URL=https://localhost:{gateway_port}/public-assets\n"
		"PLE_WEBAUTHN_RP_ID=localhost\nPLE_WEBAUTHN_RP_NAME=Peptidyle Learning Engine\n"
		f"PLE_WEBAUTHN_ORIGIN=https://localhost:{gateway_port}\n"
		"PLE_TRUSTED_PROXY_CIDRS=172.30.255.0/29\n"
		"PLE_STORAGE_TOPOLOGY=disposable-local\n"
		f"PLE_INVITATION_TOKEN_SECRET_HOST_FILE={invitation_path}\nPLE_QUESTION_ID_SECRET_HOST_FILE={question_path}\n"
		f"PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE={claim_context_path}\n"
		f"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID={LOCAL_INSTRUCTOR_ID}\nPLE_LIVE_DEMO_MARY_STUDENT_USER_ID={LOCAL_MARY_ID}\n"
		f"PLE_LIVE_DEMO_JACK_STUDENT_USER_ID={LOCAL_JACK_ID}\nPLE_LIVE_DEMO_AVERY_STUDENT_USER_ID={LOCAL_AVERY_ID}\n"
		f"PLE_LIVE_DEMO_SYSADMIN_USER_ID={LOCAL_SYSADMIN_ID}\n"
		f"PLE_WEBWORK_RENDERER_IMAGE={selections['PLE_WEBWORK_RENDERER_IMAGE']}\n"
		f"PLE_WEBWORK_RENDERER_BASE_URL={selections['PLE_WEBWORK_RENDERER_BASE_URL']}\n"
		f"PLE_WEBWORK_RENDERER_ID={selections['PLE_WEBWORK_RENDERER_ID']}\n"
		f"PLE_WEBWORK_PROVENANCE_FILE={renderer_provenance_path}\n"
		f"PLE_WEBWORK_PROBLEM_JWT_SECRET={secrets.token_hex(32)}\nPLE_WEBWORK_SESSION_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS={selections['PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS']}\n"
		f"PLE_WEBWORK_MAX_RESPONSE_BYTES={selections['PLE_WEBWORK_MAX_RESPONSE_BYTES']}\n"
		f"PLE_GATEWAY_IMAGE_SHA256={selections['PLE_GATEWAY_IMAGE_SHA256']}\n"
		f"PLE_POSTGRES_IMAGE_SHA256={selections['PLE_POSTGRES_IMAGE_SHA256']}\n"
		f"PLE_MINIO_IMAGE_SHA256={selections['PLE_MINIO_IMAGE_SHA256']}\n"
		f"PLE_MINIO_MC_IMAGE_SHA256={selections['PLE_MINIO_MC_IMAGE_SHA256']}\n"
		f"PLE_SECRET_INIT_IMAGE_SHA256={selections['PLE_SECRET_INIT_IMAGE_SHA256']}\n"
		f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
	)
	private_file(env_path, env_content)
	manifest_path = directory / "disposable.manifest"
	private_file(manifest_path, f"OWNER=live-demo-browser\nPROJECT={project}\nENV_FILE={env_path}\nCAPABILITY_FILE={capability_path}\n")
	result = project, manifest_path, claim_context_path
	return result


#============================================
def playwright_environment(input_path: pathlib.Path) -> dict[str, str]:
	"""Pass only the closed private browser-input path to Playwright."""
	environment = local_stack_control.process.current_environment()
	for name in tuple(environment):
		if name.startswith("PLE_") or name in ("AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN"):
			environment.pop(name)
	environment["PLE_LIVE_DEMO_BROWSER_REQUIRED"] = "1"
	environment["PLE_LIVE_DEMO_BROWSER_INPUT_FILE"] = str(input_path)
	return environment


#============================================
def write_browser_input(path: pathlib.Path, gateway_port: int, claim_context_path: pathlib.Path) -> None:
	"""Bridge only the installed generation-bound proof into Playwright's closed ABI."""
	context = local_stack_control.live_demo_claim_context.read_context(claim_context_path)
	if context.sysadmin_user_id != LOCAL_SYSADMIN_ID:
		raise LiveDemoBrowserError("installed live-demo claim context has the wrong Sysadmin account")
	content = json.dumps({"schemaVersion": 1, "baseUrl": f"https://localhost:{gateway_port}/", "sysadminOwnershipProof": context.ownership_proof}, separators=(",", ":"), ensure_ascii=True)
	private_file(path, content)


#============================================
def require_worker_ready(
	runner: local_stack_control.process.CommandRunner,
	manifest_path: pathlib.Path,
	root: pathlib.Path,
) -> None:
	"""Require the production worker's post-schema readiness receipt before Chromium starts."""
	result = runner.run(adapter_argv("read-evidence-logs", manifest_path), cwd=root)
	if not result.ok() or "peptidyle worker ready with 6 supported job families" not in (
		result.stdout + result.stderr
	):
		raise LiveDemoBrowserError("live-demo worker did not reach its production-ready state")


#============================================
def validate_live_compose_render(
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	manifest_path: pathlib.Path,
) -> None:
	"""Parse the exact live provider topology before any disposable service starts."""
	manifest = local_stack_control.consumer.load_manifest(root, manifest_path)
	disposable = local_stack_control.consumer.disposable_target(runner, root, manifest)
	local_stack_control.lifecycle.bootstrap_default_state(disposable)
	values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	if "PLE_LOCAL_AUTH_HOST_FILE" in values:
		raise LiveDemoBrowserError("live-demo environment unexpectedly selected local-file authentication")
	rendered = local_stack_control.lifecycle.validate_lifecycle(disposable, runner, root)
	if "PLE_LOCAL_AUTH_HOST_FILE" in rendered or "/run/ple/local-identities.json" in rendered:
		raise LiveDemoBrowserError("live-demo Compose render retained a local-auth bind or setting")


#============================================
def main() -> None:
	"""Launch, install, execute the sole journey command, and cleanup exactly."""
	root = repo_root()
	selections = local_stack_control.env_file.canonical_stack_selections(root)
	runner = local_stack_control.process.SubprocessRunner()
	ports = (random_port(53500), random_port(54000), random_port(54500), random_port(55000))
	local_stack_control.process.require_available_loopback_ports(ports, runner, root)
	state = local_stack_control.private_state.prepare(root, PRIVATE_STATE_RELATIVE_DIRECTORY, PRIVATE_STATE_DIRECTORY_PREFIX)
	project, manifest_path, claim_context_path = write_private_target(state.directory, *ports, selections)
	input_path = state.directory / "playwright-input.json"
	started = False
	try:
		print("Live-demo browser E2E: parsing the production-auth Compose topology")
		validate_live_compose_render(runner, root, manifest_path)
		print("Live-demo browser E2E: starting an isolated ordinary PLE stack")
		started = True
		run(runner, adapter_argv("launch", manifest_path, ["--timeout-seconds", "240"]), root)
		require_worker_ready(runner, manifest_path, root)
		write_browser_input(input_path, ports[3], claim_context_path)
		print("Live-demo browser E2E: executing the connected ordinary-site journey")
		run(runner, ["npx", "playwright", "test", "tests/playwright/e2e/live_demo.spec.ts", "--workers=1"], root, playwright_environment(input_path))
		print("Live-demo browser E2E: PASS")
	finally:
		if started:
			try:
				run(runner, adapter_argv("cleanup", manifest_path), root)
			except LiveDemoBrowserError:
				print(f"Live-demo browser E2E: cleanup failed; retained private state {state.directory}", file=sys.stderr)
				raise
			state.remove()
		else:
			state.remove()


if __name__ == "__main__":
	main()
