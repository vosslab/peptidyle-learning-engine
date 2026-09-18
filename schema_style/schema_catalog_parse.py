"""Parse PostgreSQL DDL into the schema catalog model."""

# Standard Library
import pathlib
import re

# local repo modules
import schema_style.schema_catalog_scan as schema_catalog_scan


IN_CHECK_RE = re.compile(
	r"^\(\s*([A-Za-z_][A-Za-z0-9_]*)\s+IN\s*\(",
	re.IGNORECASE | re.DOTALL,
)
EQ_CHECK_RE = re.compile(
	r"^\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*'([^']*)'"
	r"(?:::[A-Za-z_][A-Za-z0-9_.]*)?\s*\)$",
	re.IGNORECASE | re.DOTALL,
)

#============================================
def empty_catalog() -> dict:
	"""
	Return an empty catalog dict with the stable model keys.

	Returns:
		dict: Catalog with tables, enums, domains, and indexes.
	"""
	catalog = {
		"tables": {},
		"enums": {},
		"domains": {},
		"indexes": [],
	}
	return catalog


#============================================
def parse_source(source_dir: str, catalog: dict) -> None:
	"""
	Parse every *.sql file in source_dir into catalog.

	Args:
		source_dir: Directory of SQL modules.
		catalog: Catalog being filled.
	"""
	for path in schema_catalog_scan._sql_paths(source_dir):
		rel = schema_catalog_scan._relative_sql_path(source_dir, path)
		_parse_sql_file(path, rel, catalog)


def _parse_sql_file(path: pathlib.Path, rel: str, catalog: dict) -> None:
	"""
	Parse one SQL file into the catalog.

	Args:
		path: SQL file path.
		rel: Path relative to the source directory.
		catalog: Catalog being filled.
	"""
	text = path.read_text()
	statements = _iter_statements(text)
	for start_line, statement in statements:
		_dispatch_statement(rel, start_line, statement, catalog)


#============================================
def _iter_statements(text: str) -> list:
	"""
	Split SQL text into top-level statements.

	Skips -- comments and dollar-quoted bodies so function internals are not
	treated as DDL.

	Args:
		text: File contents.

	Returns:
		list: (start_line, statement_text) pairs without the trailing semicolon.
	"""
	statements = []
	index = 0
	line = 1
	length = len(text)
	while index < length:
		index, line = schema_catalog_scan._skip_trivia(text, index, line)
		if index >= length:
			break
		start_line = line
		start = index
		index, line = schema_catalog_scan._scan_statement(text, index, line)
		raw = text[start:index].strip()
		if raw.endswith(";"):
			raw = raw[:-1].strip()
		if raw:
			statements.append((start_line, raw))
	return statements


#============================================
def _dispatch_statement(
		file_name: str,
		start_line: int,
		statement: str,
		catalog: dict,
		) -> None:
	"""
	Route one DDL statement into the catalog.

	Args:
		file_name: SQL basename.
		start_line: 1-based line of the statement.
		statement: Statement text without the semicolon.
		catalog: Catalog being filled.
	"""
	head = _leading_keywords(statement, 4)
	if head[:2] == ["CREATE", "TABLE"]:
		_parse_create_table(file_name, start_line, statement, catalog)
		return
	if head[:2] == ["CREATE", "TYPE"] and " AS ENUM" in statement.upper():
		_parse_create_enum(file_name, start_line, statement, catalog)
		return
	if head[:2] == ["CREATE", "DOMAIN"]:
		_parse_create_domain(file_name, start_line, statement, catalog)
		return
	if head[:2] == ["CREATE", "INDEX"] or head[:3] == ["CREATE", "UNIQUE", "INDEX"]:
		_parse_create_index(file_name, start_line, statement, catalog)
		return
	if head[:2] == ["ALTER", "TABLE"] and "FOREIGN KEY" in statement.upper():
		_parse_alter_foreign_key(file_name, start_line, statement, catalog)
		return
	if head[:3] == ["COMMENT", "ON", "TABLE"]:
		_parse_comment_on_table(statement, catalog)
		return
	if head[:3] == ["COMMENT", "ON", "COLUMN"]:
		_parse_comment_on_column(statement, catalog)


