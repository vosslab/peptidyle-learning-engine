#!/usr/bin/env python3
"""Serial owner for real-stack browser scenarios."""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import secrets
import stat
import subprocess
import sys

import local_stack_control.browser_suite_developer
import local_stack_control.browser_suite_lease

ROOT = pathlib.Path(__file__).resolve().parents[2]
E2E_DIRECTORY = ROOT / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract


CURRENT_MILESTONE_JOURNEYS = (
	("assignment_release", ("bash", "tests/e2e/e2e_live_demo_assignment_release.sh", "--browser")),
	("assignment_attempt", ("bash", "tests/e2e/e2e_live_demo_assignment_attempt.sh")),
	("instructor_accounts", ("bash", "tests/e2e/e2e_live_demo_instructor_accounts.sh", "--browser")),
	("support_capability", ("bash", "tests/e2e/e2e_live_demo_support_capability.sh", "--browser")),
	("invitation_export", ("bash", "tests/e2e/e2e_live_demo_invitation_export.sh", "--dry-run")),
	("course_seed", ("node", "--import", "tsx", "tests/playwright/e2e_live_demo_course_seed_browser.mjs")),
)


def write_private(path: pathlib.Path, contents: str) -> None:
	"""Write one canonical owner hand-off, never a shared test fixture."""
	fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
	try:
		os.write(fd, contents.encode("ascii"))
		os.fsync(fd)
	finally:
		os.close(fd)
	if stat.S_IMODE(path.stat().st_mode) != 0o600:
		raise RuntimeError("production-browser hand-off is not private")


def workspace() -> pathlib.Path:
	"""Return only the running fixed owner's private workspace."""
	path = ROOT / local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY / local_stack_control.browser_suite_lease.WORKSPACE_NAME
	if not path.is_dir() or stat.S_IMODE(path.stat().st_mode) != 0o700:
		raise RuntimeError("production-browser owner workspace is unavailable")
	return path


def environment_for(
	contract: e2e_browser_scenario_contract.ScenarioContract, origin: str, namespace: str | None = None
) -> dict[str, str]:
	"""Create the exact non-secret configuration expected by one Playwright spec."""
	private = workspace()
	namespace = (
		e2e_browser_scenario_contract.namespace_for(contract.scenario_id, secrets.token_hex(6))
		if namespace is None else namespace
	)
	input_path = private / ("browser-" + namespace + ".json")
	origin_path = private / ("origin-" + namespace + ".json")
	value: dict[str, object] = {
		"schemaVersion": 2,
		"scenarioId": contract.scenario_id,
		"namespace": namespace,
		"baseUrl": origin,
		"personas": list(contract.personas),
		"baselineReads": list(contract.baseline_reads),
		"visibleObservation": contract.visible_observation,
	}
	if contract.service_receipt is not None:
		value["serviceReceipt"] = contract.service_receipt
	write_private(input_path, json.dumps(value, separators=(",", ":"), ensure_ascii=True))
	result = dict(os.environ)
	result.update({
		"NODE_EXTRA_CA_CERTS": str(private / "gateway-root.crt"),
		"PLE_LIVE_DEMO_BROWSER_REQUIRED": "1",
		"PLE_LIVE_DEMO_BROWSER_INPUT_FILE": str(input_path),
		"PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE": str(origin_path),
	})
	return result


def run_contract(contract: e2e_browser_scenario_contract.ScenarioContract, origin: str) -> None:
	"""Run a registered visible journey one at a time on the fixed HTTPS stack."""
	result = subprocess.run(
		["npx", "playwright", "test", contract.spec_path, "--workers=1"],
		cwd=ROOT,
		env=environment_for(contract, origin),
		check=False,
	)
	if result.returncode != 0:
		raise RuntimeError("production-browser scenario failed: " + contract.scenario_id)


def run_current_milestone_journeys() -> None:
	"""Run the supported visible-browser journeys serially."""
	for name, argv in CURRENT_MILESTONE_JOURNEYS:
		print("==> production-browser journey: " + name, flush=True)
		environment = dict(os.environ)
		environment["NODE_EXTRA_CA_CERTS"] = str(workspace() / "gateway-root.crt")
		result = subprocess.run(argv, cwd=ROOT, env=environment, check=False)
		if result.returncode != 0:
			raise RuntimeError("production-browser journey failed: " + name)


def ready_owner_receipt() -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
	"""Wait only on the controller's authenticated HTTPS-ready receipt."""
	return local_stack_control.browser_suite_developer.read_developer_browser_suite_start_receipt(ROOT)


def main() -> None:
	parser = argparse.ArgumentParser(description="run serial production-browser scenarios")
	parser.add_argument("--running", action="store_true", help="use an already-receipted fixed owner")
	parser.add_argument("--scenario", help="run one registered scenario during owner diagnosis")
	args = parser.parse_args()
	if not args.running:
		started = subprocess.run([sys.executable, "local_stack.py", "start", "--headless"], cwd=ROOT, check=False)
		if started.returncode != 0:
			raise SystemExit(started.returncode)
	receipt = ready_owner_receipt()
	contracts = e2e_browser_scenario_contract.scenario_contracts()
	if args.scenario is not None:
		contracts = (e2e_browser_scenario_contract.require_contract(args.scenario, contracts),)
	for contract in contracts:
		print("==> production-browser scenario: " + contract.scenario_id, flush=True)
		run_contract(contract, receipt.origin)
	if args.scenario is None:
		run_current_milestone_journeys()
	print("PASS: serial production-browser scenarios passed.")


if __name__ == "__main__":
	main()
