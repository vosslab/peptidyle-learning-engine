"""Ownership-order tests for the canonical fixed browser-suite lifecycle."""

import dataclasses
import json
import pathlib
import sys
from collections.abc import Callable

import pytest

import file_utils
import local_stack_control.browser_suite_lease
import local_stack_control.browser_suite_reset
import local_stack_control.live_demo_target
import local_stack_control.models
import local_stack_control.process


E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_suite_lifecycle
import e2e_browser_fault_orchestrator
import e2e_browser_screenshot_publisher
import e2e_browser_suite_owner
import e2e_browser_suite_oracles


#============================================
def test_owned_lifecycle_acquires_before_factory_and_resets_on_factory_failure(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A failed post-lease dependency setup leaves the fixed fixture reusable."""
	resets: list[str] = []
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: resets.append("reset"),
	)

	def factory() -> e2e_browser_suite_owner.BrowserSuiteDependencies:
		raise e2e_browser_suite_owner.BrowserSuiteError("injected dependency failure")

	with pytest.raises(e2e_browser_suite_owner.BrowserSuiteError, match="injected dependency"):
		e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
			factory,
		)
	assert resets == ["reset", "reset"]
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path) as lease:
		assert list(lease.reset_workspace().iterdir()) == []


#============================================
def test_contending_owner_stops_before_dependency_factory(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A second invocation cannot allocate ports or inspect providers while busy."""
	called = False
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)

	def factory() -> e2e_browser_suite_owner.BrowserSuiteDependencies:
		nonlocal called
		called = True
		raise AssertionError("the dependency factory must not run while locked")

	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path):
		with pytest.raises(local_stack_control.browser_suite_lease.BrowserSuiteError, match="already running"):
			e2e_browser_suite_lifecycle.run_owned_selection(
				e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
				factory,
			)
	assert not called


#============================================
def test_reset_rejects_a_released_lease_before_engine_inventory(
	tmp_path: pathlib.Path,
) -> None:
	"""Exact deletion authority cannot outlive the browser-suite lease."""
	lease = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path)
	lease.release()
	with pytest.raises(local_stack_control.browser_suite_lease.BrowserSuiteError, match="no longer held"):
		local_stack_control.browser_suite_reset.reset_live_demo_browser(
			lease,
			local_stack_control.process.SubprocessRunner(),
			tmp_path,
		)


#============================================
def test_fixed_target_carries_the_exact_project_and_owner_label(tmp_path: pathlib.Path) -> None:
	"""Compose receives the fixed reset authority with every browser launch."""
	selections = {
		"PLE_WEBWORK_RENDERER_IMAGE": "renderer",
		"PLE_WEBWORK_RENDERER_BASE_URL": "http://renderer",
		"PLE_WEBWORK_RENDERER_ID": "renderer-id",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": "10",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES": "1000",
		"PLE_GATEWAY_IMAGE_SHA256": "gateway",
		"PLE_POSTGRES_IMAGE_SHA256": "postgres",
		"PLE_MINIO_IMAGE_SHA256": "minio",
		"PLE_MINIO_MC_IMAGE_SHA256": "minio-mc",
		"PLE_SECRET_INIT_IMAGE_SHA256": "secret-init",
	}
	target = local_stack_control.live_demo_target.write_private_target(
		tmp_path,
		local_stack_control.models.LiveDemoProfile.BROWSER,
		local_stack_control.live_demo_target.LiveDemoPorts(53501, 54001, 54501, 55001),
		selections,
	)
	assert target.project == local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	assert "PLE_E2E_OWNER=live-demo-browser\n" in (tmp_path / "env.local").read_text(encoding="ascii")


#============================================
def _receipt() -> e2e_browser_suite_owner.BrowserSuiteReceipt:
	"""Build a minimal inner receipt with no residual resource or process evidence."""
	provider = e2e_browser_suite_oracles.ProviderReceipt("podman-compose", (), False)
	inventory = e2e_browser_suite_oracles.SuiteInventory(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), (), (), (), provider
	)
	origin = e2e_browser_suite_oracles.OriginReceipt("https://localhost:55001/", (), ())
	return e2e_browser_suite_owner.BrowserSuiteReceipt(
		"sysadmin_first_claim", "https://localhost:55001/",
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, "private", True, True,
		True, True, True, origin, inventory, inventory, inventory,
	)


#============================================
def test_public_receipt_has_no_pending_screenshot_publication_boundary() -> None:
	"""The public receipt cannot retain or serialize private image bytes."""
	receipt = _receipt()
	assert "pending_screenshot_publication" not in {
		field.name for field in dataclasses.fields(receipt)
	}
	assert "pending_screenshot_publication" not in repr(receipt)
	assert "pending_screenshot_publication" not in receipt.as_json()


