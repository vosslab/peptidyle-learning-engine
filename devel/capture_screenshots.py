#!/usr/bin/env python3
"""Warm screenshot driver: rebuild only what changed, then capture the full corpus."""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time

import change_scope


ROLE_FOLDERS = ("public", "instructor", "student", "sysadmin")
CONTROL_RECEIPT = pathlib.Path("local_stack_state/live_demo_browser/developer-control.json")
LIVE_DEMO_PROJECT = "ple-live-demo-browser"
# Podman `image inspect --format '{{.Created}}'` uses Go time.String():
# "2006-01-02 15:04:05.999999999 -0700 MST". JSON inspect may emit RFC3339.
_IMAGE_CREATED_STAMP = re.compile(
	r"^(?P<date>\d{4}-\d{2}-\d{2})[T ](?P<clock>\d{2}:\d{2}:\d{2})"
	r"(?:\.(?P<fraction>\d+))?"
	r"\s*(?:Z|(?P<offset>[+-]\d{2}:?\d{2})(?: [A-Za-z][A-Za-z0-9_/+-]*)?)$"
)


def repo_root() -> pathlib.Path:
	return pathlib.Path(__file__).resolve().parent.parent


def print_step(name: str, detail: str) -> float:
	"""Announce a step and return the monotonic start time."""
	print(f"[{name}] {detail}", flush=True)
	return time.monotonic()


def print_elapsed(name: str, started: float) -> None:
	elapsed = time.monotonic() - started
	print(f"[{name}] {elapsed:.1f}s", flush=True)


def run_command(argv: list[str], cwd: pathlib.Path, env: dict[str, str] | None = None) -> None:
	print("+ " + " ".join(argv), flush=True)
	result = subprocess.run(argv, cwd=cwd, env=env, check=False)
	if result.returncode != 0:
		raise SystemExit(result.returncode)


def parse_args(argv: list[str]) -> argparse.Namespace:
	parser = argparse.ArgumentParser(
		prog="./devel/capture_screenshots.sh",
		description="Publish or live-verify the manifest screenshot corpus.",
	)
	parser.add_argument(
		"--verify",
		action="store_true",
		help="validate tracked artifacts, replay live, and retain replay evidence",
	)
	parser.add_argument(
		"--headed",
		action="store_true",
		help="open Chromium; the local CLI authenticator still completes Sysadmin MFA",
	)
	parser.add_argument(
		"--fresh",
		action="store_true",
		help="stop, start, and afterwards stop an owned Live Demo instead of reusing one",
	)
	parser.add_argument(
		"--only",
		metavar="IDS",
		help="run selected scenario ids into staging without publishing",
	)
	parser.add_argument(
		"--build",
		choices=("none", "client", "wasm_client", "full"),
		help="override the timestamp build decision",
	)
	return parser.parse_args(argv)


def live_demo_running(root: pathlib.Path) -> bool:
	return (root / CONTROL_RECEIPT).is_file()


def launch_receipt_mtime(root: pathlib.Path) -> float | None:
	path = root / CONTROL_RECEIPT
	if not path.is_file():
		return None
	return path.stat().st_mtime


def parse_image_created_stamp(stamp: str) -> float | None:
	"""Return a POSIX timestamp for a podman/docker image Created stamp."""
	text = stamp.strip()
	matched = _IMAGE_CREATED_STAMP.match(text)
	if matched is None:
		return None
	fraction = matched.group("fraction")
	if fraction is None:
		fraction = "0"
	microseconds = (fraction + "000000")[:6]
	offset = matched.group("offset")
	if offset is None:
		offset = "+00:00"
	elif ":" not in offset:
		offset = f"{offset[:3]}:{offset[3:]}"
	iso_stamp = f"{matched.group('date')}T{matched.group('clock')}.{microseconds}{offset}"
	created = datetime.datetime.fromisoformat(iso_stamp)
	posix_time = created.timestamp()
	return posix_time


