"""Validate a private visual bundle and atomically publish one durable generation."""

import dataclasses
import hashlib
import json
import os
import pathlib
import stat
import zlib

import e2e_browser_screenshot_contract

MAXIMUM_PNG_BYTES = 8_000_000
MAXIMUM_RECEIPT_BYTES = 2_048
MAXIMUM_PUBLIC_BYTES = 8_000_000
MAXIMUM_PROVENANCE_BYTES = 256_000
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
PROVENANCE_PATH = pathlib.PurePosixPath("docs/screenshots/corpus_provenance.json")
class ScreenshotPublicationError(ValueError):
	"""A visual bundle cannot become the public real-stack corpus."""


@dataclasses.dataclass(frozen=True)
class PublicFileIdentity:
	"""One no-follow regular-file identity held across a publication transition."""
	device: int
	inode: int


@dataclasses.dataclass
class PublicationChange:
	"""The prior and written identity for one transactional public member."""
	directory_fd: int
	name: str
	prior: bytes | None
	prior_identity: PublicFileIdentity | None
	written_identity: PublicFileIdentity | None = None
	removed: bool = False


@dataclasses.dataclass(frozen=True)
class ScreenshotArtifactEvidence:
	"""Safe per-scenario capture evidence."""
	artifact_id: str
	scenario_id: str
	role: str
	journey: str
	capture_order: int
	journey_step: int
	viewport: str
	digest: str
	width: int
	height: int
	captured: bool
	def as_value(self) -> dict[str, object]: return dataclasses.asdict(self)


@dataclasses.dataclass(frozen=True)
class PendingScreenshotPublication:
	"""Private bounded bytes held only until the outer reset succeeds."""
	artifacts: tuple[tuple[e2e_browser_screenshot_contract.ScreenshotArtifact, bytes, ScreenshotArtifactEvidence], ...]
	origin: str
	production_dist_digest: str
	generation_identity: str


@dataclasses.dataclass(frozen=True)
class ScreenshotEvidence:
	"""Public completion facts without staging paths or image bytes."""
	requested: bool
	artifact_ids: tuple[str, ...]
	digests: tuple[str, ...]
	all_artifacts_captured: bool
	png_validation_completed: bool
	privacy_validation_completed: bool
	publication_committed: bool
	def as_value(self) -> dict[str, object]:
		value = {"requested": self.requested, "artifactIds": self.artifact_ids, "digests": self.digests, "allArtifactsCaptured": self.all_artifacts_captured, "pngValidationCompleted": self.png_validation_completed, "privacyValidationCompleted": self.privacy_validation_completed, "publicationCommitted": self.publication_committed}
		return value


def normalize_https_gateway_origin(value: str) -> str:
	"""Return the canonical gateway origin shared by receipts and provenance."""
	from urllib.parse import urlsplit
	parsed = urlsplit(value)
	if parsed.scheme != "https" or parsed.hostname != "localhost" or parsed.port is None or parsed.path not in ("", "/") or parsed.query or parsed.fragment or parsed.username or parsed.password:
		raise ScreenshotPublicationError("screenshot origin is invalid")
	return f"https://localhost:{parsed.port}"


def production_dist_digest(root: pathlib.Path) -> str:
	"""Digest the complete regular-file production bundle without Git metadata."""
	dist = root / "dist"
	if not dist.is_dir() or dist.is_symlink(): raise ScreenshotPublicationError("production dist is unavailable")
	# ASVS 11.4.1: SHA-256 binds the delivered production files to the receipt.
	digest = hashlib.sha256()
	items = sorted(dist.rglob("*"))
	for item in items:
		if item.is_symlink(): raise ScreenshotPublicationError("production dist has an unsafe artifact")
	files = [item for item in items if item.is_file()]
	if not files: raise ScreenshotPublicationError("production dist is empty")
	for item in files:
		metadata = item.lstat()
		if item.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 32_000_000: raise ScreenshotPublicationError("production dist has an unsafe artifact")
		digest.update(_framed_dist_file_digest(item.relative_to(dist).as_posix(), item))
	return digest.hexdigest()


