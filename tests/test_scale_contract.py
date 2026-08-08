"""Permanent M2 policy gates for executable SQL and typed cursor paging.

This is deliberately a lexical policy gate, not a Rust parser or a proof of
runtime query construction.  It rejects ``OFFSET`` in executable migration SQL,
direct ``sqlx::query*`` literals, and literal fragments pushed to a syntactically
declared ``QueryBuilder``.  Rust route behavior remains covered by Rust tests.
"""

# Standard Library
import pathlib
import re

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
RUST_STRING_START_RE = re.compile(r'(?:br|rb|r|b)?(#+)?"')
SQL_START_RE = re.compile(r"^\s*(?:SELECT|WITH|INSERT|UPDATE|DELETE)\b", re.IGNORECASE)
OFFSET_RE = re.compile(r"\bOFFSET\b", re.IGNORECASE)
SQL_CALL_RE = re.compile(r"\bsqlx::query(?:_[A-Za-z0-9_]+)?!?\s*\(")
QUERY_BUILDER_RE = re.compile(
	r"\blet\s+(?:mut\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b[^=;]*="
	r"\s*QueryBuilder(?:\s*::\s*<[^;()]+>)?\s*::\s*new\s*\("
)


#============================================
def _skip_rust_comment(source: str, start: int) -> int:
	"""Return the first character after one Rust line or nested block comment."""
	if source.startswith("//", start):
		end = source.find("\n", start)
		return len(source) if end == -1 else end + 1

	depth = 1
	index = start + 2
	while depth and index < len(source):
		if source.startswith("/*", index):
			depth += 1
			index += 2
		elif source.startswith("*/", index):
			depth -= 1
			index += 2
		else:
			index += 1
	return index


#============================================
def _skip_rust_character(source: str, start: int) -> int | None:
	"""Return the end of one Rust character literal, never a lifetime marker."""
	if source[start] != "'" or start + 2 >= len(source):
		return None
	end = start + 2 if source[start + 1] == "\\" else start + 1
	return end + 1 if end < len(source) and source[end] == "'" else None


#============================================
def _rust_string_literal_at(source: str, start: int) -> tuple[int, str] | None:
	"""Return one Rust string's end index and content when it starts at ``start``."""
	match = RUST_STRING_START_RE.match(source, start)
	if match is None:
		return None

	hashes = match.group(1) or ""
	content_start = match.end()
	if hashes:
		end_marker = '"' + hashes
		content_end = source.find(end_marker, content_start)
		if content_end == -1:
			return None
		return content_end + len(end_marker), source[content_start:content_end]

	content = []
	index = content_start
	while index < len(source):
		character = source[index]
		if character == "\\":
			content.append(source[index:index + 2])
			index += 2
			continue
		if character == '"':
			return index + 1, "".join(content)
		content.append(character)
		index += 1
	return None


#============================================
def _rust_string_literals(source: str) -> list[tuple[int, int, str]]:
	"""Extract Rust string literals while treating comments as non-code."""
	literals = []
	index = 0
	while index < len(source):
		if source.startswith("//", index) or source.startswith("/*", index):
			index = _skip_rust_comment(source, index)
			continue
		character_end = _skip_rust_character(source, index)
		if character_end is not None:
			index = character_end
			continue
		literal = _rust_string_literal_at(source, index)
		if literal is None:
			index += 1
			continue
		end, content = literal
		literals.append((index, end, content))
		index = end
	return literals


#============================================
def _balanced_call(source: str, open_paren: int) -> str:
	"""Return one balanced Rust call argument, with strings/comments opaque."""
	depth = 1
	index = open_paren + 1
	while depth and index < len(source):
		if source.startswith("//", index) or source.startswith("/*", index):
			index = _skip_rust_comment(source, index)
			continue
		character_end = _skip_rust_character(source, index)
		if character_end is not None:
			index = character_end
			continue
		literal = _rust_string_literal_at(source, index)
		if literal is not None:
			end, _ = literal
			if end <= index:
				break
			index = end
			continue
		if source[index] == "(":
			depth += 1
		elif source[index] == ")":
			depth -= 1
		index += 1
	return source[open_paren + 1:index - 1] if depth == 0 else ""


