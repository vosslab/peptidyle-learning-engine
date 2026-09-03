"""Default-only local environment and private secret bootstrap."""

import base64
import collections.abc
import os
import pathlib

import local_stack_control.models
import local_stack_control.private_files


#============================================
def is_default_local_environment(repo_root: pathlib.Path, env_file: pathlib.Path) -> bool:
	"""Return whether an environment path is the one supported bootstrap target."""
	default_path = (repo_root / local_stack_control.models.DEFAULT_ENV_FILE).absolute()
	result = not env_file.is_symlink() and env_file.absolute() == default_path
	return result


#============================================
def bootstrap_default_environment(
	repo_root: pathlib.Path,
	env_file: pathlib.Path,
	example_file: pathlib.Path,
) -> bool:
	"""Create the default local environment once and never rewrite an existing file."""
	if not is_default_local_environment(repo_root, env_file):
		return False
	if env_file.exists() or env_file.is_symlink():
		return False
	try:
		example_content = example_file.read_bytes()
	except OSError as error:
		raise local_stack_control.models.ControllerError("default environment template is unavailable") from error
	local_stack_control.private_files.write_atomic_file(env_file, example_content, 0o600)
	return True


#============================================
def bootstrap_secret32_file(
	path: pathlib.Path,
	random_bytes: collections.abc.Callable[[int], bytes] = os.urandom,
) -> bool:
	"""Create one missing private secret file, preserving a valid existing file."""
	if path.exists() or path.is_symlink():
		read_secret32_file(path)
		return False
	secret = random_bytes(32)
	if len(secret) != 32:
		raise local_stack_control.models.ControllerError("secret generator did not provide private secret material")
	encoded = base64.urlsafe_b64encode(secret).rstrip(b"=")
	local_stack_control.private_files.write_atomic_file(path, encoded, 0o600)
	read_secret32_file(path)
	return True


#============================================
def read_secret32_file(path: pathlib.Path) -> bytes:
	"""Load one private base64url 32-byte secret without exposing it in diagnostics."""
	content = local_stack_control.private_files.read_current_user_private_file(path, 43)
	secret = local_stack_control.private_files.decode_base64url_secret32(content)
	return secret
