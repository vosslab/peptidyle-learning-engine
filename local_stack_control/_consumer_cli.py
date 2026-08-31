"""Private adapter for closed disposable local-stack E2E owners."""

import argparse
import pathlib
import re
import shlex
import sys

import local_stack_control.compose
import local_stack_control.consumer
import local_stack_control.discovery
import local_stack_control.models
import local_stack_control.process
import local_stack_control.lifecycle


#============================================
def parse_args(argv: list[str]) -> argparse.Namespace:
	"""Parse one intentionally small adapter action."""
	parser = argparse.ArgumentParser(
		prog="python3 -m local_stack_control._consumer_cli",
		description=__doc__,
	)
	actions = parser.add_subparsers(dest="action", required=True)
	compose = actions.add_parser("compose")
	compose.add_argument("--manifest", required=True, type=pathlib.Path)
	compose.add_argument("arguments", nargs=argparse.REMAINDER)
	cleanup = actions.add_parser("cleanup")
	cleanup.add_argument("--manifest", required=True, type=pathlib.Path)
	launch = actions.add_parser("launch")
	launch.add_argument("--manifest", required=True, type=pathlib.Path)
	launch.add_argument("--timeout-seconds", required=True, type=int)
	restart = actions.add_parser("restart")
	restart.add_argument("--manifest", required=True, type=pathlib.Path)
	restart.add_argument("--service", required=True)
	restart.add_argument("--timeout-seconds", required=True, type=int)
	stop_outage = actions.add_parser("stop-outage-service")
	stop_outage.add_argument("--manifest", required=True, type=pathlib.Path)
	evidence_logs = actions.add_parser("read-evidence-logs")
	evidence_logs.add_argument("--manifest", required=True, type=pathlib.Path)
	evidence_logs.add_argument(
		"--claim", required=True, choices=("renderer_delivery",)
	)
	diagnostics = actions.add_parser("diagnostics")
	diagnostics.add_argument("--manifest", required=True, type=pathlib.Path)
	diagnostics.add_argument("--service", action="append", default=[])
	stop_instance = actions.add_parser("stop-instance")
	stop_instance.add_argument("--manifest", required=True, type=pathlib.Path)
	stop_instance.add_argument("--service", required=True)
	stop_instance.add_argument("--id-prefix", required=True)
	postgresql_count = actions.add_parser("postgresql-count")
	postgresql_count.add_argument("--manifest", required=True, type=pathlib.Path)
	postgresql_count.add_argument("--attempt-id", required=True)
	args = parser.parse_args(argv)
	if args.action == "compose":
		if len(args.arguments) > 0 and args.arguments[0] == "--":
			args.arguments = args.arguments[1:]
		if len(args.arguments) == 0:
			parser.error("compose requires an explicit Compose command")
	return args


#============================================
def repo_root() -> pathlib.Path:
	"""Anchor this private entry point to its repository checkout."""
	return local_stack_control.compose.repo_root_from_entrypoint(pathlib.Path(__file__))


#============================================
def print_cleanup_preview(plan: local_stack_control.models.CleanupPlan) -> None:
	"""Show non-secret cleanup scope before the adapter executes it."""
	containers = len(plan.snapshot.containers)
	volumes = len(plan.snapshot.volumes)
	networks = len(plan.snapshot.networks)
	print(
		f"Disposable cleanup: project {plan.project}; "
		f"{containers} containers, {volumes} volumes, {networks} networks"
	)
	print("Command: " + shlex.join(plan.argv))


