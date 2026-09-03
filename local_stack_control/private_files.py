"""Strict private-file reads and atomic local writes."""

import base64
import os
import pathlib
import stat
import tempfile

import local_stack_control.models


#============================================
def read_current_user_private_file(
	path: pathlib.Path,
	maximum_bytes: int,
	owner_id: int | None = None,
) -> bytes:
	"""Read a bounded mode-0600 regular file without following links.

	Args:
		path: Private file supplied by the selected local configuration.
		maximum_bytes: Largest permitted content size.

	Returns:
		Validated file bytes.

	Raises:
		ControllerError: If the path is not a current-user private regular file.
	"""
	if maximum_bytes < 1:
		raise local_stack_control.models.ControllerError("private file limit must be positive")
	try:
		path_stat = os.lstat(path)
	except OSError as error:
		raise local_stack_control.models.ControllerError("private file is unavailable") from error
	if stat.S_ISLNK(path_stat.st_mode) or not stat.S_ISREG(path_stat.st_mode):
		raise local_stack_control.models.ControllerError("private file must be a regular non-symbolic-link file")
	flags = os.O_RDONLY
	if hasattr(os, "O_NOFOLLOW"):
		flags |= os.O_NOFOLLOW
	try:
		file_descriptor = os.open(path, flags)
	except OSError as error:
		raise local_stack_control.models.ControllerError("private file is unavailable") from error
	try:
		file_stat = os.fstat(file_descriptor)
		validate_private_file_stat(file_stat, owner_id)
		content = read_bounded_descriptor(file_descriptor, maximum_bytes)
	finally:
		os.close(file_descriptor)
	return content


#============================================
def validate_private_file_stat(file_stat: os.stat_result, owner_id: int | None = None) -> None:
	"""Require an exact current-user, regular, mode-0600 file stat result."""
	if not stat.S_ISREG(file_stat.st_mode):
		raise local_stack_control.models.ControllerError("private file must be regular")
	current_owner = os.getuid() if owner_id is None else owner_id
	if file_stat.st_uid != current_owner:
		raise local_stack_control.models.ControllerError("private file must be owned by the current user")
	if stat.S_IMODE(file_stat.st_mode) != 0o600:
		raise local_stack_control.models.ControllerError("private file must have mode 0600")


#============================================
def require_private_directory(directory: pathlib.Path, owner_id: int | None = None) -> None:
	"""Require a current-user directory without group or other write authority."""
	try:
		path_stat = os.lstat(directory)
	except OSError as error:
		raise local_stack_control.models.ControllerError("private state directory is unavailable") from error
	if stat.S_ISLNK(path_stat.st_mode) or not stat.S_ISDIR(path_stat.st_mode):
		raise local_stack_control.models.ControllerError("private state directory must be a real directory")
	flags = os.O_RDONLY
	if hasattr(os, "O_DIRECTORY"):
		flags |= os.O_DIRECTORY
	if hasattr(os, "O_NOFOLLOW"):
		flags |= os.O_NOFOLLOW
	try:
		file_descriptor = os.open(directory, flags)
	except OSError as error:
		raise local_stack_control.models.ControllerError("private state directory is unavailable") from error
	try:
		directory_stat = os.fstat(file_descriptor)
	finally:
		os.close(file_descriptor)
	if not stat.S_ISDIR(directory_stat.st_mode):
		raise local_stack_control.models.ControllerError("private state directory must be a real directory")
	current_owner = os.getuid() if owner_id is None else owner_id
	if directory_stat.st_uid != current_owner:
		raise local_stack_control.models.ControllerError("private state directory must be owned by the current user")
	if stat.S_IMODE(directory_stat.st_mode) & 0o022:
		raise local_stack_control.models.ControllerError("private state directory must not be group or world writable")


#============================================
def read_bounded_descriptor(file_descriptor: int, maximum_bytes: int) -> bytes:
	"""Read at most a known-safe amount from one already-open descriptor."""
	content = bytearray()
	while len(content) <= maximum_bytes:
		chunk = os.read(file_descriptor, maximum_bytes + 1 - len(content))
		if chunk == b"":
			break
		content.extend(chunk)
	if len(content) > maximum_bytes:
		raise local_stack_control.models.ControllerError("private file content is too large")
	result = bytes(content)
	return result


#============================================
def decode_base64url_secret32(value: bytes) -> bytes:
	"""Decode an unpadded base64url encoding of exactly 32 secret bytes."""
	try:
		text = value.decode("ascii")
	except UnicodeDecodeError as error:
		raise local_stack_control.models.ControllerError(
			"private secret must be unpadded base64url encoding of exactly 32 bytes"
		) from error
	if len(text) != 43 or any(character not in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_" for character in text):
		raise local_stack_control.models.ControllerError(
			"private secret must be unpadded base64url encoding of exactly 32 bytes"
		)
	try:
		decoded = base64.urlsafe_b64decode(text + "=")
	except ValueError as error:
		raise local_stack_control.models.ControllerError(
			"private secret must be unpadded base64url encoding of exactly 32 bytes"
		) from error
	if len(decoded) != 32:
		raise local_stack_control.models.ControllerError(
			"private secret must be unpadded base64url encoding of exactly 32 bytes"
		)
	if base64.urlsafe_b64encode(decoded).decode("ascii").rstrip("=") != text:
		raise local_stack_control.models.ControllerError(
			"private secret must be unpadded base64url encoding of exactly 32 bytes"
		)
	result = decoded
	return result


#============================================
def write_atomic_file(path: pathlib.Path, content: bytes, mode: int) -> None:
	"""Atomically replace one local file after durable restrictive creation."""
	path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
	require_private_directory(path.parent)
	file_descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
	temporary_path = pathlib.Path(temporary_name)
	try:
		os.fchmod(file_descriptor, mode)
		write_all(file_descriptor, content)
		os.fsync(file_descriptor)
		os.close(file_descriptor)
		file_descriptor = -1
		os.replace(temporary_path, path)
		fsync_directory(path.parent)
	finally:
		if file_descriptor >= 0:
			os.close(file_descriptor)
		if temporary_path.exists():
			temporary_path.unlink()


#============================================
def write_all(file_descriptor: int, content: bytes) -> None:
	"""Write all content without assuming a single system call completes it."""
	position = 0
	while position < len(content):
		written = os.write(file_descriptor, content[position:])
		if written == 0:
			raise local_stack_control.models.ControllerError("could not write private local state")
		position += written


#============================================
def fsync_directory(directory: pathlib.Path) -> None:
	"""Durably record a replacement where the platform supports directory sync."""
	try:
		file_descriptor = os.open(directory, os.O_RDONLY)
	except OSError:
		return
	try:
		os.fsync(file_descriptor)
	except OSError:
		pass
	finally:
		os.close(file_descriptor)
