"""Shared fixed production-auth target codec for live-demo lifecycle owners."""

# Standard Library
import base64
import dataclasses
import hashlib
import os
import pathlib
import secrets
from collections.abc import Mapping

# PyPI
import yaml

# local repo modules
import local_stack_control.disposable_stack_adapter
import local_stack_control.env_file
import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_control.process


POSTGRES_USER = "ple_live_demo_browser"
POSTGRES_DATABASE = "ple_live_demo_browser"
LOCAL_MORGAN_SYSADMIN_ACCOUNT_ID = "00000000-0000-0000-0000-000000000105"
REQUIRED_SELECTION_NAMES = (
	"PLE_WEBWORK_RENDERER_IMAGE",
	"PLE_WEBWORK_RENDERER_BASE_URL",
	"PLE_WEBWORK_RENDERER_ID",
	"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
	"PLE_WEBWORK_MAX_RESPONSE_BYTES",
	"PLE_GATEWAY_IMAGE_SHA256",
	"PLE_POSTGRES_IMAGE_SHA256",
	"PLE_MINIO_IMAGE_SHA256",
	"PLE_MINIO_MC_IMAGE_SHA256",
	"PLE_SECRET_INIT_IMAGE_SHA256",
)
FORBIDDEN_LOCAL_AUTH_SETTINGS = (
	"PLE_LOCAL_AUTH_HOST_FILE",
	"PLE_AUTH_PROVIDER",
	"PLE_LOCAL_AUTH_FILE",
)
LOCAL_IDENTITIES_CONTAINER_PATH = "/run/ple/local-identities.json"


class _ComposeRenderLoader(yaml.SafeLoader):
	"""Safely parse the one Compose removal tag retained by podman-compose."""


#============================================
def _compose_reset_constructor(
	_loader: _ComposeRenderLoader,
	_node: yaml.Node,
) -> None:
	"""Map a closed Compose !reset value to its semantic absence."""
	return None


_ComposeRenderLoader.add_constructor("!reset", _compose_reset_constructor)


#============================================
def _safe_load_compose_render(rendered: str) -> object:
	"""Parse Compose output with the closed SafeLoader and dispose it promptly."""
	# The loader is a SafeLoader subclass; direct construction retains !reset while
	# avoiding the generic yaml.load entry point and its caller-selected loader API.
	loader = _ComposeRenderLoader(rendered)
	try:
		return loader.get_single_data()
	finally:
		loader.dispose()


@dataclasses.dataclass(frozen=True)
class LiveDemoPorts:
	"""Exact internal loopback ports owned by one fixed-stack lifecycle."""

	postgres: int
	minio_api: int
	minio_console: int
	gateway: int

	#============================================
	def as_tuple(self) -> tuple[int, int, int, int]:
		"""Return the stable port order used by availability validation."""
		return self.postgres, self.minio_api, self.minio_console, self.gateway


@dataclasses.dataclass(frozen=True)
class LiveDemoTarget:
	"""Private target locators and public origin for one closed profile."""

	profile: local_stack_control.models.LiveDemoProfile
	manifest_path: pathlib.Path
	environment_path: pathlib.Path
	capability_path: pathlib.Path
	origin: str
	ports: LiveDemoPorts
	project: str = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	owner: str = local_stack_control.models.LIVE_DEMO_BROWSER_OWNER


#============================================
def random_port(base: int) -> int:
	"""Select one bounded lifecycle-internal loopback port."""
	return base + secrets.randbelow(400)


#============================================
def random_ports() -> LiveDemoPorts:
	"""Select the four private ports for one lease-held target generation."""
	ports = LiveDemoPorts(
		random_port(53500),
		random_port(54000),
		random_port(54500),
		random_port(55000),
	)
	return ports


#============================================
def ports_from_tuple(values: tuple[int, int, int, int]) -> LiveDemoPorts:
	"""Convert the owner's injected deterministic port selection to the typed ABI."""
	ports = LiveDemoPorts(*values)
	validate_ports(ports)
	return ports


#============================================
def validate_ports(ports: LiveDemoPorts) -> None:
	"""Require distinct non-privileged TCP ports before writing private state."""
	# ASVS 2.2.1: positive validation constrains every security-relevant port.
	values = ports.as_tuple()
	if any(not isinstance(value, int) or isinstance(value, bool) for value in values):
		raise local_stack_control.models.ControllerError("live-demo target ports are invalid")
	if any(value < 1024 or value > 65535 for value in values) or len(set(values)) != len(values):
		raise local_stack_control.models.ControllerError("live-demo target ports are invalid")


