"""Opt-in developer storage budgets for Rust builds and Podman resources.

Run with ``source source_me.sh && python3 -m pytest
tests/test_developer_storage_budget.py``. The test is excluded from ordinary
pytest collection because Podman is local developer infrastructure.
"""

# Standard Library
import json
import os
import subprocess

# Local
import file_utils


MAX_TARGET_SIZE_KIB = 10 * 1024 * 1024
MAX_PODMAN_SIZE_BYTES = 20 * 1000 * 1000 * 1000


#============================================
def directory_size_kib(directory: str) -> int:
	"""Return physical directory usage in KiB, matching ``du -sh`` scope."""
	result = subprocess.run(
		["du", "-sk", directory],
		check=True,
		capture_output=True,
		text=True,
	)
	return int(result.stdout.split(maxsplit=1)[0])


#============================================
def podman_storage_size_bytes() -> int:
	"""Return total image, container, and volume bytes for the active connection."""
	result = subprocess.run(
		["podman", "system", "df", "--format", "json"],
		check=False,
		capture_output=True,
		text=True,
	)
	if result.returncode != 0:
		raise RuntimeError(f"podman system df failed: {result.stderr.strip()}")
	usage_rows = json.loads(result.stdout)
	total_bytes = sum(row["RawSize"] for row in usage_rows)
	return total_bytes


#============================================
def test_target_disk_usage_stays_under_10_gib() -> None:
	"""Keep Rust build artifacts within the target directory budget."""
	repo_root = file_utils.get_repo_root()
	target_directory = os.path.join(repo_root, "target")
	if not os.path.isdir(target_directory):
		return
	actual_kib = directory_size_kib(target_directory)
	assert actual_kib <= MAX_TARGET_SIZE_KIB, (
		"target/ disk usage is "
		f"{actual_kib / (1024 * 1024):.1f} GiB; the budget is 10.0 GiB. "
		"Run `cargo clean` or remove stale target/ build artifacts before continuing."
	)


#============================================
def test_podman_disk_usage_stays_under_20_gb() -> None:
	"""Keep Podman-managed images, containers, and volumes within budget."""
	actual_bytes = podman_storage_size_bytes()
	assert actual_bytes <= MAX_PODMAN_SIZE_BYTES, (
		"Podman disk usage is "
		f"{actual_bytes / (1000 * 1000 * 1000):.2f} GB; the budget is 20.00 GB. "
		"Inspect `podman system df -v` and deliberately remove stale Podman resources."
	)
