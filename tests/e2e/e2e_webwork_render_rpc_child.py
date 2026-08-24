"""Assert the WebWork service contract through one seeded production-auth HTTPS stack."""

# Standard Library
import dataclasses
import http.cookiejar
import json
import os
import pathlib
import re
import ssl
import stat
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections.abc import Mapping, Sequence


SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(E2E_DIRECTORY))

# local repo modules
import e2e_live_demo_service_input


ORACLE_NAME = "webwork_render_rpc"
ORACLE_INPUT_ENVIRONMENT_NAME = "PLE_LIVE_DEMO_SERVICE_ORACLE_INPUT_FILE"
ACCOUNT_COOKIE_NAME = "__Host-ple_account_session"
COURSE_COOKIE_NAME = "__Host-ple_session"
RENDERER_EVIDENCE_CLAIM = "renderer_delivery"
RENDERER_SERVICE = "webwork-renderer"
MAXIMUM_RESPONSE_BYTES = 4 * 1024 * 1024
MAXIMUM_SEED_MANIFEST_BYTES = 16_384
REQUEST_TIMEOUT_SECONDS = 30
ADAPTER_TIMEOUT_SECONDS = 300
QUESTION_ID_PATTERN = re.compile(r"^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$")
PRIVATE_MARKERS = (
	"problemSource",
	"passwd",
	"AnSwEr",
	"hidden_input_field",
	"render_rpc",
	"render-api",
	"answerKey",
	"correctResponse",
	"gradingPayload",
	"privateGrading",
)
VISIBLE_ANSWER_NAMES = {
	"hydrophobic": {
		"benzene",
		"toluene",
		"ethylene",
		"propane",
		"butane",
		"cyclohexane",
		"hexane",
		"octane",
	},
	"hydrophilic": {
		"acetate",
		"water",
		"erythrose",
		"glucose",
		"sucrose",
		"glycerol",
		"glycine",
		"ethanol",
		"methanol",
		"ammonia",
		"sodium chloride",
		"phosphoric acid",
		"urea",
	},
}


class WebWorkOracleError(RuntimeError):
	"""A concise browser-free WebWork assertion failure."""


@dataclasses.dataclass(frozen=True)
class SeedManifest:
	"""Private host-seed identifiers needed by this assertion child."""

	course_id: str
	assignment_id: str
	enrollment_id: str
	problem_id: str
	question_id: str
	version_id: str


@dataclasses.dataclass(frozen=True)
class GatewayResponse:
	"""One bounded localhost gateway response."""

	status: int
	body: bytes
	content_type: str


@dataclasses.dataclass(frozen=True)
class RendererEvidence:
	"""Typed counts from the redacted renderer-delivery evidence channel."""

	renderer_calls: int
	cache_hits: int
	raw: bytes


class RejectRedirects(urllib.request.HTTPRedirectHandler):
	"""Keep the authenticated localhost client on its exact configured origin."""

	#============================================
	def redirect_request(
		self,
		request: object,
		file_pointer: object,
		code: int,
		message: str,
		headers: object,
		new_url: str,
	) -> None:
		"""Reject every redirect instead of forwarding cookies to another URL."""
		return None