def pending_from_staging(
	staging: pathlib.Path,
	origin: str,
	production_dist_digest: str,
) -> PendingScreenshotPublication:
	"""Read declared artifacts through one held private-directory descriptor."""
	origin = normalize_https_gateway_origin(origin)
	_require_hex(production_dist_digest, 64, "production dist digest")
	directory_fd = _open_staging(staging)
	try:
		expected = {f"{item.artifact_id}.{suffix}" for item in e2e_browser_screenshot_contract.ARTIFACTS for suffix in ("png", "json")}
		if set(os.listdir(directory_fd)) != expected: raise ScreenshotPublicationError("private screenshot staging has unexpected artifacts")
		entries = []
		for artifact in e2e_browser_screenshot_contract.ARTIFACTS:
			content = _read_private(directory_fd, f"{artifact.artifact_id}.png", MAXIMUM_PNG_BYTES)
			width, height = _validate_png(content, artifact)
			digest = hashlib.sha256(content).hexdigest()
			receipt = json.loads(_read_private(directory_fd, f"{artifact.artifact_id}.json", MAXIMUM_RECEIPT_BYTES).decode("ascii"))
			_expected_receipt(receipt, artifact, digest, width, height, origin)
			entries.append((artifact, content, _artifact_evidence(artifact, digest, width, height)))
	except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
		raise ScreenshotPublicationError("private screenshot staging is invalid") from error
	finally: os.close(directory_fd)
	generation = _generation_identity(origin, production_dist_digest, entries)
	return PendingScreenshotPublication(tuple(entries), origin, production_dist_digest, generation)


def evidence_for(pending: PendingScreenshotPublication, committed: bool = False) -> ScreenshotEvidence:
	"""Project complete capture facts after browser-side privacy checks."""
	return ScreenshotEvidence(True, tuple(item[0].artifact_id for item in pending.artifacts), tuple(item[2].digest for item in pending.artifacts), True, True, True, committed)


def publish(root: pathlib.Path, pending: PendingScreenshotPublication) -> ScreenshotEvidence:
	"""Atomically replace the corpus and retire only recorded publisher-owned paths."""
	_validate_pending(pending)
	corpus_fd = _open_corpus(root / "docs" / "screenshots")
	directories: dict[tuple[str, ...], int] = {}
	changes: list[PublicationChange] = []
	try:
		prior_provenance, provenance_identity = _existing_bytes(corpus_fd, PROVENANCE_PATH.name)
		prior_targets = _prior_generation_targets(corpus_fd, prior_provenance)
		directories = _open_contract_directories(corpus_fd)
		for artifact, content, _ in pending.artifacts:
			name = artifact.path.name
			directory_fd = directories.get(_artifact_parent_parts(artifact), corpus_fd)
			prior, identity = _existing_bytes(directory_fd, name)
			change = PublicationChange(directory_fd, name, prior, identity)
			changes.append(change)
			change.written_identity = _atomic_write(directory_fd, name, content, 0o644, identity)
		name = PROVENANCE_PATH.name
		prior, identity = prior_provenance, provenance_identity
		change = PublicationChange(corpus_fd, name, prior, identity)
		changes.append(change)
		provenance = json.dumps(_provenance_value(pending), sort_keys=True, indent=2).encode("ascii") + b"\n"
		change.written_identity = _atomic_write(corpus_fd, name, provenance, 0o644, identity)
		for retired_path in _prior_generation_retired_targets(prior_targets):
			parts = _path_parent_parts(retired_path)
			directory_fd = corpus_fd if not parts else directories.get(parts)
			if directory_fd is None:
				directory_fd = _open_existing_contract_directory(corpus_fd, parts)
				if directory_fd is None:
					continue
				directories[parts] = directory_fd
			name = retired_path.name
			prior, identity = _existing_bytes(directory_fd, name)
			if prior is None:
				continue
			change = PublicationChange(directory_fd, name, prior, identity)
			changes.append(change)
			try:
				_atomic_remove(directory_fd, name, identity)
			except BaseException:
				change.removed = _current_public_identity(directory_fd, name) is None
				raise
			else:
				change.removed = True
	except BaseException as error:
		rollback_errors = _rollback(changes)
		if rollback_errors: raise BaseExceptionGroup("screenshot publication and rollback failures", [error, *rollback_errors])
		raise ScreenshotPublicationError("screenshot publication did not commit") from error
	finally:
		for directory_fd in reversed(tuple(directories.values())):
			os.close(directory_fd)
		os.close(corpus_fd)
	return evidence_for(pending, committed=True)


