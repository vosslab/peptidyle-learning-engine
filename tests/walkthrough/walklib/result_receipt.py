"""Create the private, redacted result receipt for one UI walkthrough run."""

import json
import os
import pathlib
import secrets

import tests.walkthrough.walklib.models as models


RunnerError = models.RunnerError


#============================================
def ensure_private_report_directory(
	repository_root: pathlib.Path,
	report_directory: pathlib.Path,
	report_path: pathlib.Path,
) -> None:
	"""Create and validate the runner-owned report location without following symlinks."""
	report_root = repository_root / "test-results"
	for path, description, private in (
		(report_root, "test-results", False),
		(report_directory, "ui_walkthrough report directory", True),
	):
		if path.is_symlink():
			raise RunnerError("walkthrough report path must not contain a symlink")
		if path.exists():
			if not path.is_dir():
				raise RunnerError(f"{description} must be a directory")
		else:
			try:
				path.mkdir()
			except FileExistsError:
				pass
			if path.is_symlink() or not path.is_dir():
				raise RunnerError("walkthrough report path must not contain a symlink")
		if private:
			path.chmod(0o700)
	if report_path.is_symlink():
		raise RunnerError("walkthrough report path must not contain a symlink")
	if report_path.exists() and not report_path.is_file():
		raise RunnerError("walkthrough report path must be a regular file")


#============================================
def build_payload(
	status: str,
	master_seed: int,
	stage: str,
	student_repeat_only: bool,
	visible_outcomes: dict[str, object] | None,
	arrangements: list[dict[str, str]] | None,
	j1_failure_checkpoint: str | None,
	j2_failure_checkpoint: str | None,
	instructor_setup_failure_checkpoint: str | None,
) -> dict[str, object]:
	"""Return the small public receipt payload while excluding private child inputs."""
	payload: dict[str, object] = {
		"status": status,
		"masterSeed": master_seed,
		"stage": stage,
	}
	if status == "PASS" and visible_outcomes is not None:
		payload = visible_outcomes
	elif status == "PASS" and student_repeat_only:
		payload["mode"] = "student_repeat_only"
	elif status == "FAIL" and stage == "playwright_j1":
		payload["j1Checkpoint"] = j1_failure_checkpoint or "unavailable"
	elif status == "FAIL" and stage == "playwright_j2":
		payload["j2Checkpoint"] = j2_failure_checkpoint or "unavailable"
	elif status == "FAIL" and stage == "playwright_instructor_setup":
		payload["instructorCheckpoint"] = instructor_setup_failure_checkpoint or "unavailable"
	elif arrangements is not None:
		payload["arrangements"] = arrangements
	return payload


#============================================
def write_private_receipt(
	repository_root: pathlib.Path,
	report_directory: pathlib.Path,
	report_path: pathlib.Path,
	report_basename: str,
	payload: dict[str, object],
) -> None:
	"""Atomically write one permission-restricted receipt after its location is revalidated."""
	ensure_private_report_directory(repository_root, report_directory, report_path)
	directory_flags = os.O_RDONLY | os.O_DIRECTORY
	if hasattr(os, "O_NOFOLLOW"):
		directory_flags |= os.O_NOFOLLOW
	directory_descriptor = os.open(report_directory, directory_flags)
	temporary_name = f".ui_walkthrough_report.{secrets.token_hex(16)}"
	file_descriptor = -1
	try:
		file_descriptor = os.open(
			temporary_name,
			os.O_WRONLY | os.O_CREAT | os.O_EXCL,
			0o600,
			dir_fd=directory_descriptor,
		)
		os.fchmod(file_descriptor, 0o600)
		with os.fdopen(file_descriptor, "w", encoding="ascii") as report_file:
			file_descriptor = -1
			json.dump(payload, report_file, separators=(",", ":"))
			report_file.write("\n")
		os.replace(
			temporary_name,
			report_basename,
			src_dir_fd=directory_descriptor,
			dst_dir_fd=directory_descriptor,
		)
		os.chmod(report_basename, 0o600, dir_fd=directory_descriptor)
		ensure_private_report_directory(repository_root, report_directory, report_path)
	finally:
		if file_descriptor >= 0:
			os.close(file_descriptor)
		try:
			os.unlink(temporary_name, dir_fd=directory_descriptor)
		except FileNotFoundError:
			pass
		os.close(directory_descriptor)
