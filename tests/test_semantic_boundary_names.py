"""Forbid production-boundary domain ETag names.

HTTP ``ETag`` / ``If-Match`` remain header spellings. A domain identifier such
as ``BlueprintRevisionEtag`` or a SQL stale-concurrency message that says
``ETag`` is a naming regression. If this gate fails, rename the production
identifier or message to the exact Edit Number or Revision Number.
"""

import os
import re

import file_utils

IDENTIFIER = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*[Ee]tag[A-Za-z0-9_]*\b")
SQL_ETAG_MESSAGE = re.compile(r"(ETag is stale|metadata ETag|Edit Number ETag)")
ALLOWED_TOKENS = {"etag", "ETAG", "ETag"}
LIVING_DOCS = (
	"docs/API_CONTRACTS.md",
	"docs/CONTRACTS.md",
	"docs/NAMING_CONVENTIONS.md",
	"docs/TERMINOLOGY_CONTRACT.md",
)
SKIP_DIR_NAMES = {
	"target",
	"node_modules",
	"generated",
	"archive",
	"playwright",
	"e2e",
	"_temp",
	"tests",
}


def _is_http_header_line(line: str, token: str) -> bool:
	if token in {"etag", "ETAG"}:
		return True
	lowered = line.lower()
	return token == "ETag" and (
		"http" in lowered or "header" in lowered or "if-match" in lowered or "`etag`" in lowered
	)


def _iter_production_files(repo_root: str) -> list[str]:
	roots = [
		os.path.join(repo_root, "crates"),
		os.path.join(repo_root, "schemas", "base_schema"),
		os.path.join(repo_root, "src"),
	]
	files: list[str] = []
	for root in roots:
		for dirpath, dirnames, filenames in os.walk(root):
			dirnames[:] = [
				name
				for name in dirnames
				if name not in SKIP_DIR_NAMES and not name.startswith(".")
			]
			if os.path.basename(dirpath) == "tests":
				continue
			for name in filenames:
				if name.endswith((".rs", ".sql", ".ts", ".tsx")):
					files.append(os.path.join(dirpath, name))
	for relative in LIVING_DOCS:
		files.append(os.path.join(repo_root, relative))
	return files


def test_production_boundaries_forbid_domain_etag_names() -> None:
	"""Domain *Etag identifiers and stale-ETag SQL messages must not ship."""
	repo_root = file_utils.get_repo_root()
	findings: list[str] = []
	for path in _iter_production_files(repo_root):
		with open(path, encoding="utf-8") as handle:
			for line_number, line in enumerate(handle, start=1):
				relative = os.path.relpath(path, repo_root)
				if SQL_ETAG_MESSAGE.search(line):
					findings.append(f"{relative}:{line_number}: {line.strip()}")
					continue
				for token in IDENTIFIER.findall(line):
					if token in ALLOWED_TOKENS and _is_http_header_line(line, token):
						continue
					if token in ALLOWED_TOKENS:
						continue
					findings.append(f"{relative}:{line_number}: {token}")
	assert findings == [], "domain ETag names remain:\n" + "\n".join(findings)
