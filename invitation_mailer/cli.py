"""Command-line composition for the temporary invitation mailer."""

# Standard Library
import sys
import pathlib
import argparse
import subprocess

# local repo modules
import invitation_mailer.input
import invitation_mailer.sender
import invitation_mailer.status_log


#============================================
def _positive_limit(value: str) -> int:
	"""Parse a positive batch limit for argparse."""
	if not value.isdigit() or int(value) < 1:
		raise argparse.ArgumentTypeError("limit must be a positive integer")
	return int(value)


#============================================
def _repo_root() -> pathlib.Path:
	"""Return the repository root reported by Git."""
	result = subprocess.run(
		["git", "rev-parse", "--show-toplevel"],
		capture_output=True,
		text=True,
		check=False,
	)
	root_text = result.stdout.strip()
	if result.returncode != 0 or not root_text:
		raise invitation_mailer.input.InvitationInputError(
			"run the invitation mailer from inside its Git work tree"
		)
	return pathlib.Path(root_text)


#============================================
def parse_args(argv: list[str]) -> argparse.Namespace:
	"""Parse the mailer's intentionally small operator surface."""
	parser = argparse.ArgumentParser(
		description="Send attended student signup emails through macOS Mail.app.",
	)
	parser.add_argument(
		"export_path",
		help="Private JSON mailing-list export inside output-email/",
	)
	mode = parser.add_mutually_exclusive_group()
	mode.add_argument(
		"-s",
		"--send",
		dest="dry_run",
		action="store_false",
		help="Send messages through Mail.app",
	)
	mode.add_argument(
		"-n",
		"--dry-run",
		dest="dry_run",
		action="store_true",
		help="Record and print the plan without sending",
	)
	parser.add_argument(
		"-l",
		"--limit",
		dest="limit",
		type=_positive_limit,
		help="Process at most this many pending recipients",
	)
	parser.add_argument(
		"-o",
		"--only",
		dest="only_address",
		help="Process only this recipient email address",
	)
	parser.add_argument(
		"-f",
		"--force-resend",
		dest="force_resend",
		action="store_true",
		help="With --send and --only, resend a sent or indeterminate recipient",
	)
	parser.set_defaults(dry_run=True, force_resend=False)
	args = parser.parse_args(argv)
	if args.force_resend and (args.dry_run or args.only_address is None):
		parser.error("--force-resend requires --send and --only")
	return args


#============================================
def _only_recipient(
	mailing_export: invitation_mailer.input.MailingExport,
	only_address: str | None,
	config: invitation_mailer.input.MailerConfig,
) -> invitation_mailer.input.MailingExport:
	"""Return the complete export or its exact requested recipient."""
	if only_address is None:
		return mailing_export
	normalized = invitation_mailer.input.validate_recipient_address(
		only_address,
		config.allowed_recipient_domains,
	)
	recipients = tuple(
		recipient
		for recipient in mailing_export.recipients
		if recipient.email == normalized
	)
	if not recipients:
		raise invitation_mailer.input.InvitationInputError(
			f"--only recipient is not present in the export: {normalized}"
		)
	selected = invitation_mailer.input.MailingExport(
		mailing_export.course_name,
		recipients,
	)
	return selected


#============================================
def run(
	argv: list[str],
	send_func: invitation_mailer.sender.SendFunction | None = None,
	repo_root: pathlib.Path | None = None,
) -> int:
	"""Run one dry or attended batch and return its process status."""
	args = parse_args(argv)
	root = _repo_root() if repo_root is None else repo_root
	config_path = root / "invitation_mailer.yaml"
	template_path = root / "invitation_mailer" / "templates" / "invitation.txt"
	working_directory = root / "output-email"
	status_path = working_directory / "invitation_status.json"
	sent_log_path = working_directory / "sent_log.csv"
	config = invitation_mailer.input.load_config(config_path)
	export_path = invitation_mailer.input.resolve_private_export(root, args.export_path)
	mailing_export = invitation_mailer.input.read_export(export_path, config)
	mailing_export = _only_recipient(mailing_export, args.only_address, config)
	cells = invitation_mailer.status_log.load(status_path)
	selection = invitation_mailer.status_log.pending_recipients(
		mailing_export,
		cells,
		force_resend=args.force_resend,
	)
	recipients = selection.recipients
	if args.limit is not None:
		recipients = recipients[: args.limit]
	template = invitation_mailer.sender.load_template(template_path)
	dispatch = invitation_mailer.sender.default_send_func if send_func is None else send_func
	summary = invitation_mailer.sender.process_batch(
		mailing_export=mailing_export,
		recipients=recipients,
		template=template,
		cells=cells,
		status_path=status_path,
		dry_run=args.dry_run,
		throttle_seconds=config.throttle_seconds,
		send_func=dispatch,
	)
	invitation_mailer.status_log.write_sent_log(sent_log_path, cells)
	print(
		f"Summary: sent={summary.sent} failed={summary.failed} "
		f"dry_run={summary.dry_run} already_sent={selection.already_sent} "
		f"held_indeterminate={selection.held_indeterminate}"
	)
	return 1 if summary.failed else 0


#============================================
def main(argv: list[str] | None = None) -> None:
	"""Run the CLI with concise expected-input diagnostics."""
	values = sys.argv[1:] if argv is None else argv
	try:
		result = run(values)
	except (
		invitation_mailer.input.InvitationInputError,
		invitation_mailer.sender.InvitationSenderError,
		invitation_mailer.status_log.StatusLogError,
		OSError,
	) as error:
		print(f"ERROR: {error}", file=sys.stderr)
		raise SystemExit(2) from error
	raise SystemExit(result)
