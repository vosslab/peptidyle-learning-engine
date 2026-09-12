"""Offline safety contracts for the disposable replica adapter."""

import pathlib

import pytest

import local_stack_control.disposable_stack_adapter
import local_stack_control.models
import local_stack_control.disposable_stack_command
import local_stack_control.process


class CountRunner(local_stack_control.process.CommandRunner):
	"""Capture the one bounded count command without invoking an engine."""

	def __init__(self, stdout: str) -> None:
		"""Select one deterministic psql response."""
		self.stdout = stdout
		self.calls: list[tuple[list[str], str | None]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Record the closed argv and SQL stdin, then return the selected count."""
		self.calls.append((argv, stdin))
		return local_stack_control.models.CommandResult(tuple(argv), 0, self.stdout, "")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Reject a streamed command because the count result must be captured."""
		raise AssertionError("bounded count must not stream a subprocess")


#============================================
def fixed_replica_target(
	root: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile = (
		local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	),
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build one fixed profile target with its exact private database selection."""
	policy = local_stack_control.models.live_demo_profile_policy(profile)
	compose_files = tuple(root / relative for relative in policy.compose_relative_paths)
	for path in compose_files:
		path.parent.mkdir(parents=True, exist_ok=True)
		path.write_text("services: {}\n", encoding="ascii")
	environment = root / "env.local"
	environment.write_text(
		"POSTGRES_USER=ple_live_demo_browser\nPOSTGRES_DB=ple_live_demo_browser\n",
		encoding="ascii",
	)
	target = local_stack_control.models.ComposeTarget(
		repo_root=root,
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		env_file=environment,
		compose_files=compose_files,
		provider=local_stack_control.models.ComposeProvider(("compose",), "compose"),
		with_smtp=False,
		env_setting_names=("POSTGRES_USER", "POSTGRES_DB"),
	)
	return local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=root / "capability",
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=environment,
		live_demo_profile=profile,
	)


#============================================
def api_container(identifier: str) -> local_stack_control.models.ContainerResource:
	"""Build one running, label-resolved API replica."""
	return local_stack_control.models.ContainerResource(
		id=identifier,
		names=(),
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		service="api",
		state="running",
		running=True,
		exit_code=0,
		health="healthy",
		image="private-image",
		ports=(),
	)


#============================================
def test_fixed_replica_profile_refuses_stopping_the_only_api_instance(
	tmp_path: pathlib.Path,
) -> None:
	"""The outage cannot remove the only running API in its project."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		containers=(api_container("0123456789ab" + "0" * 52),),
		volumes=(),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.replica_stop_container(
			fixed_replica_target(tmp_path), snapshot, "api", "0123456789ab"
		)


#============================================
def test_replica_diagnostics_redact_private_values() -> None:
	"""Captured diagnostics cannot return an env secret or URL credential."""
	redacted = local_stack_control.disposable_stack_adapter.redact_diagnostics(
		"private-value postgres://user:password@postgres/database",
		("private-value",),
	)

	assert "private-value" not in redacted and "user:password" not in redacted


#============================================
def test_replica_compose_failure_receipt_redacts_stdout_and_stderr() -> None:
	"""Closed Compose failure evidence preserves clues without private values."""
	result = local_stack_control.models.CommandResult(
		argv=("compose",),
		returncode=125,
		stdout="stdout clue private-value",
		stderr="stderr clue postgres://user:password@postgres/database",
	)
	receipt = local_stack_control.disposable_stack_command.compose_failure_diagnostics(
		result, ("private-value", "password")
	)

	assert "stdout clue" in receipt and "stderr clue" in receipt
	assert "private-value" not in receipt and "user:password" not in receipt


#============================================
def test_replica_compose_success_forwards_stdout_and_stderr(
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Closed successful Compose calls preserve output for their caller."""
	result = local_stack_control.models.CommandResult(
		argv=("compose",),
		returncode=0,
		stdout="1|1|1|1|1\n",
		stderr="compose warning\n",
	)

	local_stack_control.disposable_stack_command.write_compose_success_output(result)

	captured = capsys.readouterr()
	assert captured.out == "1|1|1|1|1\n"
	assert captured.err == "compose warning\n"


#============================================
def test_replica_count_command_uses_profile_scoped_parameters(
	tmp_path: pathlib.Path,
) -> None:
	"""The closed child capability binds scoped UUIDs without interpolating them into SQL."""
	attempt = "00000000-0000-4000-8000-000000000200"
	argv, environment, sql = local_stack_control.disposable_stack_adapter.postgresql_count_command(
		fixed_replica_target(tmp_path), attempt
	)
	assert f"attempt_id={attempt}" in argv
	assert attempt not in sql
	assert ":'attempt_id'::uuid" in sql
	assert "ple_private.question_submission" in sql
	assert "ple_private.question_submission_grading" in sql
	assert "ple_private.grading_result" in sql
	assert "ple_audit.automated_grading_receipt" in sql
	assert "submission_idempotency" not in sql
	assert environment["COMPOSE_PROJECT_NAME"] == "ple-live-demo-browser"


#============================================
def test_postgresql_count_rejects_other_fixed_profiles(
	tmp_path: pathlib.Path,
) -> None:
	"""The browser profile does not grant the replica durability query."""
	target = fixed_replica_target(tmp_path, local_stack_control.models.LiveDemoProfile.BROWSER)
	with pytest.raises(
		local_stack_control.models.ControllerError, match="fixed replica profile"
	):
		local_stack_control.disposable_stack_adapter.postgresql_count_command(
			target,
			"00000000-0000-4000-8000-000000000200",
		)


#============================================
def test_postgresql_count_rejects_non_lowercase_uuid(tmp_path: pathlib.Path) -> None:
	"""A SQL metavariable cannot carry arbitrary text or alternate UUID spelling."""
	with pytest.raises(local_stack_control.models.ControllerError, match="lowercase UUID"):
		local_stack_control.disposable_stack_adapter.postgresql_count_command(
			fixed_replica_target(tmp_path),
			"00000000-0000-4000-8000-000000000200'::uuid; SELECT 1; --",
		)


#============================================
def test_postgresql_count_cli_rejects_generic_sql_or_compose_tail() -> None:
	"""The count adapter has no generic SQL or Compose argument escape hatch."""
	with pytest.raises(SystemExit):
		local_stack_control.disposable_stack_command.parse_args(
			[
				"postgresql-count",
				"--manifest", "/private/manifest",
				"--attempt-id", "00000000-0000-4000-8000-000000000200",
				"--sql", "DROP TABLE private_data",
			]
		)


#============================================
def test_postgresql_count_cli_emits_only_the_five_counts(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""A successful adapter call forwards one exact bounded count row and nothing else."""
	target = fixed_replica_target(tmp_path)
	runner = CountRunner("1|1|1|1|1\n")
	monkeypatch.setattr(
		local_stack_control.disposable_stack_adapter,
		"require_current_resource_capability",
		lambda selected_runner, disposable: None,
	)
	result = local_stack_control.disposable_stack_command.run_postgresql_count(
		runner,
		target,
		"00000000-0000-4000-8000-000000000200",
	)
	assert result == 0 and capsys.readouterr().out == "1|1|1|1|1\n"
	assert len(runner.calls) == 1
	assert runner.calls[0][1] is not None


#============================================
def test_postgresql_count_cli_rejects_malformed_result(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The bounded child rejects count output that is not exactly five integers."""
	target = fixed_replica_target(tmp_path)
	monkeypatch.setattr(
		local_stack_control.disposable_stack_adapter,
		"require_current_resource_capability",
		lambda selected_runner, disposable: None,
	)
	with pytest.raises(
		local_stack_control.models.ControllerError, match="invalid result"
	):
		local_stack_control.disposable_stack_command.run_postgresql_count(
			CountRunner("1|1|1|1"),
			target,
			"00000000-0000-4000-8000-000000000200",
		)
