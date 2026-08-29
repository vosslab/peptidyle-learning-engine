"""Closed disposable Base Course PostgreSQL login provisioning."""

import secrets

import local_stack_control.compose
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.process


BASE_COURSE_INSTALLER_LOGIN = "ple_base_course_installer_login"
BASE_COURSE_INSTALLER_ROLE = "ple_base_course_installer"
BASE_COURSE_APP_LOGIN = "ple_base_course_app_login"
BASE_COURSE_APP_ROLE = "ple_app"
BASE_COURSE_FAST_PATH_LOGIN = "ple_base_course_fast_path_login"
BASE_COURSE_FAST_PATH_ROLE = "ple_accepted_submission_execution_fast_path"


#============================================
def provision(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
	environment: dict[str, str],
) -> tuple[str, str, str]:
	"""Provision the three post-migration capabilities needed by Base Course.

	Passwords remain process-local until their child-only URLs are constructed.
	The migration administrator provisions, but is never supplied to either Base
	Course capability (ASVS 2.3.1, 8.2.2, and 8.3.1).
	"""
	installer_password = secrets.token_hex(32)
	app_password = secrets.token_hex(32)
	fast_path_password = secrets.token_hex(32)
	child = dict(environment)
	child["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	argv = local_stack_control.compose.compose_argv(
		target,
		[
			"exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1",
			"-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"],
		],
	)
	result = runner.run(
		argv,
		child,
		target.repo_root,
		"BEGIN;\n"
		+ login_sql(
			BASE_COURSE_INSTALLER_LOGIN,
			BASE_COURSE_INSTALLER_ROLE,
			installer_password,
		)
		+ login_sql(BASE_COURSE_APP_LOGIN, BASE_COURSE_APP_ROLE, app_password)
		+ login_sql(BASE_COURSE_FAST_PATH_LOGIN, BASE_COURSE_FAST_PATH_ROLE, fast_path_password)
		+ "COMMIT;\n",
	)
	require_provision_success(result, (installer_password, app_password, fast_path_password))
	return (
		database_url(values, BASE_COURSE_INSTALLER_LOGIN, installer_password),
		database_url(values, BASE_COURSE_APP_LOGIN, app_password),
		database_url(values, BASE_COURSE_FAST_PATH_LOGIN, fast_path_password),
	)


#============================================
def require_provision_success(
	result: local_stack_control.models.CommandResult,
	private_values: tuple[str, ...],
) -> None:
	"""Raise a bounded redacted provisioning error without revealing a password."""
	if result.ok():
		return
	detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
		result, private_values
	)
	raise local_stack_control.models.ControllerError(
		"Base Course login provisioning failed "
		f"({detail}); retained stack resources are available for diagnostics"
	)


#============================================
def login_sql(login: str, role: str, password: str) -> str:
	"""Return fixed reset SQL for one closed Base Course capability."""
	if (login, role) not in (
		(BASE_COURSE_INSTALLER_LOGIN, BASE_COURSE_INSTALLER_ROLE),
		(BASE_COURSE_APP_LOGIN, BASE_COURSE_APP_ROLE),
		(BASE_COURSE_FAST_PATH_LOGIN, BASE_COURSE_FAST_PATH_ROLE),
	):
		raise local_stack_control.models.ControllerError("Base Course login profile is invalid")
	if len(password) != 64 or not password.isascii() or not password.isalnum():
		raise local_stack_control.models.ControllerError("Base Course login password is invalid")
	connection_limit = 4 if login == BASE_COURSE_FAST_PATH_LOGIN else 1
	return f"""DO $$
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
		WHERE parent.rolname = '{login}' OR member.rolname = '{login}'
	LOOP
		EXECUTE format('REVOKE %I FROM %I', membership.parent_name, membership.member_name);
	END LOOP;
END
$$;
ALTER ROLE {login}
	LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS
	CONNECTION LIMIT {connection_limit} PASSWORD '{password}';
DO $$
BEGIN
	EXECUTE format(
		'REVOKE ALL PRIVILEGES ON DATABASE %I FROM {login}', current_database()
	);
END
$$;
REVOKE ALL PRIVILEGES ON SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM {login};
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public FROM {login};
GRANT {role} TO {login} WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
"""


#============================================
def database_url(values: dict[str, str], login: str, password: str) -> str:
	"""Construct one process-local, fixed-login PostgreSQL URL."""
	port = values.get("PLE_POSTGRES_HOST_PORT", "5432")
	if (
		login not in (BASE_COURSE_INSTALLER_LOGIN, BASE_COURSE_APP_LOGIN, BASE_COURSE_FAST_PATH_LOGIN)
		or not port.isdecimal()
		or len(password) != 64
		or not password.isascii()
		or not password.isalnum()
	):
		raise local_stack_control.models.ControllerError("Base Course login database settings are invalid")
	return f"postgres://{login}:{password}@127.0.0.1:{port}/{values['POSTGRES_DB']}"


#============================================
def require_urls(urls: tuple[str, str, str] | None) -> tuple[str, str, str]:
	"""Require all three provisioned capabilities before a Base Course child starts."""
	if urls is None:
		raise local_stack_control.models.ControllerError("Base Course logins are unavailable")
	return urls


#============================================
def child_environment(
	environment: dict[str, str],
	values: dict[str, str],
	installer_database_url: str,
	app_database_url: str,
	fast_path_database_url: str,
) -> dict[str, str]:
	"""Give Base Course its three capabilities and question secret, but no ambient PLE state."""
	child = {
		name: value
		for name, value in environment.items()
		if not name.startswith("PLE_")
		and not name.startswith("COMPOSE_")
		and name not in ("PGPASSWORD", "DATABASE_URL")
	}
	child["PLE_BASE_COURSE_INSTALLER_DATABASE_URL"] = installer_database_url
	child["PLE_BASE_COURSE_APP_DATABASE_URL"] = app_database_url
	child["PLE_BASE_COURSE_FAST_PATH_DATABASE_URL"] = fast_path_database_url
	child["PLE_BASE_COURSE_DEPLOYMENT_MODE"] = "local"
	child["PLE_QUESTION_ID_SECRET_FILE"] = values["PLE_QUESTION_ID_SECRET_HOST_FILE"]
	return child
