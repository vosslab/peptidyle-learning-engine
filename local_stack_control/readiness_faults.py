"""Closed M2 readiness-fault actions for the fixed browser Live Demo."""

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter as adapter
import local_stack_control.models
import local_stack_control.process


READINESS_DEPENDENCIES = ("api", "minio", "postgres", "webwork-renderer", "worker")


#============================================
def readiness_dependency(
	disposable: local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> str:
	"""Validate one fault target against the browser profile's closed dependency set."""
	policy = adapter.disposable_policy(disposable)
	profile = adapter.live_demo_profile_policy(disposable)
	if (
		policy.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
		or profile.profile is not local_stack_control.models.LiveDemoProfile.BROWSER
		or "readiness_fault" not in profile.child_capabilities
		or service not in READINESS_DEPENDENCIES
	):
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot create the requested readiness fault"
		)
	return service


#============================================
def stop_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> tuple[list[str], dict[str, str]]:
	"""Form one exact dependency stop command outside generic Compose authority."""
	selected = readiness_dependency(disposable, service)
	return (
		local_stack_control.compose.compose_argv(disposable.target, ["stop", selected]),
		adapter.compose_environment(disposable),
	)


#============================================
def recovery_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> tuple[list[str], dict[str, str]]:
	"""Form one exact in-place dependency recovery command outside generic Compose authority."""
	selected = readiness_dependency(disposable, service)
	return (
		local_stack_control.compose.compose_argv(disposable.target, ["start", selected]),
		adapter.compose_environment(disposable),
	)


#============================================
def stop_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> local_stack_control.models.ServiceStopPlan:
	"""Bind one fault stop to exactly one running labelled dependency."""
	adapter.require_declared_outage_snapshot(disposable, snapshot)
	selected = readiness_dependency(disposable, service)
	instances = tuple(item for item in snapshot.containers if item.service == selected)
	if len(instances) != 1 or not instances[0].running:
		raise local_stack_control.models.ControllerError(
			"readiness fault requires exactly one running labelled dependency"
		)
	argv, _environment = stop_command(disposable, selected)
	return local_stack_control.models.ServiceStopPlan(
		disposable.target.project, selected, tuple(argv)
	)


#============================================
def recovery_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> local_stack_control.models.ReadinessFaultRecoveryPlan:
	"""Bind one in-place recovery to the exact stopped labelled dependency."""
	adapter.require_declared_outage_snapshot(disposable, snapshot)
	selected = readiness_dependency(disposable, service)
	instances = tuple(item for item in snapshot.containers if item.service == selected)
	if len(instances) != 1 or instances[0].running:
		raise local_stack_control.models.ControllerError(
			"readiness recovery requires exactly one stopped labelled dependency"
		)
	argv, _environment = recovery_command(disposable, selected)
	return local_stack_control.models.ReadinessFaultRecoveryPlan(
		disposable.target.project, selected, instances[0].id, tuple(argv)
	)


#============================================
def require_stopped(
	disposable: local_stack_control.models.DisposableComposeTarget,
	before: local_stack_control.models.ProjectSnapshot,
	after: local_stack_control.models.ProjectSnapshot,
	plan: local_stack_control.models.ServiceStopPlan,
) -> None:
	"""Prove a fault stopped only its selected dependency instance."""
	adapter.require_declared_outage_snapshot(disposable, before)
	adapter.require_declared_outage_snapshot(disposable, after)
	selected = readiness_dependency(disposable, plan.service)
	expected, _environment = stop_command(disposable, selected)
	if plan.project != disposable.target.project or plan.argv != tuple(expected):
		raise local_stack_control.models.ControllerError("readiness fault plan is not closed")
	if adapter.persistent_scope(before) != adapter.persistent_scope(after):
		raise local_stack_control.models.ControllerError("readiness fault changed persistent resources")
	if adapter.unrelated_containers(before, selected) != adapter.unrelated_containers(after, selected):
		raise local_stack_control.models.ControllerError("readiness fault changed an unrelated service")
	old = tuple(item for item in before.containers if item.service == selected)
	new = tuple(item for item in after.containers if item.service == selected)
	if len(old) != 1 or not old[0].running or len(new) != 1 or new[0].running or new[0].id != old[0].id:
		raise local_stack_control.models.ControllerError(
			"readiness fault did not leave the selected dependency stopped"
		)


#============================================
def require_recovered(
	disposable: local_stack_control.models.DisposableComposeTarget,
	before: local_stack_control.models.ProjectSnapshot,
	after: local_stack_control.models.ProjectSnapshot,
	plan: local_stack_control.models.ReadinessFaultRecoveryPlan,
) -> local_stack_control.models.ContainerResource:
	"""Prove a fault recovery restarted only its selected dependency instance."""
	adapter.require_declared_outage_snapshot(disposable, before)
	adapter.require_declared_outage_snapshot(disposable, after)
	selected = readiness_dependency(disposable, plan.service)
	expected, _environment = recovery_command(disposable, selected)
	if plan.project != disposable.target.project or plan.argv != tuple(expected):
		raise local_stack_control.models.ControllerError("readiness recovery plan is not closed")
	if adapter.persistent_scope(before) != adapter.persistent_scope(after):
		raise local_stack_control.models.ControllerError("readiness recovery changed persistent resources")
	if adapter.unrelated_containers(before, selected) != adapter.unrelated_containers(after, selected):
		raise local_stack_control.models.ControllerError("readiness recovery changed an unrelated service")
	old = tuple(item for item in before.containers if item.service == selected)
	new = tuple(item for item in after.containers if item.service == selected)
	if (
		len(old) != 1
		or old[0].running
		or old[0].id != plan.previous_container_id
		or len(new) != 1
		or not new[0].running
		or new[0].id != old[0].id
	):
		raise local_stack_control.models.ControllerError(
			"readiness recovery did not restart the selected dependency in place"
		)
	return new[0]


#============================================
def stop_dependency(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> local_stack_control.models.ReadinessFaultStop:
	"""Stop one closed fault target and verify its exact labelled postcondition."""
	before = adapter.require_current_resource_capability(runner, disposable)
	plan = stop_plan(disposable, before, service)
	argv, environment = stop_command(disposable, service)
	if tuple(argv) != plan.argv or runner.stream(argv, environment, disposable.target.repo_root) != 0:
		raise local_stack_control.models.ControllerError("readiness fault stop command failed")
	after = adapter.require_current_resource_capability(runner, disposable)
	require_stopped(disposable, before, after, plan)
	return local_stack_control.models.ReadinessFaultStop(plan.project, plan.service)


#============================================
def recover_dependency(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> local_stack_control.models.ReadinessFaultRecovery:
	"""Restart one closed stopped fault target and verify its labelled postcondition."""
	before = adapter.require_current_resource_capability(runner, disposable)
	plan = recovery_plan(disposable, before, service)
	argv, environment = recovery_command(disposable, service)
	if tuple(argv) != plan.argv or runner.stream(argv, environment, disposable.target.repo_root) != 0:
		raise local_stack_control.models.ControllerError("readiness recovery command failed")
	after = adapter.require_current_resource_capability(runner, disposable)
	container = require_recovered(disposable, before, after, plan)
	return local_stack_control.models.ReadinessFaultRecovery(
		plan.project, plan.service, plan.previous_container_id, container.id
	)
