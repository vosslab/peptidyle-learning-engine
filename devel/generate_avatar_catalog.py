#!/usr/bin/env python3
"""Generate the closed provided-avatar registry from its manifest and SVG sources.

Run ``source source_me.sh && python3 devel/generate_avatar_catalog.py`` to refresh
the three derived registry artifacts.  Add ``--check`` to fail when an artifact
does not exactly match the authoritative catalog inputs.
"""

import argparse
import hashlib
import json
import pathlib
import re
import subprocess

from lxml import etree


CATALOG_RELATIVE_PATH = pathlib.Path("assets/avatar_catalog")
MANIFEST_RELATIVE_PATH = CATALOG_RELATIVE_PATH / "manifest.json"
RUST_RELATIVE_PATH = pathlib.Path("crates/question_model/src/avatar_catalog_generated.rs")
TYPESCRIPT_RELATIVE_PATH = pathlib.Path("src/features/profile_avatar/avatar_catalog_generated.ts")
SQL_RELATIVE_PATH = pathlib.Path("schemas/base_schema/provided_avatar_catalog.sql")
SAFE_AVATAR_ID = re.compile(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*\Z")
SAFE_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
SAFE_XML_ID = re.compile(r"[a-z][a-z0-9-]*\Z")
SAFE_XML_TAGS = frozenset({
	"svg", "g", "rect", "path", "circle", "ellipse", "line", "polygon", "polyline",
})
SAFE_XML_ATTRIBUTES = frozenset({
	"aria-hidden", "cx", "cy", "d", "fill", "focusable", "height", "id", "points",
	"preserveAspectRatio", "r", "rx", "ry", "stroke", "stroke-linecap", "stroke-linejoin",
	"stroke-width", "transform", "viewBox", "width", "x", "x1", "x2", "y", "y1", "y2",
})
EXPECTED_MANIFEST_KEYS = (
	"catalogVersion", "license", "assetChecksumAlgorithm", "checksumInputOrder",
	"assetChecksumInput", "avatars",
)
EXPECTED_AVATAR_KEYS = ("id", "file", "selectable", "name", "description")
EXPECTED_CHECKSUM_INPUT_ORDER = [
	"catalogVersion", "license", "avatars[].id", "avatars[].file", "avatars[].selectable",
	"avatars[].name", "avatars[].description",
]
MAX_AVATARS = 128
MAX_SVG_BYTES = 64 * 1024
MAX_MANIFEST_STRING_LENGTH = 160


#============================================


def repository_root() -> pathlib.Path:
	"""Return the Git worktree root instead of deriving it from a caller cwd."""
	script_root = pathlib.Path(__file__).resolve().parent.parent
	result = subprocess.run(
		["git", "rev-parse", "--show-toplevel"],
		check=True,
		capture_output=True,
		cwd=script_root,
		text=True,
	)
	root = pathlib.Path(result.stdout.strip()).resolve()
	return root


#============================================


def parse_arguments() -> argparse.Namespace:
	"""Read the one deliberately narrow generator operation."""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("--check", action="store_true", help="fail when generated outputs are stale")
	args = parser.parse_args()
	return args


#============================================


def require_ascii_text(value: object, field_name: str, maximum_length: int) -> str:
	"""Return a bounded printable-ASCII manifest string or fail loudly."""
	if not isinstance(value, str) or not value or len(value) > maximum_length:
		raise ValueError(f"{field_name} must be a nonempty string up to {maximum_length} characters")
	if any(ord(character) < 0x20 or ord(character) > 0x7E for character in value):
		raise ValueError(f"{field_name} must contain printable ASCII only")
	return value


#============================================


def require_exact_keys(
	value: object, expected_keys: tuple[str, ...], label: str,
) -> dict[str, object]:
	"""Return an object only when its complete ordered shape is canonical."""
	if not isinstance(value, dict) or tuple(value) != expected_keys:
		raise ValueError(f"{label} must have exactly these ordered keys: {', '.join(expected_keys)}")
	return value


#============================================


def validate_svg(catalog_root: pathlib.Path, avatar_id: str, relative_file: str) -> str:
	"""Validate one bounded, inert SVG and return its exact byte checksum."""
	if relative_file != f"svg/{avatar_id}.svg":
		raise ValueError(f"{avatar_id} must use its matching svg/<id>.svg source path")
	path = catalog_root / relative_file
	resolved_catalog_root = catalog_root.resolve()
	resolved_path = path.resolve()
	if (
		path.is_symlink() or not resolved_path.is_file()
		or resolved_catalog_root not in resolved_path.parents
	):
		raise ValueError(f"{avatar_id} SVG source must be a regular file under the catalog root")
	bytes_value = resolved_path.read_bytes()
	if not bytes_value or len(bytes_value) > MAX_SVG_BYTES:
		raise ValueError(f"{avatar_id} SVG source must contain 1 to {MAX_SVG_BYTES} bytes")
	if b"<!" in bytes_value or b"<?" in bytes_value:
		raise ValueError(f"{avatar_id} SVG source must not declare XML entities, DTDs, or processing")
	parser = etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False, huge_tree=False)
	root = etree.fromstring(bytes_value, parser=parser)
	if (
		etree.QName(root).namespace != "http://www.w3.org/2000/svg"
		or etree.QName(root).localname != "svg"
	):
		raise ValueError(f"{avatar_id} SVG root must be the SVG namespace svg element")
	seen_ids: set[str] = set()
	for element in root.iter():
		if not isinstance(element.tag, str):
			raise ValueError(f"{avatar_id} SVG must not contain comments or processing nodes")
		qualified_name = etree.QName(element)
		if qualified_name.namespace != "http://www.w3.org/2000/svg":
			raise ValueError(f"{avatar_id} SVG elements must use only the SVG namespace")
		qualified_tag = qualified_name.localname
		if qualified_tag not in SAFE_XML_TAGS:
			raise ValueError(f"{avatar_id} SVG uses unsupported element {qualified_tag}")
		for qualified_name, attribute_value in element.attrib.items():
			attribute_name = etree.QName(qualified_name).localname
			if attribute_name not in SAFE_XML_ATTRIBUTES:
				raise ValueError(f"{avatar_id} SVG uses unsupported attribute {attribute_name}")
			if attribute_name.lower().startswith("on") or "url" in attribute_value.lower():
				raise ValueError(f"{avatar_id} SVG contains an active attribute value")
			if any(ord(character) < 0x20 for character in attribute_value):
				raise ValueError(f"{avatar_id} SVG attribute contains a control character")
			if attribute_name == "id":
				if (
					not SAFE_XML_ID.fullmatch(attribute_value)
					or not attribute_value.startswith(avatar_id + "-")
					or attribute_value in seen_ids
				):
					raise ValueError(f"{avatar_id} SVG ids must be unique safe identifiers")
				seen_ids.add(attribute_value)
		if element.text is not None and element.text.strip():
			raise ValueError(f"{avatar_id} SVG must not contain text nodes")
		if element.tail is not None and element.tail.strip():
			raise ValueError(f"{avatar_id} SVG must not contain tail text")
	checksum = hashlib.sha256(bytes_value).hexdigest()
	if not SAFE_SHA256.fullmatch(checksum):
		raise ValueError(f"{avatar_id} SVG checksum generation failed")
	return checksum


