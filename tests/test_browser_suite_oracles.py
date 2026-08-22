"""Offline behavior checks for browser-suite origin and lifecycle evidence."""

import json
import errno
import pathlib
import sys

import pytest

import file_utils
import local_stack_control.process

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_suite_oracles as browser_suite_oracles


#============================================
def inventory(**changes: object) -> browser_suite_oracles.SuiteInventory:
	"""Build a minimal no-resource live-demo provider inventory."""
	value: dict[str, object] = {
		"project": "ple-live-demo-browser-0123456789ab",
		"containers": (),
		"volumes": (),
		"networks": (),
		"private_artifacts": (),
		"owner_processes": (),
		"provider": browser_suite_oracles.ProviderReceipt(
			"podman-compose", ("podman-compose", "--in-pod", "false"), False
		),
	}
	value.update(changes)
	return browser_suite_oracles.SuiteInventory(**value)  # type: ignore[arg-type]


#============================================
def test_origin_receipt_accepts_only_the_expected_https_gateway(tmp_path: pathlib.Path) -> None:
	"""Chromium evidence accepts the exact production HTTPS gateway for pages and requests."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps({"pageOrigins": ["https://localhost:55001"], "requestOrigins": ["https://localhost:55001"]}),
		encoding="ascii",
	)
	receipt = browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")
	assert receipt.expected_origin == "https://localhost:55001"
	assert receipt.observed_page_origins == ("https://localhost:55001",)


#============================================
def test_origin_receipt_refuses_a_mixed_browser_origin(tmp_path: pathlib.Path) -> None:
	"""A page or request outside the gateway fails the visible-browser receipt."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps({"pageOrigins": ["https://localhost:55001"], "requestOrigins": ["https://example.test"]}),
		encoding="ascii",
	)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="outside"):
		browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")


#============================================
def test_origin_receipt_validates_each_named_browser_context(tmp_path: pathlib.Path) -> None:
	"""A two-session scenario proves that both contexts used the HTTPS gateway."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps(
			{
				"pageOrigins": ["https://localhost:55001"],
				"requestOrigins": ["https://localhost:55001"],
				"contexts": {
					"local": {
						"pageOrigins": ["https://localhost:55001"],
						"requestOrigins": ["https://localhost:55001"],
					},
					"remote": {
						"pageOrigins": ["https://localhost:55001"],
						"requestOrigins": ["https://localhost:55001"],
					},
				},
			}
		),
		encoding="ascii",
	)
	receipt = browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")
	assert tuple(item.name for item in receipt.observed_contexts) == ("local", "remote")


#============================================
def test_origin_receipt_refuses_a_named_context_outside_the_gateway(tmp_path: pathlib.Path) -> None:
	"""A canonical union cannot conceal an origin escape in one browser context."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps(
			{
				"pageOrigins": ["https://localhost:55001"],
				"requestOrigins": ["https://localhost:55001"],
				"contexts": {
					"local": {
						"pageOrigins": ["https://localhost:55001"],
						"requestOrigins": ["https://example.test"],
					},
				},
			}
		),
		encoding="ascii",
	)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="outside"):
		browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")


#============================================
def test_origin_receipt_refuses_malformed_named_context_evidence(tmp_path: pathlib.Path) -> None:
	"""Named browser-context evidence stays a closed public receipt shape."""
	path = tmp_path / "origin.json"
	path.write_text(
		json.dumps(
			{
				"pageOrigins": ["https://localhost:55001"],
				"requestOrigins": ["https://localhost:55001"],
				"contexts": {"local": {"pageOrigins": ["https://localhost:55001"]}},
			}
		),
		encoding="ascii",
	)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="shape"):
		browser_suite_oracles.origin_receipt_from_file(path, "https://localhost:55001/")


