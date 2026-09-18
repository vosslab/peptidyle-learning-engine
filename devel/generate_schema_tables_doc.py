#!/usr/bin/env python3
"""Generate docs/SCHEMA_TABLES.md and schemas/catalog_snapshot.json from the catalog.

Reads SQL source under schemas/base_schema/ by default, or a live database when
-d/--database is given. Run:

	source source_me.sh && python3 devel/generate_schema_tables_doc.py
"""

# Standard Library
import json
import signal
import argparse
import pathlib

# local repo modules
import schema_style.schema_catalog_lib as schema_catalog_lib


#============================================
def parse_args() -> argparse.Namespace:
	"""
	Parse command-line arguments.

	Returns:
		argparse.Namespace: Parsed flags.
	"""
	parser = argparse.ArgumentParser(
		description="Generate SCHEMA_TABLES.md and catalog_snapshot.json"
	)
	parser.add_argument(
		"-s", "--source-dir", dest="source_dir", default="schemas/base_schema",
		help="SQL source directory",
	)
	parser.add_argument(
		"-d", "--database", dest="database", default=None,
		help="Live PostgreSQL database name",
	)
	parser.add_argument(
		"-o", "--output", dest="output_file", default="docs/SCHEMA_TABLES.md",
		help="Markdown output path",
	)
	parser.add_argument(
		"-j", "--snapshot", dest="snapshot",
		default="schemas/catalog_snapshot.json",
		help="Catalog snapshot JSON path",
	)
	args = parser.parse_args()
	return args


#============================================
def load_catalog(args: argparse.Namespace) -> dict:
	"""
	Load the catalog from a live database or from SQL source.

	Args:
		args: Parsed flags.

	Returns:
		dict: Catalog model.
	"""
	if args.database is not None:
		catalog = schema_catalog_lib.load_from_database(args.database)
		return catalog
	catalog = schema_catalog_lib.load_from_source(args.source_dir)
	return catalog


#============================================
def attach_source_locations(catalog: dict, source_dir: str) -> None:
	"""
	Copy file and line from the source parse onto matching catalog tables.

	Live-database tables have empty file names. Source locations keep Markdown
	sections grouped by SQL module.

	Args:
		catalog: Catalog being documented.
		source_dir: SQL source directory.
	"""
	source_path = pathlib.Path(source_dir)
	if not source_path.is_dir():
		return
	source_catalog = schema_catalog_lib.load_from_source(source_dir)
	for qualified, table in catalog["tables"].items():
		if qualified not in source_catalog["tables"]:
			continue
		source_table = source_catalog["tables"][qualified]
		if table["file"]:
			continue
		table["file"] = source_table["file"]
		table["line"] = source_table["line"]


#============================================
def indexes_by_table(catalog: dict) -> dict:
	"""
	Group catalog indexes by qualified table name.

	Args:
		catalog: Catalog model.

	Returns:
		dict: table name -> list of index dicts.
	"""
	grouped = {}
	for index in catalog["indexes"]:
		table_name = index["table"]
		if table_name not in grouped:
			grouped[table_name] = []
		grouped[table_name].append(index)
	return grouped


#============================================
def section_files(source_dir: str, catalog: dict) -> list:
	"""
	Return Markdown section file names.

	Uses 20_tables/*.sql when that directory exists, otherwise every source
	file that contains a parsed CREATE TABLE.

	Args:
		source_dir: SQL source directory.
		catalog: Catalog model.

	Returns:
		list: Relative POSIX paths.
	"""
	tables_dir = pathlib.Path(source_dir) / "20_tables"
	if tables_dir.is_dir():
		files = []
		for path in sorted(tables_dir.rglob("*.sql")):
			rel = path.relative_to(source_dir).as_posix()
			files.append(rel)
		return files
	seen = {}
	for table in catalog["tables"].values():
		file_name = table["file"]
		if file_name:
			seen[file_name] = True
	files = sorted(seen)
	return files


#============================================
def table_sort_key(item: tuple) -> tuple:
	"""
	Sort key for (qualified_name, table) pairs.

	Args:
		item: (qualified name, table dict).

	Returns:
		tuple: (line, qualified name).
	"""
	qualified, table = item
	key = (table["line"], qualified)
	return key