def api_image_created_at(root: pathlib.Path) -> float | None:
	"""Return the running api image creation time, or None when unreadable."""
	inspect = subprocess.run(
		[
			"podman",
			"ps",
			"--filter",
			f"label=com.docker.compose.project={LIVE_DEMO_PROJECT}",
			"--filter",
			"label=com.docker.compose.service=api",
			"--format",
			"{{.ImageID}}",
		],
		cwd=root,
		check=False,
		capture_output=True,
		text=True,
	)
	image_id = inspect.stdout.strip().splitlines()
	if inspect.returncode != 0 or len(image_id) != 1 or image_id[0] == "":
		inspect = subprocess.run(
			[
				"podman",
				"ps",
				"--filter",
				f"label=io.podman.compose.project={LIVE_DEMO_PROJECT}",
				"--filter",
				"label=io.podman.compose.service=api",
				"--format",
				"{{.ImageID}}",
			],
			cwd=root,
			check=False,
			capture_output=True,
			text=True,
		)
		image_id = inspect.stdout.strip().splitlines()
	if inspect.returncode != 0 or len(image_id) != 1 or image_id[0] == "":
		return None
	created = subprocess.run(
		["podman", "image", "inspect", image_id[0], "--format", "{{.Created}}"],
		cwd=root,
		check=False,
		capture_output=True,
		text=True,
	)
	if created.returncode != 0 or created.stdout.strip() == "":
		return None
	return parse_image_created_stamp(created.stdout)


def build_commands(decision: change_scope.Decision) -> list[list[str]]:
	"""Return the host commands for a build decision, in order."""
	if decision.build == "none":
		return []
	if decision.build == "client":
		return [["node", "pipeline/build.mjs", "--skip-wasm"]]
	if decision.build == "wasm_client":
		return [
			["pipeline/build_wasm.sh", "--debug"],
			["node", "pipeline/build.mjs", "--skip-wasm"],
		]
	if decision.build == "full":
		return [["./build.sh", "--debug"]]
	raise ValueError(f"unsupported build decision: {decision.build}")


def container_commands(decision: change_scope.Decision) -> list[list[str]]:
	"""Return the stack commands for a container decision, in order."""
	if decision.containers == "none":
		return []
	if decision.containers == "replace_application":
		return [[sys.executable, "local_stack.py", "rebuild-application"]]
	if decision.containers == "full_restart":
		return [
			["./launchers/run_live_demo.sh", "stop"],
			["./launchers/run_live_demo.sh", "--headless"],
		]
	raise ValueError(f"unsupported container decision: {decision.containers}")


def run_build(decision: change_scope.Decision, root: pathlib.Path) -> None:
	commands = build_commands(decision)
	if len(commands) == 0:
		print("[build] none")
		return
	for argv in commands:
		started = print_step("build", " ".join(argv))
		run_command(argv, root)
		print_elapsed("build", started)


def apply_containers(decision: change_scope.Decision, root: pathlib.Path) -> None:
	commands = container_commands(decision)
	if len(commands) == 0:
		print("[containers] none")
		return
	for argv in commands:
		started = print_step("containers", " ".join(argv))
		run_command(argv, root)
		print_elapsed("containers", started)


def png_hashes(root: pathlib.Path) -> dict[str, str]:
	screenshot_root = root / "docs" / "screenshots"
	hashes: dict[str, str] = {}
	for role in ROLE_FOLDERS:
		folder = screenshot_root / role
		if not folder.is_dir():
			continue
		for path in sorted(folder.iterdir()):
			if path.suffix != ".png" or not path.is_file():
				continue
			digest = hashlib.sha256(path.read_bytes()).hexdigest()
			hashes[f"{role}/{path.name}"] = digest
	return hashes


def write_review_copies(
	root: pathlib.Path,
	before: dict[str, str],
	after: dict[str, str],
) -> list[str]:
	changed = sorted(name for name, digest in after.items() if before.get(name) != digest)
	review = root / "test-results" / "screenshot-corpus" / "review"
	if review.exists():
		shutil.rmtree(review)
	review.mkdir(parents=True, exist_ok=True)
	screenshot_root = root / "docs" / "screenshots"
	for name in changed:
		source = screenshot_root / name
		target = review / name.replace("/", "__")
		if source.is_file():
			shutil.copy2(source, target)
	return changed


def start_live_demo(root: pathlib.Path) -> str:
	started = print_step("stack", "launchers/run_live_demo.sh --headless")
	result = subprocess.run(
		["./launchers/run_live_demo.sh", "--headless"],
		cwd=root,
		check=False,
		capture_output=True,
		text=True,
	)
	sys.stderr.write(result.stdout)
	sys.stderr.write(result.stderr)
	if result.returncode != 0:
		raise SystemExit(result.returncode)
	entry = ""
	for line in result.stdout.splitlines():
		if line.startswith("Live demo entry: "):
			entry = line[len("Live demo entry: ") :]
	if entry == "":
		raise SystemExit("The Live Demo did not report its entry URL.")
	print_elapsed("stack", started)
	return entry


