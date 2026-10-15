"""Small cookie-authenticated client for ordinary PLE authoring routes."""

# Standard Library
import re
import ssl
import json
import uuid
import http.cookies
import urllib.error
import urllib.parse
import urllib.request


class PleApi:
	"""Keep the Instructor session in memory and use existing HTTP preconditions."""

	def __init__(self, api_url: str, cookie: str | None, ca_file: str | None = None) -> None:
		url = urllib.parse.urlsplit(api_url)
		# ASVS 12.2.1: credentials use TLS and redirects never carry the session.
		if url.scheme != "https" or not url.netloc or url.path not in ("", "/"):
			raise ValueError("API URL must be a root HTTPS origin")
		if url.username or url.password or url.query or url.fragment:
			raise ValueError("API URL must be a root HTTPS origin")
		if cookie is not None and (not cookie.startswith("__Host-ple_session=")
			or any(c in cookie for c in "\r\n;")):
			raise ValueError("Supply the __Host-ple_session cookie")
		self.base = api_url.rstrip("/")
		self.cookies = http.cookies.SimpleCookie()
		if cookie is not None:
			self.cookies.load(cookie)
		context = ssl.create_default_context(cafile=ca_file)
		self.opener = urllib.request.build_opener(
			urllib.request.HTTPSHandler(context=context), NoRedirect())

	def request(self, method: str, path: str, payload: dict | None = None,
		body: bytes | None = None, headers: dict | None = None) -> tuple[dict, dict]:
		"""Send one operation, without automatic retries or response-body logging."""
		cookie = "; ".join(f"{name}={value.value}" for name, value in self.cookies.items())
		request_headers = {"Cookie": cookie, "Origin": self.base}
		if headers is not None:
			request_headers.update(headers)
		if payload is not None:
			body = json.dumps(payload).encode("utf-8")
			request_headers["Content-Type"] = "application/json"
		request = urllib.request.Request(self.base + path, body, request_headers, method=method)
		try:
			response = self.opener.open(request, timeout=120)
		except urllib.error.HTTPError as error:
			message = f"PLE {method} {path}: HTTP {error.code}"
			request_id = None if error.headers is None else error.headers.get("X-Request-ID")
			# ASVS 16.4.1, 16.5.1: expose a safe correlation ID, never the error body.
			if request_id is not None and re.fullmatch(r"[A-Za-z0-9_-]+", request_id):
				message += f"; X-Request-ID: {request_id}"
			raise RuntimeError(message) from None
		with response:
			for header in response.headers.get_all("Set-Cookie", []):
				self.cookies.load(header)
			data = response.read()
			is_json = response.headers.get_content_type() == "application/json"
			result = json.loads(data) if data and is_json else {}
			response_headers = {name.lower(): value for name, value in response.headers.items()}
		return result, response_headers

	def json(self, method: str, path: str, payload: dict | None = None,
		headers: dict | None = None) -> dict:
		"""Return a JSON response for one ordinary operation."""
		result, _ = self.request(method, path, payload=payload, headers=headers)
		return result

	def classification(self, discipline: str, subject: str, topic: str | None = None) -> dict:
		"""Resolve existing vocabulary and create ordinary missing Topic entries."""
		disciplines = self.json("GET", "/api/content-classification/disciplines")["disciplines"]
		discipline_id = next(row["uuid"] for row in disciplines if row["name"] == discipline)
		query = urllib.parse.urlencode({"disciplineUuid": discipline_id})
		subjects = self.json("GET", "/api/content-classification/subjects?" + query)["subjects"]
		match = next((row for row in subjects if row["name"] == subject), None)
		if match is None:
			match = self.json("POST", "/api/content-classification/subjects",
				{"name": subject, "disciplineUuid": discipline_id})
			if match["needsAcceptance"]:
				self.json("POST", "/api/content-classification/subjects/" + match["uuid"]
					+ "/disciplines", {"disciplineUuid": discipline_id})
		subject_id = match["uuid"]
		topic_id = None
		if topic is not None:
			query = urllib.parse.urlencode({"subjectUuid": subject_id})
			topics = self.json("GET", "/api/content-classification/topics?" + query)["topics"]
			match = next((row for row in topics if row["name"] == topic), None)
			if match is None:
				match = self.json("POST", "/api/content-classification/topics",
					{"name": topic, "subjectUuid": subject_id})
			topic_id = match["uuid"]
		result = {"disciplineUuid": discipline_id, "subjectUuid": subject_id,
			"topicUuid": topic_id, "subtopicUuid": None, "tags": []}
		return result

	def create_blueprint(self, content: dict) -> dict:
		"""Supply the existing required command header for one creation."""
		result = self.json("POST", "/api/course-blueprints", content,
			{"Idempotency-Key": str(uuid.uuid4())})
		return result


class NoRedirect(urllib.request.HTTPRedirectHandler):
	"""Prevent a redirect from forwarding the in-memory session elsewhere."""

	def redirect_request(self, req: urllib.request.Request, fp: object, code: int,
		msg: str, headers: object, newurl: str) -> None:
		return None
