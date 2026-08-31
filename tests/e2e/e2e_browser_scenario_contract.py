"""Closed, catalog-owned policy for disposable production browser scenarios."""

import dataclasses
import re
from collections.abc import Iterable, Sequence

BASELINE_VERSION = "base-course-v1"
SCHEMA_VERSION = 2
NAMESPACE_PATTERN = re.compile(r"^bs1-[0-9a-f]{12}-[a-z][a-z0-9_]{0,31}$")
SCENARIO_PATTERN = re.compile(r"^[a-z][a-z0-9_]{0,31}$")
BASELINE_ALIASES = frozenset(
	{
		"base_course",
		"genetics_practice_course",
		"mary_completed_run",
		"jack_open_run",
		"published_peptide_assignment",
	}
)
PERSONAS = frozenset(
	{
		"elena_instructor",
		"mary_student",
		"jack_student",
		"avery_student",
		"morgan_sysadmin",
	}
)
RESOURCE_KINDS = frozenset(
	{
		"assignment",
		"blueprint",
		"collection",
		"course",
		"grade_scheme",
		"invitation",
		"passkey",
		"question",
		"qti_import",
		"response",
		"saved_search",
		"teaching_invitation",
	}
)
SEED_STATE_TRANSITIONS = frozenset({"avery_instructor_approval"})
SERVICE_RECEIPTS = frozenset({"renderer_delivery"})
FAULT_TRANSITIONS = frozenset({"gateway_submit_outage"})
REQUIRED_ROLE_SECURITY_SCENARIOS = {
	"direct_role_entry": (
		"tests/playwright/e2e/direct_role_entry.spec.ts",
		"morgan_sysadmin",
		"direct_sysadmin_passkey_reauthentication",
	),
	"auth_authorization": (
		"tests/playwright/e2e/auth_authorization.spec.ts",
		"elena_instructor",
		"instructor_passkey_reauthentication_and_seeded_sessions_avery_approval_and_course_boundaries",
	),
}


class ScenarioContractError(ValueError):
	"""A browser scenario request does not fit the checked-in policy."""


@dataclasses.dataclass(frozen=True)
class ScenarioContract:
	"""One UI-first journey; family modules provide facts and this module provides policy."""

	scenario_id: str
	spec_path: str
	personas: tuple[str, ...]
	baseline_reads: tuple[str, ...]
	ui_creates: tuple[str, ...]
	visible_observation: str
	seed_state_transitions: tuple[str, ...] = ()
	service_receipt: str | None = None
	fault_transition: str | None = None
	# Closed visual states exposed only when the suite runs its screenshot corpus.
	screenshot_states: tuple[str, ...] = ()


def scenario_contracts() -> tuple[ScenarioContract, ...]:
	"""Return the deterministic explicit catalog without an import cycle."""
	from e2e_browser_scenarios_catalog import contracts

	return contracts()


def require_contract(
	scenario_id: str,
	contracts: Sequence[ScenarioContract] | None = None,
) -> ScenarioContract:
	"""Find one checked-in scenario after validating its complete catalog."""
	registry = _registry(contracts)
	for contract in registry:
		if contract.scenario_id == scenario_id:
			return contract
	raise ScenarioContractError("browser suite scenario is unsupported")


def resolve_selection(
	scenario_id: str | None,
	spec_path: str | None,
	title_filter: str | None,
	contracts: Sequence[ScenarioContract] | None = None,
) -> tuple[ScenarioContract, ...]:
	"""Resolve default/all or one anchored contract without fallback semantics."""
	registry = _registry(contracts)
	if title_filter is not None and scenario_id is None and spec_path is None:
		raise ScenarioContractError(
			"browser suite grep requires a scenario or approved spec path"
		)
	if scenario_id is not None and spec_path is not None:
		raise ScenarioContractError("browser suite accepts one selector form")
	if scenario_id is None and spec_path is None:
		return registry
	if scenario_id is not None:
		return (require_contract(scenario_id, registry),)
	return _contract_for_spec_path(spec_path, registry)


def validate_contract(contract: ScenarioContract) -> None:
	"""Validate one complete scenario without inspecting runtime resources."""
	_validate_identifier(contract.scenario_id)
	_validate_spec_path(contract.spec_path)
	_validate_closed_values("persona", contract.personas, PERSONAS)
	_validate_closed_values("baseline alias", contract.baseline_reads, BASELINE_ALIASES)
	_validate_closed_values("resource kind", contract.ui_creates, RESOURCE_KINDS)
	_validate_visible_observation(contract.visible_observation)
	_validate_service_receipt(contract.service_receipt)
	_validate_fault_transition(contract.fault_transition)
	if contract.screenshot_states:
		_validate_screenshot_states(contract.screenshot_states)
	_validate_seed_state_transitions(contract.seed_state_transitions)


