"""Mechanical DATABASE_STYLE.md rules over the schema catalog model."""

# Standard Library
import collections
import pathlib
import re

# local repo modules
import schema_style.schema_catalog_lib as schema_catalog_lib
import schema_style.schema_catalog_parse as schema_catalog_parse


Finding = collections.namedtuple("Finding", ["rule", "location", "message"])
ROLE_TAGS = (
	"current state",
	"revision",
	"event",
	"snapshot",
	"student work",
	"aggregate",
	"vocabulary",
)
IN_LIST_RE = re.compile(r"\bIN\s*\(", re.IGNORECASE)
IN_VALUES_RE = re.compile(r"\bIN\s*\((.*)\)\s*$", re.IGNORECASE | re.DOTALL)
EQ_ONE_RE = re.compile(
	r"^\(\s*[A-Za-z_][A-Za-z0-9_]*\s*=\s*'[^']*'"
	r"(?:::[A-Za-z_][A-Za-z0-9_.]*)?\s*\)$",
	re.IGNORECASE | re.DOTALL,
)
COMPOSITE_OK_NAMES = frozenset({"revision_number", "edit_number"})
RULE_TITLES = {
	"rule_table_count": "table count",
	"rule_layout": "source layout (17)",
	"rule_2_constant_columns": "constant columns (2)",
	"rule_4_types": "column types (4)",
	"rule_5_duplicate_literal_sets": "duplicate IN lists (5)",
	"rule_7_key_names": "key names (7)",
	"rule_8_identity": "identity (8)",
	"rule_9_null_meaning": "null meaning (9)",
	"rule_11_student_work_keys": "student work keys (11)",
	"rule_12_immutability": "immutability (12)",
	"rule_14_unindexed_fk": "unindexed FK (14)",
	"rule_16_clock_present": "missing clock (16)",
	"rule_16_updated_clock": "updated clock (16)",
	"rule_16_clock_type": "clock type (16)",
	"rule_17_role_tag": "role tag (17)",
}


#============================================
def rule_title(rule: str) -> str:
	"""
	Return the printed title for a rule id.

	Args:
		rule: Internal rule id such as rule_7_key_names.

	Returns:
		str: Short title with the DATABASE_STYLE.md checklist number.
	"""
	title = RULE_TITLES[rule]
	return title


#============================================
def make_finding(rule: str, table: dict, extra: str, message: str) -> Finding:
	"""
	Build a finding with source file:line appended to the message.

	Args:
		rule: Rule id.
		table: Table dict with file and line.
		extra: Object location.
		message: Finding text.

	Returns:
		Finding: rule, location, message with file:line.
	"""
	file_name = table["file"]
	line = table["line"]
	if file_name:
		message = f"{message}  {file_name}:{line}"
	item = Finding(rule, extra, message)
	return item


#============================================
def _posix_name(file_name: str) -> str:
	"""
	Return the POSIX basename of a source path.

	Args:
		file_name: Relative or basename path stored on a catalog object.

	Returns:
		str: Final path component.
	"""
	posix = file_name.replace("\\", "/")
	base = posix.rsplit("/", 1)[-1]
	return base


#============================================
def _composite_key_ok(column: str, parent: str) -> bool:
	"""
	Return whether a composite key column is named for its parent table.

	Args:
		column: Key column name.
		parent: Unqualified parent table name.

	Returns:
		bool: True when the column is parent_id, revision_number, *_position,
			or edit_number.
	"""
	if column.endswith(parent + "_id"):
		return True
	if column in COMPOSITE_OK_NAMES:
		return True
	if column.endswith("_position"):
		return True
	return False


#============================================
def _parent_table(parent: str) -> str:
	"""
	Return the unqualified table name from a parent reference.

	Args:
		parent: Qualified or unqualified parent table name.

	Returns:
		str: Last dotted component.
	"""
	name = parent.rsplit(".", 1)[-1]
	return name


#============================================
def _any_role(catalog: dict) -> bool:
	"""
	Return whether any table carries a role tag.

	Args:
		catalog: Catalog model.

	Returns:
		bool: True when at least one table has a role.
	"""
	for table in catalog["tables"].values():
		if table["role"]:
			return True
	return False


