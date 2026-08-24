"""Offline contracts for the live-demo Base Course host boundary."""

import json
import pathlib
import stat

import pytest

import local_stack_control.base_course_lifecycle
import local_stack_control.commands
import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.live_demo_claim_context
import local_stack_control.lifecycle
import local_stack_control.lifecycle_profiles
import local_stack_control.models
import local_stack_control.process


class BaseCourseRunner(local_stack_control.process.CommandRunner):
	"""Capture Base Course child commands and return fixed outputs."""

	def __init__(
		self,
		*outputs: str,
		refuse_storage: bool = False,
		storage_failure: tuple[str, str] | None = None,
	) -> None:
		"""Store deterministic outputs and an optional storage refusal."""
		self.outputs = list(outputs)
		self.refuse_storage = refuse_storage
		self.storage_failure = storage_failure
		self.calls: list[tuple[list[str], dict[str, str], pathlib.Path, str | None]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Return one fixed result and retain the exact child input."""
		assert environment is not None
		assert cwd is not None
		self.calls.append((argv, environment, cwd, stdin))
		if self.storage_failure is not None and "createbuckets" in argv:
			stdout, stderr = self.storage_failure
			return local_stack_control.models.CommandResult(tuple(argv), 17, stdout, stderr)
		if self.refuse_storage and "createbuckets" in argv:
			return local_stack_control.models.CommandResult(tuple(argv), 17, "refused", "")
		if not self.outputs:
			raise AssertionError(f"unexpected Base Course command: {argv}")
		result = local_stack_control.models.CommandResult(
			tuple(argv), 0, self.outputs.pop(0), ""
		)
		return result

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Keep this unit boundary entirely offline."""
		raise AssertionError(f"unexpected Base Course stream: {argv}")


#============================================
def base_course_receipt(
	action: str,
	install_state: str,
	generation: str = "00000000-0000-0000-0000-000000000006",
) -> str:
	"""Build one exact v1 response with its canonical storage receipt."""
	storage = json.dumps(
		{
			"schemaVersion": 1,
			"baselineVersion": "base-course-v1",
			"installationGeneration": generation,
			"storageReceiptBucket": "private-content",
			"storageReceiptKey": "ple/live-demo/base-course-install-receipt.json",
			"objectManifest": [],
		},
		separators=(",", ":"),
	)
	value: dict[str, object] = {
		"schemaVersion": 1,
		"action": action,
		"installState": install_state,
		"baselineVersion": "base-course-v1",
		"objectManifest": [],
		"installationGeneration": generation,
		"storageReceiptBucket": "private-content",
		"storageReceiptKey": "ple/live-demo/base-course-install-receipt.json",
		"storageReceiptJson": storage,
	}
	if install_state == "complete":
		value["storageReceiptSha256"] = "a" * 64
		value["completionReceiptSha256"] = "b" * 64
		if action != "retained":
			value["manifest"] = {
				"assignmentId": "a",
				"enrollmentId": "e",
				"questionId": "q",
				"problemId": "p",
				"versionId": "v",
			}
	result = json.dumps(value, separators=(",", ":"))
	return result


#============================================
def lifecycle_target(
	tmp_path: pathlib.Path,
	project: str = "containers",
	compose_files: tuple[pathlib.Path, ...] = (),
) -> local_stack_control.models.ComposeTarget:
	"""Build one selected target without a tracked configuration file."""
	result = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project=project,
		env_file=tmp_path / "containers/env.local",
		compose_files=compose_files,
		provider=local_stack_control.models.ComposeProvider(
			("podman", "compose"), "podman compose"
		),
		with_smtp=False,
		env_setting_names=(),
	)
	return result


#============================================
def live_demo_browser_compose_files(
	tmp_path: pathlib.Path,
) -> tuple[pathlib.Path, pathlib.Path]:
	"""Create the policy-declared TLS topology for one offline lifecycle test."""
	primary = tmp_path / "containers" / "compose.yaml"
	overlay = tmp_path / "tests" / "e2e" / "compose.live-demo-browser.yaml"
	primary.parent.mkdir()
	overlay.parent.mkdir(parents=True)
	primary.write_text("services: {}\n", encoding="ascii")
	overlay.write_text("services: {}\n", encoding="ascii")
	return primary.resolve(), overlay.resolve()


