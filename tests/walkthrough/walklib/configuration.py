"""Command-line and local configuration boundary for the UI walkthrough."""

import argparse
import os
import pathlib
import re
import stat

import walklib.models


UINT32_MAX = 4_294_967_295
SAFE_REPORT_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*\.json$")


#============================================
def parse_args(argv: list[str]) -> argparse.Namespace:
	"""Parse the small public CLI surface before any filesystem or Podman action."""
	parser = argparse.ArgumentParser(
		prog="run_ui_walkthrough",
		description="Run the real-gateway UI walkthrough smoke after fail-closed preflight."
	)
	parser.add_argument(
		"--master-seed",
		required=True,
		metavar="UINT32",
		help="Decimal unsigned 32-bit seed used by the deterministic walkthrough.",
	)
	parser.add_argument(
		"--env-file",
		default="containers/env.local",
		metavar="PATH",
		help="Compose environment file (default: containers/env.local).",
	)
	parser.add_argument(
		"--report-file",
		metavar="BASENAME",
		help="Safe .json basename below test-results/ui_walkthrough/.",
	)
	parser.add_argument(
		"--keep",
		action="store_true",
		help="Preserve a stack started by this runner for diagnosis.",
	)
	parser.add_argument(
		"--build",
		action="store_true",
		help="Force launch_local_stack.sh to rebuild dist/.",
	)
	parser.add_argument(
		"--instructor-setup-only",
		action="store_true",
		help="Run only the fixed J11/J12/J13 instructor setup child.",
	)
	parser.add_argument(
		"--student-repeat-only",
		action="store_true",
		help="Run J11/J12/J13 followed by the partial J1/J2/J3/J4 student-repeat slice.",
	)
	args = parser.parse_args(argv)
	return args


#============================================
def resolve_inputs(
	args: argparse.Namespace,
	repository_root: pathlib.Path,
) -> walklib.models.RunnerInputs:
	"""Validate CLI values and resolve paths without side effects.

	Args:
		args: Parsed public command-line values.
		repository_root: Repository root used to resolve relative paths.

	Returns:
		Validated immutable runner inputs.

	Raises:
		walklib.models.RunnerError: A value or selected path violates the public contract.
	"""
	seed_text = args.master_seed
	if not re.fullmatch(r"[0-9]+", seed_text):
		raise walklib.models.RunnerError(
			"--master-seed must be a decimal unsigned 32-bit integer"
		)
	master_seed = int(seed_text)
	if master_seed > UINT32_MAX:
		raise walklib.models.RunnerError(
			"--master-seed must be a decimal unsigned 32-bit integer"
		)

	env_file = pathlib.Path(args.env_file)
	if not env_file.is_absolute():
		env_file = repository_root / env_file
	validate_regular_readable_file(env_file, "selected walkthrough env file")

	report_basename = args.report_file
	if report_basename is None:
		report_basename = f"ui_walkthrough_seed_{master_seed}.json"
	validate_report_basename(report_basename)

	if args.instructor_setup_only and args.student_repeat_only:
		raise walklib.models.RunnerError(
			"--instructor-setup-only and --student-repeat-only cannot be combined"
		)
	inputs = walklib.models.RunnerInputs(
		master_seed,
		env_file,
		report_basename,
		args.keep,
		args.build,
		args.instructor_setup_only,
		args.student_repeat_only,
	)
	return inputs


#============================================
def validate_regular_readable_file(path: pathlib.Path, description: str) -> None:
	"""Require a readable regular non-symlink file at a security boundary."""
	if path.is_symlink():
		raise walklib.models.RunnerError(f"{description} must not be a symlink")
	if not path.is_file() or not os.access(path, os.R_OK):
		raise walklib.models.RunnerError(f"{description} is missing or unreadable")


