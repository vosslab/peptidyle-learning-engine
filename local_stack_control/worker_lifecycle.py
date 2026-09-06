"""Closed browser-owner actions for the one internal Live Demo worker."""

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter as adapter
import local_stack_control.models
import local_stack_control.process


#============================================
def worker_service(disposable: local_stack_control.models.DisposableComposeTarget) -> str:
	"""Return the one worker stop action granted to the browser lifecycle owner."""
	policy = adapter.disposable_policy(disposable)
	profile = adapter.live_demo_profile_policy(disposable)
	if (
		policy.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
		or profile.profile is not local_stack_control.models.LiveDemoProfile.BROWSER
		or "worker_lifecycle" not in profile.child_capabilities
	):
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot stop the live-demo worker"
		)
	return "worker"


#============================================
def worker_stop_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[list[str], dict[str, str]]:
	"""Form the browser owner’s fixed worker-stop command outside generic Compose."""
	argv = local_stack_control.compose.compose_argv(
		disposable.target, ["stop", worker_service(disposable)]
	)
	return argv, adapter.compose_environment(disposable)


#============================================
def worker_replacement_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[list[str], dict[str, str]]:
	"""Form the one fixed worker recreation command outside generic Compose."""
	argv = local_stack_control.compose.compose_argv(
		disposable.target,
		["up", "-d", "--force-recreate", "--no-deps", worker_service(disposable)],
	)
	return argv, adapter.compose_environment(disposable)


#============================================
def worker_stop_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.ServiceStopPlan:
	"""Select exactly one running fixed worker from a browser-owned snapshot."""
	adapter.require_declared_outage_snapshot(disposable, snapshot)
	service = worker_service(disposable)
	selected = tuple(container for container in snapshot.containers if container.service == service)
	if len(selected) != 1 or not selected[0].running:
		raise local_stack_control.models.ControllerError(
			"live-demo worker stop requires exactly one running labelled worker instance"
		)
	argv, _environment = worker_stop_command(disposable)
	return local_stack_control.models.ServiceStopPlan(
		project=disposable.target.project,
		service=service,
		argv=tuple(argv),
	)


#============================================
def worker_replacement_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.WorkerReplacementPlan:
	"""Bind the fixed recreation command to one exact stopped worker container."""
	adapter.require_declared_outage_snapshot(disposable, snapshot)
	service = worker_service(disposable)
	selected = tuple(container for container in snapshot.containers if container.service == service)
	if len(selected) != 1 or selected[0].running:
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement requires exactly one stopped labelled worker instance"
		)
	argv, _environment = worker_replacement_command(disposable)
	return local_stack_control.models.WorkerReplacementPlan(
		project=disposable.target.project,
		service=service,
		previous_container_id=selected[0].id,
		argv=tuple(argv),
	)


#============================================
def require_worker_stopped(
	disposable: local_stack_control.models.DisposableComposeTarget,
	before: local_stack_control.models.ProjectSnapshot,
	after: local_stack_control.models.ProjectSnapshot,
	plan: local_stack_control.models.ServiceStopPlan,
) -> None:
	"""Prove the fixed worker stop changed neither persistence nor another service."""
	adapter.require_declared_outage_snapshot(disposable, before)
	adapter.require_declared_outage_snapshot(disposable, after)
	service = worker_service(disposable)
	if plan.project != disposable.target.project or plan.service != service:
		raise local_stack_control.models.ControllerError(
			"live-demo worker plan does not match its selected policy"
		)
	expected_argv, _environment = worker_stop_command(disposable)
	if plan.argv != tuple(expected_argv):
		raise local_stack_control.models.ControllerError(
			"live-demo worker plan does not match its closed command"
		)
	if adapter.persistent_scope(before) != adapter.persistent_scope(after):
		raise local_stack_control.models.ControllerError(
			"live-demo worker stop changed labelled persistent resource scope"
		)
	if adapter.unrelated_containers(before, service) != adapter.unrelated_containers(after, service):
		raise local_stack_control.models.ControllerError(
			"live-demo worker stop changed an unrelated labelled container"
		)
	selected_before = tuple(container for container in before.containers if container.service == service)
	stopped = tuple(container for container in after.containers if container.service == service)
	if len(selected_before) != 1 or not selected_before[0].running:
		raise local_stack_control.models.ControllerError(
			"live-demo worker preselection is not exactly one running labelled worker instance"
		)
	if len(stopped) != 1 or stopped[0].running or stopped[0].id != selected_before[0].id:
		raise local_stack_control.models.ControllerError(
			"live-demo worker stop did not leave exactly one selected worker stopped"
		)


