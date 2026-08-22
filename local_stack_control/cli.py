"""Argument parser and concise error boundary for local stack control."""

import sys
import pathlib
import argparse
import collections.abc

import local_stack_control.commands
import local_stack_control.models
import local_stack_control.process


CommandHandler = collections.abc.Callable[
	[argparse.Namespace, local_stack_control.process.CommandRunner, pathlib.Path],
	int,
]


#============================================
def add_target_options(parser: argparse.ArgumentParser) -> None:
	"""Add explicit environment and topology options."""
	parser.add_argument(
		"--env-file",
		default=local_stack_control.models.DEFAULT_ENV_FILE,
		help="Compose env file; default: containers/env.local",
	)
	parser.add_argument("--with-smtp", action="store_true", help="include SMTP overlay")


#============================================
def log_tail(value: str) -> str:
	"""Accept the Compose log forms that do not create ambiguous argv values."""
	if value == "all":
		return value
	if value.isascii() and value.isdecimal():
		return value
	raise argparse.ArgumentTypeError("tail must be 'all' or a non-negative integer")


#============================================
def build_parser() -> argparse.ArgumentParser:
	"""Build the supported operator command surface."""
	parser = argparse.ArgumentParser(
		prog="python3 local_stack.py",
		description="Inspect and control the local PLE Podman stack.",
	)
	subparsers = parser.add_subparsers(dest="command", required=True)

	doctor = subparsers.add_parser("doctor", help="read-only Podman diagnostics")
	add_target_options(doctor)
	doctor.add_argument("--json", action="store_true")
	doctor.set_defaults(handler=local_stack_control.commands.doctor)

	projects = subparsers.add_parser("projects", help="list labelled Compose projects")
	projects.add_argument("--json", action="store_true")
	projects.set_defaults(handler=local_stack_control.commands.projects)

	status = subparsers.add_parser("status", help="report semantic stack readiness")
	add_target_options(status)
	status.add_argument("--project", help="explicit read-only Compose project")
	status.add_argument("--json", action="store_true")
	status.set_defaults(handler=local_stack_control.commands.status)

	logs = subparsers.add_parser("logs", help="show scoped application logs")
	add_target_options(logs)
	logs.add_argument("--project", help="explicit read-only Compose project")
	logs.add_argument("--tail", type=log_tail, default="120")
	logs.add_argument("--follow", action="store_true")
	logs.add_argument("services", nargs="*")
	logs.set_defaults(handler=local_stack_control.commands.logs)

	start = subparsers.add_parser("start", help="start the fixed production-browser developer session")
	start.add_argument("--no-open", action="store_true")
	start.set_defaults(handler=local_stack_control.commands.start)

	stop = subparsers.add_parser("stop", help="clean up the fixed production-browser developer session")
	stop.set_defaults(handler=local_stack_control.commands.stop)

	restart = subparsers.add_parser("restart", help="restart api, worker, gateway, or webwork-renderer")
	add_target_options(restart)
	restart.add_argument("service")
	restart.set_defaults(handler=local_stack_control.commands.restart)

	service = subparsers.add_parser("service", help="perform one narrowly scoped service action")
	service_actions = service.add_subparsers(dest="service_action", required=True)
	service_stop = service_actions.add_parser("stop", help="stop the default WebWork renderer")
	add_target_options(service_stop)
	service_stop.add_argument("service", choices=local_stack_control.models.STOPPABLE_SERVICES)
	service_stop.set_defaults(handler=local_stack_control.commands.service_stop)

	reset = subparsers.add_parser("reset", help="remove confirmed default-stack Compose data")
	add_target_options(reset)
	reset.add_argument("--confirm-project")
	reset.add_argument("--dry-run", action="store_true")
	reset.set_defaults(handler=local_stack_control.commands.reset)

	validate = subparsers.add_parser("validate", help="read-only check of initialized config and available engine")
	add_target_options(validate)
	validate.add_argument("--json", action="store_true")
	validate.set_defaults(handler=local_stack_control.commands.validate)

	acceptance = subparsers.add_parser("acceptance", help="run no-skip live browser validation")
	acceptance.set_defaults(handler=local_stack_control.commands.acceptance)

	return parser


#============================================
def run(
	argv: list[str],
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> int:
	"""Parse and execute one command through an injected runner."""
	args = build_parser().parse_args(argv)
	handler: CommandHandler = args.handler
	try:
		result = handler(args, runner, repo_root)
	except local_stack_control.models.ControllerError as error:
		print(f"ERROR: {error}", file=sys.stderr)
		return 2
	return result