#============================================
def _leading_keywords(statement: str, count: int) -> list:
	"""
	Return the first uppercase keyword tokens of a statement.

	Args:
		statement: SQL statement.
		count: Maximum tokens to return.

	Returns:
		list: Uppercase keyword strings.
	"""
	tokens = []
	index = 0
	length = len(statement)
	while index < length and len(tokens) < count:
		while index < length and statement[index] in " \t\r\n":
			index += 1
		if index >= length:
			break
		match = schema_catalog_scan.IDENT_RE.match(statement, index)
		if not match:
			break
		tokens.append(match.group(0).upper())
		index = match.end()
	return tokens


#============================================
def _parse_create_table(
		file_name: str,
		start_line: int,
		statement: str,
		catalog: dict,
		) -> None:
	"""
	Parse a CREATE TABLE statement.

	Args:
		file_name: SQL basename.
		start_line: Line of CREATE TABLE.
		statement: Full statement.
		catalog: Catalog being filled.
	"""
	index = schema_catalog_scan._skip_keywords(statement, 0, ["CREATE", "TABLE", "IF", "NOT", "EXISTS"])
	qualified, index = schema_catalog_scan._read_qualified_name(statement, index)
	index = schema_catalog_scan._skip_space(statement, index)
	if index >= len(statement) or statement[index] != "(":
		return
	close = schema_catalog_scan._matching_paren(statement, index)
	body = statement[index + 1:close]
	parts = schema_catalog_scan._split_top_level_commas(body)
	table = {
		"file": file_name,
		"line": start_line,
		"columns": [],
		"primary_key": [],
		"uniques": [],
		"foreign_keys": [],
		"comment": None,
		"role": None,
	}
	for part in parts:
		_add_table_part(table, part, file_name, start_line)
	catalog["tables"][qualified] = table
	_add_constraint_indexes(catalog, qualified, table, file_name, start_line)


#============================================
def _add_table_part(
		table: dict,
		part: str,
		file_name: str,
		start_line: int,
		) -> None:
	"""
	Classify one CREATE TABLE comma-separated item.

	Args:
		table: Table dict being filled.
		part: One top-level comma item.
		file_name: SQL basename.
		start_line: Table line, used for inline FK locations.
	"""
	item = part.strip()
	while item.startswith("--"):
		newline = item.find("\n")
		if newline < 0:
			return
		item = item[newline + 1:].strip()
	if not item:
		return
	rest = _strip_constraint_name(item)
	upper = rest.upper()
	if upper.startswith("PRIMARY KEY"):
		table["primary_key"] = _paren_ident_list(rest)
		return
	if upper.startswith("UNIQUE"):
		table["uniques"].append(_paren_ident_list(rest))
		return
	if upper.startswith("FOREIGN KEY"):
		fk = _parse_foreign_key_clause(rest, file_name, start_line)
		if fk is not None:
			table["foreign_keys"].append(fk)
		return
	if upper.startswith("CHECK"):
		_attach_table_check(table, rest)
		return
	_add_column(table, item, file_name, start_line)


