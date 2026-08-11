"""Permanent trust-boundary checks for optional local-stack overlays."""

# Standard Library
import pathlib

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())


#============================================
def test_smtp_overlay_connects_to_a_provider_without_running_a_mail_service() -> None:
	"""Optional email adds a protected credential handoff, not another server."""
	overlay = (REPO_ROOT / "containers" / "compose.smtp.yaml").read_text()
	api = overlay.split("  api:", 1)[1]

	assert "PLE_SMTP_PASSWORD_HOST_FILE:" not in api and "PLE_SMTP_PASSWORD_FILE:" in api
	assert "ports:" not in overlay and "mailpit" not in overlay.lower() and "mailhog" not in overlay.lower()
