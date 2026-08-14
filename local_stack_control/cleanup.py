"""Exact project-scoped cleanup and acceptance preflight decisions."""

import local_stack_control.compose
import local_stack_control.models


#============================================
def duplicate_identities(values: tuple[str, ...]) -> tuple[str, ...]:
	"""Return each repeated resource identity once, in stable order."""
	seen: set[str] = set()
	duplicates: list[str] = []
	for value in values:
		if value in seen and value not in duplicates:
			duplicates.append(value)
		seen.add(value)
	return tuple(duplicates)


#============================================
def require_unambiguous_cleanup_snapshot(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Refuse cleanup when resource identities do not prove exact ownership.

	A normal Compose service can have several separately named replicas, so a
	service label is not an ownership identity.  A repeated engine ID, generated
	name, volume name, or network name is.  Refusing before Compose receives a
	mutating command keeps the controller from deleting a resource set it cannot
	describe precisely.
	"""
	container_ids = duplicate_identities(tuple(container.id for container in snapshot.containers))
	container_names = duplicate_identities(
		name for container in snapshot.containers for name in container.names
	)
	duplicate_volumes = duplicate_identities(tuple(volume.name for volume in snapshot.volumes))
	duplicate_networks = duplicate_identities(tuple(network.name for network in snapshot.networks))
	if (
		len(container_ids) == 0
		and len(container_names) == 0
		and len(duplicate_volumes) == 0
		and len(duplicate_networks) == 0
	):
		return

	details: list[str] = []
	if len(container_ids) > 0:
		details.append(f"duplicate container IDs {list(container_ids)}")
	if len(container_names) > 0:
		details.append(f"duplicate container names {list(container_names)}")
	if len(duplicate_volumes) > 0:
		details.append(f"duplicate volumes {list(duplicate_volumes)}")
	if len(duplicate_networks) > 0:
		details.append(f"duplicate networks {list(duplicate_networks)}")
	raise local_stack_control.models.ControllerError(
		"cleanup requires an unambiguous labelled resource snapshot: " + "; ".join(details)
	)


#============================================
def require_cleanup_resources(snapshot: local_stack_control.models.ProjectSnapshot) -> None:
	"""Refuse a Compose cleanup that has no labelled resources to reconcile.

	An empty result proves neither an owned running stack nor retained data.  It
	must not become a broad Compose mutation merely because the normal project
	name was selected.  A volume-only snapshot remains a valid retained-data
	target and is intentionally allowed.
	"""
	resource_count = len(snapshot.containers) + len(snapshot.volumes) + len(snapshot.networks)
	if resource_count == 0:
		raise local_stack_control.models.ControllerError(
			"no labelled project resources were found; refusing an empty cleanup mutation"
		)


#============================================
def require_declared_topology_resources(
	target: local_stack_control.models.ComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Fail before mutation when selected Compose files do not own the preview."""
	allowed_services = set(local_stack_control.models.BASE_LONG_RUNNING_SERVICES)
	allowed_services.update(local_stack_control.models.BASE_ONE_SHOT_SERVICES)
	# The launcher invokes this maintenance-profile guard before PostgreSQL
	# starts. It is not part of ordinary readiness, but a failed invocation can
	# leave a labelled container that the normal Compose down operation owns.
	allowed_services.update(local_stack_control.models.CLEANUP_ONLY_SERVICES)
	allowed_volumes = set(local_stack_control.models.DECLARED_BASE_VOLUMES)
	if target.with_smtp:
		allowed_services.update(local_stack_control.models.SMTP_ONE_SHOT_SERVICES)
		allowed_volumes.update(local_stack_control.models.DECLARED_SMTP_VOLUMES)
	unknown_services = sorted(
		{
			container.service
			for container in snapshot.containers
			if container.service is None or container.service not in allowed_services
		},
		key=lambda value: "" if value is None else value,
	)
	allowed_volume_names = {f"{target.project}_{name}" for name in allowed_volumes}
	unknown_volumes = sorted(
		volume.name for volume in snapshot.volumes if volume.name not in allowed_volume_names
	)
	allowed_network_names = {
		f"{target.project}_{name}" for name in local_stack_control.models.DECLARED_BASE_NETWORKS
	}
	unknown_networks = sorted(
		network.name for network in snapshot.networks if network.name not in allowed_network_names
	)
	if len(unknown_services) > 0 or len(unknown_volumes) > 0 or len(unknown_networks) > 0:
		details: list[str] = []
		if len(unknown_services) > 0:
			details.append(f"services {unknown_services}")
		if len(unknown_volumes) > 0:
			details.append(f"volumes {unknown_volumes}")
		if len(unknown_networks) > 0:
			details.append(f"networks {unknown_networks}")
		detail = "; ".join(details)
		raise local_stack_control.models.ControllerError(
			f"selected Compose topology does not cover labelled resources: {detail}"
		)


#============================================
def stop_plan(
	target: local_stack_control.models.ComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.CleanupPlan:
	"""Plan a default-stack stop that retains named data."""
	local_stack_control.compose.require_default_mutation_target(target)
	require_cleanup_resources(snapshot)
	require_unambiguous_cleanup_snapshot(snapshot)
	require_declared_topology_resources(target, snapshot)
	argv = local_stack_control.compose.compose_argv(target, ["down", "--remove-orphans"])
	plan = local_stack_control.models.CleanupPlan(
		project=target.project,
		snapshot=snapshot,
		argv=tuple(argv),
		removes_volumes=False,
	)
	return plan


#============================================
def reset_plan(
	target: local_stack_control.models.ComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	confirmed_project: str | None,
	dry_run: bool,
) -> local_stack_control.models.CleanupPlan:
	"""Plan a confirmed default-stack data reset."""
	local_stack_control.compose.require_default_mutation_target(target)
	require_cleanup_resources(snapshot)
	require_unambiguous_cleanup_snapshot(snapshot)
	require_declared_topology_resources(target, snapshot)
	if not dry_run and confirmed_project != local_stack_control.models.DEFAULT_PROJECT:
		raise local_stack_control.models.ControllerError(
			"reset requires --confirm-project containers"
		)
	argv = local_stack_control.compose.compose_argv(
		target,
		["down", "--volumes", "--remove-orphans"],
	)
	plan = local_stack_control.models.CleanupPlan(
		project=target.project,
		snapshot=snapshot,
		argv=tuple(argv),
		removes_volumes=True,
	)
	return plan


#============================================
def disposable_cleanup_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.CleanupPlan:
	"""Plan cleanup only after a runner proves typed disposable ownership."""
	local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"disposable snapshot does not match its owned project"
		)
	require_unambiguous_cleanup_snapshot(snapshot)
	resources = (*snapshot.containers, *snapshot.volumes, *snapshot.networks)
	if len(resources) == 0:
		raise local_stack_control.models.ControllerError(
			"no labelled disposable resources remain; cleanup is already complete"
		)
	argv = local_stack_control.compose.compose_argv(
		disposable.target,
		["down", "--volumes", "--remove-orphans"],
	)
	plan = local_stack_control.models.CleanupPlan(
		project=disposable.target.project,
		snapshot=snapshot,
		argv=tuple(argv),
		removes_volumes=True,
	)
	return plan


#============================================
def conflict_preflight(
	snapshots: tuple[local_stack_control.models.ProjectSnapshot, ...],
	allowed_projects: tuple[str, ...] = (),
) -> local_stack_control.models.ConflictPreflight:
	"""Identify active labelled projects not explicitly owned by a caller."""
	conflicts = tuple(
		snapshot.project
		for snapshot in snapshots
		if snapshot.project not in allowed_projects
		and any(container.running for container in snapshot.containers)
	)
	result = local_stack_control.models.ConflictPreflight(
		conflicting_projects=tuple(sorted(conflicts)),
		ok=len(conflicts) == 0,
	)
	return result


#============================================
def aggregate_acceptance_preflight(
	snapshots: tuple[local_stack_control.models.ProjectSnapshot, ...],
) -> local_stack_control.models.ConflictPreflight:
	"""Reject retained default or walkthrough containers before aggregate lanes.

	The aggregate suite owns neither target.  It deliberately blocks a stopped
	container too: a later lane uses fixed ports and must not silently reconcile
	or remove a prior caller's partially stopped environment.  Data-only
	projects do not block this preflight because they cannot bind a port or be
	reused as a running acceptance target.
	"""
	conflicts = tuple(
		snapshot.project
		for snapshot in snapshots
		if len(snapshot.containers) > 0
		and (
			snapshot.project == local_stack_control.models.DEFAULT_PROJECT
			or snapshot.project.startswith("ple-ui-walkthrough-")
		)
	)
	return local_stack_control.models.ConflictPreflight(
		conflicting_projects=tuple(sorted(conflicts)),
		ok=len(conflicts) == 0,
	)