#============================================
def _add_column(
		table: dict,
		item: str,
		file_name: str,
		start_line: int,
		) -> None:
	"""
	Parse a column definition into the table.

	Args:
		table: Table dict.
		item: Column definition text.
		file_name: SQL basename.
		start_line: Table line.
	"""
	index = schema_catalog_scan._skip_space(item, 0)
	name, index = schema_catalog_scan._read_ident(item, index)
	index = schema_catalog_scan._skip_space(item, index)
	type_start = index
	while index < len(item):
		index = schema_catalog_scan._skip_space(item, index)
		if index >= len(item):
			break
		if item[index] == "(":
			index = schema_catalog_scan._matching_paren(item, index) + 1
			continue
		if item[index] == "[":
			while index < len(item) and item[index] != "]":
				index += 1
			if index < len(item):
				index += 1
			continue
		match = schema_catalog_scan.IDENT_RE.match(item, index)
		if not match:
			break
		word = match.group(0).upper()
		if word in schema_catalog_scan.COLUMN_STOP:
			break
		index = match.end()
	col_type = " ".join(item[type_start:index].split())
	tail = item[index:]
	tail_upper = tail.upper()
	not_null = "NOT NULL" in tail_upper or "PRIMARY KEY" in tail_upper
	check_text = _extract_check_clause(tail)
	if "PRIMARY KEY" in tail_upper:
		table["primary_key"] = [name]
	if re.search(r"\bUNIQUE\b", tail_upper) and "PRIMARY KEY" not in tail_upper:
		table["uniques"].append([name])
	fk = _extract_column_references(name, tail, file_name, start_line)
	if fk is not None:
		table["foreign_keys"].append(fk)
	table["columns"].append({
		"name": name,
		"type": col_type,
		"not_null": not_null,
		"check": check_text,
		"comment": None,
	})


#============================================
def _extract_check_clause(tail: str) -> str | None:
	"""
	Return the CHECK (...) expression from a column tail, if present.

	Args:
		tail: Text after the column type.

	Returns:
		str | None: Parenthesized CHECK body including parens, or None.
	"""
	match = re.search(r"\bCHECK\b", tail, re.IGNORECASE)
	if not match:
		return None
	index = schema_catalog_scan._skip_space(tail, match.end())
	if index >= len(tail) or tail[index] != "(":
		return None
	close = schema_catalog_scan._matching_paren(tail, index)
	check_text = tail[index:close + 1]
	return check_text


#============================================
def _extract_column_references(
		column_name: str,
		tail: str,
		file_name: str,
		start_line: int,
		) -> dict | None:
	"""
	Parse a column-level REFERENCES clause.

	Args:
		column_name: Referencing column.
		tail: Text after the column type.
		file_name: SQL basename.
		start_line: Table line.

	Returns:
		dict | None: Foreign key dict, or None when absent.
	"""
	match = re.search(r"\bREFERENCES\b", tail, re.IGNORECASE)
	if not match:
		return None
	index = schema_catalog_scan._skip_space(tail, match.end())
	parent, index = schema_catalog_scan._read_qualified_name(tail, index)
	index = schema_catalog_scan._skip_space(tail, index)
	parent_columns = [column_name]
	if index < len(tail) and tail[index] == "(":
		parent_columns = _paren_ident_list(tail[index:])
	fk = {
		"columns": [column_name],
		"parent": parent,
		"parent_columns": parent_columns,
		"file": file_name,
		"line": start_line,
	}
	return fk


#============================================
def _parse_foreign_key_clause(
		clause: str,
		file_name: str,
		start_line: int,
		) -> dict | None:
	"""
	Parse FOREIGN KEY (cols) REFERENCES parent (cols).

	Args:
		clause: Clause beginning with FOREIGN KEY.
		file_name: SQL basename.
		start_line: Source line.

	Returns:
		dict | None: Foreign key dict.
	"""
	index = schema_catalog_scan._skip_keywords(clause, 0, ["FOREIGN", "KEY"])
	index = schema_catalog_scan._skip_space(clause, index)
	if index >= len(clause) or clause[index] != "(":
		return None
	columns = _paren_ident_list(clause[index:])
	close = schema_catalog_scan._matching_paren(clause, index)
	index = schema_catalog_scan._skip_space(clause, close + 1)
	index = schema_catalog_scan._skip_keywords(clause, index, ["REFERENCES"])
	parent, index = schema_catalog_scan._read_qualified_name(clause, index)
	index = schema_catalog_scan._skip_space(clause, index)
	parent_columns = list(columns)
	if index < len(clause) and clause[index] == "(":
		parent_columns = _paren_ident_list(clause[index:])
	fk = {
		"columns": columns,
		"parent": parent,
		"parent_columns": parent_columns,
		"file": file_name,
		"line": start_line,
	}
	return fk


