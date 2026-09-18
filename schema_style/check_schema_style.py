#!/usr/bin/env python3
"""Report mechanical DATABASE_STYLE.md findings against the SQL catalog."""

# Standard Library
import re
import sys
import signal
import argparse
import pathlib
import collections

# local repo modules
import schema_style.schema_catalog_lib as schema_catalog_lib
import schema_style.schema_style_rules as schema_style_rules


FINDINGS_PATH = "output/schema_style_findings.txt"
DEFAULT_SNAPSHOT_PATH = "schemas/catalog_snapshot.json"
RULE_ID_PATTERN = re.compile(r"^rule_(\d+)_(.*)$")


#============================================
def parse_args() -> argparse.Namespace:
	"""
	Parse command-line arguments.

	Returns:
		argparse.Namespace: Parsed flags.
	"""
	parser = argparse.ArgumentParser(
		description="Check SQL schema style against docs/DATABASE_STYLE.md",
		epilog=schema_style_rules.policy_usage_text(),
	)
	parser.add_argument(
		"-s", "--source-dir", dest="source_dir", default="schemas/base_schema",
		help="SQL source directory",
	)
	parser.add_argument(
		"-j", "--snapshot", dest="snapshot", default=None,
		help="Catalog snapshot JSON path (default: schemas/catalog_snapshot.json when that file exists)",
	)
	parser.add_argument(
		"-d", "--database", dest="database", default=None,
		help="Live PostgreSQL database name",
	)
	parser.add_argument(
		"-r", "--report", dest="report", action="store_true",
		help="Include advisory findings (rule_14_unindexed_fk) in verbose stdout",
	)
	parser.add_argument(
		"-v", "--verbose", dest="verbose", action="store_true",
		help="Print one finding line per violation on stdout",
	)
	parser.set_defaults(report=False, verbose=False)
	args = parser.parse_args()
	return args


#============================================
def load_catalog(args: argparse.Namespace) -> tuple:
	"""
	Load the catalog from source, snapshot, and/or a live database.

	Args:
		args: Parsed flags.

	Returns:
		tuple: (catalog, tier3).
	"""
	catalog = schema_catalog_lib.load_from_source(args.source_dir)
	tier3 = False
	snapshot = args.snapshot
	if snapshot is None:
		default_snapshot = pathlib.Path(DEFAULT_SNAPSHOT_PATH)
		if default_snapshot.is_file():
			snapshot = str(default_snapshot)
	if snapshot is not None:
		catalog = schema_catalog_lib.load_from_snapshot(snapshot)
		tier3 = True
	if args.database is not None:
		catalog = schema_catalog_lib.load_from_database(args.database)
		tier3 = True
	return catalog, tier3


#============================================
def format_rule_id(rule: str) -> str:
	"""
	Return a rule id with a two-digit number, for example rule_07_key_names.

	Args:
		rule: Internal rule id.

	Returns:
		str: Aligned rule id.
	"""
	match = RULE_ID_PATTERN.match(rule)
	if not match:
		return rule
	number = int(match.group(1))
	rest = match.group(2)
	formatted = f"rule_{number:02d}_{rest}"
	return formatted


#============================================
def rule_sort_key(rule: str) -> tuple:
	"""
	Sort numbered rules by checklist number, then name.

	Args:
		rule: Internal rule id.

	Returns:
		tuple: Sort key.
	"""
	match = RULE_ID_PATTERN.match(rule)
	if match:
		key = (0, int(match.group(1)), match.group(2))
		return key
	key = (1, 0, rule)
	return key


#============================================
def format_finding(item: schema_style_rules.Finding) -> str:
	"""
	Format one finding line.

	Args:
		item: Finding.

	Returns:
		str: Rule id, location, and message.
	"""
	line = f"{format_rule_id(item.rule)}  {item.location}  {item.message}"
	return line


#============================================
def format_report(
		listed: list,
		all_findings: list,
		notes: list,
		include_details: bool,
		) -> list:
	"""
	Build finding lines, per-rule summaries, skip notes, and a totals line.

	Args:
		listed: Findings to include in full.
		all_findings: All findings including advisory, for summaries.
		notes: Skip notes.
		include_details: When True, include per-finding lines.

	Returns:
		list: Report lines without trailing newlines.
	"""
	lines = []
	if include_details:
		for item in listed:
			lines.append(format_finding(item))
	counts = collections.Counter(item.rule for item in all_findings)
	for rule in sorted(counts, key=rule_sort_key):
		count_text = f"{counts[rule]:3d}"
		lines.append(count_text + "\t" + format_rule_id(rule))
	for note in notes:
		lines.append(note)
	if not all_findings:
		lines.append("clean")
		return lines
	total = len(all_findings)
	rule_count = len(counts)
	lines.append(f"{total} findings in {rule_count} rules")
	return lines


#============================================
def write_report(output_file: str, lines: list) -> None:
	"""
	Write report lines to output_file, creating parent directories.

	Args:
		output_file: Destination path.
		lines: Report lines.
	"""
	path = pathlib.Path(output_file)
	path.parent.mkdir(parents=True, exist_ok=True)
	text = "\n".join(lines) + "\n"
	path.write_text(text)


#============================================
def main() -> None:
	"""Run the schema style checker."""
	signal.signal(signal.SIGPIPE, signal.SIG_DFL)
	args = parse_args()
	catalog, tier3 = load_catalog(args)
	findings, notes, advisory_rules = schema_style_rules.collect_findings(
		catalog, args.source_dir, tier3
	)
	blocking = [item for item in findings if item.rule not in advisory_rules]
	listed = findings if args.report else blocking
	stdout_lines = format_report(listed, findings, notes, args.verbose)
	for line in stdout_lines:
		print(line)
	file_lines = format_report(findings, findings, notes, True)
	write_report(FINDINGS_PATH, file_lines)
	print("wrote " + FINDINGS_PATH, file=sys.stderr)
	if blocking:
		sys.exit(1)
	sys.exit(0)


if __name__ == '__main__':
	main()
