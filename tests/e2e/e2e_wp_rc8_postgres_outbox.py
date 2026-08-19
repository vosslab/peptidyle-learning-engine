#!/usr/bin/env python3
"""Run the WP-RC8 invitation-delivery PostgreSQL authority oracle."""

import hashlib
import os
import pathlib
import secrets
import socket
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

import local_stack_control.models


POSTGRES_USER = "ple_wp_rc8_postgres_outbox"
POSTGRES_DATABASE = "ple_wp_rc8_postgres_outbox"
LIVE_TEST = "postgres_wp_rc8_invitation_delivery_authority_and_outbox"
EMAIL_CONFLICT_TEST = "postgres_email_change_conflict_rolls_back_without_revoking_prior_sessions"
ENROLLMENT_DELIVERY_TEST = "postgres_enrollment_capability_is_locked_unique_and_role_separated"
PASSWORDLESS_CHALLENGE_TEST = "postgres_passwordless_challenge_consumption_is_binding_atomic"
COURSE_MEMBER_UPSERT_TEST = "postgres_course_member_upsert_is_atomic_idempotent_and_tenant_scoped"
SAFE_ENVIRONMENT = ("PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "TMPDIR", "LANG")


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one private runtime file without a permissive creation window."""
	descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(descriptor, "wb") as output:
		output.write(content.encode("ascii") if isinstance(content, str) else content)


#============================================
def available_port() -> int:
	"""Return a currently available loopback port for this private project."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		return int(listener.getsockname()[1])


#============================================
def adapter(action: str, manifest: pathlib.Path, *arguments: str) -> list[str]:
	"""Form one typed lifecycle call; this runner does not invoke Compose directly."""
	return [sys.executable, "-m", "local_stack_control._consumer_cli", action, "--manifest", str(manifest), *arguments]


#============================================
def run(argv: list[str], environment: dict[str, str] | None = None) -> str:
	"""Run one bounded step and surface a redacted concise failure receipt."""
	completed = subprocess.run(argv, cwd=ROOT, env=environment, text=True, capture_output=True, check=False)
	if completed.returncode != 0:
		receipt = (completed.stdout + completed.stderr).strip()
		raise local_stack_control.models.ControllerError(
			"WP-RC8 PostgreSQL outbox step failed: " + " ".join(argv) + "\n" + receipt[-12000:]
		)
	return completed.stdout + completed.stderr


#============================================
def target(directory: pathlib.Path) -> tuple[pathlib.Path, str, int]:
	"""Create owner-bound private capability evidence and PostgreSQL credentials."""
	project = "ple_wp_rc8_postgres_outbox_" + secrets.token_hex(12)
	password = secrets.token_urlsafe(30)
	port = available_port()
	capability = secrets.token_bytes(32)
	capability_path = directory / "disposable.capability"
	private_file(capability_path, capability)
	environment_path = directory / "env.local"
	private_file(environment_path, (
		f"POSTGRES_USER={POSTGRES_USER}\nPOSTGRES_PASSWORD={password}\nPOSTGRES_DB={POSTGRES_DATABASE}\n"
		f"PLE_POSTGRES_HOST_PORT={port}\nPLE_DISPOSABLE_CAPABILITY_SHA256={hashlib.sha256(capability).hexdigest()}\n"
	))
	manifest = directory / "disposable.manifest"
	private_file(manifest, (
		"OWNER=wp-rc8-postgres-outbox\n"
		f"PROJECT={project}\nENV_FILE={environment_path}\nCAPABILITY_FILE={capability_path}\n"
	))
	return manifest, password, port


#============================================
def cargo_environment(database_url: str) -> dict[str, str]:
	"""Provide Cargo its local toolchain inputs and the one disposable DB capability."""
	environment = {name: value for name, value in os.environ.items() if name in SAFE_ENVIRONMENT or name.startswith("LC_")}
	environment["PLE_TEST_DATABASE_URL"] = database_url
	return environment


#============================================
def main() -> None:
	"""Run fresh PostgreSQL authority, delivery, account, and roster oracles then clean up."""
	directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-wp-rc8-postgres-outbox-"))
	os.chmod(directory, 0o700)
	manifest, password, port = target(directory)
	keep = os.environ.get("PLE_E2E_KEEP") == "1"
	started = False
	try:
		started = True
		print("WP-RC8 PostgreSQL outbox: starting fresh private PostgreSQL")
		run(adapter("compose", manifest, "up", "-d", "--wait", "postgres"))
		database_url = f"postgres://{POSTGRES_USER}:{password}@127.0.0.1:{port}/{POSTGRES_DATABASE}"
		print("WP-RC8 PostgreSQL outbox: applying migrations and exercising worker authority")
		output = run([
			"cargo", "test", "-p", "learning-data-access", "--features", "postgres",
			"--test", "postgres_wp_rc8_outbox_live", "--", "--ignored", "--exact", LIVE_TEST,
		], cargo_environment(database_url))
		if "0 passed" in output:
			raise local_stack_control.models.ControllerError("WP-RC8 PostgreSQL outbox selected no ignored Rust test")
		email_output = run([
			"cargo", "test", "-p", "learning-data-access", "--features", "postgres",
			"--test", "postgres_enrollment_live", "--", "--ignored", "--exact", EMAIL_CONFLICT_TEST,
		], cargo_environment(database_url))
		if "0 passed" in email_output:
			raise local_stack_control.models.ControllerError("WP-RC8 PostgreSQL selected no email rollback test")
		enrollment_output = run([
			"cargo", "test", "-p", "learning-data-access", "--features", "postgres",
			"--test", "postgres_enrollment_live", "--", "--ignored", "--exact", ENROLLMENT_DELIVERY_TEST,
		], cargo_environment(database_url))
		if "0 passed" in enrollment_output:
			raise local_stack_control.models.ControllerError("WP-RC8 PostgreSQL selected no enrollment delivery test")
		passwordless_output = run([
			"cargo", "test", "-p", "learning-data-access", "--features", "postgres",
			"--test", "postgres_enrollment_live", "--", "--ignored", "--exact", PASSWORDLESS_CHALLENGE_TEST,
		], cargo_environment(database_url))
		if "0 passed" in passwordless_output:
			raise local_stack_control.models.ControllerError(
				"PostgreSQL account oracle selected no passwordless challenge consumption test"
			)
		course_member_output = run([
			"cargo", "test", "-p", "learning-data-access", "--features", "postgres",
			"--test", "postgres_enrollment_live", "--", "--ignored", "--exact", COURSE_MEMBER_UPSERT_TEST,
		], cargo_environment(database_url))
		if "0 passed" in course_member_output:
			raise local_stack_control.models.ControllerError(
				"PostgreSQL roster oracle selected no course member upsert test"
			)
		print("WP-RC8 PostgreSQL outbox: completed migration, delivery, account, and roster authority oracles")
	finally:
		if started and not keep:
			run(adapter("cleanup", manifest))
		elif keep:
			print(f"WP-RC8 PostgreSQL outbox retained private project manifest: {manifest}")


if __name__ == "__main__":
	main()