#============================================
def _rust_opaque_spans(source: str) -> list[tuple[int, int]]:
	"""Return comment and string spans so regex searches start only in Rust code."""
	spans = []
	index = 0
	while index < len(source):
		if source.startswith("//", index) or source.startswith("/*", index):
			end = _skip_rust_comment(source, index)
			spans.append((index, end))
			index = end
			continue
		character_end = _skip_rust_character(source, index)
		if character_end is not None:
			spans.append((index, character_end))
			index = character_end
			continue
		literal = _rust_string_literal_at(source, index)
		if literal is not None:
			end, _ = literal
			spans.append((index, end))
			index = end
			continue
		index += 1
	return spans


#============================================
def _is_rust_code_index(index: int, opaque_spans: list[tuple[int, int]]) -> bool:
	"""Return whether an index lies outside a comment, character, or string."""
	return not any(start <= index < end for start, end in opaque_spans)


#============================================
def _strip_sql_noncode(source: str) -> str:
	"""Blank SQL comments and quoted values, including PostgreSQL dollar quotes."""
	characters = list(source)
	index = 0
	while index < len(source):
		if source.startswith("--", index):
			end = source.find("\n", index)
			end = len(source) if end == -1 else end
		elif source.startswith("/*", index):
			depth = 1
			end = index + 2
			while depth and end < len(source):
				if source.startswith("/*", end):
					depth += 1
					end += 2
				elif source.startswith("*/", end):
					depth -= 1
					end += 2
				else:
					end += 1
		elif source[index] in "'\"`":
			quote = source[index]
			end = index + 1
			while end < len(source):
				if source[end] == quote:
					if end + 1 < len(source) and source[end + 1] == quote:
						end += 2
						continue
					end += 1
					break
				end += 2 if source[end] == "\\" else 1
		elif source[index] == "$":
			marker = re.match(r"\$[A-Za-z_][A-Za-z0-9_]*\$|\$\$", source[index:])
			if marker is None:
				index += 1
				continue
			end_marker = marker.group(0)
			end = source.find(end_marker, index + len(end_marker))
			end = len(source) if end == -1 else end + len(end_marker)
		else:
			index += 1
			continue
		for position in range(index, end):
			if characters[position] != "\n":
				characters[position] = " "
		index = end
	return "".join(characters)


#============================================
def _has_executable_offset(statement: str) -> bool:
	"""Return whether one SQL statement uses positional pagination in SQL code."""
	return OFFSET_RE.search(_strip_sql_noncode(statement)) is not None


#============================================
def _direct_sql_literals(source: str) -> list[str]:
	"""Return literal SQL supplied directly to a sqlx query macro/function."""
	statements = []
	opaque_spans = _rust_opaque_spans(source)
	for match in SQL_CALL_RE.finditer(source):
		if not _is_rust_code_index(match.start(), opaque_spans):
			continue
		open_paren = source.find("(", match.start(), match.end())
		for _, _, literal in _rust_string_literals(_balanced_call(source, open_paren)):
			if SQL_START_RE.search(literal):
				statements.append(literal)
	return statements


#============================================
def _query_builder_literals(source: str) -> list[str]:
	"""Return SQL fragments from syntactically declared QueryBuilders.

	The scanner intentionally recognizes only direct literal initializers and
	``builder.push(literal)`` calls.  It does not resolve shadowing, variables,
	or whether a builder is eventually executed; those runtime properties belong
	in Rust integration tests rather than this static policy gate.
	"""
	statements = []
	opaque_spans = _rust_opaque_spans(source)
	for declaration in QUERY_BUILDER_RE.finditer(source):
		if not _is_rust_code_index(declaration.start(), opaque_spans):
			continue
		name = declaration.group("name")
		open_paren = declaration.start() + declaration.group(0).rfind("(")
		pieces = [literal for _, _, literal in _rust_string_literals(_balanced_call(source, open_paren))]
		for push in re.finditer(rf"\b{re.escape(name)}\.push\s*\(", source[declaration.end():]):
			push_start = declaration.end() + push.start()
			open_paren = push_start + push.group(0).rfind("(")
			pieces.extend(literal for _, _, literal in _rust_string_literals(_balanced_call(source, open_paren)))
		statement = "".join(pieces)
		if SQL_START_RE.search(statement):
			statements.append(statement)
	return statements


