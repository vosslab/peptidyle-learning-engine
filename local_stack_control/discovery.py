"""Label-driven Podman resource discovery and inspected container state."""

import json
import pathlib

import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


#============================================
def canonical_oci_image_id(value: object) -> str | None:
	"""Normalize Podman's bare configuration ID for later OCI identity comparison."""
	if not isinstance(value, str):
		return None
	if len(value) == 64 and all(character in "0123456789abcdef" for character in value):
		return "sha256:" + value
	return value


#============================================
def json_array(text: str, command_name: str) -> list[dict[str, object]]:
	"""Parse and validate a Podman JSON array."""
	if text.strip() == "":
		return []
	try:
		raw_items = json.loads(text)
	except json.JSONDecodeError as error:
		raise local_stack_control.models.ControllerError(
			f"{command_name} returned invalid JSON: {error.msg}"
		) from error
	if not isinstance(raw_items, list):
		raise local_stack_control.models.ControllerError(
			f"{command_name} returned JSON that is not an array"
		)
	items: list[dict[str, object]] = []
	for index, raw_item in enumerate(raw_items):
		if not isinstance(raw_item, dict):
			raise local_stack_control.models.ControllerError(
				f"{command_name} item {index} is not an object"
			)
		items.append(raw_item)
	return items


#============================================
def resource_labels(raw: dict[str, object]) -> dict[str, object]:
	"""Return labels from Podman JSON despite command-specific casing."""
	labels = raw.get("Labels")
	if labels is None:
		labels = raw.get("labels")
	if labels is None:
		return {}
	if not isinstance(labels, dict):
		raise local_stack_control.models.ControllerError("Podman resource labels are not an object")
	return labels


#============================================
def consistent_label(
	labels: dict[str, object],
	names: tuple[str, ...],
	resource_name: str,
) -> str | None:
	"""Decode aliases only when every present value agrees."""
	values = {str(labels[name]) for name in names if name in labels and labels[name] != ""}
	if len(values) == 0:
		return None
	if len(values) > 1:
		aliases = ", ".join(names)
		raise local_stack_control.models.ControllerError(
			f"{resource_name} has conflicting Compose labels: {aliases}"
		)
	result = next(iter(values))
	return result


#============================================
def required_text(raw: dict[str, object], name: str, owner: str) -> str:
	"""Return a required non-empty text field."""
	if name not in raw or not isinstance(raw[name], str) or raw[name] == "":
		raise local_stack_control.models.ControllerError(
			f"{owner} is missing required string field {name}"
		)
	result = str(raw[name])
	return result


#============================================
def optional_int(raw: dict[str, object], name: str, owner: str) -> int | None:
	"""Return an optional integer field without coercing invalid values."""
	if name not in raw or raw[name] is None:
		return None
	if not isinstance(raw[name], int):
		raise local_stack_control.models.ControllerError(
			f"{owner} field {name} is not an integer"
		)
	return int(raw[name])


#============================================
def parse_names(raw_names: object, owner: str) -> tuple[str, ...]:
	"""Parse the Podman container names field."""
	if isinstance(raw_names, str):
		return (raw_names,)
	if not isinstance(raw_names, list):
		raise local_stack_control.models.ControllerError(f"{owner} Names is not an array")
	names = tuple(str(name) for name in raw_names)
	return names


#============================================
def parse_ports(raw_ports: object, owner: str) -> tuple[local_stack_control.models.PortBinding, ...]:
	"""Parse Podman port bindings."""
	if raw_ports is None:
		return ()
	if not isinstance(raw_ports, list):
		raise local_stack_control.models.ControllerError(f"{owner} Ports is not an array")
	ports: list[local_stack_control.models.PortBinding] = []
	for index, raw_port in enumerate(raw_ports):
		if not isinstance(raw_port, dict):
			raise local_stack_control.models.ControllerError(
				f"{owner} port {index} is not an object"
			)
		try:
			host_port = int(raw_port["host_port"])
			container_port = int(raw_port["container_port"])
		except (KeyError, TypeError, ValueError) as error:
			raise local_stack_control.models.ControllerError(
				f"{owner} port {index} has invalid numeric fields"
			) from error
		port = local_stack_control.models.PortBinding(
			host_ip=str(raw_port.get("host_ip", "")),
			host_port=host_port,
			container_port=container_port,
			protocol=str(raw_port.get("protocol", "tcp")),
		)
		ports.append(port)
	return tuple(ports)