#============================================
def tables_for_file(catalog: dict, file_name: str) -> list:
	"""
	Return tables whose source file matches file_name.

	Args:
		catalog: Catalog model.
		file_name: Relative SQL path.

	Returns:
		list: (qualified name, table dict) pairs.
	"""
	items = []
	for qualified, table in catalog["tables"].items():
		if table["file"] == file_name:
			items.append((qualified, table))
	items.sort(key=table_sort_key)
	return items


#============================================
def tables_outside_files(catalog: dict, files: list) -> list:
	"""
	Return tables whose file is not in the section file list.

	Args:
		catalog: Catalog model.
		files: Section file names.

	Returns:
		list: (qualified name, table dict) pairs.
	"""
	file_set = set(files)
	items = []
	for qualified, table in catalog["tables"].items():
		if table["file"] in file_set:
			continue
		items.append((qualified, table))
	items.sort(key=table_sort_key)
	return items


#============================================
def ascii_text(value: str) -> str:
	"""
	Replace non-ASCII characters with HTML numeric character references.

	Args:
		value: Raw text.

	Returns:
		str: ASCII text.
	"""
	encoded = value.encode("ascii", "xmlcharrefreplace").decode("ascii")
	return encoded


#============================================
def flatten_text(value: str | None) -> str:
	"""
	Collapse whitespace and force ASCII for one Markdown cell or bullet.

	Args:
		value: Raw text, or None.

	Returns:
		str: Single-line ASCII text, or none when empty.
	"""
	if value is None:
		return "none"
	parts = value.split()
	if not parts:
		return "none"
	flat = " ".join(parts)
	flat = flat.replace("|", "/")
	flat = ascii_text(flat)
	return flat


#============================================
def format_columns(table: dict) -> list:
	"""
	Format the column table for one catalog table.

	Args:
		table: Table dict.

	Returns:
		list: Markdown lines.
	"""
	lines = ["Columns:", "", "| Name | Type | Null |", "| --- | --- | --- |"]
	if not table["columns"]:
		lines.append("| none |  |  |")
		return lines
	for column in table["columns"]:
		null_label = "NOT NULL" if column["not_null"] else "NULL"
		name = flatten_text(column["name"])
		col_type = flatten_text(column["type"])
		lines.append(f"| {name} | {col_type} | {null_label} |")
	return lines


#============================================
def format_constraints(table: dict) -> list:
	"""
	Format PRIMARY KEY, UNIQUE, and column CHECK constraints.

	Args:
		table: Table dict.

	Returns:
		list: Markdown lines.
	"""
	lines = ["Constraints:", ""]
	items = []
	if table["primary_key"]:
		cols = ", ".join(table["primary_key"])
		items.append(f"- PRIMARY KEY ({cols})")
	for unique_cols in table["uniques"]:
		cols = ", ".join(unique_cols)
		items.append(f"- UNIQUE ({cols})")
	for column in table["columns"]:
		check = column["check"]
		if not check:
			continue
		expr = flatten_text(check)
		items.append(f"- CHECK {column['name']}: `{expr}`")
	if not items:
		lines.append("- none")
		return lines
	lines.extend(items)
	return lines


#============================================
def format_foreign_keys(table: dict) -> list:
	"""
	Format foreign keys for one table.

	Args:
		table: Table dict.

	Returns:
		list: Markdown lines.
	"""
	lines = ["Foreign keys:", ""]
	if not table["foreign_keys"]:
		lines.append("- none")
		return lines
	for fk in table["foreign_keys"]:
		child = ", ".join(fk["columns"])
		parent_cols = ", ".join(fk["parent_columns"])
		parent = fk["parent"]
		lines.append(f"- ({child}) -> {parent} ({parent_cols})")
	return lines