#============================================
def _rust_sql_statements(source: str) -> list[str]:
	"""Return the deliberately narrow set of literal SQL paths this gate owns."""
	return _direct_sql_literals(source) + _query_builder_literals(source)


#============================================
def _source_files(directory: pathlib.Path, suffix: str) -> list[pathlib.Path]:
	"""Return stable source paths under one production directory."""
	return sorted(path for path in directory.rglob(f"*{suffix}") if path.is_file())


#============================================
def test_sql_literal_scanner_has_deliberate_static_boundaries() -> None:
	"""Reject direct/QueryBuilder SQL while ignoring unrelated literal text."""
	assert not _has_executable_offset("SELECT '-- OFFSET' /* OFFSET */")
	assert not _has_executable_offset("SELECT $$ OFFSET $$, $tag$OFFSET$tag$")

	unrelated = '''
		fn query() {
			sqlx::query("SELECT id FROM catalog");
			log::debug!("OFFSET is explained in help text");
			let unused = "OFFSET is also harmless outside SQL";
			let example = "sqlx::query(\\\"SELECT id OFFSET 8\\\")";
			// sqlx::query("SELECT id FROM catalog OFFSET 8");
		}
	'''
	assert not any(_has_executable_offset(sql) for sql in _rust_sql_statements(unrelated))

	direct = 'sqlx::query!(r#"SELECT id FROM catalog OFFSET 8"#);'
	assert any(_has_executable_offset(sql) for sql in _rust_sql_statements(direct))

	builder = '''
		let mut query = QueryBuilder::new("SELECT id FROM catalog");
		query.push(" OFFSET 8");
	'''
	assert any(_has_executable_offset(sql) for sql in _rust_sql_statements(builder))


#============================================
def test_query_paths_reject_executable_offset_pagination() -> None:
	"""Keep literal production query paths on stable cursor predicates."""
	offending = []
	for path in _source_files(REPO_ROOT / "crates", ".rs"):
		for statement in _rust_sql_statements(path.read_text()):
			if _has_executable_offset(statement):
				offending.append(path.relative_to(REPO_ROOT).as_posix())
	for path in _source_files(REPO_ROOT / "schemas" / "migrations", ".sql"):
		if _has_executable_offset(path.read_text()):
			offending.append(path.relative_to(REPO_ROOT).as_posix())
	assert not offending, (
		"Scale policy violation: literal executable SQL uses OFFSET. Use a stable "
		f"cursor predicate instead: {sorted(set(offending))}"
	)


#============================================
def test_store_list_contracts_require_bounded_typed_pages() -> None:
	"""Keep collection storage behind PageRequest, Page, and validated PageSize."""
	store_contract = (REPO_ROOT / "crates" / "store" / "src" / "lib.rs").read_text()
	pagination = (REPO_ROOT / "crates" / "store" / "src" / "pagination.rs").read_text()
	offending = []
	for match in re.finditer(r"async\s+fn\s+(list_[A-Za-z0-9_]+)\b", store_contract):
		method = store_contract[match.start():store_contract.find(";", match.end())]
		if "PageRequest" not in method or "Result<Page<" not in method:
			offending.append(match.group(1))
	assert "pub size: PageSize" in pagination
	assert "pub const MAX: u16" in pagination
	assert "(1..=Self::MAX).contains(&value)" in pagination
	assert not offending, (
		"Scale policy violation: store collection methods must accept PageRequest "
		f"and return Page: {', '.join(offending)}"
	)
