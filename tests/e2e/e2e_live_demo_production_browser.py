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
import time

import local_stack_control.browser_suite_developer
import local_stack_control.browser_suite_lease

ROOT = pathlib.Path(__file__).resolve().parents[2]
E2E_DIRECTORY = ROOT / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract
import e2e_browser_fault_orchestrator


CURRENT_MILESTONE_JOURNEYS = (
	("assignment_release", ("bash", "tests/e2e/e2e_live_demo_assignment_release.sh", "--browser")),
	("webwork_render", ("bash", "tests/e2e/e2e_live_demo_webwork.sh", "--render")),
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
	if contract.fault_transition is not None:
		value["faultTransition"] = contract.fault_transition
	write_private(input_path, json.dumps(value, separators=(",", ":"), ensure_ascii=True))
	result = dict(os.environ)
	result.update({
		"PLE_LIVE_DEMO_BROWSER_REQUIRED": "1",
		"PLE_LIVE_DEMO_BROWSER_INPUT_FILE": str(input_path),
		"PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE": str(origin_path),
	})
	return result


def run_native_ple_lease_observation() -> None:
	"""Observe the M13 leased native-PLE job without reading response or source data."""
	project = "ple-live-demo-browser"
	for _attempt in range(300):
		listed = subprocess.run(
			[
				"podman", "ps", "-q", "--filter", f"label=com.docker.compose.project={project}",
				"--filter", "label=com.docker.compose.service=postgres",
			],
			cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False,
		)
		identifiers = tuple(line for line in listed.stdout.splitlines() if line)
		if listed.returncode == 0 and len(identifiers) == 1:
			observed = subprocess.run(
				[
					"podman", "exec", identifiers[0], "sh", "-lc",
					"psql -X -v ON_ERROR_STOP=1 -U \"$POSTGRES_USER\" -d \"$POSTGRES_DB\" -At -c \"SELECT CASE WHEN EXISTS (SELECT 1 FROM ple_private.job WHERE job_kind='grade_accepted_submission' AND job_target_kind='question_submission' AND state='leased') THEN 'leased' ELSE 'waiting' END\"",
				],
				cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False,
			)
			if observed.returncode == 0 and observed.stdout.strip() == "leased":
				return
		time.sleep(0.1)
	raise RuntimeError("production-browser recovery did not observe the M13 native PLE lease")


def run_native_ple_recovery_contract(
	contract: e2e_browser_scenario_contract.ScenarioContract, origin: str
) -> None:
	"""Use the existing browser-profile native worker actions after one visible submission."""
	manifest = workspace() / "disposable.manifest"
	if not manifest.is_file():
		raise RuntimeError("production-browser recovery requires the controller manifest")
	arguments = ["npx", "playwright", "test", contract.spec_path, "--workers=1"]
	def run_action(action: list[str]) -> local_stack_control.process.SessionCommandResult:
		result = subprocess.run(
			[sys.executable, "-m", "local_stack_control.disposable_stack_command", *action, "--manifest", str(manifest)],
			cwd=ROOT, check=False,
		)
		return local_stack_control.process.SessionCommandResult(
			local_stack_control.process.ProcessSession(os.getpid(), time.time_ns(), "owner", "native-ple-recovery"),
			result.returncode,
		)
	namespace = e2e_browser_scenario_contract.namespace_for(contract.scenario_id, secrets.token_hex(6))
	request = e2e_browser_fault_orchestrator.FaultScenarioRequest(
		ROOT, workspace(), manifest, contract.scenario_id,
		namespace, arguments, environment_for(contract, origin, namespace),
	)
	e2e_browser_fault_orchestrator.run_native_ple_submission_recovery(
		request, run_action, run_native_ple_lease_observation,
	)


def run_contract(contract: e2e_browser_scenario_contract.ScenarioContract, origin: str) -> None:
	"""Run a registered visible journey one at a time on the fixed HTTPS stack."""
	if contract.fault_transition == "native_ple_submission_recovery":
		run_native_ple_recovery_contract(contract, origin)
		return
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
		result = subprocess.run(argv, cwd=ROOT, check=False)
		if result.returncode != 0:
				raise RuntimeError("production-browser journey failed: " + name)


def ready_owner_receipt() -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
	"""Wait only on the controller's authenticated HTTPS-ready receipt."""
	return local_stack_control.browser_suite_developer.read_developer_browser_suite_start_receipt(ROOT)


def main() -> None:
	parser = argparse.ArgumentParser(description="run serial production-browser scenarios")
	parser.add_argument("--recovery", action="store_true")
	parser.add_argument("--running", action="store_true", help="use an already-receipted fixed owner")
	parser.add_argument("--scenario", help="run one registered scenario during owner diagnosis")
	args = parser.parse_args()
	if not args.running:
		started = subprocess.run([sys.executable, "local_stack.py", "start", "--headless"], cwd=ROOT, check=False)
		if started.returncode != 0:
			raise SystemExit(started.returncode)
	receipt = ready_owner_receipt()
	contracts = e2e_browser_scenario_contract.scenario_contracts()
	if args.recovery:
		contracts = tuple(item for item in contracts if item.scenario_id == "learner_native_ple_recovery")
	if args.scenario is not None:
		contracts = (e2e_browser_scenario_contract.require_contract(args.scenario, contracts),)
	for contract in contracts:
		print("==> production-browser scenario: " + contract.scenario_id, flush=True)
		run_contract(contract, receipt.origin)
	if not args.recovery and args.scenario is None:
		run_current_milestone_journeys()
	print("PASS: serial production-browser scenarios passed.")


if __name__ == "__main__":
	main()
