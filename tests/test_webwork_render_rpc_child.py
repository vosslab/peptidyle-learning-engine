"""Offline contracts for the browser-free production-auth WebWork assertion child."""

# Standard Library
import json
import pathlib
import sys

# PIP3 modules
import pytest

# local repo modules
import file_utils


E2E_DIRECTORY = pathlib.Path(file_utils.get_repo_root()) / "tests" / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))

import e2e_webwork_render_rpc_child as webwork_child


COURSE_ID = "00000000-0000-4000-8000-000000000201"
ASSIGNMENT_ID = "00000000-0000-4000-8000-000000000202"


class FakeResponseHeaders:
	"""Expose the JSON content type expected by the bounded gateway client."""

	def get_content_type(self) -> str:
		"""Return the only response type accepted by the child."""
		return "application/json"


class FakeHTTPResponse:
	"""Provide one small context-managed response for request-construction tests."""

	status = 200
	headers = FakeResponseHeaders()

	def __enter__(self) -> "FakeHTTPResponse":
		"""Enter the response context."""
		return self

	def __exit__(self, exception_type: object, exception: object, traceback: object) -> None:
		"""Close the response context without suppressing exceptions."""
		return None

	def read(self, maximum: int) -> bytes:
		"""Return a bounded JSON response body."""
		return b"{}"


class RecordingOpener:
	"""Record urllib requests without opening a socket."""

	def __init__(self) -> None:
		"""Create an empty request inventory."""
		self.requests: list[object] = []

	def open(self, request: object, timeout: float) -> FakeHTTPResponse:
		"""Record one request and return an accepted JSON response."""
		self.requests.append(request)
		return FakeHTTPResponse()


class FakeGatewayClient:
	"""Record the Mary production-auth sequence without making a network request."""

	def __init__(self) -> None:
		"""Create one deterministic response queue and call inventory."""
		self.origin = "https://localhost:55001"
		self.calls: list[tuple[str, str, object | None, dict[str, str] | None]] = []
		self.cookie_checks: list[str] = []

	#============================================
	def request(
		self,
		method: str,
		path: str,
		body: object | None = None,
		headers: dict[str, str] | None = None,
	) -> webwork_child.GatewayResponse:
		"""Return the exact response for one expected production-auth step."""
		self.calls.append((method, path, body, headers))
		if path == "/api/auth/live-demo/accounts":
			value: object = {"authenticated": True}
		elif path == "/api/auth/account/courses":
			value = {
				"courses": [
					{
						"courseId": COURSE_ID,
						"courseReference": "C-ABC-1234",
						"title": "PLE WebWork pilot E2E course",
						"role": "student",
					}
				],
				"nextCursor": None,
			}
		elif path == "/api/auth/account/course-session":
			value = {"authenticated": True, "courseId": COURSE_ID, "role": "student"}
		else:
			raise AssertionError("unexpected fake gateway path")
		encoded = json.dumps(value, separators=(",", ":")).encode("ascii")
		return webwork_child.GatewayResponse(200, encoded, "application/json")

	#============================================
	def require_secure_cookie(self, name: str) -> None:
		"""Record the cookie name required at each sequence boundary."""
		self.cookie_checks.append(name)


#============================================
def seed_value() -> dict[str, str]:
	"""Return one exact answer-free host seed manifest."""
	result = {
		"assignmentId": ASSIGNMENT_ID,
		"courseId": COURSE_ID,
		"enrollmentId": "00000000-0000-4000-8000-000000000203",
		"problemId": "00000000-0000-4000-8000-000000000204",
		"questionId": "ABC-1234",
		"versionId": "00000000-0000-4000-8000-000000000205",
	}
	return result


#============================================
def write_seed(path: pathlib.Path, value: object, mode: int = 0o600) -> None:
	"""Write one inline private seed-manifest input."""
	path.write_text(json.dumps(value, separators=(",", ":")), encoding="ascii")
	path.chmod(mode)