#============================================
def test_private_artifact_inventory_keeps_metadata_and_never_content(tmp_path: pathlib.Path) -> None:
	"""Receipt inventory exposes a leftover private path without serializing its secret bytes."""
	path = tmp_path / "secret"
	path.write_text("sensitive-value", encoding="ascii")
	path.chmod(0o600)
	items = browser_suite_oracles.private_artifacts(tmp_path)
	public = browser_suite_oracles.public_inventory(inventory(private_artifacts=items))
	assert items[0].path == "secret" and items[0].mode == 0o600
	assert "sensitive-value" not in json.dumps(public, sort_keys=True)


#============================================
def test_private_artifact_inventory_refuses_a_symlink(tmp_path: pathlib.Path) -> None:
	"""A surviving symlink cannot make a private cleanup receipt appear empty."""
	target = tmp_path / "target"
	target.write_text("content", encoding="ascii")
	link = tmp_path / "link"
	link.symlink_to(target)
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="unexpected"):
		browser_suite_oracles.private_artifacts(tmp_path)


#============================================
def test_process_group_inventory_retains_a_reparented_owner_child() -> None:
	"""A typed PPID-one row remains owned because its recorded process group persists."""
	processes = browser_suite_oracles.processes_from_rows([(42, 1, 7102)], {7102}, 99, set())
	assert processes == (browser_suite_oracles.ProcessIdentity(42, 1, 7102),)


#============================================
def test_process_marker_helper_detects_multiline_and_numeric_looking_text() -> None:
	"""Opaque command text cannot disrupt an exact random-marker presence check."""
	marker = "PLE_BROWSER_SUITE_OWNER_SESSION=ple-owner-marker"
	stdout = "ordinary command\n 42 1 8200 argument\n" + marker + "\nmore text\n"
	assert browser_suite_oracles._marker_descendant_present(stdout, {marker})
	assert not browser_suite_oracles._marker_descendant_present(stdout, {"other-marker"})


#============================================
def test_process_identity_rows_accept_command_free_numeric_snapshot() -> None:
	"""Group leakage uses only the numeric projection, never command or environment text."""
	assert browser_suite_oracles._process_identity_rows(" 42 1 7102\n 43 1 8200\n") == [
		(42, 1, 7102),
		(43, 1, 8200),
	]


#============================================
def test_marker_descendant_is_a_typed_nonempty_outcome(monkeypatch: pytest.MonkeyPatch) -> None:
	"""One raw marker match fails nonempty without attempting to parse its command text."""
	marker = "PLE_BROWSER_SUITE_OWNER_SESSION=ple-owner-marker"

	class Probe:
		"""Return one deterministic process projection without launching a subprocess."""

		def __init__(self, stdout: str, returncode: int = 0) -> None:
			self.stdout = stdout
			self.returncode = returncode

		def communicate(self) -> tuple[str, str]:
			return (self.stdout, "")

	probes = iter((Probe(" 42 1 7102\n"), Probe("42 1 7102\n" + marker + "\n")))
	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", lambda *args, **kwargs: next(probes))
	sessions = (local_stack_control.process.ProcessSession(7102, 1, "injected", marker),)
	with pytest.raises(browser_suite_oracles.OwnerMarkerDescendantError, match="marker descendant"):
		browser_suite_oracles.owner_processes(sessions)


#============================================
@pytest.mark.parametrize(
	"stdout",
	(
		" 42 1\n",
		" 42 not-a-parent 42\n",
	),
)
def test_process_identity_rows_reject_malformed_numeric_snapshot(stdout: str) -> None:
	"""Command-free ownership data rejects malformed records rather than appearing empty."""
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match="invalid"):
		browser_suite_oracles._process_identity_rows(stdout)