#============================================
def _layout_blocks(source_dir: str) -> bool:
	"""
	Return whether layout findings are blocking.

	Args:
		source_dir: SQL source directory.

	Returns:
		bool: True when 20_tables/ exists.
	"""
	path = pathlib.Path(source_dir) / "20_tables"
	exists = path.is_dir()
	return exists


#============================================
def _column_type(column: dict) -> str:
	"""
	Return the lowercased column type.

	Args:
		column: Column dict.

	Returns:
		str: Lowercased type string.
	"""
	lowered = column["type"].lower()
	return lowered


#============================================
def _is_uuid_or_public_id_type(col_type: str) -> bool:
	"""
	Return whether col_type is uuid, text, or a public-ID domain.

	Args:
		col_type: Lowercased type string.

	Returns:
		bool: True when the type is an allowed identity PK type.
	"""
	if col_type in ("uuid", "text") or "text" in col_type:
		return True
	if col_type.startswith("ple_data.") and col_type.endswith("_id"):
		return True
	return False


#============================================
def _is_clock_type(col_type: str) -> bool:
	"""
	Return whether a type is a creation clock.

	Args:
		col_type: Lowercased type.

	Returns:
		bool: True for timestamptz, timestamp with time zone, or date.
	"""
	if "timestamptz" in col_type:
		return True
	if "timestamp" in col_type and "time zone" in col_type:
		return True
	if col_type == "date" or col_type.startswith("date "):
		return True
	return False


#============================================
def rule_create_table_count(catalog: dict, source_dir: str) -> list:
	"""
	Finding when parsed tables disagree with the CREATE TABLE count.

	Args:
		catalog: Catalog model.
		source_dir: SQL source directory.

	Returns:
		list: Findings.
	"""
	parsed = len(catalog["tables"])
	counted = schema_catalog_lib.source_create_table_count(source_dir)
	if parsed == counted:
		return []
	item = Finding(
		"rule_table_count",
		source_dir,
		f"parsed {parsed} tables, CREATE TABLE count {counted}",
	)
	return [item]


#============================================
def rule_layout(catalog: dict, source_dir: str) -> list:
	"""
	CREATE TABLE outside 20_tables/; functions/grants inside it.

	Args:
		catalog: Catalog model.
		source_dir: SQL source directory.

	Returns:
		list: Findings.
	"""
	findings = []
	tables_dir = pathlib.Path(source_dir) / "20_tables"
	types_file = "10_types.sql"
	for qualified, table in catalog["tables"].items():
		file_name = table["file"].replace("\\", "/")
		if not file_name.startswith("20_tables/"):
			findings.append(make_finding("rule_layout", table, qualified, "CREATE TABLE outside 20_tables/",
			))
	if tables_dir.is_dir():
		for path in sorted(tables_dir.rglob("*.sql")):
			text = path.read_text()
			for _start, statement in schema_catalog_parse._iter_statements(text):
				keywords = schema_catalog_parse._leading_keywords(statement, 3)
				token = None
				if keywords[:2] == ["CREATE", "FUNCTION"]:
					token = "CREATE FUNCTION"
				elif keywords[:2] == ["CREATE", "POLICY"]:
					token = "CREATE POLICY"
				elif keywords[:1] == ["GRANT"]:
					token = "GRANT"
				elif keywords[:1] == ["REVOKE"]:
					token = "REVOKE"
				if token is not None:
					findings.append(Finding(
						"rule_layout",
						str(path),
						f"{token} inside 20_tables/",
					))
	for qualified, enum in catalog["enums"].items():
		if _posix_name(enum["file"]) != types_file:
			findings.append(Finding(
				"rule_layout",
				qualified,
				"enum outside 10_types.sql  " + enum["file"] + ":" + str(enum["line"]),
			))
	for qualified, domain in catalog["domains"].items():
		if _posix_name(domain["file"]) != types_file:
			findings.append(Finding(
				"rule_layout",
				qualified,
				"domain outside 10_types.sql  " + domain["file"] + ":" + str(domain["line"]),
			))
	return findings


