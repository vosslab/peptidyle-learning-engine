"""Refuse a second running copy of each developer Local Stack service."""

import collections.abc
import pathlib

import local_stack_control.discovery
import local_stack_control.lifecycle_profiles
import local_stack_control.models
import local_stack_control.process


# The ordinary developer stacks share postgres, MinIO, renderer, and api.
# Short-lived E2E owner prefixes are out of this set.
DEVELOPER_SINGLETON_PROJECTS = frozenset(
	{
		local_stack_control.models.DEFAULT_PROJECT,
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
	}
)


#============================================
def _container_label(container: local_stack_control.models.ContainerResource) -> str:
	"""Name one running instance for an operator-facing refusal."""
	name = container.id
	if len(container.names) > 0:
		name = container.names[0]
	project = container.project
	if project is None:
		project = "unlabelled"
	label = f"{name} in {project}"
	return label


#============================================
def singleton_violations(
	snapshots: tuple[local_stack_control.models.ProjectSnapshot, ...],
	target_project: str,
	expected_count: collections.abc.Callable[[str], int],
) -> tuple[str, ...]:
	"""Describe extra running copies of each long-running developer service."""
	running: dict[str, list[local_stack_control.models.ContainerResource]] = {}
	for snapshot in snapshots:
		if (
			snapshot.project not in DEVELOPER_SINGLETON_PROJECTS
			and snapshot.project != target_project
		):
			continue
		for container in snapshot.containers:
			if not container.running:
				continue
			service = container.service
			if service not in local_stack_control.models.BASE_LONG_RUNNING_SERVICES:
				continue
			running.setdefault(service, []).append(container)
	lines: list[str] = []
	for service in local_stack_control.models.BASE_LONG_RUNNING_SERVICES:
		instances = running.get(service, [])
		allowed = expected_count(service)
		foreign = tuple(item for item in instances if item.project != target_project)
		local = tuple(item for item in instances if item.project == target_project)
		if len(foreign) > 0:
			names = ", ".join(_container_label(item) for item in foreign)
			lines.append(
				f"{service} is already running outside {target_project}: {names}"
			)
		if len(local) > allowed:
			names = ", ".join(_container_label(item) for item in local)
			lines.append(
				f"{service} has {len(local)} running instances in {target_project}; "
				+ f"expected {allowed}: {names}"
			)
	return tuple(lines)


#============================================
def require_for_target(
	target: (
		local_stack_control.models.ComposeTarget
		| local_stack_control.models.DisposableComposeTarget
	),
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> None:
	"""Fail when another developer copy of postgres, MinIO, renderer, or api is up."""
	selected = local_stack_control.lifecycle_profiles.target_of(target)
	snapshots = local_stack_control.discovery.project_snapshots(runner, repo_root)
	violations = singleton_violations(
		snapshots,
		selected.project,
		lambda service: local_stack_control.lifecycle_profiles.expected_long_running_count(
			target, service
		),
	)
	if len(violations) == 0:
		return
	raise local_stack_control.models.ControllerError(
		"each Local Stack service may run only once; stop extras with "
		+ "./launchers/run_live_demo.sh stop or python3 local_stack.py stop, then retry: "
		+ "; ".join(violations)
	)


#============================================
def doctor_check(
	snapshots: tuple[local_stack_control.models.ProjectSnapshot, ...],
) -> local_stack_control.models.DoctorCheck:
	"""Report extra developer copies without selecting a lifecycle target."""
	active_projects: list[str] = []
	for snapshot in snapshots:
		if snapshot.project not in DEVELOPER_SINGLETON_PROJECTS:
			continue
		running = any(
			container.running
			and container.service in local_stack_control.models.BASE_LONG_RUNNING_SERVICES
			for container in snapshot.containers
		)
		if running:
			active_projects.append(snapshot.project)
	if len(active_projects) > 1:
		detail = "running in " + ", ".join(sorted(active_projects))
		check = local_stack_control.models.DoctorCheck("service singletons", "FAIL", detail)
		return check
	project = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	if len(active_projects) == 1:
		project = active_projects[0]
	violations = singleton_violations(snapshots, project, lambda service: 1 if service != "api" else 2)
	if len(violations) > 0:
		check = local_stack_control.models.DoctorCheck(
			"service singletons", "FAIL", "; ".join(violations)
		)
		return check
	check = local_stack_control.models.DoctorCheck(
		"service singletons",
		"OK",
		"one running copy of each developer service",
	)
	return check