#============================================
def format_indexes(index_list: list) -> list:
	"""
	Format indexes covering one table.

	Args:
		index_list: Index dicts for this table.

	Returns:
		list: Markdown lines.
	"""
	lines = ["Indexes:", ""]
	if not index_list:
		lines.append("- none")
		return lines
	for index in index_list:
		cols = ", ".join(index["columns"])
		unique = " UNIQUE" if index["unique"] else ""
		partial = index["partial"]
		where = ""
		if partial:
			where = " WHERE " + flatten_text(partial)
		lines.append(f"- {index['name']}{unique} ({cols}){where}")
	return lines


#============================================
def format_one_table(qualified: str, table: dict, index_list: list) -> list:
	"""
	Format one table heading and its facts.

	Args:
		qualified: schema.table.
		table: Table dict.
		index_list: Indexes on this table.

	Returns:
		list: Markdown lines.
	"""
	role = table["role"] if table["role"] else "none"
	comment = flatten_text(table["comment"])
	lines = [
		f"### {qualified}",
		"",
		f"- Role: {role}",
		f"- Comment: {comment}",
		"",
	]
	lines.extend(format_columns(table))
	lines.append("")
	lines.extend(format_constraints(table))
	lines.append("")
	lines.extend(format_foreign_keys(table))
	lines.append("")
	lines.extend(format_indexes(index_list))
	lines.append("")
	return lines


#============================================
def format_section(title: str, items: list, grouped_indexes: dict) -> list:
	"""
	Format one ## file section.

	Args:
		title: Section heading text.
		items: (qualified name, table dict) pairs.
		grouped_indexes: table name -> index list.

	Returns:
		list: Markdown lines.
	"""
	lines = [f"## {title}", ""]
	if not items:
		lines.append("No tables.")
		lines.append("")
		return lines
	for qualified, table in items:
		index_list = grouped_indexes.get(qualified, [])
		lines.extend(format_one_table(qualified, table, index_list))
	return lines


#============================================
def format_markdown(catalog: dict, source_dir: str) -> str:
	"""
	Build SCHEMA_TABLES.md from the catalog.

	Args:
		catalog: Catalog model.
		source_dir: SQL source directory, used to choose section files.

	Returns:
		str: Markdown document.
	"""
	grouped_indexes = indexes_by_table(catalog)
	files = section_files(source_dir, catalog)
	lines = [
		"# Schema tables",
		"",
		"Generated by "
		+ "[../devel/generate_schema_tables_doc.py]"
		+ "(../devel/generate_schema_tables_doc.py) "
		+ "from the SQL catalog. Do not edit by hand. Style rules live in "
		+ "[DATABASE_STYLE.md](DATABASE_STYLE.md). Usage is in "
		+ "[USAGE.md](USAGE.md).",
		"",
	]
	for file_name in files:
		items = tables_for_file(catalog, file_name)
		lines.extend(format_section(file_name, items, grouped_indexes))
	leftovers = tables_outside_files(catalog, files)
	if leftovers:
		lines.extend(format_section("Other tables", leftovers, grouped_indexes))
	text = "\n".join(lines)
	if not text.endswith("\n"):
		text += "\n"
	return text


#============================================
def write_text(path_name: str, text: str) -> None:
	"""
	Write text to path_name, creating parent directories.

	Args:
		path_name: Destination path.
		text: File contents.
	"""
	path = pathlib.Path(path_name)
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text(text)


#============================================
def write_snapshot(path_name: str, catalog: dict) -> None:
	"""
	Write the catalog model as sorted JSON.

	Args:
		path_name: Destination path.
		catalog: Catalog model.
	"""
	payload = json.dumps(catalog, indent=2, sort_keys=True, ensure_ascii=True)
	payload += "\n"
	write_text(path_name, payload)


#============================================
def main() -> None:
	"""Load the catalog and write the Markdown doc plus JSON snapshot."""
	signal.signal(signal.SIGPIPE, signal.SIG_DFL)
	args = parse_args()
	catalog = load_catalog(args)
	if args.database is not None:
		attach_source_locations(catalog, args.source_dir)
	markdown = format_markdown(catalog, args.source_dir)
	write_text(args.output_file, markdown)
	write_snapshot(args.snapshot, catalog)
	print("wrote " + args.output_file)
	print("wrote " + args.snapshot)


if __name__ == '__main__':
	main()
