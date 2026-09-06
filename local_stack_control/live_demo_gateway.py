"""Policy-owned HTTPS gateway details for the disposable live-demo browser lane."""

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.models


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
	if not url.startswith("https://localhost:") or not url.endswith("/"):
		raise local_stack_control.models.ControllerError(
			"live-demo session probe requires the fixed HTTPS origin"
		)
	origin = url.removesuffix("/")
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
