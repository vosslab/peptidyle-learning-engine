"""Load the schema catalog model from a live PostgreSQL database via psql."""

# Standard Library
import json
import subprocess

# local repo modules
import schema_style.schema_catalog_parse as schema_catalog_parse


DATABASE_SCHEMA_LIST = "('ple_data','ple_private','ple_audit','ple_migration')"
DATABASE_TABLES_SQL = (
	"SELECT json_agg(json_build_object("
	"'schema', n.nspname, 'name', c.relname) ORDER BY n.nspname, c.relname) "
	"FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace "
	"WHERE n.nspname IN " + DATABASE_SCHEMA_LIST + " AND c.relkind IN ('r','p')"
)


def load_from_database(database_name: str) -> dict:
	"""
	Load the catalog model from a live database through psql.

	Args:
		database_name: PostgreSQL database name (-d).

	Returns:
		dict: Catalog model using the same keys as load_from_source.

	Raises:
		subprocess.CalledProcessError: When psql fails.
	"""
	raw = _psql_json(database_name, DATABASE_TABLES_SQL)
	rows = json.loads(raw)
	if rows is None:
		rows = []
	catalog = schema_catalog_parse.empty_catalog()
	for row in rows:
		qualified = row["schema"] + "." + row["name"]
		catalog["tables"][qualified] = {
			"file": "",
			"line": 0,
			"columns": _database_columns(database_name, row["schema"], row["name"]),
			"primary_key": _database_key(
				database_name, row["schema"], row["name"], "p"
			),
			"uniques": _database_uniques(
				database_name, row["schema"], row["name"]
			),
			"foreign_keys": _database_foreign_keys(
				database_name, row["schema"], row["name"]
			),
			"comment": _database_table_comment(
				database_name, row["schema"], row["name"]
			),
			"role": None,
		}
		comment = catalog["tables"][qualified]["comment"]
		catalog["tables"][qualified]["role"] = schema_catalog_parse._role_from_comment(comment)
	catalog["indexes"] = _database_indexes(database_name)
	catalog["enums"] = _database_enums(database_name)
	catalog["domains"] = _database_domains(database_name)
	return catalog


#============================================
def _psql_json(database_name: str, sql: str) -> str:
	"""
	Run psql and return stdout.

	Args:
		database_name: Database name.
		sql: SQL that prints one JSON value.

	Returns:
		str: stdout.

	Raises:
		subprocess.CalledProcessError: When psql fails.
	"""
	command = [
		"psql",
		"-d", database_name,
		"-v", "ON_ERROR_STOP=1",
		"-At",
		"-c", sql,
	]
	completed = subprocess.run(command, capture_output=True, text=True, check=True)
	return completed.stdout


#============================================
def _database_columns(database_name: str, schema: str, table: str) -> list:
	"""
	Load columns for one table from pg_catalog.

	Args:
		database_name: Database name.
		schema: Schema name.
		table: Table name.

	Returns:
		list: Column dicts.
	"""
	sql = (
		"SELECT json_agg(json_build_object("
		"'name', a.attname, "
		"'type', pg_catalog.format_type(a.atttypid, a.atttypmod), "
		"'not_null', a.attnotnull, "
		"'check', NULL, "
		"'comment', col_description(a.attrelid, a.attnum)"
		") ORDER BY a.attnum) "
		"FROM pg_attribute a "
		"JOIN pg_class c ON c.oid = a.attrelid "
		"JOIN pg_namespace n ON n.oid = c.relnamespace "
		"WHERE n.nspname = '" + schema + "' AND c.relname = '" + table + "' "
		"AND a.attnum > 0 AND NOT a.attisdropped"
	)
	raw = _psql_json(database_name, sql)
	rows = json.loads(raw)
	if rows is None:
		return []
	return rows


