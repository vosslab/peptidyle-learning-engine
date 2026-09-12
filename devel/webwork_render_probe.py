#!/usr/bin/env python3
"""Capture one-time opaque WeBWorK renderer evidence for supplied PG sources.

This maintainer command deliberately accepts sources and student submissions at
the command line.  It does not carry a question corpus or turn the supplied
matrix into a permanent fixture.
"""

import argparse
import base64
import hashlib
import http.client
import http.server
import json
import pathlib
import re
import shutil
import subprocess
import threading
import urllib.error
import urllib.parse
import urllib.request


RENDER_PATH = "/render-api?_format=json"
ASSET_ROOT = "/api/webwork-assets"
MAX_RESPONSE_BYTES = 1_048_576
JWT_INPUT_PATTERN = re.compile(r"<input\b[^>]*\bname=[\"'][^\"']*JWT[^\"']*[\"']", re.IGNORECASE)
URL_PATTERN = re.compile(r"\b(?:src|href)=[\"']([^\"']+)[\"']", re.IGNORECASE)
FORM_ACTION_PATTERN = re.compile(r"<form\b[^>]*\baction=", re.IGNORECASE)
PROTECTED_MARKERS = ("PG_ANSWERS_HASH", "problemJWT", "sessionJWT", "answerJWT")


# ============================================
def url_origin(value: str, label: str) -> str:
	"""Return the normalized HTTP(S) origin of an absolute URL."""
	parts = urllib.parse.urlsplit(value)
	if parts.scheme not in ("http", "https") or not parts.hostname:
		raise ValueError(f"{label} must be an absolute HTTP(S) origin")
	if parts.username or parts.password:
		raise ValueError(f"{label} must not contain credentials")
	try:
		port = parts.port
	except ValueError as error:
		raise ValueError(f"{label} has an invalid port") from error
	host = parts.hostname
	if ":" in host:
		host = f"[{host}]"
	return f"{parts.scheme}://{host}" + (f":{port}" if port else "")


# ============================================
def configured_origin(value: str, label: str) -> str:
	"""Validate and normalize one explicit HTTP(S) origin supplied to the probe."""
	parts = urllib.parse.urlsplit(value)
	if parts.path not in ("", "/") or parts.query or parts.fragment:
		raise ValueError(f"{label} must contain only an HTTP(S) origin")
	return url_origin(value, label)


# ============================================
def origin_url(origin: str, path: str) -> str:
	"""Join an already validated origin to an absolute path."""
	if not path.startswith("/"):
		raise ValueError("probe endpoint paths must start with '/'")
	return origin + path


# ============================================
def open_configured_url(
	request: urllib.request.Request | str, origin: str, timeout: int
) -> http.client.HTTPResponse:
	"""Open a URL only after pinning it to one explicitly configured origin."""
	url = request.full_url if isinstance(request, urllib.request.Request) else request
	if url_origin(url, "request URL") != origin:
		raise ValueError("probe request escaped its configured origin")
	# Bandit B310: the URL has been validated as the exact configured origin above.
	return urllib.request.urlopen(request, timeout=timeout)  # nosec B310


# ============================================
def parse_args() -> argparse.Namespace:
	"""Parse explicit evidence inputs."""
	parser = argparse.ArgumentParser(description=__doc__)
	inputs = parser.add_mutually_exclusive_group(required=True)
	inputs.add_argument("-i", "--source", type=pathlib.Path, help="one PG or PGML source")
	inputs.add_argument("--matrix", type=pathlib.Path, help="one-time source/submission evidence JSON")
	parser.add_argument("--seed", type=int, default=271828, help="Question Seed for --source")
	parser.add_argument(
		"--submission",
		action="append",
		default=[],
		help="JSON object or ordered [[name, value], ...] student fields; repeatable",
	)
	parser.add_argument("-o", "--output", required=True, type=pathlib.Path, help="evidence directory")
	parser.add_argument("--renderer-url", default="http://127.0.0.1:3111", help="private renderer origin")
	parser.add_argument("--ple-origin", default=None, help="temporary PLE origin for rendered URLs")
	parser.add_argument("--browser", action=argparse.BooleanOptionalAction, default=True, help="compare opaque-origin and same-origin iframe sandboxes")
	return parser.parse_args()


