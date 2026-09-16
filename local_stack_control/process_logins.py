"""Set up closed disposable PostgreSQL service logins."""

import pathlib
import secrets

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


API_LOGIN = "ple_api_login"
API_ROLES = ("ple_app", "ple_auth")
WORKER_LOGIN = "ple_worker_login"
WORKER_ROLES = ("ple_assessment_attempt_expiry_worker",)
PUBLISHER_LOGIN = "ple_publisher_login"
PUBLISHER_ROLES = ("ple_public_asset_publisher",)
COURSE_RETENTION_LOGIN = "ple_course_retention_login"
COURSE_RETENTION_ROLES = ("ple_course_retention_executor",)
COURSE_RETENTION_NOTIFIER_LOGIN = "ple_course_retention_notifier_login"
COURSE_RETENTION_NOTIFIER_ROLES: tuple[str, ...] = ()
COURSE_RETENTION_NOTIFIER_FUNCTIONS = (
	"ple_api.claim_course_retention_notification(timestamp with time zone, integer)",
	"ple_api.record_course_retention_notification_provider_acceptance("
	"uuid, uuid, uuid, timestamp with time zone)",
	"ple_api.fail_course_retention_notification_before_acceptance("
	"uuid, uuid, timestamp with time zone, text)",
)
LOGIN_PROFILES = (
	(API_LOGIN, API_ROLES, "PLE_API_DATABASE_URL"),
	(WORKER_LOGIN, WORKER_ROLES, "PLE_WORKER_DATABASE_URL"),
	(PUBLISHER_LOGIN, PUBLISHER_ROLES, "PLE_PUBLISHER_DATABASE_URL"),
	(COURSE_RETENTION_LOGIN, COURSE_RETENTION_ROLES, "PLE_COURSE_RETENTION_DATABASE_URL"),
	(
		COURSE_RETENTION_NOTIFIER_LOGIN,
		COURSE_RETENTION_NOTIFIER_ROLES,
		"PLE_COURSE_RETENTION_NOTIFIER_DATABASE_URL",
	),
)


#============================================
def setup_service_logins(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
	environment: dict[str, str],
) -> tuple[str, ...]:
	"""Reconcile disposable service logins and write capability-specific URLs.

	The administrator is confined to this migration-adjacent psql child.  Each
	service receives only its needed generated URL through the private Compose
	env file (ASVS 13.2.2, 13.3.2, and 14.2.4).  The return value is the matching
	host-only URL for an immediate, non-persistent application-schema check.
	"""
	passwords = tuple(secrets.token_hex(32) for _ in LOGIN_PROFILES)
	child = dict(environment)
	# The psql administrator consumes its one administrator password.  Service
	# credentials belong only in the mode-0600 Compose env file, never this child.
	for _, _, setting_name in LOGIN_PROFILES:
		child.pop(setting_name, None)
	child["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	argv = local_stack_control.compose.compose_argv(
		target,
		[
			"exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1",
			"-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"],
		],
	)
	sql = "BEGIN;\n"
	for (login, roles, _), password in zip(LOGIN_PROFILES, passwords, strict=True):
		sql += login_sql(login, roles, password)
	sql += "COMMIT;\n"
	result = runner.run(argv, child, target.repo_root, sql)
	private_values = (values["POSTGRES_PASSWORD"],) + passwords
	require_service_login_setup_success(result, private_values)
	urls = tuple(
		database_url(values, login, password)
		for (login, _, _), password in zip(LOGIN_PROFILES, passwords, strict=True)
	)
	write_runtime_urls(
		target.env_file,
		urls,
	)
	return tuple(
		host_database_url(values, login, password)
		for (login, _, _), password in zip(LOGIN_PROFILES, passwords, strict=True)
	)


