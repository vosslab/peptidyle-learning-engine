"""Offline policy tests for the generic browser scenario contract."""

import dataclasses
import json
import pathlib
import sys

import pytest

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_contract as browser_scenario_contract


def scenario_contract(
	scenario_id: str = "first_claim",
	spec_path: str = "tests/playwright/e2e/first_claim.spec.ts",
	*,
	sysadmin_requirement: str = "unclaimed",
	exclusive_seed_mutations: tuple[str, ...] = (
		"sysadmin_first_claim",
		"avery_instructor_approval",
	),
) -> browser_scenario_contract.ScenarioContract:
	"""Build one complete local policy fixture without importing any family provider."""
	return browser_scenario_contract.ScenarioContract(
		scenario_id=scenario_id,
		spec_path=spec_path,
		personas=("elena_instructor", "morgan_sysadmin"),
		baseline_reads=("base_course",),
		ui_creates=("course", "passkey"),
		sysadmin_requirement=sysadmin_requirement,
		visible_observation="visible_policy_observation",
		exclusive_seed_mutations=exclusive_seed_mutations,
	)


def installed_receipt(
	generation: str = "00000000-0000-0000-0000-000000000006",
) -> str:
	"""Return a canonical completed Rust Base Course lifecycle receipt."""
	storage = json.dumps(
		{
			"schemaVersion": 1,
			"baselineVersion": "base-course-v1",
			"installationGeneration": generation,
			"storageReceiptBucket": "private-content",
			"storageReceiptKey": "ple/live-demo/base-course-install-receipt.json",
			"objectManifest": [],
		},
		separators=(",", ":"),
	)
	return json.dumps(
		{
			"schemaVersion": 1,
			"action": "installed",
			"installState": "complete",
			"baselineVersion": "base-course-v1",
			"objectManifest": [],
			"installationGeneration": generation,
			"storageReceiptBucket": "private-content",
			"storageReceiptKey": "ple/live-demo/base-course-install-receipt.json",
			"storageReceiptJson": storage,
			"storageReceiptSha256": "a" * 64,
			"manifest": {
				"assignmentId": "a",
				"enrollmentId": "e",
				"questionId": "q",
				"problemId": "p",
				"versionId": "v",
			},
		},
		separators=(",", ":"),
	)


#============================================
def test_generic_registry_keeps_one_unique_mapping_per_contract() -> None:
	"""The primitive validates an explicit catalog without importing a family provider."""
	contract = scenario_contract()
	browser_scenario_contract.validate_registry((contract,))
	assert {contract.spec_path: contract.scenario_id} == {
		"tests/playwright/e2e/first_claim.spec.ts": "first_claim"
	}
	assert not hasattr(browser_scenario_contract, "LIVE_DEMO_CONTRACT")
	assert not hasattr(browser_scenario_contract, "SCENARIO_CONTRACTS")


#============================================
@pytest.mark.parametrize(
	"contract",
	[
		dataclasses.replace(scenario_contract(), scenario_id="invalid-id!"),
		dataclasses.replace(scenario_contract(), personas=("unknown",)),
		dataclasses.replace(scenario_contract(), baseline_reads=("unknown",)),
		dataclasses.replace(scenario_contract(), ui_creates=("unknown",)),
		dataclasses.replace(scenario_contract(), sysadmin_state="impossible"),
	],
)
def test_invalid_contracts_reject_before_lifecycle_allocation(
	contract: browser_scenario_contract.ScenarioContract,
) -> None:
	"""Closed IDs, aliases, resource kinds, and states form a preallocation boundary."""
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.validate_contract(contract)


#============================================
def test_duplicate_contract_ids_and_paths_reject_before_lifecycle_allocation() -> None:
	"""A registry never leaves selected-scenario behavior ambiguous."""
	contract = scenario_contract()
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="unique"):
		browser_scenario_contract.validate_registry((contract, contract))
	duplicate_path = dataclasses.replace(contract, scenario_id="other")
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="spec paths"):
		browser_scenario_contract.validate_registry((contract, duplicate_path))


#============================================
def test_unclaimed_first_claim_and_later_approval_can_have_separate_owners() -> None:
	"""A claimed authentication journey may own approval after the first-claim transition."""
	first_claim = scenario_contract(exclusive_seed_mutations=("sysadmin_first_claim",))
	approval = scenario_contract(
		"approval",
		"tests/playwright/e2e/approval.spec.ts",
		sysadmin_requirement="claimed",
		exclusive_seed_mutations=("avery_instructor_approval",),
	)
	browser_scenario_contract.validate_registry((first_claim, approval))


#============================================
def test_unclaimed_contract_requires_sysadmin_first_claim_exclusive() -> None:
	"""The visible unclaimed journey retains exclusive ownership of its first claim."""
	missing_first_claim = scenario_contract(
		exclusive_seed_mutations=("avery_instructor_approval",)
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="first claim"):
		browser_scenario_contract.validate_contract(missing_first_claim)


