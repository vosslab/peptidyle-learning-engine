"""Bundled installation-data and Live Demo persona provisioning after readiness."""

import local_stack_control.compose
import local_stack_control.disposable_stack_cleanup
import local_stack_control.env_file
import local_stack_control.lifecycle_commands
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
	*,
	without_live_demo: bool,
) -> None:
	"""Run canonical bundled-content provisioning after application readiness."""
	selected = selected_compose_target(target)
	command = [
		"--profile", "migration", "run", "--rm", "--no-deps",
		"database-migrator", "installation-data", "provision",
	]
	if without_live_demo:
		command.append("--without-live-demo")
	result = runner.run(
		local_stack_control.compose.compose_argv(
			selected, command,
		),
		local_stack_control.lifecycle_commands.child_environment(selected),
		selected.repo_root,
	)
	private_values = local_stack_control.disposable_stack_cleanup.private_environment_values(
		selected.env_file
	)
	local_stack_control.lifecycle_commands.require_command(
		result, "Installation content provisioning", private_values
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
