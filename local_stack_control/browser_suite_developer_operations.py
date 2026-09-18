"""External stack operations for the fixed Developer Browser Suite supervisor."""

# Standard Library
import os
import sys
import signal
import secrets
import pathlib
import subprocess
import dataclasses
from collections.abc import Callable

# local repo modules
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.env_file
import local_stack_control.live_demo_target
import local_stack_control.live_demo_gateway
import local_stack_control.models
import local_stack_control.process
from local_stack_control.browser_suite_private_state import DeveloperBrowserSuiteError


LIFECYCLE_LAUNCH_TIMEOUT_SECONDS = 240.0


@dataclasses.dataclass(frozen=True)
class RunningDeveloperStack:
	"""Private launch state retained exclusively by the supervisor."""

	manifest_path: pathlib.Path
	origin: str


@dataclasses.dataclass(frozen=True)
class DeveloperOperations:
	"""External stack operations kept injectable for deterministic lifecycle tests."""

	start: Callable[[local_stack_control.browser_suite_lease.BrowserSuiteLease, pathlib.Path, pathlib.Path], RunningDeveloperStack]
	stop: Callable[[RunningDeveloperStack, pathlib.Path], None]
	verify_empty: Callable[[local_stack_control.browser_suite_lease.BrowserSuiteLease, pathlib.Path], None]
	# Called from the supervisor's termination handler so a launch in progress stops
	# within seconds instead of running its complete build before cleanup.
	interrupt: Callable[[], None]


#============================================
def report_phase(text: str) -> None:
	"""Leave one non-secret progress line in the supervisor log."""
	print(text, file=sys.stderr, flush=True)


#============================================
def stream_launch(
	argv: list[str],
	environment: dict[str, str],
	root: pathlib.Path,
	active: list[object],
) -> tuple[int, str]:
	"""Run the launch child, echoing every output line to the supervisor log as it appears.

	The lines are also retained so a failure keeps the same diagnostic the captured
	form provided.  The child leads its own process group so an interrupt reaches the
	build and Compose processes beneath it.
	"""
	child = subprocess.Popen(
		argv,
		stdin=subprocess.DEVNULL,
		stdout=subprocess.PIPE,
		stderr=subprocess.STDOUT,
		text=True,
		env=environment,
		cwd=root,
		start_new_session=True,
	)
	active.append(child)
	lines: list[str] = []
	try:
		if child.stdout is None:
			raise DeveloperBrowserSuiteError("developer browser launch output is unavailable")
		for line in child.stdout:
			sys.stderr.write(line)
			sys.stderr.flush()
			lines.append(line)
		returncode = child.wait()
	finally:
		active.clear()
	output = "".join(lines)
	return returncode, output


#============================================
def interrupt_launch(active: list[object]) -> None:
	"""Send SIGTERM to the launch child's process group when a launch is running."""
	for child in tuple(active):
		if not isinstance(child, subprocess.Popen) or child.poll() is not None:
			continue
		try:
			os.killpg(child.pid, signal.SIGTERM)
		except ProcessLookupError:
			continue


#============================================
def adapter_argv(
	action: str,
	manifest_path: pathlib.Path,
	arguments: tuple[str, ...] = (),
) -> list[str]:
	"""Form one closed adapter command for the shared fixed target."""
	result = [
		sys.executable,
		"-m",
		"local_stack_control.disposable_stack_command",
		action,
		"--manifest",
		str(manifest_path),
	]
	result.extend(arguments)
	return result


#============================================
def default_operations(
	without_live_demo: bool,
	launch_diagnostic: Callable[[str], str],
) -> DeveloperOperations:
	"""Use the same production manifest, gateway, and auth path as Playwright."""
	# The launch child in progress, if any; shared with the interrupt operation.
	active_launch: list[object] = []

	def start_stack(
		lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		root: pathlib.Path,
		workspace: pathlib.Path,
	) -> RunningDeveloperStack:
		"""Start the production browser stack and wait for its declared readiness."""
		runner = local_stack_control.process.SubprocessRunner()
		report_phase("Step: choosing ports and writing the private target")
		selections = local_stack_control.env_file.tracked_stack_selections(root)
		ports = local_stack_control.live_demo_target.random_ports()
		local_stack_control.process.require_available_loopback_ports(
			ports.as_tuple(), runner, root
		)
		target = local_stack_control.live_demo_target.write_private_target(
			workspace,
			local_stack_control.models.LiveDemoProfile.BROWSER,
			ports,
			selections,
		)
		report_phase("Step: validating the production auth render")
		local_stack_control.live_demo_target.validate_production_auth_render(
			runner, root, target.manifest_path
		)
		arguments = (
			"--timeout-seconds",
			str(int(LIFECYCLE_LAUNCH_TIMEOUT_SECONDS)),
		)
		if without_live_demo:
			arguments += ("--without-live-demo",)
		argv = adapter_argv("launch", target.manifest_path, arguments)
		environment = local_stack_control.env_file.sanitized_runtime_environment(
			local_stack_control.process.current_environment()
		)
		environment["PLE_BROWSER_SUITE_OWNER_SESSION"] = "ple-owner-" + secrets.token_hex(16)
		report_phase("Step: launching the production stack (host build, images, database)")
		returncode, output = stream_launch(argv, environment, root, active_launch)
		if returncode != 0:
			diagnostic = launch_diagnostic(output)
			raise DeveloperBrowserSuiteError("developer browser stack launch failed: " + diagnostic)
		report_phase("Step: trusting the browser certificate")
		local_stack_control.live_demo_gateway.write_browser_certificate_trust(
			runner, root, workspace, target.origin
		)
		return RunningDeveloperStack(target.manifest_path, target.origin)

	def interrupt_stack_launch() -> None:
		"""Stop a launch in progress so termination does not wait for the whole build."""
		interrupt_launch(active_launch)

	def stop_stack(running: RunningDeveloperStack, root: pathlib.Path) -> None:
		"""Stop the production browser stack and report cleanup failures."""
		result = local_stack_control.process.stream_in_owner_session(
			local_stack_control.process.SubprocessRunner(),
			adapter_argv("cleanup", running.manifest_path),
			None,
			root,
		)
		if result.returncode != 0:
			raise DeveloperBrowserSuiteError("developer browser stack cleanup failed")

	def verify_empty_workspace(
		lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		root: pathlib.Path,
	) -> None:
		"""Verify that owned resources and private workspace artifacts are absent."""
		snapshot = local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, local_stack_control.process.SubprocessRunner(), root
		)
		if snapshot.containers or snapshot.volumes or snapshot.networks:
			raise DeveloperBrowserSuiteError("developer browser cleanup left owned resources")
		workspace = lease.reset_workspace()
		if tuple(workspace.iterdir()):
			raise DeveloperBrowserSuiteError(
				"developer browser cleanup left private workspace artifacts"
			)

	return DeveloperOperations(
		start_stack, stop_stack, verify_empty_workspace, interrupt_stack_launch
	)
