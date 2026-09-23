"""Decide screenshot-loop rebuild work from source timestamps against named outputs."""

from __future__ import annotations

import dataclasses
import pathlib


BUILD_NONE = "none"
BUILD_CLIENT = "client"
BUILD_WASM_CLIENT = "wasm_client"
BUILD_FULL = "full"
CONTAINERS_NONE = "none"
CONTAINERS_REPLACE_APPLICATION = "replace_application"
CONTAINERS_FULL_RESTART = "full_restart"

BUILD_RANK = {
	BUILD_NONE: 0,
	BUILD_CLIENT: 1,
	BUILD_WASM_CLIENT: 2,
	BUILD_FULL: 3,
}
CONTAINER_RANK = {
	CONTAINERS_NONE: 0,
	CONTAINERS_REPLACE_APPLICATION: 1,
	CONTAINERS_FULL_RESTART: 2,
}

SKIP_DIRECTORY_NAMES = frozenset(
	{
		"generated",
		"dist",
		"dist_wasm",
		"target",
		".git",
		"node_modules",
		"__pycache__",
	}
)
GENERATED_SCHEMA_FILE_NAMES = frozenset({"catalog_snapshot.json"})

CLIENT_OUTPUT = pathlib.Path("dist/index.html")
WASM_OUTPUT = pathlib.Path("dist/wasm/ple_bridge_bg.wasm")


@dataclasses.dataclass(frozen=True)
class Decision:
	"""One rebuild choice for host artifacts and running containers."""

	build: str
	containers: str


def newest_mtime(
	root: pathlib.Path,
	skip_names: frozenset[str] = SKIP_DIRECTORY_NAMES,
	skip_file_names: frozenset[str] = frozenset(),
) -> float | None:
	"""Return the newest file mtime under root, skipping named output directories."""
	if not root.exists():
		return None
	newest: float | None = None
	if root.is_file():
		return root.stat().st_mtime
	for current_root, directory_names, file_names in os_walk(root, skip_names):
		for name in file_names:
			if name in skip_file_names:
				continue
			mtime = (current_root / name).stat().st_mtime
			if newest is None or mtime > newest:
				newest = mtime
	return newest


def os_walk(
	root: pathlib.Path,
	skip_names: frozenset[str],
) -> list[tuple[pathlib.Path, list[str], list[str]]]:
	"""Walk files while pruning skipped directory names in place."""
	entries: list[tuple[pathlib.Path, list[str], list[str]]] = []
	stack = [root]
	while stack:
		current = stack.pop()
		directory_names: list[str] = []
		file_names: list[str] = []
		for child in current.iterdir():
			if child.is_dir():
				if child.name in skip_names:
					continue
				directory_names.append(child.name)
				stack.append(child)
			elif child.is_file():
				file_names.append(child.name)
		entries.append((current, directory_names, file_names))
	return entries


def source_is_newer(source_mtime: float | None, output_mtime: float | None) -> bool:
	"""Treat a missing named output as stale whenever sources exist."""
	if source_mtime is None:
		return False
	if output_mtime is None:
		return True
	return source_mtime > output_mtime


def _crate_source_mtime(repo_root: pathlib.Path) -> float | None:
	newest: float | None = None
	crates_root = repo_root / "crates"
	if crates_root.is_dir():
		for crate in crates_root.iterdir():
			if not crate.is_dir() or crate.name == "wasm":
				continue
			candidate = newest_mtime(crate)
			if candidate is not None and (newest is None or candidate > newest):
				newest = candidate
	for name in ("Cargo.toml", "Cargo.lock"):
		path = repo_root / name
		if path.is_file():
			mtime = path.stat().st_mtime
			if newest is None or mtime > newest:
				newest = mtime
	return newest


def _schema_source_mtime(repo_root: pathlib.Path) -> float | None:
	newest: float | None = None
	for tree in (repo_root / "schemas", repo_root / "containers"):
		skip_file_names = GENERATED_SCHEMA_FILE_NAMES if tree.name == "schemas" else frozenset()
		candidate = newest_mtime(tree, skip_file_names=skip_file_names)
		if candidate is not None and (newest is None or candidate > newest):
			newest = candidate
	compose_files = list(repo_root.glob("compose*.yaml"))
	compose_files.extend((repo_root / "containers").glob("compose*.yaml"))
	for path in compose_files:
		if not path.is_file():
			continue
		mtime = path.stat().st_mtime
		if newest is None or mtime > newest:
			newest = mtime
	return newest


def _output_mtime(repo_root: pathlib.Path, relative: pathlib.Path) -> float | None:
	path = repo_root / relative
	if not path.is_file():
		return None
	return path.stat().st_mtime


def _strongest_build(values: list[str]) -> str:
	winner = BUILD_NONE
	for value in values:
		if BUILD_RANK[value] > BUILD_RANK[winner]:
			winner = value
	return winner


def _strongest_containers(values: list[str]) -> str:
	winner = CONTAINERS_NONE
	for value in values:
		if CONTAINER_RANK[value] > CONTAINER_RANK[winner]:
			winner = value
	return winner


def decide(
	repo_root: pathlib.Path,
	api_image_created_at: float | None,
	launch_receipt_at: float | None,
	build_override: str | None = None,
) -> Decision:
	"""Return the strongest build and container work implied by stale sources.

	A full stack restart performs its own ``./build.sh``, so the driver build is
	``none`` whenever containers are ``full_restart``.
	"""
	builds: list[str] = []
	containers: list[str] = []

	client_source = newest_mtime(repo_root / "src")
	assets_source = newest_mtime(repo_root / "assets")
	client_newest = client_source
	if assets_source is not None and (client_newest is None or assets_source > client_newest):
		client_newest = assets_source
	if source_is_newer(client_newest, _output_mtime(repo_root, CLIENT_OUTPUT)):
		builds.append(BUILD_CLIENT)
		containers.append(CONTAINERS_NONE)

	wasm_source = newest_mtime(repo_root / "crates" / "wasm")
	if source_is_newer(wasm_source, _output_mtime(repo_root, WASM_OUTPUT)):
		builds.append(BUILD_WASM_CLIENT)
		containers.append(CONTAINERS_NONE)

	crate_source = _crate_source_mtime(repo_root)
	if source_is_newer(crate_source, api_image_created_at):
		builds.append(BUILD_FULL)
		containers.append(CONTAINERS_REPLACE_APPLICATION)

	schema_source = _schema_source_mtime(repo_root)
	if source_is_newer(schema_source, launch_receipt_at):
		builds.append(BUILD_NONE)
		containers.append(CONTAINERS_FULL_RESTART)

	build = _strongest_build(builds)
	container_choice = _strongest_containers(containers)
	if container_choice == CONTAINERS_FULL_RESTART:
		build = BUILD_NONE
	if build_override is not None:
		if build_override not in BUILD_RANK:
			raise ValueError(f"unsupported build override: {build_override}")
		build = build_override
		if container_choice == CONTAINERS_FULL_RESTART:
			build = BUILD_NONE
	return Decision(build=build, containers=container_choice)
