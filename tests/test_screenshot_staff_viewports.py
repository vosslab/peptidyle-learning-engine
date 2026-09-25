"""Keep published Instructor and Sysadmin screenshots in the laptop view."""

import json
import pathlib

import file_utils


def test_staff_screenshot_folders_have_no_alternate_viewports() -> None:
	"""Reject alternate staff viewports in names, folders, and the manifest."""
	screenshot_root = pathlib.Path(file_utils.get_repo_root()) / "docs" / "screenshots"
	forbidden = ("phone", "square", "tablet")
	violations = []
	for role in ("instructor", "sysadmin"):
		role_folder = screenshot_root / role
		assert role_folder.is_dir(), f"Missing screenshot folder: {role_folder}"
		for entry in role_folder.rglob("*"):
			name = entry.name.casefold()
			if entry.is_file():
				# Chi-square names Question content, not the square viewport.
				name = name.replace("chi_square", "")
			if any(viewport in name for viewport in forbidden):
				violations.append(entry.relative_to(screenshot_root).as_posix())
	manifest_path = screenshot_root / "current_capture_manifest.json"
	manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
	for capture in manifest["captures"]:
		if capture["role"] in ("instructor", "sysadmin") and capture["viewport"] != "laptop":
			violations.append(f"{capture['path']} (viewport: {capture['viewport']})")
	assert not violations, (
		"Instructor and Sysadmin screenshots use the laptop view only. "
		"Use laptop captures, remove stray viewport folders, then rebuild the corpus: "
		+ ", ".join(sorted(violations))
	)
