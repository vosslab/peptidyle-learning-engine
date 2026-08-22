"""Closed, catalog-owned policy for disposable production browser scenarios."""

import dataclasses
import json
import pathlib
import re
from collections.abc import Iterable, Sequence

import local_stack_control.base_course_lifecycle
import local_stack_control.models

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
	{"elena_instructor", "mary_student", "avery_student", "morgan_sysadmin"}
)
RESOURCE_KINDS = frozenset(
	{
		"assignment",
		"course",
		"course_group",
		"grade_scheme",
		"invitation",
		"passkey",
		"question",
		"qti_import",
		"response",
		"teaching_invitation",
	}
)
SYSADMIN_REQUIREMENTS = frozenset({"not_required", "unclaimed", "claimed"})
EXCLUSIVE_SEED_MUTATIONS = frozenset(
	{"sysadmin_first_claim", "avery_instructor_approval"}
)
SERVICE_RECEIPTS = frozenset({"renderer_delivery", "worker_completion"})
FAULT_TRANSITIONS = frozenset({"gateway_submit_outage"})


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
	sysadmin_requirement: str
	visible_observation: str
	exclusive_seed_mutations: tuple[str, ...] = ()
	service_receipt: str | None = None
	fault_transition: str | None = None
	# Closed visual states exposed only when the suite runs its screenshot corpus.
	screenshot_states: tuple[str, ...] = ()
	# Compatibility-only construction field; V2 consumers use sysadmin_requirement.
	sysadmin_state: str | None = None


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
	_validate_sysadmin_requirement(contract)
	_validate_visible_observation(contract.visible_observation)
	_validate_service_receipt(contract.service_receipt)
	_validate_fault_transition(contract.fault_transition)
	if contract.screenshot_states:
		_validate_screenshot_states(contract.screenshot_states)
	_validate_exclusive_seed_mutations(contract.exclusive_seed_mutations)
	_validate_sysadmin_dependency(contract)


def validate_registry(
	contracts: Iterable[ScenarioContract] | None = None,
	repo_root: pathlib.Path | None = None,
) -> None:
	"""Validate catalog uniqueness and optional checked-in spec input consumption."""
	registry = tuple(scenario_contracts() if contracts is None else contracts)
	if not registry:
		raise ScenarioContractError("browser scenario registry is empty")
	seen_ids: set[str] = set()
	seen_specs: set[str] = set()
	seen_exclusive: set[str] = set()
	unclaimed_count = 0
	for contract in registry:
		_validate_registry_entry(contract, seen_ids, seen_specs, seen_exclusive)
		unclaimed_count += contract.sysadmin_requirement == "unclaimed"
		if repo_root is not None:
			_validate_spec_input_consumption(repo_root, contract)
	if unclaimed_count > 1:
		raise ScenarioContractError(
			"browser scenario registry has multiple first-claim contracts"
		)


def namespace_for(scenario_id: str, entropy: str) -> str:
	"""Create a public-safe scenario namespace from owner-generated entropy."""
	if re.fullmatch(r"[0-9a-f]{12}", entropy) is None:
		raise ScenarioContractError("browser scenario namespace entropy is invalid")
	result = f"bs1-{entropy}-{scenario_id}"
	if NAMESPACE_PATTERN.fullmatch(result) is None:
		raise ScenarioContractError("browser scenario namespace is invalid")
	return result


def validate_installed_baseline(
	receipt_path: pathlib.Path,
	contract: ScenarioContract,
) -> None:
	"""Bind a contract's semantic baseline to Rust's completed install receipt."""
	validate_contract(contract)
	receipt = _decode_installed_receipt(receipt_path)
	if json.loads(receipt.raw_output).get("baselineVersion") != BASELINE_VERSION:
		raise ScenarioContractError(
			"installed Base Course receipt has an incompatible baseline"
		)


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


def _validate_sysadmin_requirement(contract: ScenarioContract) -> None:
	if contract.sysadmin_requirement not in SYSADMIN_REQUIREMENTS:
		raise ScenarioContractError("browser scenario Sysadmin requirement is invalid")
	if contract.sysadmin_state is not None:
		raise ScenarioContractError(
			"browser scenario uses the retired Sysadmin state field"
		)


def _validate_visible_observation(value: str) -> None:
	if not value or not value.isascii():
		raise ScenarioContractError("browser scenario visible observation is invalid")


def _validate_service_receipt(value: str | None) -> None:
	if value is not None and value not in SERVICE_RECEIPTS:
		raise ScenarioContractError("browser scenario service receipt is invalid")