#============================================
def _attach_table_check(table: dict, clause: str) -> None:
	"""
	Attach a single-column IN or equality CHECK to that column.

	Args:
		table: Table dict.
		clause: CHECK (...) clause.
	"""
	index = schema_catalog_scan._skip_keywords(clause, 0, ["CHECK"])
	index = schema_catalog_scan._skip_space(clause, index)
	if index >= len(clause) or clause[index] != "(":
		return
	close = schema_catalog_scan._matching_paren(clause, index)
	expr = clause[index:close + 1]
	column_name = _single_column_check_name(expr)
	if column_name is None:
		return
	for column in table["columns"]:
		if column["name"] == column_name:
			if column["check"] is None or " IN " in expr.upper():
				column["check"] = expr
			return


#============================================
def _single_column_check_name(expr: str) -> str | None:
	"""
	Return the column name when a CHECK is a literal IN list or equality.

	Args:
		expr: Parenthesized CHECK expression.

	Returns:
		str | None: Column name, or None when the check is not a single-column
			literal set.
	"""
	in_match = IN_CHECK_RE.match(expr)
	if in_match:
		return in_match.group(1)
	eq_match = EQ_CHECK_RE.match(expr)
	if eq_match:
		return eq_match.group(1)
	return None


#============================================
def _add_constraint_indexes(
		catalog: dict,
		qualified: str,
		table: dict,
		file_name: str,
		start_line: int,
		) -> None:
	"""
	Record PK and UNIQUE as unique indexes for FK covering checks.

	Args:
		catalog: Catalog.
		qualified: schema.table.
		table: Table dict.
		file_name: SQL basename.
		start_line: Table line.
	"""
	if table["primary_key"]:
		catalog["indexes"].append({
			"name": qualified + "_pkey",
			"table": qualified,
			"columns": list(table["primary_key"]),
			"unique": True,
			"partial": None,
			"file": file_name,
			"line": start_line,
		})
	for offset, unique_cols in enumerate(table["uniques"]):
		catalog["indexes"].append({
			"name": qualified + "_unique_" + str(offset),
			"table": qualified,
			"columns": list(unique_cols),
			"unique": True,
			"partial": None,
			"file": file_name,
			"line": start_line,
		})


#============================================
def _parse_create_enum(
		file_name: str,
		start_line: int,
		statement: str,
		catalog: dict,
		) -> None:
	"""
	Parse CREATE TYPE ... AS ENUM.

	Args:
		file_name: SQL basename.
		start_line: Statement line.
		statement: Full statement.
		catalog: Catalog.
	"""
	index = schema_catalog_scan._skip_keywords(statement, 0, ["CREATE", "TYPE"])
	qualified, index = schema_catalog_scan._read_qualified_name(statement, index)
	index = schema_catalog_scan._skip_keywords(statement, index, ["AS", "ENUM"])
	index = schema_catalog_scan._skip_space(statement, index)
	labels = []
	if index < len(statement) and statement[index] == "(":
		inner_close = schema_catalog_scan._matching_paren(statement, index)
		inner = statement[index + 1:inner_close]
		for piece in schema_catalog_scan._split_top_level_commas(inner):
			label = piece.strip().strip("'")
			if label:
				labels.append(label)
	catalog["enums"][qualified] = {
		"file": file_name,
		"line": start_line,
		"labels": labels,
	}