#============================================
def test_seed_manifest_requires_exact_private_course_and_assignment_ids(
	tmp_path: pathlib.Path,
) -> None:
	"""The child selects product state only from the exact mode-0600 host receipt."""
	path = tmp_path / "seed.json"
	write_seed(path, seed_value())
	manifest = webwork_child.read_seed_manifest(path)
	assert (manifest.course_id, manifest.assignment_id) == (COURSE_ID, ASSIGNMENT_ID)


#============================================
def test_seed_manifest_rejects_extension_fields_and_insecure_mode(
	tmp_path: pathlib.Path,
) -> None:
	"""Extra authority and group-readable private identifiers both fail closed."""
	path = tmp_path / "seed.json"
	write_seed(path, {**seed_value(), "project": "ple-live-demo-browser"})
	with pytest.raises(webwork_child.WebWorkOracleError, match="manifest is invalid"):
		webwork_child.read_seed_manifest(path)
	path.unlink()
	write_seed(path, seed_value(), 0o640)
	with pytest.raises(webwork_child.WebWorkOracleError, match="manifest is unsafe"):
		webwork_child.read_seed_manifest(path)


#============================================
def test_mary_authentication_uses_exact_origin_course_and_secure_cookie_sequence() -> None:
	"""The child cannot treat account selection as a tenant course session."""
	client = FakeGatewayClient()
	webwork_child.authenticate_mary(client, COURSE_ID)
	assert client.calls == [
		(
			"POST",
			"/api/auth/live-demo/accounts",
			{"persona": "maryStudent"},
			None,
		),
		("GET", "/api/auth/account/courses", None, None),
		(
			"POST",
			"/api/auth/account/course-session",
			{"courseId": COURSE_ID},
			None,
		),
	]
	assert client.cookie_checks == [
		webwork_child.ACCOUNT_COOKIE_NAME,
		webwork_child.ACCOUNT_COOKIE_NAME,
		webwork_child.COURSE_COOKIE_NAME,
	]


#============================================
def test_mary_authentication_rejects_a_different_seeded_course() -> None:
	"""A course-list membership cannot substitute for the host-seeded course identity."""
	client = FakeGatewayClient()
	with pytest.raises(webwork_child.WebWorkOracleError, match="exact seeded WebWork course"):
		webwork_child.authenticate_mary(
			client, "00000000-0000-4000-8000-000000000299"
		)


#============================================
def test_bounded_adapter_has_only_evidence_and_exact_renderer_outage_actions(
	tmp_path: pathlib.Path,
) -> None:
	"""The child cannot request launch, cleanup, project, or generic Compose authority."""
	manifest = tmp_path / "disposable.manifest"
	adapter = webwork_child.BoundedAdapter(manifest)
	assert adapter._argv("read-evidence-logs")[-2:] == [
		"--claim",
		"renderer_delivery",
	]
	assert adapter._argv("restart")[-4:] == [
		"--service",
		"webwork-renderer",
		"--timeout-seconds",
		"240",
	]
	assert adapter._argv("stop-outage-service")[-2:] == ["--manifest", str(manifest)]
	with pytest.raises(webwork_child.WebWorkOracleError, match="action is invalid"):
		adapter._argv("launch")


#============================================
def test_localhost_client_rejects_absolute_and_traversing_request_paths() -> None:
	"""Authenticated cookies stay on the strict localhost HTTPS origin."""
	client = webwork_child.GatewayClient("https://localhost:55001/")
	assert client._url("/health") == "https://localhost:55001/health"
	for path in ("https://example.test/", "/api/../private", "//example.test/path"):
		with pytest.raises(webwork_child.WebWorkOracleError, match="path is invalid"):
			client._url(path)


