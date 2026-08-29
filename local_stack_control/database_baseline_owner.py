"""Lease-owned PostgreSQL baseline acceptance lifecycle."""

from __future__ import annotations

import os
import pathlib
import socket
import subprocess
from collections.abc import Callable

import local_stack_control.browser_suite_lease
import local_stack_control.acceptance_profile_owner
import local_stack_control.compose
import local_stack_control.models
import local_stack_control.process
import local_stack_control.runtime_manifest


#============================================
def _select_loopback_port() -> int:
	"""Ask the kernel for one ephemeral loopback port after the lease is held."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		return int(listener.getsockname()[1])


#============================================
def _run_oracle(repository_root: pathlib.Path, workspace: pathlib.Path, port: int) -> None:
	"""Run the private shell oracle with one generated non-secret manifest locator."""
	local_stack_control.runtime_manifest.write_database_baseline_runtime(workspace, port)
	environment = {
		name: value
		for name, value in os.environ.items()
		if not name.startswith("PLE_") and not name.startswith("COMPOSE_")
	}
	result = subprocess.run(
		[
			"bash",
			str(repository_root / "tests/e2e/e2e_database_baseline.sh"),
			"--owned-child",
			"--runtime-manifest",
			local_stack_control.runtime_manifest.MANIFEST_NAME,
		],
		cwd=workspace,
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
	def profile_oracle(
		root: pathlib.Path, workspace: pathlib.Path, ports: tuple[int, ...]
	) -> None:
		oracle_runner(root, workspace, ports[0])

	local_stack_control.acceptance_profile_owner.run_owned_acceptance_profile(
		repository_root,
		"database baseline",
		profile_oracle,
		lease_factory,
		reset_runner_factory,
		lambda: (port_selector(),),
		port_checker,
	)


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
