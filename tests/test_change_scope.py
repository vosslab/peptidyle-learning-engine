"""Which source change may rebuild the application image on a warm Live Demo."""

import importlib.util
import os
import pathlib
import sys
import types

import file_utils


def load_devel_module(name: str) -> types.ModuleType:
	"""Load a devel/*.py module without treating devel as a package."""
	devel = pathlib.Path(file_utils.get_repo_root()) / "devel"
	if str(devel) not in sys.path:
		sys.path.insert(0, str(devel))
	path = devel / f"{name}.py"
	spec = importlib.util.spec_from_file_location(name, path)
	if spec is None or spec.loader is None:
		raise RuntimeError(f"unable to load {path}")
	module = importlib.util.module_from_spec(spec)
	sys.modules[spec.name] = module
	spec.loader.exec_module(module)
	return module


change_scope = load_devel_module("change_scope")
capture_screenshots = load_devel_module("capture_screenshots")


def _write(path: pathlib.Path, text: str, mtime: float) -> None:
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text(text)
	os.utime(path, (mtime, mtime))


def _repo(tmp_path: pathlib.Path) -> pathlib.Path:
	_write(tmp_path / "src" / "app.tsx", "export {}\n", 100.0)
	_write(tmp_path / "assets" / "logo.svg", "<svg/>\n", 100.0)
	_write(tmp_path / "crates" / "wasm" / "src" / "lib.rs", "fn main() {}\n", 100.0)
	_write(tmp_path / "crates" / "server" / "src" / "lib.rs", "fn main() {}\n", 100.0)
	_write(tmp_path / "Cargo.toml", "[workspace]\n", 100.0)
	_write(tmp_path / "Cargo.lock", "# lock\n", 100.0)
	_write(tmp_path / "schemas" / "install.sql", "SELECT 1;\n", 100.0)
	_write(tmp_path / "containers" / "compose.yaml", "services: {}\n", 100.0)
	_write(tmp_path / "dist" / "index.html", "<html></html>\n", 200.0)
	_write(tmp_path / "dist" / "wasm" / "ple_bridge_bg.wasm", "wasm\n", 200.0)
	_write(tmp_path / "crates" / "server" / "target" / "debug" / "out", "obj\n", 900.0)
	_write(tmp_path / "generated" / "api" / "x.ts", "export {}\n", 900.0)
	return tmp_path


def test_warm_tree_with_fresh_outputs_rebuilds_nothing(tmp_path: pathlib.Path) -> None:
	"""A running stack whose bundles are newer than every source stays idle."""
	repo = _repo(tmp_path)
	decision = change_scope.decide(repo, 200.0, 200.0)
	assert decision.build == "none"
	assert decision.containers == "none"


def test_typescript_edit_does_not_replace_containers(tmp_path: pathlib.Path) -> None:
	"""TypeScript or CSS newer than dist/index.html rebuilds only the bind-mounted bundle."""
	repo = _repo(tmp_path)
	_write(repo / "src" / "style.css", "body {}\n", 300.0)
	decision = change_scope.decide(repo, 400.0, 400.0)
	assert decision.build == "client"
	assert decision.containers == "none"


def test_wasm_edit_does_not_replace_containers(tmp_path: pathlib.Path) -> None:
	"""crates/wasm newer than the copied bridge stays on the bind mount."""
	repo = _repo(tmp_path)
	_write(repo / "crates" / "wasm" / "src" / "lib.rs", "fn newer() {}\n", 300.0)
	decision = change_scope.decide(repo, 400.0, 400.0)
	assert decision.build == "wasm_client"
	assert decision.containers == "none"


def test_server_crate_edit_rebuilds_the_application_group(tmp_path: pathlib.Path) -> None:
	"""Other crate sources newer than the api image replace api/worker/publisher only."""
	repo = _repo(tmp_path)
	_write(repo / "crates" / "server" / "src" / "lib.rs", "fn newer() {}\n", 300.0)
	decision = change_scope.decide(repo, 200.0, 400.0)
	assert decision.build == "full"
	assert decision.containers == "replace_application"


def test_schema_edit_restarts_without_a_second_driver_build(tmp_path: pathlib.Path) -> None:
	"""A stack restart owns ./build.sh, even when crates are also stale."""
	repo = _repo(tmp_path)
	_write(repo / "crates" / "server" / "src" / "lib.rs", "fn newer() {}\n", 300.0)
	_write(repo / "containers" / "compose.yaml", "services: {api: {}}\n", 300.0)
	decision = change_scope.decide(repo, 200.0, 200.0)
	assert decision.build == "none"
	assert decision.containers == "full_restart"


def test_generated_schema_catalog_does_not_restart_the_running_stack(
	tmp_path: pathlib.Path,
) -> None:
	"""The fast-check catalog snapshot is not a runtime schema source."""
	repo = _repo(tmp_path)
	_write(repo / "schemas" / "catalog_snapshot.json", "{}\n", 300.0)
	assert change_scope._schema_source_mtime(repo) == 100.0
	decision = change_scope.decide(repo, 200.0, 200.0)
	assert decision.build == "none"
	assert decision.containers == "none"


def test_podman_go_created_stamp_does_not_rebuild_application(
	tmp_path: pathlib.Path,
) -> None:
	"""Podman's Go Created stamp is a real image time; a CSS edit still replaces no containers."""
	created = capture_screenshots.parse_image_created_stamp(
		"2026-09-20 07:03:03.033094562 +0000 UTC"
	)
	assert created is not None
	repo = _repo(tmp_path)
	_write(repo / "src" / "style.css", "body {}\n", 300.0)
	decision = change_scope.decide(repo, created, created)
	assert decision.build == "client"
	assert decision.containers == "none"


def test_png_hashes_include_viewport_subdirectories(tmp_path: pathlib.Path) -> None:
	"""Review copies include every published responsive rendition."""
	root = tmp_path / "repository"
	flat = root / "docs" / "screenshots" / "student" / "course_list.png"
	nested = root / "docs" / "screenshots" / "student" / "phone" / "course_list.png"
	flat.parent.mkdir(parents=True)
	nested.parent.mkdir(parents=True)
	flat.write_bytes(b"flat")
	nested.write_bytes(b"phone")

	hashes = capture_screenshots.png_hashes(root)

	assert set(hashes) == {"student/course_list.png", "student/phone/course_list.png"}