#============================================
def _database_key(
		database_name: str,
		schema: str,
		table: str,
		kind: str,
		) -> list:
	"""
	Load primary-key column names from pg_constraint.

	Args:
		database_name: Database name.
		schema: Schema name.
		table: Table name.
		kind: Constraint type character, p for primary key.

	Returns:
		list: Column names.
	"""
	sql = (
		"SELECT json_agg(a.attname ORDER BY x.ordinality) "
		"FROM pg_constraint k "
		"JOIN pg_class c ON c.oid = k.conrelid "
		"JOIN pg_namespace n ON n.oid = c.relnamespace "
		"JOIN LATERAL unnest(k.conkey) WITH ORDINALITY AS x(attnum, ordinality) "
		"ON true "
		"JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = x.attnum "
		"WHERE n.nspname = '" + schema + "' AND c.relname = '" + table + "' "
		"AND k.contype = '" + kind + "'"
	)
	raw = _psql_json(database_name, sql)
	rows = json.loads(raw)
	if rows is None:
		return []
	return rows


#============================================
def _database_uniques(database_name: str, schema: str, table: str) -> list:
	"""
	Load UNIQUE constraint column lists, excluding the primary key.

	Args:
		database_name: Database name.
		schema: Schema name.
		table: Table name.

	Returns:
		list: Lists of column names.
	"""
	sql = (
		"SELECT json_agg(cols) FROM ("
		"SELECT json_agg(a.attname ORDER BY x.ordinality) AS cols "
		"FROM pg_constraint k "
		"JOIN pg_class c ON c.oid = k.conrelid "
		"JOIN pg_namespace n ON n.oid = c.relnamespace "
		"JOIN LATERAL unnest(k.conkey) WITH ORDINALITY AS x(attnum, ordinality) "
		"ON true "
		"JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = x.attnum "
		"WHERE n.nspname = '" + schema + "' AND c.relname = '" + table + "' "
		"AND k.contype = 'u' "
		"GROUP BY k.oid) t"
	)
	raw = _psql_json(database_name, sql)
	rows = json.loads(raw)
	if rows is None:
		return []
	return rows


#============================================
def _database_foreign_keys(database_name: str, schema: str, table: str) -> list:
	"""
	Load foreign keys for one table from pg_constraint.

	Args:
		database_name: Database name.
		schema: Schema name.
		table: Table name.

	Returns:
		list: Foreign key dicts.
	"""
	sql = (
		"SELECT json_agg(json_build_object("
		"'columns', child_cols, "
		"'parent', n2.nspname || '.' || c2.relname, "
		"'parent_columns', parent_cols, "
		"'file', '', "
		"'line', 0"
		")) FROM ("
		"SELECT k.oid, "
		"json_agg(a.attname ORDER BY x.ordinality) AS child_cols, "
		"json_agg(a2.attname ORDER BY x.ordinality) AS parent_cols, "
		"k.confrelid "
		"FROM pg_constraint k "
		"JOIN pg_class c ON c.oid = k.conrelid "
		"JOIN pg_namespace n ON n.oid = c.relnamespace "
		"JOIN LATERAL unnest(k.conkey, k.confkey) WITH ORDINALITY "
		"AS x(attnum, fattnum, ordinality) ON true "
		"JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = x.attnum "
		"JOIN pg_attribute a2 ON a2.attrelid = k.confrelid AND a2.attnum = x.fattnum "
		"WHERE n.nspname = '" + schema + "' AND c.relname = '" + table + "' "
		"AND k.contype = 'f' "
		"GROUP BY k.oid, k.confrelid"
		") s "
		"JOIN pg_class c2 ON c2.oid = s.confrelid "
		"JOIN pg_namespace n2 ON n2.oid = c2.relnamespace"
	)
	raw = _psql_json(database_name, sql)
	rows = json.loads(raw)
	if rows is None:
		return []
	return rows


#============================================
def _database_table_comment(database_name: str, schema: str, table: str) -> str | None:
	"""
	Load COMMENT ON TABLE from the catalog.

	Args:
		database_name: Database name.
		schema: Schema name.
		table: Table name.

	Returns:
		str | None: Comment text.
	"""
	sql = (
		"SELECT to_json(obj_description(c.oid, 'pg_class')) "
		"FROM pg_class c "
		"JOIN pg_namespace n ON n.oid = c.relnamespace "
		"WHERE n.nspname = '" + schema + "' AND c.relname = '" + table + "'"
	)
	raw = _psql_json(database_name, sql).strip()
	if raw == "" or raw == "null":
		return None
	comment = json.loads(raw)
	return comment


