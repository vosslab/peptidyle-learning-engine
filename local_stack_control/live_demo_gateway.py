"""Policy-owned HTTPS gateway details for the disposable live-demo browser lane."""

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.models


#============================================
def is_tls_target(target: local_stack_control.models.ComposeTarget) -> bool:
	"""Recognize only the declared two-file live-demo TLS topology."""
	try:
		expected = local_stack_control.compose.disposable_policy_compose_files(
			target.repo_root, "live-demo-browser"
		)
	except local_stack_control.models.ControllerError:
		return False
	return target.compose_files == expected


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
