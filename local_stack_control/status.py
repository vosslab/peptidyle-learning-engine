"""Semantic project and service readiness classification."""

import local_stack_control.lifecycle_profiles
import local_stack_control.models


#============================================
def required_one_shots(unused_with_smtp: bool = False) -> tuple[str, ...]:
	"""Return the one-shot services required by the supported topology."""
	return local_stack_control.models.BASE_ONE_SHOT_SERVICES


#============================================
def required_long_running(unused_with_smtp: bool = False) -> tuple[str, ...]:
	"""Return long-running services required by the supported topology."""
	return local_stack_control.models.BASE_LONG_RUNNING_SERVICES


#============================================
def service_containers(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> tuple[local_stack_control.models.ContainerResource, ...]:
	"""Return every observed instance for one service."""
	containers = tuple(item for item in snapshot.containers if item.service == service)
	return containers


#============================================
def cardinality_failure_status(
	service: str,
	instances: int,
	expected_instances: int,
) -> local_stack_control.models.StackServiceStatus:
	"""Build a service result whose observed cardinality misses its contract."""
	state = "missing"
	if instances > expected_instances:
		state = "ambiguous"
	status = local_stack_control.models.StackServiceStatus(
		service=service,
		instances=instances,
		present=instances > 0,
		running=False,
		healthy=False,
		complete=False,
		state=state,
		health=None,
		exit_code=None,
	)
	return status


#============================================
def one_shot_status(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> local_stack_control.models.StackServiceStatus:
	"""Compute one-shot completion from inspected state."""
	containers = service_containers(snapshot, service)
	if len(containers) != 1:
		return cardinality_failure_status(service, len(containers), 1)
	container = containers[0]
	complete = container.state == "exited" and container.exit_code == 0
	status = local_stack_control.models.StackServiceStatus(
		service=service,
		instances=1,
		present=True,
		running=container.running,
		healthy=complete,
		complete=complete,
		state=container.state,
		health=container.health,
		exit_code=container.exit_code,
	)
	return status


#============================================
def long_running_status(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
	expected_instances: int,
) -> local_stack_control.models.StackServiceStatus:
	"""Compute long-running health from inspected state."""
	containers = service_containers(snapshot, service)
	if len(containers) != expected_instances:
		return cardinality_failure_status(
			service, len(containers), expected_instances
		)
	healthy = all(
		container.running and container.health == "healthy"
		for container in containers
	)
	running = all(container.running for container in containers)
	state = "running"
	if not running:
		state = next(container.state for container in containers if not container.running)
	health_values = {container.health for container in containers}
	health = containers[0].health if len(health_values) == 1 else "mixed"
	exit_code = next(
		(container.exit_code for container in containers if container.exit_code not in (None, 0)),
		None,
	)
	status = local_stack_control.models.StackServiceStatus(
		service=service,
		instances=len(containers),
		present=True,
		running=running,
		healthy=healthy,
		complete=False,
		state=state,
		health=health,
		exit_code=exit_code,
	)
	return status


#============================================
def project_state(snapshot: local_stack_control.models.ProjectSnapshot) -> str:
	"""Classify coarse project activity independent of declared topology."""
	if len(snapshot.containers) == 0:
		if len(snapshot.volumes) > 0 or len(snapshot.networks) > 0:
			return "stopped-with-data"
		return "absent"
	running = sum(1 for item in snapshot.containers if item.running)
	if running == 0:
		return "stopped-with-data"
	clean_one_shots = all(
		item.running or (item.state == "exited" and item.exit_code == 0)
		for item in snapshot.containers
	)
	if not clean_one_shots:
		return "partially-active"
	return "active"


#============================================
def build_report(
	project: str,
	with_smtp: bool,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.StatusReport:
	"""Build meaningful readiness for the supported topology."""
	return _build_report(project, with_smtp, snapshot, None)


#============================================
def build_target_report(
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.StatusReport:
	"""Build readiness using the selected target's closed lifecycle profile."""
	selected = local_stack_control.lifecycle_profiles.target_of(target)
	report = _build_report(selected.project, selected.with_smtp, snapshot, target)
	return report


#============================================
def _build_report(
	project: str,
	with_smtp: bool,
	snapshot: local_stack_control.models.ProjectSnapshot,
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget
	| None,
) -> local_stack_control.models.StatusReport:
	"""Build readiness with default or closed target-derived cardinality."""
	statuses: list[local_stack_control.models.StackServiceStatus] = []
	for service in required_one_shots():
		statuses.append(one_shot_status(snapshot, service))
	for service in required_long_running():
		expected_instances = 1
		if target is not None:
			expected_instances = local_stack_control.lifecycle_profiles.expected_long_running_count(
				target, service
			)
		statuses.append(long_running_status(snapshot, service, expected_instances))

	coarse_state = project_state(snapshot)
	ok = len(statuses) > 0 and all(item.healthy for item in statuses)
	if ok:
		state = "ready"
		message = "all required services are ready"
	elif coarse_state in ("absent", "stopped-with-data"):
		state = coarse_state
		message = "no active stack resources"
	elif any(item.state == "ambiguous" for item in statuses):
		state = "failed"
		message = "a required service has unexpected extra instances"
	elif any(
		item.present and item.state == "exited" and item.exit_code not in (None, 0)
		for item in statuses
	):
		state = "failed"
		message = "a required service failed"
	elif any(item.state == "missing" for item in statuses):
		state = "partially-active"
		message = "required service instances are missing"
	elif all(item.present for item in statuses):
		state = "starting"
		message = "required services are not ready yet"
	else:
		state = "partially-active"
		message = "required services are missing"
	report = local_stack_control.models.StatusReport(
		project=project,
		with_smtp=with_smtp,
		snapshot=snapshot,
		services=tuple(statuses),
		ok=ok,
		state=state,
		message=message,
	)
	return report


#============================================
def project_summary(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.ProjectSummary:
	"""Build a non-secret inventory summary."""
	summary = local_stack_control.models.ProjectSummary(
		project=snapshot.project,
		containers=len(snapshot.containers),
		running=sum(1 for item in snapshot.containers if item.running),
		volumes=len(snapshot.volumes),
		networks=len(snapshot.networks),
		state=project_state(snapshot),
	)
	return summary