#============================================
def test_screenshot_publication_follows_all_final_fixture_observations(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Private capture bytes publish only after reset, process, and fault proof."""
	events: list[str] = []
	reported: list[e2e_browser_suite_owner.BrowserSuiteReceipt] = []
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: (
			events.append("reset"),
			local_stack_control.models.ProjectSnapshot(
				local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
			),
		)[1],
	)
	monkeypatch.setattr(
		e2e_browser_fault_orchestrator,
		"reset_stale_protocol_directory",
		lambda: events.append("initial-fault") is None,
	)
	monkeypatch.setattr(
		e2e_browser_fault_orchestrator,
		"require_protocol_directory_absent",
		lambda: events.append("final-fault") is None,
	)

	def run_inner(
		_selection: object,
		dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies,
	) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
		events.append("run")
		assert dependencies._screenshot_collector is not None
		dependencies._screenshot_collector(object())
		inner = _receipt()
		dependencies.receipt_reporter(inner)
		return inner

	def publish(_root: pathlib.Path, pending: object) -> object:
		assert pending is not None
		events.append("publish")
		return object()

	monkeypatch.setattr(e2e_browser_suite_owner, "run_selection", run_inner)
	monkeypatch.setattr(e2e_browser_screenshot_publisher, "publish", publish)

	def read_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		assert sessions == ()
		events.append("processes")
		return ()

	receipt = e2e_browser_suite_lifecycle.run_owned_selection(
		e2e_browser_suite_owner.BrowserSuiteSelection(None, None, True, None, True),
		lambda: _dependencies(tmp_path, reported.append),
		owner_process_reader=read_processes,
	)
	assert receipt.screenshot_evidence is not None
	assert reported == [receipt]
	assert events == [
		"initial-fault", "reset", "run", "reset", "processes", "final-fault", "publish"
	]


#============================================
def test_screenshot_publication_failure_never_reports_a_partial_receipt(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A failed final publication leaves no public capture completion claim."""
	reported: list[e2e_browser_suite_owner.BrowserSuiteReceipt] = []
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		),
	)
	monkeypatch.setattr(
		e2e_browser_fault_orchestrator, "reset_stale_protocol_directory", lambda: True
	)
	monkeypatch.setattr(
		e2e_browser_fault_orchestrator, "require_protocol_directory_absent", lambda: True
	)

	def run_inner(
		_selection: object,
		dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies,
	) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
		assert dependencies._screenshot_collector is not None
		dependencies._screenshot_collector(object())
		inner = _receipt()
		dependencies.receipt_reporter(inner)
		return inner

	monkeypatch.setattr(e2e_browser_suite_owner, "run_selection", run_inner)
	monkeypatch.setattr(
		e2e_browser_screenshot_publisher,
		"publish",
		lambda _root, _pending: (_ for _ in ()).throw(
			e2e_browser_screenshot_publisher.ScreenshotPublicationError("injected publication failure")
		),
	)
	with pytest.raises(e2e_browser_screenshot_publisher.ScreenshotPublicationError):
		e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection(None, None, True, None, True),
			lambda: _dependencies(tmp_path, reported.append),
		)
	assert reported == []


#============================================
def test_public_receipt_retains_named_context_origin_evidence() -> None:
	"""The final receipt keeps each real browser context's safe gateway proof."""
	contexts = (
		e2e_browser_suite_oracles.ContextOriginReceipt(
			"remote", ("https://localhost:55001",), ("https://localhost:55001",)
		),
		e2e_browser_suite_oracles.ContextOriginReceipt(
			"local", ("https://localhost:55001",), ("https://localhost:55001",)
		),
	)
	base = _receipt()
	origin = e2e_browser_suite_oracles.OriginReceipt(
		"https://localhost:55001", ("https://localhost:55001",),
		("https://localhost:55001",), contexts
	)
	scenario = e2e_browser_suite_owner.ScenarioRunReceipt(
		"grade_settings_conflict", "bs1-0123456789ab-grade_settings_conflict",
		"https://localhost:55001", ("https://localhost:55001",),
		("https://localhost:55001",), True, contexts,
	)
	receipt = base.__class__(
		base.scenario, base.origin, base.project, base.private_state_directory,
		base.lifecycle_launch_attempted, base.lifecycle_launch_completed,
		base.cleanup_attempted, base.cleanup_completed, base.private_state_removed,
		origin, base.before_inventory, base.launched_inventory, base.after_inventory,
		(scenario,), base.final_fixture_evidence, base.owner_process_sessions,
	)
	value = json.loads(receipt.as_json())
	assert [item["name"] for item in value["originReceipt"]["observedContexts"]] == [
		"local", "remote"
	]
	assert [item["name"] for item in value["scenarioReceipts"][0]["observedContexts"]] == [
		"local", "remote"
	]
