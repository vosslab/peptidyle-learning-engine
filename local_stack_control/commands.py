"""Operator command behavior for the local stack controller."""

import argparse
import dataclasses
import json
import pathlib
import platform
import shlex

import local_stack_control.cleanup
import local_stack_control.acceptance_lanes
import local_stack_control.compose
import local_stack_control.consumer
import local_stack_control.discovery
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.status
import local_stack_control.lifecycle


DEFAULT_LIFECYCLE_TIMEOUT_SECONDS = 180.0


#============================================
def authorize_start_target(target: local_stack_control.models.ComposeTarget) -> None:
	"""Authorize an existing default target or the one first-run default location."""
	if target.env_file.exists():
		local_stack_control.compose.require_default_mutation_target(target)
		return
	expected_env_file = (target.repo_root / local_stack_control.models.DEFAULT_ENV_FILE).absolute()
	expected_compose_files = local_stack_control.compose.compose_files(
		target.repo_root, target.with_smtp
	)
	if (
		target.project != local_stack_control.models.DEFAULT_PROJECT
		or target.env_file != expected_env_file
		or target.env_file.is_symlink()
		or target.compose_files != expected_compose_files
	):
		raise local_stack_control.models.ControllerError(
			"a custom mutating env file must already exist and have mode 0600"
		)
	for compose_file in target.compose_files:
		if not compose_file.is_file():
			raise local_stack_control.models.ControllerError(f"{compose_file} does not exist")


#============================================
def asdict_for_json(value: object) -> object:
	"""Convert public dataclass output into JSON-ready values."""
	if isinstance(value, local_stack_control.models.ContainerResource):
		result = {}
		for field in dataclasses.fields(value):
			field_value = getattr(value, field.name)
			# Container IDs are internal engine identity.  A short prefix is enough
			# to correlate this diagnostic with local ``podman ps`` output without
			# turning public status JSON into an implementation-identity dump.
			if field.name == "id":
				field_value = display_container_id(str(field_value))
			result[field.name] = asdict_for_json(field_value)
		return result
	if dataclasses.is_dataclass(value) and not isinstance(value, type):
		result = {}
		for field in dataclasses.fields(value):
			if field.name in ("repo_root", "env_file", "compose_files", "env_setting_names", "provider"):
				continue
			result[field.name] = asdict_for_json(getattr(value, field.name))
		return result
	if isinstance(value, pathlib.Path):
		return str(value)
	if isinstance(value, tuple) or isinstance(value, list):
		return [asdict_for_json(item) for item in value]
	if isinstance(value, dict):
		return {str(key): asdict_for_json(item) for key, item in value.items()}
	return value


#============================================
def display_container_id(container_id: str) -> str:
	"""Return the compact local diagnostic identity for one container."""
	result = container_id[:12]
	return result


#============================================
def print_json(value: object) -> None:
	"""Print deterministic JSON."""
	print(json.dumps(asdict_for_json(value), indent=2, sort_keys=True))


#============================================
def target_from_args(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	allow_missing_env: bool = False,
) -> local_stack_control.models.ComposeTarget:
	"""Resolve target options from parsed CLI arguments."""
	project = getattr(args, "project", None)
	target = local_stack_control.compose.resolve_target(
		runner,
		repo_root,
		args.env_file,
		args.with_smtp,
		project,
		allow_missing_env,
	)
	return target


#============================================
def child_environment(target: local_stack_control.models.ComposeTarget) -> dict[str, str]:
	"""Build a sanitized child environment for one target."""
	environment = local_stack_control.compose.target_environment(
		target,
		local_stack_control.process.current_environment(),
	)
	return environment


#============================================
def print_snapshot(snapshot: local_stack_control.models.ProjectSnapshot) -> None:
	"""Print an exact non-secret resource preview."""
	print(f"Project: {snapshot.project}")
	print("Containers:")
	for container in snapshot.containers:
		name = container.names[0] if len(container.names) > 0 else display_container_id(container.id)
		print(f"  {container.service or '-'}: {name}")
	print("Volumes:")
	for volume in snapshot.volumes:
		print(f"  {volume.name}")
	print("Networks:")
	for network in snapshot.networks:
		print(f"  {network.name}")


#============================================
def print_host_path_preview(plan: local_stack_control.models.CleanupPlan) -> None:
	"""Show each database-bound host record selected by one cleanup plan."""
	for path in plan.host_paths_to_remove:
		relative_path = path.relative_to(path.parents[1])
		print(f"Host record to remove after Compose cleanup: {relative_path}")