#============================================
def test_marker_probe_failure_remains_a_read_failure(monkeypatch: pytest.MonkeyPatch) -> None:
	"""The raw marker probe cannot be mistaken for verified empty ownership evidence."""
	class Probe:
		"""Return one deterministic process projection without launching a subprocess."""

		def __init__(self, stdout: str, returncode: int) -> None:
			self.stdout = stdout
			self.returncode = returncode

		def communicate(self) -> tuple[str, str]:
			return (self.stdout, "")

	probes = iter((Probe(" 42 1 7102\n", 0), Probe("", 1)))
	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", lambda *args, **kwargs: next(probes))
	sessions = (local_stack_control.process.ProcessSession(7102, 1, "injected", "marker"),)
	with pytest.raises(browser_suite_oracles.OwnerProcessMarkerProbeError, match="marker probe failed"):
		browser_suite_oracles.owner_processes(sessions)


#============================================
def test_identity_probe_spawn_and_exit_failures_are_typed(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Identity spawn and nonzero completion keep distinct fixed safe failure types."""
	sessions = (local_stack_control.process.ProcessSession(7102, 1, "injected", "marker"),)

	def fail_spawn(*args: object, **kwargs: object) -> object:
		raise OSError(errno.EINVAL, "private ps spawn failure")

	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", fail_spawn)
	with pytest.raises(browser_suite_oracles.OwnerProcessIdentitySpawnOtherError, match="could not start"):
		browser_suite_oracles.owner_processes(sessions)

	class Probe:
		"""Return one command-free probe completion without launching a subprocess."""

		returncode = 1

		def communicate(self) -> tuple[str, str]:
			return ("", "")

	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", lambda *args, **kwargs: Probe())
	with pytest.raises(browser_suite_oracles.OwnerProcessIdentityExitError, match="exited unsuccessfully"):
		browser_suite_oracles.owner_processes(sessions)


#============================================
def test_identity_probe_retries_resource_exhaustion_with_a_fresh_process(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A transient EAGAIN receives one bounded retry rather than weakening final proof."""
	class Probe:
		"""Represent one successful fresh identity probe process."""

		returncode = 0
		pid = 7

		def communicate(self) -> tuple[str, str]:
			return ("", "")

	attempts: list[int] = []

	def spawn(*args: object, **kwargs: object) -> Probe:
		attempts.append(1)
		if len(attempts) == 1:
			raise OSError(errno.EAGAIN, "private resource exhaustion")
		return Probe()

	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", spawn)
	monkeypatch.setattr(browser_suite_oracles.time, "sleep", lambda _seconds: None)
	result = browser_suite_oracles._spawn_identity_probe()
	assert isinstance(result, Probe) and len(attempts) == 2


#============================================
@pytest.mark.parametrize(
	("error_number", "error_type"),
	(
		(errno.ENOMEM, browser_suite_oracles.OwnerProcessIdentitySpawnExhaustedError),
		(errno.EACCES, browser_suite_oracles.OwnerProcessIdentitySpawnPermissionError),
		(errno.EPERM, browser_suite_oracles.OwnerProcessIdentitySpawnPermissionError),
		(errno.ENOENT, browser_suite_oracles.OwnerProcessIdentitySpawnUnavailableError),
		(errno.EINVAL, browser_suite_oracles.OwnerProcessIdentitySpawnOtherError),
	),
)
def test_identity_spawn_errno_categories_are_fixed(
	error_number: int,
	error_type: type[browser_suite_oracles.OwnerProcessIdentitySpawnError],
) -> None:
	"""OS text stays internal while the public spawn category remains fixed."""
	error = browser_suite_oracles._identity_spawn_error(
		OSError(error_number, "private operating system message")
	)
	assert isinstance(error, error_type)


#============================================
def test_identity_resource_exhaustion_stops_at_the_monotonic_deadline(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Persistent EAGAIN remains a failure after the bounded retry window ends."""
	def spawn(*args: object, **kwargs: object) -> object:
		raise OSError(errno.EAGAIN, "private resource exhaustion")

	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", spawn)
	monkeypatch.setattr(browser_suite_oracles, "IDENTITY_PROBE_SPAWN_RETRY_SECONDS", 0.0)
	with pytest.raises(browser_suite_oracles.OwnerProcessIdentitySpawnExhaustedError):
		browser_suite_oracles._spawn_identity_probe()


#============================================
@pytest.mark.parametrize(
	("error_number", "error_type"),
	(
		(errno.EACCES, browser_suite_oracles.OwnerProcessIdentitySpawnPermissionError),
		(errno.ENOENT, browser_suite_oracles.OwnerProcessIdentitySpawnUnavailableError),
		(errno.EINVAL, browser_suite_oracles.OwnerProcessIdentitySpawnOtherError),
	),
)
def test_identity_nonretryable_spawn_errors_do_not_retry(
	monkeypatch: pytest.MonkeyPatch,
	error_number: int,
	error_type: type[browser_suite_oracles.OwnerProcessIdentitySpawnError],
) -> None:
	"""Permission, missing executable, and other OS failures remain one-attempt errors."""
	attempts: list[int] = []

	def spawn(*args: object, **kwargs: object) -> object:
		attempts.append(1)
		raise OSError(error_number, "private operating system message")

	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", spawn)
	with pytest.raises(error_type):
		browser_suite_oracles._spawn_identity_probe()
	assert attempts == [1]


#============================================
def test_identity_output_decode_and_marker_read_failures_are_typed(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Output, decode, and marker read failures retain distinct probe categories."""
	sessions = (local_stack_control.process.ProcessSession(7102, 1, "injected", "marker"),)

	class Probe:
		"""Return deterministic output or one read failure without launching a subprocess."""

		def __init__(self, stdout: str, error: UnicodeDecodeError | None = None) -> None:
			self.stdout = stdout
			self.error = error
			self.returncode = 0

		def communicate(self) -> tuple[str, str]:
			if self.error is not None:
				raise self.error
			return (self.stdout, "")

	monkeypatch.setattr(
		browser_suite_oracles.subprocess,
		"Popen",
		lambda *args, **kwargs: Probe("", UnicodeDecodeError("utf-8", b"\xff", 0, 1, "invalid byte")),
	)
	with pytest.raises(browser_suite_oracles.OwnerProcessIdentityOutputError, match="could not return output"):
		browser_suite_oracles.owner_processes(sessions)

	monkeypatch.setattr(
		browser_suite_oracles.subprocess,
		"Popen",
		lambda *args, **kwargs: Probe(" 42 invalid 7102\n"),
	)
	with pytest.raises(browser_suite_oracles.OwnerProcessIdentityDecodeError, match="identity data is invalid"):
		browser_suite_oracles.owner_processes(sessions)

	probes = iter(
		(
			Probe(" 42 1 7102\n"),
			Probe("", UnicodeDecodeError("utf-8", b"\xff", 0, 1, "invalid byte")),
		)
	)
	monkeypatch.setattr(browser_suite_oracles.subprocess, "Popen", lambda *args, **kwargs: next(probes))
	with pytest.raises(browser_suite_oracles.OwnerProcessMarkerProbeError, match="marker probe failed"):
		browser_suite_oracles.owner_processes(sessions)


#============================================
@pytest.mark.parametrize(
	("change", "message"),
	[
		({"private_artifacts": (browser_suite_oracles.PrivateArtifact("leftover", 0o600, 1),)}, "private artifacts"),
		({"owner_processes": (browser_suite_oracles.ProcessIdentity(9, 1, 9),)}, "background processes"),
		({"provider": browser_suite_oracles.ProviderReceipt("podman-compose", ("podman-compose", "--in-pod", "true"), True)}, "pod ownership disabled"),
	],
)
def test_cleanup_oracle_refuses_remaining_owned_state(change: dict[str, object], message: str) -> None:
	"""The post-cleanup gate fails on any remaining owned resource class or pod policy change."""
	with pytest.raises(browser_suite_oracles.BrowserSuiteOracleError, match=message):
		browser_suite_oracles.empty_after_cleanup(inventory(**change))