# ============================================
def json_value(raw: str) -> object:
	"""Load inline JSON, or JSON held in an explicit @path."""
	if raw.startswith("@"):
		content = pathlib.Path(raw[1:]).read_text(encoding="utf-8")
	else:
		content = raw
	return json.loads(content)


# ============================================
def submission_pairs(raw: str) -> list[list[str]]:
	"""Validate a submission while preserving input order and duplicate names."""
	value = json_value(raw)
	if isinstance(value, dict):
		pairs = [[str(name), str(field)] for name, field in value.items()]
	elif isinstance(value, list):
		pairs = []
		for item in value:
			if not isinstance(item, list) or len(item) != 2:
				raise ValueError("submission arrays must contain [name, value] pairs")
			pairs.append([str(item[0]), str(item[1])])
	else:
		raise ValueError("submission must be a JSON object or [name, value] pair array")
	return pairs


# ============================================
def normalized_pairs(value: object) -> list[list[str]]:
	"""Normalize a matrix field value to the canonical ordered pair array."""
	encoded = json.dumps(value, ensure_ascii=True)
	return submission_pairs(encoded)


# ============================================
def renderer_request(renderer_url: str, pairs: list[tuple[str, str]]) -> dict:
	"""POST ordered form fields to the standalone renderer and decode its JSON envelope."""
	renderer_origin = configured_origin(renderer_url, "--renderer-url")
	body = urllib.parse.urlencode(pairs).encode("utf-8")
	request = urllib.request.Request(
		origin_url(renderer_origin, RENDER_PATH),
		data=body,
		headers={"Content-Type": "application/x-www-form-urlencoded", "Accept": "application/json"},
		method="POST",
	)
	try:
		with open_configured_url(request, renderer_origin, timeout=30) as response:
			content = response.read(MAX_RESPONSE_BYTES + 1)
	except urllib.error.HTTPError as error:
		body_text = error.read(4096).decode("utf-8", errors="replace")
		raise RuntimeError(f"renderer HTTP {error.code}: {body_text}") from error
	if len(content) > MAX_RESPONSE_BYTES:
		raise RuntimeError("renderer response exceeded the probe limit")
	value = json.loads(content)
	if not isinstance(value, dict):
		raise RuntimeError("renderer response is not a JSON object")
	return value


# ============================================
def render_fields(source: str, source_name: str, seed: int, ple_origin: str) -> list[tuple[str, str]]:
	"""Build the fixed server-owned render request fields."""
	encoded_source = base64.b64encode(source.encode("utf-8")).decode("ascii")
	fields = [
		("_format", "json"),
		("problemSource", encoded_source),
		("sourceFilePath", source_name),
		("problemSeed", str(seed)),
		("outputFormat", "ple_embed"),
		("pleOrigin", ple_origin),
		("pleAssetBase", ple_origin.rstrip("/") + ASSET_ROOT),
		("displayMode", "MathJax"),
		("isInstructor", "0"),
		("showHints", "0"),
		("showSolutions", "0"),
		("showSummary", "0"),
		("hidePreviewButton", "1"),
		("hideCheckAnswersButton", "1"),
		("hideAttemptsTable", "1"),
		("hideMessages", "1"),
		("showCorrectAnswersButton", "0"),
		("showFooter", "0"),
	]
	return fields


# ============================================
def canonical(value: object) -> str:
	"""Return a stable JSON representation for evidence comparisons."""
	result = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
	return result