#============================================


def load_catalog(root: pathlib.Path) -> list[dict[str, object]]:
	"""Load the exact closed catalog and enrich each validated entry with its checksum."""
	catalog_root = root / CATALOG_RELATIVE_PATH
	manifest_path = root / MANIFEST_RELATIVE_PATH
	manifest_value = json.loads(manifest_path.read_text(encoding="utf-8"))
	manifest = require_exact_keys(manifest_value, EXPECTED_MANIFEST_KEYS, "avatar manifest")
	if manifest["catalogVersion"] != 1:
		raise ValueError("avatar manifest catalogVersion must remain 1 until an explicit migration")
	if manifest["license"] != "CC-BY-4.0":
		raise ValueError("avatar manifest license must be CC-BY-4.0")
	if manifest["assetChecksumAlgorithm"] != "SHA-256":
		raise ValueError("avatar manifest checksum algorithm must be SHA-256")
	if manifest["checksumInputOrder"] != EXPECTED_CHECKSUM_INPUT_ORDER:
		raise ValueError("avatar manifest checksumInputOrder must remain canonical")
	if manifest["assetChecksumInput"] != (
		"The exact UTF-8 bytes of each manifest-relative SVG file, with no newline normalization."
	):
		raise ValueError("avatar manifest assetChecksumInput must remain canonical")
	avatars_value = manifest["avatars"]
	if not isinstance(avatars_value, list) or not avatars_value or len(avatars_value) > MAX_AVATARS:
		raise ValueError(f"avatar manifest must contain 1 to {MAX_AVATARS} avatars")
	entries: list[dict[str, object]] = []
	seen_ids: set[str] = set()
	for index, avatar_value in enumerate(avatars_value):
		avatar = require_exact_keys(avatar_value, EXPECTED_AVATAR_KEYS, f"avatars[{index}]")
		avatar_id = require_ascii_text(avatar["id"], f"avatars[{index}].id", 64)
		if not SAFE_AVATAR_ID.fullmatch(avatar_id) or avatar_id in seen_ids:
			raise ValueError(f"avatars[{index}].id must be a unique lowercase kebab identifier")
		seen_ids.add(avatar_id)
		file_name = require_ascii_text(avatar["file"], f"avatars[{index}].file", 128)
		name = require_ascii_text(avatar["name"], f"avatars[{index}].name", MAX_MANIFEST_STRING_LENGTH)
		description = require_ascii_text(
			avatar["description"], f"avatars[{index}].description", MAX_MANIFEST_STRING_LENGTH,
		)
		if not isinstance(avatar["selectable"], bool):
			raise ValueError(f"avatars[{index}].selectable must be boolean")
		checksum = validate_svg(catalog_root, avatar_id, file_name)
		entries.append({
			"id": avatar_id,
			"file": file_name,
			"selectable": avatar["selectable"],
			"name": name,
			"description": description,
			"asset_sha256": checksum,
		})
	if entries != sorted(entries, key=lambda entry: str(entry["id"])):
		raise ValueError("avatar manifest entries must be ordered by id")
	return entries