#============================================
def _dependencies(tmp_path: pathlib.Path, reporter: Callable[[e2e_browser_suite_owner.BrowserSuiteReceipt], None]) -> e2e_browser_suite_owner.BrowserSuiteDependencies:
	"""Build the minimum typed dependency surface exercised by the lease wrapper."""
	runner = local_stack_control.process.SubprocessRunner()
	return e2e_browser_suite_owner.BrowserSuiteDependencies(
		tmp_path, runner, {}, (1, 2, 3, 4), lambda *_args: None,
		lambda *_args: None, lambda *_args: None, lambda *_args: None,
		lambda *_args: None, lambda *_args: None, lambda *_args: None,
		lambda *_args: None, lambda *_args: None, lambda *_args: None, reporter,
		lambda *_args: None, lambda *_args: "",
	)


#============================================
@pytest.mark.parametrize("failure", (None, "launch failure", "child failure", "internal cleanup failure"))
def test_owned_receipt_follows_final_reset_for_success_and_inner_failures(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	failure: str | None,
) -> None:
	"""The final fixed-fixture receipt follows reset for every inner outcome."""
	events: list[str] = []
	reported: list[e2e_browser_suite_owner.BrowserSuiteReceipt] = []
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: (events.append("reset"), local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		))[1],
	)

	def factory() -> e2e_browser_suite_owner.BrowserSuiteDependencies:
		events.append("factory")
		return _dependencies(tmp_path, lambda receipt: (events.append("report"), reported.append(receipt)))

	def run_inner(_selection: object, dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
		events.append("run")
		inner = _receipt()
		dependencies.receipt_reporter(inner)
		if failure is not None:
			raise e2e_browser_suite_owner.BrowserSuiteError(failure)
		return inner

	monkeypatch.setattr(e2e_browser_suite_owner, "run_selection", run_inner)
	if failure is None:
		receipt = e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False), factory
		)
		assert receipt.final_fixture_evidence is not None
		assert all(receipt.final_fixture_evidence.as_value().values())
	else:
		with pytest.raises(e2e_browser_suite_owner.BrowserSuiteError, match=failure):
			e2e_browser_suite_lifecycle.run_owned_selection(
				e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False), factory
			)
		assert reported[-1].final_fixture_evidence is not None
	assert events == ["reset", "factory", "run", "reset", "report"]
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path):
		pass


#============================================
def test_complete_lifecycle_prints_pass_only_after_public_final_evidence(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""PASS follows the final reset's public receipt, rather than a child result."""
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		),
	)
	def run_inner(
		_selection: object,
		dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies,
	) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
		inner = _receipt()
		dependencies.receipt_reporter(inner)
		return inner

	monkeypatch.setattr(e2e_browser_suite_owner, "run_selection", run_inner)
	e2e_browser_suite_lifecycle.run_owned_selection(
		e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
		lambda: _dependencies(tmp_path, lambda _receipt: print("public final receipt")),
	)
	assert capsys.readouterr().out.splitlines() == ["public final receipt", "Browser-suite: PASS"]


#============================================
def test_child_failure_never_prints_pass(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""A final cleanup receipt cannot turn a failed child into a reported success."""
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		),
	)
	monkeypatch.setattr(
		e2e_browser_suite_owner,
		"run_selection",
		lambda _selection, dependencies: (
			dependencies.receipt_reporter(_receipt()),
			(_ for _ in ()).throw(e2e_browser_suite_owner.BrowserSuiteError("child failure")),
		)[1],
	)
	with pytest.raises(e2e_browser_suite_owner.BrowserSuiteError, match="child failure"):
		e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
			lambda: _dependencies(tmp_path, lambda _receipt: print("public final receipt")),
		)
	assert capsys.readouterr().out.splitlines() == ["public final receipt"]


#============================================
def test_final_reset_failure_preserves_the_inner_failure_and_releases_lock(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A reset failure joins the original operation error rather than replacing it."""
	resets = 0
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)

	def reset(lease: object, runner: object, root: pathlib.Path) -> local_stack_control.models.ProjectSnapshot:
		nonlocal resets
		resets += 1
		if resets == 2:
			raise e2e_browser_suite_owner.BrowserSuiteError("final reset failure")
		return local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		)

	monkeypatch.setattr(local_stack_control.browser_suite_reset, "reset_live_demo_browser", reset)
	monkeypatch.setattr(
		e2e_browser_suite_owner,
		"run_selection",
		lambda _selection, dependencies: (
			dependencies.receipt_reporter(_receipt()),
			(_ for _ in ()).throw(e2e_browser_suite_owner.BrowserSuiteError("child failure")),
		)[1],
	)
	with pytest.raises(BaseExceptionGroup) as raised:
		e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
			lambda: _dependencies(tmp_path, lambda _receipt: None),
		)
	messages = str(raised.value)
	assert "browser suite lifecycle failures" in messages
	assert any("child failure" in str(item) for item in raised.value.exceptions)
	assert any("final reset failure" in str(item) for item in raised.value.exceptions)
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path):
		pass