# ============================================
def document_checks(document: str, source: str, ple_origin: str, renderer_url: str) -> dict:
	"""Classify document URLs and verify the credential-free embed contract."""
	urls = URL_PATTERN.findall(document)
	asset_prefix = ple_origin.rstrip("/") + ASSET_ROOT + "/"
	bridge_url = ple_origin.rstrip("/") + "/ple_bridge.js"
	style_url = ple_origin.rstrip("/") + "/styles/ple_embed.css"
	classified = []
	for url in urls:
		if url == bridge_url:
			kind = "bridge"
		elif url == style_url:
			kind = "ple_style"
		elif url.startswith(asset_prefix + "pg_files/") or url.startswith(asset_prefix + "webwork2_files/"):
			kind = "renderer_asset"
		elif url.startswith("#") or not urllib.parse.urlsplit(url).scheme:
			kind = "non_resource_link"
		else:
			kind = "unexpected_url"
		classified.append({"url": url, "kind": kind})
	protected = [marker for marker in PROTECTED_MARKERS if marker in document]
	bridge_count = sum(1 for item in classified if item["kind"] == "bridge")
	style_count = sum(1 for item in classified if item["kind"] == "ple_style")
	asset_urls = [item["url"] for item in classified if item["kind"] == "renderer_asset"]
	checks = {
		"no_base": not bool(re.search(r"<base\b", document, re.IGNORECASE)),
		"no_renderer_jwt_hidden_input": not bool(JWT_INPUT_PATTERN.search(document)),
		"no_submit_container": "submit-buttons-container" not in document,
		"no_footer": not bool(re.search(r"id=[\"']footer[\"']", document, re.IGNORECASE)),
		"no_onload": not bool(re.search(r"\bonLoad\s*=", document, re.IGNORECASE)),
		"form_has_no_action": not bool(FORM_ACTION_PATTERN.search(document)),
		"one_bridge": bridge_count == 1,
		"one_ple_style": style_count == 1,
		"renderer_assets_query_free": all("?" not in url and "#" not in url for url in asset_urls),
		"renderer_assets_in_proxy_namespaces": all(item["kind"] != "unexpected_url" for item in classified),
		"no_protected_markers": not protected,
		"no_renderer_origin": renderer_url.rstrip("/") not in document,
	}
	return {
		"checks": checks,
		"urls": classified,
		"protected_markers": protected,
		"source_line_leaks": [],
	}


# ============================================
def grade_fields(base: list[tuple[str, str]], pairs: list[list[str]], envelope: dict, carry_state: bool) -> list[tuple[str, str]]:
	"""Build exactly one grade request, optionally carrying renderer-issued state."""
	fields = list(base)
	fields.extend([("submitAnswers", "1"), ("answersSubmitted", "1")])
	fields.extend((name, value) for name, value in pairs)
	if carry_state:
		jwt = envelope["JWT"]["session"]
		if jwt:
			fields.append(("sessionJWT", str(jwt)))
		fields.append(("problem_state", canonical(envelope["problem_state"])))
	return fields


# ============================================
def grade_results(renderer_url: str, base: list[tuple[str, str]], submissions: list[list[list[str]]], envelope: dict) -> list[dict]:
	"""Record no-state and state-carrying one-grade-request results."""
	results = []
	for index, pairs in enumerate(submissions):
		without_state = renderer_request(renderer_url, grade_fields(base, pairs, envelope, False))
		with_state = renderer_request(renderer_url, grade_fields(base, pairs, envelope, True))
		without_result = without_state["problem_result"]
		with_result = with_state["problem_result"]
		without_problem_state = without_state["problem_state"]
		with_problem_state = with_state["problem_state"]
		results.append({
			"submission_index": index,
			"pairs": pairs,
			"without_state": {"problem_result": without_result, "problem_state": without_problem_state},
			"with_state": {"problem_result": with_result, "problem_state": with_problem_state},
			"problem_result_identical": canonical(without_result) == canonical(with_result),
			"problem_state_identical": canonical(without_problem_state) == canonical(with_problem_state),
		})
	return results


