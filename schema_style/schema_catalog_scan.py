"""Scan PostgreSQL SQL text: comments, statements, identifiers, and parens."""

# Standard Library
import pathlib
import re


COLUMN_STOP = (
	"NOT",
	"NULL",
	"DEFAULT",
	"CHECK",
	"REFERENCES",
	"PRIMARY",
	"UNIQUE",
	"GENERATED",
	"COLLATE",
	"CONSTRAINT",
)
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
DOLLAR_TAG_RE = re.compile(r"\$[A-Za-z0-9_]*\$")

def _strip_comments_and_quotes(text: str) -> str:
	"""
	Replace comments and dollar-quoted bodies with spaces.

	Used so CREATE TABLE counts ignore comments and function bodies.

	Args:
		text: SQL file contents.

	Returns:
		str: Text with comments and dollar quotes blanked.
	"""
	chars = []
	index = 0
	length = len(text)
	while index < length:
		ch = text[index]
		if ch == "-" and index + 1 < length and text[index + 1] == "-":
			while index < length and text[index] != "\n":
				chars.append(" ")
				index += 1
			continue
		if ch == "/" and index + 1 < length and text[index + 1] == "*":
			chars.append(" ")
			chars.append(" ")
			index += 2
			while index + 1 < length and text[index:index + 2] != "*/":
				chars.append("\n" if text[index] == "\n" else " ")
				index += 1
			if index + 1 < length:
				chars.append(" ")
				chars.append(" ")
				index += 2
			continue
		if ch == "'":
			end, _line = _skip_string(text, index, 1)
			chars.append(" " * (end - index))
			index = end
			continue
		dollar = DOLLAR_TAG_RE.match(text, index)
		if dollar:
			end, _line = _skip_dollar(text, index, 1, dollar.group(0))
			chars.append(" " * (end - index))
			index = end
			continue
		chars.append(ch)
		index += 1
	stripped = "".join(chars)
	return stripped


#============================================
def _sql_paths(source_dir: str) -> list:
	"""
	Return sorted *.sql paths under source_dir, including subdirectories.

	Args:
		source_dir: Directory of SQL modules.

	Returns:
		list: pathlib.Path objects.
	"""
	directory = pathlib.Path(source_dir)
	paths = sorted(path for path in directory.rglob("*.sql") if path.is_file())
	return paths


#============================================
def _relative_sql_path(source_dir: str, path: pathlib.Path) -> str:
	"""
	Return path relative to source_dir using POSIX separators.

	Args:
		source_dir: Directory of SQL modules.
		path: Absolute or relative SQL file path.

	Returns:
		str: Relative POSIX path, for example 20_tables/course.sql.
	"""
	relative = path.relative_to(pathlib.Path(source_dir)).as_posix()
	return relative


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
		index, line = _skip_trivia(text, index, line)
		if index >= length:
			break
		start_line = line
		start = index
		index, line = _scan_statement(text, index, line)
		raw = text[start:index].strip()
		if raw.endswith(";"):
			raw = raw[:-1].strip()
		if raw:
			statements.append((start_line, raw))
	return statements


#============================================
def _skip_trivia(text: str, index: int, line: int) -> tuple:
	"""
	Skip whitespace and comments.

	Args:
		text: Source text.
		index: Current offset.
		line: Current 1-based line.

	Returns:
		tuple: (new_index, new_line).
	"""
	length = len(text)
	while index < length:
		ch = text[index]
		if ch in " \t\r":
			index += 1
			continue
		if ch == "\n":
			index += 1
			line += 1
			continue
		if ch == "-" and index + 1 < length and text[index + 1] == "-":
			while index < length and text[index] != "\n":
				index += 1
			continue
		if ch == "/" and index + 1 < length and text[index + 1] == "*":
			index += 2
			while index + 1 < length and text[index:index + 2] != "*/":
				if text[index] == "\n":
					line += 1
				index += 1
			index = min(index + 2, length)
			continue
		break
	return index, line


#============================================
def _scan_statement(text: str, index: int, line: int) -> tuple:
	"""
	Advance from the start of a statement to the terminating semicolon.

	Args:
		text: Source text.
		index: Start offset.
		line: Start line.

	Returns:
		tuple: (offset after semicolon, line).
	"""
	length = len(text)
	paren = 0
	while index < length:
		ch = text[index]
		if ch == "-" and index + 1 < length and text[index + 1] == "-":
			while index < length and text[index] != "\n":
				index += 1
			continue
		if ch == "\n":
			index += 1
			line += 1
			continue
		if ch == "'":
			index, line = _skip_string(text, index, line)
			continue
		dollar = DOLLAR_TAG_RE.match(text, index)
		if dollar:
			index, line = _skip_dollar(text, index, line, dollar.group(0))
			continue
		if ch == "(":
			paren += 1
			index += 1
			continue
		if ch == ")":
			paren -= 1
			index += 1
			continue
		if ch == ";" and paren <= 0:
			index += 1
			break
		index += 1
	return index, line