#============================================
def rule_7_key_names(catalog: dict) -> list:
	"""
	Single-column FK names must end with the parent table name plus _id.

	This is the audit meter (72 of 180 on 9156ee17). PK and composite FK
	naming are the same style rule in DATABASE_STYLE.md but are not folded
	into this count.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		for fk in table["foreign_keys"]:
			if len(fk["columns"]) != 1:
				continue
			parent = _parent_table(fk["parent"])
			column = fk["columns"][0]
			if column.endswith(parent + "_id"):
				continue
			findings.append(make_finding(
				"rule_7_key_names",
				table,
				qualified + "." + column,
				f"ends with {column}; parent is {parent}",
			))
	return findings


#============================================
def rule_4_types(catalog: dict) -> list:
	"""
	Forbidden types and text columns constrained by a literal IN list.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		for column in table["columns"]:
			col_type = _column_type(column)
			extra = qualified + "." + column["name"]
			if "varchar(" in col_type:
				findings.append(make_finding("rule_4_types", table, extra, "varchar"))
			if re.search(r"(^| )char\(", col_type):
				findings.append(make_finding("rule_4_types", table, extra, "char"))
			if col_type in ("serial", "bigserial", "money"):
				findings.append(make_finding("rule_4_types", table, extra, col_type))
			if col_type == "json" or col_type.startswith("json "):
				findings.append(make_finding("rule_4_types", table, extra, "json not jsonb"))
			if (
				"timestamp" in col_type
				and "time zone" not in col_type
				and "timestamptz" not in col_type
			):
				findings.append(make_finding(
					"rule_4_types", table, extra, "timestamp without time zone"
				))
			if "timestamptz(" in col_type:
				findings.append(make_finding("rule_4_types", table, extra, "timestamptz(n)"))
			check = column["check"]
			if col_type == "text" and check and IN_LIST_RE.search(check):
				findings.append(make_finding(
					"rule_4_types", table, extra, "text CHECK IN (...)"
				))
	return findings


#============================================
def rule_2_constant_columns(catalog: dict) -> list:
	"""
	CHECK that admits exactly one literal.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		for column in table["columns"]:
			check = column["check"]
			if check is None:
				continue
			if not EQ_ONE_RE.match(check):
				continue
			findings.append(make_finding("rule_2_constant_columns", table, qualified + "." + column["name"], "CHECK admits exactly one literal",
			))
	return findings


#============================================
def rule_5_duplicate_literal_sets(catalog: dict) -> list:
	"""
	Identical IN (...) sets on two or more columns.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	by_set = collections.defaultdict(list)
	for qualified, table in catalog["tables"].items():
		for column in table["columns"]:
			check = column["check"]
			if check is None:
				continue
			values = _in_value_set(check)
			if values is None:
				continue
			by_set[values].append((qualified, table, column["name"]))
	findings = []
	for values, owners in by_set.items():
		if len(owners) < 2:
			continue
		for qualified, table, name in owners:
			findings.append(make_finding("rule_5_duplicate_literal_sets", table, qualified + "." + name, "duplicate IN set used on "
				+ str(len(owners))
				+ " columns",
			))
	return findings


#============================================
def _in_value_set(check: str) -> tuple | None:
	"""
	Return a frozenset of IN-list labels, or None.

	Args:
		check: CHECK expression.

	Returns:
		tuple | None: Sorted labels, or None when not an IN list.
	"""
	match = IN_VALUES_RE.search(check)
	if not match:
		return None
	inner = match.group(1)
	labels = []
	for piece in inner.split(","):
		token = piece.strip().strip("'")
		if token:
			labels.append(token)
	if not labels:
		return None
	frozen = tuple(sorted(labels))
	return frozen


#============================================
def rule_16_clock_present(catalog: dict) -> list:
	"""
	Every table needs a timestamptz or date column.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		has_clock = False
		for column in table["columns"]:
			if _is_clock_type(_column_type(column)):
				has_clock = True
				break
		if not has_clock:
			findings.append(make_finding("rule_16_clock_present", table, qualified, "no timestamptz or date column",
			))
	return findings


#============================================
def rule_14_unindexed_fk(catalog: dict) -> list:
	"""
	Advisory: FK with no index or PK/UNIQUE whose leading columns match.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	by_table = collections.defaultdict(list)
	for index in catalog["indexes"]:
		by_table[index["table"]].append(index["columns"])
	findings = []
	for qualified, table in catalog["tables"].items():
		indexes = by_table[qualified]
		for fk in table["foreign_keys"]:
			if _fk_is_covered(fk["columns"], indexes):
				continue
			cols = ",".join(fk["columns"])
			findings.append(make_finding(
				"rule_14_unindexed_fk",
				table,
				qualified + " (" + cols + ")",
				"FK has no covering index",
			))
	return findings