# ============================================
def generated_asset_fetches(urls: list[dict], ple_origin: str) -> list[dict]:
	"""Fetch discovered generated image assets through the temporary proxy."""
	configured_ple_origin = configured_origin(ple_origin, "PLE origin")
	image_urls = [
		entry["url"]
		for entry in urls
		if entry["kind"] == "renderer_asset"
		and re.search(r"\.(?:png|svg|gif|jpe?g)$", urllib.parse.urlsplit(entry["url"]).path, re.IGNORECASE)
	]
	results = []
	for url in image_urls:
		try:
			with open_configured_url(url, configured_ple_origin, timeout=15) as response:
				body = response.read(MAX_RESPONSE_BYTES)
				results.append({
					"url": url,
					"status": response.status,
					"content_type": response.headers.get_content_type(),
					"bytes": len(body),
				})
		except urllib.error.HTTPError as error:
			results.append({"url": url, "status": error.code, "content_type": None, "bytes": 0})
	return results


# ============================================
def browser_experiment(output_dir: pathlib.Path, document_url: str) -> dict:
	"""Compare opaque-origin and same-origin iframe sandboxes with Playwright."""
	if shutil.which("node") is None:
		return {"status": "unavailable", "reason": "node is not installed"}
	script = output_dir / "temporary_browser_experiment.mjs"
	# This temporary page does not emulate PLE's browser integration. It records
	# whether the captured document loads under each sandbox setting.
	script_text = """import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('PLAYWRIGHT_MODULE');
const documentUrl = process.argv[2];
const browser = await chromium.launch({ headless: true });
const outcomes = [];
for (const [name, sandbox] of [['opaque-origin', 'allow-scripts allow-forms'], ['same-origin', 'allow-scripts allow-forms allow-same-origin']]) {
  const page = await browser.newPage();
  const errors = [];
  page.on('console', entry => { if (entry.type() === 'error') errors.push(entry.text()); });
  page.on('pageerror', error => errors.push(error.message));
  const frameUrl = name === 'same-origin' ? `${documentUrl}?csp=same-origin` : documentUrl;
  const harnessUrl = new URL('/probe_harness.html', documentUrl).href;
  await page.goto(harnessUrl);
  await page.evaluate(({ frameUrl, sandbox }) => {
    const frame = document.createElement('iframe');
    frame.id = 'probe'; frame.sandbox.value = sandbox; frame.src = frameUrl;
    document.body.append(frame);
  }, { frameUrl, sandbox });
  const frame = page.frames().find(candidate => candidate !== page.mainFrame());
  let keyboard = false;
  let controls = 0;
  let origin = null;
  if (frame) {
    await frame.waitForLoadState('domcontentloaded').catch(() => {});
    controls = await frame.locator('input, select, textarea, button').count();
    if (controls) { await frame.locator('input, select, textarea, button').first().focus().catch(() => {}); keyboard = true; }
    origin = await frame.evaluate(() => location.origin).catch(() => null);
  }
  outcomes.push({ name, sandbox, csp: name === 'same-origin', frame_loaded: Boolean(frame), origin, controls, keyboard, console_errors: errors });
  await page.close();
}
await browser.close();
console.log(JSON.stringify(outcomes));
	"""
	playwright_module = pathlib.Path(__file__).parent.parent / "node_modules" / "playwright"
	script_text = script_text.replace("PLAYWRIGHT_MODULE", playwright_module.as_posix())
	script.write_text(script_text, encoding="utf-8")
	try:
		result = subprocess.run(
			["node", str(script), document_url],
			cwd=output_dir,
			capture_output=True,
			text=True,
			timeout=60,
			check=False,
		)
	except subprocess.TimeoutExpired:
		return {"status": "unavailable", "reason": "temporary browser experiment timed out"}
	if result.returncode != 0:
		return {"status": "unavailable", "reason": result.stderr[-1000:]}
	return {"status": "completed", "outcomes": json.loads(result.stdout)}


