"""Closed disposable PostgreSQL API and worker login provisioning."""

import pathlib
import secrets

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


API_LOGIN = "ple_api_login"
WORKER_LOGIN = "ple_worker_login"
API_ROLES = ("ple_app", "ple_auth")
WORKER_ROLES = ("ple_app", "ple_accepted_submission_execution")


#============================================
def provision(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
	environment: dict[str, str],
) -> None:
	"""Reconcile disposable API and worker logins and write child-only URLs.

	The administrator is confined to this migration-adjacent psql child.  Each
	service later receives its own generated URL through the private Compose env
	file (ASVS 8.1-8.4, 13.3.1, and 14.2.4).
	"""
	api_password = secrets.token_hex(32)
	worker_password = secrets.token_hex(32)
	child = dict(environment)
	child["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	argv = local_stack_control.compose.compose_argv(
		target,
		[
			"exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1",
			"-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"],
		],
	)
	sql = "BEGIN;\n" + login_sql(API_LOGIN, API_ROLES, api_password)
	sql += login_sql(WORKER_LOGIN, WORKER_ROLES, worker_password) + "COMMIT;\n"
	result = runner.run(argv, child, target.repo_root, sql)
	require_provision_success(
		result, (values["POSTGRES_PASSWORD"], api_password, worker_password)
	)
	write_runtime_urls(
		target.env_file,
		database_url(values, API_LOGIN, api_password),
		database_url(values, WORKER_LOGIN, worker_password),
	)


#============================================
def login_sql(login: str, roles: tuple[str, ...], password: str) -> str:
	"""Return fixed, idempotent SQL for one allowlisted process profile."""
	if (login, roles) not in ((API_LOGIN, API_ROLES), (WORKER_LOGIN, WORKER_ROLES)):
		raise local_stack_control.models.ControllerError("process login profile is invalid")
	if len(password) != 64 or not password.isascii() or not password.isalnum():
		raise local_stack_control.models.ControllerError("process login password is invalid")
	grants = "".join(
		f"GRANT {role} TO {login} WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;\n"
		for role in roles
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
END
$$;
REVOKE ALL PRIVILEGES ON SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public FROM {login};
"""
	return sql + grants


#============================================
def database_url(values: dict[str, str], login: str, password: str) -> str:
	"""Construct one local process URL from validated private settings."""
	port = values.get("PLE_POSTGRES_HOST_PORT", "5432")
	if (
		login not in (API_LOGIN, WORKER_LOGIN)
		or not port.isdecimal()
		or len(password) != 64
		or not password.isascii()
		or not password.isalnum()
	):
		raise local_stack_control.models.ControllerError("process login database settings are invalid")
	result = f"postgres://{login}:{password}@postgres:5432/{values['POSTGRES_DB']}"
	return result


#============================================
def write_runtime_urls(
	env_file: pathlib.Path,
	api_url: str,
	worker_url: str,
) -> None:
	"""Replace only service URLs inside the selected mode-0600 Compose input."""
	local_stack_control.env_file.require_mutation_env_file(env_file)
	api_valid = api_url.startswith("postgres://ple_api_login:")
	worker_valid = worker_url.startswith("postgres://ple_worker_login:")
	if api_url == worker_url or not api_valid or not worker_valid:
		raise local_stack_control.models.ControllerError("process login URLs are invalid")
	settings = local_stack_control.env_file.env_settings(env_file)
	settings["PLE_API_DATABASE_URL"] = api_url
	settings["PLE_WORKER_DATABASE_URL"] = worker_url
	content = "".join(f"{name}={value}\n" for name, value in settings.items()).encode("utf-8")
	local_stack_control.private_files.write_atomic_file(env_file, content, 0o600)


#============================================
def require_provision_success(
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
		"process login provisioning failed "
		f"({detail}); retained stack resources are available for diagnostics"
	)
