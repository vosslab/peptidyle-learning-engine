"""Strict host-side boundary for the authoritative Base Course lifecycle."""

import dataclasses
import json
import uuid

import local_stack_control.compose
import local_stack_control.lifecycle_diagnostics
import local_stack_control.models
import local_stack_control.process


@dataclasses.dataclass(frozen=True)
class Receipt:
	"""One strictly decoded response from the authoritative Base Course command."""

	action: str
	install_state: str
	storage_receipt_bucket: str
	storage_receipt_key: str
	storage_receipt_json: str
	installation_generation: str
	raw_output: str


def ensure_storage_receipt(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	receipt: Receipt,
	environment: dict[str, str],
) -> None:
	"""Create or resume only the generation-bound receipt in otherwise empty storage."""
	argv = local_stack_control.compose.compose_argv(target, [
		"run", "--rm", "--no-deps", "-T", "--entrypoint", "/bin/sh", "createbuckets", "-ec",
		"stage=alias; printf 'storage-receipt-stage=%s\\n' \"$stage\" >&2; "
		"mc alias set local http://minio:9000 \"$MINIO_ROOT_USER\" \"$MINIO_ROOT_PASSWORD\" >/dev/null; "
		"stage=receipt-read; printf 'storage-receipt-stage=%s\\n' \"$stage\" >&2; "
		"expected=$(cat); receipt_file=; trap 'rm -f \"$receipt_file\"' EXIT; total=0; allowed=0; "
		"for bucket in public-assets private-content student-records temp-processing; do "
		"stage=inventory-$bucket; printf 'storage-receipt-stage=%s\\n' \"$stage\" >&2; "
		"entries=$(mc find \"local/$bucket\" --json) || exit $?; "
		"if [ -z \"$entries\" ]; then count=0; else count=$(printf '%s\\n' \"$entries\" | wc -l); fi; "
		"total=$((total + count)); "
		"if [ \"$bucket\" = \"$1\" ] && [ \"$count\" -eq 1 ] && mc stat \"local/$1/$2\" >/dev/null 2>&1; then allowed=1; fi; "
		"done; if [ \"$total\" -eq 0 ]; then stage=create; printf 'storage-receipt-stage=%s\\n' \"$stage\" >&2; receipt_file=$(mktemp /tmp/base-course-storage-receipt.XXXXXX); chmod 600 \"$receipt_file\"; printf '%s' \"$expected\" >\"$receipt_file\"; mc cp --disable-multipart \"$receipt_file\" \"local/$1/$2\" >/dev/null; printf created; "
		"elif [ \"$total\" -eq 1 ] && [ \"$allowed\" -eq 1 ]; then stage=resume-read; printf 'storage-receipt-stage=%s\\n' \"$stage\" >&2; if [ \"$(mc cat \"local/$1/$2\")\" = \"$expected\" ]; then printf resumed; else printf refused; exit 17; fi; "
		"else printf refused; exit 17; fi",
		"base-course-storage",
		receipt.storage_receipt_bucket, receipt.storage_receipt_key,
	])
	result = runner.run(argv, environment, target.repo_root, receipt.storage_receipt_json)
	if result.returncode != 0 or result.stdout.strip() not in {"created", "resumed"}:
		private_values = tuple(
			value for value in (
				*environment.values(), receipt.storage_receipt_json,
			) if len(value) >= 8
		)
		detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
			result, private_values
		)
		raise local_stack_control.models.ControllerError(
			"installed Base Course storage receipt cannot safely resume "
			f"(exit status {result.returncode}; {detail})"
		)


def decode(output: str, phase: str) -> Receipt:
	"""Strictly decode a v1 lifecycle response without accepting extension fields."""
	try:
		value = json.loads(output)
	except json.JSONDecodeError as error:
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt") from error
	if not isinstance(value, dict):
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt")
	base = {"schemaVersion", "action", "installState", "baselineVersion", "objectManifest", "installationGeneration", "storageReceiptBucket", "storageReceiptKey", "storageReceiptJson"}
	complete = base | {"storageReceiptSha256", "completionReceiptSha256"}
	if phase == "install" and value.get("action") != "retained":
		complete.add("manifest")
	expected = complete if value.get("action") == "retained" or phase == "install" else base
	if set(value) != expected or type(value.get("schemaVersion")) is not int or value.get("schemaVersion") != 1 or value.get("baselineVersion") != "base-course-v1" or value.get("objectManifest") != []:
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt")
	if not all(isinstance(value.get(field), str) for field in ("installationGeneration", "storageReceiptBucket", "storageReceiptKey", "storageReceiptJson")):
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt")
	try:
		generation = uuid.UUID(value["installationGeneration"])
	except ValueError as error:
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt") from error
	if str(generation) != value["installationGeneration"]:
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt")
	storage = value["storageReceiptJson"]
	try:
		storage_value = json.loads(storage)
	except json.JSONDecodeError as error:
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned a noncanonical storage receipt") from error
	if json.dumps(storage_value, separators=(",", ":"), ensure_ascii=True) != storage or storage_value != {"schemaVersion": 1, "baselineVersion": "base-course-v1", "installationGeneration": value["installationGeneration"], "storageReceiptBucket": value["storageReceiptBucket"], "storageReceiptKey": value["storageReceiptKey"], "objectManifest": []}:
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned a noncanonical storage receipt")
	if phase == "prepare":
		valid = (value["action"] == "retained" and value["installState"] == "complete") or (value["action"] in {"prepared", "resumed"} and value["installState"] == "installing")
	else:
		valid = (
			value["action"] in {"installed", "resumed", "retained"}
			and value["installState"] == "complete"
			and all(
				isinstance(value.get(field), str)
				and len(value[field]) == 64
				and all(character in "0123456789abcdef" for character in value[field])
				for field in ("storageReceiptSha256", "completionReceiptSha256")
			)
		)
	if not valid:
		raise local_stack_control.models.ControllerError("installed Base Course lifecycle returned an invalid state")
	manifest = value.get("manifest")
	if manifest is not None and (not isinstance(manifest, dict) or set(manifest) != {"assignmentId", "enrollmentId", "questionId", "problemId", "versionId"} or any(not isinstance(item, str) or item == "" for item in manifest.values())):
		raise local_stack_control.models.ControllerError("installed Base Course reconciliation returned an invalid receipt")
	return Receipt(value["action"], value["installState"], value["storageReceiptBucket"], value["storageReceiptKey"], storage, value["installationGeneration"], output)