#============================================
def require_empty_post_cleanup(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> None:
	"""Require the label-derived resource set to disappear before image removal."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	local_stack_control.consumer.require_empty_post_cleanup_snapshot(snapshot)


#============================================
def remove_owned_project_images(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> int:
	"""Remove only policy-derived project image tags after cleanup proof."""
	for image in local_stack_control.consumer.owned_project_images(disposable):
		exists = runner.run(["podman", "image", "exists", image], cwd=disposable.target.repo_root)
		if exists.returncode == 1:
			continue
		if not exists.ok():
			raise local_stack_control.models.ControllerError(
				"cannot determine whether an owned disposable image tag exists"
			)
		argv = ["podman", "image", "rm", image]
		print("Command: " + shlex.join(argv))
		result = runner.stream(argv, cwd=disposable.target.repo_root)
		if result != 0:
			return result
	return 0


#============================================
def run_diagnostics(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
	services: tuple[str, ...],
) -> int:
	"""Print bounded, redacted status and logs for the replica owner."""
	environment = local_stack_control.consumer.compose_environment(disposable)
	commands = local_stack_control.consumer.diagnostic_commands(disposable, services)
	outputs: list[str] = []
	ok = True
	for argv in commands:
		result = runner.run(argv, environment, disposable.target.repo_root)
		ok = ok and result.ok()
		outputs.extend((result.stdout, result.stderr))
	private_values = local_stack_control.consumer.private_environment_values(
		disposable.target.env_file
	)
	print(local_stack_control.consumer.redact_diagnostics("\n".join(outputs), private_values))
	return 0 if ok else 1


#============================================
def compose_failure_diagnostics(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...],
) -> str:
	"""Return the bounded redacted receipt for one failed closed Compose call."""
	return local_stack_control.consumer.redact_diagnostics(
		"\n".join((result.stdout, result.stderr)), private_values
	)


#============================================
def write_compose_success_output(result: local_stack_control.models.CommandResult) -> None:
	"""Forward successful closed Compose output without changing its bytes."""
	sys.stdout.write(result.stdout)
	sys.stderr.write(result.stderr)


#============================================
def read_evidence_logs(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
	receipt_claim: str,
) -> int:
	"""Read the policy-selected service logs through one redacted bounded action."""
	snapshot = local_stack_control.consumer.require_current_resource_capability(runner, disposable)
	argv, environment = local_stack_control.consumer.evidence_log_command(
		disposable, receipt_claim, snapshot
	)
	result = runner.run(argv, environment, disposable.target.repo_root)
	private_values = local_stack_control.consumer.private_environment_values(
		disposable.target.env_file
	)
	sys.stdout.write(
		local_stack_control.consumer.redact_evidence_logs(result.stdout, private_values)
	)
	sys.stderr.write(
		local_stack_control.consumer.redact_evidence_logs(result.stderr, private_values)
	)
	return result.returncode


#============================================
def stop_replica_instance(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
	service: str,
	id_prefix: str,
) -> int:
	"""Stop one label-resolved API replica and prove a peer remains running."""
	before = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	local_stack_control.consumer.require_capability_snapshot(disposable, before)
	container = local_stack_control.consumer.replica_stop_container(
		disposable,
		before,
		service,
		id_prefix,
	)
	argv = ["podman", "stop", container.id]
	print("Command: " + shlex.join(argv))
	result = runner.stream(argv, cwd=disposable.target.repo_root)
	if result != 0:
		return result
	after = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	before_scope = (
		tuple(sorted(item.name for item in before.volumes)),
		tuple(sorted(item.name for item in before.networks)),
	)
	after_scope = (
		tuple(sorted(item.name for item in after.volumes)),
		tuple(sorted(item.name for item in after.networks)),
	)
	if before_scope != after_scope:
		raise local_stack_control.models.ControllerError(
			"replica stop changed labelled persistent resource scope"
		)
	local_stack_control.consumer.require_replica_stopped(disposable, after, container.id)
	return 0


#============================================
def run_postgresql_count(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
	attempt_id: str,
) -> int:
	"""Run and emit only the replica profile's five bounded durability counts."""
	local_stack_control.consumer.require_current_resource_capability(runner, disposable)
	argv, environment, sql = local_stack_control.consumer.postgresql_count_command(
		disposable, attempt_id
	)
	result = runner.run(argv, environment, disposable.target.repo_root, sql)
	if not result.ok():
		raise local_stack_control.models.ControllerError(
			"bounded PostgreSQL count did not complete"
		)
	counts = result.stdout.strip()
	if re.fullmatch(r"[0-9]{1,10}(?:\|[0-9]{1,10}){4}", counts) is None:
		raise local_stack_control.models.ControllerError(
			"bounded PostgreSQL count returned an invalid result"
		)
	print(counts)
	return 0


#============================================
def main() -> None:
	"""Run a closed Compose call or one exact disposable cleanup."""
	args = parse_args(sys.argv[1:])
	try:
		runner = local_stack_control.process.SubprocessRunner()
		root = repo_root()
		# The owned caller passes a non-secret manifest locator.  Runtime YAML
		# locates private files beneath its current private workspace.
		manifest = local_stack_control.consumer.load_manifest(root, args.manifest)
		disposable = local_stack_control.consumer.disposable_target(runner, root, manifest)
		if args.action != "diagnostics":
			local_stack_control.process.require_rootless_local_engine(runner, root)
		if args.action == "compose":
			local_stack_control.consumer.require_mutating_capability(runner, disposable)
			argv, environment = local_stack_control.consumer.compose_command(
				disposable,
				args.arguments,
			)
			result = runner.run(argv, environment, root)
			if result.ok():
				write_compose_success_output(result)
			else:
				private_values = local_stack_control.consumer.private_environment_values(
					disposable.target.env_file
				)
				diagnostic = compose_failure_diagnostics(result, private_values)
				if diagnostic != "":
					print(diagnostic, file=sys.stderr)
			raise SystemExit(result.returncode)
		if args.action == "launch":
			local_stack_control.consumer.require_mutating_capability(runner, disposable)
			result = local_stack_control.lifecycle.start_lifecycle(
				disposable,
				runner,
				root,
				local_stack_control.consumer.lifecycle_options(
					disposable, args.timeout_seconds
				),
			)
			print(f"Disposable stack ready: {result.gateway_url}")
			raise SystemExit(0)
		if args.action == "restart":
			local_stack_control.consumer.require_mutating_capability(runner, disposable)
			if args.service != local_stack_control.consumer.outage_service(disposable):
				raise local_stack_control.models.ControllerError(
					"disposable restart is limited to its declared outage service"
				)
			result = local_stack_control.lifecycle.restart_lifecycle(
				disposable,
				runner,
				root,
				args.service,
				local_stack_control.consumer.restart_options(
					disposable, args.timeout_seconds
				),
			)
			print(f"Disposable stack ready: {result.gateway_url}")
			raise SystemExit(0)
		if args.action == "stop-outage-service":
			completed = local_stack_control.consumer.stop_declared_outage_service(runner, disposable)
			print(f"Disposable outage stopped: {completed.service}")
			raise SystemExit(0)
		if args.action == "read-evidence-logs":
			result = read_evidence_logs(runner, disposable, args.claim)
			raise SystemExit(result)
		if args.action == "diagnostics":
			result = run_diagnostics(runner, disposable, tuple(args.service))
			raise SystemExit(result)
		if args.action == "stop-instance":
			local_stack_control.consumer.require_mutating_capability(runner, disposable)
			result = stop_replica_instance(
				runner,
				disposable,
				args.service,
				args.id_prefix,
			)
			raise SystemExit(result)
		if args.action == "postgresql-count":
			result = run_postgresql_count(
				runner,
				disposable,
				args.attempt_id,
			)
			raise SystemExit(result)

		before = local_stack_control.consumer.require_mutating_capability(runner, disposable)
		if len(before.containers) + len(before.volumes) + len(before.networks) == 0:
			print(f"Disposable cleanup: project {disposable.target.project} is already empty")
			result = remove_owned_project_images(runner, disposable)
			raise SystemExit(result)
		plan = local_stack_control.consumer.cleanup_plan(runner, disposable)
		print_cleanup_preview(plan)
		environment = local_stack_control.consumer.compose_environment(disposable)
		result = runner.stream(list(plan.argv), environment, root)
		if result != 0:
			raise SystemExit(result)
		require_empty_post_cleanup(runner, disposable)
		result = remove_owned_project_images(runner, disposable)
		raise SystemExit(result)
	except local_stack_control.models.ControllerError as error:
		print(f"ERROR: {error}", file=sys.stderr)
		raise SystemExit(2) from error


if __name__ == "__main__":
	main()
