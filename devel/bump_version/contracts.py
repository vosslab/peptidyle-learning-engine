# Standard Library
import argparse
import re

BASE_VERSION_PATTERN = re.compile(r"^(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)$")
PEP440_PATTERN = re.compile(
	r"^(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)(?P<tag>a|b|rc)(?P<num>\d+)$"
)
SHORT_PEP440_PATTERN = re.compile(
	r"^(?P<major>\d+)\.(?P<minor>\d+)(?P<tag>a|b|rc)(?P<num>\d+)$"
)
DASH_PATTERN = re.compile(
	r"^(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)-(?P<tag>alpha|beta|rc)"
	r"(?:[\\.-]?(?P<num>\d+))?$"
)
YY_MM_PATCH_PATTERN = re.compile(
	r"^(?P<major>\d{2})\.(?P<minor>\d{2})\.(?P<patch>\d+)"
	r"(?:(?P<tag>a|b|rc)(?P<num>\d+))?$"
)
YY_MM_SHORT_PATTERN = re.compile(
	r"^(?P<major>\d{2})\.(?P<minor>\d{2})(?P<tag>a|b|rc)(?P<num>\d+)$"
)
YY_MM_BARE_PATTERN = re.compile(r"^(?P<major>\d{2})\.(?P<minor>\d{2})$")
SIMPLE_VERSION_PATTERN = re.compile(r"\d+\.\d+\.\d+(?:[A-Za-z0-9\.-]+)?")
ASSIGNMENT_PATTERN = re.compile(
	r"^(?P<indent>\s*)(?P<name>__version__|VERSION|version)\s*=\s*"
	r"(?P<quote>['\"])(?P<version>[^'\"]+)(?P=quote)(?P<rest>.*)$"
)
SECTION_HEADER_PATTERN = re.compile(r"^\[(?P<section>[^\]]+)\]\s*$")
VERSION_LINE_PATTERN = re.compile(
	r"^(?P<indent>\s*)version\s*=\s*(?P<quote>['\"])(?P<version>[^'\"]+)(?P=quote)(?P<rest>.*)$"
)
SKIP_DIRS = {
	".git",
	".venv",
	"venv",
	"env",
	"build",
	"dist",
	"__pycache__",
	"node_modules",
	"site-packages",
}
# Skipped only at the repo root, matching the root-anchored /OTHER_REPOS/ rule in
# templates/gitignore.universal. Vendored sibling repos there carry their own
# unrelated versions and must never be rewritten by a bump of this repo.
ROOT_SKIP_DIRS = {
	"OTHER_REPOS",
}
CANDIDATE_FILENAMES = {
	"Cargo.lock",
	"Cargo.toml",
	"pyproject.toml",
	"VERSION",
	"version",
	"version.txt",
	"version.py",
}
CARGO_PACKAGE_HEADER_PATTERN = re.compile(r"^\[\[package\]\]\s*$")
CARGO_NAME_PATTERN = re.compile(r"^\s*name\s*=\s*['\"](?P<name>[^'\"]+)['\"]\s*$")
# Prerelease tag vocabularies, one per direction. Previously rebuilt inside
# parse_version_details (twice), format_version, and normalize_cargo_version.
# PRE_TAG_NAMES: PEP 440 short tag as written -> internal long name.
PRE_TAG_NAMES = {
	"a": "alpha",
	"b": "beta",
	"rc": "rc",
}
# PRE_TAG_SHORT: internal long name -> PEP 440 short tag (the inverse).
PRE_TAG_SHORT = {
	"alpha": "a",
	"beta": "b",
	"rc": "rc",
}
# CARGO_PRE_TAG_NAMES: Cargo emits long names and accepts either spelling,
# since parse_version_details keeps dash-style tags in their source form.
CARGO_PRE_TAG_NAMES = {
	"a": "alpha",
	"alpha": "alpha",
	"b": "beta",
	"beta": "beta",
	"rc": "rc",
}
SHORT_BUMP_ALIASES = {
	"M": "major",
	"m": "minor",
	"p": "patch",
	"a": "alpha",
	"b": "beta",
	"r": "rc",
}
ADVANCED_HELP = argparse.SUPPRESS
