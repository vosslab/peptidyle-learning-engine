"""Lease-owned PostgreSQL 17 SD1 staging-oracle lifecycle."""

from __future__ import annotations

import os
import pathlib
import socket
import subprocess
from collections.abc import Callable

import local_stack_control.acceptance_profile_owner
import local_stack_control.browser_suite_lease
import local_stack_control.compose
import local_stack_control.models
import local_stack_control.process
import local_stack_control.runtime_manifest


#============================================
def _select_loopback_port() -> int:
	"""Ask the kernel for one loopback port while the fixed lease is held."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		return int(listener.getsockname()[1])


#============================================
def _run_oracle(repository_root: pathlib.Path, workspace: pathlib.Path, port: int) -> None:
	"""Run the private SD1 staging oracle with its generated manifest locator."""
	# ASVS 1.2.4 and 2.2.1: the child receives only a validated private locator;
	# it cannot select a database URL, Compose project, or migration directory.
	# The root manifest remains the fixed Compose/cleanup authority. The SD1
	# writer adds a separate nested migrator-only manifest for project-tools.
	local_stack_control.runtime_manifest.write_database_baseline_runtime(workspace, port)
	local_stack_control.runtime_manifest.write_sd1_staged_database_runtime(workspace, port)
	environment = {
		name: value
		for name, value in os.environ.items()
		if not name.startswith("PLE_") and not name.startswith("COMPOSE_")
	}
	result = subprocess.run(
		[
			"bash",
			str(repository_root / "tests/e2e/e2e_sd1_staged_database.sh"),
			"--owned-child",
			"--runtime-manifest",
			local_stack_control.runtime_manifest.MANIFEST_NAME,
		],
		cwd=workspace,
		env=environment,
		check=False,
	)
	if result.returncode != 0:
		raise local_stack_control.models.ControllerError(
			"SD1 staged PostgreSQL oracle failed"
		)


#============================================
def run_owned_sd1_staged_database(
	repository_root: pathlib.Path,
	oracle_runner: Callable[[pathlib.Path, pathlib.Path, int], None] = _run_oracle,
	acquire_browser_suite_lease: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	create_command_runner: Callable[[], local_stack_control.process.CommandRunner] = local_stack_control.process.SubprocessRunner,
	port_selector: Callable[[], int] = _select_loopback_port,
	port_checker: Callable[[tuple[int, ...], local_stack_control.process.CommandRunner, pathlib.Path], None] = local_stack_control.process.require_available_loopback_ports,
) -> None:
	"""Run one serial SD1 staging oracle with exact fresh and final resets."""
	def profile_oracle(
		root: pathlib.Path, workspace: pathlib.Path, ports: tuple[int, ...]
	) -> None:
		oracle_runner(root, workspace, ports[0])

	local_stack_control.acceptance_profile_owner.run_owned_acceptance_profile(
		repository_root,
		"SD1 staged database",
		profile_oracle,
		acquire_browser_suite_lease,
		create_command_runner,
		lambda: (port_selector(),),
		port_checker,
	)


#============================================
def main() -> None:
	"""Run the only public SD1 staging-oracle lifecycle entry point."""
	repository_root = local_stack_control.compose.repo_root_from_entrypoint(
		pathlib.Path(__file__)
	)
	run_owned_sd1_staged_database(repository_root)
	print("SD1 staged PostgreSQL: PASS")


if __name__ == "__main__":
	main()