def validate_registry(
	contracts: Iterable[ScenarioContract] | None = None,
) -> None:
	"""Validate the executable scenario catalog and its role-security journeys."""
	registry = tuple(scenario_contracts() if contracts is None else contracts)
	if not registry:
		raise ScenarioContractError("browser scenario registry is empty")
	seen_ids: set[str] = set()
	seen_specs: set[str] = set()
	for contract in registry:
		_validate_registry_entry(contract, seen_ids, seen_specs)
	_validate_required_role_security_scenarios(registry)


def namespace_for(scenario_id: str, entropy: str) -> str:
	"""Create a public-safe scenario namespace from owner-generated entropy."""
	if re.fullmatch(r"[0-9a-f]{12}", entropy) is None:
		raise ScenarioContractError("browser scenario namespace entropy is invalid")
	result = f"bs1-{entropy}-{scenario_id}"
	if NAMESPACE_PATTERN.fullmatch(result) is None:
		raise ScenarioContractError("browser scenario namespace is invalid")
	return result


def _registry(
	contracts: Sequence[ScenarioContract] | None,
) -> tuple[ScenarioContract, ...]:
	registry = scenario_contracts() if contracts is None else tuple(contracts)
	validate_registry(registry)
	return registry


def _contract_for_spec_path(
	spec_path: str | None,
	registry: Sequence[ScenarioContract],
) -> tuple[ScenarioContract, ...]:
	for contract in registry:
		if contract.spec_path == spec_path:
			return (contract,)
	raise ScenarioContractError("browser suite focused file is unsupported")


def _validate_identifier(scenario_id: str) -> None:
	if SCENARIO_PATTERN.fullmatch(scenario_id) is None:
		raise ScenarioContractError("browser scenario id is invalid")


def _validate_spec_path(spec_path: str) -> None:
	if not spec_path.startswith("tests/playwright/e2e/"):
		raise ScenarioContractError("browser scenario spec path is invalid")
	if not spec_path.endswith(".spec.ts"):
		raise ScenarioContractError("browser scenario spec path is invalid")


def _validate_required_role_security_scenarios(
	registry: Sequence[ScenarioContract],
) -> None:
	"""Keep both named live-demo passkey journeys in every executable catalog."""
	by_id = {contract.scenario_id: contract for contract in registry}
	for scenario_id, requirement in REQUIRED_ROLE_SECURITY_SCENARIOS.items():
		contract = by_id.get(scenario_id)
		spec_path, persona, visible_observation = requirement
		if (
			contract is None
			or contract.spec_path != spec_path
			or persona not in contract.personas
			or "passkey" not in contract.ui_creates
			or contract.visible_observation != visible_observation
		):
			raise ScenarioContractError(
				"browser scenario registry requires both named role-security journeys"
			)


def _validate_visible_observation(value: str) -> None:
	if not value.strip() or not value.isascii():
		raise ScenarioContractError("browser scenario visible observation is invalid")


def _validate_service_receipt(value: str | None) -> None:
	if value is not None and value not in SERVICE_RECEIPTS:
		raise ScenarioContractError("browser scenario service receipt is invalid")


def _validate_fault_transition(value: str | None) -> None:
	"""Allow only a checked-in lifecycle fault transition."""
	if value is not None and value not in FAULT_TRANSITIONS:
		raise ScenarioContractError("browser scenario fault transition is invalid")


def _validate_seed_state_transitions(values: tuple[str, ...]) -> None:
	if len(values) != len(set(values)):
		raise ScenarioContractError(
			"browser scenario seed state transitions are invalid"
		)
	if not set(values).issubset(SEED_STATE_TRANSITIONS):
		raise ScenarioContractError(
			"browser scenario seed state transitions are invalid"
		)


def _validate_registry_entry(
	contract: ScenarioContract,
	seen_ids: set[str],
	seen_specs: set[str],
) -> None:
	if contract.scenario_id in seen_ids:
		raise ScenarioContractError("browser scenario ids must be unique")
	if contract.spec_path in seen_specs:
		raise ScenarioContractError("browser scenario spec paths must be unique")
	seen_ids.add(contract.scenario_id)
	seen_specs.add(contract.spec_path)
	validate_contract(contract)


def _validate_closed_values(
	name: str,
	values: tuple[str, ...],
	allowed: frozenset[str],
) -> None:
	if not values or len(values) != len(set(values)):
		raise ScenarioContractError(f"browser scenario {name} values are invalid")
	if not set(values).issubset(allowed):
		raise ScenarioContractError(f"browser scenario {name} values are invalid")


def _validate_screenshot_states(values: tuple[str, ...]) -> None:
	"""Keep provider states unique while the JSON corpus owns their closed inventory."""
	if not values or len(values) != len(set(values)):
		raise ScenarioContractError("browser scenario screenshot state values are invalid")
	for value in values:
		_validate_identifier(value)