#============================================
def test_final_report_failure_preserves_the_inner_failure_and_releases_lock(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The post-reset public reporter cannot replace the original child failure."""
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		),
	)
	monkeypatch.setattr(
		e2e_browser_suite_owner,
		"run_selection",
		lambda _selection, dependencies: (
			dependencies.receipt_reporter(_receipt()),
			(_ for _ in ()).throw(e2e_browser_suite_owner.BrowserSuiteError("child failure")),
		)[1],
	)
	with pytest.raises(BaseExceptionGroup) as raised:
		e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
			lambda: _dependencies(
				tmp_path,
				lambda _receipt: (_ for _ in ()).throw(
					e2e_browser_suite_owner.BrowserSuiteError("final report failure")
				),
			),
		)
	assert any("child failure" in str(item) for item in raised.value.exceptions)
	assert any("final report failure" in str(item) for item in raised.value.exceptions)
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path):
		pass


#============================================
@pytest.mark.parametrize("shape", ("zero", "multiple", "mismatch"))
def test_successful_inner_run_requires_one_matching_captured_receipt(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	shape: str,
) -> None:
	"""A returned inner success cannot bypass its single captured receipt."""
	resets: list[str] = []
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: (resets.append("reset"), local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		))[1],
	)

	def run_inner(_selection: object, dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
		first = _receipt()
		if shape == "multiple":
			dependencies.receipt_reporter(first)
			dependencies.receipt_reporter(_receipt())
		elif shape == "mismatch":
			dependencies.receipt_reporter(first)
			return _receipt()
		return first

	monkeypatch.setattr(e2e_browser_suite_owner, "run_selection", run_inner)
	with pytest.raises(e2e_browser_suite_owner.BrowserSuiteError, match="inner receipt is incomplete"):
		e2e_browser_suite_lifecycle.run_owned_selection(
			e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
			lambda: _dependencies(tmp_path, lambda _receipt: None),
		)
	assert resets == ["reset", "reset"]
	with local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire(tmp_path):
		pass


#============================================
def test_final_process_observation_follows_reset_and_never_serializes_sessions(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The final process projection is freshly read after reset before reporting."""
	events: list[str] = []
	reported: list[e2e_browser_suite_owner.BrowserSuiteReceipt] = []
	monkeypatch.setattr(e2e_browser_suite_owner, "repo_root", lambda: tmp_path)
	monkeypatch.setattr(
		local_stack_control.browser_suite_reset,
		"reset_live_demo_browser",
		lambda lease, runner, root: (events.append("reset"), local_stack_control.models.ProjectSnapshot(
			local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), ()
		))[1],
	)
	def run_inner(_selection: object, dependencies: e2e_browser_suite_owner.BrowserSuiteDependencies) -> e2e_browser_suite_owner.BrowserSuiteReceipt:
		inner = _receipt()
		dependencies.receipt_reporter(inner)
		return inner

	monkeypatch.setattr(e2e_browser_suite_owner, "run_selection", run_inner)

	def read_processes(
		sessions: tuple[local_stack_control.process.ProcessSession, ...],
	) -> tuple[e2e_browser_suite_oracles.ProcessIdentity, ...]:
		events.append("processes")
		assert sessions == ()
		return ()

	receipt = e2e_browser_suite_lifecycle.run_owned_selection(
		e2e_browser_suite_owner.BrowserSuiteSelection("sysadmin_first_claim", None, False),
		lambda: _dependencies(tmp_path, lambda item: (events.append("report"), reported.append(item))),
		owner_process_reader=read_processes,
	)
	assert events == ["reset", "reset", "processes", "report"]
	assert receipt.final_fixture_evidence is not None
	assert "ownerProcessSessions" not in receipt.as_json()
