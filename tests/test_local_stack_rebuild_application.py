"""Rebuild must not replace application services while PostgreSQL is down."""

import pathlib

import pytest

import local_stack_control.disposable_stack_adapter
import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_lifecycle_helpers
from tests.test_local_stack_lifecycle_restart import restart_report, restart_status


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