#============================================
def _database_indexes(database_name: str) -> list:
	"""
	Load indexes for PLE schemas from pg_index.

	Args:
		database_name: Database name.

	Returns:
		list: Index dicts matching the source catalog model.
	"""
	sql = (
		"SELECT json_agg(json_build_object("
		"'name', index_name, "
		"'table', schema_name || '.' || table_name, "
		"'columns', COALESCE(columns, '[]'::json), "
		"'unique', is_unique, "
		"'partial', partial, "
		"'file', '', "
		"'line', 0"
		") ORDER BY schema_name, table_name, index_name) "
		"FROM ("
		"SELECT n.nspname AS schema_name, "
		"t.relname AS table_name, "
		"i.relname AS index_name, "
		"ix.indisunique AS is_unique, "
		"pg_get_expr(ix.indpred, ix.indrelid) AS partial, "
		"("
		"SELECT json_agg(a.attname ORDER BY x.ordinality) "
		"FROM unnest(ix.indkey) WITH ORDINALITY AS x(attnum, ordinality) "
		"JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = x.attnum "
		"WHERE x.attnum > 0"
		") AS columns "
		"FROM pg_index ix "
		"JOIN pg_class i ON i.oid = ix.indexrelid "
		"JOIN pg_class t ON t.oid = ix.indrelid "
		"JOIN pg_namespace n ON n.oid = t.relnamespace "
		"WHERE n.nspname IN " + DATABASE_SCHEMA_LIST + " "
		"AND t.relkind IN ('r','p')"
		") s"
	)
	raw = _psql_json(database_name, sql)
	rows = json.loads(raw)
	if rows is None:
		return []
	return rows


#============================================
def _database_enums(database_name: str) -> dict:
	"""
	Load enum types for PLE schemas from pg_enum.

	Args:
		database_name: Database name.

	Returns:
		dict: Qualified enum name -> {file, line, labels}.
	"""
	sql = (
		"SELECT COALESCE(json_object_agg("
		"schema_name || '.' || type_name, "
		"json_build_object('file', '', 'line', 0, 'labels', labels)"
		"), '{}'::json) "
		"FROM ("
		"SELECT n.nspname AS schema_name, t.typname AS type_name, "
		"json_agg(e.enumlabel ORDER BY e.enumsortorder) AS labels "
		"FROM pg_type t "
		"JOIN pg_enum e ON e.enumtypid = t.oid "
		"JOIN pg_namespace n ON n.oid = t.typnamespace "
		"WHERE n.nspname IN " + DATABASE_SCHEMA_LIST + " "
		"GROUP BY n.nspname, t.typname"
		") s"
	)
	raw = _psql_json(database_name, sql)
	enums = json.loads(raw)
	if enums is None:
		return {}
	return enums


#============================================
def _database_domains(database_name: str) -> dict:
	"""
	Load domains for PLE schemas from pg_type.

	Args:
		database_name: Database name.

	Returns:
		dict: Qualified domain name -> {file, line, base_type, check}.
	"""
	sql = (
		"SELECT COALESCE(json_object_agg("
		"n.nspname || '.' || t.typname, "
		"json_build_object("
		"'file', '', "
		"'line', 0, "
		"'base_type', pg_catalog.format_type(t.typbasetype, t.typtypmod), "
		"'check', ("
		"SELECT pg_get_constraintdef(c.oid) "
		"FROM pg_constraint c "
		"WHERE c.contypid = t.oid AND c.contype = 'c' "
		"LIMIT 1"
		")"
		")"
		"), '{}'::json) "
		"FROM pg_type t "
		"JOIN pg_namespace n ON n.oid = t.typnamespace "
		"WHERE n.nspname IN " + DATABASE_SCHEMA_LIST + " AND t.typtype = 'd'"
	)
	raw = _psql_json(database_name, sql)
	domains = json.loads(raw)
	if domains is None:
		return {}
	return domains
