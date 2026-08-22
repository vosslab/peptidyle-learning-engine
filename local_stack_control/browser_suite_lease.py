"""Single-flight private workspace for the disposable live-demo browser suite."""

from __future__ import annotations

import fcntl
import os
import pathlib
import stat

import local_stack_control.models


PRIVATE_ROOT = pathlib.Path("target") / "live-demo-browser"
LOCK_NAME = "browser-suite.lock"
WORKSPACE_NAME = "workspace"


class BrowserSuiteError(local_stack_control.models.ControllerError):
	"""A concise browser-suite lifecycle boundary failure."""


#============================================
def _identity(metadata: os.stat_result) -> tuple[int, int]:
	"""Return the stable device and inode fields for one checked descriptor."""
	result = metadata.st_dev, metadata.st_ino
	return result


#============================================
def _open_checked_directory(path: pathlib.Path, mode: int | None) -> tuple[int, tuple[int, int]]:
	"""Open one current-user directory without following a replacement link."""
	try:
		metadata = path.lstat()
		descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0))
	except OSError as error:
		raise BrowserSuiteError("browser-suite private root is unavailable") from error
	opened = os.fstat(descriptor)
	if (
		stat.S_ISLNK(metadata.st_mode)
		or not stat.S_ISDIR(metadata.st_mode)
		or metadata.st_uid != os.getuid()
		or stat.S_IMODE(metadata.st_mode) & 0o022
		or (mode is not None and stat.S_IMODE(metadata.st_mode) != mode)
		or not stat.S_ISDIR(opened.st_mode)
		or opened.st_uid != os.getuid()
		or stat.S_IMODE(opened.st_mode) & 0o022
		or (mode is not None and stat.S_IMODE(opened.st_mode) != mode)
		or _identity(metadata) != _identity(opened)
	):
		os.close(descriptor)
		raise BrowserSuiteError("browser-suite private root is unavailable")
	os.set_inheritable(descriptor, False)
	return descriptor, _identity(opened)


#============================================
def _remove_open_directory_contents(directory_descriptor: int) -> None:
	"""Clear descriptor-opened workspace descendants without path traversal.

	The fixed workspace is acceptance infrastructure. Its parent descriptor is
	owned by the held suite lease (ASVS 5.3.2 and 15.4.2).
	"""
	try:
		names = os.listdir(directory_descriptor)
	except OSError as error:
		raise BrowserSuiteError("browser-suite workspace is unavailable") from error
	for name in names:
		if name in (".", ".."):
			raise BrowserSuiteError("browser-suite workspace is unavailable")
		try:
			metadata = os.stat(name, dir_fd=directory_descriptor, follow_symlinks=False)
		except OSError as error:
			raise BrowserSuiteError("browser-suite workspace is unavailable") from error
		if stat.S_ISDIR(metadata.st_mode):
			try:
				child = os.open(
					name,
					os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
					dir_fd=directory_descriptor,
				)
			except OSError as error:
				raise BrowserSuiteError("browser-suite workspace is unavailable") from error
			try:
				opened = os.fstat(child)
				if _identity(opened) != _identity(metadata) or opened.st_uid != os.getuid():
					raise BrowserSuiteError("browser-suite workspace is unavailable")
				_remove_open_directory_contents(child)
				current = os.stat(name, dir_fd=directory_descriptor, follow_symlinks=False)
				if _identity(current) != _identity(opened):
					raise BrowserSuiteError("browser-suite workspace is unavailable")
				os.rmdir(name, dir_fd=directory_descriptor)
			finally:
				os.close(child)
		else:
			# A link is an entry to remove, never a target to follow.
			try:
				current = os.stat(name, dir_fd=directory_descriptor, follow_symlinks=False)
				if _identity(current) != _identity(metadata):
					raise BrowserSuiteError("browser-suite workspace is unavailable")
				os.unlink(name, dir_fd=directory_descriptor)
			except OSError as error:
				raise BrowserSuiteError("browser-suite workspace is unavailable") from error
	try:
		os.fsync(directory_descriptor)
	except OSError as error:
		raise BrowserSuiteError("browser-suite workspace is unavailable") from error


