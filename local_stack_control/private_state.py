"""VM-mountable, descriptor-anchored private E2E state directories."""

from __future__ import annotations

import abc
import json
import os
import pathlib
import shutil
import stat
import tempfile
from dataclasses import dataclass

import local_stack_control.models
import local_stack_control.private_files


_RECEIPT_SUFFIX = ".private-state.json"


class PrivateStateHandle(abc.ABC):
	"""Minimal lifecycle contract shared by checked private-state owners."""

	directory: pathlib.Path

	@abc.abstractmethod
	def remove(self) -> None:
		"""Remove or reset the checked private state represented by this handle."""


def _identity(metadata: os.stat_result) -> tuple[int, int]:
	"""Return the immutable identity fields used to reject replacement paths."""
	return metadata.st_dev, metadata.st_ino


def _directory_descriptor(path: pathlib.Path) -> int:
	"""Open one directory without allowing a symbolic-link traversal."""
	return os.open(path, os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0))


@dataclass(frozen=True)
class PrivateState(PrivateStateHandle):
	"""One exact run directory below a checked repository-owned target root."""

	repository_root: pathlib.Path
	relative_root: pathlib.Path
	directory: pathlib.Path
	parent_identity: tuple[int, int]
	root_identity: tuple[int, int]
	directory_identity: tuple[int, int]
	prefix: str

	def _unavailable(self, action: str) -> local_stack_control.models.ControllerError:
		"""Build the safe, non-path-disclosing failure for one state operation."""
		return local_stack_control.models.ControllerError(f"private state is unavailable for {action}")

	def _root_descriptor(self, action: str) -> int:
		"""Open the exact private root through its saved parent identity."""
		parent = self.repository_root / self.relative_root.parent
		try:
			parent_metadata = parent.lstat()
		except OSError as error:
			raise self._unavailable(action) from error
		if (
			stat.S_ISLNK(parent_metadata.st_mode)
			or not stat.S_ISDIR(parent_metadata.st_mode)
			or _identity(parent_metadata) != self.parent_identity
		):
			raise self._unavailable(action)
		parent_descriptor = -1
		root_descriptor = -1
		try:
			parent_descriptor = _directory_descriptor(parent)
			opened_parent = os.fstat(parent_descriptor)
			if (
				not stat.S_ISDIR(opened_parent.st_mode)
				or _identity(opened_parent) != self.parent_identity
			):
				raise self._unavailable(action)
			root_descriptor = os.open(
				self.relative_root.name,
				os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
				dir_fd=parent_descriptor,
			)
			opened_root = os.fstat(root_descriptor)
			if (
				not stat.S_ISDIR(opened_root.st_mode)
				or stat.S_IMODE(opened_root.st_mode) != 0o700
				or _identity(opened_root) != self.root_identity
			):
				raise self._unavailable(action)
		except OSError as error:
			if root_descriptor >= 0:
				os.close(root_descriptor)
			raise self._unavailable(action) from error
		except BaseException:
			if root_descriptor >= 0:
				os.close(root_descriptor)
			raise
		finally:
			if parent_descriptor >= 0:
				os.close(parent_descriptor)
		if root_descriptor < 0:
			raise self._unavailable(action)
		return root_descriptor

	def directory_descriptor(self) -> int:
		"""Open the exact mode-0700 run directory without following replacements.

		The caller owns the returned descriptor and must close it.  The descriptor,
		rather than the pathname, is the authority for private artifact operations.
		"""
		if (
			self.directory.parent != self.repository_root / self.relative_root
			or not self.directory.name.startswith(self.prefix)
		):
			raise self._unavailable("directory access")
		try:
			metadata = self.directory.lstat()
			if (
				stat.S_ISLNK(metadata.st_mode)
				or not stat.S_ISDIR(metadata.st_mode)
				or stat.S_IMODE(metadata.st_mode) != 0o700
				or _identity(metadata) != self.directory_identity
			):
				raise self._unavailable("directory access")
			descriptor = _directory_descriptor(self.directory)
		except OSError as error:
			raise self._unavailable("directory access") from error
		try:
			opened = os.fstat(descriptor)
			if (
				not stat.S_ISDIR(opened.st_mode)
				or stat.S_IMODE(opened.st_mode) != 0o700
				or _identity(opened) != self.directory_identity
			):
				raise self._unavailable("directory access")
		except BaseException:
			os.close(descriptor)
			raise
		return descriptor

	def remove(self) -> None:
		"""Remove only this checked directory through its verified root descriptor."""
		if (
			self.directory.parent != self.repository_root / self.relative_root
			or not self.directory.name.startswith(self.prefix)
		):
			raise local_stack_control.models.ControllerError(
				"private state path is outside its owned target subtree"
			)
		root_descriptor = self._root_descriptor("cleanup")
		try:
			entry_metadata = os.stat(
				self.directory.name,
				dir_fd=root_descriptor,
				follow_symlinks=False,
			)
			if (
				stat.S_ISLNK(entry_metadata.st_mode)
				or not stat.S_ISDIR(entry_metadata.st_mode)
				or stat.S_IMODE(entry_metadata.st_mode) != 0o700
				or _identity(entry_metadata) != self.directory_identity
			):
				raise self._unavailable("cleanup")
			shutil.rmtree(self.directory.name, dir_fd=root_descriptor)
		except OSError as error:
			raise self._unavailable("cleanup") from error
		finally:
			os.close(root_descriptor)