def _open_staging(path: pathlib.Path) -> int:
	metadata = path.lstat()
	if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or metadata.st_uid != os.getuid() or (metadata.st_mode & 0o777) != 0o700: raise ScreenshotPublicationError("private screenshot staging is unsafe")
	fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
	_verify_held_directory(fd, metadata, 0o700, "private screenshot staging")
	return fd


def _open_corpus(path: pathlib.Path) -> int:
	metadata = path.lstat()
	if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or metadata.st_uid != os.getuid() or metadata.st_mode & 0o022: raise ScreenshotPublicationError("screenshots directory is unsafe")
	fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
	_verify_held_directory(fd, metadata, metadata.st_mode & 0o777, "screenshots directory")
	return fd


def _artifact_parent_parts(artifact: e2e_browser_screenshot_contract.ScreenshotArtifact) -> tuple[str, ...]:
	"""Return the closed contract-relative directory for one public artifact."""
	return _path_parent_parts(artifact.path)


def _path_parent_parts(path: pathlib.PurePosixPath) -> tuple[str, ...]:
	"""Return one manifest-derived path parent without accepting caller paths."""
	try:
		relative = path.relative_to(e2e_browser_screenshot_contract.CORPUS_DIRECTORY)
	except ValueError as error:
		raise ScreenshotPublicationError("screenshot path is outside the stable corpus") from error
	parts = relative.parent.parts
	if any(part in ("", ".", "..") for part in parts):
		raise ScreenshotPublicationError("screenshot path has an unsafe parent")
	return parts


def _open_contract_directories(corpus_fd: int) -> dict[tuple[str, ...], int]:
	"""Hold the complete closed nested public layout for one generation."""
	parents = {_path_parent_parts(artifact.path) for artifact in e2e_browser_screenshot_contract.ARTIFACTS}
	parents.discard(())
	directories: dict[tuple[str, ...], int] = {}
	try:
		for parent in sorted(parents, key=lambda value: (len(value), value)):
			prefix: tuple[str, ...] = ()
			parent_fd = corpus_fd
			for component in parent:
				prefix += (component,)
				if prefix not in directories:
					directories[prefix] = _open_or_create_public_directory(parent_fd, component)
				parent_fd = directories[prefix]
	except BaseException:
		for directory_fd in reversed(tuple(directories.values())):
			os.close(directory_fd)
		raise
	return directories


def _open_existing_contract_directory(corpus_fd: int, parts: tuple[str, ...]) -> int | None:
	"""Open an existing prior-generation parent without recreating retired folders."""
	if not parts:
		return None
	parent_fd = corpus_fd
	opened: list[int] = []
	try:
		for component in parts:
			try:
				metadata = os.stat(component, dir_fd=parent_fd, follow_symlinks=False)
			except FileNotFoundError:
				return None
			if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or metadata.st_uid != os.getuid() or metadata.st_mode & 0o022:
				raise ScreenshotPublicationError("prior screenshot parent directory is unsafe")
			current = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent_fd)
			_verify_held_directory(current, metadata, metadata.st_mode & 0o777, "prior screenshot parent directory")
			opened.append(current)
			parent_fd = current
		result = opened.pop()
		return result
	finally:
		for directory_fd in reversed(opened):
			os.close(directory_fd)


def _open_or_create_public_directory(parent_fd: int, name: str) -> int:
	"""Open or create one contract-named owner-safe public directory."""
	try:
		metadata = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
	except FileNotFoundError:
		try: os.mkdir(name, 0o755, dir_fd=parent_fd)
		except FileExistsError: pass
		metadata = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
	if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or metadata.st_uid != os.getuid() or metadata.st_mode & 0o022:
		raise ScreenshotPublicationError("screenshot parent directory is unsafe")
	fd = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent_fd)
	_verify_held_directory(fd, metadata, metadata.st_mode & 0o777, "screenshot parent directory")
	return fd


