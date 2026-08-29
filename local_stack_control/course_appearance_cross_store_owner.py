"""Lease-owned PostgreSQL and MinIO course-appearance acceptance lifecycle."""

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
	"""Ask the kernel for one loopback port while the suite lease is held."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		result = int(listener.getsockname()[1])
	return result


#============================================
def _select_loopback_ports() -> tuple[int, int]:
	"""Select distinct PostgreSQL and MinIO loopback ports for one private runtime."""
	postgres_port = _select_loopback_port()
	minio_port = _select_loopback_port()
	if postgres_port == minio_port:
		raise local_stack_control.models.ControllerError("course appearance cross-store ports are invalid")
	result = postgres_port, minio_port
	return result


#============================================
def _run_oracle(repository_root: pathlib.Path, workspace: pathlib.Path, ports: tuple[int, ...]) -> None:
	"""Run the private cross-store child with only its non-secret manifest locator."""
	if len(ports) != 2:
		raise local_stack_control.models.ControllerError("course appearance cross-store ports are invalid")
	local_stack_control.runtime_manifest.write_course_appearance_cross_store_runtime(
		workspace, ports[0], ports[1]
	)
	environment = {
		name: value
		for name, value in os.environ.items()
		if not name.startswith("PLE_") and not name.startswith("COMPOSE_")
	}
	result = subprocess.run(
		[
			"bash",
			str(repository_root / "tests/e2e/e2e_course_appearance.sh"),
			"--owned-child",
			"--runtime-manifest",
			local_stack_control.runtime_manifest.MANIFEST_NAME,
		],
		cwd=workspace,
		env=environment,
		check=False,
	)
	if result.returncode != 0:
		raise local_stack_control.models.ControllerError("course appearance cross-store oracle failed")


#============================================
def run_owned_course_appearance_cross_store(
	repository_root: pathlib.Path,
	oracle_runner: Callable[[pathlib.Path, pathlib.Path, tuple[int, ...]], None] = _run_oracle,
	lease_factory: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	reset_runner_factory: Callable[[], local_stack_control.process.CommandRunner] = local_stack_control.process.SubprocessRunner,
	ports_selector: Callable[[], tuple[int, int]] = _select_loopback_ports,
	port_checker: Callable[[tuple[int, ...], local_stack_control.process.CommandRunner, pathlib.Path], None] = local_stack_control.process.require_available_loopback_ports,
) -> None:
	"""Run the fixed cross-store oracle with one serial lease and two final resets."""
	local_stack_control.acceptance_profile_owner.run_owned_acceptance_profile(
		repository_root,
		"course appearance cross-store",
		oracle_runner,
		lease_factory,
		reset_runner_factory,
		ports_selector,
		port_checker,
	)


#============================================
def main() -> None:
	"""Run the public cross-store profile entry point."""
	repository_root = local_stack_control.compose.repo_root_from_entrypoint(pathlib.Path(__file__))
	run_owned_course_appearance_cross_store(repository_root)
	print("Course-appearance cross-store: PASS")


if __name__ == "__main__":
	main()