#============================================
def _parse_create_domain(
		file_name: str,
		start_line: int,
		statement: str,
		catalog: dict,
		) -> None:
	"""
	Parse CREATE DOMAIN.

	Args:
		file_name: SQL basename.
		start_line: Statement line.
		statement: Full statement.
		catalog: Catalog.
	"""
	index = schema_catalog_scan._skip_keywords(statement, 0, ["CREATE", "DOMAIN"])
	qualified, index = schema_catalog_scan._read_qualified_name(statement, index)
	index = schema_catalog_scan._skip_keywords(statement, index, ["AS"])
	index = schema_catalog_scan._skip_space(statement, index)
	type_start = index
	while index < len(statement):
		index = schema_catalog_scan._skip_space(statement, index)
		if index >= len(statement):
			break
		match = schema_catalog_scan.IDENT_RE.match(statement, index)
		if not match:
			break
		if match.group(0).upper() in ("CHECK", "NOT", "COLLATE", "DEFAULT"):
			break
		index = match.end()
	base_type = " ".join(statement[type_start:index].split())
	check_text = _extract_check_clause(statement[index:])
	catalog["domains"][qualified] = {
		"file": file_name,
		"line": start_line,
		"base_type": base_type,
		"check": check_text,
	}


#============================================
def _parse_create_index(
		file_name: str,
		start_line: int,
		statement: str,
		catalog: dict,
		) -> None:
	"""
	Parse CREATE INDEX.

	Args:
		file_name: SQL basename.
		start_line: Statement line.
		statement: Full statement.
		catalog: Catalog.
	"""
	unique = "UNIQUE" in _leading_keywords(statement, 3)
	index = schema_catalog_scan._skip_keywords(
		statement, 0, ["CREATE", "UNIQUE", "INDEX", "IF", "NOT", "EXISTS"]
	)
	name, index = schema_catalog_scan._read_ident(statement, index)
	index = schema_catalog_scan._skip_keywords(statement, index, ["ON"])
	table_name, index = schema_catalog_scan._read_qualified_name(statement, index)
	index = schema_catalog_scan._skip_space(statement, index)
	columns = []
	if index < len(statement) and statement[index] == "(":
		columns = _paren_ident_list(statement[index:])
	partial = None
	where = re.search(r"\bWHERE\b", statement, re.IGNORECASE)
	if where:
		partial = statement[where.end():].strip()
	catalog["indexes"].append({
		"name": name,
		"table": table_name,
		"columns": columns,
		"unique": unique,
		"partial": partial,
		"file": file_name,
		"line": start_line,
	})


#============================================
def _parse_alter_foreign_key(
		file_name: str,
		start_line: int,
		statement: str,
		catalog: dict,
		) -> None:
	"""
	Parse ALTER TABLE ... ADD [CONSTRAINT name] FOREIGN KEY.

	Args:
		file_name: SQL basename.
		start_line: Statement line.
		statement: Full statement.
		catalog: Catalog.
	"""
	index = schema_catalog_scan._skip_keywords(statement, 0, ["ALTER", "TABLE"])
	index = schema_catalog_scan._skip_keywords(statement, index, ["ONLY"])
	table_name, index = schema_catalog_scan._read_qualified_name(statement, index)
	fk_match = re.search(r"\bFOREIGN\s+KEY\b", statement, re.IGNORECASE)
	if not fk_match:
		return
	fk = _parse_foreign_key_clause(statement[fk_match.start():], file_name, start_line)
	if fk is None:
		return
	if table_name not in catalog["tables"]:
		return
	catalog["tables"][table_name]["foreign_keys"].append(fk)


#============================================
def _parse_comment_on_table(statement: str, catalog: dict) -> None:
	"""
	Parse COMMENT ON TABLE and store comment plus role tag.

	Args:
		statement: Full statement.
		catalog: Catalog.
	"""
	index = schema_catalog_scan._skip_keywords(statement, 0, ["COMMENT", "ON", "TABLE"])
	qualified, index = schema_catalog_scan._read_qualified_name(statement, index)
	index = schema_catalog_scan._skip_keywords(statement, index, ["IS"])
	comment = _read_sql_string(statement, index)
	if qualified not in catalog["tables"]:
		return
	catalog["tables"][qualified]["comment"] = comment
	catalog["tables"][qualified]["role"] = _role_from_comment(comment)