class GatewayClient:
	"""Request-local HTTPS client and cookie jar for the fixed localhost gateway."""

	def __init__(self, base_url: str) -> None:
		"""Create one exact-origin opener without changing process-wide TLS policy."""
		self.base_url = base_url
		self.origin = base_url.removesuffix("/")
		self.cookies = http.cookiejar.CookieJar()
		# The disposable Caddy CA is private to this run. The accepted architecture permits
		# this unverified context only in this exact localhost helper (ASVS 12.3.4 exception).
		context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
		context.check_hostname = False
		context.verify_mode = ssl.CERT_NONE
		self.opener = urllib.request.build_opener(
			urllib.request.ProxyHandler({}),
			RejectRedirects(),
			urllib.request.HTTPSHandler(context=context),
			urllib.request.HTTPCookieProcessor(self.cookies),
		)

	#============================================
	def _url(self, path: str) -> str:
		"""Form one same-origin URL from a validated absolute-path reference."""
		# ASVS 2.2.1 and 13.2.4: response identifiers cannot select a host or scheme.
		parsed = urllib.parse.urlsplit(path)
		if (
			not path.startswith("/")
			or parsed.scheme != ""
			or parsed.netloc != ""
			or parsed.fragment != ""
			or ".." in pathlib.PurePosixPath(parsed.path).parts
			or "\r" in path
			or "\n" in path
		):
			raise WebWorkOracleError("gateway request path is invalid")
		result = self.origin + path
		return result

	#============================================
	def request(
		self,
		method: str,
		path: str,
		body: object | None = None,
		headers: Mapping[str, str] | None = None,
	) -> GatewayResponse:
		"""Send one bounded GET or POST and return even an expected HTTP error status."""
		if method not in ("GET", "POST"):
			raise WebWorkOracleError("gateway request method is invalid")
		request_headers = dict(headers) if headers is not None else {}
		origin_headers = [
			value
			for key, value in request_headers.items()
			if key.casefold() == "origin"
		]
		if len(origin_headers) > 1:
			raise WebWorkOracleError("gateway request has duplicate Origin headers")
		if origin_headers and origin_headers[0] != self.origin:
			raise WebWorkOracleError("gateway request Origin does not match its origin")
		request_headers = {
			key: value
			for key, value in request_headers.items()
			if key.casefold() != "origin"
		}
		if method == "POST":
			request_headers["Origin"] = self.origin
		request_data = None
		if body is not None:
			request_data = json.dumps(
				body, separators=(",", ":"), ensure_ascii=True
			).encode("ascii")
			request_headers["content-type"] = "application/json"
		request = urllib.request.Request(
			self._url(path), data=request_data, headers=request_headers, method=method
		)
		try:
			response = self.opener.open(request, timeout=REQUEST_TIMEOUT_SECONDS)
		except urllib.error.HTTPError as error:
			response = error
		with response:
			contents = response.read(MAXIMUM_RESPONSE_BYTES + 1)
			status = response.status
			content_type = response.headers.get_content_type()
		if len(contents) > MAXIMUM_RESPONSE_BYTES:
			raise WebWorkOracleError("gateway response exceeds its bounded size")
		if content_type != "application/json":
			raise WebWorkOracleError("gateway response is not JSON")
		result = GatewayResponse(status, contents, content_type)
		return result

	#============================================
	def require_secure_cookie(self, name: str) -> None:
		"""Require one live Secure host cookie without exposing its value."""
		selected = tuple(cookie for cookie in self.cookies if cookie.name == name)
		if (
			len(selected) != 1
			or not selected[0].secure
			or selected[0].path != "/"
			or selected[0].is_expired()
		):
			raise WebWorkOracleError("production authentication cookie is missing or insecure")


class BoundedAdapter:
	"""Invoke only the profile-authorized renderer evidence and outage actions."""

	def __init__(self, manifest_path: pathlib.Path) -> None:
		"""Retain only the owner-validated private manifest locator."""
		self.manifest_path = manifest_path

	#============================================
	def _argv(self, action: str) -> list[str]:
		"""Resolve one closed action without project or generic Compose arguments."""
		base = [
			sys.executable,
			"-m",
			"local_stack_control._consumer_cli",
			action,
			"--manifest",
			str(self.manifest_path),
		]
		if action == "read-evidence-logs":
			base.extend(("--claim", RENDERER_EVIDENCE_CLAIM))
		elif action == "restart":
			base.extend(
				("--service", RENDERER_SERVICE, "--timeout-seconds", "240")
			)
		elif action != "stop-outage-service":
			raise WebWorkOracleError("renderer adapter action is invalid")
		return base

	#============================================
	def _run(self, action: str) -> subprocess.CompletedProcess[str]:
		"""Await one bounded adapter action without a shell or ambient control input."""
		result = subprocess.run(
			self._argv(action),
			cwd=SCRIPT_REPOSITORY_ROOT,
			env=dict(os.environ),
			capture_output=True,
			text=True,
			timeout=ADAPTER_TIMEOUT_SECONDS,
			check=False,
		)
		if result.returncode != 0:
			raise WebWorkOracleError("bounded renderer adapter action did not complete")
		return result

	#============================================
	def evidence(self) -> RendererEvidence:
		"""Read one redacted typed renderer-delivery evidence snapshot."""
		result = self._run("read-evidence-logs")
		raw = (result.stdout + result.stderr).encode("utf-8")
		text = raw.decode("utf-8", errors="replace")
		calls = len(re.findall(r'ple[.]webwork[.]cache.*event="renderer_call"', text))
		hits = len(re.findall(r'ple[.]webwork[.]cache.*event="cache_hit"', text))
		evidence = RendererEvidence(calls, hits, raw)
		return evidence

	#============================================
	def stop_renderer(self) -> None:
		"""Stop only the profile-declared renderer outage service."""
		self._run("stop-outage-service")

	#============================================
	def restart_renderer(self) -> None:
		"""Restart only the same stateless renderer without a build."""
		self._run("restart")