#============================================


def rust_string(value: str) -> str:
	"""Render one validated ASCII string as a Rust literal."""
	literal = json.dumps(value, ensure_ascii=True)
	return literal


#============================================


def render_rust(entries: list[dict[str, object]]) -> str:
	"""Render the browser-safe Rust registry with no SVG parsing requirement."""
	lines = [
		"// Generated by devel/generate_avatar_catalog.py. Do not edit by hand.",
		"",
		"/// One closed PLE-provided avatar with exact source-asset evidence.",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub struct ProvidedAvatarCatalogEntry {",
		"    pub id: &'static str,",
		"    pub name: &'static str,",
		"    pub description: &'static str,",
		"    pub asset_path: &'static str,",
		"    pub asset_sha256: &'static str,",
		"    pub is_selectable: bool,",
		"}",
		"",
		"/// The manifest order is canonical and all source bytes are SHA-256 evidenced.",
		"pub const PROVIDED_AVATAR_CATALOG: &[ProvidedAvatarCatalogEntry] = &[",
	]
	for entry in entries:
		asset_path = "/assets/avatar_catalog/" + str(entry["file"])
		lines.extend([
			"    ProvidedAvatarCatalogEntry {",
			f"        id: {rust_string(str(entry['id']))},",
			f"        name: {rust_string(str(entry['name']))},",
			f"        description: {rust_string(str(entry['description']))},",
			f"        asset_path: {rust_string(asset_path)},",
			f"        asset_sha256: {rust_string(str(entry['asset_sha256']))},",
			f"        is_selectable: {str(entry['selectable']).lower()},",
			"    },",
		])
	lines.append("];\n")
	result = "\n".join(lines)
	return result


#============================================


def render_typescript(entries: list[dict[str, object]]) -> str:
	"""Render a typed browser catalog that needs no SVG parser or inline XML."""
	lines = [
		"// Generated by devel/generate_avatar_catalog.py. Do not edit by hand.",
		"",
		"export interface ProvidedAvatarCatalogEntry {",
		"  readonly id: string;",
		"  readonly name: string;",
		"  readonly description: string;",
		"  readonly assetPath: string;",
		"  readonly assetSha256: string;",
		"  readonly isSelectable: boolean;",
		"}",
		"",
		"export const PROVIDED_AVATAR_CATALOG = [",
	]
	for entry in entries:
		asset_path = "/assets/avatar_catalog/" + str(entry["file"])
		lines.extend([
			"  {",
			f"    id: {json.dumps(entry['id'])},",
			f"    name: {json.dumps(entry['name'])},",
			f"    description: {json.dumps(entry['description'])},",
			f"    assetPath: {json.dumps(asset_path)},",
			f"    assetSha256: {json.dumps(entry['asset_sha256'])},",
			f"    isSelectable: {str(entry['selectable']).lower()},",
			"  },",
		])
	lines.extend([
		"] as const satisfies readonly ProvidedAvatarCatalogEntry[];",
		"",
	])
	result = "\n".join(lines)
	return result


#============================================


def render_sql(entries: list[dict[str, object]]) -> str:
	"""Render idempotent catalog rows without deleting retired identifiers."""
	lines = [
		"-- Generated by devel/generate_avatar_catalog.py. Do not edit by hand.",
		"-- This runs after ple_data.provided_avatar exists.",
		"-- Removed manifest IDs remain rows with their recorded checksum and is_selectable = false.",
		"SET LOCAL ROLE ple_data_owner;",
		"INSERT INTO ple_data.provided_avatar",
		"    (provided_avatar_id, asset_sha256, is_selectable)",
		"VALUES",
	]
	for index, entry in enumerate(entries):
		suffix = "," if index + 1 < len(entries) else ""
		lines.append(
			"    ("
			+ repr(str(entry["id"]))
			+ ", "
			+ repr(str(entry["asset_sha256"]))
			+ ", "
			+ str(entry["selectable"]).lower()
			+ ")"
			+ suffix
		)
	lines.extend([
		"ON CONFLICT (provided_avatar_id) DO UPDATE",
		"SET asset_sha256 = EXCLUDED.asset_sha256,",
		"    is_selectable = EXCLUDED.is_selectable",
		"WHERE ple_data.provided_avatar.asset_sha256 IS DISTINCT FROM EXCLUDED.asset_sha256",
		"   OR ple_data.provided_avatar.is_selectable IS DISTINCT FROM EXCLUDED.is_selectable;",
		"",
		"-- Retire omitted catalog IDs without deleting their historical identity or checksum.",
		"UPDATE ple_data.provided_avatar",
		"SET is_selectable = false",
		"WHERE is_selectable",
		"  AND provided_avatar_id NOT IN (",
	])
	for index, entry in enumerate(entries):
		suffix = "," if index + 1 < len(entries) else ""
		lines.append("    " + repr(str(entry["id"])) + suffix)
	lines.extend([
		");",
		"",
	])
	result = "\n".join(lines)
	return result


#============================================


def output_values(entries: list[dict[str, object]]) -> dict[pathlib.Path, str]:
	"""Return every generated artifact from the exact same validated entry list."""
	values = {
		RUST_RELATIVE_PATH: render_rust(entries),
		TYPESCRIPT_RELATIVE_PATH: render_typescript(entries),
		SQL_RELATIVE_PATH: render_sql(entries),
	}
	return values


#============================================


def write_or_check(root: pathlib.Path, outputs: dict[pathlib.Path, str], check: bool) -> None:
	"""Write deterministic artifacts or report every stale output together."""
	stale_paths: list[pathlib.Path] = []
	for relative_path, expected in outputs.items():
		path = root / relative_path
		current = path.read_text(encoding="utf-8") if path.is_file() else None
		if current != expected:
			stale_paths.append(relative_path)
			if not check:
				path.parent.mkdir(parents=True, exist_ok=True)
				path.write_text(expected, encoding="utf-8")
	if check and stale_paths:
		paths = ", ".join(str(path) for path in stale_paths)
		raise ValueError(f"provided-avatar generated artifacts are stale: {paths}")


#============================================


def main() -> None:
	"""Generate or verify all registry projections."""
	args = parse_arguments()
	root = repository_root()
	entries = load_catalog(root)
	outputs = output_values(entries)
	write_or_check(root, outputs, args.check)
	operation = "checked" if args.check else "generated"
	print(
		f"{operation} {len(outputs)} provided-avatar catalog artifacts from "
		+ f"{len(entries)} SVG sources"
	)


if __name__ == "__main__":
	main()
