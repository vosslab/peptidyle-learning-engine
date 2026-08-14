"""Fail-closed checks for stacks and fixed host ports outside one walkthrough."""

import pathlib

import local_stack_control.discovery
import local_stack_control.models
import local_stack_control.process

import walklib.models


#============================================
def assert_no_active_ple_stack(
	repository_root: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Refuse fixed ports when shared labels show a default or retained walkthrough stack."""
	try:
		snapshots = local_stack_control.discovery.project_snapshots(runner, repository_root)
	except local_stack_control.models.ControllerError as error:
		raise walklib.models.RunnerError("cannot inspect active PLE stack before walkthrough") from error
	for snapshot in snapshots:
		if len(snapshot.containers) == 0:
			continue
		if snapshot.project == local_stack_control.models.DEFAULT_PROJECT:
			raise walklib.models.RunnerError("an active default PLE stack blocks walkthrough acceptance")
		if snapshot.project.startswith("ple-ui-walkthrough-"):
			raise walklib.models.RunnerError("another disposable PLE walkthrough blocks acceptance")


#============================================
def assert_ports_available(
	ports: tuple[int, ...],
	repository_root: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Map the shared loopback-port preflight into the walkthrough error boundary."""
	try:
		local_stack_control.process.require_available_loopback_ports(
			ports,
			runner,
			repository_root,
		)
	except local_stack_control.models.ControllerError as error:
		raise walklib.models.RunnerError(str(error)) from error