#============================================
def _checked_private_bytes(path: pathlib.Path, maximum: int) -> bytes:
	"""Read one current-user mode-0600 regular file through its checked descriptor."""
	try:
		path_metadata = path.lstat()
		file_descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
	except OSError as error:
		raise WebWorkOracleError("private WebWork seed manifest is unsafe") from error
	try:
		metadata = os.fstat(file_descriptor)
		if (
			not path.is_absolute()
			or not stat.S_ISREG(path_metadata.st_mode)
			or not stat.S_ISREG(metadata.st_mode)
			or path_metadata.st_uid != os.getuid()
			or metadata.st_uid != os.getuid()
			or (path_metadata.st_mode & 0o777) != 0o600
			or (metadata.st_mode & 0o777) != 0o600
			or (path_metadata.st_dev, path_metadata.st_ino)
			!= (metadata.st_dev, metadata.st_ino)
			or metadata.st_size < 1
			or metadata.st_size > maximum
		):
			raise WebWorkOracleError("private WebWork seed manifest is unsafe")
		with os.fdopen(file_descriptor, "rb") as source:
			contents = source.read(maximum + 1)
			file_descriptor = -1
	finally:
		if file_descriptor >= 0:
			os.close(file_descriptor)
	if len(contents) > maximum:
		raise WebWorkOracleError("private WebWork seed manifest is unsafe")
	return contents


#============================================
def require_uuid(value: object, label: str) -> str:
	"""Require one canonical lowercase UUID string."""
	if not isinstance(value, str):
		raise WebWorkOracleError(f"{label} is invalid")
	try:
		parsed = uuid.UUID(value)
	except ValueError as error:
		raise WebWorkOracleError(f"{label} is invalid") from error
	if str(parsed) != value:
		raise WebWorkOracleError(f"{label} is invalid")
	return value