#============================================
def _fk_is_covered(fk_columns: list, indexes: list) -> bool:
	"""
	Return whether some index or unique leading columns match the FK.

	Args:
		fk_columns: Referencing columns.
		indexes: Index column lists on the same table.

	Returns:
		bool: True when covered.
	"""
	width = len(fk_columns)
	for columns in indexes:
		if len(columns) >= width and columns[:width] == fk_columns:
			return True
	return False


#============================================
def rule_17_role_tag(catalog: dict) -> list:
	"""
	Every table comment begins with an allowed role tag.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		role = table["role"]
		if role in ROLE_TAGS:
			continue
		findings.append(make_finding("rule_17_role_tag", table, qualified, "comment does not begin with a role tag",
		))
	return findings


#============================================
def rule_16_updated_clock(catalog: dict) -> list:
	"""
	current state and aggregate tables carry updated_at or updated_on.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		role = table["role"]
		if role not in ("current state", "aggregate"):
			continue
		names = {column["name"]: column for column in table["columns"]}
		updated = names.get("updated_at") or names.get("updated_on")
		if updated is None:
			findings.append(make_finding("rule_16_updated_clock", table, qualified, "missing updated_at/updated_on",
			))
	return findings


#============================================
def rule_16_clock_type(catalog: dict) -> list:
	"""
	Authored revision/aggregate/current-state tables use date; others timestamptz.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		role = table["role"]
		if role is None:
			continue
		comment = table["comment"] or ""
		authored = "authored" in comment.lower()
		want_date = authored and role in ("revision", "aggregate", "current state")
		for column in table["columns"]:
			if column["name"] not in ("created_at", "created_on", "published_on", "saved_on"):
				continue
			col_type = _column_type(column)
			is_date = col_type == "date" or col_type.startswith("date ")
			is_tz = _is_clock_type(col_type) and not is_date
			if want_date and not is_date:
				findings.append(make_finding("rule_16_clock_type", table, qualified + "." + column["name"], "authored role wants date",
				))
			if not want_date and not is_tz:
				findings.append(make_finding("rule_16_clock_type", table, qualified + "." + column["name"], "role wants timestamptz",
				))
	return findings


#============================================
def rule_11_student_work_keys(catalog: dict) -> list:
	"""
	student work tables carry course_instance_id and lead keys with it.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		if table["role"] != "student work":
			continue
		names = [column["name"] for column in table["columns"]]
		not_null = {
			column["name"]: column["not_null"] for column in table["columns"]
		}
		if "course_instance_id" not in names or not not_null["course_instance_id"]:
			findings.append(make_finding("rule_11_student_work_keys", table, qualified, "missing course_instance_id NOT NULL",
			))
			continue
		keys = [table["primary_key"]] + table["uniques"]
		for key in keys:
			if not key:
				continue
			if key[0] != "course_instance_id":
				findings.append(make_finding("rule_11_student_work_keys", table, qualified, "key does not lead with course_instance_id",
				))
	return findings


#============================================
def rule_8_identity(catalog: dict) -> list:
	"""
	Identity convention: public ID PK, uuid internals, no identity FK targets.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	identity_cols = set()
	for qualified, table in catalog["tables"].items():
		for column in table["columns"]:
			col_type = _column_type(column)
			if "generated" in col_type or "identity" in col_type:
				identity_cols.add((qualified, column["name"]))
		pk = table["primary_key"]
		if qualified.startswith("ple_migration."):
			continue
		if len(pk) == 1:
			pk_col = None
			for column in table["columns"]:
				if column["name"] == pk[0]:
					pk_col = column
					break
			if pk_col is None:
				continue
			col_type = _column_type(pk_col)
			# Public-ID tables use a text domain; internals use uuid.
			if not _is_uuid_or_public_id_type(col_type):
				findings.append(make_finding("rule_8_identity", table, qualified + "." + pk[0], "PK type is not uuid or text public ID",
				))
	for qualified, table in catalog["tables"].items():
		for fk in table["foreign_keys"]:
			parent = fk["parent"]
			for column in fk["parent_columns"]:
				if (parent, column) in identity_cols:
					findings.append(make_finding("rule_8_identity", table, qualified, "FK targets GENERATED IDENTITY column",
					))
	return findings


#============================================
def rule_12_immutability(catalog: dict) -> list:
	"""
	Placeholder until catalog grants/triggers are loaded; no source-only hits.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	return findings


