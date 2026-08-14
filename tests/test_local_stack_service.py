"""Offline contracts for the narrowly scoped local renderer outage action."""

import argparse
import pathlib

import pytest

import local_stack_control.cli
import local_stack_control.commands
import local_stack_control.models


#============================================
def target(repo_root: pathlib.Path) -> local_stack_control.models.ComposeTarget:
	"""Build the private default target required by a service outage."""
	env_file = repo_root / "env.local"
	env_file.write_text("LOCAL_SECRET=private\n", encoding="ascii")
	env_file.chmod(0o600)
	compose_file = repo_root / "compose.yaml"
	compose_file.write_text("services: {}\n", encoding="ascii")
	return local_stack_control.models.ComposeTarget(
		repo_root=repo_root,
		project="containers",
		env_file=env_file,
		compose_files=(compose_file,),
		provider=local_stack_control.models.ComposeProvider(
			argv=("podman", "compose"),
			name="podman compose",
		),
		with_smtp=False,
		env_setting_names=("LOCAL_SECRET",),
	)


#============================================
def renderer(*, running: bool) -> local_stack_control.models.ContainerResource:
	"""Build one inspected renderer record with no generated-name authority."""
	return local_stack_control.models.ContainerResource(
		id="engine-private-id",
		names=("renderer",),
		project="containers",
		service="webwork-renderer",
		state="running" if running else "exited",
		running=running,
		exit_code=0,
		health="healthy" if running else None,
		image="local-renderer",
		ports=(),
	)


#============================================
def snapshot(*containers: local_stack_control.models.ContainerResource) -> local_stack_control.models.ProjectSnapshot:
	"""Build a controller snapshot without live Podman discovery."""
	return local_stack_control.models.ProjectSnapshot(
		project="containers",
		containers=containers,
		volumes=(),
		networks=(),
	)


#============================================
def test_service_stop_plan_rejects_another_service(tmp_path: pathlib.Path) -> None:
	"""The outage action cannot be repurposed for arbitrary services."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.commands.service_stop_plan(
			target(tmp_path), snapshot(renderer(running=True)), "api"
		)


#============================================
def test_service_stop_plan_requires_one_running_renderer(tmp_path: pathlib.Path) -> None:
	"""A stopped renderer fails before a second outage command exists."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.commands.service_stop_plan(
			target(tmp_path),
			snapshot(renderer(running=False)),
			"webwork-renderer",
		)


#============================================
def test_service_stop_plan_rejects_duplicate_renderer_labels(tmp_path: pathlib.Path) -> None:
	"""Ambiguous renderer labels fail before the controller can stop either one."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.commands.service_stop_plan(
			target(tmp_path),
			snapshot(renderer(running=True), renderer(running=True)),
			"webwork-renderer",
		)


#============================================
@pytest.mark.parametrize("tail", ("-1", "recent", "1.5"))
def test_log_tail_rejects_ambiguous_values(tail: str) -> None:
	"""The log controller rejects values that could alter the scoped command."""
	with pytest.raises(argparse.ArgumentTypeError):
		local_stack_control.cli.log_tail(tail)