#============================================
def database_values() -> dict[str, str]:
	"""Return fixed values for the local Base Course child."""
	result = {
		"POSTGRES_USER": "ple",
		"POSTGRES_PASSWORD": "database-password",
		"POSTGRES_DB": "ple",
		"PLE_POSTGRES_HOST_PORT": "5432",
		"PLE_QUESTION_ID_SECRET_HOST_FILE": "containers/.secrets/question_id_secret",
	}
	return result


#============================================
def base_course_database_urls() -> tuple[str, str]:
	"""Return fixed child-only URLs for the closed Base Course pools."""
	return (
		"postgres://ple_base_course_installer_login:installer-private@127.0.0.1:5432/ple",
		"postgres://ple_base_course_app_login:application-private@127.0.0.1:5432/ple",
	)


#============================================
def diagnostic_path(target: local_stack_control.models.ComposeTarget) -> pathlib.Path:
	"""Return the selected environment's private Base Course diagnostic path."""
	return target.env_file.parent / local_stack_control.models.DEFAULT_BASE_COURSE_MANIFEST_FILE


#============================================
def test_decoder_preserves_canonical_storage_receipt() -> None:
	"""The outer response and embedded storage authority remain bound."""
	output = base_course_receipt("prepared", "installing")
	receipt = local_stack_control.base_course_lifecycle.decode(output, "prepare")
	expected = json.loads(output)

	assert (
		receipt.storage_receipt_bucket,
		receipt.storage_receipt_key,
		receipt.storage_receipt_json,
	) == (
		"private-content",
		"ple/live-demo/base-course-install-receipt.json",
		expected["storageReceiptJson"],
	)
	assert "storageReceiptSha256" not in expected
	assert "completionReceiptSha256" not in expected


#============================================
def test_storage_command_binds_label_bucket_key_and_receipt_stdin(
	tmp_path: pathlib.Path,
) -> None:
	"""The shell suffix binds its label, bucket, and key to zero, one, and two."""
	target = lifecycle_target(tmp_path)
	receipt = local_stack_control.base_course_lifecycle.decode(
		base_course_receipt("prepared", "installing"), "prepare"
	)
	runner = BaseCourseRunner("created")
	local_stack_control.base_course_lifecycle.ensure_storage_receipt(
		target, runner, receipt, {"PATH": "/usr/bin:/bin"}
	)
	argv, _, _, stdin = runner.calls[0]

	assert argv[-3:] == [
		"base-course-storage",
		"private-content",
		"ple/live-demo/base-course-install-receipt.json",
	]
	assert "mktemp /tmp/base-course-storage-receipt.XXXXXX" in argv[-4]
	assert 'chmod 600 "$receipt_file"' in argv[-4]
	assert 'mc cp --disable-multipart "$receipt_file" "local/$1/$2"' in argv[-4]
	assert "mc pipe" not in argv[-4]
	assert stdin == receipt.storage_receipt_json


