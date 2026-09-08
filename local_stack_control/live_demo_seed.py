"""Fixed, private source and answer-free manifest for the disposable Live Demo."""

import base64
import dataclasses
import hashlib
import json
import pathlib


SEED_TIMESTAMP = "2026-09-06T00:00:00Z"
SEED_TIMESTAMP_MILLIS = 1_788_652_800_000
PLE_QUESTION_JSON_MEDIA_TYPE = "application/vnd.peptidyle.question+json"
PNG_MEDIA_TYPE = "image/png"
SEED_SOURCE_DIRECTORY = pathlib.Path("local_stack_control/live_demo_seed_data")


@dataclasses.dataclass(frozen=True)
class SeededAccount:
	"""One fixed Product Role mapping for deployment-gated seeded entry."""

	setting: str
	account_id: str
	product_role: str


@dataclasses.dataclass(frozen=True)
class SeededPublishedQuestion:
	"""One private source with an answer-free Question Library declaration."""

	question_id: str
	backend: str
	question_title: str
	question_description: str
	source_object_id: str
	source_filename: str


@dataclasses.dataclass(frozen=True)
class SeededQuestionAssetPublication:
	"""One fixed private raster and Pending public rendition for PNE-0004."""

	question_id: str
	revision_number: int
	asset_id: str
	private_object_id: str
	public_object_id: str
	delivery_id: str
	job_id: str
	filename: str
	checksum: str
	width: int
	height: int


SEEDED_ACCOUNTS = (
	SeededAccount(
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
		"00000000-0000-0000-0000-000000000101",
		"instructor",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
		"00000000-0000-0000-0000-000000000102",
		"student",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
		"00000000-0000-0000-0000-000000000103",
		"student",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
		"00000000-0000-0000-0000-000000000104",
		"student",
	),
	SeededAccount(
		"PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
		"00000000-0000-0000-0000-000000000105",
		"sysadmin",
	),
)

# Private fixed Student Authentication Email bindings let the seeded sessions
# exercise the normal Course Roster Import resolution path.
SEEDED_STUDENT_AUTHENTICATION_EMAILS = (
	("00000000-0000-0000-0000-000000000102", "mary.student@live-demo.invalid"),
	("00000000-0000-0000-0000-000000000103", "jack.student@live-demo.invalid"),
	("00000000-0000-0000-0000-000000000104", "avery.student@live-demo.invalid"),
)

SEEDED_PUBLISHED_QUESTIONS = (
	SeededPublishedQuestion(
		"PNE-0001",
		"ple",
		"Peptide bond rotation",
		"Relate peptide-bond resonance to backbone conformational constraints.",
		"00000000-0000-0000-0000-000000001101",
		"ple_0001.json",
	),
	SeededPublishedQuestion(
		"PNE-0002",
		"ple",
		"Alpha-helix hydrogen bonds",
		"Identify the repeated backbone interaction that stabilizes an alpha helix.",
		"00000000-0000-0000-0000-000000001102",
		"ple_0002.json",
	),
	SeededPublishedQuestion(
		"PNE-0003",
		"ple",
		"Peptide net charge",
		"Calculate a simple peptide net charge from stated ionizable groups.",
		"00000000-0000-0000-0000-000000001103",
		"ple_0003.json",
	),
	SeededPublishedQuestion(
		"PNE-0004",
		"ple",
		"Amino terminus",
		"Use peptide directionality vocabulary to name the free alpha-amino end.",
		"00000000-0000-0000-0000-000000001104",
		"ple_0004.json",
	),
)


# This is seed-owned acceptance infrastructure, not a browser-supplied upload.
# The dedicated publisher is the only component allowed to copy this private
# restricted Question Asset to its fixed Public Assets rendition and activate it.
SEEDED_QUESTION_ASSET_PUBLICATION = SeededQuestionAssetPublication(
	question_id="PNE-0004",
	revision_number=1,
	asset_id="00000000-0000-0000-0000-000000001204",
	private_object_id="00000000-0000-0000-0000-000000001304",
	public_object_id="00000000-0000-0000-0000-000000001404",
	delivery_id="00000000-0000-0000-0000-000000001504",
	job_id="00000000-0000-0000-0000-000000001604",
	filename="two_residue_peptide_backbone.png",
	checksum="0aa945dc75c80da56e657d7bed4202aa3b410063f827d01a9b64fbc81697cfde",
	width=560,
	height=320,
)


