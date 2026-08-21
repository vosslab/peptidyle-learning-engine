"""Typed, non-secret evidence for one disposable browser-suite invocation."""

import dataclasses
import json
import pathlib
import subprocess
from urllib.parse import urlparse

import local_stack_control.discovery
import local_stack_control.models
import local_stack_control.process


class BrowserSuiteOracleError(local_stack_control.models.ControllerError):
	"""One browser-suite receipt failed to prove its closed ownership contract."""


@dataclasses.dataclass(frozen=True)
class PrivateArtifact:
	"""Public metadata for one private owner artifact, without its content."""

	path: str
	mode: int
	size: int


@dataclasses.dataclass(frozen=True)
class ProcessIdentity:
	"""A process identity scoped to the running browser-suite owner."""

	pid: int
	parent_pid: int
	process_group_id: int


@dataclasses.dataclass(frozen=True)
class ProviderReceipt:
	"""The exact provider policy selected by the disposable lifecycle adapter."""

	name: str
	argv: tuple[str, ...]
	pod_provider_enabled: bool


@dataclasses.dataclass(frozen=True)
class SuiteInventory:
	"""One exact project, private-tree, and child-process inventory."""

	project: str
	containers: tuple[local_stack_control.models.ContainerResource, ...]
	volumes: tuple[local_stack_control.models.VolumeResource, ...]
	networks: tuple[local_stack_control.models.NetworkResource, ...]
	private_artifacts: tuple[PrivateArtifact, ...]
	owner_processes: tuple[ProcessIdentity, ...]
	provider: ProviderReceipt


@dataclasses.dataclass(frozen=True)
class OriginReceipt:
	"""Exact HTTPS origin evidence written by the Playwright process."""

	expected_origin: str
	observed_page_origins: tuple[str, ...]
	observed_request_origins: tuple[str, ...]


#============================================
def provider_receipt(target: local_stack_control.models.DisposableComposeTarget) -> ProviderReceipt:
	"""Bind receipt policy to the already validated lifecycle provider selection."""
	provider = target.target.provider
	expected_argv = (
		local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER,
		*local_stack_control.models.DISPOSABLE_PROVIDER_GLOBAL_ARGS,
	)
	if provider.name != local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER or provider.argv != expected_argv:
		raise BrowserSuiteOracleError("browser-suite lifecycle provider does not prove the no-pod policy")
	result = ProviderReceipt(provider.name, provider.argv, False)
	return result


#============================================
def private_artifacts(directory: pathlib.Path) -> tuple[PrivateArtifact, ...]:
	"""Return public metadata for files remaining below one private owner directory."""
	if not directory.exists():
		return ()
	if not directory.is_dir() or directory.is_symlink():
		raise BrowserSuiteOracleError("browser-suite private state is not a directory")
	items: list[PrivateArtifact] = []
	for path in sorted(directory.rglob("*")):
		if path.is_dir() and not path.is_symlink():
			continue
		if path.is_symlink() or not path.is_file():
			raise BrowserSuiteOracleError("browser-suite private state contains an unexpected entry")
		metadata = path.stat()
		items.append(PrivateArtifact(str(path.relative_to(directory)), metadata.st_mode & 0o777, metadata.st_size))
	return tuple(items)


#============================================
def owner_processes(sessions: tuple[local_stack_control.process.ProcessSession, ...]) -> tuple[ProcessIdentity, ...]:
	"""Return live members of owner-created process groups after parent reaping or reparenting."""
	groups = {item.process_group_id for item in sessions if item.process_group_id > 0}
	if not groups:
		return ()
	markers = {item.owner_marker for item in sessions if item.owner_marker != ""}
	probe = subprocess.Popen(
		["ps", "-axeww", "-o", "pid=,ppid=,pgid=,command="], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
	)
	stdout, _stderr = probe.communicate()
	if probe.returncode != 0:
		raise BrowserSuiteOracleError("browser-suite process inventory could not read process identities")
	rows: list[tuple[int, int, int]] = []
	marked_processes: set[int] = set()
	for line in stdout.splitlines():
		parts = line.split(maxsplit=3)
		if len(parts) != 4 or not all(item.isdigit() for item in parts[:3]):
			raise BrowserSuiteOracleError("browser-suite process inventory returned an invalid identity")
		pid, parent_pid, process_group_id = (int(item) for item in parts[:3])
		rows.append((pid, parent_pid, process_group_id))
		if any(marker in parts[3] for marker in markers):
			marked_processes.add(pid)
	result = processes_from_rows(rows, groups, probe.pid, marked_processes)
	return tuple(sorted(result, key=lambda item: item.pid))


#============================================
def processes_from_rows(
	rows: list[tuple[int, int, int]], groups: set[int], probe_pid: int, marked_processes: set[int]
) -> tuple[ProcessIdentity, ...]:
	"""Project a typed process snapshot into owner members without serializing command text."""
	result = tuple(
		ProcessIdentity(pid, parent_pid, process_group_id)
		for pid, parent_pid, process_group_id in rows
		if pid != probe_pid and (process_group_id in groups or pid in marked_processes)
	)
	return result