#============================================
def test_installing_base_course_routes_receipt_and_writes_diagnostic(
	tmp_path: pathlib.Path,
) -> None:
	"""Installing state routes one receipt through storage and completion."""
	target = lifecycle_target(tmp_path)
	target.env_file.parent.mkdir()
	prepared = base_course_receipt("prepared", "installing")
	completed = base_course_receipt("installed", "complete")
	runner = BaseCourseRunner(prepared, "created", completed)
	database_urls = base_course_database_urls()
	preparation = local_stack_control.lifecycle.prepare_installed_base_course(
		runner, tmp_path, target, database_values(), {"PATH": "/usr/bin"}, database_urls
	)
	local_stack_control.lifecycle.finalize_installed_base_course(
		runner,
		tmp_path,
		target,
		database_values(),
		{"PATH": "/usr/bin"},
		preparation,
		database_urls,
	)
	canonical = json.loads(prepared)["storageReceiptJson"]
	command = [
		"cargo", "tools", "base-course",
		"--tenant", "00000000-0000-0000-0000-000000000100",
		"--instructor", "00000000-0000-0000-0000-000000000101",
		"--mary", "00000000-0000-0000-0000-000000000102",
		"--jack", "00000000-0000-0000-0000-000000000103",
		"--approval-candidate", "00000000-0000-0000-0000-000000000104",
		"--sysadmin", "00000000-0000-0000-0000-000000000105",
	]

	assert runner.calls[0][0] == command + ["--lifecycle-phase", "prepare"]
	assert runner.calls[2][0] == command + [
		"--lifecycle-phase", "install", "--storage-receipt", canonical,
	]
	assert (runner.calls[1][3], runner.calls[2][0][-1]) == (canonical, canonical)
	for call in (runner.calls[0], runner.calls[2]):
		environment = call[1]
		assert environment["PLE_BASE_COURSE_INSTALLER_DATABASE_URL"] == database_urls[0]
		assert environment["PLE_BASE_COURSE_APP_DATABASE_URL"] == database_urls[1]
		assert environment["PLE_BASE_COURSE_DEPLOYMENT_MODE"] == "local"
		assert "PLE_MIGRATION_DATABASE_URL" not in environment
	diagnostic = diagnostic_path(target)
	assert diagnostic.read_text(encoding="utf-8") == completed
	assert diagnostic.parent.name == ".runtime"
	assert stat.S_IMODE(diagnostic.parent.stat().st_mode) == 0o700
	assert stat.S_IMODE(diagnostic.stat().st_mode) == 0o600
	assert not (target.env_file.parent / "base-course.json").exists()


#============================================
def test_retained_base_course_has_zero_storage_calls(tmp_path: pathlib.Path) -> None:
	"""Retained state performs only its authoritative database read."""
	target = lifecycle_target(tmp_path)
	target.env_file.parent.mkdir()
	runner = BaseCourseRunner(base_course_receipt("retained", "complete"))
	database_urls = base_course_database_urls()
	preparation = local_stack_control.lifecycle.prepare_installed_base_course(
		runner, tmp_path, target, database_values(), {"PATH": "/usr/bin"}, database_urls
	)
	local_stack_control.lifecycle.finalize_installed_base_course(
		runner,
		tmp_path,
		target,
		database_values(),
		{"PATH": "/usr/bin"},
		preparation,
		database_urls,
	)

	assert [call[0][-2:] for call in runner.calls] == [
		["--lifecycle-phase", "prepare"]
	]


#============================================
def test_base_course_failure_redacts_both_child_database_urls() -> None:
	"""A failed runtime child cannot disclose either closed database credential."""
	database_urls = base_course_database_urls()
	result = local_stack_control.models.CommandResult(
		("cargo", "tools", "base-course"),
		17,
		"",
		"; ".join(database_urls),
	)

	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle.require_command(
			result, "installed Base Course prepare", database_urls
		)

	message = str(error.value)
	assert all(database_url not in message for database_url in database_urls)
	assert message.count("[private]") == 2


#============================================
def test_baseline_owner_finalizes_without_sysadmin_claim_context(
	tmp_path: pathlib.Path,
) -> None:
	"""The database and storage baseline owner does not exercise browser ownership setup."""
	target = lifecycle_target(tmp_path, "ple_live_demo_baseline_0123456789ab")
	target.env_file.parent.mkdir()
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy="live-demo-baseline",
		capability_file=tmp_path / "capability",
		project_prefix="ple_live_demo_baseline_",
		private_environment_file=target.env_file,
	)
	preparation = local_stack_control.base_course_lifecycle.decode(
		base_course_receipt("retained", "complete"), "prepare"
	)

	assert not local_stack_control.lifecycle_profiles.uses_live_demo_sysadmin_claim_context(disposable)

	local_stack_control.lifecycle.finalize_installed_base_course(
		BaseCourseRunner(),
		tmp_path,
		disposable,
		database_values(),
		{"PATH": "/usr/bin"},
		preparation,
		None,
	)

	assert diagnostic_path(target).is_file()


