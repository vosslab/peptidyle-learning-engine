"""External stack operations for the fixed Developer Browser Suite supervisor."""

from __future__ import annotations

# Standard Library
import dataclasses
import pathlib
import secrets
import subprocess
import sys
from collections.abc import Callable

# local repo modules
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.env_file
import local_stack_control.live_demo_target
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
	provision_stop_after: str | None,
	launch_diagnostic: Callable[[str], str],
) -> DeveloperOperations:
	"""Use the same production manifest, gateway, and auth path as Playwright."""
	def start_stack(
		lease: local_stack_control.browser_suite_lease.BrowserSuiteLease,
		root: pathlib.Path,
		workspace: pathlib.Path,
	) -> RunningDeveloperStack:
		"""Start the production browser stack and wait for its declared readiness."""
		runner = local_stack_control.process.SubprocessRunner()
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
		local_stack_control.live_demo_target.validate_production_auth_render(
			runner, root, target.manifest_path
		)
		arguments = (
			"--timeout-seconds",
			str(int(LIFECYCLE_LAUNCH_TIMEOUT_SECONDS)),
		)
		if provision_stop_after is not None:
			arguments += ("--stop-after", provision_stop_after)
		argv = adapter_argv("launch", target.manifest_path, arguments)
		environment = local_stack_control.env_file.sanitized_runtime_environment(
			local_stack_control.process.current_environment()
		)
		environment["PLE_BROWSER_SUITE_OWNER_SESSION"] = "ple-owner-" + secrets.token_hex(16)
		completed = subprocess.run(
			argv, check=False, capture_output=True, text=True, env=environment, cwd=root,
		)
		if completed.returncode != 0:
			diagnostic = launch_diagnostic(completed.stdout + "\n" + completed.stderr)
			raise DeveloperBrowserSuiteError("developer browser stack launch failed: " + diagnostic)
		return RunningDeveloperStack(target.manifest_path, target.origin)

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

	return DeveloperOperations(start_stack, stop_stack, verify_empty_workspace)
