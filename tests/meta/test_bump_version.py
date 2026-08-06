# Standard Library
import sys
import pathlib

# PIP3 modules
import pytest

# local repo modules
import devel.bump_version


#============================================
def test_set_version_synchronizes_rust_versions(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""An explicit CalVer synchronizes repo and Cargo representations."""
	cargo_toml = tmp_path / "Cargo.toml"
	cargo_lock = tmp_path / "Cargo.lock"
	version_file = tmp_path / "VERSION"
	cargo_toml.write_text(
		'[package]\nname = "demo"\nversion = "26.5.0"\nedition = "2024"\n',
		encoding="utf-8",
	)
	cargo_lock.write_text(
		'version = 4\n\n'
		'[[package]]\n'
		'name = "demo"\n'
		'version = "26.5.0"\n\n'
		'[[package]]\n'
		'name = "dependency"\n'
		'version = "1.2.3"\n',
		encoding="utf-8",
	)
	version_file.write_text("26.06\n", encoding="utf-8")
	monkeypatch.setattr(
		sys,
		"argv",
		[
			"bump_version.py",
			"-A",
			"--set-version",
			"26.07",
			"--base-dir",
			str(tmp_path),
		],
	)

	devel.bump_version.main()

	cargo_outputs = (
		cargo_toml.read_text(encoding="utf-8"),
		cargo_lock.read_text(encoding="utf-8"),
	)
	assert cargo_outputs == (
		'[package]\nname = "demo"\nversion = "26.7.0"\nedition = "2024"\n',
		'version = 4\n\n'
		'[[package]]\n'
		'name = "demo"\n'
		'version = "26.7.0"\n\n'
		'[[package]]\n'
		'name = "dependency"\n'
		'version = "1.2.3"\n',
	)
	assert version_file.read_text(encoding="utf-8") == "26.07\n"