#============================================
def test_storage_refusal_fails_before_install_or_diagnostic(
	tmp_path: pathlib.Path,
) -> None:
	"""Mixed storage fails closed before the install phase."""
	target = lifecycle_target(tmp_path)
	target.env_file.parent.mkdir()
	runner = BaseCourseRunner(
		base_course_receipt("prepared", "installing"),
		refuse_storage=True,
	)
	database_urls = base_course_database_urls()
	preparation = local_stack_control.lifecycle.prepare_installed_base_course(
		runner, tmp_path, target, database_values(), {"PATH": "/usr/bin"}, database_urls
	)
	with pytest.raises(
		local_stack_control.models.ControllerError,
		match="storage receipt cannot safely resume \\(exit status 17; refused\\)",
	):
		local_stack_control.lifecycle.finalize_installed_base_course(
			runner,
			tmp_path,
			target,
			database_values(),
			{"PATH": "/usr/bin"},
			preparation,
			database_urls,
		)

	assert len(runner.calls) == 2
	assert not diagnostic_path(target).exists()


#============================================
def test_storage_failure_redacts_receipt_echoed_by_child(
	tmp_path: pathlib.Path,
) -> None:
	"""Receipt stdin remains private when a failing child echoes both streams."""
	target = lifecycle_target(tmp_path)
	receipt = local_stack_control.base_course_lifecycle.decode(
		base_course_receipt("prepared", "installing"), "prepare"
	)
	runner = BaseCourseRunner(
		storage_failure=(receipt.storage_receipt_json, receipt.storage_receipt_json),
	)

	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.base_course_lifecycle.ensure_storage_receipt(
			target, runner, receipt, {"PATH": "/usr/bin:/bin"}
		)

	message = str(error.value)
	assert receipt.storage_receipt_json not in message
	assert "[private]" in message and "exit status 17" in message


#============================================
def test_storage_failure_retains_nonsecret_child_stage(
	tmp_path: pathlib.Path,
) -> None:
	"""One failed receipt command identifies its non-secret child stage."""
	target = lifecycle_target(tmp_path)
	receipt = local_stack_control.base_course_lifecycle.decode(
		base_course_receipt("prepared", "installing"), "prepare"
	)
	runner = BaseCourseRunner(
		storage_failure=("", "storage-receipt-stage=inventory-private-content"),
	)

	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.base_course_lifecycle.ensure_storage_receipt(
			target, runner, receipt, {"PATH": "/usr/bin:/bin"}
		)

	assert "storage-receipt-stage=inventory-private-content" in str(error.value)


#============================================
def test_prepare_rejects_receipt_bound_to_another_generation() -> None:
	"""The host refuses an embedded generation that differs from Rust output."""
	value = json.loads(base_course_receipt("prepared", "installing"))
	storage = json.loads(value["storageReceiptJson"])
	storage["installationGeneration"] = "00000000-0000-0000-0000-000000000007"
	value["storageReceiptJson"] = json.dumps(storage, separators=(",", ":"))
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.base_course_lifecycle.decode(
			json.dumps(value, separators=(",", ":")), "prepare"
		)


