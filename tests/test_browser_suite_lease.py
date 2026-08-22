"""Offline contracts for the browser suite's fixed private workspace."""

import os
import pathlib
import stat

import pytest

import local_stack_control.browser_suite_lease


#============================================
def test_lease_is_nonblocking_noninheritable_and_reusable(tmp_path: pathlib.Path) -> None:
	"""A second suite stops immediately and release permits the next suite."""
	first = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path)
	with pytest.raises(local_stack_control.browser_suite_lease.BrowserSuiteError, match="already running"):
		local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path)
	assert not os.get_inheritable(first._lock_descriptor)
	assert not os.get_inheritable(first._repository_descriptor)
	first.release()
	second = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path)
	second.release()


#============================================
def test_workspace_reset_removes_prior_private_state_and_reuses_one_path(tmp_path: pathlib.Path) -> None:
	"""The next holder gets a fresh fixed workspace, not a recovered run record."""
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		workspace = lease.reset_workspace()
		(workspace / "stale").write_text("old", encoding="ascii")
		(workspace / "nested").mkdir()
		(workspace / "nested" / "private").write_text("old", encoding="ascii")
		assert lease.reset_workspace() == workspace and list(workspace.iterdir()) == []
		assert stat.S_IMODE(workspace.stat().st_mode) == 0o700


#============================================
def test_workspace_reset_refuses_a_linked_workspace(tmp_path: pathlib.Path) -> None:
	"""The lease never clears a substituted target outside its verified root."""
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		outside = tmp_path / "outside"
		outside.mkdir()
		lease.workspace.symlink_to(outside, target_is_directory=True)
		with pytest.raises(local_stack_control.browser_suite_lease.BrowserSuiteError, match="workspace"):
			lease.reset_workspace()
		assert outside.is_dir()


#============================================
def test_released_lease_cannot_reset_the_workspace(tmp_path: pathlib.Path) -> None:
	"""A stale lease object has no post-release local-state authority."""
	lease = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path)
	lease.release()
	with pytest.raises(local_stack_control.browser_suite_lease.BrowserSuiteError, match="no longer held"):
		lease.reset_workspace()
