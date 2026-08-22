"""Focused durable-publication checks for the canonical screenshot corpus."""

import hashlib
import json
import pathlib
import stat
import sys
import zlib

import pytest

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_screenshot_contract as contract
import e2e_browser_screenshot_publisher as publisher


def png(width: int = 1280, height: int = 800) -> bytes:
	"""Build the smallest complete CRC-valid PNG accepted by the publisher."""
	def chunk(kind: bytes, data: bytes) -> bytes:
		prefix = len(data).to_bytes(4, "big") + kind + data
		return prefix + (zlib.crc32(kind + data) & 0xffffffff).to_bytes(4, "big")
	ihdr = width.to_bytes(4, "big") + height.to_bytes(4, "big") + b"\x08\x02\x00\x00\x00"
	value = publisher.PNG_SIGNATURE + chunk(b"IHDR", ihdr) + chunk(b"IDAT", b"\x78\x9c\x03\x00\x00\x00\x00\x01") + chunk(b"IEND", b"")
	return value


def staging(path: pathlib.Path) -> publisher.PendingScreenshotPublication:
	"""Produce one private fixed twenty-artifact bundle with exact receipts."""
	path.mkdir(mode=0o700)
	path.chmod(0o700)
	for artifact in contract.ARTIFACTS:
		viewport = contract.VIEWPORT_PROFILES[artifact.viewport]
		content = png(viewport.width, viewport.height)
		digest = hashlib.sha256(content).hexdigest()
		image = path / f"{artifact.artifact_id}.png"
		image.write_bytes(content)
		image.chmod(0o600)
		checks = publisher.privacy_checks(artifact)
		receipt = {"artifactId": artifact.artifact_id, "scenarioId": artifact.scenario_id, "stateId": artifact.state_id, "sha256": digest, "viewport": artifact.viewport, "width": viewport.width, "height": viewport.height, "origin": "https://localhost:55000", "privacyValidated": True, "privacyChecks": checks}
		item = path / f"{artifact.artifact_id}.json"
		item.write_text(json.dumps(receipt), encoding="ascii")
		item.chmod(0o600)
	pending = publisher.pending_from_staging(path, "https://localhost:55000", "b" * 64)
	return pending


def corpus(root: pathlib.Path, existing: bool = True) -> pathlib.Path:
	"""Create the owner-controlled public corpus directory and prior bytes."""
	directory = root / "docs" / "screenshots"
	directory.mkdir(parents=True)
	directory.chmod(0o755)
	if existing:
		for artifact in contract.ARTIFACTS:
			target = root / artifact.path
			target.parent.mkdir(parents=True, exist_ok=True)
			target.write_bytes(b"prior-" + artifact.artifact_id.encode("ascii"))
			target.chmod(0o644)
	return directory


def prior_provenance(directory: pathlib.Path, pending: publisher.PendingScreenshotPublication, retired: pathlib.PurePosixPath) -> None:
	"""Write one valid prior generation that grants ownership of a renamed image."""
	value = publisher._provenance_value(pending)
	record = value["artifacts"][0]
	assert isinstance(record, dict)
	record["path"] = str(retired)
	root = directory.parent.parent
	for artifact, content, _ in pending.artifacts[1:]:
		target = root / artifact.path
		target.parent.mkdir(parents=True, exist_ok=True)
		target.write_bytes(content)
	value["generationIdentity"] = publisher._prior_generation_identity(
		value["origin"], value["productionDistDigest"], value["artifacts"]
	)
	path = directory / publisher.PROVENANCE_PATH.name
	path.write_text(json.dumps(value), encoding="ascii")
	path.chmod(0o644)


def corpus_files(directory: pathlib.Path) -> dict[str, bytes]:
	"""Return only durable public members, independent of safe empty directories."""
	return {
		str(item.relative_to(directory)): item.read_bytes()
		for item in directory.rglob("*")
		if item.is_file()
	}


@pytest.mark.parametrize("mutation", [lambda value: value[:-1], lambda value: value[:29] + b"\x00" + value[30:], lambda value: value + b"x"])
def test_full_png_validation_rejects_bad_crc_framing_and_terminal_data(mutation: object) -> None:
	"""Private capture inputs require every PNG structural invariant."""
	with pytest.raises(publisher.ScreenshotPublicationError): publisher._validate_png(mutation(png()))