#============================================
@pytest.mark.parametrize(
	"output",
	(
		"not-json",
		'{"schemaVersion":1}',
		base_course_receipt("prepared", "complete"),
	),
)
def test_decoder_rejects_malformed_or_wrong_phase_output(output: str) -> None:
	"""Ambiguous, incomplete, and wrong-phase output fails closed."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.base_course_lifecycle.decode(output, "prepare")


#============================================
@pytest.mark.parametrize(
	"completed",
	(
		"not-json",
		'{"schemaVersion":1,"action":"installed","installState":"complete",'
		'"baselineVersion":"base-course-v1","objectManifest":["unexpected"]}',
		'{"schemaVersion":1,"action":"installed","installState":"unexpected_state",'
		'"baselineVersion":"base-course-v1","objectManifest":[]}',
		'{"schemaVersion":1,"action":"installed","installState":"complete",'
		'"baselineVersion":"base-course-v1","objectManifest":[],"manifest":"unexpected"}',
	),
)
def test_invalid_completion_fails_before_diagnostic(
	tmp_path: pathlib.Path,
	completed: str,
) -> None:
	"""Only a complete authoritative result may replace the diagnostic."""
	target = lifecycle_target(tmp_path)
	target.env_file.parent.mkdir()
	runner = BaseCourseRunner(
		base_course_receipt("prepared", "installing"), "created", completed
	)
	database_urls = base_course_database_urls()
	preparation = local_stack_control.lifecycle.prepare_installed_base_course(
		runner, tmp_path, target, database_values(), {"PATH": "/usr/bin"}, database_urls
	)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.finalize_installed_base_course(
			runner,
			tmp_path,
			target,
			database_values(),
			{"PATH": "/usr/bin"},
			preparation,
			database_urls,
		)

	assert not diagnostic_path(target).exists()


#============================================
@pytest.mark.parametrize(
	"field",
	("storageReceiptSha256", "completionReceiptSha256"),
)
def test_decoder_rejects_invalid_completion_digest(field: str) -> None:
	"""Completed lifecycle output requires each lowercase hexadecimal digest."""
	value = json.loads(base_course_receipt("installed", "complete"))
	value[field] = "A" + "a" * 63

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.base_course_lifecycle.decode(
			json.dumps(value, separators=(",", ":")), "install"
		)


#============================================
def test_custom_target_skips_base_course(tmp_path: pathlib.Path) -> None:
	"""An unowned custom target receives no supported local Base Course seed."""
	runner = BaseCourseRunner('{"course_id":"unexpected"}\n')
	preparation = local_stack_control.lifecycle.prepare_installed_base_course(
		runner, tmp_path, lifecycle_target(tmp_path, "custom"), {}, {}, None
	)

	assert (preparation, runner.calls) == (None, [])


#============================================
def test_claim_context_preserves_matching_generation_and_rotates_new_generation(

	tmp_path: pathlib.Path,
) -> None:
	"""The authoritative receipt retains or rotates the private proof by generation."""
	path = tmp_path / ".runtime/live-demo-sysadmin-claim-context.json"
	first = local_stack_control.live_demo_claim_context.ensure_context(
		path,
		"00000000-0000-0000-0000-000000000006",
		local_stack_control.lifecycle.LOCAL_SYSADMIN_ID,
		lambda _: b"a" * 32,
	)
	same = local_stack_control.live_demo_claim_context.ensure_context(
		path,
		"00000000-0000-0000-0000-000000000006",
		local_stack_control.lifecycle.LOCAL_SYSADMIN_ID,
		lambda _: b"b" * 32,
	)
	rotated = local_stack_control.live_demo_claim_context.ensure_context(
		path,
		"00000000-0000-0000-0000-000000000007",
		local_stack_control.lifecycle.LOCAL_SYSADMIN_ID,
		lambda _: b"c" * 32,
	)

	assert (same.ownership_proof, rotated.installation_generation) == (
		first.ownership_proof,
		"00000000-0000-0000-0000-000000000007",
	)
	assert rotated.ownership_proof != first.ownership_proof
	assert stat.S_IMODE(path.stat().st_mode) == 0o600


#============================================
def test_malformed_claim_context_fails_without_replacement(tmp_path: pathlib.Path) -> None:
	"""An ambiguous private context never becomes a new unreviewed proof."""
	path = tmp_path / ".runtime/live-demo-sysadmin-claim-context.json"
	path.parent.mkdir()
	path.write_text("not-json", encoding="ascii")
	path.chmod(0o600)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.live_demo_claim_context.ensure_context(
			path,
			"00000000-0000-0000-0000-000000000006",
			local_stack_control.lifecycle.LOCAL_SYSADMIN_ID,
			lambda _: b"a" * 32,
		)

	assert path.read_text(encoding="ascii") == "not-json"


#============================================
def test_default_environment_omits_live_demo_identity_settings(
	tmp_path: pathlib.Path,
) -> None:
	"""The local deployment keeps demo-only identity settings out of its environment."""
	target = lifecycle_target(tmp_path)
	target.env_file.parent.mkdir()
	target.env_file.write_text("", encoding="ascii")
	target.env_file.chmod(0o600)

	local_stack_control.lifecycle.configure_default_environment(target, None)
	values = local_stack_control.env_file.env_settings(target.env_file)

	assert not any(name.startswith("PLE_LIVE_DEMO_") for name in values)