#============================================
def read_seed_manifest(path: pathlib.Path) -> SeedManifest:
	"""Strictly decode the exact private host-seed identifier receipt."""
	contents = _checked_private_bytes(path, MAXIMUM_SEED_MANIFEST_BYTES)
	try:
		value = json.loads(contents.decode("ascii"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise WebWorkOracleError("private WebWork seed manifest is invalid") from error
	expected = {
		"assignmentId",
		"courseId",
		"enrollmentId",
		"problemId",
		"questionId",
		"versionId",
	}
	if not isinstance(value, dict) or set(value) != expected:
		raise WebWorkOracleError("private WebWork seed manifest is invalid")
	question_id = value["questionId"]
	if not isinstance(question_id, str) or QUESTION_ID_PATTERN.fullmatch(question_id) is None:
		raise WebWorkOracleError("seed manifest questionId is invalid")
	result = SeedManifest(
		require_uuid(value["courseId"], "seed manifest courseId"),
		require_uuid(value["assignmentId"], "seed manifest assignmentId"),
		require_uuid(value["enrollmentId"], "seed manifest enrollmentId"),
		require_uuid(value["problemId"], "seed manifest problemId"),
		question_id,
		require_uuid(value["versionId"], "seed manifest versionId"),
	)
	return result


#============================================
def decode_json(response: GatewayResponse, label: str) -> object:
	"""Decode one bounded UTF-8 JSON response without object construction hooks."""
	try:
		value = json.loads(response.body.decode("utf-8"))
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise WebWorkOracleError(f"{label} returned invalid JSON") from error
	return value


#============================================
def require_status(response: GatewayResponse, expected: int, label: str) -> None:
	"""Require one exact HTTP status without reflecting response contents."""
	if response.status != expected:
		raise WebWorkOracleError(f"{label} returned HTTP {response.status}")


#============================================
def require_closed_record(value: object, fields: set[str], label: str) -> dict[str, object]:
	"""Require one JSON object with exactly its allowlisted fields."""
	if not isinstance(value, dict) or set(value) != fields:
		raise WebWorkOracleError(f"{label} has an invalid shape")
	return value


#============================================
def authenticate_mary(client: GatewayClient, course_id: str) -> list[bytes]:
	"""Perform the exact seeded-account to course-session production sequence."""
	selector = client.request(
		"POST",
		"/api/auth/live-demo/accounts",
		{"persona": "maryStudent"},
	)
	require_status(selector, 200, "Mary seeded-account selection")
	selected = require_closed_record(
		decode_json(selector, "Mary seeded-account selection"),
		{"authenticated"},
		"Mary seeded-account selection",
	)
	if selected["authenticated"] is not True:
		raise WebWorkOracleError("Mary seeded-account selection was not authenticated")
	client.require_secure_cookie(ACCOUNT_COOKIE_NAME)

	courses_response = client.request("GET", "/api/auth/account/courses")
	require_status(courses_response, 200, "Mary account-course list")
	page = require_closed_record(
		decode_json(courses_response, "Mary account-course list"),
		{"courses", "nextCursor"},
		"Mary account-course list",
	)
	if page["nextCursor"] is not None or not isinstance(page["courses"], list):
		raise WebWorkOracleError("Mary account-course list is incomplete")
	matches: list[dict[str, object]] = []
	for index, item in enumerate(page["courses"]):
		course = require_closed_record(
			item,
			{"courseId", "courseReference", "title", "role"},
			f"Mary account-course list item {index}",
		)
		require_uuid(course["courseId"], "Mary account courseId")
		if course["courseId"] == course_id:
			matches.append(course)
	if len(matches) != 1 or matches[0]["role"] != "student":
		raise WebWorkOracleError("Mary cannot select the exact seeded WebWork course")

	course_session = client.request(
		"POST",
		"/api/auth/account/course-session",
		{"courseId": course_id},
	)
	require_status(course_session, 200, "Mary course-session selection")
	session = require_closed_record(
		decode_json(course_session, "Mary course-session selection"),
		{"authenticated", "courseId", "role"},
		"Mary course-session selection",
	)
	if (
		session["authenticated"] is not True
		or session["courseId"] != course_id
		or session["role"] != "student"
	):
		raise WebWorkOracleError("Mary course session does not match the seeded course")
	client.require_secure_cookie(ACCOUNT_COOKIE_NAME)
	client.require_secure_cookie(COURSE_COOKIE_NAME)
	return [selector.body, courses_response.body, course_session.body]


#============================================
def require_uuid_field(value: object, field: str, label: str) -> str:
	"""Read one required UUID field from a JSON object."""
	if not isinstance(value, dict) or field not in value:
		raise WebWorkOracleError(f"{label} omitted {field}")
	return require_uuid(value[field], f"{label} {field}")


#============================================
def create_attempt(
	client: GatewayClient,
	course_id: str,
	assignment_id: str,
) -> tuple[str, str, list[bytes]]:
	"""Create one run and require its exact single WebWork attempt."""
	path = f"/api/courses/{course_id}/assignments/{assignment_id}/runs"
	run_response = client.request("POST", path)
	require_status(run_response, 201, "starting a PLE WebWork run")
	run_value = decode_json(run_response, "starting a PLE WebWork run")
	run_id = require_uuid_field(run_value, "id", "PLE WebWork run")
	attempts_response = client.request("GET", f"/api/runs/{run_id}/attempts")
	require_status(attempts_response, 200, "listing PLE WebWork attempts")
	attempts_value = decode_json(attempts_response, "listing PLE WebWork attempts")
	if not isinstance(attempts_value, dict) or "items" not in attempts_value:
		raise WebWorkOracleError("PLE WebWork attempt list is invalid")
	items = attempts_value["items"]
	if not isinstance(items, list) or len(items) != 1:
		raise WebWorkOracleError("expected exactly one WebWork attempt")
	attempt_id = require_uuid_field(items[0], "id", "PLE WebWork attempt")
	return run_id, attempt_id, [run_response.body, attempts_response.body]


#============================================
def question_answer(value: object, classification: str) -> str:
	"""Choose an answer only from learner-visible PLE projection text."""
	if classification not in VISIBLE_ANSWER_NAMES:
		raise WebWorkOracleError("visible-choice classification is invalid")
	encoded = json.dumps(value, separators=(",", ":"), ensure_ascii=True)
	assert_no_private_material([encoded.encode("ascii")])
	if not isinstance(value, dict) or not isinstance(value.get("response"), dict):
		raise WebWorkOracleError("PLE question omitted its response projection")
	choices = value["response"].get("choices")
	if not isinstance(choices, list):
		raise WebWorkOracleError("PLE question did not expose a multiple-choice projection")
	matches: list[str] = []
	for choice in choices:
		if not isinstance(choice, dict):
			continue
		blocks = choice.get("body", [])
		if not isinstance(blocks, list):
			blocks = []
		visible_parts = [str(choice.get(key, "")) for key in ("label", "text", "content")]
		visible_parts.extend(
			str(block.get("markdown", ""))
			for block in blocks
			if isinstance(block, dict)
		)
		visible = " ".join(visible_parts).casefold()
		choice_id = choice.get("id")
		if (
			any(name in visible for name in VISIBLE_ANSWER_NAMES[classification])
			and isinstance(choice_id, str)
			and choice_id != ""
		):
			matches.append(choice_id)
	if classification == "hydrophobic" and len(matches) != 1:
		raise WebWorkOracleError("expected one visible hydrophobic choice")
	if classification == "hydrophilic" and len(matches) < 1:
		raise WebWorkOracleError("expected one visible hydrophilic distractor")
	return matches[0]


#============================================
def number_field(value: object, label: str) -> float:
	"""Require a finite score field represented by a JSON number."""
	if isinstance(value, bool) or not isinstance(value, (int, float)):
		raise WebWorkOracleError(f"{label} is invalid")
	result = float(value)
	return result


#============================================
def assert_completed_receipt(value: object, expected_score: float) -> None:
	"""Require authorized grading feedback without answer-bearing fields."""
	if not isinstance(value, dict) or value.get("accepted") is not True:
		raise WebWorkOracleError("submission receipt was not accepted")
	attempt = value.get("attempt")
	feedback = value.get("feedback")
	if not isinstance(attempt, dict) or not isinstance(feedback, dict):
		raise WebWorkOracleError("completed WebWork receipt omitted its authorized result")
	result = attempt.get("result")
	if not isinstance(result, dict):
		raise WebWorkOracleError("completed WebWork receipt omitted its authorized result")
	expected_correct = expected_score == 1.0
	if (
		result.get("correct") is not expected_correct
		or number_field(result.get("pointsEarned"), "attempt pointsEarned") != expected_score
		or number_field(result.get("pointsPossible"), "attempt pointsPossible") != 1.0
	):
		raise WebWorkOracleError("completed WebWork receipt carried the wrong result")
	if set(feedback) != {"correctness", "pointsEarned", "pointsPossible"}:
		raise WebWorkOracleError("completed WebWork feedback exceeded its exact allowlist")
	if (
		feedback["correctness"] is not expected_correct
		or number_field(feedback["pointsEarned"], "feedback pointsEarned") != expected_score
		or number_field(feedback["pointsPossible"], "feedback pointsPossible") != 1.0
	):
		raise WebWorkOracleError("completed WebWork feedback carried the wrong result")
	assert_no_private_material(
		[json.dumps(value, separators=(",", ":"), ensure_ascii=True).encode("ascii")]
	)


#============================================
def assert_summary_score(value: object, expected_score: float) -> None:
	"""Require one completed run and its exact latest-score projection."""
	if not isinstance(value, dict):
		raise WebWorkOracleError("completed-run summary is invalid")
	run = value.get("run")
	summary = value.get("summary")
	if not isinstance(run, dict) or not isinstance(summary, dict):
		raise WebWorkOracleError("completed-run summary is invalid")
	if (
		number_field(run.get("score"), "run score") != expected_score
		or number_field(summary.get("latestScore"), "latest score") != expected_score
		or run.get("completedAt") is None
	):
		raise WebWorkOracleError("completed-run summary carried the wrong score")


#============================================
def assert_no_private_material(blobs: Sequence[bytes]) -> None:
	"""Reject upstream-private, answer-bearing, or unredacted credential material."""
	for blob in blobs:
		for marker in PRIVATE_MARKERS:
			if marker.encode("ascii") in blob:
				raise WebWorkOracleError("private renderer material leaked into public evidence")
		text = blob.decode("utf-8", errors="replace")
		if re.search(r"postgres://(?!\[redacted\])[^@\s]+@", text) is not None:
			raise WebWorkOracleError("renderer evidence contains an unredacted database credential")


#============================================
def submit_answer(
	client: GatewayClient,
	course_id: str,
	assignment_id: str,
	attempt_id: str,
	choice_id: str,
	idempotency_key: str,
) -> GatewayResponse:
	"""Submit one learner-visible choice through a fixed idempotency contract."""
	response = client.request(
		"POST",
		(
			f"/api/courses/{course_id}/assignments/{assignment_id}"
			f"/attempts/{attempt_id}/submissions"
		),
		{"response": {"kind": "multipleChoice", "selected": [choice_id]}},
		{"idempotency-key": idempotency_key},
	)
	require_status(response, 200, "PLE WebWork submission")
	return response


#============================================
def run_oracle(value: e2e_live_demo_service_input.LiveDemoServiceOracleInputV1) -> None:
	"""Run the retained render, durability, grade, outage, and secrecy assertions."""
	manifest = read_seed_manifest(value.seed_manifest_path)
	client = GatewayClient(value.base_url)
	adapter = BoundedAdapter(value.manifest_path)
	public_blobs = authenticate_mary(client, manifest.course_id)

	before_run = adapter.evidence()
	run_one_id, attempt_one, run_one_blobs = create_attempt(
		client, manifest.course_id, manifest.assignment_id
	)
	public_blobs.extend(run_one_blobs)
	after_run = adapter.evidence()
	question_path_one = (
		f"/api/courses/{manifest.course_id}/assignments/{manifest.assignment_id}"
		f"/attempts/{attempt_one}/question"
	)
	question_one = client.request("GET", question_path_one)
	require_status(question_one, 200, "first PLE WebWork question request")
	after_first = adapter.evidence()
	question_two = client.request("GET", question_path_one)
	require_status(question_two, 200, "second PLE WebWork question request")
	after_second = adapter.evidence()
	if question_one.body != question_two.body:
		raise WebWorkOracleError("persisted same-attempt WebWork projection changed")
	if after_run.renderer_calls not in (
		before_run.renderer_calls,
		before_run.renderer_calls + 1,
	):
		raise WebWorkOracleError("run creation emitted an invalid renderer_call count")
	if (
		after_first.renderer_calls != after_run.renderer_calls
		or after_first.cache_hits != after_run.cache_hits
		or after_second.renderer_calls != after_first.renderer_calls
		or after_second.cache_hits != after_first.cache_hits
	):
		raise WebWorkOracleError("persisted same-attempt replay emitted adapter evidence")

	question_one_value = decode_json(question_one, "first PLE WebWork question")
	correct_choice = question_answer(question_one_value, "hydrophobic")
	receipt_one = submit_answer(
		client,
		manifest.course_id,
		manifest.assignment_id,
		attempt_one,
		correct_choice,
		"ple-webwork-correct-1",
	)
	receipt_one_value = decode_json(receipt_one, "correct PLE WebWork submission")
	assert_completed_receipt(receipt_one_value, 1.0)
	receipt_one_replay = submit_answer(
		client,
		manifest.course_id,
		manifest.assignment_id,
		attempt_one,
		correct_choice,
		"ple-webwork-correct-1",
	)
	if receipt_one.body != receipt_one_replay.body:
		raise WebWorkOracleError("idempotent WebWork submission replay changed its receipt")
	summary_one = client.request("GET", f"/api/runs/{run_one_id}/summary?pageSize=1")
	require_status(summary_one, 200, "correct WebWork run summary")
	assert_summary_score(decode_json(summary_one, "correct WebWork run summary"), 1.0)

	before_second_run = adapter.evidence()
	run_two_id, attempt_two, run_two_blobs = create_attempt(
		client, manifest.course_id, manifest.assignment_id
	)
	public_blobs.extend(run_two_blobs)
	after_second_run = adapter.evidence()
	if after_second_run.renderer_calls != before_second_run.renderer_calls + 1:
		raise WebWorkOracleError("continued-practice issuance lacked one renderer_call event")
	hit_delta = after_second_run.cache_hits - before_second_run.cache_hits
	if hit_delta not in (0, 1):
		raise WebWorkOracleError("continued-practice issuance emitted an invalid cache_hit delta")
	question_path_two = (
		f"/api/courses/{manifest.course_id}/assignments/{manifest.assignment_id}"
		f"/attempts/{attempt_two}/question"
	)
	question_three = client.request("GET", question_path_two)
	require_status(question_three, 200, "continued-practice WebWork question request")
	incorrect_choice = question_answer(
		decode_json(question_three, "continued-practice WebWork question"), "hydrophilic"
	)
	receipt_two = submit_answer(
		client,
		manifest.course_id,
		manifest.assignment_id,
		attempt_two,
		incorrect_choice,
		"ple-webwork-incorrect-1",
	)
	assert_completed_receipt(
		decode_json(receipt_two, "incorrect PLE WebWork submission"), 0.0
	)
	summary_two = client.request("GET", f"/api/runs/{run_two_id}/summary?pageSize=1")
	require_status(summary_two, 200, "incorrect WebWork run summary")
	assert_summary_score(decode_json(summary_two, "incorrect WebWork run summary"), 0.0)

	adapter.stop_renderer()
	try:
		health = client.request("GET", "/health")
		require_status(health, 200, "native gateway health during renderer outage")
		outage = client.request(
			"POST",
			f"/api/courses/{manifest.course_id}/assignments/{manifest.assignment_id}/runs",
		)
		require_status(outage, 503, "renderer-outage WebWork issuance")
	finally:
		adapter.restart_renderer()

	public_blobs.extend(
		[
			question_one.body,
			question_two.body,
			receipt_one.body,
			receipt_one_replay.body,
			summary_one.body,
			question_three.body,
			receipt_two.body,
			summary_two.body,
			health.body,
			outage.body,
			before_run.raw,
			after_run.raw,
			after_first.raw,
			after_second.raw,
			before_second_run.raw,
			after_second_run.raw,
		]
	)
	assert_no_private_material(public_blobs)


#============================================
def main() -> None:
	"""Read the strict owner input and execute the WebWork assertion child."""
	input_name = os.environ.get(ORACLE_INPUT_ENVIRONMENT_NAME)
	if input_name is None or input_name == "":
		raise WebWorkOracleError("live-demo service-oracle input is not configured")
	value = e2e_live_demo_service_input.read_private_input(
		pathlib.Path(input_name), ORACLE_NAME
	)
	run_oracle(value)
	print(
		"PASS: PLE WebWork service acceptance proved production Mary course authentication, "
		"safe projection, persisted replay, renderer evidence, full/zero scoring, outage "
		"isolation and restart, redaction, answer secrecy, and browser-free execution."
	)


#============================================
def command_line_main() -> None:
	"""Present bounded failure text without private path, cookie, or seed material."""
	# ASVS 16.5.1: unexpected failures never emit a traceback or private request state.
	try:
		main()
	except Exception as error:
		message = str(error) if isinstance(error, WebWorkOracleError) else "WebWork oracle failed"
		print("FAIL: " + message, file=sys.stderr)
		raise SystemExit(1) from None


if __name__ == "__main__":
	command_line_main()
