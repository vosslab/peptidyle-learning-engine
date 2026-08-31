"""Offline contracts for the unregistered real WebWork UI scenario."""

import json
import pathlib
import sys

import pytest

import file_utils

E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_browser_scenario_webwork_delivery as webwork_delivery


#============================================
def test_catalog_baseline_receipt_accepts_only_public_question_location() -> None:
	"""The host hand-off contains only the reviewed Question ID and student title."""
	baseline = webwork_delivery.decode_catalog_baseline_receipt(
		'{"questionId":"ABC-1234","title":"Biochemistry: Identify hydrophobic compounds from formulas"}'
	)
	assert baseline.question_id == "ABC-1234"
	assert baseline.title == webwork_delivery.CATALOG_TITLE
	with pytest.raises(webwork_delivery.WebworkDeliveryEvidenceError, match="invalid"):
		webwork_delivery.decode_catalog_baseline_receipt(
			'{"questionId":"ABC-1234","source":"private","title":"Biochemistry: Identify hydrophobic compounds from formulas"}'
		)


#============================================
def test_catalog_baseline_private_input_is_canonical_and_private(tmp_path: pathlib.Path) -> None:
	"""The browser gets a small validated input rather than provider configuration."""
	path = tmp_path / "webwork-catalog-input.json"
	baseline = webwork_delivery.CatalogBaseline("ABC-1234", webwork_delivery.CATALOG_TITLE)
	webwork_delivery.write_catalog_baseline_input(path, baseline)
	assert path.stat().st_mode & 0o777 == 0o600
	assert webwork_delivery.validate_catalog_baseline_input(path) == baseline
	value = json.loads(path.read_text(encoding="ascii"))
	assert value["questionId"] == "ABC-1234"
	assert value["scenarioId"] == webwork_delivery.SCENARIO_ID
	assert value["schemaVersion"] == webwork_delivery.CATALOG_INPUT_SCHEMA_VERSION
	assert value["title"] == webwork_delivery.CATALOG_TITLE


#============================================
def test_renderer_call_witness_requires_one_new_content_free_event() -> None:
	"""A visible issue receives one safe service receipt without renderer internals."""
	before = 'INFO ple.webwork.cache event="cache_hit"\n'
	after = before + 'INFO ple.webwork.cache event="renderer_call"\n'
	witness = webwork_delivery.renderer_call_witness(before, after, 120)
	assert witness.as_value() == {
		"scenario": "webwork_delivery",
		"eventType": "renderer_call",
		"eventCount": 1,
		"observationWindowSeconds": 120,
	}
	with pytest.raises(webwork_delivery.WebworkDeliveryEvidenceError, match="one renderer"):
		webwork_delivery.renderer_call_witness(before, before, 120)
