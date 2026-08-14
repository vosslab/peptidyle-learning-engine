"""Typed subprocess boundary for local stack commands."""

import os
import json
import pathlib
import shutil
import subprocess
import abc

import local_stack_control.models
import local_stack_control.env_file


class CommandRunner(abc.ABC):
	"""Abstract command runner used by controller decisions."""

	@abc.abstractmethod
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Run a captured command."""

	@abc.abstractmethod
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Run a command with inherited output."""


class SubprocessRunner(CommandRunner):
	"""Standard-library command runner."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Run a command and capture its output."""
		if shutil.which(argv[0]) is None:
			result = local_stack_control.models.CommandResult(
				argv=tuple(argv),
				returncode=127,
				stdout="",
				stderr=f"{argv[0]} not found on PATH",
			)
			return result

		base_environment = current_environment() if environment is None else environment
		effective_environment = local_stack_control.env_file.sanitized_runtime_environment(
			base_environment
		)
		completed = subprocess.run(
			argv,
			check=False,
			capture_output=True,
			text=True,
			env=effective_environment,
			cwd=cwd,
		)
		result = local_stack_control.models.CommandResult(
			argv=tuple(argv),
			returncode=completed.returncode,
			stdout=completed.stdout,
			stderr=completed.stderr,
		)
		return result

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Run a command with inherited output."""
		if shutil.which(argv[0]) is None:
			raise local_stack_control.models.ControllerError(
				f"{argv[0]} not found on PATH"
			)
		base_environment = current_environment() if environment is None else environment
		effective_environment = local_stack_control.env_file.sanitized_runtime_environment(
			base_environment
		)
		completed = subprocess.run(argv, check=False, env=effective_environment, cwd=cwd)
		result = completed.returncode
		return result


#============================================
def current_environment() -> dict[str, str]:
	"""Return a mutable copy of the process environment."""
	environment = dict(os.environ)
	return environment


#============================================
def rootless_from_podman_info(info_text: str) -> bool:
	"""Decode the active Podman connection's rootless state from typed JSON."""
	try:
		info = json.loads(info_text)
	except json.JSONDecodeError as error:
		raise local_stack_control.models.ControllerError(
			"podman info returned invalid JSON while checking the rootless engine"
		) from error
	if not isinstance(info, dict):
		raise local_stack_control.models.ControllerError(
			"podman info returned unexpected JSON while checking the rootless engine"
		)
	host = info.get("host")
	if not isinstance(host, dict):
		raise local_stack_control.models.ControllerError(
			"podman info has no host metadata for the rootless engine check"
		)
	security = host.get("security")
	if not isinstance(security, dict) or not isinstance(security.get("rootless"), bool):
		raise local_stack_control.models.ControllerError(
			"podman info has no rootless security metadata"
		)
	result = security["rootless"]
	return result


#============================================
def require_rootless_local_engine(
	runner: CommandRunner,
	repo_root: pathlib.Path,
) -> None:
	"""Require the active default Podman connection to be rootless before mutation."""
	result = runner.run(
		["podman", "info", "--format", "json"],
		local_stack_control.env_file.sanitized_runtime_environment(current_environment()),
		repo_root,
	)
	if not result.ok():
		detail = result.stderr.strip() or "Podman engine is unavailable"
		raise local_stack_control.models.ControllerError(
			"a rootless Podman engine is required before changing the local stack: " + detail
		)
	if not rootless_from_podman_info(result.stdout):
		raise local_stack_control.models.ControllerError(
			"the active default Podman connection is not rootless; select or start the "
			"rootless local Podman machine, then retry"
		)


#============================================
def require_available_loopback_ports(
	ports: tuple[int, ...],
	runner: CommandRunner,
	repo_root: pathlib.Path,
) -> None:
	"""Fail when a caller-owned loopback port already has a TCP listener."""
	for port in ports:
		if port < 1 or port > 65535:
			raise local_stack_control.models.ControllerError(
				f"loopback port {port} is outside the supported range"
			)
		result = runner.run(
			["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"],
			cwd=repo_root,
		)
		if result.returncode not in (0, 1):
			raise local_stack_control.models.ControllerError(
				"cannot inspect local ports before starting a disposable stack"
			)
		if result.stdout.strip() != "":
			raise local_stack_control.models.ControllerError(
				f"local port {port} is already listening"
			)