def authenticator_setup(root: pathlib.Path) -> str:
	started = print_step("authenticator", "python3 local_stack.py authenticator")
	result = subprocess.run(
		[
			sys.executable,
			"local_stack.py",
			"authenticator",
			"--env-file",
			"local_stack_state/live_demo_browser/workspace/env.local",
		],
		cwd=root,
		check=False,
		capture_output=True,
		text=True,
	)
	if result.returncode != 0:
		sys.stderr.write(result.stdout)
		sys.stderr.write(result.stderr)
		raise SystemExit(result.returncode)
	setup = ""
	for line in result.stdout.splitlines():
		if line.startswith("Local authenticator setup URI: "):
			setup = line[len("Local authenticator setup URI: ") :]
	if setup == "":
		raise SystemExit("The local authenticator did not report its private setup path.")
	print_elapsed("authenticator", started)
	return setup


def capture_corpus(
	root: pathlib.Path,
	entry: str,
	args: argparse.Namespace,
) -> None:
	runner = root / "tests" / "playwright" / "capture_live_demo_screenshots.mjs"
	env = os.environ.copy()
	env["PLE_LOCAL_DEMO_TOTP_SETUP_FILE"] = authenticator_setup(root)
	env["NODE_EXTRA_CA_CERTS"] = str(
		root / "local_stack_state" / "live_demo_browser" / "workspace" / "gateway-root.crt"
	)
	env["DEBUG"] = ""
	env["PWDEBUG"] = ""
	argv = ["node", "--import", "tsx", str(runner)]
	if args.only:
		argv.extend(["--only", entry, args.only])
	elif args.verify:
		argv.extend(["--verify"])
		if args.headed:
			argv.append("--headed")
		argv.append(entry)
	else:
		argv.append("--publish")
		if args.headed:
			argv.append("--headed")
		argv.append(entry)
	started = print_step("capture", "Playwright screenshot corpus")
	run_command(argv, root, env)
	print_elapsed("capture", started)


def main() -> None:
	args = parse_args(sys.argv[1:])
	root = repo_root()
	os.chdir(root)
	if args.verify:
		started = print_step("verify-static", "tracked screenshot artifacts")
		run_command(
			[
				"node",
				"--import",
				"tsx",
				str(root / "tests" / "playwright" / "capture_live_demo_screenshots.mjs"),
				"--verify-static",
			],
			root,
			{**os.environ, "DEBUG": "", "PWDEBUG": ""},
		)
		print_elapsed("verify-static", started)
	started = print_step("playwright", "devel/setup_playwright.sh")
	run_command([str(root / "devel" / "setup_playwright.sh")], root)
	print_elapsed("playwright", started)

	owned_stop = False
	if args.fresh:
		started = print_step("fresh", "stop any owned Live Demo")
		run_command(["./launchers/run_live_demo.sh", "stop"], root)
		print_elapsed("fresh", started)
		owned_stop = True

	warm = live_demo_running(root)
	if not warm:
		start_live_demo(root)
	else:
		print("[stack] reusing the running Live Demo")
		decision = change_scope.decide(
			root,
			api_image_created_at(root),
			launch_receipt_mtime(root),
			args.build,
		)
		print(f"[decide] build={decision.build} containers={decision.containers}")
		run_build(decision, root)
		apply_containers(decision, root)
		if not live_demo_running(root):
			start_live_demo(root)

	control = root / CONTROL_RECEIPT
	origin = json.loads(control.read_text())["origin"]
	entry = origin + "sign-in" if origin.endswith("/") else origin + "/sign-in"

	before = png_hashes(root)
	capture_corpus(root, entry, args)
	after = png_hashes(root)
	if not args.verify and not args.only:
		changed = write_review_copies(root, before, after)
		if len(changed) == 0:
			print("no visual change")
		else:
			print("changed screenshots:")
			for name in changed:
				print(f"  {name}")

	if owned_stop:
		run_command(["./launchers/run_live_demo.sh", "stop"], root)
		print("Screenshot corpus complete; the owned Live Demo stack is clean.")
		return
	print("Screenshot corpus complete; the existing Live Demo stack is still running.")


if __name__ == "__main__":
	main()
