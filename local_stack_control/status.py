"""Semantic project and service readiness classification."""

import local_stack_control.models


#============================================
def required_one_shots(with_smtp: bool) -> tuple[str, ...]:
	"""Return one-shot services required by the selected topology."""
	services = list(local_stack_control.models.BASE_ONE_SHOT_SERVICES)
	if with_smtp:
		services.extend(local_stack_control.models.SMTP_ONE_SHOT_SERVICES)
	return tuple(services)


#============================================
def required_long_running(with_smtp: bool) -> tuple[str, ...]:
	"""Return long-running services required by the selected topology."""
	services = list(local_stack_control.models.BASE_LONG_RUNNING_SERVICES)
	if with_smtp:
		services.extend(local_stack_control.models.SMTP_LONG_RUNNING_SERVICES)
	return tuple(services)


#============================================
def smtp_topology_present(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> bool:
	"""Return whether labelled resources prove the SMTP overlay was selected."""
	if any(item.service == "smtp-secret-init" for item in snapshot.containers):
		return True
	volume_name = f"{snapshot.project}_ple_smtp_runtime"
	present = any(item.name == volume_name for item in snapshot.volumes)
	return present


#============================================
def service_containers(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> tuple[local_stack_control.models.ContainerResource, ...]:
	"""Return every observed instance for one service."""
	containers = tuple(item for item in snapshot.containers if item.service == service)
	return containers


#============================================
def absent_or_ambiguous_status(
	service: str,
	instances: int,
) -> local_stack_control.models.StackServiceStatus:
	"""Build a missing or duplicate service result."""
	state = "missing"
	if instances > 1:
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
		return absent_or_ambiguous_status(service, len(containers))
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
) -> local_stack_control.models.StackServiceStatus:
	"""Compute long-running health from inspected state."""
	containers = service_containers(snapshot, service)
	if len(containers) != 1:
		return absent_or_ambiguous_status(service, len(containers))
	container = containers[0]
	healthy = container.running and container.health == "healthy"
	if service in ("worker", "invitation-delivery-worker"):
		healthy = container.running and container.health in (None, "", "disabled")
	status = local_stack_control.models.StackServiceStatus(
		service=service,
		instances=1,
		present=True,
		running=container.running,
		healthy=healthy,
		complete=False,
		state=container.state,
		health=container.health,
		exit_code=container.exit_code,
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
	"""Build meaningful readiness, inferring a persisted SMTP overlay safely."""
	effective_with_smtp = with_smtp or smtp_topology_present(snapshot)
	statuses: list[local_stack_control.models.StackServiceStatus] = []
	for service in required_one_shots(effective_with_smtp):
		statuses.append(one_shot_status(snapshot, service))
	for service in required_long_running(effective_with_smtp):
		statuses.append(long_running_status(snapshot, service))

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
		message = "a required service has duplicate instances"
	elif any(item.present and item.state == "exited" and item.exit_code not in (None, 0) for item in statuses):
		state = "failed"
		message = "a required service failed"
	elif all(item.present for item in statuses):
		state = "starting"
		message = "required services are not ready yet"
	else:
		state = "partially-active"
		message = "required services are missing"
	report = local_stack_control.models.StatusReport(
		project=project,
		with_smtp=effective_with_smtp,
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
