"""Lease-owned PostgreSQL baseline acceptance lifecycle."""

from __future__ import annotations

import os
import pathlib
import socket
import stat
import subprocess
from collections.abc import Callable

import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.compose
import local_stack_control.models
import local_stack_control.process


#============================================
def _resources_empty(snapshot: local_stack_control.models.ProjectSnapshot) -> bool:
	"""Report whether the fixed browser-owner inventory is empty."""
	return not (snapshot.containers or snapshot.volumes or snapshot.networks)


#============================================
def _select_loopback_port() -> int:
	"""Ask the kernel for one ephemeral loopback port after the lease is held."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		return int(listener.getsockname()[1])


#============================================
def _write_owner_input(workspace: pathlib.Path) -> pathlib.Path:
	"""Write the private owner handoff that enables the shell oracle child."""
	path = workspace / "database-baseline.owner"
	descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	try:
		os.write(descriptor, b"lease-held\n")
	finally:
		os.close(descriptor)
	metadata = path.stat()
	if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
		raise local_stack_control.models.ControllerError("database baseline owner input is unavailable")
	return path


#============================================
def _run_oracle(repository_root: pathlib.Path, workspace: pathlib.Path, port: int) -> None:
	"""Run the private shell oracle while its Python owner retains the lease."""
	environment = os.environ.copy()
	environment["PLE_DATABASE_BASELINE_OWNER_INPUT"] = str(_write_owner_input(workspace))
	environment["PLE_DATABASE_BASELINE_WORKSPACE"] = str(workspace)
	environment["PLE_DATABASE_BASELINE_PORT"] = str(port)
	result = subprocess.run(
		["bash", "tests/e2e/e2e_database_baseline.sh", "--owned-child"],
		cwd=repository_root,
		env=environment,
		check=False,
	)
	if result.returncode != 0:
		raise local_stack_control.models.ControllerError("database baseline oracle failed")


#============================================
def run_owned_database_baseline(
	repository_root: pathlib.Path,
	oracle_runner: Callable[[pathlib.Path, pathlib.Path, int], None] = _run_oracle,
	lease_factory: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	reset_runner_factory: Callable[[], local_stack_control.process.CommandRunner] = local_stack_control.process.SubprocessRunner,
	port_selector: Callable[[], int] = _select_loopback_port,
	port_checker: Callable[[tuple[int, ...], local_stack_control.process.CommandRunner, pathlib.Path], None] = local_stack_control.process.require_available_loopback_ports,
) -> None:
	"""Run one serial database oracle with fresh and final fixed-stack resets."""
	lease = lease_factory(repository_root)
	failures: list[BaseException] = []
	try:
		local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease, reset_runner_factory(), repository_root
		)
		workspace = lease.reset_workspace()
		port = port_selector()
		port_checker((port,), reset_runner_factory(), repository_root)
		try:
			oracle_runner(repository_root, workspace, port)
		except BaseException as error:
			failures.append(error)
	finally:
		try:
			final_snapshot = local_stack_control.browser_suite_reset.reset_live_demo_browser(
				lease, reset_runner_factory(), repository_root
			)
			if not _resources_empty(final_snapshot):
				raise local_stack_control.models.ControllerError(
					"database baseline final reset left fixed browser resources"
				)
			workspace = lease.reset_workspace()
			if any(workspace.iterdir()):
				raise local_stack_control.models.ControllerError(
					"database baseline final workspace is not empty"
				)
		except BaseException as error:
			failures.append(error)
		finally:
			lease.release()
	if len(failures) == 1:
		raise failures[0]
	if failures:
		raise BaseExceptionGroup("database baseline lifecycle failures", failures)


#============================================
def main() -> None:
	"""Run the only public database-baseline lifecycle entry point."""
	repository_root = local_stack_control.compose.repo_root_from_entrypoint(
		pathlib.Path(__file__)
	)
	run_owned_database_baseline(repository_root)
	print("Database-baseline: PASS")


if __name__ == "__main__":
	main()