#============================================
def test_duplicate_exclusive_mutations_reject_across_split_contracts() -> None:
	"""Separate family contracts still receive each exclusive fixture transition once."""
	first_claim = scenario_contract(exclusive_seed_mutations=("sysadmin_first_claim",))
	duplicate = scenario_contract(
		"other_unclaimed",
		"tests/playwright/e2e/other_unclaimed.spec.ts",
		exclusive_seed_mutations=("sysadmin_first_claim",),
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match="exclusive"):
		browser_scenario_contract.validate_registry((first_claim, duplicate))


#============================================
def test_transitional_combined_exclusives_remain_valid() -> None:
	"""A combined journey remains valid until its dedicated family split is complete."""
	browser_scenario_contract.validate_contract(scenario_contract())


#============================================
def test_not_required_contract_has_no_sysadmin_dependency() -> None:
	"""Instructor and learner journeys may run before or after claim without reordering."""
	contract = dataclasses.replace(
		scenario_contract(),
		scenario_id="indifferent",
		spec_path="tests/playwright/e2e/indifferent.spec.ts",
		personas=("elena_instructor",),
		ui_creates=("course",),
		sysadmin_requirement="not_required",
		exclusive_seed_mutations=(),
	)
	browser_scenario_contract.validate_contract(contract)


#============================================
@pytest.mark.parametrize(
	("change", "message"),
	[
		(lambda contract: dataclasses.replace(contract, personas=("elena_instructor", "morgan_sysadmin")), "persona"),
		(lambda contract: dataclasses.replace(contract, ui_creates=("course", "passkey")), "passkey"),
		(lambda contract: dataclasses.replace(contract, exclusive_seed_mutations=("sysadmin_first_claim",)), "first-claim"),
	],
)
def test_not_required_contract_rejects_sysadmin_dependencies(
	change: object,
	message: str,
) -> None:
	"""The indifferent state means no Sysadmin lifecycle dependency, not pristine state."""
	contract = dataclasses.replace(
		scenario_contract(),
		scenario_id="indifferent",
		spec_path="tests/playwright/e2e/indifferent.spec.ts",
		personas=("elena_instructor",),
		ui_creates=("course",),
		sysadmin_requirement="not_required",
		exclusive_seed_mutations=(),
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError, match=message):
		change(contract)  # type: ignore[operator]
		browser_scenario_contract.validate_contract(change(contract))  # type: ignore[operator]


#============================================
def test_current_instructor_and_learner_catalog_contracts_remain_indifferent() -> None:
	"""The registered UI-only families continue to validate as no-Sysadmin-dependency journeys."""
	registry = browser_scenario_contract.scenario_contracts()
	browser_scenario_contract.validate_registry(registry)
	for scenario_id in ("instructor_authoring", "learner_delivery"):
		contract = browser_scenario_contract.require_contract(scenario_id, registry)
		assert contract.sysadmin_requirement == "not_required"


#============================================
def test_namespace_is_public_safe_and_bound_to_the_selected_scenario() -> None:
	"""Focus and order receive independent namespaces without caller-selected state."""
	assert (
		browser_scenario_contract.namespace_for("first_claim", "0123456789ab")
		== "bs1-0123456789ab-first_claim"
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.namespace_for("first_claim", "UPPERCASE000")


#============================================
def test_installed_receipt_binds_semantic_aliases_without_copying_database_layout(
	tmp_path: pathlib.Path,
) -> None:
	"""Rust lifecycle version authority supports the semantic browser contract."""
	contract = scenario_contract()
	receipt = tmp_path / "base-course.json"
	receipt.write_text(installed_receipt(), encoding="ascii")
	browser_scenario_contract.validate_installed_baseline(receipt, contract)
	receipt.write_text(
		installed_receipt().replace("base-course-v1", "different", 1),
		encoding="ascii",
	)
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		browser_scenario_contract.validate_installed_baseline(receipt, contract)
	with pytest.raises(browser_scenario_contract.ScenarioContractError):
		receipt.write_text(installed_receipt("not-a-uuid"), encoding="ascii")
		browser_scenario_contract.validate_installed_baseline(receipt, contract)


#============================================
def test_live_demo_product_names_use_only_the_owner_namespace() -> None:
	"""Scenario execution never derives product state labels from wall clock or worker index."""
	spec_path = pathlib.Path(file_utils.get_repo_root()) / "tests/playwright/e2e/live_demo.spec.ts"
	contents = spec_path.read_text(encoding="utf-8")
	assert "scenarioInput.namespace" in contents
	assert "Date.now" not in contents
	assert "parallelIndex" not in contents
	assert ".route(" not in contents
	assert "await signOut(avery);" in contents
	assert "await chooseSeededAccount(avery, /Avery Singh/u);" in contents
