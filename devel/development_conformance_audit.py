#!/usr/bin/env python3
"""Check source naming and unsupported implementation scaffolding."""

import argparse
import ast
import json
import pathlib
import re
import subprocess


SOURCE_SUFFIXES = (".py", ".rs", ".sql", ".ts", ".tsx", ".js", ".mjs", ".sh", ".css", ".html")
SOURCE_PREFIXES = ("crates/", "src/", "schemas/", "devel/", "launchers/")
APPROVED_OWNER_SECTIONS = {
	"docs/CONTRACTS.md": {"## Jobs, objects, and database lifecycle"},
	"docs/DESIGN_DECISIONS.md": {
		"## Product model",
		"## Accounts, roles, and authorization",
		"## Questions and Pools",
		"## Blueprint Courses",
		"## Assessments and Student Work",
		"## Interface",
		"## Data, privacy, and operations",
		"## Implementation and evidence",
	},
	"docs/NAMING_CONVENTIONS.md": {"## Language matrix", "## PostgreSQL", "## Browser and TypeScript"},
}
SNAKE_CASE_NAME = re.compile(r"[a-z][a-z0-9]*(?:_[a-z0-9]+)*$")
SCAFFOLDING_SYMBOL_MARKER = re.compile(
	r"placeholder|stub|todo|(?:compat(?:ibility)?|legacy)(?:adapter|layer|shim|facade|worker)",
	re.IGNORECASE,
)
SCAFFOLDING_PATH_MARKER = re.compile(
	r"placeholder|stub|todo|(?:compat(?:ibility)?|legacy)(?:adapter|layer|shim|facade|worker)|api_compatibility",
	re.IGNORECASE,
)
DECLARATION_PATTERNS = {
	"rust": re.compile(r"^\s*(?:pub\s+)?(?:struct|enum|trait|fn|mod)\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE),
	"typescript": re.compile(r"^\s*(?:export\s+)?(?:abstract\s+)?(?:class|interface|type|function|const)\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE),
	"sql": re.compile(r"^\s*CREATE\s+(?:OR\s+REPLACE\s+)?(?:TABLE|TYPE|VIEW|FUNCTION)\s+(?:[A-Za-z_][A-Za-z0-9_]*\.)?([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE | re.IGNORECASE),
}


#============================================
def parse_args() -> argparse.Namespace:
	"""Parse the focused audit command options.

	Returns:
		argparse.Namespace: Parsed command options.
	"""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("--check", action="store_true", help="fail when the inventory finds a violation")
	args = parser.parse_args()
	return args


#============================================
def repository_root() -> pathlib.Path:
	"""Return the resolved repository root containing this script.

	Returns:
		pathlib.Path: Resolved repository root.
	"""
	script_directory = pathlib.Path(__file__).resolve().parent
	result = subprocess.run(
		["git", "rev-parse", "--show-toplevel"],
		cwd=script_directory,
		check=True,
		stdout=subprocess.PIPE,
		stderr=subprocess.PIPE,
	)
	root_text = result.stdout.decode("utf-8").strip()
	if not root_text or "\n" in root_text:
		raise RuntimeError("git did not return exactly one repository root")
	repo_root = pathlib.Path(root_text).resolve()
	if not (repo_root / ".git").exists() or not pathlib.Path(__file__).resolve().is_relative_to(repo_root):
		raise RuntimeError(f"development audit requires its script inside the Git root: {repo_root}")
	return repo_root


#============================================
def contained_path(repo_root: pathlib.Path, relative_path: str) -> pathlib.Path:
	"""Resolve one repository-relative input without allowing path escape.

	Args:
		repo_root: Resolved repository root.
		relative_path: Repository-relative path from Git or the allowlist.

	Returns:
		pathlib.Path: Resolved path inside the repository root.
	"""
	path = pathlib.PurePosixPath(relative_path)
	if path.is_absolute() or ".." in path.parts:
		raise RuntimeError(f"unsafe repository-relative path: {relative_path}")
	resolved_path = (repo_root / path).resolve()
	if not resolved_path.is_relative_to(repo_root):
		raise RuntimeError(f"repository path escapes root: {relative_path}")
	return resolved_path


#============================================
def git_paths(repo_root: pathlib.Path, command: list[str]) -> set[str]:
	"""Return one NUL-delimited Git path set while rejecting malformed output.

	Args:
		repo_root: Resolved repository root.
		command: Exact Git path-listing command.

	Returns:
		set[str]: Repository-relative paths from Git.
	"""
	result = subprocess.run(
		command,
		cwd=repo_root,
		check=True,
		stdout=subprocess.PIPE,
		stderr=subprocess.PIPE,
	)
	raw_paths = result.stdout.split(b"\0")
	paths = set()
	for raw_path in raw_paths:
		if not raw_path:
			continue
		relative_path = raw_path.decode("utf-8")
		contained_path(repo_root, relative_path)
		paths.add(relative_path)
	return paths


#============================================
def current_source_paths(repo_root: pathlib.Path, candidate_paths: set[str]) -> list[str]:
	"""Return current regular source files from tracked and untracked candidates.

	Missing tracked paths are an expected dirty-worktree deletion and are not current
	source. Every existing source candidate must be a regular nonsymlink file so an
	audit cannot silently follow an unexpected filesystem object.

	Args:
		repo_root: Resolved repository root.
		candidate_paths: Git-reported tracked or nonignored untracked paths.

	Returns:
		list[str]: Sorted current source paths.
	"""
	paths = []
	for relative_path in sorted(candidate_paths):
		if not is_source_path(relative_path):
			continue
		unresolved_path = repo_root / pathlib.PurePosixPath(relative_path)
		contained_path(repo_root, relative_path)
		if unresolved_path.is_symlink():
			raise RuntimeError(f"source candidate must not be a symlink: {relative_path}")
		if not unresolved_path.exists():
			continue
		if not unresolved_path.is_file():
			raise RuntimeError(f"source candidate must be a regular file: {relative_path}")
		paths.append(relative_path)
	return paths


#============================================
def inventory_paths(repo_root: pathlib.Path) -> list[str]:
	"""Return current tracked and nonignored untracked regular source files.

	Args:
		repo_root: Resolved repository root.

	Returns:
		list[str]: Sorted current repository source paths.
	"""
	tracked = git_paths(repo_root, ["git", "ls-files", "-z"])
	untracked = git_paths(repo_root, ["git", "ls-files", "--others", "--exclude-standard", "-z"])
	paths = current_source_paths(repo_root, tracked | untracked)
	return paths


#============================================
def load_allowlist(repo_root: pathlib.Path) -> dict[str, set[tuple[str, str]] | set[str]]:
	"""Load exact, documented exceptions and reject an unsafe allowlist.

	Args:
		repo_root: Resolved repository root.

	Returns:
		dict[str, set[tuple[str, str]] | set[str]]: Exact permitted source names and symbols.
	"""
	allowlist_path = contained_path(repo_root, "devel/development_conformance_allowlist.json")
	data = json.loads(allowlist_path.read_text(encoding="utf-8"))
	expected_keys = {"source_name_exceptions", "scaffolding_exceptions"}
	if set(data) != expected_keys:
		raise RuntimeError("allowlist must contain only source_name_exceptions and scaffolding_exceptions")
	name_exceptions = set()
	for entry in data["source_name_exceptions"]:
		if set(entry) != {"path", "owner_document", "owner_section", "reason"}:
			raise RuntimeError("source-name exception must contain path, owner_document, owner_section, and reason")
		validate_allowlist_entry(repo_root, entry)
		name_exceptions.add(entry["path"])
	scaffolding_exceptions = set()
	for entry in data["scaffolding_exceptions"]:
		if set(entry) != {"path", "symbol", "owner_document", "owner_section", "reason"}:
			raise RuntimeError("scaffolding exception must contain path, symbol, owner_document, owner_section, and reason")
		validate_allowlist_entry(repo_root, entry)
		if not entry["symbol"]:
			raise RuntimeError("scaffolding exception requires a symbol")
		scaffolding_exceptions.add((entry["path"], entry["symbol"]))
	allowlist = {"source_names": name_exceptions, "scaffolding": scaffolding_exceptions}
	return allowlist


#============================================
def validate_allowlist_entry(repo_root: pathlib.Path, entry: dict[str, str]) -> None:
	"""Require an exact existing path, authority document, and useful reason.

	Args:
		repo_root: Resolved repository root.
		entry: One decoded allowlist record.
	"""
	path = contained_path(repo_root, entry["path"])
	owner_document_path = entry["owner_document"]
	if owner_document_path not in APPROVED_OWNER_SECTIONS:
		raise RuntimeError(f"allowlist owner must be an approved durable authority: {owner_document_path}")
	if entry["owner_section"] not in APPROVED_OWNER_SECTIONS[owner_document_path]:
		raise RuntimeError(f"allowlist owner section is not approved: {entry['owner_section']}")
	owner_document = contained_path(repo_root, owner_document_path)
	if not path.is_file() or not owner_document.is_file():
		raise RuntimeError(f"allowlist record needs existing files: {entry['path']}")
	if entry["owner_section"] not in owner_document.read_text(encoding="utf-8"):
		raise RuntimeError(f"allowlist owner section is absent: {entry['owner_section']}")
	if len(entry["reason"].strip()) < 20:
		raise RuntimeError(f"allowlist record needs a specific reason: {entry['path']}")


#============================================
def is_source_path(relative_path: str) -> bool:
	"""Return whether a tracked path belongs to the audited source inventory.

	Args:
		relative_path: Repository-relative tracked path.

	Returns:
		bool: Whether the path is a source file in an owned source area.
	"""
	if not relative_path.startswith(SOURCE_PREFIXES):
		return False
	path_parts = pathlib.PurePosixPath(relative_path).parts
	if "tests" in path_parts or relative_path.endswith(("_tests.rs", "_test.rs", "test_support.rs")):
		return False
	return relative_path.endswith(SOURCE_SUFFIXES)


#============================================
def source_stem(relative_path: str) -> str:
	"""Return the physical source-name stem used by the naming audit.

	Args:
		relative_path: Repository-relative tracked source path.

	Returns:
		str: Filename minus its language suffix.
	"""
	name = pathlib.PurePosixPath(relative_path).name
	if name.endswith(".d.ts"):
		stem = name[:-5]
	else:
		stem = name.rsplit(".", maxsplit=1)[0]
	return stem


#============================================
def source_name_violations(paths: list[str], allowed_names: set[str]) -> tuple[list[str], set[str]]:
	"""Find owned source filenames that are not readable snake_case.

	Args:
		paths: Tracked repository-relative paths.
		allowed_names: Exact externally owned filename exceptions.

	Returns:
		tuple[list[str], set[str]]: Violations and consumed exception paths.
	"""
	violations = []
	consumed = set()
	for relative_path in paths:
		if not is_source_path(relative_path):
			continue
		stem = source_stem(relative_path)
		if stem in {"__init__", "lib", "mod"} or SNAKE_CASE_NAME.fullmatch(stem):
			continue
		if relative_path in allowed_names:
			consumed.add(relative_path)
			continue
		violations.append(relative_path)
	return violations, consumed


#============================================
def source_language(relative_path: str) -> str | None:
	"""Return the bounded declaration parser appropriate to one source path.

	Args:
		relative_path: Repository-relative source path.

	Returns:
		str | None: Parser name, or None when this audit does not parse declarations.
	"""
	if relative_path.endswith(".py"):
		language = "python"
	elif relative_path.endswith(".rs"):
		language = "rust"
	elif relative_path.endswith((".ts", ".tsx", ".js", ".mjs")):
		language = "typescript"
	elif relative_path.endswith(".sql"):
		language = "sql"
	else:
		language = None
	return language


#============================================
def all_python_declarations(source_text: str) -> set[str]:
	"""Return Python definitions through syntax-aware AST parsing.

	Args:
		source_text: UTF-8 Python source text.

	Returns:
		set[str]: Class and function definition names.
	"""
	parsed_source = ast.parse(source_text)
	symbols = set()
	for node in ast.walk(parsed_source):
		if isinstance(node, (ast.AsyncFunctionDef, ast.ClassDef, ast.FunctionDef)):
			symbols.add(node.name)
	return symbols


#============================================
def mask_non_code_characters(source_text: str, start: int, end: int) -> tuple[list[str], int]:
	"""Replace one non-code range with spaces while preserving line positions.

	Args:
		source_text: Complete source text.
		start: Inclusive range start.
		end: Exclusive range end.

	Returns:
		tuple[list[str], int]: Masked characters and the range end.
	"""
	masked = []
	for character in source_text[start:end]:
		masked.append("\n" if character == "\n" else " ")
	return masked, end


#============================================
def find_quoted_end(source_text: str, start: int, quote: str) -> int:
	"""Find the end of one escaped single-character quote form.

	Args:
		source_text: Complete source text.
		start: Index of the opening quote.
		quote: Quote delimiter.

	Returns:
		int: Exclusive end index, or the source length for an unterminated literal.
	"""
	index = start + 1
	while index < len(source_text):
		if source_text[index] == "\\":
			index += 2
			continue
		if source_text[index] == quote:
			return index + 1
		index += 1
	return len(source_text)


#============================================
def find_rust_raw_string_end(source_text: str, start: int) -> int | None:
	"""Return the end of a Rust raw string that begins at start, if present.

	Args:
		source_text: Complete Rust source text.
		start: Index of a possible `r` or `br` prefix.

	Returns:
		int | None: Exclusive raw-string end, or None when no raw string starts there.
	"""
	index = start
	if source_text.startswith("br", index):
		index += 2
	elif source_text.startswith("r", index):
		index += 1
	else:
		return None
	hash_start = index
	while index < len(source_text) and source_text[index] == "#":
		index += 1
	if index >= len(source_text) or source_text[index] != '"':
		return None
	delimiter = '"' + source_text[hash_start:index]
	end_start = source_text.find(delimiter, index + 1)
	if end_start == -1:
		return len(source_text)
	return end_start + len(delimiter)


#============================================
def find_sql_dollar_quote_end(source_text: str, start: int) -> int | None:
	"""Return the end of a PostgreSQL dollar-quoted literal, if one begins here.

	Args:
		source_text: Complete SQL source text.
		start: Index of a possible dollar-quote delimiter.

	Returns:
		int | None: Exclusive literal end, or None when no dollar quote starts here.
	"""
	match = re.match(r"\$[A-Za-z_][A-Za-z0-9_]*\$|\$\$", source_text[start:])
	if match is None:
		return None
	delimiter = match.group(0)
	end_start = source_text.find(delimiter, start + len(delimiter))
	if end_start == -1:
		return len(source_text)
	return end_start + len(delimiter)


#============================================
def masked_non_python_source(language: str, source_text: str) -> str:
	"""Mask comments and literals before bounded non-Python declaration scanning.

	TypeScript and Rust are intentionally lexical, not full language parsers: declarations
	inside template/string/comment text do not count, while actual top-level declarations keep
	their source positions. SQL masks comments and single-quoted literals; double-quoted SQL
	identifiers remain code because DDL may legally declare them.

	Args:
		language: Bounded parser name.
		source_text: UTF-8 source text.

	Returns:
		str: Source-shaped text with non-code content replaced by spaces.
	"""
	masked = []
	index = 0
	while index < len(source_text):
		if language in {"typescript", "rust"} and source_text.startswith("//", index):
			end = source_text.find("\n", index)
			if end == -1:
				end = len(source_text)
			characters, index = mask_non_code_characters(source_text, index, end)
			masked.extend(characters)
			continue
		if language == "sql" and source_text.startswith("--", index):
			end = source_text.find("\n", index)
			if end == -1:
				end = len(source_text)
			characters, index = mask_non_code_characters(source_text, index, end)
			masked.extend(characters)
			continue
		if source_text.startswith("/*", index) and language in {"typescript", "rust", "sql"}:
			depth = 1
			end = index + 2
			while end < len(source_text) and depth:
				if source_text.startswith("/*", end):
					depth += 1
					end += 2
				elif source_text.startswith("*/", end):
					depth -= 1
					end += 2
				else:
					end += 1
			characters, index = mask_non_code_characters(source_text, index, end)
			masked.extend(characters)
			continue
		if language == "rust":
			raw_end = find_rust_raw_string_end(source_text, index)
			if raw_end is not None:
				characters, index = mask_non_code_characters(source_text, index, raw_end)
				masked.extend(characters)
				continue
		if language == "typescript" and source_text[index] in {"'", '"', "`"}:
			end = find_quoted_end(source_text, index, source_text[index])
			characters, index = mask_non_code_characters(source_text, index, end)
			masked.extend(characters)
			continue
		if language == "rust" and source_text[index] == '"':
			end = find_quoted_end(source_text, index, '"')
			characters, index = mask_non_code_characters(source_text, index, end)
			masked.extend(characters)
			continue
		if language == "rust" and source_text[index] == "'":
			end = find_quoted_end(source_text, index, "'")
			is_closed_character = end <= len(source_text) and source_text[end - 1:end] == "'"
			if is_closed_character and end - index <= 12:
				characters, index = mask_non_code_characters(source_text, index, end)
				masked.extend(characters)
				continue
		if language == "sql" and source_text[index] == "$":
			dollar_quote_end = find_sql_dollar_quote_end(source_text, index)
			if dollar_quote_end is not None:
				characters, index = mask_non_code_characters(source_text, index, dollar_quote_end)
				masked.extend(characters)
				continue
		if language == "sql" and source_text[index] == "'":
			end = index + 1
			while end < len(source_text):
				if source_text[end] == "'" and source_text[end + 1:end + 2] == "'":
					end += 2
					continue
				if source_text[end] == "'":
					end += 1
					break
				end += 1
			characters, index = mask_non_code_characters(source_text, index, end)
			masked.extend(characters)
			continue
		masked.append(source_text[index])
		index += 1
	masked_text = "".join(masked)
	return masked_text


#============================================
def declared_symbols(relative_path: str, source_text: str) -> set[str]:
	"""Return declarations using a bounded parser for the source language.

	Rust, TypeScript, and SQL use anchored declaration patterns rather than full parsers.
	Python uses its AST so comments and triple-quoted documentation cannot look like code.

	Args:
		relative_path: Repository-relative source path.
		source_text: UTF-8 source text from an audited source path.

	Returns:
		set[str]: Recognized declared symbols.
	"""
	language = source_language(relative_path)
	if language == "python":
		symbols = all_python_declarations(source_text)
	elif language in DECLARATION_PATTERNS:
		pattern = DECLARATION_PATTERNS[language]
		masked_source = masked_non_python_source(language, source_text)
		symbols = {match.group(1) for match in pattern.finditer(masked_source)}
	else:
		symbols = set()
	return symbols


#============================================
def declaration_symbols(relative_path: str, source_text: str) -> set[str]:
	"""Return declared scaffolding-like symbols from one owned source file.

	Args:
		source_text: UTF-8 source text from an audited source path.

	Returns:
		set[str]: Declared symbols that use an explicit scaffolding marker.
	"""
	symbols = set()
	for symbol in declared_symbols(relative_path, source_text):
		if SCAFFOLDING_SYMBOL_MARKER.search(symbol):
			symbols.add(symbol)
	return symbols


#============================================
def scaffolding_violations(
	repo_root: pathlib.Path,
	paths: list[str],
	allowed_scaffolding: set[tuple[str, str]],
) -> tuple[list[str], set[tuple[str, str]]]:
	"""Find declared placeholder or compatibility scaffolding without a record.

	Args:
		repo_root: Resolved repository root.
		paths: Tracked repository-relative paths.
		allowed_scaffolding: Exact approved path-and-symbol pairs.

	Returns:
		tuple[list[str], set[tuple[str, str]]]: Violations and consumed exceptions.
	"""
	violations = []
	consumed = set()
	for relative_path in paths:
		if not is_source_path(relative_path):
			continue
		source_path = contained_path(repo_root, relative_path)
		source_text = source_path.read_text(encoding="utf-8")
		symbols = declaration_symbols(relative_path, source_text)
		if SCAFFOLDING_PATH_MARKER.search(relative_path):
			symbols.update(declared_symbols(relative_path, source_text))
		for symbol in sorted(symbols):
			key = (relative_path, symbol)
			if key in allowed_scaffolding:
				consumed.add(key)
				continue
			violations.append(f"{relative_path}: {symbol}")
	return violations, consumed


#============================================
def audit(repo_root: pathlib.Path) -> list[str]:
	"""Run every focused conformance inventory and return sorted findings.

	Args:
		repo_root: Resolved repository root.

	Returns:
		list[str]: Deterministic audit findings.
	"""
	allowlist = load_allowlist(repo_root)
	paths = inventory_paths(repo_root)
	name_violations, used_names = source_name_violations(paths, allowlist["source_names"])
	scaffolding_violations_found, used_scaffolding = scaffolding_violations(
		repo_root,
		paths,
		allowlist["scaffolding"],
	)
	unused_names = sorted(allowlist["source_names"] - used_names)
	unused_scaffolding = sorted(allowlist["scaffolding"] - used_scaffolding)
	findings = []
	findings.extend(f"source name is not readable snake_case: {path}" for path in name_violations)
	findings.extend(f"unsupported product scaffolding: {item}" for item in scaffolding_violations_found)
	findings.extend(f"unused source-name exception: {path}" for path in unused_names)
	findings.extend(
		f"unused scaffolding exception: {path}: {symbol}"
		for path, symbol in unused_scaffolding
	)
	findings.sort()
	return findings


#============================================
def main() -> None:
	"""Run the development-conformance audit command."""
	args = parse_args()
	if not args.check:
		raise RuntimeError("run with --check")
	findings = audit(repository_root())
	if findings:
		for finding in findings:
			print(f"FAIL: {finding}")
		raise RuntimeError(f"development conformance audit found {len(findings)} issue(s)")
	print("PASS: development conformance audit found no unsupported source names or scaffolding")


if __name__ == "__main__":
	main()
