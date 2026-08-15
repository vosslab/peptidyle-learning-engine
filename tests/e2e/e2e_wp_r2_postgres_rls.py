#!/usr/bin/env python3
"""Run the WP-R2 PostgreSQL and RLS oracle in one private disposable project."""

import hashlib
import os
import pathlib
import secrets
import socket
import subprocess
import sys
import tempfile


SCRIPT_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_ROOT))

import local_stack_control.models


POSTGRES_USER = "ple_wp_r2_postgres_rls"
POSTGRES_DATABASE = "ple_wp_r2_postgres_rls"
LIVE_TEST = "postgres_wp_r2_persistence_rls_and_no_drift"
CARGO_RUNTIME_ENVIRONMENT = (
	"PATH",
	"HOME",
	"CARGO_HOME",
	"RUSTUP_HOME",
	"TMPDIR",
	"LANG",
)


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one private runtime file without a permissive creation window."""
	descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(descriptor, "wb") as output:
		output.write(content.encode("ascii") if isinstance(content, str) else content)


#============================================
def available_port() -> int:
	"""Reserve an available loopback port long enough to select this private stack."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		return int(listener.getsockname()[1])


#============================================
def adapter_argv(action: str, manifest: pathlib.Path, *arguments: str) -> list[str]:
	"""Form one typed controller command; the runner never addresses Compose directly."""
	return [
		sys.executable,
		"-m",
		"local_stack_control._consumer_cli",
		action,
		"--manifest",
		str(manifest),
		*arguments,
	]


#============================================
def run(argv: list[str], root: pathlib.Path, environment: dict[str, str] | None = None) -> str:
	"""Run one boundary command and retain a concise, redacted failure receipt."""
	completed = subprocess.run(
		argv,
		cwd=root,
		env=environment,
		text=True,
		capture_output=True,
		check=False,
	)
	if completed.returncode != 0:
		receipt = (completed.stdout + completed.stderr).strip()
		raise local_stack_control.models.ControllerError(
			"WP-R2 PostgreSQL/RLS step failed: " + " ".join(argv) + "\n" + receipt[-12000:]
		)
	return completed.stdout + completed.stderr


#============================================
def write_target(directory: pathlib.Path) -> tuple[pathlib.Path, str, int]:
	"""Create the owner-bound manifest, capability, and Compose environment."""
	project = "ple_wp_r2_postgres_rls_" + secrets.token_hex(12)
	password = secrets.token_urlsafe(30)
	port = available_port()
	capability = secrets.token_bytes(32)
	capability_path = directory / "disposable.capability"
	private_file(capability_path, capability)
	environment_path = directory / "env.local"
	private_file(
		environment_path,
		(
			f"POSTGRES_USER={POSTGRES_USER}\n"
			f"POSTGRES_PASSWORD={password}\n"
			f"POSTGRES_DB={POSTGRES_DATABASE}\n"
			f"PLE_POSTGRES_HOST_PORT={port}\n"
			f"PLE_DISPOSABLE_CAPABILITY_SHA256={hashlib.sha256(capability).hexdigest()}\n"
		),
	)
	manifest = directory / "disposable.manifest"
	private_file(
		manifest,
		(
			"OWNER=wp-r2-postgres-rls\n"
			f"PROJECT={project}\n"
			f"ENV_FILE={environment_path}\n"
			f"CAPABILITY_FILE={capability_path}\n"
		),
	)
	return manifest, password, port


#============================================
def rust_test_environment(database_url: str) -> dict[str, str]:
	"""Provide Cargo only its local toolchain inputs and the one database capability."""
	environment = {
		name: value
		for name, value in os.environ.items()
		if name in CARGO_RUNTIME_ENVIRONMENT or name.startswith("LC_")
	}
	environment["PLE_TEST_DATABASE_URL"] = database_url
	return environment


#============================================
def main() -> None:
	"""Start a fresh database, run the named live oracle, then clean exactly that project."""
	root = SCRIPT_ROOT
	directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-wp-r2-postgres-rls-"))
	os.chmod(directory, 0o700)
	manifest, password, port = write_target(directory)
	keep = os.environ.get("PLE_E2E_KEEP") == "1"
	started = False
	try:
		print("WP-R2 PostgreSQL/RLS: starting private PostgreSQL with Compose health readiness")
		started = True
		run(adapter_argv("compose", manifest, "up", "-d", "--wait", "postgres"), root)
		database_url = f"postgres://{POSTGRES_USER}:{password}@127.0.0.1:{port}/{POSTGRES_DATABASE}"
		environment = rust_test_environment(database_url)
		print("WP-R2 PostgreSQL/RLS: applying embedded migrations and exercising restricted-role behavior")
		output = run(
			[
				"cargo",
				"test",
				"-p",
				"learning-data-access",
				"--features",
				"postgres",
				"--test",
				"postgres_wp_r2_live",
				"--",
				"--ignored",
				"--exact",
				LIVE_TEST,
			],
			root,
			environment,
		)
		if "0 passed" in output:
			raise local_stack_control.models.ControllerError(
				"WP-R2 PostgreSQL/RLS selected no ignored Rust test"
			)
		print("WP-R2 PostgreSQL/RLS: completed fresh migration, RLS, CAS, and no-drift oracle")
	finally:
		if started and not keep:
			try:
				run(adapter_argv("cleanup", manifest), root)
			except local_stack_control.models.ControllerError as error:
				print(f"WP-R2 PostgreSQL/RLS cleanup receipt: {error}", file=sys.stderr)
				raise
		elif keep:
			print(f"WP-R2 PostgreSQL/RLS retained private project manifest: {manifest}")


if __name__ == "__main__":
	main()