#============================================
def inspect_state(
	raw: dict[str, object],
	container_id: str,
) -> tuple[str, bool, int | None, str | None]:
	"""Read authoritative runtime state from one inspect record."""
	state = raw.get("State")
	if not isinstance(state, dict):
		raise local_stack_control.models.ControllerError(
			f"podman inspect {container_id} State is not an object"
		)
	status = required_text(state, "Status", f"podman inspect {container_id} State")
	running = state.get("Running")
	if not isinstance(running, bool):
		raise local_stack_control.models.ControllerError(
			f"podman inspect {container_id} State.Running is not boolean"
		)
	exit_code = optional_int(state, "ExitCode", f"podman inspect {container_id} State")
	health: str | None = None
	health_record = state.get("Health")
	if health_record is not None:
		if not isinstance(health_record, dict):
			raise local_stack_control.models.ControllerError(
				f"podman inspect {container_id} State.Health is not an object"
			)
		health_value = health_record.get("Status")
		if health_value is not None:
			if not isinstance(health_value, str):
				raise local_stack_control.models.ControllerError(
					f"podman inspect {container_id} State.Health.Status is not text"
				)
			health = health_value
	return status, running, exit_code, health


#============================================
def container_from_json(
	raw: dict[str, object],
	inspection: dict[str, object],
) -> local_stack_control.models.ContainerResource:
	"""Combine one inventory record with its inspect state."""
	container_id = required_text(raw, "Id", "podman ps item")
	labels = resource_labels(raw)
	project = consistent_label(labels, local_stack_control.models.COMPOSE_PROJECT_LABELS, container_id)
	service = consistent_label(labels, local_stack_control.models.COMPOSE_SERVICE_LABELS, container_id)
	state, running, exit_code, health = inspect_state(inspection, container_id)
	container = local_stack_control.models.ContainerResource(
		id=container_id,
		names=parse_names(raw.get("Names"), container_id),
		project=project,
		service=service,
		state=state,
		running=running,
		exit_code=exit_code,
		health=health,
		image=str(raw.get("Image", "")),
		ports=parse_ports(raw.get("Ports"), container_id),
		capability_digest=labels.get(local_stack_control.models.DISPOSABLE_CAPABILITY_LABEL)
		if isinstance(labels.get(local_stack_control.models.DISPOSABLE_CAPABILITY_LABEL), str)
		else None,
		image_id=canonical_oci_image_id(inspection.get("Image")),
	)
	return container


#============================================
def volume_from_json(raw: dict[str, object]) -> local_stack_control.models.VolumeResource:
	"""Convert one Podman volume record."""
	name = required_text(raw, "Name", "podman volume ls item")
	labels = resource_labels(raw)
	project = consistent_label(labels, local_stack_control.models.COMPOSE_PROJECT_LABELS, name)
	capability = labels.get(local_stack_control.models.DISPOSABLE_CAPABILITY_LABEL)
	return local_stack_control.models.VolumeResource(
		name=name,
		project=project,
		capability_digest=capability if isinstance(capability, str) else None,
	)


#============================================
def network_from_json(raw: dict[str, object]) -> local_stack_control.models.NetworkResource:
	"""Convert one Podman network record."""
	name_value = raw.get("name")
	if name_value is None:
		name_value = raw.get("Name")
	if not isinstance(name_value, str) or name_value == "":
		raise local_stack_control.models.ControllerError("podman network ls item has no name")
	labels = resource_labels(raw)
	project = consistent_label(labels, local_stack_control.models.COMPOSE_PROJECT_LABELS, name_value)
	capability = labels.get(local_stack_control.models.DISPOSABLE_CAPABILITY_LABEL)
	return local_stack_control.models.NetworkResource(
		name=name_value,
		project=project,
		capability_digest=capability if isinstance(capability, str) else None,
	)


#============================================
def run_json(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	repo_root: pathlib.Path,
) -> list[dict[str, object]]:
	"""Run one read-only Podman JSON command."""
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	result = runner.run(argv, environment, repo_root)
	if not result.ok():
		detail = result.stderr.strip()
		if detail == "":
			detail = f"command exited {result.returncode}"
		raise local_stack_control.models.ControllerError(
			f"{' '.join(argv)} failed: {detail}"
		)
	return json_array(result.stdout, " ".join(argv))


