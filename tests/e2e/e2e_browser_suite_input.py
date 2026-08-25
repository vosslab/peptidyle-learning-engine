"""Validation for the private owner-to-Playwright input ABI."""

import json
import pathlib

import local_stack_control.consumer

import e2e_browser_scenario_contract
import e2e_browser_screenshot_contract

# Match the browser-side closed ABI ceiling. The screenshot projection carries
# only owner-selected artifact/state identifiers, but the largest focused
# scenario legitimately exceeds the original pre-corpus 1 KiB envelope.
PRIVATE_INPUT_MAXIMUM_BYTES = 16_384


class BrowserSuiteInputError(ValueError):
	"""The owner-created private browser input is invalid."""


def validate(
	path: pathlib.Path,
	gateway_port: int,
	contract: e2e_browser_scenario_contract.ScenarioContract,
	screenshot_mode: bool = False,
) -> None:
	"""Confirm the private Playwright ABI before Chromium can start."""
	local_stack_control.consumer.require_private_regular_file(path, "browser-suite input")
	contents = path.read_text(encoding="ascii")
	if len(contents.encode("ascii")) > PRIVATE_INPUT_MAXIMUM_BYTES:
		raise BrowserSuiteInputError("browser-suite input is too large")
	try:
		value = json.loads(contents)
	except json.JSONDecodeError as error:
		raise BrowserSuiteInputError("browser-suite input is not valid JSON") from error
	if not isinstance(value, dict):
		raise BrowserSuiteInputError("browser-suite input has an invalid shape")
	expected_keys = {
		"schemaVersion", "scenarioId", "namespace", "baseUrl", "personas",
		"baselineReads", "visibleObservation",
	}
	if contract.service_receipt is not None:
		expected_keys.add("serviceReceipt")
	if contract.fault_transition is not None:
		expected_keys.add("faultTransition")
	if screenshot_mode:
		expected_keys.add("screenshotCapture")
	if set(value) != expected_keys:
		raise BrowserSuiteInputError("browser-suite input has an invalid shape")
	if (
		value["schemaVersion"] != e2e_browser_scenario_contract.SCHEMA_VERSION
		or value["scenarioId"] != contract.scenario_id
		or value["baseUrl"] != f"https://localhost:{gateway_port}/"
		or not isinstance(value["namespace"], str)
		or e2e_browser_scenario_contract.NAMESPACE_PATTERN.fullmatch(value["namespace"])
		is None
		or not value["namespace"].endswith("-" + contract.scenario_id)
	):
		raise BrowserSuiteInputError("browser-suite input has an invalid shape")
	if (
		not isinstance(value["personas"], list)
		or tuple(value["personas"]) != contract.personas
		or not isinstance(value["baselineReads"], list)
		or tuple(value["baselineReads"]) != contract.baseline_reads
		or value["visibleObservation"] != contract.visible_observation
		or value.get("serviceReceipt") != contract.service_receipt
		or value.get("faultTransition") != contract.fault_transition
	):
		raise BrowserSuiteInputError("browser-suite input has an invalid shape")
	if screenshot_mode:
		try:
			e2e_browser_screenshot_contract.validate_input(
				value.get("screenshotCapture"), contract.scenario_id
			)
		except e2e_browser_screenshot_contract.ScreenshotContractError as error:
			raise BrowserSuiteInputError(str(error)) from error
	canonical_value: dict[str, object] = {
		"schemaVersion": value["schemaVersion"], "scenarioId": value["scenarioId"],
		"namespace": value["namespace"], "baseUrl": value["baseUrl"],
		"personas": value["personas"], "baselineReads": value["baselineReads"],
		"visibleObservation": value["visibleObservation"],
	}
	for key in ("serviceReceipt", "faultTransition", "screenshotCapture"):
		if key in value:
			canonical_value[key] = value[key]
	if contents != json.dumps(canonical_value, separators=(",", ":"), ensure_ascii=True):
		raise BrowserSuiteInputError("browser-suite input must use canonical ASCII JSON")
