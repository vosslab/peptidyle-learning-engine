"""Shared non-destructive ownership checks for the fixed browser-suite project."""

import local_stack_control.models


#============================================
def require_live_demo_browser_ownership(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Validate fixed owner labels and the closed browser Compose topology."""
	project = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	owner = local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
	if snapshot.project != project:
		raise local_stack_control.models.ControllerError(
			"live-demo browser ownership has an invalid project"
		)
	allowed_services = set(local_stack_control.models.BASE_LONG_RUNNING_SERVICES)
	allowed_services.update(local_stack_control.models.BASE_ONE_SHOT_SERVICES)
	allowed_services.update(local_stack_control.models.CLEANUP_ONLY_SERVICES)
	allowed_volumes = {
		f"{project}_{name}" for name in local_stack_control.models.DECLARED_BASE_VOLUMES
	}
	allowed_networks = {
		f"{project}_{name}" for name in local_stack_control.models.DECLARED_BASE_NETWORKS
	}
	for container in snapshot.containers:
		if (
			container.project != project
			or container.owner != owner
			or container.service not in allowed_services
		):
			raise local_stack_control.models.ControllerError(
				"live-demo browser ownership found foreign resource ownership"
			)
	for volume in snapshot.volumes:
		if volume.project != project or volume.owner != owner or volume.name not in allowed_volumes:
			raise local_stack_control.models.ControllerError(
				"live-demo browser ownership found foreign resource ownership"
			)
	for network in snapshot.networks:
		if network.project != project or network.owner != owner or network.name not in allowed_networks:
			raise local_stack_control.models.ControllerError(
				"live-demo browser ownership found foreign resource ownership"
			)