#============================================
def source_path(repo_root: pathlib.Path, question: SeededPublishedQuestion) -> pathlib.Path:
	"""Return one checked-in private source path under the repository root."""
	path = repo_root / SEED_SOURCE_DIRECTORY / question.source_filename
	return path


#============================================
def question_asset_path(repo_root: pathlib.Path) -> pathlib.Path:
	"""Return the one repository-owned raster accepted by the seeded baseline."""
	return repo_root / SEED_SOURCE_DIRECTORY / SEEDED_QUESTION_ASSET_PUBLICATION.filename


#============================================
def question_asset_bytes(repo_root: pathlib.Path) -> bytes:
	"""Read the fixed PNE-0004 private asset without accepting a file selector."""
	return question_asset_path(repo_root).read_bytes()


#============================================
def question_asset_dimensions(repo_root: pathlib.Path) -> tuple[int, int]:
	"""Read intrinsic dimensions from the fixed PNG, refusing another container."""
	data = question_asset_bytes(repo_root)
	if data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR" or len(data) < 24:
		raise ValueError("seeded Question Asset is not a PNG with an IHDR header")
	return int.from_bytes(data[16:20], "big"), int.from_bytes(data[20:24], "big")


#============================================
def require_question_asset_baseline(repo_root: pathlib.Path) -> None:
	"""Refuse modified seed art before any private object upload or SQL write."""
	asset = SEEDED_QUESTION_ASSET_PUBLICATION
	if hashlib.sha256(question_asset_bytes(repo_root)).hexdigest() != asset.checksum:
		raise ValueError("seeded Question Asset checksum is incompatible")
	if question_asset_dimensions(repo_root) != (asset.width, asset.height):
		raise ValueError("seeded Question Asset dimensions are incompatible")


#============================================
def source_bytes(repo_root: pathlib.Path, question: SeededPublishedQuestion) -> bytes:
	"""Read one fixed source without accepting a caller-selected file path."""
	data = source_path(repo_root, question).read_bytes()
	return data


#============================================
def source_checksum(repo_root: pathlib.Path, question: SeededPublishedQuestion) -> str:
	"""Return the exact lowercase SHA-256 for one immutable source."""
	checksum = hashlib.sha256(source_bytes(repo_root, question)).hexdigest()
	return checksum


#============================================
def source_object_path(question: SeededPublishedQuestion) -> str:
	"""Return the typed private-content key for one first Question Revision."""
	path = (
		f"questions/{question.question_id}/versions/1/source/"
		f"{question.source_object_id}"
	)
	return path


#============================================
def restricted_question_asset_object_path() -> str:
	"""Return the typed private key for PNE-0004's fixed hotspot surface."""
	asset = SEEDED_QUESTION_ASSET_PUBLICATION
	return (
		f"questions/{asset.question_id}/versions/{asset.revision_number}/restricted-assets/"
		f"{asset.asset_id}/{asset.private_object_id}"
	)