def _validate_fault_transition(value: str | None) -> None:
	"""Allow only a checked-in lifecycle fault transition."""
	if value is not None and value not in FAULT_TRANSITIONS:
		raise ScenarioContractError("browser scenario fault transition is invalid")


def _validate_exclusive_seed_mutations(values: tuple[str, ...]) -> None:
	if len(values) != len(set(values)):
		raise ScenarioContractError(
			"browser scenario exclusive seed mutations are invalid"
		)
	if not set(values).issubset(EXCLUSIVE_SEED_MUTATIONS):
		raise ScenarioContractError(
			"browser scenario exclusive seed mutations are invalid"
		)


def _validate_sysadmin_dependency(contract: ScenarioContract) -> None:
	"""Bind claim-dependent transitions and reject Sysadmin dependencies when absent."""
	if contract.sysadmin_requirement == "not_required":
		_validate_no_sysadmin_dependency(contract)
		return
	if contract.sysadmin_requirement == "unclaimed":
		owns_first_claim = (
			"morgan_sysadmin" in contract.personas
			and "passkey" in contract.ui_creates
			and "sysadmin_first_claim" in contract.exclusive_seed_mutations
		)
		if not owns_first_claim:
			raise ScenarioContractError(
				"unclaimed Sysadmin scenario must own the visible first claim"
			)
		return
	if contract.sysadmin_requirement == "claimed":
		if "morgan_sysadmin" not in contract.personas:
			raise ScenarioContractError(
				"claimed Sysadmin scenario must declare the Sysadmin persona"
			)
		if "passkey" in contract.ui_creates:
			raise ScenarioContractError(
				"claimed Sysadmin scenario consumes but does not create a passkey"
			)
	if (
		contract.sysadmin_requirement != "unclaimed"
		and "sysadmin_first_claim" in contract.exclusive_seed_mutations
	):
		raise ScenarioContractError(
			"only unclaimed Sysadmin scenario can mutate first claim"
		)


def _validate_no_sysadmin_dependency(contract: ScenarioContract) -> None:
	"""Keep an indifferent scenario independent of Sysadmin claim state."""
	if "morgan_sysadmin" in contract.personas:
		raise ScenarioContractError(
			"not-required browser scenario has a Sysadmin persona dependency"
		)
	if "passkey" in contract.ui_creates:
		raise ScenarioContractError(
			"not-required browser scenario has a Sysadmin passkey dependency"
		)
	if "sysadmin_first_claim" in contract.exclusive_seed_mutations:
		raise ScenarioContractError(
			"not-required browser scenario has a Sysadmin first-claim dependency"
		)


def _validate_registry_entry(
	contract: ScenarioContract,
	seen_ids: set[str],
	seen_specs: set[str],
	seen_exclusive: set[str],
) -> None:
	if contract.scenario_id in seen_ids:
		raise ScenarioContractError("browser scenario ids must be unique")
	if contract.spec_path in seen_specs:
		raise ScenarioContractError("browser scenario spec paths must be unique")
	if seen_exclusive.intersection(contract.exclusive_seed_mutations):
		raise ScenarioContractError(
			"browser scenario exclusive seed mutations must be unique"
		)
	seen_ids.add(contract.scenario_id)
	seen_specs.add(contract.spec_path)
	seen_exclusive.update(contract.exclusive_seed_mutations)
	validate_contract(contract)


def _validate_spec_input_consumption(
	repo_root: pathlib.Path,
	contract: ScenarioContract,
) -> None:
	spec = repo_root / contract.spec_path
	if not spec.is_file():
		raise ScenarioContractError("browser scenario spec is unavailable")
	contents = spec.read_text(encoding="utf-8")
	if "scenarioInput.namespace" not in contents:
		raise ScenarioContractError("browser scenario spec does not consume owner input")
	if "scenarioInput.scenarioId" not in contents:
		raise ScenarioContractError("browser scenario spec does not consume owner input")


def _decode_installed_receipt(
	receipt_path: pathlib.Path,
) -> local_stack_control.base_course_lifecycle.Receipt:
	try:
		contents = receipt_path.read_text(encoding="ascii")
		return local_stack_control.base_course_lifecycle.decode(contents, "install")
	except (
		OSError,
		UnicodeDecodeError,
		local_stack_control.models.ControllerError,
	) as error:
		raise ScenarioContractError(
			"installed Base Course receipt is unavailable"
		) from error


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
