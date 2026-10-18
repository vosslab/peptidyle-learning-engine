"""Bundled installation-data and Live Demo persona provisioning after readiness."""

import shlex
import sys

import local_stack_control.compose
import local_stack_control.disposable_stack_cleanup
import local_stack_control.env_file
import local_stack_control.lifecycle_commands
import local_stack_control.installation_import
import local_stack_control.live_demo_seed
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


LifecycleTarget = (
	local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget
)


#============================================
def selected_compose_target(target: LifecycleTarget) -> local_stack_control.models.ComposeTarget:
	"""Return the common Compose target without changing owner authority."""
	import local_stack_control.lifecycle
	return local_stack_control.lifecycle.target_of(target)


#============================================
def provision_ready_installation_data(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
	include_live_demo: bool = True,
) -> None:
	"""Run canonical bundled-content provisioning after application readiness."""
	selected = selected_compose_target(target)
	local_stack_control.installation_import.provision(selected, runner, include_live_demo)


#============================================
def backfill_question_thumbnails(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Run the existing migrator's fixed published-Revision thumbnail backfill."""
	run_question_thumbnail_maintenance(target, runner, "backfill")


#============================================
def run_question_thumbnail_maintenance(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
	action: str,
) -> None:
	"""Run one fixed thumbnail action and emit its complete redacted report."""
	if action != "backfill":
		raise local_stack_control.models.ControllerError(
			"unsupported fixed Question thumbnail maintenance action"
		)
	selected = selected_compose_target(target)
	arguments = [
		"--profile", "migration", "run", "--rm", "--no-deps", "-T",
		"database-migrator", "question-thumbnails", action,
	]
	local_stack_control.lifecycle_commands.report_step("compose " + shlex.join(arguments))
	result = runner.run(
		local_stack_control.compose.compose_argv(selected, arguments),
		local_stack_control.lifecycle_commands.child_environment(selected),
		selected.repo_root,
	)
	private_values = local_stack_control.disposable_stack_cleanup.private_environment_values(
		selected.env_file
	)
	# Preserve every safe progress line so an early failed revision remains visible.
	sys.stdout.write(
		local_stack_control.disposable_stack_cleanup.redact_private_values(
			result.stdout, private_values
		)
	)
	local_stack_control.lifecycle_commands.require_command(
		result, "Question thumbnail " + action, private_values
	)


#============================================
def record_live_demo_persona_account_ids(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Write server-minted persona Account IDs after installation-data provision."""
	selected = selected_compose_target(target)
	local_stack_control.env_file.require_mutation_env_file(selected.env_file)
	result = runner.run(
		local_stack_control.compose.compose_argv(
			selected,
			[
				"--profile", "migration", "run", "--rm", "--no-deps",
				"--entrypoint", "/bin/sh", "database-migrator", "-ec",
				local_stack_control.live_demo_seed.seeded_account_id_query_script(),
			],
		),
		local_stack_control.lifecycle_commands.child_environment(selected),
		selected.repo_root,
	)
	private_values = local_stack_control.disposable_stack_cleanup.private_environment_values(
		selected.env_file
	)
	local_stack_control.lifecycle_commands.require_command(
		result, "Live Demo Account ID recording", private_values
	)
	try:
		minted = local_stack_control.live_demo_seed.parse_seeded_account_id_report(
			result.stdout
		)
	except ValueError as error:
		raise local_stack_control.models.ControllerError(str(error)) from error
	values = local_stack_control.env_file.env_settings(selected.env_file)
	values.update(minted)
	content = "".join(f"{name}={value}\n" for name, value in values.items()).encode("utf-8")
	local_stack_control.private_files.write_atomic_file(selected.env_file, content, 0o600)


#============================================
def provision_local_sysadmin_totp(
	target: LifecycleTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Run the non-listening Morgan seed wrapper after Account installation."""
	selected = selected_compose_target(target)
	result = runner.run(
		local_stack_control.compose.compose_argv(
			selected,
			["run", "--rm", "--no-deps", "api", "--provision-local-sysadmin-totp"],
		),
		local_stack_control.lifecycle_commands.child_environment(selected),
		selected.repo_root,
	)
	private_values = local_stack_control.disposable_stack_cleanup.private_environment_values(
		selected.env_file
	)
	local_stack_control.lifecycle_commands.require_command(
		result, "Local Sysadmin TOTP provisioning", private_values
	)