#============================================
class BrowserSuiteLease:
	"""One nonblocking host lease and one resettable private workspace."""

	def __init__(
		self,
		repository_root: pathlib.Path,
		repository_descriptor: int,
		repository_identity: tuple[int, int],
		root_descriptor: int,
		root_identity: tuple[int, int],
		lock_descriptor: int,
	) -> None:
		"""Store descriptors that remain authoritative for the held lifetime."""
		self.repository_root = repository_root
		self.root = repository_root / PRIVATE_ROOT
		self.workspace = self.root / WORKSPACE_NAME
		self._repository_descriptor = repository_descriptor
		self._repository_identity = repository_identity
		self._root_descriptor = root_descriptor
		self._root_identity = root_identity
		self._lock_descriptor = lock_descriptor
		self._released = False

	@classmethod
	def acquire(cls, repository_root: pathlib.Path) -> "BrowserSuiteLease":
		"""Acquire the suite lease before ports, state, builds, or engine work."""
		repository_descriptor, repository_identity = _open_checked_directory(repository_root, None)
		try:
			fcntl.flock(repository_descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
		except BlockingIOError as error:
			os.close(repository_descriptor)
			raise BrowserSuiteError("the live-demo browser suite is already running in this checkout") from error
		try:
			os.mkdir("target", 0o755, dir_fd=repository_descriptor)
		except FileExistsError:
			pass
		except OSError as error:
			fcntl.flock(repository_descriptor, fcntl.LOCK_UN)
			os.close(repository_descriptor)
			raise BrowserSuiteError("browser-suite private root is unavailable") from error
		root_descriptor = -1
		lock_descriptor = -1
		try:
			target_descriptor = os.open("target", os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0), dir_fd=repository_descriptor)
			try:
				try:
					os.mkdir(PRIVATE_ROOT.name, 0o700, dir_fd=target_descriptor)
				except FileExistsError:
					pass
				root_descriptor, root_identity = _open_checked_directory(repository_root / PRIVATE_ROOT, 0o700)
			finally:
				os.close(target_descriptor)
			lock_descriptor = os.open(LOCK_NAME, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600, dir_fd=root_descriptor)
			lock_metadata = os.fstat(lock_descriptor)
			if not stat.S_ISREG(lock_metadata.st_mode) or lock_metadata.st_uid != os.getuid() or stat.S_IMODE(lock_metadata.st_mode) != 0o600:
				raise BrowserSuiteError("browser-suite lease is unavailable")
			os.set_inheritable(lock_descriptor, False)
			fcntl.flock(lock_descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
		except BlockingIOError as error:
			if lock_descriptor >= 0:
				os.close(lock_descriptor)
			if root_descriptor >= 0:
				os.close(root_descriptor)
			fcntl.flock(repository_descriptor, fcntl.LOCK_UN)
			os.close(repository_descriptor)
			raise BrowserSuiteError("the live-demo browser suite is already running in this checkout") from error
		except (BrowserSuiteError, OSError) as error:
			if lock_descriptor >= 0:
				os.close(lock_descriptor)
			if root_descriptor >= 0:
				os.close(root_descriptor)
			fcntl.flock(repository_descriptor, fcntl.LOCK_UN)
			os.close(repository_descriptor)
			if isinstance(error, BrowserSuiteError):
				raise
			raise BrowserSuiteError("browser-suite lease is unavailable") from error
		return cls(repository_root, repository_descriptor, repository_identity, root_descriptor, root_identity, lock_descriptor)

	def __enter__(self) -> "BrowserSuiteLease":
		"""Return this held lease for a scoped browser-suite run."""
		return self

	def __exit__(self, exception_type: object, exception: object, traceback: object) -> None:
		"""Release descriptors after the owner has reported and reset the fixture."""
		self.release()

	def require_held(self) -> None:
		"""Require the active lease and retained descriptor identities."""
		if self._released:
			raise BrowserSuiteError("browser-suite lease is no longer held")
		try:
			repository_metadata = self.repository_root.lstat()
			root_metadata = self.root.lstat()
		except OSError as error:
			raise BrowserSuiteError("browser-suite private root was replaced") from error
		if (
			_identity(repository_metadata) != self._repository_identity
			or _identity(os.fstat(self._repository_descriptor)) != self._repository_identity
			or _identity(root_metadata) != self._root_identity
			or _identity(os.fstat(self._root_descriptor)) != self._root_identity
		):
			raise BrowserSuiteError("browser-suite private root was replaced")

	def reset_workspace(self) -> pathlib.Path:
		"""Clear and recreate the fixed mode-0700 private workspace under the lease."""
		self.require_held()
		try:
			metadata = os.stat(WORKSPACE_NAME, dir_fd=self._root_descriptor, follow_symlinks=False)
		except FileNotFoundError:
			metadata = None
		except OSError as error:
			raise BrowserSuiteError("browser-suite workspace is unavailable") from error
		if metadata is not None:
			if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) != 0o700:
				raise BrowserSuiteError("browser-suite workspace is unavailable")
			try:
				descriptor = os.open(WORKSPACE_NAME, os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0), dir_fd=self._root_descriptor)
			except OSError as error:
				raise BrowserSuiteError("browser-suite workspace is unavailable") from error
			try:
				if _identity(os.fstat(descriptor)) != _identity(metadata):
					raise BrowserSuiteError("browser-suite workspace is unavailable")
				_remove_open_directory_contents(descriptor)
			finally:
				os.close(descriptor)
		else:
			try:
				os.mkdir(WORKSPACE_NAME, 0o700, dir_fd=self._root_descriptor)
			except OSError as error:
				raise BrowserSuiteError("browser-suite workspace is unavailable") from error
		return self.workspace

	def inherited_descriptors(self) -> tuple[int, int, int]:
		"""Expose the held descriptors for one atomic supervisor handoff only.

		The launcher keeps the same flock open until the child inherits these exact
		descriptors.  This avoids any unlocked interval between developer start and
		the long-lived owner (ASVS 15.4.2 and 15.4.3).
		"""
		self.require_held()
		for descriptor in (
			self._repository_descriptor,
			self._root_descriptor,
			self._lock_descriptor,
		):
			os.set_inheritable(descriptor, True)
		result = self._repository_descriptor, self._root_descriptor, self._lock_descriptor
		return result

	@classmethod
	def adopt(
		cls,
		repository_root: pathlib.Path,
		repository_descriptor: int,
		root_descriptor: int,
		lock_descriptor: int,
	) -> "BrowserSuiteLease":
		"""Adopt the launcher's already-held descriptor locks in its supervisor child."""
		try:
			repository_metadata = repository_root.lstat()
			root_metadata = (repository_root / PRIVATE_ROOT).lstat()
			repository_opened = os.fstat(repository_descriptor)
			root_opened = os.fstat(root_descriptor)
			lock_metadata = os.fstat(lock_descriptor)
		except OSError as error:
			raise BrowserSuiteError("browser-suite inherited lease is unavailable") from error
		if (
			not stat.S_ISDIR(repository_opened.st_mode)
			or not stat.S_ISDIR(root_opened.st_mode)
			or not stat.S_ISREG(lock_metadata.st_mode)
			or _identity(repository_metadata) != _identity(repository_opened)
			or _identity(root_metadata) != _identity(root_opened)
			or root_opened.st_uid != os.getuid()
			or stat.S_IMODE(root_opened.st_mode) != 0o700
			or lock_metadata.st_uid != os.getuid()
			or stat.S_IMODE(lock_metadata.st_mode) != 0o600
		):
			raise BrowserSuiteError("browser-suite inherited lease is unavailable")
		for descriptor in (repository_descriptor, root_descriptor, lock_descriptor):
			os.set_inheritable(descriptor, False)
		result = cls(
			repository_root,
			repository_descriptor,
			_identity(repository_opened),
			root_descriptor,
			_identity(root_opened),
			lock_descriptor,
		)
		return result

	def release(self) -> None:
		"""Release the non-inheritable root and checkout locks exactly once."""
		if self._released:
			return
		self._released = True
		try:
			fcntl.flock(self._lock_descriptor, fcntl.LOCK_UN)
		finally:
			os.close(self._lock_descriptor)
			os.close(self._root_descriptor)
			fcntl.flock(self._repository_descriptor, fcntl.LOCK_UN)
			os.close(self._repository_descriptor)

	def detach_for_supervisor_handoff(self) -> None:
		"""Close the launcher's copies without unlocking an inherited child lease.

		`flock(LOCK_UN)` would release the shared open-file-description lock for
		the child as well.  Closing only this process's descriptors preserves the
		continuous lock while the adopted supervisor descriptors remain open.
		"""
		if self._released:
			return
		self._released = True
		os.close(self._lock_descriptor)
		os.close(self._root_descriptor)
		os.close(self._repository_descriptor)