# ============================================
class EvidenceServer(http.server.ThreadingHTTPServer):
	"""Temporary PLE-origin server for captured documents and renderer assets."""

	def __init__(self, output_dir: pathlib.Path, renderer_url: str) -> None:
		self.output_dir = output_dir
		self.renderer_origin = configured_origin(renderer_url, "--renderer-url")
		super().__init__(("127.0.0.1", 0), EvidenceHandler)

	def handle_error(self, request: object, client_address: object) -> None:
		"""Ignore browser-aborted temporary asset reads."""
		return


class EvidenceHandler(http.server.SimpleHTTPRequestHandler):
	"""Serve captured evidence and forward only the two renderer asset namespaces."""

	def __init__(self, *args: object, **kwargs: object) -> None:
		server = args[2]
		super().__init__(*args, directory=str(server.output_dir), **kwargs)

	def do_GET(self) -> None:
		if urllib.parse.urlsplit(self.path).path == "/probe_harness.html":
			body = b"<!doctype html><title>PLE probe harness</title>\n"
			self.send_response(200)
			self.send_header("Content-Type", "text/html; charset=utf-8")
			self.send_header("Content-Length", str(len(body)))
			self.end_headers()
			self.wfile.write(body)
			return
		if self.path == "/ple_bridge.js":
			body = b"window.addEventListener('message', function () {});\n"
			self.send_response(200)
			self.send_header("Content-Type", "application/javascript")
			self.send_header("Content-Length", str(len(body)))
			self.end_headers()
			self.wfile.write(body)
			return
		if self.path == "/styles/ple_embed.css":
			body = b"/* temporary PLE baseline stylesheet endpoint */\n"
			self.send_response(200)
			self.send_header("Content-Type", "text/css; charset=utf-8")
			self.send_header("Content-Length", str(len(body)))
			self.end_headers()
			self.wfile.write(body)
			return
		if self.path.startswith(ASSET_ROOT + "/"):
			path = urllib.parse.urlsplit(self.path)
			if path.query or path.fragment:
				self.send_error(404)
				return
			relative = path.path[len(ASSET_ROOT) + 1:]
			if not (relative.startswith("pg_files/") or relative.startswith("webwork2_files/")):
				self.send_error(404)
				return
			try:
				asset_url = origin_url(self.server.renderer_origin, "/" + relative)
				with open_configured_url(asset_url, self.server.renderer_origin, timeout=15) as upstream:
					body = upstream.read(MAX_RESPONSE_BYTES)
					content_type = upstream.headers.get_content_type()
			except urllib.error.URLError:
				self.send_error(502)
				return
			self.send_response(200)
			self.send_header("Content-Type", content_type)
			self.send_header("Content-Length", str(len(body)))
			self.end_headers()
			self.wfile.write(body)
			return
		super().do_GET()

	def log_message(self, format: str, *args: object) -> None:
		return

	def end_headers(self) -> None:
		query = urllib.parse.urlsplit(self.path).query
		if query == "csp=same-origin":
			self.send_header(
				"Content-Security-Policy",
				"default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; "
				"img-src 'self' data:; base-uri 'none'; object-src 'none'; "
				"frame-ancestors 'self'; form-action 'none'",
			)
		super().end_headers()


