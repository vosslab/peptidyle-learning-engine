"""Offline behavioral coverage for default private local state."""

import base64
import hashlib
import json
import os
import pathlib
import stat

import pytest

import local_stack_control.local_environment
import local_stack_control.local_identity
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.compose


#============================================
def configuration(tmp_path: pathlib.Path) -> local_stack_control.local_identity.LocalIdentityConfiguration:
	"""Build a private local identity boundary under pytest-owned storage."""
	result = local_stack_control.local_identity.LocalIdentityConfiguration(
		credential_file=tmp_path / "local-login.txt",
		identity_file=tmp_path / "local-identities.json",
		tenant_id="00000000-0000-0000-0000-000000000100",
		instructor_id="00000000-0000-0000-0000-000000000101",
		student_id="00000000-0000-0000-0000-000000000102",
	)
	return result


#============================================
def test_bootstrap_is_idempotent_and_projects_hashes(tmp_path: pathlib.Path) -> None:
	"""Bootstrap creates private credentials once and publicly exposes hashes only."""
	identity = configuration(tmp_path)
	values = iter((b"A" * 32, b"B" * 32))
	local_stack_control.local_identity.bootstrap_local_identities(identity, lambda size: next(values))
	first_credentials = identity.credential_file.read_bytes()
	local_stack_control.local_identity.bootstrap_local_identities(identity)
	projection = json.loads(identity.identity_file.read_text(encoding="ascii"))
	assert identity.credential_file.read_bytes() == first_credentials
	assert all(value.decode("ascii") not in identity.identity_file.read_text(encoding="ascii") for value in (b"A" * 32, b"B" * 32))
	assert projection["credentials"][0]["credential_sha256"] == hashlib.sha256(b"A" * 32).hexdigest()


#============================================
@pytest.mark.parametrize("kind", ("symlink", "directory", "mode", "content", "owner"))
def test_rejected_private_credentials_leave_projection_untouched(
	tmp_path: pathlib.Path,
	kind: str,
) -> None:
	"""Invalid credentials cannot replace a previously generated public projection."""
	identity = configuration(tmp_path)
	sentinel = b'{"credentials":["unchanged"]}\n'
	identity.identity_file.write_bytes(sentinel)
	os.chmod(identity.identity_file, 0o644)
	if kind == "symlink":
		target = tmp_path / "target.txt"
		target.write_bytes(b"instructor=" + b"A" * 43 + b"\nstudent=" + b"B" * 43 + b"\n")
		os.chmod(target, 0o600)
		identity.credential_file.symlink_to(target)
	elif kind == "directory":
		identity.credential_file.mkdir()
	elif kind == "mode":
		identity.credential_file.write_bytes(b"instructor=" + b"A" * 43 + b"\nstudent=" + b"B" * 43 + b"\n")
		os.chmod(identity.credential_file, 0o644)
	elif kind == "content":
		identity.credential_file.write_bytes(b"instructor=not-a-secret\nstudent=also-not-a-secret\n")
		os.chmod(identity.credential_file, 0o600)
	else:
		identity.credential_file.write_bytes(b"instructor=" + b"A" * 43 + b"\nstudent=" + b"B" * 43 + b"\n")
		os.chmod(identity.credential_file, 0o600)
		with pytest.raises(local_stack_control.models.ControllerError):
			local_stack_control.local_identity.bootstrap_local_identities(identity, owner_id=os.getuid() + 1)
		assert identity.identity_file.read_bytes() == sentinel
		return
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.local_identity.bootstrap_local_identities(identity)
	assert identity.identity_file.read_bytes() == sentinel


#============================================
def test_identity_rotation_and_duplicate_credentials_are_refused(tmp_path: pathlib.Path) -> None:
	"""A public projection prevents unintentional rotation and credentials stay distinct."""
	identity = configuration(tmp_path)
	identity.identity_file.write_text("{}\n", encoding="ascii")
	os.chmod(identity.identity_file, 0o644)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.local_identity.bootstrap_local_identities(identity)
	identity.identity_file.unlink()
	value = base64.urlsafe_b64encode(b"A" * 32).rstrip(b"=")
	identity.credential_file.write_bytes(b"instructor=" + value + b"\nstudent=" + value + b"\n")
	os.chmod(identity.credential_file, 0o600)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.local_identity.bootstrap_local_identities(identity)


#============================================
def test_private_secret_bootstrap_and_errors_do_not_disclose_secret(tmp_path: pathlib.Path) -> None:
	"""Default private secret bootstrap is canonical, idempotent, and redactable."""
	secret_path = tmp_path / "state" / "issuer-secret"
	created = local_stack_control.local_environment.bootstrap_secret32_file(
		secret_path, lambda size: b"C" * 32
	)
	value = secret_path.read_bytes()
	assert created and stat.S_IMODE(secret_path.stat().st_mode) == 0o600
	assert not local_stack_control.local_environment.bootstrap_secret32_file(secret_path)
	secret_path.write_bytes(value + b"x")
	with pytest.raises(local_stack_control.models.ControllerError) as caught:
		local_stack_control.local_environment.read_secret32_file(secret_path)
	assert value.decode("ascii") not in str(caught.value)


#============================================
def test_default_environment_creation_never_overwrites_custom_or_existing(tmp_path: pathlib.Path) -> None:
	"""Only the selected missing default path receives a mode-0600 environment."""
	repo_root = tmp_path / "repo"
	example = repo_root / "containers" / "env.example"
	example.parent.mkdir(parents=True)
	example.write_text("VALUE=example\n", encoding="ascii")
	default_env = repo_root / "containers" / "env.local"
	assert local_stack_control.local_environment.bootstrap_default_environment(repo_root, default_env, example)
	assert stat.S_IMODE(default_env.stat().st_mode) == 0o600
	default_env.write_text("VALUE=kept\n", encoding="ascii")
	assert not local_stack_control.local_environment.bootstrap_default_environment(repo_root, default_env, example)
	assert not local_stack_control.local_environment.bootstrap_default_environment(
		repo_root, tmp_path / "custom.env", example
	)


#============================================
def test_private_writer_refuses_a_shared_parent_directory(tmp_path: pathlib.Path) -> None:
	"""Private state is never replaced inside a group-writable directory."""
	directory = tmp_path / "shared"
	directory.mkdir()
	directory.chmod(0o775)
	path = directory / "secret"
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.private_files.write_atomic_file(path, b"private", 0o600)
	assert not path.exists()


#============================================
def test_disposable_capability_reader_refuses_a_link_without_returning_target_bytes(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A pathname swap to a link cannot make disposable authority read target bytes."""
	target = tmp_path / "target-capability"
	target.write_bytes(b"C" * 32)
	target.chmod(0o600)
	capability = tmp_path / "capability"
	capability.symlink_to(target)
	metadata = target.stat()
	monkeypatch.setattr(local_stack_control.private_files.os, "lstat", lambda path: metadata)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.compose.require_disposable_capability_file(capability)
	assert target.read_bytes() == b"C" * 32
