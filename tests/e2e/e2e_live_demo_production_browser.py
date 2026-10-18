#!/usr/bin/env python3
"""Serial owner for real-stack browser scenarios."""

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
import local_stack_control.local_totp_authenticator

ROOT = pathlib.Path(__file__).resolve().parents[2]
E2E_DIRECTORY = ROOT / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract

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
	path = (
		ROOT
		/ local_stack_control.browser_suite_lease.LIVE_DEMO_BROWSER_STATE_DIRECTORY
		/ local_stack_control.browser_suite_lease.WORKSPACE_NAME
	)
	if not path.is_dir() or stat.S_IMODE(path.stat().st_mode) != 0o700:
		raise RuntimeError("production-browser owner workspace is unavailable")
	return path


def environment_for(
	contract: e2e_browser_scenario_contract.ScenarioContract,
	origin: str,
	namespace: str | None = None,
	temporary_spec_path: str | None = None,
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
	if temporary_spec_path is not None:
		value["temporarySpecPath"] = temporary_spec_path
	write_private(input_path, json.dumps(value, separators=(",", ":"), ensure_ascii=True))
	result = dict(os.environ)
	result.update({
		"NODE_EXTRA_CA_CERTS": str(private / "gateway-root.crt"),
		"PLE_LIVE_DEMO_BROWSER_REQUIRED": "1",
		"PLE_LIVE_DEMO_BROWSER_INPUT_FILE": str(input_path),
		"PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE": str(origin_path),
	})
	if "morgan_sysadmin" in contract.personas:
		setup_uri = local_stack_control.local_totp_authenticator.write_authenticator_setup_uri(
			private / local_stack_control.local_totp_authenticator.MORGAN_TOTP_ARTIFACT_FILE
		)
		result["PLE_LOCAL_DEMO_TOTP_SETUP_FILE"] = str(setup_uri)
	return result


def run_contract(
	contract: e2e_browser_scenario_contract.ScenarioContract,
	origin: str,
	temporary_spec_path: str | None = None,
) -> None:
	"""Run a registered visible journey one at a time on the fixed HTTPS stack."""
	command = ["npx", "playwright", "test"]
	if temporary_spec_path is None:
		command.append(contract.spec_path)
	command.append("--workers=1")
	result = subprocess.run(
		command,
		cwd=ROOT,
		env=environment_for(contract, origin, temporary_spec_path=temporary_spec_path),
		check=False,
	)
	if result.returncode != 0:
		raise RuntimeError("production-browser scenario failed: " + contract.scenario_id)


def ready_owner_receipt() -> local_stack_control.browser_suite_developer.DeveloperStartReceipt:
	"""Wait only on the controller's authenticated HTTPS-ready receipt."""
	return local_stack_control.browser_suite_developer.read_developer_browser_suite_start_receipt(ROOT)


def temporary_spec_path(raw_path: str) -> str:
	"""Validate and return one repo-relative spec below the contained temp root."""
	# ASVS 2.2.1 and 5.3.2: allow only a canonical repo-relative test path.
	requested = pathlib.PurePosixPath(raw_path)
	if (
		requested.is_absolute()
		or "\\" in raw_path
		or requested.as_posix() != raw_path
		or requested.parts[:2] != ("tests", "_temp")
		or ".." in requested.parts
		or requested.suffixes[-2:] != [".spec", ".ts"]
	):
		raise ValueError("--spec must name one tests/_temp/*.spec.ts file")

	repository_root = ROOT.resolve()
	temporary_root = (ROOT / "tests" / "_temp").resolve(strict=True)
	try:
		temporary_root.relative_to(repository_root)
		resolved = (ROOT / pathlib.Path(*requested.parts)).resolve(strict=True)
		resolved.relative_to(temporary_root)
	except (OSError, ValueError) as error:
		raise ValueError("--spec must resolve inside the repository tests/_temp directory") from error
	if not resolved.is_file():
		raise ValueError("--spec must name an existing regular file")
	return requested.as_posix()


def temporary_contract(spec_path: str) -> e2e_browser_scenario_contract.ScenarioContract:
	"""Use the existing seeded account and visible browser helper contract."""
	return e2e_browser_scenario_contract.ScenarioContract(
		scenario_id="temporary_browser",
		spec_path=spec_path,
		personas=("elena_instructor", "priya_instructor", "morgan_sysadmin"),
		baseline_reads=("seeded_accounts",),
		ui_creates=(),
		visible_observation="temporary browser proof",
	)


def main() -> None:
	parser = argparse.ArgumentParser(description="run serial production-browser scenarios")
	parser.add_argument("--running", action="store_true", help="use an already-receipted fixed owner")
	parser.add_argument("--scenario", help="run one registered scenario during owner diagnosis")
	parser.add_argument("--spec", help="run one temporary tests/_temp/*.spec.ts file")
	args = parser.parse_args()
	if args.scenario is not None and args.spec is not None:
		parser.error("--scenario and --spec are mutually exclusive")
	if args.spec is not None and not args.running:
		parser.error("--spec requires --running")

	try:
		selected_spec = temporary_spec_path(args.spec) if args.spec is not None else None
		contracts = e2e_browser_scenario_contract.scenario_contracts()
		if args.scenario is not None:
			contracts = (e2e_browser_scenario_contract.require_contract(args.scenario, contracts),)
	except (OSError, ValueError) as error:
		parser.error(str(error))

	if selected_spec is not None:
		contracts = (temporary_contract(selected_spec),)
	if not args.running:
		started = subprocess.run(
			[sys.executable, "local_stack.py", "start", "--headless"],
			cwd=ROOT,
			check=False,
		)
		if started.returncode != 0:
			raise SystemExit(started.returncode)
	receipt = ready_owner_receipt()
	for contract in contracts:
		print("==> production-browser scenario: " + contract.scenario_id, flush=True)
		run_contract(
			contract,
			receipt.origin,
			temporary_spec_path=selected_spec,
		)
	print("PASS: serial production-browser scenarios passed.")


if __name__ == "__main__":
	main()