#============================================
def remove_reset_host_paths(plan: local_stack_control.models.CleanupPlan) -> None:
	"""Remove the reset-owned manifest after Compose cleanup proves an empty project."""
	for path in plan.host_paths_to_remove:
		if not path.exists() and not path.is_symlink():
			continue
		local_stack_control.consumer.require_private_regular_file(
			path,
			"reset-owned Chapter One manifest",
		)
		path.unlink()


#============================================
def project_snapshots(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> tuple[local_stack_control.models.ProjectSnapshot, ...]:
	"""Discover every labelled project through the shared layer."""
	return local_stack_control.discovery.project_snapshots(runner, repo_root)


#============================================
def doctor(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Report engine, provider, machine, env metadata, and resource inventory."""
	checks: list[local_stack_control.models.DoctorCheck] = []
	runtime_environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	info_result = runner.run(
		["podman", "info", "--format", "json"], runtime_environment, repo_root
	)
	if info_result.ok():
		items = local_stack_control.discovery.json_array(
			f"[{info_result.stdout}]",
			"podman info",
		)
		info = items[0]
		host = info.get("host")
		version = info.get("version")
		client = info.get("Client")
		if not isinstance(host, dict) or not isinstance(version, dict) or not isinstance(client, dict):
			raise local_stack_control.models.ControllerError("podman info has incomplete metadata")
		security = host.get("security")
		if not isinstance(security, dict) or not isinstance(security.get("rootless"), bool):
			raise local_stack_control.models.ControllerError("podman info has no rootless status")
		checks.append(local_stack_control.models.DoctorCheck("client", "OK", str(client.get("Version", "unknown"))))
		checks.append(local_stack_control.models.DoctorCheck("server", "OK", str(version.get("Version", "unknown"))))
		rootless = "yes" if security["rootless"] else "no"
		checks.append(local_stack_control.models.DoctorCheck("rootless", "OK" if rootless == "yes" else "FAIL", rootless))
	else:
		detail = info_result.stderr.strip() or "engine unavailable"
		checks.append(local_stack_control.models.DoctorCheck("podman", "FAIL", detail))

	provider = local_stack_control.compose.choose_provider(runner, repo_root)
	checks.append(local_stack_control.models.DoctorCheck("compose provider", "OK", provider.name))
	if platform.system() == "Darwin":
		machine_result = runner.run(
			["podman", "machine", "list", "--format", "json"], runtime_environment, repo_root
		)
		if machine_result.ok():
			machines = local_stack_control.discovery.json_array(machine_result.stdout, "podman machine list")
			running = any(item.get("Running") is True for item in machines)
			detail = "running" if running else "stopped"
			checks.append(local_stack_control.models.DoctorCheck("podman machine", "OK" if running else "WARN", detail))
		else:
			detail = machine_result.stderr.strip() or "metadata unavailable"
			checks.append(local_stack_control.models.DoctorCheck("podman machine", "WARN", detail))

	env_file = local_stack_control.compose.resolve_path(repo_root, args.env_file)
	env_errors = local_stack_control.env_file.mutation_env_file_errors(env_file)
	if len(env_errors) == 0:
		file_stat = env_file.stat()
		detail = f"{env_file}; owner uid {file_stat.st_uid}; mode {file_stat.st_mode & 0o777:04o}"
		checks.append(local_stack_control.models.DoctorCheck("env file", "OK", detail))
	else:
		status = "WARN" if not env_file.exists() and not env_file.is_symlink() else "FAIL"
		checks.append(local_stack_control.models.DoctorCheck("env file", status, "; ".join(env_errors)))

	summaries = tuple(local_stack_control.status.project_summary(item) for item in project_snapshots(runner, repo_root))
	checks.append(local_stack_control.models.DoctorCheck("compose projects", "OK", str(len(summaries))))
	output = {"checks": checks, "projects": summaries}
	if args.json:
		print_json(output)
	else:
		for check in checks:
			print(f"{check.status:4} {check.name}: {check.detail}")
	return 1 if any(check.status == "FAIL" for check in checks) else 0


#============================================
def status(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Report semantic readiness for an explicit read-only target."""
	target = target_from_args(args, runner, repo_root)
	snapshot = local_stack_control.discovery.discover_snapshot(runner, repo_root, target.project)
	report = local_stack_control.status.build_report(target.project, target.with_smtp, snapshot)
	if args.json:
		print_json(report)
	else:
		print(f"Project: {report.project}")
		print(f"State: {report.state}")
		print(f"Status: {report.message}")
		print(f"Resources: {len(snapshot.containers)} containers, {len(snapshot.volumes)} volumes, {len(snapshot.networks)} networks")
		for service in report.services:
			print(f"  {service.service:30} instances={service.instances} state={service.state} health={service.health or '-'}")
	return 0 if report.ok else 1


#============================================
def projects(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""List all labelled projects, including retained-data-only projects."""
	summaries = tuple(local_stack_control.status.project_summary(item) for item in project_snapshots(runner, repo_root))
	if args.json:
		print_json(summaries)
	else:
		for summary in summaries:
			print(f"{summary.project}: {summary.state}; {summary.containers} containers, {summary.volumes} volumes, {summary.networks} networks")
	return 0


#============================================
def start(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Initialize and start the selected default local stack in process."""
	target = target_from_args(args, runner, repo_root, allow_missing_env=True)
	authorize_start_target(target)
	result = local_stack_control.lifecycle.start_lifecycle(
		target,
		runner,
		repo_root,
		local_stack_control.lifecycle.LifecycleOptions(
			DEFAULT_LIFECYCLE_TIMEOUT_SECONDS,
			not args.skip_build,
			args.release,
			not args.no_open,
		),
	)
	print(f"Local stack ready: {result.gateway_url}")
	return 0


#============================================
def execute_cleanup(
	plan: local_stack_control.models.CleanupPlan,
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	dry_run: bool,
) -> int:
	"""Preview then execute one exact cleanup plan."""
	print_snapshot(plan.snapshot)
	print_host_path_preview(plan)
	print("Command:", shlex.join(plan.argv))
	if dry_run:
		return 0
	local_stack_control.process.require_rootless_local_engine(runner, target.repo_root)
	result = runner.stream(list(plan.argv), child_environment(target), target.repo_root)
	if result != 0:
		return result
	remaining = local_stack_control.discovery.discover_snapshot(runner, target.repo_root, target.project)
	if plan.removes_volumes:
		if len(remaining.containers) + len(remaining.volumes) + len(remaining.networks) > 0:
			raise local_stack_control.models.ControllerError(
				"reset finished but labelled project resources remain"
			)
		remove_reset_host_paths(plan)
	elif any(container.running for container in remaining.containers):
		raise local_stack_control.models.ControllerError(
			"stop finished but labelled project containers are still running"
		)
	return 0


#============================================
def stop(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Stop the default stack and retain named data."""
	target = target_from_args(args, runner, repo_root)
	snapshot = local_stack_control.discovery.discover_snapshot(runner, repo_root, target.project)
	plan = local_stack_control.cleanup.stop_plan(target, snapshot)
	return execute_cleanup(plan, target, runner, False)


#============================================
def reset(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Remove only the confirmed default project's Compose data."""
	target = target_from_args(args, runner, repo_root)
	snapshot = local_stack_control.discovery.discover_snapshot(runner, repo_root, target.project)
	plan = local_stack_control.cleanup.reset_plan(
		target, snapshot, args.confirm_project, args.dry_run
	)
	return execute_cleanup(plan, target, runner, args.dry_run)


#============================================
def logs(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Show validated, explicitly targeted Compose service logs."""
	target = target_from_args(args, runner, repo_root)
	allowed = set(local_stack_control.models.BASE_LONG_RUNNING_SERVICES)
	allowed.update(local_stack_control.models.BASE_ONE_SHOT_SERVICES)
	if target.with_smtp:
		allowed.update(local_stack_control.models.SMTP_ONE_SHOT_SERVICES)
		allowed.update(local_stack_control.models.SMTP_LONG_RUNNING_SERVICES)
	services = list(args.services)
	if len(services) == 0:
		services = ["gateway", "api", "worker"]
	unknown = sorted(set(services) - allowed)
	if len(unknown) > 0:
		raise local_stack_control.models.ControllerError(
			f"unknown service for this topology: {', '.join(unknown)}"
		)
	argv = local_stack_control.compose.compose_argv(
		target,
		["logs", "--no-color", "--tail", args.tail],
	)
	if args.follow:
		argv.append("--follow")
	argv.extend(services)
	print("WARNING: application logs may contain private local diagnostic data.")
	print("Command:", shlex.join(argv))
	return runner.stream(argv, child_environment(target), repo_root)


#============================================
def service_stop_plan(
	target: local_stack_control.models.ComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> local_stack_control.models.ServiceStopPlan:
	"""Authorize an outage only for one currently running renderer instance."""
	if service not in local_stack_control.models.STOPPABLE_SERVICES:
		raise local_stack_control.models.ControllerError(
			"service stop is limited to webwork-renderer"
		)
	local_stack_control.compose.require_default_mutation_target(target)
	containers = local_stack_control.status.service_containers(snapshot, service)
	if len(containers) != 1 or not containers[0].running:
		raise local_stack_control.models.ControllerError(
			"service stop requires exactly one running labelled webwork-renderer"
		)
	argv = local_stack_control.compose.compose_argv(target, ["stop", service])
	return local_stack_control.models.ServiceStopPlan(
		project=target.project,
		service=service,
		argv=tuple(argv),
	)


#============================================
def service_is_stopped(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> bool:
	"""Return whether label discovery proves one stopped service instance."""
	containers = local_stack_control.status.service_containers(snapshot, service)
	return len(containers) == 1 and not containers[0].running


#============================================
def require_service_stopped(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> None:
	"""Verify the scoped stop left one labelled service instance stopped."""
	if not service_is_stopped(snapshot, service):
		raise local_stack_control.models.ControllerError(
			"service stop did not leave exactly one stopped labelled webwork-renderer"
		)


#============================================
def persistent_scope(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> tuple[tuple[str, ...], tuple[str, ...]]:
	"""Return order-independent labelled persistent resource identity."""
	return (
		tuple(sorted(item.name for item in snapshot.volumes)),
		tuple(sorted(item.name for item in snapshot.networks)),
	)


#============================================
def service_stop(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Stop only the default renderer after label-derived authority checks."""
	local_stack_control.process.require_rootless_local_engine(runner, repo_root)
	target = target_from_args(args, runner, repo_root)
	local_stack_control.compose.require_default_mutation_target(target)
	snapshot = local_stack_control.discovery.discover_snapshot(runner, repo_root, target.project)
	plan = service_stop_plan(target, snapshot, args.service)
	print(f"Project: {plan.project}")
	print("Command:", shlex.join(plan.argv))
	result = runner.stream(list(plan.argv), child_environment(target), repo_root)
	if result != 0:
		return result
	remaining = local_stack_control.discovery.discover_snapshot(runner, repo_root, target.project)
	if persistent_scope(remaining) != persistent_scope(snapshot):
		raise local_stack_control.models.ControllerError(
			"service stop changed labelled persistent resource scope"
		)
	require_service_stopped(remaining, plan.service)
	return 0


#============================================
def restart(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Restart one allowed stateless service through the lifecycle owner."""
	target = target_from_args(args, runner, repo_root)
	if args.service not in local_stack_control.models.restartable_services(target.with_smtp):
		raise local_stack_control.models.ControllerError(
			"restart is limited to stateless services in the selected topology"
		)
	local_stack_control.compose.require_default_mutation_target(target)
	result = local_stack_control.lifecycle.restart_lifecycle(
		target, runner, repo_root, args.service,
		local_stack_control.lifecycle.LifecycleOptions(
			DEFAULT_LIFECYCLE_TIMEOUT_SECONDS, False, False, False
		),
	)
	print(f"Local stack ready: {result.gateway_url}")
	return 0


#============================================
def validate(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Read-only validate initialized canonical configuration and runtime state."""
	target = target_from_args(args, runner, repo_root)
	if target.project != local_stack_control.models.DEFAULT_PROJECT:
		raise local_stack_control.models.ControllerError("validate is canonical-stack-only")
	local_stack_control.lifecycle.validate_lifecycle(target, runner, repo_root)
	snapshot = local_stack_control.discovery.discover_snapshot(runner, repo_root, target.project)
	report = local_stack_control.status.build_report(target.project, target.with_smtp, snapshot)
	if args.json:
		print_json(report)
	else:
		print(f"Runtime state: {report.state}. {report.message}")
	return 0


#============================================
def acceptance(
	args: argparse.Namespace,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Run aggregate browser lanes after shared read-only conflict preflight."""
	local_stack_control.process.require_rootless_local_engine(runner, repo_root)
	snapshots = project_snapshots(runner, repo_root)
	preflight = local_stack_control.cleanup.aggregate_acceptance_preflight(snapshots)
	if not preflight.ok:
		projects = ", ".join(preflight.conflicting_projects)
		raise local_stack_control.models.ControllerError(
			"acceptance requires no existing default or walkthrough containers; "
			f"stop and remove only the projects you own, then retry: {projects}"
		)
	environment = local_stack_control.env_file.sanitized_acceptance_environment(
		local_stack_control.process.current_environment()
	)
	return local_stack_control.acceptance_lanes.run(runner, repo_root, environment)