#============================================
def rule_9_null_meaning(catalog: dict) -> list:
	"""
	Nullable columns need a CHECK naming them or a column comment.

	Args:
		catalog: Catalog model.

	Returns:
		list: Findings.
	"""
	findings = []
	for qualified, table in catalog["tables"].items():
		for column in table["columns"]:
			if column["not_null"]:
				continue
			check = column["check"] or ""
			comment = column["comment"]
			if column["name"] in check or comment:
				continue
			findings.append(make_finding("rule_9_null_meaning", table, qualified + "." + column["name"], "nullable column lacks CHECK or comment",
			))
	return findings



#============================================
def advisory_rule_ids(source_dir: str) -> set:
	"""
	Return advisory rule ids for this source tree.

	rule_14_unindexed_fk is always advisory. rule_layout is advisory only
	when 20_tables/ is absent. M2 Student Work keys stay advisory until M2.

	Args:
		source_dir: SQL source directory.

	Returns:
		set: Advisory rule id strings.
	"""
	ids = {
		"rule_14_unindexed_fk",
		"rule_11_student_work_keys",
	}
	if not _layout_blocks(source_dir):
		ids.add("rule_layout")
	return ids


#============================================
def policy_usage_text() -> str:
	"""
	Return the usage sentences shared with docs/USAGE.md and argparse.

	Returns:
		str: Command policy for stdout, flags, and exit codes.
	"""
	text = (
		"Reports mechanical rules from docs/DATABASE_STYLE.md against "
		"schemas/base_schema/ (override with -s/--source-dir). Default stdout "
		"is one count, tab, rule_##_title line per rule with findings "
		"(two-digit numbers), skip notes, and N findings in M rules only. "
		"One finding line per violation (rule_##_title, location, message, "
		"source file:line) is written to output/schema_style_findings.txt. "
		"-v/--verbose also prints those finding lines to stdout. "
		"-r/--report includes advisory findings (rule_14_unindexed_fk, and "
		"rule_layout only when 20_tables/ is absent) in verbose stdout and "
		"the findings file without changing the exit code. "
		"-j/--snapshot reads a catalog snapshot (default "
		"schemas/catalog_snapshot.json when that file exists); "
		"-d/--database reads a live database. Snapshot and database runs "
		"apply Tier 3 rules. Exit 1 on blocking findings, 0 if clean."
	)
	return text


#============================================
def collect_findings(catalog: dict, source_dir: str, tier3: bool) -> tuple:
	"""
	Run style rules and return findings, skip notes, and advisory rule ids.

	Args:
		catalog: Catalog model.
		source_dir: SQL source directory.
		tier3: True when snapshot or database input was used.

	Returns:
		tuple: (findings, notes, advisory_rules).
	"""
	notes = []
	findings = []
	findings.extend(rule_create_table_count(catalog, source_dir))
	findings.extend(rule_layout(catalog, source_dir))
	findings.extend(rule_7_key_names(catalog))
	findings.extend(rule_4_types(catalog))
	findings.extend(rule_2_constant_columns(catalog))
	findings.extend(rule_5_duplicate_literal_sets(catalog))
	findings.extend(rule_16_clock_present(catalog))
	findings.extend(rule_14_unindexed_fk(catalog))
	if _any_role(catalog):
		findings.extend(rule_17_role_tag(catalog))
		findings.extend(rule_16_updated_clock(catalog))
		findings.extend(rule_16_clock_type(catalog))
		findings.extend(rule_11_student_work_keys(catalog))
	else:
		notes.append("Tier 2 skipped (no role tags)")
	if tier3:
		findings.extend(rule_8_identity(catalog))
		findings.extend(rule_12_immutability(catalog))
		findings.extend(rule_9_null_meaning(catalog))
	else:
		notes.append("Tier 3 skipped (source-only)")
	advisory_rules = advisory_rule_ids(source_dir)
	return findings, notes, advisory_rules
