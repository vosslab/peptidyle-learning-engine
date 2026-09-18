"""Parse PostgreSQL DDL into one catalog model for the schema style checker."""

# Standard Library
import json
import pathlib

# local repo modules
import schema_style.schema_catalog_database as schema_catalog_database
import schema_style.schema_catalog_parse as schema_catalog_parse
import schema_style.schema_catalog_scan as schema_catalog_scan


CREATE_TABLE_COUNT_TOKEN = "CREATE TABLE"


#============================================
def empty_catalog() -> dict:
	"""
	Return an empty catalog dict with the stable model keys.

	Returns:
		dict: Catalog with tables, enums, domains, and indexes.
	"""
	catalog = schema_catalog_parse.empty_catalog()
	return catalog


#============================================
def source_create_table_count(source_dir: str) -> int:
	"""
	Count CREATE TABLE occurrences in *.sql files, ignoring comments.

	Args:
		source_dir: Directory of SQL modules.

	Returns:
		int: Number of CREATE TABLE tokens outside comments and quotes.
	"""
	total = 0
	for path in schema_catalog_scan._sql_paths(source_dir):
		text = path.read_text()
		stripped = schema_catalog_scan._strip_comments_and_quotes(text)
		total += stripped.count(CREATE_TABLE_COUNT_TOKEN)
	return total


#============================================
def load_from_source(source_dir: str) -> dict:
	"""
	Parse CREATE TABLE and related DDL from a source directory.

	Args:
		source_dir: Directory containing *.sql modules.

	Returns:
		dict: Catalog model with tables, enums, domains, and indexes.
	"""
	catalog = empty_catalog()
	schema_catalog_parse.parse_source(source_dir, catalog)
	return catalog


#============================================
def load_from_snapshot(snapshot_path: str) -> dict:
	"""
	Load the catalog model from a JSON snapshot file.

	Args:
		snapshot_path: Path to schemas/catalog_snapshot.json or equivalent.

	Returns:
		dict: Catalog model.

	Raises:
		FileNotFoundError: When the snapshot path does not exist.
	"""
	path = pathlib.Path(snapshot_path)
	with path.open() as handle:
		catalog = json.load(handle)
	return catalog


#============================================
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
	catalog = schema_catalog_database.load_from_database(database_name)
	return catalog