def _verify_held_directory(fd: int, expected: os.stat_result, mode: int, label: str) -> None:
	metadata = os.fstat(fd)
	if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != os.getuid() or (metadata.st_mode & 0o777) != mode or (metadata.st_dev, metadata.st_ino) != (expected.st_dev, expected.st_ino): raise ScreenshotPublicationError(f"{label} changed while opening")


def _read_private(directory_fd: int, name: str, maximum: int) -> bytes:
	fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
	try:
		metadata = os.fstat(fd)
		if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.getuid() or (metadata.st_mode & 0o777) != 0o600 or not 0 < metadata.st_size <= maximum: raise ScreenshotPublicationError("private screenshot artifact is unsafe")
		content = _read_exact(fd, metadata.st_size, "private screenshot artifact")
		return content
	finally: os.close(fd)


def _validate_png(
	content: bytes,
	artifact: e2e_browser_screenshot_contract.ScreenshotArtifact | None = None,
) -> tuple[int, int]:
	"""Validate PNG framing and, when declared, its manifest viewport dimensions."""
	if len(content) > MAXIMUM_PNG_BYTES or not content.startswith(PNG_SIGNATURE): raise ScreenshotPublicationError("screenshot artifact is not a PNG")
	offset, ihdr, idat, terminal, width, height = len(PNG_SIGNATURE), 0, 0, False, 0, 0
	while offset < len(content):
		if terminal or len(content) - offset < 12: raise ScreenshotPublicationError("screenshot artifact has invalid PNG framing")
		length = int.from_bytes(content[offset:offset + 4], "big")
		end = offset + 12 + length
		if end > len(content): raise ScreenshotPublicationError("screenshot artifact has invalid PNG framing")
		kind = content[offset + 4:offset + 8]
		data = content[offset + 8:offset + 8 + length]
		crc = int.from_bytes(content[offset + 8 + length:end], "big")
		if zlib.crc32(kind + data) & 0xffffffff != crc: raise ScreenshotPublicationError("screenshot artifact has invalid PNG CRC")
		if kind == b"IHDR":
			ihdr += 1
			if ihdr != 1 or offset != len(PNG_SIGNATURE) or length != 13: raise ScreenshotPublicationError("screenshot artifact has invalid IHDR")
			width, height = int.from_bytes(data[:4], "big"), int.from_bytes(data[4:8], "big")
		elif kind == b"IDAT": idat += 1
		elif kind == b"IEND":
			if length != 0 or end != len(content): raise ScreenshotPublicationError("screenshot artifact has invalid IEND")
			terminal = True
		offset = end
	if ihdr != 1 or idat < 1 or not terminal:
		raise ScreenshotPublicationError("screenshot artifact has invalid PNG structure")
	if artifact is not None:
		viewport = e2e_browser_screenshot_contract.VIEWPORT_PROFILES[artifact.viewport]
		if (width, height) != (viewport.width, viewport.height):
			raise ScreenshotPublicationError("screenshot artifact has invalid dimensions")
	return width, height


def _expected_receipt(value: object, artifact: e2e_browser_screenshot_contract.ScreenshotArtifact, digest: str, width: int, height: int, origin: str) -> None:
	checks = privacy_checks(artifact)
	expected = {"artifactId": artifact.artifact_id, "scenarioId": artifact.scenario_id, "stateId": artifact.state_id, "sha256": digest, "viewport": artifact.viewport, "width": width, "height": height, "origin": origin, "privacyValidated": True, "privacyChecks": checks}
	if value != expected: raise ScreenshotPublicationError("screenshot receipt does not match the declared capture")


def privacy_checks(artifact: e2e_browser_screenshot_contract.ScreenshotArtifact) -> list[str]:
	"""Return the manifest-declared privacy evidence for one visible capture state."""
	return list(artifact.privacy_checks)


