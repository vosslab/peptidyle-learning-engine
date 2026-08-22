"""Private disposable PostgreSQL and MinIO support for the live-demo E2E lane."""

import base64
import hashlib
import os
import pathlib
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time


SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.consumer
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


POSTGRES_USER = "ple_live_demo_baseline"
POSTGRES_DATABASE = "ple_live_demo_baseline"
MINIO_USER = "ple-live-demo-baseline"
BUCKETS = ("private-content", "public-assets", "student-records", "temp-processing")


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Delegate real subprocesses while retaining their command boundaries."""

	def __init__(self) -> None:
		self.delegate = local_stack_control.process.SubprocessRunner()
		self.calls: list[tuple[str, ...]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Run and record one captured command."""
		self.calls.append(tuple(argv))
		result = self.delegate.run(argv, environment, cwd, stdin)
		return result

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Run and record one streamed command."""
		self.calls.append(tuple(argv))
		result = self.delegate.stream(argv, environment, cwd)
		return result


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Write one new current-user-only E2E file."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		if isinstance(content, str):
			output.write(content.encode("ascii"))
		else:
			output.write(content)


#============================================
def available_ports() -> tuple[int, int]:
	"""Ask the kernel for two distinct private loopback ports."""
	sockets: list[socket.socket] = []
	ports: list[int] = []
	try:
		for _ in range(2):
			listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
			listener.bind(("127.0.0.1", 0))
			sockets.append(listener)
			ports.append(listener.getsockname()[1])
	finally:
		for listener in sockets:
			listener.close()
	result = ports[0], ports[1]
	return result


#============================================
def require_result(
	result: local_stack_control.models.CommandResult,
	operation: str,
) -> str:
	"""Return stdout from a successful real command."""
	if not result.ok():
		detail = result.stderr.strip() or result.stdout.strip()
		raise local_stack_control.models.ControllerError(
			f"live-demo baseline E2E {operation} failed: {detail}"
		)
	return result.stdout.strip()


class DisposableStack:
	"""Own one capability-bound PostgreSQL 17 and MinIO Compose project."""

	def __init__(self, root: pathlib.Path) -> None:
		self.root = root
		self.directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-live-demo-baseline-"))
		os.chmod(self.directory, 0o700)
		self.postgres_password = secrets.token_urlsafe(24)
		self.minio_password = secrets.token_urlsafe(24)
		self.postgres_port, self.minio_port = available_ports()
		self.question_secret_path = self.directory / "question-id-secret"
		question_secret = base64.urlsafe_b64encode(secrets.token_bytes(32)).rstrip(b"=")
		private_file(self.question_secret_path, question_secret)
		self.project, self.manifest_path = self._write_target()
		self.runner = RecordingRunner()
		manifest = local_stack_control.consumer.load_manifest(root, self.manifest_path)
		self.disposable = local_stack_control.consumer.disposable_target(
			self.runner, root, manifest
		)
		self.values = local_stack_control.env_file.env_settings(
			self.disposable.target.env_file
		)
		self.started = False

	#============================================
	def _write_target(self) -> tuple[str, pathlib.Path]:
		"""Write the private typed disposable manifest and environment."""
		project = "ple_live_demo_baseline_" + secrets.token_hex(12)
		capability_path = self.directory / "disposable.capability"
		capability = secrets.token_bytes(32)
		private_file(capability_path, capability)
		capability_digest = hashlib.sha256(capability).hexdigest()
		env_path = self.directory / "env.local"
		env_content = (
			f"POSTGRES_USER={POSTGRES_USER}\n"
			f"POSTGRES_PASSWORD={self.postgres_password}\n"
			f"POSTGRES_DB={POSTGRES_DATABASE}\n"
			f"PLE_POSTGRES_HOST_PORT={self.postgres_port}\n"
			f"MINIO_ROOT_USER={MINIO_USER}\n"
			f"MINIO_ROOT_PASSWORD={self.minio_password}\n"
			f"PLE_MINIO_API_HOST_PORT={self.minio_port}\n"
			f"PLE_QUESTION_ID_SECRET_HOST_FILE={self.question_secret_path}\n"
			f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
		)
		private_file(env_path, env_content)
		manifest_path = self.directory / "disposable.manifest"
		manifest_content = (
			"OWNER=live-demo-baseline\n"
			f"PROJECT={project}\n"
			f"ENV_FILE={env_path}\n"
			f"CAPABILITY_FILE={capability_path}\n"
		)
		private_file(manifest_path, manifest_content)
		return project, manifest_path

	#============================================
	def compose(self, arguments: list[str]) -> str:
		"""Run one capability-bound Compose command."""
		argv, environment = local_stack_control.consumer.compose_command(
			self.disposable, arguments
		)
		result = self.runner.run(argv, environment, self.root)
		output = require_result(result, "Compose command")
		return output

	#============================================
	def running_services(self) -> tuple[str, ...]:
		"""Return capability-verified running services for only this project."""
		snapshot = local_stack_control.consumer.require_current_resource_capability(
			self.runner, self.disposable
		)
		services = sorted(
			item.service
			for item in snapshot.containers
			if item.running and item.service is not None
		)
		return tuple(services)

	#============================================
	def start_service(self, service: str) -> None:
		"""Start and wait for one disposable stateful service."""
		self.started = True
		self.compose(["up", "-d", service])
		self.wait_for_service(service)

	#============================================
	def wait_for_service(self, service: str) -> None:
		"""Wait for one real service through its project-scoped container."""
		for _ in range(40):
			if service == "postgres":
				arguments = [
					"exec", "-T", "postgres", "pg_isready", "-U", POSTGRES_USER,
					"-d", POSTGRES_DATABASE,
				]
			elif service == "minio":
				arguments = ["exec", "-T", "minio", "mc", "ready", "local"]
			else:
				raise local_stack_control.models.ControllerError(
					"unsupported live-demo E2E service"
				)
			argv, environment = local_stack_control.consumer.compose_command(
				self.disposable, arguments
			)
			result = self.runner.run(argv, environment, self.root)
			if result.ok():
				return
			time.sleep(1)
		raise local_stack_control.models.ControllerError(
			f"live-demo baseline E2E disposable {service} did not become ready"
		)

	#============================================
	def psql(self, database: str, sql: str) -> str:
		"""Run one SQL statement inside the isolated PostgreSQL container."""
		arguments = [
			"exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1",
			"-U", POSTGRES_USER, "-d", database, "-Atc", sql,
		]
		result = self.compose(arguments)
		return result

	#============================================
	def create_database(self, database: str) -> None:
		"""Create one validated database name inside the disposable cluster."""
		if not database.replace("_", "").isalnum() or database.lower() != database:
			raise local_stack_control.models.ControllerError(
				"invalid disposable database name"
			)
		self.psql("postgres", f'CREATE DATABASE "{database}"')

	#============================================
	def minio_shell(self, script: str, arguments: list[str] | None = None) -> str:
		"""Run one bounded MinIO Client script inside the disposable project."""
		argv = [
			"run", "--rm", "--no-deps", "-T", "--entrypoint", "/bin/sh",
			"createbuckets", "-ec", script, "ple-live-demo-e2e",
		]
		if arguments is not None:
			argv.extend(arguments)
		result = self.compose(argv)
		return result

	#============================================
	def put_object(self, bucket: str, key: str, content: str) -> None:
		"""Write one object through the real MinIO Client boundary."""
		script = (
			'mc alias set local http://minio:9000 "$MINIO_ROOT_USER" '
			'"$MINIO_ROOT_PASSWORD" >/dev/null; object_file=; '
			'trap \'rm -f "$object_file"\' EXIT; '
			'object_file=$(mktemp /tmp/live-demo-object.XXXXXX); '
			'chmod 600 "$object_file"; printf "%s" "$3" >"$object_file"; '
			'mc cp --disable-multipart "$object_file" "local/$1/$2" >/dev/null'
		)
		self.minio_shell(script, [bucket, key, content])

	#============================================
	def remove_object(self, bucket: str, key: str) -> None:
		"""Remove one exact object from this disposable test store."""
		script = (
			'mc alias set local http://minio:9000 "$MINIO_ROOT_USER" '
			'"$MINIO_ROOT_PASSWORD" >/dev/null; '
			'mc rm --force "local/$1/$2" >/dev/null'
		)
		self.minio_shell(script, [bucket, key])

	#============================================
	def remove_empty_bucket(self, bucket: str) -> None:
		"""Remove one empty bucket to exercise MinIO inventory failure."""
		if bucket not in BUCKETS:
			raise local_stack_control.models.ControllerError(
				"unsupported live-demo E2E bucket"
			)
		script = (
			'mc alias set local http://minio:9000 "$MINIO_ROOT_USER" '
			'"$MINIO_ROOT_PASSWORD" >/dev/null; mc rb "local/$1" >/dev/null'
		)
		self.minio_shell(script, [bucket])

	#============================================
	def create_bucket(self, bucket: str) -> None:
		"""Restore one required empty bucket after a failure probe."""
		if bucket not in BUCKETS:
			raise local_stack_control.models.ControllerError(
				"unsupported live-demo E2E bucket"
			)
		script = (
			'mc alias set local http://minio:9000 "$MINIO_ROOT_USER" '
			'"$MINIO_ROOT_PASSWORD" >/dev/null; mc mb "local/$1" >/dev/null'
		)
		self.minio_shell(script, [bucket])

	#============================================
	def read_object(self, bucket: str, key: str) -> str:
		"""Read one exact object from this disposable test store."""
		script = (
			'mc alias set local http://minio:9000 "$MINIO_ROOT_USER" '
			'"$MINIO_ROOT_PASSWORD" >/dev/null; mc cat "local/$1/$2"'
		)
		result = self.minio_shell(script, [bucket, key])
		return result

	#============================================
	def clear_storage(self) -> None:
		"""Empty only the four buckets owned by this disposable project."""
		script = (
			'mc alias set local http://minio:9000 "$MINIO_ROOT_USER" '
			'"$MINIO_ROOT_PASSWORD" >/dev/null; '
			'for bucket in private-content public-assets student-records temp-processing; do '
			'mc rm --recursive --force "local/$bucket" >/dev/null; done'
		)
		self.minio_shell(script)

	#============================================
	def cleanup(self) -> None:
		"""Remove only this capability-bound project's containers and volumes."""
		if not self.started:
			shutil.rmtree(self.directory)
			return
		completed = subprocess.run(
			[
				sys.executable, "-m", "local_stack_control._consumer_cli",
				"cleanup", "--manifest", str(self.manifest_path),
			],
			check=False,
			cwd=self.root,
		)
		if completed.returncode != 0:
			# ASVS 5.3.2: retain trusted private state for an exact cleanup retry.
			raise local_stack_control.models.ControllerError(
				"live-demo baseline E2E typed cleanup failed "
				f"(exit {completed.returncode}); retained private cleanup directory: "
				f"{self.directory}"
			)
		shutil.rmtree(self.directory)