#============================================
def _skip_string(text: str, index: int, line: int) -> tuple:
	"""
	Skip a single-quoted SQL string, including doubled quotes.

	Args:
		text: Source text.
		index: Offset of the opening quote.
		line: Current line.

	Returns:
		tuple: (offset after the string, line).
	"""
	index += 1
	length = len(text)
	while index < length:
		ch = text[index]
		if ch == "\n":
			line += 1
		if ch == "'" and index + 1 < length and text[index + 1] == "'":
			index += 2
			continue
		if ch == "'":
			index += 1
			break
		index += 1
	return index, line


#============================================
def _skip_dollar(text: str, index: int, line: int, tag: str) -> tuple:
	"""
	Skip a dollar-quoted body.

	Args:
		text: Source text.
		index: Offset of the opening tag.
		line: Current line.
		tag: Opening dollar tag, for example $$.

	Returns:
		tuple: (offset after the closing tag, line).
	"""
	index += len(tag)
	length = len(text)
	while index < length:
		if text.startswith(tag, index):
			index += len(tag)
			break
		if text[index] == "\n":
			line += 1
		index += 1
	return index, line


#============================================

def _split_top_level_commas(text: str) -> list:
	"""
	Split text on commas that are not inside parens or quotes.

	Args:
		text: CREATE TABLE body or similar.

	Returns:
		list: Pieces.
	"""
	parts = []
	start = 0
	index = 0
	paren = 0
	length = len(text)
	while index < length:
		ch = text[index]
		if ch == "-" and index + 1 < length and text[index + 1] == "-":
			while index < length and text[index] != "\n":
				index += 1
			continue
		if ch == "'":
			index, _line = _skip_string(text, index, 1)
			continue
		dollar = DOLLAR_TAG_RE.match(text, index)
		if dollar:
			index, _line = _skip_dollar(text, index, 1, dollar.group(0))
			continue
		if ch == "(":
			paren += 1
		elif ch == ")":
			paren -= 1
		elif ch == "," and paren == 0:
			parts.append(text[start:index])
			start = index + 1
		index += 1
	parts.append(text[start:])
	return parts


#============================================
def _matching_paren(text: str, index: int) -> int:
	"""
	Return the index of the closing paren matching text[index].

	Args:
		text: Source text.
		index: Offset of '('.

	Returns:
		int: Offset of the matching ')'.
	"""
	paren = 0
	length = len(text)
	pos = index
	while pos < length:
		ch = text[pos]
		if ch == "-" and pos + 1 < length and text[pos + 1] == "-":
			while pos < length and text[pos] != "\n":
				pos += 1
			continue
		if ch == "'":
			pos, _line = _skip_string(text, pos, 1)
			continue
		dollar = DOLLAR_TAG_RE.match(text, pos)
		if dollar:
			pos, _line = _skip_dollar(text, pos, 1, dollar.group(0))
			continue
		if ch == "(":
			paren += 1
		elif ch == ")":
			paren -= 1
			if paren == 0:
				return pos
		pos += 1
	return length - 1


#============================================
def _skip_space(text: str, index: int) -> int:
	"""
	Skip whitespace and -- line comments.

	Args:
		text: Source.
		index: Offset.

	Returns:
		int: New offset.
	"""
	length = len(text)
	while index < length:
		ch = text[index]
		if ch in " \t\r\n":
			index += 1
			continue
		if ch == "-" and index + 1 < length and text[index + 1] == "-":
			while index < length and text[index] != "\n":
				index += 1
			continue
		break
	return index


#============================================
def _skip_keywords(text: str, index: int, words: list) -> int:
	"""
	Skip any of the given keywords in order, repeatedly as they appear.

	Args:
		text: Source.
		index: Offset.
		words: Keyword list, matched case-insensitively when next.

	Returns:
		int: New offset.
	"""
	wanted = [word.upper() for word in words]
	index = _skip_space(text, index)
	progress = True
	while progress:
		progress = False
		for word in wanted:
			index = _skip_space(text, index)
			match = IDENT_RE.match(text, index)
			if match and match.group(0).upper() == word:
				index = match.end()
				progress = True
	return index


#============================================
def _read_ident(text: str, index: int) -> tuple:
	"""
	Read one identifier.

	Args:
		text: Source.
		index: Offset.

	Returns:
		tuple: (identifier, new_index).
	"""
	index = _skip_space(text, index)
	if index < len(text) and text[index] == '"':
		end = text.find('"', index + 1)
		if end < 0:
			name = text[index + 1:]
			return name, len(text)
		name = text[index + 1:end]
		return name, end + 1
	match = IDENT_RE.match(text, index)
	if not match:
		return "", index
	name = match.group(0)
	return name, match.end()


#============================================
def _read_qualified_name(text: str, index: int) -> tuple:
	"""
	Read schema.table or a single identifier.

	Args:
		text: Source.
		index: Offset.

	Returns:
		tuple: (qualified_name, new_index).
	"""
	first, index = _read_ident(text, index)
	index = _skip_space(text, index)
	if index < len(text) and text[index] == ".":
		index += 1
		second, index = _read_ident(text, index)
		qualified = first + "." + second
		return qualified, index
	return first, index