#============================================
def require_worker_replaced(
	disposable: local_stack_control.models.DisposableComposeTarget,
	before: local_stack_control.models.ProjectSnapshot,
	after: local_stack_control.models.ProjectSnapshot,
	plan: local_stack_control.models.WorkerReplacementPlan,
) -> local_stack_control.models.ContainerResource:
	"""Prove a fixed recreation replaced one stopped worker and nothing else."""
	adapter.require_declared_outage_snapshot(disposable, before)
	adapter.require_declared_outage_snapshot(disposable, after)
	service = worker_service(disposable)
	if plan.project != disposable.target.project or plan.service != service:
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement plan does not match its selected policy"
		)
	expected_argv, _environment = worker_replacement_command(disposable)
	if plan.argv != tuple(expected_argv):
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement plan does not match its closed command"
		)
	if adapter.persistent_scope(before) != adapter.persistent_scope(after):
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement changed labelled persistent resource scope"
		)
	if adapter.unrelated_containers(before, service) != adapter.unrelated_containers(after, service):
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement changed an unrelated labelled container"
		)
	selected_before = tuple(container for container in before.containers if container.service == service)
	selected_after = tuple(container for container in after.containers if container.service == service)
	if (
		len(selected_before) != 1
		or selected_before[0].running
		or selected_before[0].id != plan.previous_container_id
	):
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement preselection is not one selected stopped worker"
		)
	if len(selected_after) != 1 or not selected_after[0].running:
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement did not leave one running labelled worker"
		)
	if selected_after[0].id == plan.previous_container_id:
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement retained the stopped worker container"
		)
	return selected_after[0]


#============================================
def stop_worker_service(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.WorkerStop:
	"""Stop only the browser owner’s worker and prove the exact postcondition."""
	before = adapter.require_current_resource_capability(runner, disposable)
	plan = worker_stop_plan(disposable, before)
	argv, environment = worker_stop_command(disposable)
	if tuple(argv) != plan.argv:
		raise local_stack_control.models.ControllerError(
			"live-demo worker command does not match its closed plan"
		)
	result = runner.stream(argv, environment, disposable.target.repo_root)
	if result != 0:
		raise local_stack_control.models.ControllerError("live-demo worker stop command failed")
	after = adapter.require_current_resource_capability(runner, disposable)
	require_worker_stopped(disposable, before, after, plan)
	return local_stack_control.models.WorkerStop(plan.project, plan.service)


#============================================
def replace_worker_service(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.WorkerReplacement:
	"""Recreate only the browser owner’s stopped worker and prove its replacement."""
	before = adapter.require_current_resource_capability(runner, disposable)
	plan = worker_replacement_plan(disposable, before)
	argv, environment = worker_replacement_command(disposable)
	if tuple(argv) != plan.argv:
		raise local_stack_control.models.ControllerError(
			"live-demo worker replacement command does not match its closed plan"
		)
	result = runner.stream(argv, environment, disposable.target.repo_root)
	if result != 0:
		raise local_stack_control.models.ControllerError("live-demo worker replacement command failed")
	after = adapter.require_current_resource_capability(runner, disposable)
	container = require_worker_replaced(disposable, before, after, plan)
	return local_stack_control.models.WorkerReplacement(
		plan.project, plan.service, plan.previous_container_id, container.id
	)
