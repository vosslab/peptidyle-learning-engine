"""Checked private receipt storage for the Developer Browser Suite."""

from __future__ import annotations

import os
import pathlib
import stat

import local_stack_control.browser_suite_lease
import local_stack_control.models


CONTROL_NAME = "developer-control.json"
LAUNCH_NAME = "developer-launch.json"
RESULT_NAME = "developer-result.json"
SOCKET_NAME = "developer-control.sock"
MAXIMUM_CONTROL_BYTES = 1024


class DeveloperBrowserSuiteError(local_stack_control.models.ControllerError):
	"""A concise fixed-owner developer lifecycle failure."""


#============================================
def _checked_root_descriptor(repository_root: pathlib.Path) -> int:
	"""Open the immutable private root through the shared checked-lease authority."""
	try:
		descriptor, _identity = local_stack_control.browser_suite_lease._open_checked_directory(
			repository_root / local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY, 0o700
		)
	except local_stack_control.browser_suite_lease.BrowserSuiteError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
	return descriptor


#============================================
def _require_control_name(name: str) -> None:
	"""Keep all developer control paths fixed below the checked private root."""
	if name not in (CONTROL_NAME, LAUNCH_NAME, RESULT_NAME, SOCKET_NAME):
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable")


#============================================
def _write_private_file(root_descriptor: int, name: str, content: bytes) -> None:
	"""Atomically publish a bounded mode-0600 control receipt (ASVS 5.3.2)."""
	_require_control_name(name)
	if len(content) > MAXIMUM_CONTROL_BYTES:
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	temporary = "." + name + ".new"
	try:
		file_descriptor = os.open(
			temporary,
			os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
			0o600,
			dir_fd=root_descriptor,
		)
		with os.fdopen(file_descriptor, "wb") as output:
			output.write(content)
			output.flush()
			os.fsync(output.fileno())
		os.replace(temporary, name, src_dir_fd=root_descriptor, dst_dir_fd=root_descriptor)
		os.fsync(root_descriptor)
	except OSError as error:
		try:
			os.unlink(temporary, dir_fd=root_descriptor)
		except OSError:
			pass
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error


#============================================
def _read_private_file(repository_root: pathlib.Path, name: str) -> bytes:
	"""Read one fixed private receipt only after checking type, owner, and mode."""
	_require_control_name(name)
	root_descriptor = _checked_root_descriptor(repository_root)
	try:
		try:
			metadata = os.stat(name, dir_fd=root_descriptor, follow_symlinks=False)
			file_descriptor = os.open(name, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0), dir_fd=root_descriptor)
		except OSError as error:
			raise DeveloperBrowserSuiteError("Developer Browser Suite is not running") from error
		try:
			opened = os.fstat(file_descriptor)
			if (
				not stat.S_ISREG(metadata.st_mode)
				or metadata.st_uid != os.getuid()
				or stat.S_IMODE(metadata.st_mode) != 0o600
				or not stat.S_ISREG(opened.st_mode)
				or opened.st_uid != os.getuid()
				or stat.S_IMODE(opened.st_mode) != 0o600
				or (metadata.st_dev, metadata.st_ino) != (opened.st_dev, opened.st_ino)
			):
				raise DeveloperBrowserSuiteError("developer browser control state is unavailable")
			content = os.read(file_descriptor, MAXIMUM_CONTROL_BYTES + 1)
		finally:
			os.close(file_descriptor)
	finally:
		os.close(root_descriptor)
	if len(content) > MAXIMUM_CONTROL_BYTES:
		raise DeveloperBrowserSuiteError("developer browser control receipt is invalid")
	return content


#============================================
def _remove_private_entry(root_descriptor: int, name: str) -> None:
	"""Remove a fixed control artifact without following a replacement link."""
	_require_control_name(name)
	try:
		metadata = os.stat(name, dir_fd=root_descriptor, follow_symlinks=False)
	except FileNotFoundError:
		return
	except OSError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
	if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) & 0o022:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable")
	try:
		os.unlink(name, dir_fd=root_descriptor)
	except OSError as error:
		raise DeveloperBrowserSuiteError("developer browser control state is unavailable") from error
