"""Disposable whole-workflow oracle for the attended invitation mailer."""

# Standard Library
import os
import sys
import json
import pathlib
import tempfile
import dataclasses
import subprocess

SCRIPT_REPOSITORY_ROOT = pathlib.Path(
	subprocess.check_output(
		["git", "rev-parse", "--show-toplevel"],
		text=True,
	).strip()
)
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

# local repo modules
import invitation_mailer.cli
import invitation_mailer.status_log


#============================================
@dataclasses.dataclass
class RecordingSender:
	"""Record accepted dispatches without contacting Mail.app."""

	addresses: list[str] = dataclasses.field(default_factory=list)

	def __call__(
		self,
		name: str,
		address: str,
		subject: str,
		body: str,
	) -> str | None:
		"""Record one accepted message."""
		if "https://" not in body or not subject or not name:
			return "rendered message is incomplete"
		self.addresses.append(address)
		return None


#============================================
def _write_export(root: pathlib.Path, addresses: tuple[str, ...]) -> None:
	"""Replace the private roster export with the requested addresses."""
	payload = {
		"course_name": "Genetics 301",
		"students": [
			{
				"email": address,
				"signup_url": f"https://example.edu/signup/{index}",
			}
			for index, address in enumerate(addresses, start=1)
		],
	}
	path = root / "output-email" / "roster.json"
	path.write_text(json.dumps(payload), encoding="utf-8")
	os.chmod(path, 0o600)


#============================================
def _prepare_root(root: pathlib.Path, addresses: tuple[str, ...]) -> None:
	"""Create the fixed config, template, and private work directory."""
	(root / "invitation_mailer" / "templates").mkdir(parents=True)
	(root / "output-email").mkdir(mode=0o700)
	(root / "invitation_mailer.yaml").write_text(
		"allowed_recipient_domains:\n  - mail.roosevelt.edu\nthrottle_seconds: 0\n",
		encoding="ascii",
	)
	(root / "invitation_mailer" / "templates" / "invitation.txt").write_text(
		"Hello {recipient_name},\n\n{course_name}\n\n{signup_url}\n",
		encoding="ascii",
	)
	_write_export(root, addresses)


#============================================
def main() -> None:
	"""Prove the launcher plus rerun and targeted-resend workflows."""
	with tempfile.TemporaryDirectory(prefix="ple-invitation-mailer-") as temp_name:
		root = pathlib.Path(temp_name)
		addresses = (
			"first@mail.roosevelt.edu",
			"second@mail.roosevelt.edu",
			"third@mail.roosevelt.edu",
		)
		_prepare_root(root, addresses)
		subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
		launcher = subprocess.run(
			[
				sys.executable,
				str(SCRIPT_REPOSITORY_ROOT / "launchers" / "send_invitations.py"),
				"output-email/roster.json",
				"--limit",
				"1",
			],
			cwd=root,
			capture_output=True,
			text=True,
			check=False,
		)
		assert launcher.returncode == 0, launcher.stderr
		assert "dry_run=1" in launcher.stdout
		sender = RecordingSender()
		arguments = ["output-email/roster.json", "--send"]

		assert invitation_mailer.cli.run(arguments, sender, root) == 0
		assert sender.addresses == list(addresses)
		assert invitation_mailer.cli.run(arguments, sender, root) == 0
		assert sender.addresses == list(addresses)

		new_address = "fourth@mail.roosevelt.edu"
		_write_export(root, addresses + (new_address,))
		assert invitation_mailer.cli.run(arguments, sender, root) == 0
		assert sender.addresses[-1] == new_address
		assert sender.addresses.count(new_address) == 1

		assert invitation_mailer.cli.run(
			arguments + ["--only", addresses[0], "--force-resend"],
			sender,
			root,
		) == 0
		assert sender.addresses[-1] == addresses[0]
		cells = invitation_mailer.status_log.load(
			root / "output-email" / "invitation_status.json"
		)
		assert cells[("Genetics 301", addresses[0])].deliberate_resend is True
	print("invitation mailer E2E passed")


if __name__ == "__main__":
	main()
