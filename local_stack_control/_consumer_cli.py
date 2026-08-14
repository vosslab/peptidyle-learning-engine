"""Private adapter for closed disposable local-stack E2E owners."""

import argparse
import pathlib
import shlex
import sys

import local_stack_control.compose
import local_stack_control.consumer
import local_stack_control.discovery
import local_stack_control.models
import local_stack_control.process


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
	diagnostics = actions.add_parser("diagnostics")
	diagnostics.add_argument("--manifest", required=True, type=pathlib.Path)
	diagnostics.add_argument("--service", action="append", default=[])
	stop_instance = actions.add_parser("stop-instance")
	stop_instance.add_argument("--manifest", required=True, type=pathlib.Path)
	stop_instance.add_argument("--service", required=True)
	stop_instance.add_argument("--id-prefix", required=True)
	args = parser.parse_args(argv)
	if args.action == "compose":
		if len(args.arguments) > 0 and args.arguments[0] == "--":
			args.arguments = args.arguments[1:]
		if len(args.arguments) == 0:
			parser.error("compose requires an explicit Compose command")
	return args


#============================================
def repo_root(runner: local_stack_control.process.CommandRunner) -> pathlib.Path:
	"""Anchor this private entry point to its repository checkout."""
	return local_stack_control.compose.repo_root_from_entrypoint(pathlib.Path(__file__), runner)


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
	selected = local_stack_control.consumer.diagnostic_services(disposable, services)
	environment = local_stack_control.consumer.compose_environment(disposable)
	commands = (
		local_stack_control.consumer.compose_command(disposable, ["ps"])[0],
		local_stack_control.consumer.compose_command(
			disposable,
			["logs", "--no-color", "--tail", "80", *selected],
		)[0],
	)
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
def main() -> None:
	"""Run a closed Compose call or one exact disposable cleanup."""
	args = parse_args(sys.argv[1:])
	try:
		runner = local_stack_control.process.SubprocessRunner()
		root = repo_root(runner)
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
			result = runner.stream(argv, environment, root)
			raise SystemExit(result)
		if args.action == "launch":
			local_stack_control.consumer.require_mutating_capability(runner, disposable)
			argv, environment = local_stack_control.consumer.launch_command(
				disposable,
				args.timeout_seconds,
			)
			print("Command: " + shlex.join(argv))
			result = runner.stream(argv, environment, root)
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