#============================================
def inventory_for(
	project: str,
	directory: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
	root: pathlib.Path,
	provider: ProviderReceipt,
	sessions: tuple[local_stack_control.process.ProcessSession, ...],
) -> SuiteInventory:
	"""Read only the exact labelled Compose project and its private owner state."""
	snapshot = local_stack_control.discovery.discover_snapshot(runner, root, project)
	result = SuiteInventory(
		project,
		snapshot.containers,
		snapshot.volumes,
		snapshot.networks,
		private_artifacts(directory),
		owner_processes(sessions),
		provider,
	)
	return result


#============================================
def empty_after_cleanup(inventory: SuiteInventory) -> None:
	"""Require this invocation's known resource classes to be absent after cleanup."""
	if inventory.provider.pod_provider_enabled:
		raise BrowserSuiteOracleError("live-demo browser provider must keep Podman pod ownership disabled")
	if inventory.containers or inventory.volumes or inventory.networks:
		raise BrowserSuiteOracleError("browser-suite cleanup left labelled project resources")
	if inventory.private_artifacts:
		raise BrowserSuiteOracleError("browser-suite cleanup left private artifacts")
	if inventory.owner_processes:
		raise BrowserSuiteOracleError("browser-suite cleanup left owner background processes")


#============================================
def canonical_origin(value: str) -> str:
	"""Require one exact HTTPS origin with no credentials, query, fragment, or path."""
	parsed = urlparse(value)
	if (
		parsed.scheme != "https"
		or parsed.hostname != "localhost"
		or parsed.port is None
		or parsed.username is not None
		or parsed.password is not None
		or parsed.path not in ("", "/")
		or parsed.params != ""
		or parsed.query != ""
		or parsed.fragment != ""
	):
		raise BrowserSuiteOracleError("browser-suite origin must be an exact localhost HTTPS origin")
	result = f"https://localhost:{parsed.port}"
	return result


#============================================
def origin_receipt_from_file(path: pathlib.Path, expected_origin: str) -> OriginReceipt:
	"""Decode strict public Playwright evidence and require exact expected-origin use."""
	# ASVS 3.5.1: state-changing browser journeys retain one trusted application origin.
	if not path.is_file() or path.is_symlink():
		raise BrowserSuiteOracleError("browser-suite origin receipt is missing")
	contents = path.read_text(encoding="ascii")
	try:
		value = json.loads(contents)
	except json.JSONDecodeError as error:
		raise BrowserSuiteOracleError("browser-suite origin receipt is not valid JSON") from error
	if not isinstance(value, dict) or set(value) != {"pageOrigins", "requestOrigins"}:
		raise BrowserSuiteOracleError("browser-suite origin receipt has an invalid shape")
	page_origins = value["pageOrigins"]
	request_origins = value["requestOrigins"]
	if (
		not isinstance(page_origins, list)
		or not isinstance(request_origins, list)
		or not page_origins
		or not request_origins
		or not all(isinstance(item, str) for item in [*page_origins, *request_origins])
	):
		raise BrowserSuiteOracleError("browser-suite origin receipt has an invalid shape")
	expected = canonical_origin(expected_origin)
	observed_pages = tuple(sorted(set(page_origins)))
	observed_requests = tuple(sorted(set(request_origins)))
	for item in (*observed_pages, *observed_requests):
		try:
			observed = canonical_origin(item)
		except BrowserSuiteOracleError as error:
			raise BrowserSuiteOracleError("Chromium observed an origin outside the production HTTPS gateway") from error
		if observed != expected:
			raise BrowserSuiteOracleError("Chromium observed an origin outside the production HTTPS gateway")
	result = OriginReceipt(expected, observed_pages, observed_requests)
	return result


#============================================
def unavailable_origin_receipt(expected_origin: str) -> OriginReceipt:
	"""Describe a child failure that ended before Chromium could produce evidence."""
	result = OriginReceipt(canonical_origin(expected_origin), (), ())
	return result


#============================================
def public_inventory(inventory: SuiteInventory) -> dict[str, object]:
	"""Project typed inventory into canonical public evidence without capability material."""
	containers = tuple(
		{
			"id": item.id,
			"names": item.names,
			"service": item.service,
			"state": item.state,
			"running": item.running,
		}
		for item in inventory.containers
	)
	volumes = tuple({"name": item.name} for item in inventory.volumes)
	networks = tuple({"name": item.name} for item in inventory.networks)
	artifacts = tuple(dataclasses.asdict(item) for item in inventory.private_artifacts)
	processes = tuple(dataclasses.asdict(item) for item in inventory.owner_processes)
	result: dict[str, object] = {
		"project": inventory.project,
		"containers": containers,
		"volumes": volumes,
		"networks": networks,
		"privateArtifacts": artifacts,
		"ownerProcesses": processes,
		"provider": {
			"name": inventory.provider.name,
			"argv": inventory.provider.argv,
			"podProviderEnabled": inventory.provider.pod_provider_enabled,
		},
	}
	return result