# ============================================
def probe_one(source_path: pathlib.Path, seed: int, submissions: list[list[list[str]]], output_dir: pathlib.Path, renderer_url: str, ple_origin: str, browser: bool, document_url: str) -> dict:
	"""Collect one-time renderer evidence for one explicit source and seed."""
	source = source_path.read_text(encoding="utf-8")
	base = render_fields(source, source_path.name, seed, ple_origin)
	envelope = renderer_request(renderer_url, base)
	document = envelope["renderedHTML"]
	document_path = output_dir / "document.html"
	document_path.write_text(document, encoding="utf-8")
	(output_dir / "envelope.json").write_text(json.dumps(envelope, indent=2) + "\n", encoding="utf-8")
	inspection = document_checks(document, source, ple_origin, renderer_url)
	(output_dir / "asset_urls.json").write_text(json.dumps(inspection["urls"], indent=2) + "\n", encoding="utf-8")
	asset_fetches = generated_asset_fetches(inspection["urls"], ple_origin)
	grades = grade_results(renderer_url, base, submissions, envelope)
	(output_dir / "grade_results.json").write_text(json.dumps(grades, indent=2) + "\n", encoding="utf-8")
	state_identical = all(row["problem_result_identical"] and row["problem_state_identical"] for row in grades)
	browser_result = browser_experiment(output_dir, document_url) if browser else {"status": "skipped"}
	asset_fetches_pass = all(200 <= item["status"] < 300 for item in asset_fetches)
	fixed_pass = all(inspection["checks"].values()) and asset_fetches_pass
	return {
		"source": str(source_path),
		"source_sha256": hashlib.sha256(source.encode("utf-8")).hexdigest(),
		"seed": seed,
		"fixed_requirements_pass": fixed_pass,
		"document": inspection,
		"generated_asset_fetches": asset_fetches,
		"generated_assets_fetchable": asset_fetches_pass,
		"grades": grades,
		"stateless_grading": {"one_grade_request_results_and_state_are_identical": state_identical},
		"browser": browser_result,
	}


# ============================================
def matrix_rows(matrix_path: pathlib.Path) -> list[tuple[pathlib.Path, int, list[list[list[str]]], str]]:
	"""Turn a temporary source/submission record into explicit probe inputs."""
	matrix = json.loads(matrix_path.read_text(encoding="utf-8"))
	rows = list(matrix["observations"] if "observations" in matrix else matrix["items"])
	rows.extend(matrix.get("supplementary", {}).values())
	resolved = []
	for row in rows:
		submissions = []
		for entry in row.get("submissions", []):
			field_value = entry["ordered_pairs"] if "ordered_pairs" in entry else entry["fields"]
			submissions.append(normalized_pairs(field_value))
		source_name = row["canonical_path"] if "canonical_path" in row else row["source"]
		source_path = pathlib.Path(source_name)
		if not source_path.is_absolute():
			repo_root = pathlib.Path(__file__).parent.parent
			source_path = repo_root / source_path
		resolved.append((source_path, int(row["seed"]), submissions, row["id"]))
	return resolved


# ============================================
def main() -> None:
	"""Run the evidence probe and write one machine-readable findings document."""
	args = parse_args()
	args.output.mkdir(parents=True, exist_ok=True)
	server = None
	server_thread = None
	if args.ple_origin:
		ple_origin = args.ple_origin.rstrip("/")
	else:
		server = EvidenceServer(args.output, args.renderer_url)
		server_thread = threading.Thread(target=server.serve_forever, daemon=True)
		server_thread.start()
		ple_origin = f"http://127.0.0.1:{server.server_port}"
	if args.matrix:
		rows = matrix_rows(args.matrix)
	else:
		rows = [(args.source, args.seed, [submission_pairs(raw) for raw in args.submission], args.source.stem)]
	findings = {
		"renderer_url": args.renderer_url,
		"ple_origin": ple_origin,
		"evidence_scope": "one-time connected renderer probe; not a permanent corpus or fixture test",
		"items": {},
	}
	try:
		for source_path, seed, submissions, identifier in rows:
			item_dir = args.output / identifier
			item_dir.mkdir(exist_ok=True)
			document_url = ple_origin + "/" + identifier + "/document.html"
			findings["items"][identifier] = probe_one(
				source_path, seed, submissions, item_dir, args.renderer_url, ple_origin, args.browser, document_url
			)
		all_fixed = all(item["fixed_requirements_pass"] for item in findings["items"].values())
		findings["fixed_requirements_pass"] = all_fixed
		(args.output / "findings.json").write_text(json.dumps(findings, indent=2) + "\n", encoding="utf-8")
		print(json.dumps({"output": str(args.output), "fixed_requirements_pass": all_fixed}, indent=2))
		if not all_fixed:
			raise RuntimeError("one or more fixed renderer requirements failed")
	finally:
		if server:
			server.shutdown()
			server.server_close()
			server_thread.join()


if __name__ == "__main__":
	main()