#============================================
def test_gateway_client_adds_origin_to_run_submission_and_outage_posts() -> None:
	"""Every mutating request carries exactly the validated localhost Origin."""
	client = webwork_child.GatewayClient("https://localhost:55001/")
	opener = RecordingOpener()
	client.opener = opener
	for path, body in (
		(f"/api/courses/{COURSE_ID}/assignments/{ASSIGNMENT_ID}/runs", None),
		(
			f"/api/courses/{COURSE_ID}/assignments/{ASSIGNMENT_ID}"
			"/attempts/attempt/submissions",
			{"response": {"kind": "multipleChoice", "selected": ["choice"]}},
		),
		("/api/runs/outage", {"assignmentId": ASSIGNMENT_ID}),
	):
		client.request("POST", path, body)
	origins = [
		[value for key, value in request.header_items() if key.casefold() == "origin"]
		for request in opener.requests
	]
	assert origins == [[client.origin], [client.origin], [client.origin]]


#============================================
def test_gateway_client_keeps_gets_free_of_origin_header() -> None:
	"""GET requests do not carry the mutation-only Origin header."""
	client = webwork_child.GatewayClient("https://localhost:55001/")
	opener = RecordingOpener()
	client.opener = opener
	client.request("GET", "/api/runs/run/attempts", headers={"Origin": client.origin})
	request = opener.requests[0]
	assert [
		value for key, value in request.header_items() if key.casefold() == "origin"
	] == []


#============================================
def test_gateway_client_accepts_one_exact_caller_origin() -> None:
	"""A caller may repeat the exact origin while the wire header stays singular."""
	client = webwork_child.GatewayClient("https://localhost:55001/")
	opener = RecordingOpener()
	client.opener = opener
	client.request(
		"POST",
		f"/api/courses/{COURSE_ID}/assignments/{ASSIGNMENT_ID}/runs",
		headers={"origin": client.origin},
	)
	request = opener.requests[0]
	assert [
		value for key, value in request.header_items() if key.casefold() == "origin"
	] == [client.origin]


#============================================
def test_gateway_client_rejects_mismatched_and_duplicate_origin_headers() -> None:
	"""Hostile or ambiguous caller Origin input fails before any network request."""
	client = webwork_child.GatewayClient("https://localhost:55001/")
	opener = RecordingOpener()
	client.opener = opener
	with pytest.raises(webwork_child.WebWorkOracleError, match="does not match"):
		client.request(
			"POST",
			f"/api/courses/{COURSE_ID}/assignments/{ASSIGNMENT_ID}/runs",
			headers={"Origin": "https://example.test"},
		)
	with pytest.raises(webwork_child.WebWorkOracleError, match="duplicate"):
		client.request(
			"POST",
			f"/api/courses/{COURSE_ID}/assignments/{ASSIGNMENT_ID}/runs",
			headers={"Origin": client.origin, "origin": client.origin},
		)
	assert opener.requests == []


#============================================
def test_visible_answer_and_receipt_checks_preserve_answer_secrecy() -> None:
	"""Only visible choice text selects an answer and feedback exposes no key."""
	question = {
		"response": {
			"choices": [
				{"id": "visible-correct", "body": [{"markdown": "Benzene"}]},
				{"id": "visible-wrong", "body": [{"markdown": "Water"}]},
			]
		}
	}
	assert webwork_child.question_answer(question, "hydrophobic") == "visible-correct"
	assert webwork_child.question_answer(question, "hydrophilic") == "visible-wrong"
	receipt = {
		"accepted": True,
		"attempt": {
			"result": {"correct": True, "pointsEarned": 1.0, "pointsPossible": 1.0}
		},
		"feedback": {"correctness": True, "pointsEarned": 1.0, "pointsPossible": 1.0},
	}
	webwork_child.assert_completed_receipt(receipt, 1.0)
	with pytest.raises(webwork_child.WebWorkOracleError, match="private renderer material"):
		webwork_child.assert_completed_receipt({**receipt, "answerKey": "visible-correct"}, 1.0)