def prepare(
	repository_root: pathlib.Path,
	relative_root: pathlib.Path,
	prefix: str = "run-",
) -> PrivateState:
	"""Create a mode-0700 run directory under an ignored repository target root.

	The target root is deliberately repository-relative so it is visible to a
	remote Podman machine's checked-out-worktree share; system temporary space is
	never suitable for files later used as bind-mount sources.
	"""
	if (
		relative_root.is_absolute()
		or len(relative_root.parts) < 2
		or relative_root.parts[:1] != ("target",)
		or ".." in relative_root.parts
	):
		raise local_stack_control.models.ControllerError(
			"private state root must be a relative target directory"
		)
	parent = repository_root / relative_root.parent
	private_root = repository_root / relative_root
	try:
		parent.mkdir(exist_ok=True)
		parent_metadata = parent.lstat()
		if stat.S_ISLNK(parent_metadata.st_mode) or not stat.S_ISDIR(parent_metadata.st_mode):
			raise local_stack_control.models.ControllerError("could not prepare private state")
		try:
			root_metadata = private_root.lstat()
		except FileNotFoundError:
			private_root.mkdir(mode=0o700)
			private_root.chmod(0o700)
			root_metadata = private_root.lstat()
		if (
			stat.S_ISLNK(root_metadata.st_mode)
			or not stat.S_ISDIR(root_metadata.st_mode)
			or stat.S_IMODE(root_metadata.st_mode) != 0o700
		):
			raise local_stack_control.models.ControllerError("could not prepare private state")
		directory = pathlib.Path(tempfile.mkdtemp(prefix=prefix, dir=private_root))
		directory.chmod(0o700)
		directory_metadata = directory.lstat()
	except OSError as error:
		raise local_stack_control.models.ControllerError("could not prepare private state") from error
	if (
		stat.S_ISLNK(directory_metadata.st_mode)
		or not stat.S_ISDIR(directory_metadata.st_mode)
		or stat.S_IMODE(directory_metadata.st_mode) != 0o700
	):
		raise local_stack_control.models.ControllerError("could not prepare private state")
	return PrivateState(
		repository_root=repository_root,
		relative_root=relative_root,
		directory=directory,
		parent_identity=_identity(parent_metadata),
		root_identity=_identity(root_metadata),
		directory_identity=_identity(directory_metadata),
		prefix=prefix,
	)


def _receipt_path(directory: pathlib.Path) -> pathlib.Path:
	"""Return the private cross-process receipt beside one run directory."""
	return directory.parent / f".{directory.name}{_RECEIPT_SUFFIX}"


def prepare_persisted(
	repository_root: pathlib.Path,
	relative_root: pathlib.Path,
	prefix: str = "run-",
) -> PrivateState:
	"""Create state whose captured identities can be verified by a later process."""
	state = prepare(repository_root, relative_root, prefix)
	receipt = {
		"directoryIdentity": list(state.directory_identity),
		"directoryName": state.directory.name,
		"parentIdentity": list(state.parent_identity),
		"prefix": state.prefix,
		"rootIdentity": list(state.root_identity),
	}
	try:
		local_stack_control.private_files.write_atomic_file(
			_receipt_path(state.directory),
			(json.dumps(receipt, separators=(",", ":"), sort_keys=True) + "\n").encode("ascii"),
			0o600,
		)
	except BaseException:
		state.remove()
		raise
	return state


def _receipt_identity(value: object) -> tuple[int, int]:
	"""Decode one non-negative device and inode pair from a private receipt."""
	if not isinstance(value, list) or len(value) != 2:
		raise local_stack_control.models.ControllerError("private state receipt is invalid")
	device, inode = value
	if not isinstance(device, int) or device < 0 or not isinstance(inode, int) or inode < 0:
		raise local_stack_control.models.ControllerError("private state receipt is invalid")
	return device, inode


def remove_persisted(
	repository_root: pathlib.Path,
	relative_root: pathlib.Path,
	directory: pathlib.Path,
	prefix: str = "run-",
) -> None:
	"""Remove cross-process state using only its original captured identities."""
	expected_root = repository_root / relative_root
	if directory.parent != expected_root or not directory.name.startswith(prefix):
		raise local_stack_control.models.ControllerError(
			"private state path is outside its owned target subtree"
		)
	receipt_path = _receipt_path(directory)
	try:
		content = local_stack_control.private_files.read_current_user_private_file(
			receipt_path, 1_024
		)
		receipt = json.loads(content.decode("ascii"))
	except (
		local_stack_control.models.ControllerError,
		UnicodeError,
		json.JSONDecodeError,
	) as error:
		raise local_stack_control.models.ControllerError("private state receipt is invalid") from error
	if not isinstance(receipt, dict) or set(receipt) != {
		"directoryIdentity",
		"directoryName",
		"parentIdentity",
		"prefix",
		"rootIdentity",
	}:
		raise local_stack_control.models.ControllerError("private state receipt is invalid")
	if receipt["directoryName"] != directory.name or receipt["prefix"] != prefix:
		raise local_stack_control.models.ControllerError("private state receipt is invalid")
	state = PrivateState(
		repository_root=repository_root,
		relative_root=relative_root,
		directory=directory,
		parent_identity=_receipt_identity(receipt["parentIdentity"]),
		root_identity=_receipt_identity(receipt["rootIdentity"]),
		directory_identity=_receipt_identity(receipt["directoryIdentity"]),
		prefix=prefix,
	)
	state.remove()
	try:
		receipt_path.unlink()
	except OSError as error:
		raise local_stack_control.models.ControllerError(
			"private state receipt could not be removed"
		) from error