#============================================
def require_safe_selections(selections: Mapping[str, str]) -> None:
	"""Require every live-demo selection to have a complete line-safe value."""
	# ASVS 2.2.1: allow only the documented ASCII value shape at this boundary.
	for name in REQUIRED_SELECTION_NAMES:
		if name not in selections:
			raise local_stack_control.models.ControllerError(
				"live-demo target selections omit " + name
			)
		value = selections[name]
		if not isinstance(value, str) or value == "" or value.strip() != value:
			raise local_stack_control.models.ControllerError(
				"live-demo target selection is unsafe: " + name
			)
		if "\n" in value or "\r" in value or "\x00" in value:
			raise local_stack_control.models.ControllerError(
				"live-demo target selection is unsafe: " + name
			)
		try:
			value.encode("ascii")
		except UnicodeEncodeError as error:
			raise local_stack_control.models.ControllerError(
				"live-demo target selection is unsafe: " + name
			) from error


#============================================
def random_secret32() -> str:
	"""Return one unpadded base64url encoding of exactly 32 random bytes."""
	return base64.urlsafe_b64encode(secrets.token_bytes(32)).decode("ascii").rstrip("=")


#============================================
def _write_private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one exact current-user mode-0600 file without following replacements."""
	# ASVS 5.3.2 and 13.3.2: the lease supplies trusted paths and least-privilege mode.
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		output.write(content.encode("ascii") if isinstance(content, str) else content)


#============================================
def write_private_target(
	directory: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
	ports: LiveDemoPorts,
	selections: Mapping[str, str],
) -> LiveDemoTarget:
	"""Write one fixed owner/project target with a closed production-auth profile."""
	policy = local_stack_control.models.live_demo_profile_policy(profile)
	validate_ports(ports)
	require_safe_selections(selections)
	if not directory.is_dir() or directory.is_symlink():
		raise local_stack_control.models.ControllerError(
			"live-demo target directory must be an existing private directory"
		)
	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	_write_private_file(capability_path, capability)
	capability_digest = hashlib.sha256(capability).hexdigest()
	invitation_path = directory / "invitation-secret"
	question_path = directory / "question-id-secret"
	_write_private_file(invitation_path, random_secret32())
	_write_private_file(question_path, random_secret32())
	renderer_version_path = directory / "question-renderer-version"
	environment_path = directory / "env.local"
	application_image_setting = ""
	if policy.application_image is not None:
		application_image_setting = f"PLE_APPLICATION_IMAGE={policy.application_image}\n"
	# ASVS 13.3.1: generated credentials exist only in the private runtime file.
	environment_content = (
		f"POSTGRES_USER={POSTGRES_USER}\nPOSTGRES_PASSWORD={secrets.token_hex(24)}\n"
		f"POSTGRES_DB={POSTGRES_DATABASE}\nPLE_POSTGRES_HOST_PORT={ports.postgres}\n"
		"MINIO_ROOT_USER=ple-live-demo-browser\n"
		f"MINIO_ROOT_PASSWORD={secrets.token_hex(24)}\n"
		f"PLE_MINIO_API_HOST_PORT={ports.minio_api}\n"
		f"PLE_MINIO_CONSOLE_HOST_PORT={ports.minio_console}\n"
		f"PLE_GATEWAY_HOST_PORT={ports.gateway}\n"
		f"PLE_LOCAL_AUTOMATED_GRADING_PASSWORD={secrets.token_hex(24)}\n"
		f"PLE_E2E_OWNER={local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
		f"PLE_PUBLIC_ASSET_BASE_URL=https://localhost:{ports.gateway}/public-assets\n"
		"PLE_WEBAUTHN_RP_ID=localhost\n"
		"PLE_WEBAUTHN_RP_NAME=Peptidyle Learning Engine\n"
		f"PLE_WEBAUTHN_ORIGIN=https://localhost:{ports.gateway}\n"
		"PLE_TRUSTED_PROXY_CIDRS=172.30.255.0/29\n"
		"PLE_STORAGE_TOPOLOGY=disposable-local\n"
		f"PLE_INVITATION_TOKEN_SECRET_HOST_FILE={invitation_path}\n"
		f"PLE_QUESTION_ID_SECRET_HOST_FILE={question_path}\n"
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID=00000000-0000-0000-0000-000000000101\n"
		"PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID=00000000-0000-0000-0000-000000000102\n"
		"PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID=00000000-0000-0000-0000-000000000103\n"
		"PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID=00000000-0000-0000-0000-000000000104\n"
		f"PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID={LOCAL_MORGAN_SYSADMIN_ACCOUNT_ID}\n"
		f"PLE_WEBWORK_RENDERER_IMAGE={selections['PLE_WEBWORK_RENDERER_IMAGE']}\n"
		f"PLE_WEBWORK_RENDERER_BASE_URL={selections['PLE_WEBWORK_RENDERER_BASE_URL']}\n"
		f"PLE_WEBWORK_RENDERER_ID={selections['PLE_WEBWORK_RENDERER_ID']}\n"
		f"PLE_WEBWORK_RENDERER_VERSION_FILE={renderer_version_path}\n"
		f"PLE_WEBWORK_PROBLEM_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_SESSION_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS={selections['PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS']}\n"
		f"PLE_WEBWORK_MAX_RESPONSE_BYTES={selections['PLE_WEBWORK_MAX_RESPONSE_BYTES']}\n"
		f"PLE_GATEWAY_IMAGE_SHA256={selections['PLE_GATEWAY_IMAGE_SHA256']}\n"
		f"PLE_POSTGRES_IMAGE_SHA256={selections['PLE_POSTGRES_IMAGE_SHA256']}\n"
		f"PLE_MINIO_IMAGE_SHA256={selections['PLE_MINIO_IMAGE_SHA256']}\n"
		f"PLE_MINIO_MC_IMAGE_SHA256={selections['PLE_MINIO_MC_IMAGE_SHA256']}\n"
		f"PLE_SECRET_INIT_IMAGE_SHA256={selections['PLE_SECRET_INIT_IMAGE_SHA256']}\n"
		f"{application_image_setting}"
		f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
	)
	_write_private_file(environment_path, environment_content)
	manifest_path = directory / "disposable.manifest"
	manifest_content = (
		f"OWNER={local_stack_control.models.LIVE_DEMO_BROWSER_OWNER}\n"
		f"PROJECT={local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT}\n"
		f"PROFILE={policy.profile.value}\n"
		f"ENV_FILE={environment_path}\n"
		f"CAPABILITY_FILE={capability_path}\n"
	)
	_write_private_file(manifest_path, manifest_content)
	result = LiveDemoTarget(
		profile=policy.profile,
		manifest_path=manifest_path,
		environment_path=environment_path,
		capability_path=capability_path,
		origin=f"https://localhost:{ports.gateway}/",
		ports=ports,
	)
	return result


#============================================
def _environment_retains_local_auth(environment: object) -> bool:
	"""Report whether a rendered Compose environment actively selects local auth."""
	if isinstance(environment, dict):
		return any(
			name in environment and environment[name] is not None
			for name in FORBIDDEN_LOCAL_AUTH_SETTINGS
		)
	if isinstance(environment, list):
		for item in environment:
			if not isinstance(item, str):
				continue
			if any(item == name or item.startswith(name + "=") for name in FORBIDDEN_LOCAL_AUTH_SETTINGS):
				return True
	return False


#============================================
def _volumes_retain_local_identities(volumes: object) -> bool:
	"""Report whether rendered service volumes mount the local identities file."""
	if not isinstance(volumes, list):
		return False
	for volume in volumes:
		if isinstance(volume, dict) and volume.get("target") == LOCAL_IDENTITIES_CONTAINER_PATH:
			return True
		if isinstance(volume, str):
			parts = volume.split(":")
			if volume == LOCAL_IDENTITIES_CONTAINER_PATH or LOCAL_IDENTITIES_CONTAINER_PATH in parts[1:]:
				return True
	return False


#============================================
def require_production_auth_topology(rendered: str) -> None:
	"""Require rendered service fields to omit active local-file authentication."""
	# ASVS 1.5.2: SafeLoader cannot construct caller-selected Python objects.
	try:
		topology = _safe_load_compose_render(rendered)
	except yaml.YAMLError as error:
		raise local_stack_control.models.ControllerError(
			"live-demo Compose render is not valid YAML"
		) from error
	if not isinstance(topology, dict) or not isinstance(topology.get("services"), dict):
		raise local_stack_control.models.ControllerError(
			"live-demo Compose render omits the service topology"
		)
	for service in topology["services"].values():
		if not isinstance(service, dict):
			continue
		if _environment_retains_local_auth(service.get("environment")):
			raise local_stack_control.models.ControllerError(
				"live-demo Compose render retained a local-auth setting"
			)
		if _volumes_retain_local_identities(service.get("volumes")):
			raise local_stack_control.models.ControllerError(
				"live-demo Compose render retained a local-auth setting"
			)


#============================================
def validate_production_auth_render(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	manifest_path: pathlib.Path,
) -> None:
	"""Require the exact fixed profile and a rendered topology without local auth."""
	manifest = local_stack_control.disposable_stack_adapter.load_manifest(repo_root, manifest_path)
	if (
		manifest.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
		or manifest.project != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		or manifest.live_demo_profile is None
	):
		raise local_stack_control.models.ControllerError(
			"live-demo target manifest does not select the fixed production-auth owner"
		)
	disposable = local_stack_control.disposable_stack_adapter.disposable_target(runner, repo_root, manifest)
	local_stack_control.lifecycle.bootstrap_default_state(disposable)
	values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	if any(name in values for name in FORBIDDEN_LOCAL_AUTH_SETTINGS):
		raise local_stack_control.models.ControllerError(
			"live-demo environment selected local-file authentication"
		)
	selected = disposable.target
	local_stack_control.lifecycle.validate_static(selected)
	local_stack_control.process.require_rootless_local_engine(runner, repo_root)
	rendered = local_stack_control.lifecycle.validate_compose(selected, runner, repo_root)
	require_production_auth_topology(rendered)
