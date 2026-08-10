# Standard Library
import argparse
import datetime
import sys

# local repo modules
import bump_version.contracts

def parse_args() -> argparse.Namespace:
	"""Parse command line arguments.

	Returns:
		argparse.Namespace: Parsed arguments.
	"""
	show_advanced = "--help-advanced" in sys.argv[1:]
	parser = argparse.ArgumentParser(
		description=(
			"Bump or set version numbers across common version files. "
			"Defaults to dry-run mode."
		),
	)
	parser.add_argument(
		"--help-advanced",
		action="help",
		help="Show advanced options and exit.",
	)

	parser.add_argument(
		"-b", "--base-dir",
		dest="base_dir",
		default=".",
		help=advanced_help(show_advanced, "Base directory to scan."),
	)
	parser.add_argument(
		"-s", "--source",
		dest="source",
		default="",
		help=advanced_help(show_advanced, "Source file to anchor version selection."),
	)
	parser.add_argument(
		"-m", "--max-depth",
		dest="max_depth",
		type=int,
		default=4,
		help=advanced_help(show_advanced, "Max directory depth to scan."),
	)

	parser.add_argument(
		"action",
		nargs="?",
		default="",
		help="Version to set, such as 26.05.",
	)
	parser.add_argument(
		"--bump",
		dest="bump",
		default="",
		choices=["major", "minor", "patch", "alpha", "beta", "rc"],
		help=advanced_help(show_advanced, "Bump by major, minor, patch, alpha, beta, or rc."),
	)
	parser.add_argument(
		"-v", "--set-version",
		dest="set_version",
		default="",
		help="Set an explicit version, such as 26.05.",
	)
	parser.add_argument(
		"-c", "--calver",
		dest="calver",
		action="store_true",
		help="Use the current YY.MM CalVer value.",
	)

	parser.add_argument(
		"-A", "--apply",
		dest="apply",
		action="store_true",
		help="Write changes to disk.",
	)
	parser.add_argument(
		"-n", "--dry-run",
		dest="apply",
		action="store_false",
		help=advanced_help(show_advanced, "Only print planned changes."),
	)
	parser.set_defaults(apply=False)

	parser.add_argument(
		"-u", "--update-all",
		dest="update_all",
		action="store_true",
		help=advanced_help(show_advanced, "Update all discovered versions, even if they differ."),
	)
	parser.add_argument(
		"--pre-style",
		dest="pre_style",
		choices=["pep440", "dash"],
		default="pep440",
		help=advanced_help(show_advanced, "Prerelease style when adding alpha/beta/rc."),
	)
	parser.add_argument(
		"--no-enforce-yy-mm",
		dest="enforce_yy_mm",
		action="store_false",
		help=advanced_help(show_advanced, "Disable YY.MM.PATCH enforcement."),
	)
	parser.set_defaults(enforce_yy_mm=True)

	args = parser.parse_args()
	if args.calver and args.set_version:
		parser.error("Use either --calver or --set-version, not both.")
	if args.action:
		if args.action in bump_version.contracts.SHORT_BUMP_ALIASES:
			if args.bump:
				parser.error("Use either positional bump shortcut or --bump, not both.")
			if args.set_version or args.calver:
				parser.error(
					"Use either positional bump shortcut or version source, not both."
				)
			args.bump = bump_version.contracts.SHORT_BUMP_ALIASES[args.action]
		else:
			if args.set_version or args.calver:
				parser.error("Use either positional version or version flag, not both.")
			args.set_version = args.action
	if args.calver:
		args.set_version = current_calver_month()
	if not args.bump and not args.set_version:
		args.set_version = current_calver_month()
	return args

#============================================

def advanced_help(show_advanced: bool, help_text: str) -> str:
	"""Return help text only when advanced help was requested.

	Args:
		show_advanced (bool): Whether advanced help is visible.
		help_text (str): Help text for the argument.

	Returns:
		str: Help text or argparse suppression marker.
	"""
	if show_advanced:
		return help_text
	return bump_version.contracts.ADVANCED_HELP

#============================================

def current_calver_month() -> str:
	"""Return the current month in repo CalVer format.

	Kept local rather than imported from devel/changelog_lib.py: that module
	pulls in rich, and this tool stays stdlib-only.

	Returns:
		str: Current YY.MM value.
	"""
	today = datetime.date.today()
	return f"{today.year % 100:02d}.{today.month:02d}"

#============================================
