"""Host-owned shared curriculum import between private installer phases."""

import json
import pathlib
import shutil
import sys
import subprocess
import tempfile

import local_stack_control.compose
import local_stack_control.disposable_stack_cleanup
import local_stack_control.env_file
import local_stack_control.lifecycle_commands
import local_stack_control.models
import local_stack_control.process


#============================================
def importer_paths(target: local_stack_control.models.ComposeTarget) -> tuple[pathlib.Path, str]:
	"""Resolve explicit source/tool configuration or adjacent development checkouts."""
	values = local_stack_control.env_file.env_settings(target.env_file)
	repo_root = target.repo_root
	bpw_value = values.get("PLE_BPW_ROOT")
	bpw = pathlib.Path(bpw_value) if bpw_value else repo_root.parent / "biology-problems-website"
	if not bpw_value and not bpw.is_dir():
		bpw = repo_root / "OTHER_REPOS" / "biology-problems-website"
	if not (bpw / "site_docs").is_dir():
		raise local_stack_control.models.ControllerError(
			f"set PLE_BPW_ROOT in {target.env_file} to a BPW checkout with site_docs")
	qpm = values.get("PLE_QPM_CLI") or shutil.which("bbq-converter")
	if qpm is None:
		qpm_root = bpw.resolve().parent / "qti-package-maker-rs"
		for profile in ("release", "debug"):
			candidate = qpm_root / "target" / profile / "bbq-converter"
			if candidate.is_file():
				qpm = str(candidate)
				break
	if qpm is None:
		raise local_stack_control.models.ControllerError(
			f"set PLE_QPM_CLI in {target.env_file} to the QPM CLI executable")
	return bpw.resolve(), qpm


#============================================
def installer_phase(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	phase: list[str],
	stdin: str | None = None,
) -> local_stack_control.models.CommandResult:
	"""Keep temporary credentials in captured private pipes rather than argv or logs."""
	command = ["--profile", "migration", "run", "--rm", "--no-deps", "-T",
		"database-migrator", "installation-data", *phase]
	return runner.run(local_stack_control.compose.compose_argv(target, command),
		local_stack_control.lifecycle_commands.child_environment(target), target.repo_root, stdin)


#============================================
def provision(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	include_live_demo: bool = True,
) -> None:
	"""Import all four Courses through the user CLI, then adopt and provision fictional work."""
	bpw, qpm = importer_paths(target)
	private_values = local_stack_control.disposable_stack_cleanup.private_environment_values(target.env_file)
	phase = ["prepare"] if include_live_demo else ["prepare", "--without-live-demo"]
	prepared = installer_phase(target, runner, phase)
	# ASVS 13.3.2: a failed prepare may contain partial credential output; omit stdout.
	if not prepared.ok():
		raise local_stack_control.models.ControllerError("installation import context preparation failed")
	session = json.loads(prepared.stdout)
	encoded_session = json.dumps(session, separators=(",", ":"))
	private_values += (session["sessionCookie"], session["sessionCookie"].split("=", 1)[1])
	try:
		with tempfile.TemporaryDirectory(prefix="ple_install_") as directory:
			workspace = pathlib.Path(directory)
			values = local_stack_control.env_file.env_settings(target.env_file)
			origin = values["PLE_BROWSER_ORIGIN"]
			command = [sys.executable, str(target.repo_root / "launchers" / "import_bpw.py"),
				"--bpw-root", str(bpw), "--api-url", origin, "--qpm-cli", qpm,
				"--result-file", str(workspace / "result.json")]
			if origin.startswith("https://"):
				ca = gateway_ca(target, runner)
				ca_file = workspace / "gateway_root.crt"
				ca_file.write_text(ca, encoding="ascii")
				command.extend(["--ca-file", str(ca_file)])
			local_stack_control.lifecycle_commands.report_step("importing all four complete Blueprint Courses through the shared Python loader")
			result = run_loader(target, runner, command,
				json.dumps({"sessionCookie": session["sessionCookie"]}), private_values)
			local_stack_control.lifecycle_commands.require_command(result, "Curriculum HTTP import", private_values)
			imported = json.loads((workspace / "result.json").read_text(encoding="utf-8"))
			local_stack_control.lifecycle_commands.report_step("adopting Biochemistry and preparing representative Student work" if include_live_demo else "finalizing bundled curriculum")
			finalized = installer_phase(target, runner, ["finalize"],
				json.dumps({"session": session, "importResult": imported}))
			local_stack_control.lifecycle_commands.require_command(finalized, "Installation finalization", private_values)
	finally:
		# ASVS 7.4.1: publication, adoption, and failed imports all end the exact session.
		revoked = installer_phase(target, runner, ["revoke"], encoded_session)
		local_stack_control.lifecycle_commands.require_command(revoked, "Installation session revocation", private_values)


#============================================
def gateway_ca(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> str:
	"""Trust the selected owner's Caddy CA for this import process only."""
	result = runner.run(local_stack_control.compose.compose_argv(target, [
		"exec", "-T", "gateway", "cat", "/data/caddy/pki/authorities/local/root.crt",
	]), local_stack_control.lifecycle_commands.child_environment(target), target.repo_root)
	if not result.ok() or not result.stdout.startswith("-----BEGIN CERTIFICATE-----"):
		raise local_stack_control.models.ControllerError("selected gateway public root certificate is unavailable")
	return result.stdout


#============================================
def run_loader(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	command: list[str],
	stdin: str,
	private_values: tuple[str, ...],
) -> local_stack_control.models.CommandResult:
	"""Keep long imports informative while retaining the runner's private credential boundary."""
	environment = local_stack_control.lifecycle_commands.child_environment(target)
	if not isinstance(runner, local_stack_control.process.SubprocessRunner):
		return runner.run(command, environment, target.repo_root, stdin)
	# ASVS 1.2.5/13.3.2: structured argv and a private stdin pipe carry the session.
	child = subprocess.Popen(command, cwd=target.repo_root,
		env=local_stack_control.env_file.sanitized_runtime_environment(environment),
		stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
	child.stdin.write(stdin)
	child.stdin.close()
	lines = []
	for line in child.stdout:
		for value in private_values:
			line = line.replace(value, "[private]")
		lines.append(line)
		if line.startswith("[") and "/" in line:
			local_stack_control.lifecycle_commands.report_step("curriculum " + line.strip())
	child.stdout.close()
	return local_stack_control.models.CommandResult(tuple(command), child.wait(), "".join(lines), "")
