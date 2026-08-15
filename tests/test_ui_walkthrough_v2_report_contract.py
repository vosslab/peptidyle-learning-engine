"""Hostile offline checks for the public schema-v2 walkthrough report boundary."""

import importlib
import json
import pathlib
import sys
import tempfile
import unittest
from unittest import mock


WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parent / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))
contract = importlib.import_module("walklib.v2_report_contract")
walkthrough = importlib.import_module("walklib.runner")


def valid_output(master_seed: int = 1) -> str:
	"""Create one exact canonical public report with no identity-bearing fields."""
	journeys = [
		{
			"journey": journey,
			"status": "PASS",
			"elapsedMs": index + 1,
			"visibleOutcomeCodes": codes,
			"diagnostics": [],
		}
		for index, (journey, codes) in enumerate(contract.EXPECTED_JOURNEYS)
	]
	payload = {
		"schemaVersion": 2,
		"status": "PASS",
		"masterSeed": master_seed,
		"stage": "complete",
		"elapsedMs": sum(row["elapsedMs"] for row in journeys),
		"arrangements": [{"label": "api-retry-corpus-publication"}],
		"journeys": journeys,
	}
	return json.dumps(payload, separators=(",", ":")) + "\n"


class V2ReportContractTests(unittest.TestCase):
	"""Prove the report boundary does not confuse Python booleans with integer IDs."""

	def test_boolean_seeds_duplicate_keys_and_inconsistent_aggregate_fail_closed(self) -> None:
		"""Canonical JSON cannot smuggle a boolean, duplicate key, or false elapsed total."""
		for seed, replacement in ((1, "true"), (0, "false")):
			with self.subTest(seed=seed):
				output = valid_output(seed).replace(f'"masterSeed":{seed}', f'"masterSeed":{replacement}')
				with self.assertRaises(ValueError):
					contract.parse_public_v2_report(output, seed)
		with self.assertRaises(ValueError):
			contract.parse_public_v2_report(
				valid_output().replace('"elapsedMs":45', '"elapsedMs":46', 1), 1
			)
		with self.assertRaises(ValueError):
			contract.parse_public_v2_report(
				valid_output().replace('"status":"PASS"', '"status":"PASS","status":"PASS"', 1), 1
			)

	def test_renderer_success_stderr_fails_without_reading_its_public_output(self) -> None:
		"""Unexpected renderer diagnostics cannot accompany a successful redacted report."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = pathlib.Path(temporary_name)
			arranger = repository_root / "node_modules" / "tsx" / "dist"
			arranger.mkdir(parents=True)
			(arranger / "cli.mjs").write_text("export {};\n", encoding="ascii")
			state = repository_root / "journeys.json"
			state.write_text("[]\n", encoding="ascii")
			inputs = walkthrough.RunnerInputs(1, state, "report.json", False, False, False, False)

			def run_child(
				command: list[str], environ: dict[str, str] | None, stdin: str | None = None
			) -> object:
				if stdin is not None:
					raise AssertionError("report rendering does not accept stdin")
				return walkthrough.CommandResult(0, valid_output(), "unexpected diagnostic")

			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, run_child)
			runner.journey_state_file = state
			with mock.patch.object(walkthrough.shutil, "which", return_value="/bin/echo"):
				with self.assertRaisesRegex(walkthrough.RunnerError, "renderer failed"):
					runner.collect_visible_outcomes()


if __name__ == "__main__":
	unittest.main()
