"""Private database identities and connection values for local-stack lifecycle."""

# local repo modules
import local_stack_control.models


MIGRATION_DATABASE_OWNER = "ple_database_owner"
MIGRATION_ROLE = "ple_migrator"
ORDINARY_PLE_ROLES = (
	"ple_data_owner",
	"ple_private_owner",
	"ple_audit_owner",
	"ple_api_owner",
	"ple_app",
	"ple_auth",
	"ple_student",
)
SCHEMA_OWNER_ROLES = ORDINARY_PLE_ROLES[:4]


#============================================
def _safe_postgres_password(value: str) -> bool:
	"""Accept the closed ASCII credential alphabet used by private runtime URLs."""
	# ASVS 2.2.1 and 1.2.5: accept only the credential alphabet that remains
	# literal-safe in the fixed SQL delivered to psql stdin.
	return value.isascii() and value.replace("-", "").replace("_", "").isalnum()


#============================================
def _create_or_validate_role_sql(role: str, attributes: str, predicate: str) -> str:
	"""Create one fixed role or reject a pre-existing role outside its boundary."""
	return (
		"DO $$\nBEGIN\n"
		f"\tIF EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = '{role}') THEN\n"
		f"\t\tIF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles AS role "
		f"WHERE role.rolname = '{role}' AND {predicate}) THEN\n"
		"\t\t\tRAISE EXCEPTION USING ERRCODE = '55000', "
		f"MESSAGE = 'the existing {role} role does not satisfy the bootstrap contract';\n"
		"\t\tEND IF;\n"
		"\tELSE\n"
		f"\t\tCREATE ROLE {role} {attributes};\n"
		"\tEND IF;\nEND\n$$;\n"
	)


#============================================
def _migrator_memberships_are_expected_sql() -> str:
	"""Reject an existing migrator membership graph before the bootstrap mutates it."""
	return """DO $$
BEGIN
	IF (
		SELECT count(*)
		FROM pg_catalog.pg_auth_members AS membership
		WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = 'ple_migrator')
	) <> 0 AND (
		(
			SELECT count(*)
			FROM pg_catalog.pg_auth_members AS membership
			WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = 'ple_migrator')
		) <> 6 OR (
			SELECT count(*)
			FROM pg_catalog.pg_auth_members AS membership
			JOIN pg_catalog.pg_roles AS granted_role ON granted_role.oid = membership.roleid
			WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = 'ple_migrator')
			AND granted_role.rolname IN (
				'ple_database_owner', 'ple_unrelease_executor', 'ple_data_owner',
				'ple_private_owner', 'ple_audit_owner', 'ple_api_owner'
			)
			AND NOT membership.admin_option AND NOT membership.inherit_option AND membership.set_option
		) <> 6
	) THEN
		RAISE EXCEPTION USING ERRCODE = '55000',
			MESSAGE = 'the existing migrator memberships do not satisfy the bootstrap contract';
	END IF;
END
$$;
"""


#============================================
def migration_principal_bootstrap_sql(database_name: str, migrator_password: str) -> str:
	"""Build the closed fresh-database principal baseline delivered only over stdin."""
	if not database_name.replace("_", "").isalnum() or not _safe_postgres_password(migrator_password):
		raise local_stack_control.models.ControllerError(
			"local PostgreSQL migration principal settings are invalid"
		)
	ordinary_attributes = (
		"NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS"
	)
	ordinary_predicate = (
		"NOT role.rolcanlogin AND NOT role.rolinherit AND NOT role.rolsuper "
		"AND NOT role.rolcreatedb AND NOT role.rolcreaterole AND NOT role.rolreplication "
		"AND NOT role.rolbypassrls AND role.rolconnlimit = -1"
	)
	result = ["BEGIN;\n"]
	result.append(
		_create_or_validate_role_sql(
			MIGRATION_DATABASE_OWNER, ordinary_attributes, ordinary_predicate
		)
	)
	result.extend(
		_create_or_validate_role_sql(role, ordinary_attributes, ordinary_predicate)
		for role in ORDINARY_PLE_ROLES
	)
	result.extend(
		_create_or_validate_role_sql(role, ordinary_attributes, ordinary_predicate)
		for role in (
			"ple_public_asset_publisher",
			"ple_native_ple_grading_worker",
			"ple_webwork_grading_worker",
			"ple_imathas_question_backend_grading_worker",
			"ple_unrelease_executor",
		)
	)
	result.append(
		_create_or_validate_role_sql(
			MIGRATION_ROLE,
			"LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION "
			"NOBYPASSRLS CONNECTION LIMIT 2",
			"role.rolcanlogin AND NOT role.rolinherit AND NOT role.rolsuper "
			"AND NOT role.rolcreatedb AND NOT role.rolcreaterole AND NOT role.rolreplication "
			"AND NOT role.rolbypassrls AND role.rolconnlimit = 2",
		)
	)
	result.append(_migrator_memberships_are_expected_sql())
	result.extend(
		(
			f"ALTER ROLE {MIGRATION_ROLE} PASSWORD '{migrator_password}';\n",
			f"GRANT {MIGRATION_DATABASE_OWNER} TO {MIGRATION_ROLE} "
			"WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;\n",
			"GRANT ple_unrelease_executor TO ple_migrator "
			"WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;\n",
		)
	)
	result.extend(
		f"GRANT {role} TO {MIGRATION_ROLE} WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;\n"
		for role in SCHEMA_OWNER_ROLES
	)
	result.extend(
		(
			f"ALTER DATABASE {database_name} OWNER TO {MIGRATION_DATABASE_OWNER};\n",
			f"REVOKE ALL PRIVILEGES ON DATABASE {database_name} FROM PUBLIC;\n",
			f"GRANT CONNECT ON DATABASE {database_name} TO {MIGRATION_ROLE};\n",
			"REVOKE ALL PRIVILEGES ON SCHEMA public FROM PUBLIC;\n",
			f"GRANT USAGE ON SCHEMA pg_catalog TO {MIGRATION_ROLE};\n",
			"COMMIT;\n",
		)
	)
	return "".join(result)


#============================================
def postgres_role_sql(role: str, password: str) -> str:
	"""Build a closed local-role command delivered over stdin, never argv or diagnostics."""
	if not role.replace("_", "").isalnum() or not _safe_postgres_password(password):
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