#============================================
def login_sql(login: str, roles: tuple[str, ...], password: str) -> str:
	"""Return fixed, idempotent SQL for one allowlisted process profile."""
	valid_profiles = tuple(
		(profile_login, profile_roles)
		for profile_login, profile_roles, _ in LOGIN_PROFILES
	)
	if (login, roles) not in valid_profiles:
		raise local_stack_control.models.ControllerError("service login profile is invalid")
	if len(password) != 64 or not password.isascii() or not password.isalnum():
		raise local_stack_control.models.ControllerError("service login password is invalid")
	grants = "".join(
		f"GRANT {role} TO {login} WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;\n"
		for role in roles
	)
	function_grants = "".join(
		f"GRANT EXECUTE ON FUNCTION {function} TO {login};\n"
		for function in direct_functions(login)
	)
	schema_grants = "".join(
		f"GRANT USAGE ON SCHEMA {schema} TO {login};\n"
		for schema in direct_schemas(login)
	)
	sql = f"""DO $$
BEGIN
	IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{login}') THEN
		CREATE ROLE {login} LOGIN;
	END IF;
END
$$;
DO $$
DECLARE membership record;
BEGIN
	FOR membership IN
		SELECT parent.rolname AS parent_name, member.rolname AS member_name
		FROM pg_auth_members AS grant_map
		JOIN pg_roles AS parent ON parent.oid = grant_map.roleid
		JOIN pg_roles AS member ON member.oid = grant_map.member
		WHERE member.rolname = '{login}'
	LOOP
		EXECUTE format('REVOKE %I FROM %I', membership.parent_name, membership.member_name);
	END LOOP;
END
$$;
ALTER ROLE {login}
	LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS
	CONNECTION LIMIT 8 PASSWORD '{password}';
	DO $$
BEGIN
		EXECUTE format('REVOKE ALL PRIVILEGES ON DATABASE %I FROM {login}', current_database());
		EXECUTE format('GRANT CONNECT ON DATABASE %I TO {login}', current_database());
END
$$;
REVOKE ALL PRIVILEGES ON SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public FROM {login};
DO $$
DECLARE selected_schema record;
BEGIN
	-- A pre-existing service login may have been provisioned by an older
	-- controller.  Remove every direct PLE ACL before its one SET-only
	-- capability membership is reconciled; inherited authority is separately
	-- removed above.  ASVS 8.2.1 and 13.2.2.
	FOR selected_schema IN
		SELECT namespace.nspname
		FROM pg_catalog.pg_namespace AS namespace
		WHERE namespace.nspname LIKE 'ple\\_%' ESCAPE '\\'
	LOOP
		EXECUTE format('REVOKE ALL PRIVILEGES ON SCHEMA %I FROM {login}', selected_schema.nspname);
		EXECUTE format('REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA %I FROM {login}', selected_schema.nspname);
		EXECUTE format('REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA %I FROM {login}', selected_schema.nspname);
		EXECUTE format('REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA %I FROM {login}', selected_schema.nspname);
	END LOOP;
END
$$;
"""
	return sql + grants + schema_grants + function_grants


#============================================
def direct_functions(login: str) -> tuple[str, ...]:
	"""Return the exact direct-function authority for a closed login profile."""
	if login == COURSE_RETENTION_NOTIFIER_LOGIN:
		return COURSE_RETENTION_NOTIFIER_FUNCTIONS
	return ()


#============================================
def direct_schemas(login: str) -> tuple[str, ...]:
	"""Return schema USAGE needed solely to invoke a closed direct procedure set."""
	if login == COURSE_RETENTION_NOTIFIER_LOGIN:
		return ("ple_api",)
	return ()


#============================================
def database_url(values: dict[str, str], login: str, password: str) -> str:
	"""Construct one local process URL from validated private settings."""
	port = values.get("PLE_POSTGRES_HOST_PORT", "5432")
	database_name = values["POSTGRES_DB"]
	if (
		login not in tuple(profile_login for profile_login, _, _ in LOGIN_PROFILES)
		or not port.isdecimal()
		or not database_name.isascii()
		or not database_name.isidentifier()
		or len(password) != 64
		or not password.isascii()
		or not password.isalnum()
	):
		raise local_stack_control.models.ControllerError("service login database settings are invalid")
	result = f"postgres://{login}:{password}@postgres:5432/{database_name}"
	return result


#============================================
def host_database_url(values: dict[str, str], login: str, password: str) -> str:
	"""Construct the matching host-only URL for a bounded immediate verifier."""
	_ = database_url(values, login, password)
	port = values["PLE_POSTGRES_HOST_PORT"]
	result = f"postgres://{login}:{password}@127.0.0.1:{port}/{values['POSTGRES_DB']}"
	return result


#============================================
def write_runtime_urls(
	env_file: pathlib.Path,
	urls: tuple[str, ...],
) -> None:
	"""Replace every service-specific database URL inside private Compose input."""
	local_stack_control.env_file.require_mutation_env_file(env_file)
	if len(urls) != len(LOGIN_PROFILES) or len(set(urls)) != len(LOGIN_PROFILES):
		raise local_stack_control.models.ControllerError("service login URLs are invalid")
	for url, (login, _, _) in zip(urls, LOGIN_PROFILES, strict=True):
		if not url.startswith(f"postgres://{login}:"):
			raise local_stack_control.models.ControllerError("service login URLs are invalid")
	settings = local_stack_control.env_file.env_settings(env_file)
	for url, (_, _, setting_name) in zip(urls, LOGIN_PROFILES, strict=True):
		settings[setting_name] = url
	content = "".join(f"{name}={value}\n" for name, value in settings.items()).encode("utf-8")
	local_stack_control.private_files.write_atomic_file(env_file, content, 0o600)


#============================================
def require_service_login_setup_success(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...],
) -> None:
	"""Raise a bounded failure that never reproduces process credentials."""
	if result.ok():
		return
	detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
		result, private_values
	)
	raise local_stack_control.models.ControllerError(
		"Service Login Setup failed "
		f"({detail}); retained stack resources are available for diagnostics"
	)
