"""Every license reset_repo.py offers must be installable on equal footing.

reset_repo.py treats licenses uniformly: copy_license is license-agnostic and
preflight_check requires a source file for whatever was chosen. These tests pin
that equality so no license is privileged and none is left without a backing
file. The original bug privileged MIT (only MIT passed the old gate); this guards
against any return of per-license special-casing.
"""

import pathlib

import pytest

import file_utils

import reset_repo

REPO_ROOT = pathlib.Path(file_utils.get_repo_root())

# Every selectable license that ships a body file. "none" is a docs sentinel
# meaning "no docs license", so it ships no file and is excluded here.
INSTALLABLE_LICENSES = reset_repo.CODE_LICENSES + [
	spdx for spdx in reset_repo.DOCS_LICENSES if spdx != "none"
]


#============================================
# Every offered license ships a source file
#============================================

@pytest.mark.parametrize("spdx", INSTALLABLE_LICENSES)
def test_offered_license_has_source_file(spdx: str) -> None:
	"""Each selectable license must have a LICENSES/LICENSE.<spdx>.md to install."""
	source = REPO_ROOT / "LICENSES" / f"LICENSE.{spdx}.md"
	assert source.is_file()


#============================================
# copy_license installs every license identically
#============================================

@pytest.mark.parametrize("spdx", INSTALLABLE_LICENSES)
def test_copy_license_installs_each_license(spdx: str, tmp_path: pathlib.Path) -> None:
	"""copy_license reproduces every license body byte-for-byte, no exceptions."""
	source = REPO_ROOT / "LICENSES" / f"LICENSE.{spdx}.md"
	target_filename = f"LICENSE.{spdx}.md"
	reset_repo.copy_license(str(tmp_path), str(source), target_filename, dry_run=False)
	installed = (tmp_path / target_filename).read_text(encoding="utf-8")
	assert installed == source.read_text(encoding="utf-8")
