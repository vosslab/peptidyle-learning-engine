"""Policy-owned HTTPS gateway details for the disposable live-demo browser lane."""

# Standard Library
import json
import pathlib
import urllib.parse

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.models


SEEDED_DEMO_PERSONAS = (
	"elenaInstructor",
	"maryStudent",
	"jackStudent",
	"averyStudent",
	"morganSysadmin",
)


#============================================
def live_demo_origin(url: str) -> str:
	"""Return the fixed HTTPS origin accepted by first-party demo requests."""
	if not url.startswith("https://localhost:") or not url.endswith("/"):
		raise local_stack_control.models.ControllerError(
			"live-demo request requires the fixed HTTPS origin"
		)
	origin = url.removesuffix("/")
	return origin


#============================================
def demo_request_path(path: str) -> str:
	"""Return one bounded same-origin API path without a fragment or authority."""
	parsed = urllib.parse.urlsplit(path)
	# ASVS 1.2.2 and 4.2.5: accept only a bounded relative product API URI.
	if (
		parsed.scheme != ""
		or parsed.netloc != ""
		or parsed.fragment != ""
		or not parsed.path.startswith("/api/")
		or len(path) > 2_048
		or "\r" in path
		or "\n" in path
	):
		raise local_stack_control.models.ControllerError("live-demo API path is invalid")
	return path


#============================================
def is_tls_target(target: local_stack_control.models.ComposeTarget) -> bool:
	"""Recognize only one of the closed production-auth live-demo topologies."""
	for profile in local_stack_control.models.LiveDemoProfile:
		try:
			expected = local_stack_control.compose.disposable_policy_compose_files(
				target.repo_root,
				local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
				profile,
			)
		except local_stack_control.models.ControllerError:
			continue
		if target.compose_files == expected:
			return True
	return False


#============================================
def gateway_url(target: local_stack_control.models.ComposeTarget) -> str:
	"""Return the selected loopback gateway origin after validating its port."""
	values = local_stack_control.env_file.env_settings(target.env_file)
	port = values.get("PLE_GATEWAY_HOST_PORT", "8080")
	if not port.isdecimal() or not 1 <= int(port) <= 65535:
		raise local_stack_control.models.ControllerError("selected gateway port is invalid")
	if is_tls_target(target):
		return f"https://localhost:{port}/"
	return f"http://127.0.0.1:{port}/"


#============================================
def health_probe_argv(url: str) -> list[str]:
	"""Build a gateway health probe, trusting only the lane's internal certificate."""
	argv = ["curl", "--fail", "--silent", "--show-error", "--max-time", "2"]
	if url.startswith("https://"):
		argv.append("--insecure")
	argv.extend(("--output", "/dev/null", url + "health"))
	return argv


#============================================
def seeded_session_probe_argv(url: str) -> list[str]:
	"""Build one same-origin demo-session probe after generic health succeeds."""
	origin = live_demo_origin(url)
	return [
		"curl", "--fail", "--silent", "--show-error", "--max-time", "2", "--insecure",
		"--request", "POST",
		"--header", f"origin: {origin}",
		"--header", "accept: application/json",
		"--header", "content-type: application/json",
		"--data", '{"persona":"elenaInstructor"}',
		"--output", "/dev/null",
		origin + "/api/auth/live-demo/accounts",
	]


#============================================
def persona_session_argv(url: str, persona: str, cookie_jar_path: pathlib.Path) -> list[str]:
	"""Build one seeded-persona session request that writes only to a cookie jar."""
	if persona not in SEEDED_DEMO_PERSONAS:
		raise local_stack_control.models.ControllerError("live-demo persona is invalid")
	origin = live_demo_origin(url)
	body = json.dumps({"persona": persona}, separators=(",", ":"))
	# ASVS 13.3.2: the credential remains in the private jar and never enters argv.
	argv = [
		"curl", "--silent", "--show-error", "--max-time", "12", "--insecure",
		"--request", "POST",
		"--header", f"origin: {origin}",
		"--header", "accept: application/json",
		"--header", "content-type: application/json",
		"--data", body,
		"--cookie-jar", str(cookie_jar_path),
		"--output", "/dev/null",
		"--write-out", "\n%{http_code}",
		origin + "/api/auth/live-demo/accounts",
	]
	return argv


#============================================
def demo_request_argv(
	url: str,
	path: str,
	cookie_jar_path: pathlib.Path,
	method: str,
	body: dict | None = None,
	if_match: str | None = None,
) -> list[str]:
	"""Build one authenticated same-origin JSON product request."""
	if method not in ("GET", "POST", "PUT"):
		raise local_stack_control.models.ControllerError("live-demo HTTP method is invalid")
	if method == "GET" and body is not None:
		raise local_stack_control.models.ControllerError("live-demo GET request must not carry a body")
	if if_match is not None and (method == "GET" or not if_match.isdecimal() or int(if_match) < 1):
		raise local_stack_control.models.ControllerError("live-demo If-Match value is invalid")
	origin = live_demo_origin(url)
	checked_path = demo_request_path(path)
	argv = [
		"curl", "--silent", "--show-error", "--max-time", "12", "--insecure",
		"--request", method,
		"--header", f"origin: {origin}",
		"--header", "accept: application/json",
		"--header", "content-type: application/json",
		"--cookie", str(cookie_jar_path),
		"--write-out", "\n%{http_code}",
	]
	if if_match is not None:
		argv.extend(("--header", f'if-match: "{if_match}"'))
	if body is not None:
		# ASVS 1.2.3 and 1.5.2: one JSON encoder owns the request representation.
		encoded_body = json.dumps(body, separators=(",", ":"))
		argv.extend(("--data", encoded_body))
	argv.append(origin + checked_path)
	return argv
