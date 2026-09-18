"""Prune unused Podman images before a complete image-building lifecycle."""

import collections.abc
import contextlib
import fcntl
import os
import pathlib

import local_stack_control.models
import local_stack_control.process


#============================================
@contextlib.contextmanager
def image_build_lease(repo_root: pathlib.Path) -> collections.abc.Iterator[None]:
	"""Exclude overlapping checkout image cycles until their containers attach."""
	# ASVS 2.3.4: hold one stable directory inode for the complete image cycle.
	descriptor = os.open(repo_root / "containers", os.O_RDONLY | os.O_NOFOLLOW)
	try:
		try:
			fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
		except BlockingIOError as error:
			raise local_stack_control.models.ControllerError("another project image build is active") from error
		yield
	finally:
		os.close(descriptor)


#============================================
def remove_obsolete_images_before_build(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> None:
	"""Remove dangling (untagged) image layers left behind by earlier rebuilds.

	Tagged images stay: the reviewed renderer takes minutes to rebuild, pulled
	service images take minutes to fetch, and the operator's unrelated images are
	not this controller's to remove.  Only layers no tag references any more go.
	"""
	# ASVS 1.2.5: the authorized engine-wide operation uses fixed parameterized argv.
	result = runner.run(
		["podman", "image", "prune", "-f"],
		local_stack_control.process.current_environment(), repo_root,
	)
	if not result.ok():
		raise local_stack_control.models.ControllerError("pre-build unused image pruning failed")