def test_receipt_viewport_must_match_the_manifest_profile(tmp_path: pathlib.Path) -> None:
	"""The browser receipt cannot relabel captured dimensions as another profile."""
	private = tmp_path / "private"
	staging(private)
	artifact = contract.ARTIFACTS[0]
	receipt_path = private / f"{artifact.artifact_id}.json"
	receipt = json.loads(receipt_path.read_text(encoding="ascii"))
	receipt["viewport"] = "square"
	receipt_path.write_text(json.dumps(receipt), encoding="ascii")
	with pytest.raises(publisher.ScreenshotPublicationError):
		publisher.pending_from_staging(private, "https://localhost:55000", "b" * 64)


def test_screenshot_contract_accepts_a_single_contiguous_artifact(monkeypatch: pytest.MonkeyPatch) -> None:
	"""The corpus contract remains valid when growth leaves only one ordered artifact."""
	artifact = contract.dataclasses.replace(contract.ARTIFACTS[0], capture_order=1)
	monkeypatch.setattr(contract, "ARTIFACTS", (artifact,))
	contract.validate()


def test_staging_and_existing_corpus_reject_links(tmp_path: pathlib.Path) -> None:
	"""Descriptor-relative reads reject both private and public link substitution."""
	private = tmp_path / "private"
	staging(private)
	(private / f"{contract.ARTIFACTS[0].artifact_id}.png").unlink()
	(private / f"{contract.ARTIFACTS[0].artifact_id}.png").symlink_to(tmp_path / "missing")
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.pending_from_staging(private, "https://localhost:55000", "b" * 64)
	pending = staging(tmp_path / "private-again")
	corpus(tmp_path / "public")
	target = tmp_path / "public" / contract.ARTIFACTS[0].path
	target.unlink()
	target.symlink_to(tmp_path / "missing")
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.publish(tmp_path / "public", pending)


@pytest.mark.parametrize("failed_call", range(1, len(contract.ARTIFACTS) + 2))
def test_publication_restores_existing_generation_after_each_write_failure(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch, failed_call: int
) -> None:
	"""Each image or final provenance failure restores the prior complete corpus."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	prior = corpus_files(directory)
	original = publisher._atomic_write
	calls = 0

	def fail_once(directory_fd: int, name: str, content: bytes, mode: int, expected: publisher.PublicFileIdentity | None) -> publisher.PublicFileIdentity:
		nonlocal calls
		calls += 1
		if calls == failed_call: raise OSError("injected publication failure")
		return original(directory_fd, name, content, mode, expected)

	monkeypatch.setattr(publisher, "_atomic_write", fail_once)
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.publish(tmp_path / "public", pending)
	assert corpus_files(directory) == prior
	assert not list(directory.glob(".*.tmp"))


def test_publication_removes_new_targets_after_provenance_failure(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> None:
	"""A new corpus remains empty when its final generation member cannot commit."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public", existing=False)
	original = publisher._atomic_write
	calls = 0

	def fail_final(directory_fd: int, name: str, content: bytes, mode: int, expected: publisher.PublicFileIdentity | None) -> publisher.PublicFileIdentity:
		nonlocal calls
		calls += 1
		if calls == len(contract.ARTIFACTS) + 1: raise OSError("injected provenance failure")
		return original(directory_fd, name, content, mode, expected)

	monkeypatch.setattr(publisher, "_atomic_write", fail_final)
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.publish(tmp_path / "public", pending)
	assert corpus_files(directory) == {}


def test_invalid_pending_provenance_is_rejected_before_public_write(tmp_path: pathlib.Path) -> None:
	"""Publication binds the exact production dist, origin, and generation identity."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	invalid = publisher.dataclasses.replace(pending, production_dist_digest="D" * 64)
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.publish(tmp_path / "public", invalid)
	assert not (directory / publisher.PROVENANCE_PATH.name).exists()


def test_descriptor_reads_accumulate_partial_kernel_reads(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> None:
	"""Private and backup readers consume exact snapshots despite short os.read calls."""
	private = tmp_path / "private"
	staging(private)
	original = publisher.os.read

	def short_read(fd: int, count: int) -> bytes: return original(fd, min(count, 2))

	monkeypatch.setattr(publisher.os, "read", short_read)
	pending = publisher.pending_from_staging(private, "https://localhost:55000", "b" * 64)
	corpus(tmp_path / "public")
	publisher.publish(tmp_path / "public", pending)
	assert (tmp_path / "public" / contract.ARTIFACTS[0].path).read_bytes() == png()


def test_pending_content_evidence_is_revalidated_before_public_write(tmp_path: pathlib.Path) -> None:
	"""Pending bytes cannot be changed after private receipt validation."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	artifact, _, evidence = pending.artifacts[0]
	invalid = publisher.dataclasses.replace(pending, artifacts=((artifact, png() + b"x", evidence), *pending.artifacts[1:]))
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.publish(tmp_path / "public", invalid)
	assert not (directory / publisher.PROVENANCE_PATH.name).exists()


