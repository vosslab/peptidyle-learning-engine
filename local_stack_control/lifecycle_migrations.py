"""Network-scoped database lifecycle jobs for the local stack."""

import pathlib
import secrets

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.env_file
import local_stack_control.lifecycle_commands
import local_stack_control.lifecycle_database
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


MIGRATION_SERVICE = "database-migrator"
MIGRATION_URL_SETTING = "PLE_MIGRATION_DATABASE_URL"


#============================================
def run_migrations(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	values: dict[str, str],
	environment: dict[str, str],
) -> None:
	"""Run one canonical database action inside the Compose data network."""
	del repo_root, environment
	selected = target_of(target)
	operation = database_operation_for(target)
	url = migration_database_url_for(target, runner, values)
	build_database_migrator(selected, runner)
	run_database_command(selected, runner, operation, f"database {operation}", url)
	write_migration_database_url(selected.env_file, url)


#============================================
def verify_migrated_application_schema(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Verify the application role through the one restricted schema projection."""
	settings = local_stack_control.env_file.env_settings(target.env_file)
	try:
		url = settings["PLE_API_DATABASE_URL"]
	except KeyError as error:
		raise local_stack_control.models.ControllerError(
			"live-demo application-schema verification credentials are unavailable"
		) from error
	run_database_command(target, runner, "verify", "application-schema verification", url)


#============================================
def run_database_command(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	operation: str,
	description: str,
	migration_url: str | None = None,
) -> None:
	"""Run one database administration action without a host-published port."""
	environment = local_stack_control.lifecycle_commands.child_environment(target)
	if migration_url is not None:
		if not migration_url.startswith("postgres://") or any(
			character in migration_url for character in "\r\n\x00"
		):
			raise local_stack_control.models.ControllerError("migration database URL is invalid")
		environment[MIGRATION_URL_SETTING] = migration_url
	result = runner.run(
		local_stack_control.compose.compose_argv(
			target,
			[
				"--profile", "migration", "run", "--rm", "--no-deps",
				MIGRATION_SERVICE, "database", operation,
			],
		),
		environment,
		target.repo_root,
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		target.env_file
	)
	if migration_url is not None:
		private_values = (*private_values, migration_url)
	local_stack_control.lifecycle_commands.require_command(result, description, private_values)


#============================================
def database_operation_for(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> str:
	"""Use the existing private migration-URL setting as the first-start fact.

	The first lifecycle call has no migration URL, so it performs ``initialize``.
	A successful database action records a private URL and next uses ``migrate``.
	This controller does not inspect PostgreSQL roles, ACLs, schemas, or objects:
	the database coordinator accepts only its supported states and fails closed.
	"""
	settings = local_stack_control.env_file.env_settings(target_of(target).env_file)
	if MIGRATION_URL_SETTING in settings:
		return "migrate"
	return "initialize"


#============================================
def build_database_migrator(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Build the one existing short-lived migration image."""
	result = runner.run(
		local_stack_control.compose.compose_argv(
			target, ["--profile", "migration", "build", MIGRATION_SERVICE]
		),
		local_stack_control.lifecycle_commands.child_environment(target),
		target.repo_root,
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		target.env_file
	)
	local_stack_control.lifecycle_commands.require_command(
		result, "database migrator build", private_values
	)


#============================================
def migration_database_url_for(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
	operation: str | None = None,
) -> str:
	"""Run the known platform action and return the short-lived migrator URL."""
	selected = target_of(target)
	selected_operation = database_operation_for(target) if operation is None else operation
	if selected_operation not in ("initialize", "migrate"):
		raise local_stack_control.models.ControllerError("database lifecycle operation is invalid")
	migrator_password = secrets.token_hex(32)
	bootstrap_environment = local_stack_control.lifecycle_commands.child_environment(selected)
	# Both credentials stay in the restricted child environment or psql stdin.
	bootstrap_environment["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	bootstrap_argv = local_stack_control.compose.compose_argv(
		selected,
		[
			"exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1",
			"-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"],
		],
	)
	local_stack_control.lifecycle_commands.require_command(
		runner.run(
			bootstrap_argv,
			bootstrap_environment,
			selected.repo_root,
			migration_principal_sql(
				values["POSTGRES_DB"], migrator_password, selected_operation == "initialize"
			),
		),
		"migration-principal bootstrap",
		(values["POSTGRES_PASSWORD"], migrator_password),
	)
	migrator_values = dict(values)
	migrator_values["POSTGRES_USER"] = MIGRATION_ROLE
	migrator_values["POSTGRES_PASSWORD"] = migrator_password
	return database_url(migrator_values)


#============================================
def write_migration_database_url(env_file: pathlib.Path, url: str) -> None:
	"""Replace the private URL consumed by the migration container."""
	if not url.startswith("postgres://") or any(character in url for character in "\r\n\x00"):
		raise local_stack_control.models.ControllerError("migration database URL is invalid")
	local_stack_control.env_file.require_mutation_env_file(env_file)
	settings = local_stack_control.env_file.env_settings(env_file)
	settings[MIGRATION_URL_SETTING] = url
	content = "".join(f"{name}={value}\n" for name, value in settings.items()).encode("utf-8")
	local_stack_control.private_files.write_atomic_file(env_file, content, 0o600)


#============================================
def target_of(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ComposeTarget:
	"""Return the shared Compose target from a lifecycle owner."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return target.target
	return target


#============================================
def migration_principal_sql(
	database_name: str,
	migrator_password: str,
	initial_bootstrap: bool,
) -> str:
	"""Build one platform-owned principal action for a known lifecycle phase."""
	if initial_bootstrap:
		return migration_principal_bootstrap_sql(database_name, migrator_password)
	return postgres_role_sql(MIGRATION_ROLE, migrator_password)


#============================================
def migration_principal_bootstrap_sql(database_name: str, migrator_password: str) -> str:
	"""Build the fixed fresh-database platform principal baseline."""
	return local_stack_control.lifecycle_database.migration_principal_bootstrap_sql(
		database_name, migrator_password
	)


#============================================
def postgres_role_sql(role: str, password: str) -> str:
	"""Build one bounded local role-password update."""
	return local_stack_control.lifecycle_database.postgres_role_sql(role, password)


#============================================
def database_url(values: dict[str, str]) -> str:
	"""Construct the private Compose-network PostgreSQL URL for one migration job."""
	return local_stack_control.lifecycle_database.database_url(values)


MIGRATION_DATABASE_OWNER = local_stack_control.lifecycle_database.MIGRATION_DATABASE_OWNER
MIGRATION_ROLE = local_stack_control.lifecycle_database.MIGRATION_ROLE
