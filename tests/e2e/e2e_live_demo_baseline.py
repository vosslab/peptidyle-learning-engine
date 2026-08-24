#!/usr/bin/env python3
"""Exercise the installed Base Course against disposable PostgreSQL and MinIO."""

import hashlib
import json
import pathlib
import subprocess
import sys
import urllib.parse
import uuid


SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import e2e_live_demo_stack
import local_stack_control.base_course_logins
import local_stack_control.base_course_lifecycle
import local_stack_control.lifecycle
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.process


TENANT_ID = "00000000-0000-0000-0000-000000000100"
PRIMARY_INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
MARY_ID = "00000000-0000-0000-0000-000000000102"
JACK_ID = "00000000-0000-0000-0000-000000000103"
APPROVAL_CANDIDATE_ID = "00000000-0000-0000-0000-000000000104"
SYSADMIN_ID = "00000000-0000-0000-0000-000000000105"
USER_CREATED_COURSE_ID = "00000000-0000-4000-8000-000000000999"
RECEIPT_BUCKET = "private-content"
RECEIPT_KEY = "ple/live-demo/base-course-install-receipt.json"


#============================================
def database_url(stack: e2e_live_demo_stack.DisposableStack, database: str) -> str:
	"""Build one loopback URL for a disposable database."""
	user = urllib.parse.quote(e2e_live_demo_stack.POSTGRES_USER, safe="")
	password = urllib.parse.quote(stack.postgres_password, safe="")
	name = urllib.parse.quote(database, safe="")
	result = f"postgres://{user}:{password}@127.0.0.1:{stack.postgres_port}/{name}"
	return result