#============================================
def labelled_inventory(
	runner: local_stack_control.process.CommandRunner,
	base_argv: list[str],
	repo_root: pathlib.Path,
	identity_fields: tuple[str, ...],
	project: str | None = None,
) -> list[dict[str, object]]:
	"""Union records found through either supported Compose project label."""
	items_by_identity: dict[str, dict[str, object]] = {}
	for label_name in local_stack_control.models.COMPOSE_PROJECT_LABELS:
		label_filter = f"label={label_name}"
		if project is not None:
			label_filter = f"{label_filter}={project}"
		argv = [*base_argv, "--filter", label_filter, "--format", "json"]
		for item in run_json(runner, argv, repo_root):
			identity: str | None = None
			for field in identity_fields:
				value = item.get(field)
				if isinstance(value, str) and value != "":
					identity = value
					break
			if identity is None:
				raise local_stack_control.models.ControllerError(
					f"{' '.join(base_argv)} returned a resource without identity"
				)
			items_by_identity[identity] = item
	return list(items_by_identity.values())


#============================================
def discover_resources(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	project: str | None = None,
) -> tuple[
	tuple[local_stack_control.models.ContainerResource, ...],
	tuple[local_stack_control.models.VolumeResource, ...],
	tuple[local_stack_control.models.NetworkResource, ...],
]:
	"""Discover only resources bearing a Compose project label."""
	container_records = labelled_inventory(
		runner,
		["podman", "ps", "-a"],
		repo_root,
		("Id",),
		project,
	)
	inspect_by_id: dict[str, dict[str, object]] = {}
	if len(container_records) > 0:
		container_ids = [required_text(item, "Id", "podman ps item") for item in container_records]
		inspection_records = run_json(runner, ["podman", "inspect", *container_ids], repo_root)
		for inspection in inspection_records:
			inspection_id = required_text(inspection, "Id", "podman inspect item")
			if inspection_id in inspect_by_id:
				raise local_stack_control.models.ControllerError(
					f"podman inspect returned duplicate container {inspection_id}"
				)
			inspect_by_id[inspection_id] = inspection

	containers: list[local_stack_control.models.ContainerResource] = []
	for raw in container_records:
		container_id = required_text(raw, "Id", "podman ps item")
		if container_id not in inspect_by_id:
			raise local_stack_control.models.ControllerError(
				f"podman inspect omitted container {container_id}"
			)
		containers.append(container_from_json(raw, inspect_by_id[container_id]))

	volume_records = labelled_inventory(
		runner,
		["podman", "volume", "ls"],
		repo_root,
		("Name",),
		project,
	)
	network_records = labelled_inventory(
		runner,
		["podman", "network", "ls"],
		repo_root,
		("name", "Name"),
		project,
	)
	volumes = tuple(volume_from_json(raw) for raw in volume_records)
	networks = tuple(network_from_json(raw) for raw in network_records)
	return tuple(containers), volumes, networks


#============================================
def snapshot_for_project(
	project: str,
	containers: tuple[local_stack_control.models.ContainerResource, ...],
	volumes: tuple[local_stack_control.models.VolumeResource, ...],
	networks: tuple[local_stack_control.models.NetworkResource, ...],
) -> local_stack_control.models.ProjectSnapshot:
	"""Filter a discovered resource set by consistent project identity."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project=project,
		containers=tuple(item for item in containers if item.project == project),
		volumes=tuple(item for item in volumes if item.project == project),
		networks=tuple(item for item in networks if item.project == project),
	)
	return snapshot


#============================================
def discover_snapshot(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	project: str,
) -> local_stack_control.models.ProjectSnapshot:
	"""Discover the current labelled resources for one project."""
	resources = discover_resources(runner, repo_root, project)
	return snapshot_for_project(project, *resources)


#============================================
def all_projects(
	containers: tuple[local_stack_control.models.ContainerResource, ...],
	volumes: tuple[local_stack_control.models.VolumeResource, ...],
	networks: tuple[local_stack_control.models.NetworkResource, ...],
) -> tuple[str, ...]:
	"""Return the union of project identities across resource classes."""
	projects: set[str] = set()
	for resource in (*containers, *volumes, *networks):
		if resource.project is not None:
			projects.add(resource.project)
	return tuple(sorted(projects))


#============================================
def project_snapshots(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> tuple[local_stack_control.models.ProjectSnapshot, ...]:
	"""Discover snapshots for every labelled Compose project."""
	resources = discover_resources(runner, repo_root)
	projects = all_projects(*resources)
	return tuple(snapshot_for_project(project, *resources) for project in projects)