def test_rollback_failure_is_aggregated_and_leaves_no_temporary_files(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> None:
	"""The original failure and an irrecoverable rollback failure remain observable."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	original = publisher._atomic_write
	calls = 0

	def fail_publish_and_rollback(directory_fd: int, name: str, content: bytes, mode: int, expected: publisher.PublicFileIdentity | None) -> publisher.PublicFileIdentity:
		nonlocal calls
		calls += 1
		if calls in (2, 3): raise OSError("injected irreversible publication failure")
		return original(directory_fd, name, content, mode, expected)

	monkeypatch.setattr(publisher, "_atomic_write", fail_publish_and_rollback)
	with pytest.raises(BaseExceptionGroup) as captured: publisher.publish(tmp_path / "public", pending)
	assert len(captured.value.exceptions) == 2
	assert not list(directory.glob(".*.tmp"))


def test_successful_publication_uses_exact_public_file_mode(tmp_path: pathlib.Path) -> None:
	"""A replacement does not inherit permissive process-umask file permissions."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	publisher.publish(tmp_path / "public", pending)
	assert set(corpus_files(directory)) == {
		str(artifact.path.relative_to(contract.CORPUS_DIRECTORY))
		for artifact in contract.ARTIFACTS
	} | {publisher.PROVENANCE_PATH.name}
	for artifact in contract.ARTIFACTS:
		assert stat.S_IMODE((tmp_path / "public" / artifact.path).stat().st_mode) == 0o644


def test_publication_prunes_a_renamed_prior_generation_path(tmp_path: pathlib.Path) -> None:
	"""A valid prior receipt retires a former canonical path absent from this manifest."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	retired = pathlib.PurePosixPath("docs/screenshots/instructor/roster/02_fresh_session_roster.png")
	target = tmp_path / "public" / retired
	target.parent.mkdir(parents=True, exist_ok=True)
	target.write_bytes(png())
	prior_provenance(directory, pending, retired)
	unrelated = directory / "unrelated.png"
	unrelated.write_bytes(b"keep")
	publisher.publish(tmp_path / "public", pending)
	assert not target.exists()
	assert unrelated.read_bytes() == b"keep"


def test_publication_preserves_unowned_legacy_path(tmp_path: pathlib.Path) -> None:
	"""A completed one-time legacy inventory never grants deletion authority."""
	pending = staging(tmp_path / "private")
	corpus(tmp_path / "public")
	legacy = tmp_path / "public" / "docs" / "screenshots" / "instructor_page_library.png"
	legacy.write_bytes(b"keep")
	publisher.publish(tmp_path / "public", pending)
	assert legacy.read_bytes() == b"keep"


def test_prior_generation_prune_rolls_back_with_the_rest_of_the_generation(
	tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch
) -> None:
	"""A failed prior-owned removal restores its image and exact former provenance."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	retired = pathlib.PurePosixPath("docs/screenshots/instructor/roster/02_fresh_session_roster.png")
	target = tmp_path / "public" / retired
	target.parent.mkdir(parents=True, exist_ok=True)
	target.write_bytes(png())
	prior_provenance(directory, pending, retired)
	prior = corpus_files(directory)
	original = publisher._atomic_remove

	def remove_then_fail(directory_fd: int, name: str, expected: publisher.PublicFileIdentity | None) -> None:
		original(directory_fd, name, expected)
		raise OSError("injected prior-owned removal failure")

	monkeypatch.setattr(publisher, "_atomic_remove", remove_then_fail)
	with pytest.raises(publisher.ScreenshotPublicationError):
		publisher.publish(tmp_path / "public", pending)
	assert corpus_files(directory) == prior


def test_malformed_prior_provenance_fails_closed_before_publication(tmp_path: pathlib.Path) -> None:
	"""An unvalidated receipt never grants deletion authority or starts a write."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	provenance = directory / publisher.PROVENANCE_PATH.name
	provenance.write_bytes(b"not-a-real-stack-receipt")
	provenance.chmod(0o644)
	prior = corpus_files(directory)
	with pytest.raises(publisher.ScreenshotPublicationError):
		publisher.publish(tmp_path / "public", pending)
	assert corpus_files(directory) == prior


def test_prior_generation_requires_the_receipted_no_follow_png(tmp_path: pathlib.Path) -> None:
	"""A path is publisher-owned only while its bounded PNG still matches its receipt."""
	pending = staging(tmp_path / "private")
	directory = corpus(tmp_path / "public")
	retired = pathlib.PurePosixPath("docs/screenshots/instructor/roster/02_fresh_session_roster.png")
	target = tmp_path / "public" / retired
	target.parent.mkdir(parents=True, exist_ok=True)
	target.write_bytes(png())
	prior_provenance(directory, pending, retired)
	target.write_bytes(b"changed-after-receipt")
	prior = corpus_files(directory)
	with pytest.raises(publisher.ScreenshotPublicationError):
		publisher.publish(tmp_path / "public", pending)
	assert corpus_files(directory) == prior


def test_replace_race_preserves_an_unknown_current_target(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> None:
	"""A target replaced between backup and rename remains untouched and fails truthfully."""
	pending = staging(tmp_path / "private")
	corpus(tmp_path / "public")
	original = publisher._atomic_write
	calls = 0

	def replace_before_rename(directory_fd: int, name: str, content: bytes, mode: int, expected: publisher.PublicFileIdentity | None) -> publisher.PublicFileIdentity:
		nonlocal calls
		calls += 1
		if calls == 2:
			fd = publisher.os.open("replacement", publisher.os.O_WRONLY | publisher.os.O_CREAT | publisher.os.O_EXCL, 0o600, dir_fd=directory_fd)
			publisher.os.write(fd, b"unknown-replacement")
			publisher.os.close(fd)
			publisher.os.replace("replacement", name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
		return original(directory_fd, name, content, mode, expected)

	monkeypatch.setattr(publisher, "_atomic_write", replace_before_rename)
	with pytest.raises(publisher.ScreenshotPublicationError): publisher.publish(tmp_path / "public", pending)
	assert (tmp_path / "public" / contract.ARTIFACTS[1].path).read_bytes() == b"unknown-replacement"


def test_rollback_race_preserves_an_unknown_current_target(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> None:
	"""Rollback only restores the exact inode written by this publication attempt."""
	pending = staging(tmp_path / "private")
	corpus(tmp_path / "public")
	original = publisher._atomic_write
	calls = 0

	def fail_then_replace_before_rollback(directory_fd: int, name: str, content: bytes, mode: int, expected: publisher.PublicFileIdentity | None) -> publisher.PublicFileIdentity:
		nonlocal calls
		calls += 1
		if calls == 2: raise OSError("injected publication failure")
		if calls == 3:
			fd = publisher.os.open("replacement", publisher.os.O_WRONLY | publisher.os.O_CREAT | publisher.os.O_EXCL, 0o600, dir_fd=directory_fd)
			publisher.os.write(fd, b"unknown-rollback-replacement")
			publisher.os.close(fd)
			publisher.os.replace("replacement", name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
		return original(directory_fd, name, content, mode, expected)

	monkeypatch.setattr(publisher, "_atomic_write", fail_then_replace_before_rollback)
	with pytest.raises(BaseExceptionGroup): publisher.publish(tmp_path / "public", pending)
	assert (tmp_path / "public" / contract.ARTIFACTS[0].path).read_bytes() == b"unknown-rollback-replacement"


def test_production_dist_digest_distinguishes_path_and_content_boundaries(tmp_path: pathlib.Path) -> None:
	"""Production receipts frame each relative path, size, and content digest."""
	first = tmp_path / "first" / "dist"
	second = tmp_path / "second" / "dist"
	first.mkdir(parents=True)
	second.mkdir(parents=True)
	(first / "a").write_bytes(b"bc")
	(second / "ab").write_bytes(b"c")
	assert publisher.production_dist_digest(first.parent) != publisher.production_dist_digest(second.parent)


def test_production_dist_digest_rejects_links_without_following(
	tmp_path: pathlib.Path,
) -> None:
	"""The delivered production bundle contains only held regular files."""
	dist = tmp_path / "dist"
	dist.mkdir()
	(dist / "index.html").write_text("production", encoding="ascii")
	(dist / "unsafe").symlink_to(dist / "index.html")
	with pytest.raises(publisher.ScreenshotPublicationError):
		publisher.production_dist_digest(tmp_path)
