"""Strict private input ABI for one browser-free live-demo service oracle."""

# Standard Library
import dataclasses
import json
import os
import pathlib
import re
import stat


SCHEMA_VERSION = 1
PRIVATE_INPUT_MAXIMUM_BYTES = 16_384
ORACLE_NAMES = ("webwork_render_rpc", "replica_restart")
ORIGIN_PATTERN = re.compile(r"^https://localhost:([1-9][0-9]{0,4})/$")
EXPECTED_KEYS = {
	"schemaVersion",
	"oracle",
	"baseUrl",
	"manifestPath",
	"seedManifestPath",
	"workspacePath",
}


class LiveDemoServiceInputError(ValueError):
	"""The owner-created private service-oracle input is invalid."""


@dataclasses.dataclass(frozen=True)
class LiveDemoServiceOracleInputV1:
	"""Exact private owner-to-child service-oracle contract."""

	oracle: str
	base_url: str
	manifest_path: pathlib.Path
	seed_manifest_path: pathlib.Path
	workspace_path: pathlib.Path
	schema_version: int = SCHEMA_VERSION

	#============================================
	def as_value(self) -> dict[str, object]:
		"""Return the exact JSON field projection for this ABI version."""
		result: dict[str, object] = {
			"schemaVersion": self.schema_version,
			"oracle": self.oracle,
			"baseUrl": self.base_url,
			"manifestPath": str(self.manifest_path),
			"seedManifestPath": str(self.seed_manifest_path),
			"workspacePath": str(self.workspace_path),
		}
		return result


#============================================
def _require_absolute_owner_path(value: object, label: str) -> pathlib.Path:
	"""Decode one absolute owner-selected path without traversal components."""
	# ASVS 2.2.1 and 5.3.2: accept only the owner's absolute normalized path shape.
	if not isinstance(value, str) or value == "" or "\x00" in value:
		raise LiveDemoServiceInputError(f"service-oracle input {label} is invalid")
	path = pathlib.Path(value)
	if not path.is_absolute() or ".." in path.parts or str(path) != value:
		raise LiveDemoServiceInputError(f"service-oracle input {label} is invalid")
	return path


#============================================
def _require_origin(value: object) -> str:
	"""Require the exact disposable localhost HTTPS origin shape."""
	# ASVS 2.2.1: the child cannot redirect its authenticated requests off-host.
	if not isinstance(value, str):
		raise LiveDemoServiceInputError("service-oracle input baseUrl is invalid")
	match = ORIGIN_PATTERN.fullmatch(value)
	if match is None or int(match.group(1)) > 65_535:
		raise LiveDemoServiceInputError("service-oracle input baseUrl is invalid")
	return value


#============================================
def decode_value(value: object) -> LiveDemoServiceOracleInputV1:
	"""Decode one exact allowlisted V1 JSON value without extension fields."""
	# ASVS 1.5.2 and 15.3.3: JSON may populate only this version's closed fields.
	if not isinstance(value, dict) or set(value) != EXPECTED_KEYS:
		raise LiveDemoServiceInputError("service-oracle input has an invalid shape")
	if type(value["schemaVersion"]) is not int or value["schemaVersion"] != SCHEMA_VERSION:
		raise LiveDemoServiceInputError("service-oracle input schemaVersion is invalid")
	oracle = value["oracle"]
	if not isinstance(oracle, str) or oracle not in ORACLE_NAMES:
		raise LiveDemoServiceInputError("service-oracle input oracle is invalid")
	base_url = _require_origin(value["baseUrl"])
	workspace_path = _require_absolute_owner_path(value["workspacePath"], "workspacePath")
	manifest_path = _require_absolute_owner_path(value["manifestPath"], "manifestPath")
	seed_manifest_path = _require_absolute_owner_path(
		value["seedManifestPath"], "seedManifestPath"
	)
	if manifest_path.parent != workspace_path or seed_manifest_path.parent != workspace_path:
		raise LiveDemoServiceInputError("service-oracle input paths leave the fixed workspace")
	result = LiveDemoServiceOracleInputV1(
		oracle,
		base_url,
		manifest_path,
		seed_manifest_path,
		workspace_path,
	)
	return result


#============================================
def canonical_json(value: LiveDemoServiceOracleInputV1) -> str:
	"""Encode one canonical ASCII JSON object without private-value logging."""
	decoded = decode_value(value.as_value())
	result = json.dumps(decoded.as_value(), separators=(",", ":"), ensure_ascii=True)
	return result


#============================================
def write_private_input(path: pathlib.Path, value: LiveDemoServiceOracleInputV1) -> None:
	"""Create and verify one current-user mode-0600 canonical input file."""
	# ASVS 5.3.2 and 13.3.2: the owner chooses the path and grants only its child read access.
	contents = canonical_json(value).encode("ascii")
	flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
	try:
		file_descriptor = os.open(path, flags, 0o600)
	except OSError as error:
		raise LiveDemoServiceInputError("service-oracle input cannot be created safely") from error
	try:
		with os.fdopen(file_descriptor, "wb") as output:
			output.write(contents)
			output.flush()
			os.fsync(output.fileno())
	except OSError as error:
		raise LiveDemoServiceInputError("service-oracle input cannot be created safely") from error
	read_private_input(path, value.oracle)


#============================================
def _read_checked_bytes(path: pathlib.Path) -> bytes:
	"""Atomically open and read one bounded private regular file."""
	# ASVS 15.4.2: compare path and descriptor identities across the checked open.
	try:
		path_metadata = path.lstat()
		file_descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
	except OSError as error:
		raise LiveDemoServiceInputError("service-oracle input file is unsafe") from error
	try:
		metadata = os.fstat(file_descriptor)
		if (
			stat.S_ISLNK(path_metadata.st_mode)
			or not stat.S_ISREG(metadata.st_mode)
			or path_metadata.st_uid != os.getuid()
			or metadata.st_uid != os.getuid()
			or stat.S_IMODE(path_metadata.st_mode) != 0o600
			or stat.S_IMODE(metadata.st_mode) != 0o600
			or (path_metadata.st_dev, path_metadata.st_ino)
			!= (metadata.st_dev, metadata.st_ino)
			or metadata.st_size < 1
			or metadata.st_size > PRIVATE_INPUT_MAXIMUM_BYTES
		):
			raise LiveDemoServiceInputError("service-oracle input file is unsafe")
		with os.fdopen(file_descriptor, "rb") as source:
			contents = source.read(PRIVATE_INPUT_MAXIMUM_BYTES + 1)
		file_descriptor = -1
	finally:
		if file_descriptor >= 0:
			os.close(file_descriptor)
	if len(contents) > PRIVATE_INPUT_MAXIMUM_BYTES:
		raise LiveDemoServiceInputError("service-oracle input file is unsafe")
	return contents


#============================================
def read_private_input(
	path: pathlib.Path,
	expected_oracle: str | None = None,
) -> LiveDemoServiceOracleInputV1:
	"""Read, strictly decode, and canonicalize one private child input."""
	contents = _read_checked_bytes(path)
	try:
		text = contents.decode("ascii")
		value = json.loads(text)
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise LiveDemoServiceInputError("service-oracle input is not canonical JSON") from error
	result = decode_value(value)
	if expected_oracle is not None and result.oracle != expected_oracle:
		raise LiveDemoServiceInputError("service-oracle input names the wrong child")
	if text != canonical_json(result):
		raise LiveDemoServiceInputError("service-oracle input is not canonical JSON")
	return result