#============================================
def answer_free_manifest() -> tuple[dict[str, str | int], ...]:
	"""Return the browser-safe Question Library declaration without source data."""
	manifest = tuple(
		{
			"questionId": question.question_id,
			"revisionNumber": 1,
			"backend": question.backend,
			"questionTitle": question.question_title,
			"questionDescription": question.question_description,
		}
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	return manifest


#============================================
def object_record_metadata(
	repo_root: pathlib.Path,
	question: SeededPublishedQuestion,
) -> str:
	"""Encode the immutable object record metadata required by the S3 adapter."""
	checksum = source_checksum(repo_root, question)
	address = {
		"kind": "questionSource",
		"questionRevision": {
			"questionId": question.question_id,
			"revisionNumber": 1,
		},
		"object": question.source_object_id,
	}
	record = {
		"id": question.source_object_id,
		"storageArea": "private-content",
		"dataClass": "question-source",
		"address": address,
		"sha256": checksum,
		"sizeBytes": len(source_bytes(repo_root, question)),
		"mediaType": PLE_QUESTION_JSON_MEDIA_TYPE,
		"questionRevision": address["questionRevision"],
		"createdAt": SEED_TIMESTAMP_MILLIS,
	}
	encoded = json.dumps(record, separators=(",", ":"), ensure_ascii=True).encode("ascii")
	value = base64.urlsafe_b64encode(encoded).decode("ascii").rstrip("=")
	return value


#============================================
def question_asset_record_metadata(repo_root: pathlib.Path) -> str:
	"""Encode the closed private Object Record metadata for the fixed PNG only."""
	require_question_asset_baseline(repo_root)
	asset = SEEDED_QUESTION_ASSET_PUBLICATION
	address = {
		"kind": "restrictedQuestionAsset",
		"questionRevision": {
			"questionId": asset.question_id,
			"revisionNumber": asset.revision_number,
		},
		"asset": asset.asset_id,
		"object": asset.private_object_id,
	}
	record = {
		"id": asset.private_object_id,
		"storageArea": "private-content",
		"dataClass": "question-asset",
		"address": address,
		"sha256": asset.checksum,
		"sizeBytes": len(question_asset_bytes(repo_root)),
		"mediaType": PNG_MEDIA_TYPE,
		"questionRevision": address["questionRevision"],
		"createdAt": SEED_TIMESTAMP_MILLIS,
	}
	encoded = json.dumps(record, separators=(",", ":"), ensure_ascii=True).encode("ascii")
	return base64.urlsafe_b64encode(encoded).decode("ascii").rstrip("=")


#============================================
def seed_sql(repo_root: pathlib.Path) -> str:
	"""Return one fixed idempotent SQL transaction for the declared demo baseline."""
	require_question_asset_baseline(repo_root)
	asset = SEEDED_QUESTION_ASSET_PUBLICATION
	accounts = ",\n\t".join(
		f"('{account.account_id}', '{account.product_role}', '{SEED_TIMESTAMP}')"
		for account in SEEDED_ACCOUNTS
	)
	expected_accounts = ",\n\t".join(
		f"('{account.account_id}', '{account.product_role}')"
		for account in SEEDED_ACCOUNTS
	)
	student_authentication_emails = ",\n\t".join(
		f"('{account_id}', '{email}', '{email}', '{SEED_TIMESTAMP}', '{SEED_TIMESTAMP}')"
		for account_id, email in SEEDED_STUDENT_AUTHENTICATION_EMAILS
	)
	question_roots = ",\n\t".join(
		f"('{question.question_id}', '{SEED_TIMESTAMP}')"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	question_revisions = ",\n\t".join(
		f"('{question.question_id}', 1, '{question.backend}', '{SEED_TIMESTAMP}')"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	metadata = ",\n\t".join(
		"(" + ", ".join((
			f"'{question.question_id}'",
			f"'{question.question_title}'",
			f"'{question.question_description}'",
			f"'{SEED_TIMESTAMP}'",
			f"'{SEED_TIMESTAMP}'",
		)) + ")"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	objects = ",\n\t".join(
		"(" + ", ".join((
			f"'{question.source_object_id}'",
			"jsonb_build_object('kind', 'questionSource', 'questionRevision', "
			f"jsonb_build_object('questionId', '{question.question_id}', 'revisionNumber', 1), "
			f"'object', '{question.source_object_id}'::uuid)",
			"'private-content'",
			"'question-source'",
			f"decode('{source_checksum(repo_root, question)}', 'hex')",
			str(len(source_bytes(repo_root, question))),
			f"'{PLE_QUESTION_JSON_MEDIA_TYPE}'",
			f"'{SEED_TIMESTAMP}'",
		)) + ")"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	source_bindings = ",\n\t".join(
		"(" + ", ".join((
			f"'{question.question_id}'",
			"1",
			f"'{question.backend}'",
			"'pleQuestionJson'",
			f"'{question.source_object_id}'",
			f"'{source_checksum(repo_root, question)}'",
			f"'{SEED_TIMESTAMP}'",
		)) + ")"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	asset_object = "(" + ", ".join((
		f"'{asset.private_object_id}'",
		"jsonb_build_object('kind', 'restrictedQuestionAsset', 'questionRevision', "
		f"jsonb_build_object('questionId', '{asset.question_id}', 'revisionNumber', {asset.revision_number}), "
		f"'asset', '{asset.asset_id}'::uuid, 'object', '{asset.private_object_id}'::uuid)",
		"'private-content'",
		"'question-asset'",
		f"decode('{asset.checksum}', 'hex')",
		str(len(question_asset_bytes(repo_root))),
		f"'{PNG_MEDIA_TYPE}'",
		f"'{SEED_TIMESTAMP}'",
	)) + ")"
	acceptances = ",\n\t".join(
		f"('{question.question_id}', 1, NULL, '{SEEDED_ACCOUNTS[0].account_id}', "
		f"'{SEEDED_ACCOUNTS[0].account_id}', '{SEED_TIMESTAMP}', 'Initial Live Demo baseline')"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	authorship = ",\n\t".join(
		f"('{question.question_id}', 1, 1, 'Elena Instructor', '{SEEDED_ACCOUNTS[0].account_id}')"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	licenses = ",\n\t".join(
		f"('{question.question_id}', 1, 'CC-BY-SA-4.0')"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	ownership = ",\n\t".join(
		f"('00000000-0000-0000-0000-00000000210{index}', '{question.question_id}', "
		f"'{SEEDED_ACCOUNTS[0].account_id}', '{SEEDED_ACCOUNTS[0].account_id}', 'initial', '{SEED_TIMESTAMP}')"
		for index, question in enumerate(SEEDED_PUBLISHED_QUESTIONS, start=1)
	)
	publication_events = ",\n\t".join(
		f"('00000000-0000-0000-0000-00000000310{index}', '{question.question_id}', 1, '{SEED_TIMESTAMP}')"
		for index, question in enumerate(SEEDED_PUBLISHED_QUESTIONS, start=1)
	)
	availability_events = ",\n\t".join(
		f"('00000000-0000-0000-0000-00000000410{index}', '{question.question_id}', 1, 'available', NULL, '{SEED_TIMESTAMP}')"
		for index, question in enumerate(SEEDED_PUBLISHED_QUESTIONS, start=1)
	)
	expected_questions = ",\n\t".join(
		"(" + ", ".join((
			f"'{question.question_id}'",
			f"'{question.backend}'",
			f"'{question.question_title}'",
			f"'{question.question_description}'",
			f"'{question.source_object_id}'",
			f"'{source_checksum(repo_root, question)}'",
		)) + ")"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	sql = f"""BEGIN;
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES
	{accounts}
ON CONFLICT (account_id) DO NOTHING;
INSERT INTO ple_private.account_authentication_email (
	account_id, normalized_email, delivery_email, verified_at, updated_at
)
VALUES
	{student_authentication_emails}
ON CONFLICT (account_id) DO NOTHING;
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES
	{question_roots}
ON CONFLICT (question_id) DO NOTHING;
INSERT INTO ple_data.question_revision (question_id, revision_number, backend, published_at)
VALUES
	{question_revisions}
ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.published_question_metadata (
	question_id, question_title, question_description, created_at, updated_at
) VALUES
	{metadata}
ON CONFLICT (question_id) DO NOTHING;
INSERT INTO ple_private.object_record (
	object_id, object_address, object_storage_area, object_data_class, sha256,
	size_bytes, media_type, created_at
) VALUES
	{objects}
ON CONFLICT (object_id) DO NOTHING;
INSERT INTO ple_private.object_record (
	object_id, object_address, object_storage_area, object_data_class, sha256,
	size_bytes, media_type, created_at
) VALUES
	{asset_object}
ON CONFLICT (object_id) DO NOTHING;
INSERT INTO ple_private.question_revision_source_binding (
	question_id, revision_number, backend, question_format, source_object_id,
	source_object_checksum, created_at
) VALUES
	{source_bindings}
ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_revision_acceptance (
	question_id, revision_number, parent_revision_number, editor_account_id,
	accepted_by_account_id, accepted_at, reason_for_edit
) VALUES
	{acceptances}
ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_revision_authorship (
	question_id, revision_number, author_position, author_display_name, author_account_id
) VALUES
	{authorship}
ON CONFLICT (question_id, revision_number, author_position) DO NOTHING;
INSERT INTO ple_data.question_revision_license (question_id, revision_number, spdx_expression)
VALUES
	{licenses}
ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_ownership_event (
	question_ownership_event_id, question_id, owner_account_id, recorded_by_account_id,
	event_kind, occurred_at
) VALUES
	{ownership}
ON CONFLICT (question_ownership_event_id) DO NOTHING;
INSERT INTO ple_data.question_publication_event (
	event_id, question_id, revision_number, published_at
) VALUES
	{publication_events}
ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_revision_availability_event (
	event_id, question_id, revision_number, availability, reason, occurred_at
) VALUES
	{availability_events}
ON CONFLICT (question_id, revision_number, availability) DO NOTHING;
INSERT INTO ple_private.job (
	job_id, job_kind, job_target_kind, question_id, revision_number, generation,
	payload, state, available_at, max_attempts, created_at
) VALUES (
	'{asset.job_id}', 'publish_public_assets', 'public_asset_publication',
	'{asset.question_id}', {asset.revision_number}, 1, '{{}}'::jsonb, 'ready',
	'{SEED_TIMESTAMP}', 1, '{SEED_TIMESTAMP}'
)
ON CONFLICT (job_id) DO NOTHING;
INSERT INTO ple_data.object_delivery (
	delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at
) VALUES (
	'{asset.delivery_id}', '{asset.public_object_id}', decode('{asset.checksum}', 'hex'),
	'{PNG_MEDIA_TYPE}', {len(question_asset_bytes(repo_root))}, 'pending', '{SEED_TIMESTAMP}'
)
ON CONFLICT (delivery_id) DO NOTHING;
INSERT INTO ple_data.question_asset_delivery (
	delivery_id, object_id, question_id, revision_number, asset_id
) VALUES (
	'{asset.delivery_id}', '{asset.public_object_id}', '{asset.question_id}',
	{asset.revision_number}, '{asset.asset_id}'
)
ON CONFLICT (delivery_id) DO NOTHING;
INSERT INTO ple_private.question_asset_publication (
	question_id, revision_number, asset_id, source_object_id, source_object_checksum,
	public_object_id, public_object_checksum, public_byte_length, verified_media_type,
	intrinsic_width, intrinsic_height, delivery_id, job_id, publication_state
) VALUES (
	'{asset.question_id}', {asset.revision_number}, '{asset.asset_id}', '{asset.private_object_id}',
	decode('{asset.checksum}', 'hex'), '{asset.public_object_id}', decode('{asset.checksum}', 'hex'),
	{len(question_asset_bytes(repo_root))}, '{PNG_MEDIA_TYPE}', {asset.width}, {asset.height},
	'{asset.delivery_id}', '{asset.job_id}', 'pending'
)
ON CONFLICT (question_id, revision_number, asset_id) DO NOTHING;
DO $$
BEGIN
	IF EXISTS (
		SELECT 1
				FROM (VALUES
				{expected_accounts}
				) AS expected(account_id, product_role)
				LEFT JOIN ple_private.account AS actual
					ON actual.account_id = expected.account_id::uuid
				WHERE actual.product_role IS DISTINCT FROM expected.product_role
	) THEN
		RAISE EXCEPTION USING ERRCODE = '23514',
			MESSAGE = 'seeded Live Demo Account configuration is incompatible';
	END IF;
	IF EXISTS (
		SELECT 1
			FROM (VALUES
			{expected_questions}
			) AS expected(question_id, backend, question_title, question_description, source_object_id, source_checksum)
			LEFT JOIN ple_data.question_revision AS revision
				ON revision.question_id = expected.question_id AND revision.revision_number = 1
			LEFT JOIN ple_data.published_question_metadata AS metadata
				ON metadata.question_id = expected.question_id
			LEFT JOIN ple_private.question_revision_source_binding AS source_binding
				ON source_binding.question_id = expected.question_id AND source_binding.revision_number = 1
			WHERE revision.backend IS DISTINCT FROM expected.backend
			OR metadata.question_title IS DISTINCT FROM expected.question_title
			OR metadata.question_description IS DISTINCT FROM expected.question_description
			OR source_binding.source_object_id IS DISTINCT FROM expected.source_object_id::uuid
			OR source_binding.source_object_checksum IS DISTINCT FROM expected.source_checksum
	) THEN
		RAISE EXCEPTION USING ERRCODE = '23514',
			MESSAGE = 'seeded Live Demo Question configuration is incompatible';
	END IF;
	IF NOT EXISTS (
		SELECT 1
			FROM ple_private.question_asset_publication AS publication
			JOIN ple_private.object_record AS source_record
				ON source_record.object_id = publication.source_object_id
			JOIN ple_data.object_delivery AS delivery
				ON delivery.delivery_id = publication.delivery_id
			JOIN ple_data.question_asset_delivery AS asset_delivery
				ON asset_delivery.delivery_id = publication.delivery_id
			JOIN ple_private.job AS job ON job.job_id = publication.job_id
			WHERE publication.question_id = '{asset.question_id}'
				AND publication.revision_number = {asset.revision_number}
				AND publication.asset_id = '{asset.asset_id}'::uuid
				AND publication.source_object_id = '{asset.private_object_id}'::uuid
				AND publication.public_object_id = '{asset.public_object_id}'::uuid
				AND publication.publication_state = 'pending'
				AND source_record.sha256 = decode('{asset.checksum}', 'hex')
				AND delivery.delivery_state = 'pending'
				AND asset_delivery.asset_id = '{asset.asset_id}'::uuid
				AND job.job_kind = 'publish_public_assets'
				AND job.job_target_kind = 'public_asset_publication'
				AND job.state IN ('ready', 'leased', 'completed')
	) THEN
		RAISE EXCEPTION USING ERRCODE = '23514',
			MESSAGE = 'seeded Live Demo Question Asset publication configuration is incompatible';
	END IF;
END
$$;
COMMIT;
"""
	return sql


#============================================
def inventory_sql(repo_root: pathlib.Path) -> str:
	"""Return the one answer-free inventory projection allowed to the M4 runner."""
	require_question_asset_baseline(repo_root)
	asset = SEEDED_QUESTION_ASSET_PUBLICATION
	expected_accounts = ",\n\t".join(
		f"('{account.account_id}', '{account.product_role}')"
		for account in SEEDED_ACCOUNTS
	)
	expected_questions = ",\n\t".join(
		"(" + ", ".join((
			f"'{question.question_id}'",
			f"'{question.backend}'",
			f"'{question.question_title}'",
			f"'{question.question_description}'",
			f"'{question.source_object_id}'",
			f"'{source_checksum(repo_root, question)}'",
		)) + ")"
		for question in SEEDED_PUBLISHED_QUESTIONS
	)
	sql = f"""WITH expected_accounts(account_id, product_role) AS (
	VALUES
		{expected_accounts}
), expected_questions(
	question_id, backend, question_title, question_description, source_object_id, source_checksum
) AS (
	VALUES
		{expected_questions}
)
SELECT
	(SELECT count(*)
		FROM expected_accounts
		JOIN ple_private.account AS account
			ON account.account_id = expected_accounts.account_id::uuid
			AND account.product_role = expected_accounts.product_role),
	(SELECT count(*)
		FROM expected_questions
		JOIN ple_data.question_revision AS revision
			ON revision.question_id = expected_questions.question_id
			AND revision.revision_number = 1
			AND revision.backend = expected_questions.backend
		JOIN ple_data.published_question_metadata AS metadata
			ON metadata.question_id = expected_questions.question_id
			AND metadata.question_title = expected_questions.question_title
			AND metadata.question_description = expected_questions.question_description),
	(SELECT count(*)
		FROM expected_questions
		JOIN ple_private.question_revision_source_binding AS source_binding
			ON source_binding.question_id = expected_questions.question_id
			AND source_binding.revision_number = 1
			AND source_binding.source_object_id = expected_questions.source_object_id::uuid
			AND source_binding.source_object_checksum = expected_questions.source_checksum),
	(SELECT count(*)
		FROM expected_questions
		JOIN ple_data.question_publication_event AS publication
			ON publication.question_id = expected_questions.question_id
			AND publication.revision_number = 1),
	(SELECT count(*)
		FROM expected_questions
		JOIN ple_private.object_record AS object_record
			ON object_record.object_id = expected_questions.source_object_id::uuid
			AND encode(object_record.sha256, 'hex') = expected_questions.source_checksum),
	(SELECT count(*)
		FROM ple_private.object_record
		WHERE object_id = '{asset.private_object_id}'::uuid
			AND object_storage_area = 'private-content'
			AND object_data_class = 'question-asset'
			AND sha256 = decode('{asset.checksum}', 'hex')),
	(SELECT count(*)
		FROM ple_data.object_delivery
		WHERE delivery_id = '{asset.delivery_id}'::uuid
			AND object_id = '{asset.public_object_id}'::uuid
			AND delivery_state = 'pending'),
	(SELECT count(*)
		FROM ple_private.job
		WHERE job_id = '{asset.job_id}'::uuid
			AND job_kind = 'publish_public_assets'
			AND job_target_kind = 'public_asset_publication'
			AND state IN ('ready', 'leased', 'completed')),
	(SELECT count(*)
		FROM ple_private.question_asset_publication
		WHERE question_id = '{asset.question_id}'
			AND revision_number = {asset.revision_number}
			AND asset_id = '{asset.asset_id}'::uuid
			AND publication_state IN ('pending', 'ready'));
"""
	return sql
