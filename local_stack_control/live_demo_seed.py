"""Stable local-stack account selectors for ordinary Live Demo product data."""

import dataclasses
import re


ACCOUNT_MINT_PLACEHOLDER = "U00000009"
ACCOUNT_ID_PATTERN = re.compile(r"^U[0-9A-HJKMNP-TV-Z]{8}$")


@dataclasses.dataclass(frozen=True)
class SeededAccount:
	"""One email-keyed Product Role mapping for deployment-gated demo entry."""

	setting: str
	email: str
	product_role: str


SEEDED_ACCOUNTS = (
	SeededAccount(
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
		"elena.martinez@live-demo.invalid",
		"instructor",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
		"mary.okafor@biology.roosevelt.edu",
		"student",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
		"jack.nguyen@biology.roosevelt.edu",
		"student",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
		"avery.thompson@biology.roosevelt.edu",
		"student",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
		"morgan.delgado@live-demo.invalid",
		"sysadmin",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_PRIYA_INSTRUCTOR_ACCOUNT_ID",
		"priya.shah@live-demo.invalid",
		"instructor",
	),
)


#============================================
def seeded_account_id_query_script() -> str:
	"""Return psql that prints minted persona setting=id lines."""
	value_rows = ", ".join(
		f"('{account.email}', '{account.setting}', '{account.product_role}')"
		for account in SEEDED_ACCOUNTS
	)
	script = (
		"psql \"$PLE_MIGRATION_DATABASE_URL\" --no-psqlrc "
		"--set=ON_ERROR_STOP=1 --quiet --tuples-only --no-align --command \""
		"BEGIN; SET LOCAL ROLE ple_private_owner; "
		"SELECT wanted.setting || '=' || matched.account_id "
		"FROM (VALUES "
		f"{value_rows}"
		") AS wanted(normalized_email, setting, product_role) "
		"JOIN LATERAL ("
		"SELECT account.account_id "
		"FROM ple_private.account AS account "
		"LEFT JOIN ple_private.account_authentication_email AS email "
		"ON email.account_id = account.account_id "
		"AND email.normalized_email = wanted.normalized_email "
		"WHERE account.product_role = wanted.product_role::ple_data.product_role "
		"AND (wanted.product_role = 'sysadmin' "
		"OR email.normalized_email = wanted.normalized_email) "
		"ORDER BY account.created_at, account.account_id "
		"LIMIT 1"
		") AS matched ON true; "
		"COMMIT;\""
	)
	return script


#============================================
def parse_seeded_account_id_report(stdout: str) -> dict[str, str]:
	"""Parse minted persona IDs from the installation query."""
	wanted = {account.setting: account for account in SEEDED_ACCOUNTS}
	found: dict[str, str] = {}
	for raw_line in stdout.splitlines():
		line = raw_line.strip()
		if line == "":
			continue
		if "=" not in line:
			raise ValueError("minted Live Demo Account report is not setting=id")
		setting, account_id = line.split("=", 1)
		if setting not in wanted:
			raise ValueError(f"unexpected Live Demo Account setting {setting}")
		if ACCOUNT_ID_PATTERN.fullmatch(account_id) is None:
			raise ValueError(f"{setting} is not a canonical Account ID")
		if account_id == ACCOUNT_MINT_PLACEHOLDER:
			raise ValueError(f"{setting} kept the mint placeholder")
		if setting in found:
			raise ValueError(f"{setting} was reported more than once")
		found[setting] = account_id
	missing = [setting for setting in wanted if setting not in found]
	if missing:
		raise ValueError("minted Live Demo Account report is incomplete")
	return found