def _existing_bytes(directory_fd: int, name: str) -> tuple[bytes | None, PublicFileIdentity | None]:
	metadata = _current_public_identity(directory_fd, name)
	if metadata is None: return None, None
	fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
	try:
		opened = os.fstat(fd)
		if (opened.st_dev, opened.st_ino, opened.st_size) != (metadata.st_dev, metadata.st_ino, metadata.st_size): raise ScreenshotPublicationError("existing screenshot target changed while opening")
		if opened.st_size > MAXIMUM_PUBLIC_BYTES: raise ScreenshotPublicationError("existing screenshot target is too large")
		content = _read_exact(fd, opened.st_size, "existing screenshot target")
		return content, PublicFileIdentity(opened.st_dev, opened.st_ino)
	finally: os.close(fd)


def _atomic_write(directory_fd: int, name: str, content: bytes, mode: int, expected: PublicFileIdentity | None) -> PublicFileIdentity:
	temporary = f".{name}.{os.getpid()}.{os.urandom(8).hex()}.tmp"
	fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode, dir_fd=directory_fd)
	try:
		os.fchmod(fd, mode)
		# ASVS 2.2.1: persist the complete bounded member before replacement.
		written = 0
		while written < len(content):
			count = os.write(fd, content[written:])
			if count == 0: raise OSError("screenshot temporary write stopped")
			written += count
		os.fsync(fd)
		os.close(fd)
		fd = -1
		if _identity_of(_current_public_identity(directory_fd, name)) != expected: raise ScreenshotPublicationError("screenshot target changed before replacement")
		os.replace(temporary, name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
		os.fsync(directory_fd)
		metadata = _current_public_identity(directory_fd, name)
		if metadata is None: raise ScreenshotPublicationError("screenshot replacement disappeared")
		identity = _identity_of(metadata)
		if identity is None: raise ScreenshotPublicationError("screenshot replacement is invalid")
		return identity
	except BaseException:
		if fd >= 0: os.close(fd)
		try: os.unlink(temporary, dir_fd=directory_fd)
		except FileNotFoundError: pass
		raise


def _atomic_remove(directory_fd: int, name: str, expected: PublicFileIdentity | None) -> None:
	"""Remove one prior-generation member only when its inode is still held."""
	if expected is None or _identity_of(_current_public_identity(directory_fd, name)) != expected:
		raise ScreenshotPublicationError("prior screenshot target changed before removal")
	os.unlink(name, dir_fd=directory_fd)
	os.fsync(directory_fd)
	if _current_public_identity(directory_fd, name) is not None:
		raise ScreenshotPublicationError("prior screenshot target survived removal")


def _rollback(changes: list[PublicationChange]) -> list[BaseException]:
	errors = []
	for change in reversed(changes):
		try:
			current = _identity_of(_current_public_identity(change.directory_fd, change.name))
			if change.removed:
				if current is not None: raise ScreenshotPublicationError("screenshot target changed before rollback")
				if change.prior is None: raise ScreenshotPublicationError("screenshot rollback has no removed target")
				_atomic_write(change.directory_fd, change.name, change.prior, 0o644, None)
				continue
			if change.written_identity is None: continue
			if current != change.written_identity: raise ScreenshotPublicationError("screenshot target changed before rollback")
			if change.prior is None:
				try: os.unlink(change.name, dir_fd=change.directory_fd)
				except FileNotFoundError: pass
				os.fsync(change.directory_fd)
			else: _atomic_write(change.directory_fd, change.name, change.prior, 0o644, change.written_identity)
		except BaseException as error: errors.append(error)
	return errors


def _current_public_identity(directory_fd: int, name: str) -> os.stat_result | None:
	"""Resolve only a safe regular public member without following a link."""
	try: metadata = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
	except FileNotFoundError: return None
	if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.getuid() or metadata.st_mode & 0o022: raise ScreenshotPublicationError("existing screenshot target is unsafe")
	return metadata


def _identity_of(metadata: os.stat_result | None) -> PublicFileIdentity | None:
	if metadata is None: return None
	identity = PublicFileIdentity(metadata.st_dev, metadata.st_ino)
	return identity


def _artifact_evidence(
	artifact: e2e_browser_screenshot_contract.ScreenshotArtifact,
	digest: str,
	width: int,
	height: int,
) -> ScreenshotArtifactEvidence:
	"""Project the complete manifest identity carried by one private capture."""
	return ScreenshotArtifactEvidence(
		artifact.artifact_id,
		artifact.scenario_id,
		artifact.role,
		artifact.journey,
		artifact.capture_order,
		artifact.journey_step,
		artifact.viewport,
		digest,
		width,
		height,
		True,
	)


def _prior_generation_retired_targets(prior_targets: tuple[pathlib.PurePosixPath, ...]) -> tuple[pathlib.PurePosixPath, ...]:
	"""Return only prior-generation-owned paths absent from the new corpus."""
	canonical = {artifact.path for artifact in e2e_browser_screenshot_contract.ARTIFACTS}
	return tuple(dict.fromkeys(path for path in prior_targets if path not in canonical))


def _prior_generation_targets(corpus_fd: int, provenance: bytes | None) -> tuple[pathlib.PurePosixPath, ...]:
	"""Validate one held provenance receipt before it grants bounded retirement ownership."""
	if provenance is None:
		return ()
	if len(provenance) > MAXIMUM_PROVENANCE_BYTES:
		raise ScreenshotPublicationError("prior screenshot provenance is too large")
	try:
		value = json.loads(provenance.decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid") from error
	if not isinstance(value, dict) or set(value) != {
		"schemaVersion", "pipeline", "browserSuite", "origin", "productionDistDigest",
		"generationIdentity", "artifacts",
	}:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	if value["schemaVersion"] != 2 or value["pipeline"] != "realStack" or value["browserSuite"] != "ple-live-demo-browser":
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	if not isinstance(value["origin"], str) or normalize_https_gateway_origin(value["origin"]) != value["origin"]:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	if not isinstance(value["productionDistDigest"], str) or not isinstance(value["generationIdentity"], str):
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	_require_hex(value["productionDistDigest"], 64, "prior screenshot provenance")
	_require_hex(value["generationIdentity"], 64, "prior screenshot provenance")
	records = value["artifacts"]
	if not isinstance(records, list) or not 1 <= len(records) <= 256:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	paths: list[pathlib.PurePosixPath] = []
	identifiers: set[str] = set()
	orders: set[int] = set()
	for record in records:
		path, artifact_id, capture_order = _validate_prior_provenance_record(record)
		if path in paths or artifact_id in identifiers or capture_order in orders:
			raise ScreenshotPublicationError("prior screenshot provenance is invalid")
		paths.append(path)
		identifiers.add(artifact_id)
		orders.add(capture_order)
	if _prior_generation_identity(value["origin"], value["productionDistDigest"], records) != value["generationIdentity"]:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	for path, record in zip(paths, records, strict=True):
		assert isinstance(record, dict)
		_validate_prior_public_artifact(corpus_fd, path, record["sha256"], record["width"], record["height"])
	return tuple(paths)


def _validate_prior_public_artifact(
	corpus_fd: int,
	path: pathlib.PurePosixPath,
	digest: object,
	width: object,
	height: object,
) -> None:
	"""Bind prior deletion authority to one held regular PNG and its receipt digest."""
	if not isinstance(digest, str) or isinstance(width, bool) or not isinstance(width, int) or isinstance(height, bool) or not isinstance(height, int):
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	parts = _path_parent_parts(path)
	directory_fd = _open_existing_contract_directory(corpus_fd, parts)
	if directory_fd is None:
		raise ScreenshotPublicationError("prior screenshot provenance is incomplete")
	try:
		content, _ = _existing_bytes(directory_fd, path.name)
	finally:
		os.close(directory_fd)
	if content is None:
		raise ScreenshotPublicationError("prior screenshot provenance is incomplete")
	actual_width, actual_height = _validate_png(content)
	if (actual_width, actual_height) != (width, height) or hashlib.sha256(content).hexdigest() != digest:
		raise ScreenshotPublicationError("prior screenshot provenance is incomplete")


def _validate_prior_provenance_record(record: object) -> tuple[pathlib.PurePosixPath, str, int]:
	"""Validate the complete stable shape of one older publisher artifact receipt."""
	if not isinstance(record, dict) or set(record) != {
		"artifactId", "scenarioId", "stateId", "role", "journey", "captureOrder",
		"journeyStep", "viewport", "path", "sha256", "width", "height", "privacyChecks",
	}:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	text_keys = ("artifactId", "scenarioId", "stateId", "role", "journey", "path", "sha256")
	if any(not isinstance(record[key], str) or not record[key].isascii() or not record[key] for key in text_keys):
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	if any(isinstance(record[key], bool) or not isinstance(record[key], int) or record[key] < 1 for key in ("captureOrder", "journeyStep", "width", "height")):
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	_require_hex(record["sha256"], 64, "prior screenshot provenance")
	try:
		path = pathlib.PurePosixPath(record["path"])
		parts = _path_parent_parts(path)
	except (TypeError, ValueError) as error:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid") from error
	if path.suffix != ".png" or not parts or path == PROVENANCE_PATH:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	viewport = record["viewport"]
	if not isinstance(viewport, dict) or set(viewport) != {"name", "width", "height", "deviceScaleFactor"}:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	if not isinstance(viewport["name"], str) or viewport["name"] not in e2e_browser_screenshot_contract.VIEWPORT_PROFILES:
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	profile = e2e_browser_screenshot_contract.VIEWPORT_PROFILES[viewport["name"]]
	if viewport != {"name": viewport["name"], "width": profile.width, "height": profile.height, "deviceScaleFactor": profile.device_scale_factor} or (record["width"], record["height"]) != (profile.width, profile.height):
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	checks = record["privacyChecks"]
	if not isinstance(checks, list) or not checks or checks[0] != "no_private_material" or any(not isinstance(check, str) or not check.isascii() for check in checks) or len(checks) != len(set(checks)):
		raise ScreenshotPublicationError("prior screenshot provenance is invalid")
	return path, record["artifactId"], record["captureOrder"]


def _prior_generation_identity(origin: str, dist_digest: str, records: list[object]) -> str:
	"""Recompute the generation digest that binds a held prior ownership receipt."""
	artifacts = [
		{
			"artifactId": record["artifactId"], "captureOrder": record["captureOrder"],
			"path": record["path"], "sha256": record["sha256"], "viewport": record["viewport"],
		}
		for record in records
		if isinstance(record, dict)
	]
	value = json.dumps({"artifacts": artifacts, "origin": origin, "productionDistDigest": dist_digest}, separators=(",", ":"), sort_keys=True)
	return hashlib.sha256(value.encode("ascii")).hexdigest()


def _generation_record(
	artifact: e2e_browser_screenshot_contract.ScreenshotArtifact,
	evidence: ScreenshotArtifactEvidence,
) -> dict[str, object]:
	"""Return the ordered public identity bound into a generation digest."""
	viewport = e2e_browser_screenshot_contract.VIEWPORT_PROFILES[artifact.viewport]
	return {
		"artifactId": artifact.artifact_id,
		"captureOrder": artifact.capture_order,
		"path": str(artifact.path),
		"sha256": evidence.digest,
		"viewport": {"name": artifact.viewport, "width": viewport.width, "height": viewport.height, "deviceScaleFactor": viewport.device_scale_factor},
	}


def _provenance_record(
	artifact: e2e_browser_screenshot_contract.ScreenshotArtifact,
	evidence: ScreenshotArtifactEvidence,
) -> dict[str, object]:
	"""Return one complete artifact receipt from the manifest and held PNG bytes."""
	viewport = e2e_browser_screenshot_contract.VIEWPORT_PROFILES[artifact.viewport]
	return {
		"artifactId": artifact.artifact_id,
		"scenarioId": artifact.scenario_id,
		"stateId": artifact.state_id,
		"role": artifact.role,
		"journey": artifact.journey,
		"captureOrder": artifact.capture_order,
		"journeyStep": artifact.journey_step,
		"viewport": {"name": artifact.viewport, "width": viewport.width, "height": viewport.height, "deviceScaleFactor": viewport.device_scale_factor},
		"path": str(artifact.path),
		"sha256": evidence.digest,
		"width": evidence.width,
		"height": evidence.height,
		"privacyChecks": privacy_checks(artifact),
	}


def _validate_pending(pending: PendingScreenshotPublication) -> None:
	# ASVS 2.2.1: the pending value must match the closed publication contract.
	if tuple(item[0] for item in pending.artifacts) != e2e_browser_screenshot_contract.ARTIFACTS: raise ScreenshotPublicationError("screenshot publication has an invalid artifact contract")
	normalize_https_gateway_origin(pending.origin)
	_require_hex(pending.production_dist_digest, 64, "screenshot provenance")
	_require_hex(pending.generation_identity, 64, "screenshot provenance")
	for artifact, content, evidence in pending.artifacts:
		if not isinstance(content, bytes): raise ScreenshotPublicationError("screenshot artifact content is invalid")
		width, height = _validate_png(content, artifact)
		digest = hashlib.sha256(content).hexdigest()
		if evidence != _artifact_evidence(artifact, digest, width, height): raise ScreenshotPublicationError("screenshot artifact evidence is invalid")
	if pending.generation_identity != _generation_identity(pending.origin, pending.production_dist_digest, list(pending.artifacts)): raise ScreenshotPublicationError("screenshot provenance generation is invalid")


def _require_hex(value: str, length: int, label: str) -> None:
	if len(value) != length or any(character not in "0123456789abcdef" for character in value): raise ScreenshotPublicationError(f"{label} is invalid")


def _read_exact(fd: int, size: int, label: str) -> bytes:
	"""Read exactly one bounded regular-file snapshot, rejecting concurrent growth."""
	parts = []
	remaining = size
	while remaining:
		block = os.read(fd, min(1_048_576, remaining))
		if not block: raise ScreenshotPublicationError(f"{label} changed while reading")
		parts.append(block)
		remaining -= len(block)
	if os.read(fd, 1): raise ScreenshotPublicationError(f"{label} changed while reading")
	value = b"".join(parts)
	return value


def _framed_dist_file_digest(relative: str, item: pathlib.Path) -> bytes:
	"""Hash one regular dist file with unambiguous path and length framing."""
	metadata = item.lstat()
	if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or metadata.st_size > 32_000_000: raise ScreenshotPublicationError("production dist has an unsafe artifact")
	fd = os.open(item, os.O_RDONLY | os.O_NOFOLLOW)
	try:
		opened = os.fstat(fd)
		if (opened.st_dev, opened.st_ino, opened.st_mode, opened.st_size) != (metadata.st_dev, metadata.st_ino, metadata.st_mode, metadata.st_size): raise ScreenshotPublicationError("production dist changed while opening")
		digest = hashlib.sha256(_read_exact(fd, opened.st_size, "production dist")).digest()
		after_open = os.fstat(fd)
		after_path = item.lstat()
		if (after_open.st_dev, after_open.st_ino, after_open.st_mode, after_open.st_size) != (opened.st_dev, opened.st_ino, opened.st_mode, opened.st_size) or (after_path.st_dev, after_path.st_ino, after_path.st_mode, after_path.st_size) != (metadata.st_dev, metadata.st_ino, metadata.st_mode, metadata.st_size): raise ScreenshotPublicationError("production dist changed while reading")
	finally: os.close(fd)
	path = relative.encode("utf-8")
	value = path + b"\0F" + metadata.st_size.to_bytes(8, "big") + digest
	return value


def _generation_identity(origin: str, production_dist_digest: str, entries: list[tuple[object, bytes, ScreenshotArtifactEvidence]] | tuple[tuple[e2e_browser_screenshot_contract.ScreenshotArtifact, bytes, ScreenshotArtifactEvidence], ...]) -> str:
	value = json.dumps({"artifacts": [_generation_record(item[0], item[2]) for item in entries], "origin": origin, "productionDistDigest": production_dist_digest}, separators=(",", ":"), sort_keys=True)
	return hashlib.sha256(value.encode("ascii")).hexdigest()


def _provenance_value(pending: PendingScreenshotPublication) -> dict[str, object]:
	return {"schemaVersion": 2, "pipeline": "realStack", "browserSuite": "ple-live-demo-browser", "origin": pending.origin, "productionDistDigest": pending.production_dist_digest, "generationIdentity": pending.generation_identity, "artifacts": [_provenance_record(item[0], item[2]) for item in pending.artifacts]}