#============================================
def validate_report_basename(report_basename: str) -> None:
	"""Reject traversal and unsafe names before creating the private report directory."""
	if "/" in report_basename or ".." in report_basename:
		raise walklib.models.RunnerError(
			"--report-file must not contain a path or traversal"
		)
	if not SAFE_REPORT_NAME.fullmatch(report_basename):
		raise walklib.models.RunnerError(
			"--report-file must contain only safe ASCII filename characters and end in .json"
		)


#============================================
def has_reusable_dist(repository_root: pathlib.Path) -> bool:
	"""Return whether the exact publish outputs can safely be reused without building."""
	for relative_path in ("dist/index.html", "dist/main.js"):
		path = repository_root / relative_path
		if path.is_symlink() or not path.is_file() or not os.access(path, os.R_OK):
			return False
	return True


#============================================
def reuse_existing_dist(
	inputs: walklib.models.RunnerInputs,
	repository_root: pathlib.Path,
) -> bool:
	"""Reuse safe publish outputs unless the public CLI explicitly requests a build."""
	return not inputs.force_build and has_reusable_dist(repository_root)


#============================================
def env_value(env_file: pathlib.Path, setting_name: str) -> str:
	"""Read the last exact assignment from a Compose env file without sourcing it."""
	value = ""
	for line in env_file.read_text(encoding="ascii").splitlines():
		if "=" not in line:
			continue
		key, candidate = line.split("=", 1)
		if key == setting_name:
			value = candidate.strip()
	return value


#============================================
def effective_gateway_port(
	inputs: walklib.models.RunnerInputs,
	environ: dict[str, str],
) -> int:
	"""Resolve the gateway port using the launcher's exact precedence.

	Args:
		inputs: Validated runner inputs containing the selected environment file.
		environ: Process environment whose nonempty port takes precedence.

	Returns:
		A validated TCP port in the inclusive range 1 through 65535.

	Raises:
		walklib.models.RunnerError: The effective port is malformed or out of range.
	"""
	configured_port = env_value(inputs.env_file, "PLE_GATEWAY_HOST_PORT")
	inherited_port = environ.get("PLE_GATEWAY_HOST_PORT", "")
	port_text = inherited_port if inherited_port else configured_port
	if not port_text:
		port_text = "8080"
	if not re.fullmatch(r"[0-9]+", port_text):
		raise walklib.models.RunnerError(
			"PLE_GATEWAY_HOST_PORT must be an unquoted integer"
		)
	port = int(port_text)
	if port < 1 or port > 65_535:
		raise walklib.models.RunnerError(
			"PLE_GATEWAY_HOST_PORT must be between 1 and 65535"
		)
	return port


#============================================
def validate_compose_project_name(
	inputs: walklib.models.RunnerInputs,
	environ: dict[str, str],
) -> None:
	"""Refuse project-name overrides that would make cleanup ownership ambiguous."""
	configured_name = env_value(inputs.env_file, "COMPOSE_PROJECT_NAME")
	effective_name = environ.get("COMPOSE_PROJECT_NAME", "") or configured_name
	if effective_name and effective_name != "containers":
		raise walklib.models.RunnerError(
			"COMPOSE_PROJECT_NAME must be unset, empty, or exactly containers"
		)


#============================================
def credential_file(inputs: walklib.models.RunnerInputs) -> pathlib.Path:
	"""Return the selected env file's sibling local login boundary without reading it."""
	path = inputs.env_file.parent / "local-login.txt"
	return path


#============================================
def validate_credential_file(path: pathlib.Path) -> None:
	"""Require the post-launch login handoff file to be private, regular, and readable."""
	validate_regular_readable_file(path, "required sibling local-login.txt")
	mode = stat.S_IMODE(path.stat().st_mode)
	if mode != 0o600:
		raise walklib.models.RunnerError(
			"required sibling local-login.txt must have mode 0600"
		)


#============================================
def validate_existing_credential_file(inputs: walklib.models.RunnerInputs) -> None:
	"""Fail early only when an existing sibling credential is malformed or unsafe."""
	path = credential_file(inputs)
	if path.exists() or path.is_symlink():
		validate_credential_file(path)