#============================================
def _parse_comment_on_column(statement: str, catalog: dict) -> None:
	"""
	Parse COMMENT ON COLUMN schema.table.col.

	Args:
		statement: Full statement.
		catalog: Catalog.
	"""
	index = schema_catalog_scan._skip_keywords(statement, 0, ["COMMENT", "ON", "COLUMN"])
	schema, index = schema_catalog_scan._read_ident(statement, index)
	index = schema_catalog_scan._skip_space(statement, index)
	if index < len(statement) and statement[index] == ".":
		index += 1
	table, index = schema_catalog_scan._read_ident(statement, index)
	index = schema_catalog_scan._skip_space(statement, index)
	if index < len(statement) and statement[index] == ".":
		index += 1
	column_name, index = schema_catalog_scan._read_ident(statement, index)
	index = schema_catalog_scan._skip_keywords(statement, index, ["IS"])
	comment = _read_sql_string(statement, index)
	qualified = schema + "." + table
	if qualified not in catalog["tables"]:
		return
	for column in catalog["tables"][qualified]["columns"]:
		if column["name"] == column_name:
			column["comment"] = comment
			return


#============================================
def _role_from_comment(comment: str | None) -> str | None:
	"""
	Parse a role tag from a table comment that begins with role:.

	Args:
		comment: Table comment text.

	Returns:
		str | None: Role tag, or None.
	"""
	if comment is None:
		return None
	stripped = comment.strip()
	if not stripped.lower().startswith("role:"):
		return None
	rest = stripped[5:].strip()
	role = rest.split(",", 1)[0].strip()
	if not role:
		return None
	return role


#============================================
def _read_sql_string(text: str, index: int) -> str | None:
	"""
	Read a SQL string literal at index.

	Args:
		text: Statement text.
		index: Offset after IS.

	Returns:
		str | None: Unquoted string, or None.
	"""
	index = schema_catalog_scan._skip_space(text, index)
	if index >= len(text):
		return None
	if text[index] != "'":
		return None
	index += 1
	chunks = []
	while index < len(text):
		ch = text[index]
		if ch == "'" and index + 1 < len(text) and text[index + 1] == "'":
			chunks.append("'")
			index += 2
			continue
		if ch == "'":
			break
		chunks.append(ch)
		index += 1
	value = "".join(chunks)
	return value


#============================================
def _strip_constraint_name(item: str) -> str:
	"""
	Drop a leading CONSTRAINT name from a table item.

	Args:
		item: Table-body item.

	Returns:
		str: Remainder starting at PRIMARY/UNIQUE/FOREIGN/CHECK or the original.
	"""
	index = schema_catalog_scan._skip_space(item, 0)
	if not item[index:].upper().startswith("CONSTRAINT"):
		return item
	index = schema_catalog_scan._skip_keywords(item, index, ["CONSTRAINT"])
	_name, index = schema_catalog_scan._read_ident(item, index)
	rest = item[index:].strip()
	return rest


#============================================
def _paren_ident_list(text: str) -> list:
	"""
	Read identifiers from the first parenthesized list in text.

	Args:
		text: Text beginning at or containing (a, b).

	Returns:
		list: Identifier names.
	"""
	index = text.find("(")
	if index < 0:
		return []
	close = schema_catalog_scan._matching_paren(text, index)
	inner = text[index + 1:close]
	names = []
	for piece in schema_catalog_scan._split_top_level_commas(inner):
		token = piece.strip()
		if not token:
			continue
		name, _end = schema_catalog_scan._read_ident(token, 0)
		if name:
			names.append(name)
	return names


#============================================
