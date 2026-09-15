#!/usr/bin/env python3
"""Stop an unsafe Question-ID schema reinitialization before it starts.

Run this deployment-preparation check through the migration connection before
``cargo tools database initialize``.  It deliberately has no initialize,
migration, DDL, delete, or rewrite path.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys


PERMIT_RESULT = "PERMIT: published Question relation is missing or empty; initialize may proceed."
STOP_RESULT = "STOP: published Question rows exist; do not reinitialize. Escalate for a human-approved cutover."
UNVERIFIED_STOP_RESULT = (
	"STOP: published Question rows could not be verified; do not reinitialize. "
	"Escalate for a human-approved cutover."
)
PSQL_QUERY = """
DO $preflight$
DECLARE
    result text;
BEGIN
    IF to_regclass('ple_data.published_question') IS NULL THEN
        result := 'permit';
    ELSE
        EXECUTE 'SELECT CASE WHEN count(*) = 0 THEN ''permit'' ELSE ''stop'' END '
            'FROM ple_data.published_question' INTO result;
    END IF;
    RAISE WARNING 'C843_RESULT:%', result;
END
$preflight$;
"""


def parse_arguments(arguments: list[str]) -> argparse.Namespace:
	parser = argparse.ArgumentParser(
		description="inspect published Questions before a destructive schema reinitialization"
	)
	parser.add_argument(
		"--migration-database",
		action="store_true",
		help="use PLE_MIGRATION_DATABASE_URL and inspect only the existing database",
	)
	parsed = parser.parse_args(arguments)
	if not parsed.migration_database:
		parser.error("--migration-database is required")
	return parsed


def migration_database_url(environment: dict[str, str]) -> str:
	value = environment.get("PLE_MIGRATION_DATABASE_URL", "")
	if not value.startswith(("postgres://", "postgresql://")) or any(
		character in value for character in "\r\n\x00"
	):
		raise ValueError("migration database URL is unavailable")
	return value


def inspect_published_questions(migration_url: str) -> str:
	"""Return the closed database answer, or ``None`` without diagnostics.

	``PGDATABASE`` is intentionally the only connection channel.  This keeps
	the migration URL out of argv and leaves psql no caller-controlled SQL.
	"""
	environment = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "PGDATABASE": migration_url}
	try:
		completed = subprocess.run(
			[
				"psql",
				"-X",
				"--no-psqlrc",
				"--set=ON_ERROR_STOP=1",
				"--tuples-only",
				"--no-align",
				"--quiet",
				"--command",
				PSQL_QUERY,
			],
			env=environment,
			capture_output=True,
			text=True,
			check=False,
		)
	except OSError:
		return "unverified"
	if completed.returncode != 0:
		return "unverified"
	result_text = completed.stdout + completed.stderr
	for result in ("permit", "stop"):
		if f"C843_RESULT:{result}" in result_text:
			return result
	return "unverified"


def main(arguments: list[str]) -> int:
	parse_arguments(arguments)
	try:
		migration_url = migration_database_url(dict(os.environ))
	except ValueError:
		print(UNVERIFIED_STOP_RESULT)
		return 1
	answer = inspect_published_questions(migration_url)
	if answer == "permit":
		print(PERMIT_RESULT)
		return 0
	if answer == "unverified":
		print(UNVERIFIED_STOP_RESULT)
		return 1
	print(STOP_RESULT)
	return 1


if __name__ == "__main__":
	sys.exit(main(sys.argv[1:]))
