"""Rebuild must not replace application services while PostgreSQL is down."""

import pathlib

import pytest

import local_stack_control.disposable_stack_adapter
import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_lifecycle_helpers


def restart_status(
	service: str, *, healthy: bool = True, instances: int = 1
) -> local_stack_control.models.StackServiceStatus:
	"""One semantic restart-baseline observation without an engine fixture."""
	return local_stack_control.models.StackServiceStatus(
		service=service,
		instances=instances,
		present=instances > 0,
		running=healthy,
		healthy=healthy,
		complete=healthy if service in local_stack_control.models.BASE_ONE_SHOT_SERVICES else False,
		state="running" if healthy else ("ambiguous" if instances > 1 else "exited"),
		health="healthy" if healthy else None,
		exit_code=None if healthy else 137,
	)


def restart_report(
	*statuses: local_stack_control.models.StackServiceStatus,
) -> local_stack_control.models.StatusReport:
	"""Status report for deterministic recovery-policy tests."""
	return local_stack_control.models.StatusReport(
		project="containers",
		with_smtp=False,
		snapshot=local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		services=statuses,
		ok=False,
		state="failed",
		message="renderer recovery is required",
	)


lifecycle_target = local_stack_lifecycle_helpers.lifecycle_target


#============================================
def test_rebuild_application_refuses_an_unhealthy_database(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""PostgreSQL must stay up; the rebuild must not take down stateful services."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	unhealthy = restart_report(
		restart_status("postgres", healthy=False),
		restart_status("minio"),
		restart_status("webwork-renderer"),
		restart_status("api"),
		restart_status("worker"),
		restart_status("public-asset-publisher"),
		restart_status("gateway"),
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "status_report", lambda *args: unhealthy)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.require_application_rebuild_baseline(
			target, local_stack_lifecycle_helpers.UnexpectedRunner()
		)