#============================================
def database_values(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> dict[str, str]:
	"""Bind private selected values to one disposable database only."""
	values = dict(stack.values)
	values["POSTGRES_DB"] = database
	return values


#============================================
def migrate_database(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> None:
	"""Apply the complete schema using migration administration only (ASVS 2.3.1)."""
	migration_database_url = database_url(stack, database)
	environment = local_stack_control.lifecycle.child_environment(stack.disposable.target)
	environment["PLE_MIGRATION_DATABASE_URL"] = migration_database_url
	result = stack.runner.run(
		["cargo", "tools", "database", "migrate"], environment, stack.root
	)
	local_stack_control.lifecycle.require_command(
		result,
		f"live-demo baseline migration for {database}",
		(migration_database_url,),
	)


#============================================
def provision_base_course_database_urls(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> tuple[str, str]:
	"""Create closed child logins only after this database has its complete schema."""
	values = database_values(stack, database)
	return local_stack_control.base_course_logins.provision(
		stack.disposable.target,
		stack.runner,
		values,
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
	)


#============================================
def prepare_database(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> tuple[str, str]:
	"""Migrate one disposable database, then issue its two closed runtime identities."""
	migrate_database(stack, database)
	return provision_base_course_database_urls(stack, database)


#============================================
def phase_environment(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	base_course_database_urls: tuple[str, str],
) -> dict[str, str]:
	"""Build an answerable Base Course child without migration administration."""
	installer_database_url, app_database_url = local_stack_control.base_course_logins.require_urls(
		base_course_database_urls
	)
	return local_stack_control.base_course_logins.child_environment(
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
		database_values(stack, database),
		installer_database_url,
		app_database_url,
	)


#============================================
def base_course_private_values(
	base_course_database_urls: tuple[str, str],
) -> tuple[str, ...]:
	"""Return both closed URLs and their passwords for bounded child diagnostics."""
	private_values = list(base_course_database_urls)
	for database_url_value in base_course_database_urls:
		password = urllib.parse.urlsplit(database_url_value).password
		if password is not None:
			private_values.append(password)
	return tuple(private_values)


#============================================
def run_phase(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	base_course_database_urls: tuple[str, str],
	phase: str,
	receipt: str | None = None,
) -> local_stack_control.base_course_lifecycle.Receipt:
	"""Run one ordinary Base Course CLI phase."""
	environment = phase_environment(stack, database, base_course_database_urls)
	result = local_stack_control.lifecycle.run_base_course_phase(
		stack.runner,
		stack.root,
		environment,
		phase,
		receipt,
		private_values=base_course_private_values(base_course_database_urls),
	)
	return result


#============================================
def deterministic_id(label: str) -> str:
	"""Reproduce the installed baseline's reviewed deterministic identity."""
	hasher = hashlib.sha256()
	hasher.update(b"ple-installed-base-course-v1:")
	hasher.update(uuid.UUID(TENANT_ID).bytes)
	hasher.update(label.encode("ascii"))
	value = bytearray(hasher.digest()[:16])
	value[6] = (value[6] & 0x0F) | 0x50
	value[8] = (value[8] & 0x3F) | 0x80
	result = str(uuid.UUID(bytes=bytes(value)))
	return result


#============================================
def require_exact_value(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	sql: str,
	expected: str,
	description: str,
) -> None:
	"""Require one named live-demo record to retain its semantic value."""
	actual = stack.psql(database, sql)
	if actual != expected:
		raise local_stack_control.models.ControllerError(
			f"live-demo baseline E2E {description} differs: {actual!r}"
		)


#============================================
def verify_accounts(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> None:
	"""Verify each named account and its ordinary platform-role state."""
	accounts = (
		(
			PRIMARY_INSTRUCTOR_ID,
			"elena.rivera@live-demo.ple.example",
			"Dr. Elena Rivera",
			"[]",
			"primary Instructor account",
		),
		(
			MARY_ID,
			"mary.okafor@live-demo.ple.example",
			"Mary Okafor",
			"[]",
			"Mary account",
		),
		(
			JACK_ID,
			"jack.chen@live-demo.ple.example",
			"Jack Chen",
			"[]",
			"Jack account",
		),
		(
			APPROVAL_CANDIDATE_ID,
			"avery.singh@live-demo.ple.example",
			"Avery Singh",
			"[]",
			"approval-candidate account",
		),
		(
			SYSADMIN_ID,
			"morgan.reyes@live-demo.ple.example",
			"Morgan Reyes",
			'["sysadmin"]',
			"Sysadmin account",
		),
	)
	for user_id, email, name, roles, description in accounts:
		require_exact_value(
			stack,
			database,
			"SELECT normalized_email || '|' || delivery_email || '|' || display_name "
			"|| '|' || platform_roles::text FROM ple_account "
			f"WHERE user_id = '{user_id}'",
			f"{email}|{email}|{name}|{roles}",
			description,
		)


#============================================
def verify_courses_and_memberships(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> tuple[str, str]:
	"""Verify the two named courses and the complete participant matrix."""
	base_course_id = deterministic_id("course")
	practice_course_id = deterministic_id("practice-course")
	require_exact_value(
		stack,
		database,
		f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
		f"AND course_id = '{base_course_id}'",
		"Biochemistry Base Course",
		"named Base Course",
	)
	require_exact_value(
		stack,
		database,
		f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
		f"AND course_id = '{practice_course_id}'",
		"Genetics Practice Course",
		"named Genetics Practice Course",
	)
	memberships = (
		(base_course_id, PRIMARY_INSTRUCTOR_ID, "instructor|active", "Base Course Instructor"),
		(base_course_id, MARY_ID, "student|active", "Base Course Mary"),
		(base_course_id, JACK_ID, "student|active", "Base Course Jack"),
		(base_course_id, APPROVAL_CANDIDATE_ID, "", "Base Course approval candidate absence"),
		(base_course_id, SYSADMIN_ID, "", "Base Course Sysadmin absence"),
		(practice_course_id, PRIMARY_INSTRUCTOR_ID, "", "practice-course Instructor absence"),
		(practice_course_id, MARY_ID, "", "practice-course Mary absence"),
		(practice_course_id, JACK_ID, "", "practice-course Jack absence"),
		(
			practice_course_id,
			APPROVAL_CANDIDATE_ID,
			"student|active",
			"practice-course approval candidate",
		),
		(practice_course_id, SYSADMIN_ID, "instructor|active", "practice-course Sysadmin"),
	)
	for course_id, user_id, expected, description in memberships:
		require_exact_value(
			stack,
			database,
			"SELECT role || '|' || status FROM course_member "
			f"WHERE tenant_id = '{TENANT_ID}' AND course_id = '{course_id}' "
			f"AND user_id = '{user_id}'",
			expected,
			description,
		)
	return base_course_id, practice_course_id


#============================================
def verify_unclaimed_authentication_state(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> None:
	"""Verify candidate approval and Sysadmin ownership remain unclaimed."""
	absent = (
		(
			f"SELECT user_id FROM instructor_approval WHERE user_id = '{APPROVAL_CANDIDATE_ID}'",
			"approval-candidate approval",
		),
		(
			f"SELECT user_id FROM account_passkey WHERE user_id = '{SYSADMIN_ID}'",
			"seeded Sysadmin passkey",
		),
		(
			"SELECT user_id FROM account_authentication_session "
			f"WHERE user_id = '{SYSADMIN_ID}'",
			"seeded Sysadmin account session",
		),
		(
			f"SELECT user_id FROM auth_session WHERE tenant_id = '{TENANT_ID}' "
			f"AND user_id = '{SYSADMIN_ID}'",
			"seeded Sysadmin tenant session",
		),
	)
	for sql, description in absent:
		require_exact_value(stack, database, sql, "", description)


#============================================
def verify_exact_baseline(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
) -> tuple[str, str]:
	"""Verify named baseline records, normal activity states, and lifecycle receipt."""
	workspace_id = deterministic_id("workspace")
	assignment_id = deterministic_id("assignment")
	assignment_item_id = deterministic_id("assignment-item")
	problem_id = deterministic_id("problem")
	version_id = deterministic_id("version")
	state = stack.psql(
		database,
		"SELECT state || '|' || baseline_version || '|' || installation_generation::text "
		"|| '|' || storage_receipt_sha256 FROM live_demo_install_state",
	)
	parts = state.split("|")
	if len(parts) != 4 or parts[0] != "complete" or parts[1] != "base-course-v1":
		raise local_stack_control.models.ControllerError(
			"live-demo baseline E2E lifecycle marker is not complete"
		)
	verify_accounts(stack, database)
	course_id, _ = verify_courses_and_memberships(stack, database)
	verify_unclaimed_authentication_state(stack, database)
	require_exact_value(
		stack, database,
		f"SELECT title || '|' || lifecycle FROM assignment WHERE tenant_id = '{TENANT_ID}' "
		f"AND assignment_id = '{assignment_id}' AND course_id = '{course_id}'",
		"Peptide Bonds: Structure and Resonance|published", "named Base Course assignment",
	)
	require_exact_value(
		stack, database,
		f"SELECT lifecycle FROM problem WHERE problem_id = '{problem_id}'",
		"published", "named published problem",
	)
	require_exact_value(
		stack, database,
		f"SELECT title || '|' || lifecycle FROM problem_version WHERE problem_id = '{problem_id}' "
		f"AND version_id = '{version_id}'",
		"Peptide bond resonance and planarity|published", "named published problem version",
	)
	require_exact_value(
		stack, database,
		f"SELECT problem_id || '|' || version_id || '|' || delivery_state FROM assignment_item "
		f"WHERE tenant_id = '{TENANT_ID}' AND assignment_id = '{assignment_id}' "
		f"AND assignment_item_id = '{assignment_item_id}'",
		f"{problem_id}|{version_id}|active", "named assignment item",
	)
	require_exact_value(
		stack, database,
		f"SELECT workspace_id FROM workspace_draft WHERE tenant_id = '{TENANT_ID}' "
		f"AND workspace_id = '{workspace_id}'",
		"", "published workspace draft removal",
	)
	require_exact_value(
		stack,
		database,
		"SELECT enrollment.user_id::text || '|' "
		"|| (activity_run.completed_at IS NOT NULL)::text || '|' || attempt.attempt_status "
		"FROM assignment_run AS activity_run JOIN enrollment AS enrollment "
		"ON enrollment.tenant_id = activity_run.tenant_id "
		"AND enrollment.enrollment_id = activity_run.enrollment_id "
		"JOIN question_attempt AS attempt ON attempt.tenant_id = activity_run.tenant_id "
		"AND attempt.run_id = activity_run.run_id "
		f"WHERE activity_run.tenant_id = '{TENANT_ID}' "
		f"AND activity_run.run_id = '{deterministic_id('run')}' "
		f"AND attempt.attempt_id = '{deterministic_id('attempt')}'",
		f"{MARY_ID}|true|submitted",
		"Mary completed activity",
	)
	require_exact_value(
		stack, database,
		f"SELECT attempt_id FROM submission WHERE tenant_id = '{TENANT_ID}' "
		f"AND attempt_id = '{deterministic_id('attempt')}'",
		deterministic_id("attempt"), "completed learner submission",
	)
	require_exact_value(
		stack,
		database,
		"SELECT enrollment.user_id::text || '|' "
		"|| (activity_run.completed_at IS NULL)::text || '|' || attempt.attempt_status "
		"FROM assignment_run AS activity_run JOIN enrollment AS enrollment "
		"ON enrollment.tenant_id = activity_run.tenant_id "
		"AND enrollment.enrollment_id = activity_run.enrollment_id "
		"JOIN question_attempt AS attempt ON attempt.tenant_id = activity_run.tenant_id "
		"AND attempt.run_id = activity_run.run_id "
		f"WHERE activity_run.tenant_id = '{TENANT_ID}' "
		f"AND activity_run.run_id = '{deterministic_id('additional-run')}' "
		f"AND attempt.attempt_id = '{deterministic_id('additional-attempt')}'",
		f"{JACK_ID}|true|in_progress",
		"Jack active activity",
	)
	return parts[2], parts[3]


#============================================
def expect_storage_refusal(
	stack: e2e_live_demo_stack.DisposableStack,
	prepared: local_stack_control.base_course_lifecycle.Receipt,
) -> None:
	"""Require current MinIO inventory to fail the installing-state boundary."""
	try:
		local_stack_control.base_course_lifecycle.ensure_storage_receipt(
			stack.disposable.target,
			stack.runner,
			prepared,
			local_stack_control.lifecycle.child_environment(stack.disposable.target),
		)
	except local_stack_control.models.ControllerError:
		return
	raise local_stack_control.models.ControllerError(
		"live-demo baseline E2E unsafe storage inventory unexpectedly resumed"
	)


#============================================
def verify_storage_protocol(
	stack: e2e_live_demo_stack.DisposableStack,
	prepared: local_stack_control.base_course_lifecycle.Receipt,
) -> None:
	"""Prove inventory failure, mixed refusal, exact creation, and exact resume."""
	stack.remove_empty_bucket("public-assets")
	expect_storage_refusal(stack, prepared)
	stack.create_bucket("public-assets")

	stack.put_object("temp-processing", "unexpected/mixed.txt", "mixed")
	expect_storage_refusal(stack, prepared)
	stack.remove_object("temp-processing", "unexpected/mixed.txt")

	environment = local_stack_control.lifecycle.child_environment(stack.disposable.target)
	local_stack_control.base_course_lifecycle.ensure_storage_receipt(
		stack.disposable.target, stack.runner, prepared, environment
	)
	actual = stack.read_object(prepared.storage_receipt_bucket, prepared.storage_receipt_key)
	if actual != prepared.storage_receipt_json:
		raise local_stack_control.models.ControllerError(
			"MinIO shell arguments did not place the exact receipt at bucket/key $1/$2"
		)

	stack.put_object("temp-processing", "unexpected/second.txt", "second")
	expect_storage_refusal(stack, prepared)
	stack.remove_object("temp-processing", "unexpected/second.txt")
	local_stack_control.base_course_lifecycle.ensure_storage_receipt(
		stack.disposable.target, stack.runner, prepared, environment
	)


#============================================
def expect_object_absent(
	stack: e2e_live_demo_stack.DisposableStack,
	bucket: str,
	key: str,
	description: str,
) -> None:
	"""Require a reset store not to retain one meaningful ordinary object."""
	try:
		stack.read_object(bucket, key)
	except local_stack_control.models.ControllerError:
		return
	raise local_stack_control.models.ControllerError(
		f"live-demo baseline E2E retained {description} after a fresh reset"
	)


#============================================
def verify_generation_bound_receipt(
	stack: e2e_live_demo_stack.DisposableStack,
	prepared: local_stack_control.base_course_lifecycle.Receipt,
	generation: str,
	receipt_sha256: str,
) -> None:
	"""Require the authoritative receipt object to bind the installed generation."""
	receipt = stack.read_object(RECEIPT_BUCKET, RECEIPT_KEY)
	if receipt != prepared.storage_receipt_json:
		raise local_stack_control.models.ControllerError(
			"MinIO receipt does not match the prepared installation generation"
		)
	if hashlib.sha256(receipt.encode("ascii")).hexdigest() != receipt_sha256:
		raise local_stack_control.models.ControllerError(
			"PostgreSQL receipt hash does not bind the actual MinIO receipt"
		)
	value = json.loads(receipt)
	expected = {
		"schemaVersion": 1,
		"baselineVersion": "base-course-v1",
		"installationGeneration": generation,
		"storageReceiptBucket": RECEIPT_BUCKET,
		"storageReceiptKey": RECEIPT_KEY,
		"objectManifest": [],
	}
	if value != expected:
		raise local_stack_control.models.ControllerError(
			"receipt does not authoritatively bind the installed baseline generation"
		)


#============================================
def install_trigger(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	table: str,
) -> None:
	"""Install one transaction-failing trigger at a reviewed seed boundary."""
	allowed = {
		"assignment", "enrollment", "problem", "question_attempt", "submission",
		"workspace_draft",
	}
	if table not in allowed:
		raise local_stack_control.models.ControllerError("unsupported interruption table")
	sql = (
		"CREATE FUNCTION public.ple_e2e_interrupt_seed() RETURNS trigger LANGUAGE plpgsql AS "
		"$$ BEGIN RAISE EXCEPTION 'intentional live-demo seed interruption'; END $$; "
		f"CREATE TRIGGER ple_e2e_interrupt_seed BEFORE INSERT ON public.{table} "
		"FOR EACH ROW EXECUTE FUNCTION public.ple_e2e_interrupt_seed()"
	)
	stack.psql(database, sql)


#============================================
def remove_trigger(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	table: str,
) -> None:
	"""Remove the one injected interruption before retrying the real installer."""
	stack.psql(
		database,
		f"DROP TRIGGER ple_e2e_interrupt_seed ON public.{table}; "
		"DROP FUNCTION public.ple_e2e_interrupt_seed()",
	)


#============================================
def expect_phase_failure(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	base_course_database_urls: tuple[str, str],
	receipt: str,
) -> None:
	"""Require the install phase to fail at an injected interruption."""
	try:
		run_phase(stack, database, base_course_database_urls, "install", receipt)
	except local_stack_control.models.ControllerError:
		return
	raise local_stack_control.models.ControllerError(
		"live-demo baseline E2E injected seed interruption unexpectedly completed"
	)


#============================================
def verify_prefix_state(
	stack: e2e_live_demo_stack.DisposableStack,
	database: str,
	boundary: str,
) -> None:
	"""Require one representative retained publication or activity boundary."""
	if boundary == "publication":
		sql = (
			f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
			f"AND course_id = '{deterministic_id('course')}'"
		)
		expected = "Biochemistry Base Course"
	elif boundary == "activity":
		sql = (
			f"SELECT completed_at IS NULL FROM assignment_run WHERE tenant_id = '{TENANT_ID}' "
			f"AND run_id = '{deterministic_id('run')}'"
		)
		expected = "t"
	else:
		raise local_stack_control.models.ControllerError("unsupported interruption boundary")
	if stack.psql(database, sql) != expected:
		raise local_stack_control.models.ControllerError(
			f"live-demo baseline E2E did not retain the {boundary} boundary"
		)
	if stack.psql(database, "SELECT state FROM live_demo_install_state") != "installing":
		raise local_stack_control.models.ControllerError(
			"interrupted Base Course did not remain in installing state"
		)


#============================================
def verify_interrupted_boundaries(stack: e2e_live_demo_stack.DisposableStack) -> None:
	"""Prove representative publication and activity boundaries resume safely."""
	boundaries = (
		("publication", "workspace_draft"),
		("activity", "question_attempt"),
	)
	for boundary, table in boundaries:
		database = f"ple_live_demo_{boundary}_boundary"
		stack.create_database(database)
		base_course_database_urls = prepare_database(stack, database)
		prepared = run_phase(stack, database, base_course_database_urls, "prepare")
		install_trigger(stack, database, table)
		expect_phase_failure(
			stack, database, base_course_database_urls, prepared.storage_receipt_json
		)
		verify_prefix_state(stack, database, boundary)
		remove_trigger(stack, database, table)
		completed = run_phase(
			stack,
			database,
			base_course_database_urls,
			"install",
			prepared.storage_receipt_json,
		)
		if completed.install_state != "complete":
			raise local_stack_control.models.ControllerError(
				f"live-demo baseline E2E {boundary} boundary did not resume"
			)
		verify_exact_baseline(stack, database)


#============================================
def base_course_argv(receipt: str) -> list[str]:
	"""Return the ordinary install-phase command for concurrency proof."""
	result = [
		"cargo", "tools", "base-course", "--tenant", TENANT_ID,
		"--instructor", PRIMARY_INSTRUCTOR_ID, "--mary", MARY_ID, "--jack", JACK_ID,
		"--approval-candidate", APPROVAL_CANDIDATE_ID, "--sysadmin", SYSADMIN_ID,
		"--lifecycle-phase", "install", "--storage-receipt", receipt,
	]
	return result


#============================================
def verify_concurrent_installers(stack: e2e_live_demo_stack.DisposableStack) -> None:
	"""Prove two real installers serialize to one completed baseline."""
	database = "ple_live_demo_concurrent"
	stack.create_database(database)
	base_course_database_urls = prepare_database(stack, database)
	prepared = run_phase(stack, database, base_course_database_urls, "prepare")
	environment = phase_environment(stack, database, base_course_database_urls)
	argv = base_course_argv(prepared.storage_receipt_json)
	first = subprocess.Popen(
		argv, cwd=stack.root, env=environment, text=True,
		stdout=subprocess.PIPE, stderr=subprocess.PIPE,
	)
	second = subprocess.Popen(
		argv, cwd=stack.root, env=environment, text=True,
		stdout=subprocess.PIPE, stderr=subprocess.PIPE,
	)
	first_stdout, first_stderr = first.communicate()
	second_stdout, second_stderr = second.communicate()
	if first.returncode != 0 or second.returncode != 0:
		failure = local_stack_control.models.CommandResult(
			tuple(argv),
			17,
			first_stdout + second_stdout,
			first_stderr + second_stderr,
		)
		detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
			failure, base_course_private_values(base_course_database_urls)
		)
		raise local_stack_control.models.ControllerError(
			"concurrent Base Course installers failed: " + detail
		)
	actions = {json.loads(first_stdout)["action"], json.loads(second_stdout)["action"]}
	if actions != {"resumed", "retained"}:
		raise local_stack_control.models.ControllerError(
			"concurrent Base Course installers did not serialize to one writer"
		)
	verify_exact_baseline(stack, database)


#============================================
def verify_pre_marker_refusal(stack: e2e_live_demo_stack.DisposableStack) -> None:
	"""Prove ordinary upgrades work while first Base Course install rejects live state."""
	database = "ple_live_demo_pre_marker"
	stack.create_database(database)
	base_course_database_urls = prepare_database(stack, database)
	run_phase(stack, database, base_course_database_urls, "prepare")
	stack.psql(
		database,
		"DROP TABLE live_demo_install_state; "
		"DELETE FROM _sqlx_migrations WHERE version = 2026081808",
	)
	stack.psql(
		database,
		f"INSERT INTO course (tenant_id, course_id, title, term_start_date, "
		f"term_end_date, time_zone) VALUES ('{TENANT_ID}', '{deterministic_id('course')}', "
		"'pre-marker course', DATE '2026-01-01', DATE '2099-12-31', 'America/Chicago')",
	)
	migrate_database(stack, database)
	marker = stack.psql(
		database, "SELECT to_regclass('public.live_demo_install_state')"
	)
	if marker != "live_demo_install_state":
		raise local_stack_control.models.ControllerError(
			"ordinary upgrade did not create its lifecycle table"
		)
	base_course_database_urls = provision_base_course_database_urls(stack, database)
	prepare_argv = base_course_argv("")[:-3]
	prepare_argv.append("prepare")
	environment = phase_environment(stack, database, base_course_database_urls)
	result = stack.runner.run(prepare_argv, environment, stack.root)
	if result.ok():
		raise local_stack_control.models.ControllerError(
			"Base Course installer accepted populated unmarked application state"
		)
	state = stack.psql(
		database, "SELECT count(*) FROM live_demo_install_state WHERE singleton"
	)
	if state != "0":
		raise local_stack_control.models.ControllerError(
			"rejected Base Course install left a lifecycle marker"
		)


#============================================
def verify_retained_data(
	stack: e2e_live_demo_stack.DisposableStack,
	base_course_database_urls: tuple[str, str],
) -> None:
	"""Prove one-command retained startup preserves ordinary database and storage data."""
	database = e2e_live_demo_stack.POSTGRES_DATABASE
	course_id = deterministic_id("course")
	practice_course_id = deterministic_id("practice-course")
	stack.psql(
		database,
		f"UPDATE course SET title = 'Instructor edited Base Course' "
		f"WHERE tenant_id = '{TENANT_ID}' AND course_id = '{course_id}'; INSERT INTO course "
		"(tenant_id, course_id, title, term_start_date, term_end_date, time_zone) VALUES "
		f"('{TENANT_ID}', '{USER_CREATED_COURSE_ID}', "
		"'User-created course', DATE '2026-01-01', DATE '2099-12-31', 'America/Chicago')",
	)
	stack.put_object("student-records", "ordinary/user-created.txt", "preserve me")
	stack.compose(["restart", "postgres", "minio"])
	stack.wait_for_service("postgres")
	stack.wait_for_service("minio")
	stack.runner.calls.clear()
	preparation = local_stack_control.lifecycle.prepare_installed_base_course(
		stack.runner,
		stack.root,
		stack.disposable,
		stack.values,
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
		base_course_database_urls,
	)
	local_stack_control.lifecycle.finalize_installed_base_course(
		stack.runner,
		stack.root,
		stack.disposable,
		stack.values,
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
		preparation,
		base_course_database_urls,
	)
	if len(stack.runner.calls) != 1 or stack.runner.calls[0][:3] != (
		"cargo", "tools", "base-course"
	):
		raise local_stack_control.models.ControllerError(
			"retained Base Course startup touched storage or invoked a second writer"
		)
	require_exact_value(
		stack,
		database,
		f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
		f"AND course_id = '{course_id}'",
		"Instructor edited Base Course",
		"retained Instructor edit",
	)
	require_exact_value(
		stack,
		database,
		f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
		f"AND course_id = '{practice_course_id}'",
		"Genetics Practice Course",
		"retained practice course",
	)
	require_exact_value(
		stack,
		database,
		f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
		f"AND course_id = '{USER_CREATED_COURSE_ID}'",
		"User-created course",
		"retained user-created course",
	)
	ordinary_object = stack.read_object(
		"student-records", "ordinary/user-created.txt"
	)
	if ordinary_object != "preserve me":
		raise local_stack_control.models.ControllerError(
			"retained Base Course startup changed ordinary live storage"
		)


#============================================
def verify_regeneration(stack: e2e_live_demo_stack.DisposableStack) -> None:
	"""Prove fresh PostgreSQL and storage restore the reviewed baseline."""
	stack.clear_storage()
	expect_object_absent(
		stack, "student-records", "ordinary/user-created.txt", "ordinary retained object"
	)
	expect_object_absent(stack, RECEIPT_BUCKET, RECEIPT_KEY, "previous installation receipt")
	database = "ple_live_demo_regenerated"
	stack.create_database(database)
	base_course_database_urls = prepare_database(stack, database)
	prepared = run_phase(stack, database, base_course_database_urls, "prepare")
	local_stack_control.base_course_lifecycle.ensure_storage_receipt(
		stack.disposable.target,
		stack.runner,
		prepared,
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
	)
	completed = run_phase(
		stack,
		database,
		base_course_database_urls,
		"install",
		prepared.storage_receipt_json,
	)
	if completed.install_state != "complete":
		raise local_stack_control.models.ControllerError(
			"fresh regeneration did not complete"
		)
	generation, receipt_sha256 = verify_exact_baseline(stack, database)
	require_exact_value(
		stack,
		database,
		f"SELECT title FROM course WHERE tenant_id = '{TENANT_ID}' "
		f"AND course_id = '{USER_CREATED_COURSE_ID}'",
		"",
		"regenerated user-created course absence",
	)
	verify_generation_bound_receipt(stack, prepared, generation, receipt_sha256)


#============================================
def run_connected_lane(stack: e2e_live_demo_stack.DisposableStack) -> None:
	"""Run the connected Base Course lifecycle checks in dependency order."""
	local_stack_control.process.require_rootless_local_engine(stack.runner, stack.root)
	print("live-demo baseline E2E: preparing against isolated PostgreSQL 17")
	stack.start_service("postgres")
	version = stack.psql(e2e_live_demo_stack.POSTGRES_DATABASE, "SHOW server_version")
	if not version.startswith("17."):
		raise local_stack_control.models.ControllerError(
			f"live-demo baseline E2E requires PostgreSQL 17, got {version}"
		)
	base_course_database_urls = prepare_database(
		stack, e2e_live_demo_stack.POSTGRES_DATABASE
	)
	prepared = local_stack_control.lifecycle.prepare_installed_base_course(
		stack.runner,
		stack.root,
		stack.disposable,
		stack.values,
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
		base_course_database_urls,
	)
	if prepared is None:
		raise local_stack_control.models.ControllerError(
			"live-demo baseline E2E owner did not select Base Course installation"
		)
	if stack.running_services() != ("postgres",):
		raise local_stack_control.models.ControllerError(
			"Base Course prepare did not complete before MinIO startup"
		)
	if (
		prepared.storage_receipt_bucket != RECEIPT_BUCKET
		or prepared.storage_receipt_key != RECEIPT_KEY
	):
		raise local_stack_control.models.ControllerError(
			"prepared lifecycle returned a different receipt bucket or key"
		)

	print("live-demo baseline E2E: exercising portable MinIO inventory boundaries")
	stack.start_service("minio")
	stack.compose(["up", "--no-deps", "createbuckets"])
	verify_storage_protocol(stack, prepared)

	print("live-demo baseline E2E: installing exact generation-bound baseline")
	local_stack_control.lifecycle.finalize_installed_base_course(
		stack.runner,
		stack.root,
		stack.disposable,
		stack.values,
		local_stack_control.lifecycle.child_environment(stack.disposable.target),
		prepared,
		base_course_database_urls,
	)
	generation, receipt_sha256 = verify_exact_baseline(
		stack, e2e_live_demo_stack.POSTGRES_DATABASE
	)
	verify_generation_bound_receipt(stack, prepared, generation, receipt_sha256)

	print("live-demo baseline E2E: preserving retained edits without storage access")
	verify_retained_data(stack, base_course_database_urls)
	print("live-demo baseline E2E: repairing representative interrupted boundaries")
	verify_interrupted_boundaries(stack)
	print("live-demo baseline E2E: serializing concurrent installers")
	verify_concurrent_installers(stack)
	print("live-demo baseline E2E: refusing populated pre-marker database")
	verify_pre_marker_refusal(stack)
	print("live-demo baseline E2E: restoring baseline from fresh stores")
	verify_regeneration(stack)


#============================================
def main() -> None:
	"""Run the connected Base Course lifecycle acceptance lane."""
	stack = e2e_live_demo_stack.DisposableStack(SCRIPT_REPOSITORY_ROOT)
	test_failure: BaseException | None = None
	try:
		run_connected_lane(stack)
		print("live-demo baseline E2E: PASS")
	except BaseException as error:
		test_failure = error
		raise
	finally:
		try:
			stack.cleanup()
		except BaseException as cleanup_error:
			# ASVS 16.5.3: preserve the original test failure when cleanup also fails.
			print(
				"live-demo baseline E2E cleanup failed; retained private cleanup "
				f"directory for retry: {stack.directory} ({cleanup_error})",
				file=sys.stderr,
			)
			if test_failure is None:
				raise


if __name__ == "__main__":
	main()
