"""Private database identities and connection values for local-stack lifecycle."""

# local repo modules
import local_stack_control.models


MIGRATION_DATABASE_OWNER = "ple_database_owner"
MIGRATION_ROLE = "ple_migrator"


#============================================
def migration_principal_bootstrap_sql(database_name: str, migrator_password: str) -> str:
	"""Build the closed fresh-database principal baseline delivered only over stdin."""
	if not database_name.replace("_", "").isalnum() or not migrator_password.isalnum():
		raise local_stack_control.models.ControllerError(
			"local PostgreSQL migration principal settings are invalid"
		)
	return (
		f"CREATE ROLE {MIGRATION_DATABASE_OWNER} NOLOGIN NOINHERIT NOSUPERUSER "
		"NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;\n"
		"CREATE ROLE ple_public_asset_publisher "
		"NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE "
		"NOREPLICATION NOBYPASSRLS;\n"
		"CREATE ROLE ple_native_ple_grading_worker "
		"NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE "
		"NOREPLICATION NOBYPASSRLS;\n"
		"CREATE ROLE ple_webwork_grading_worker "
		"NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE "
		"NOREPLICATION NOBYPASSRLS;\n"
		f"CREATE ROLE {MIGRATION_ROLE} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB "
		"CREATEROLE NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 2;\n"
		f"ALTER ROLE {MIGRATION_ROLE} PASSWORD '{migrator_password}';\n"
		f"GRANT {MIGRATION_DATABASE_OWNER} TO {MIGRATION_ROLE} "
		"WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;\n"
		f"ALTER DATABASE {database_name} OWNER TO {MIGRATION_DATABASE_OWNER};\n"
		f"REVOKE ALL PRIVILEGES ON DATABASE {database_name} FROM PUBLIC;\n"
		f"GRANT CONNECT ON DATABASE {database_name} TO {MIGRATION_ROLE};\n"
		"REVOKE ALL PRIVILEGES ON SCHEMA public FROM PUBLIC;\n"
		f"GRANT CREATE, USAGE ON SCHEMA public TO {MIGRATION_ROLE};\n"
		f"GRANT USAGE ON SCHEMA pg_catalog TO {MIGRATION_ROLE};\n"
	)


#============================================
def postgres_role_sql(role: str, password: str) -> str:
	"""Build a closed local-role command delivered over stdin, never argv or diagnostics."""
	if not role.replace("_", "").isalnum() or not password.replace("-", "").replace("_", "").isalnum():
		raise local_stack_control.models.ControllerError("local PostgreSQL role settings are invalid")
	result = f"ALTER ROLE {role} PASSWORD '{password}';\n"
	return result


#============================================
def database_url(values: dict[str, str]) -> str:
	"""Construct a Compose-network PostgreSQL URL from selected private values."""
	port = values.get("PLE_POSTGRES_HOST_PORT", "5432")
	if not port.isdecimal():
		raise local_stack_control.models.ControllerError("selected PostgreSQL port is invalid")
	result = (
		f"postgres://{values['POSTGRES_USER']}:{values['POSTGRES_PASSWORD']}"
		f"@postgres:5432/{values['POSTGRES_DB']}"
	)
	return result
