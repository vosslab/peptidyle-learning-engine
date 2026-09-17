"""Stable local-stack account selectors for ordinary Live Demo product data."""

import dataclasses


@dataclasses.dataclass(frozen=True)
class SeededAccount:
	"""One fixed Product Role mapping for deployment-gated demo entry."""

	setting: str
	account_id: str
	product_role: str


SEEDED_ACCOUNTS = (
	SeededAccount("PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID", "00000000-0000-0000-0000-000000000101", "instructor"),
	SeededAccount("PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID", "00000000-0000-0000-0000-000000000102", "student"),
	SeededAccount("PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID", "00000000-0000-0000-0000-000000000103", "student"),
	SeededAccount("PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID", "00000000-0000-0000-0000-000000000104", "student"),
	SeededAccount("PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID", "00000000-0000-0000-0000-000000000105", "sysadmin"),
	SeededAccount("PLE_LIVE_DEMO_PRIYA_INSTRUCTOR_ACCOUNT_ID", "00000000-0000-0000-0000-000000000107", "instructor"),
)

SEEDED_STUDENT_AUTHENTICATION_EMAILS = (
	("00000000-0000-0000-0000-000000000102", "mary.okafor@live-demo.invalid"),
	("00000000-0000-0000-0000-000000000103", "jack.nguyen@live-demo.invalid"),
	("00000000-0000-0000-0000-000000000104", "avery.thompson@live-demo.invalid"),
)


#============================================
def bundled_without_demo_oracle_script(publisher_account_id: str, demo_course_id: str) -> str:
	"""Build the fixed shipped-content versus optional-demo absence oracle."""
	# The migrator image alone carries psql. Two fixed read-only queries use its
	# existing capabilities: the migration role proves the Demo-only private
	# roots are absent, and the API role proves the public reusable Blueprint is
	# available while the fixed Demo Course is absent. Neither exposes a general
	# SQL interface or broadens an application role.
	script = (
		"private_result=$(psql \"$PLE_MIGRATION_DATABASE_URL\" --no-psqlrc "
		"--set=ON_ERROR_STOP=1 --quiet --tuples-only --no-align --command \""
		"BEGIN; SET LOCAL ROLE ple_private_owner; "
		"SELECT CASE WHEN "
		"NOT EXISTS (SELECT 1 FROM ple_private.account WHERE account_id IN "
		"('00000000-0000-0000-0000-000000000101'::uuid, "
		"'00000000-0000-0000-0000-000000000102'::uuid, "
		"'00000000-0000-0000-0000-000000000103'::uuid, "
		"'00000000-0000-0000-0000-000000000104'::uuid, "
		"'00000000-0000-0000-0000-000000000105'::uuid, "
		"'00000000-0000-0000-0000-000000000107'::uuid)) "
		"AND NOT EXISTS (SELECT 1 FROM ple_private.account_authentication_email "
		"WHERE account_id IN ('00000000-0000-0000-0000-000000000101'::uuid, "
		"'00000000-0000-0000-0000-000000000102'::uuid, "
		"'00000000-0000-0000-0000-000000000103'::uuid, "
		"'00000000-0000-0000-0000-000000000104'::uuid, "
		"'00000000-0000-0000-0000-000000000107'::uuid)) "
		"AND NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace "
		"WHERE workspace_id IN ('00000000-0000-0000-0000-000000000201'::uuid, "
		"'00000000-0000-0000-0000-000000000206'::uuid)) "
		"THEN 'demo_roots_absent' ELSE 'invalid' END; COMMIT;\")\n"
		"api_result=$(psql \"$DATABASE_URL\" --no-psqlrc --set=ON_ERROR_STOP=1 "
		"--quiet --tuples-only --no-align --command \""
		"BEGIN; SET LOCAL ROLE ple_app; "
		f"SET LOCAL ple.session_account_id = '{publisher_account_id}'; "
		"SELECT CASE WHEN "
		"EXISTS (SELECT 1 FROM ple_api.list_blueprint_courses(false, true, 'Genetics', "
		"NULL::text, NULL::text, 101) "
		"WHERE short_name = 'Genetics' AND long_name = 'Fall Genetics' "
		"AND availability = 'public' AND is_owner) "
		f"AND NOT EXISTS (SELECT 1 FROM ple_api.read_course_theme('{demo_course_id}'::uuid)) "
		"THEN 'bundled_without_demo' ELSE 'invalid' END; COMMIT;\")\n"
		"[ \"$private_result\" = demo_roots_absent ] && "
		"[ \"$api_result\" = bundled_without_demo ] && "
		"printf '%s\\n' \"$private_result/$api_result\""
	)
	return script
