#!/usr/bin/env python3
"""Build Rust QPM's browser package and refresh PLE's vendored copy."""

# Standard Library
import json
import shutil
import hashlib
import pathlib
import argparse
import subprocess


#============================================
def vendor_wasm(repo: pathlib.Path, source_repo: pathlib.Path) -> None:
	"""Copy the browser runtime and license and record consumed-file hashes.

	Args:
		repo: PLE repository root.
		source_repo: QPM source repository root.
	"""
	dist = source_repo / "packages" / "qti-wasm" / "dist"
	if not (dist / "generated" / "qti_wasm_bg.wasm").is_file():
		raise FileNotFoundError(f"Build the WASM package first: {dist}")
	destination = repo / "assets" / "qpm_wasm"
	if destination.exists():
		shutil.rmtree(destination)
	# Upstream demos have no PLE consumer; keep only the wrapper and generated runtime.
	for directory in ("src", "generated"):
		shutil.copytree(dist / directory, destination / directory)
	shutil.copy2(dist / "LICENSE.LGPL-3.0", destination / "LICENSE.LGPL-3.0")
	# Hash the consumed bytes; the checkout revision does not certify a local build.
	hashes = {}
	for artifact in sorted(destination.rglob("*")):
		if artifact.is_file():
			name = artifact.relative_to(destination).as_posix()
			hashes[name] = hashlib.sha256(artifact.read_bytes()).hexdigest()
	commit = subprocess.check_output(
		["git", "rev-parse", "HEAD"], cwd=source_repo, text=True,
	).strip()
	provenance = {
		"sourceCommit": commit,
		"sourcePath": "packages/qti-wasm/dist",
		"provenanceNote": (
			"Local built artifacts; sourceCommit records the checkout HEAD. "
			"Artifact hashes identify consumed bytes."
		),
		"sha256": hashes,
	}
	(destination / "source.json").write_text(json.dumps(provenance, indent=2) + "\n")
	print(f"Vendored QPM WASM from {dist}; source revision {commit}")


#============================================
def main() -> None:
	"""Build the QPM browser package before replacing PLE's current copy."""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument(
		"-q", "--qpm-root", dest="qpm_root",
		default="../qti-package-maker-rs",
		help="Rust QPM checkout (default: ../qti-package-maker-rs)",
	)
	args = parser.parse_args()
	repo = pathlib.Path(__file__).resolve().parents[1]
	source_repo = (repo / pathlib.Path(args.qpm_root).expanduser()).resolve()
	package = source_repo / "packages" / "qti-wasm"
	print(f"Building QPM WASM from {package}", flush=True)
	# ASVS 1.2.5: fixed argument lists, without shell interpolation.
	# check=True stops before vendoring if dependency installation or compilation fails.
	subprocess.run(["npm", "ci"], cwd=package, check=True)
	subprocess.run(["npm", "run", "build"], cwd=package, check=True)
	vendor_wasm(repo, source_repo)


if __name__ == "__main__":
	main()
