"""Permanent schema boundary checks for the email-free local roster path."""

# Standard Library
import pathlib

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
MIGRATION = REPO_ROOT / "schemas/migrations/2026080909_passwordless_identity.sql"


#============================================
def test_local_development_roster_rows_have_no_email_or_canonical_roster_key() -> None:
	"""Keep the pilot-only roster source structurally distinct from invitations."""
	migration = MIGRATION.read_text()
	assert "source IN ('invitation', 'local_development')" in migration
	assert "'legacy'" not in migration
	assert "source = 'local_development'" in migration
	for column in (
		"roster_email_normalized IS NULL",
		"roster_email_delivery IS NULL",
		"roster_id IS NULL",
	):
		assert column in migration
