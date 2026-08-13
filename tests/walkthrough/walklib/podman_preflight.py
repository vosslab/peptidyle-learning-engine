"""Fail-closed checks for stacks and fixed host ports outside one walkthrough."""

import walklib.models
import walklib.podman_ownership


#============================================
def assert_no_active_ple_stack(run_command: walklib.models.CommandRunner) -> None:
	"""Refuse fixed-port acceptance while a default or disposable PLE stack exists."""
	for label_key in walklib.podman_ownership.PROJECT_LABEL_KEYS:
		for project_name in ("containers",):
			result = run_command(
				[
					"podman", "ps", "--all", "--quiet", "--filter",
					f"label={label_key}={project_name}",
				],
				None,
			)
			if result.returncode != 0:
				raise walklib.models.RunnerError("cannot inspect active PLE stack before walkthrough")
			if result.stdout.strip():
				raise walklib.models.RunnerError("an active default PLE stack blocks walkthrough acceptance")
		result = run_command(
			["podman", "ps", "--all", "--quiet", "--filter", f"label={label_key}"], None
		)
		if result.returncode != 0:
			raise walklib.models.RunnerError("cannot inspect active PLE stack before walkthrough")
		for container_id in result.stdout.splitlines():
			if not container_id:
				continue
			inspection = run_command(
				["podman", "container", "inspect", "--format", f"{{{{ index .Config.Labels \"{label_key}\" }}}}", container_id],
				None,
			)
			if inspection.returncode != 0:
				raise walklib.models.RunnerError("cannot inspect active PLE stack before walkthrough")
			if inspection.stdout.strip().startswith("ple-ui-walkthrough-"):
				raise walklib.models.RunnerError("another disposable PLE walkthrough blocks acceptance")


#============================================
def assert_ports_available(
	ports: tuple[int, ...], run_command: walklib.models.CommandRunner
) -> None:
	"""Require each fixed loopback port to be listener-free before the launcher runs."""
	for port in ports:
		result = run_command(
			["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"], None
		)
		if result.returncode not in (0, 1):
			raise walklib.models.RunnerError("cannot inspect local ports before walkthrough")
		if result.stdout.strip():
			raise walklib.models.RunnerError(f"local port {port} is already listening")
