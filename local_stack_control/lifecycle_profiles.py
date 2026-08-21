"""Closed lifecycle bootstrap profiles for normal and disposable targets."""

import local_stack_control.local_environment
import local_stack_control.models


#============================================
def target_of(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ComposeTarget:
	"""Return the common target without changing its owner authority."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return target.target
	return target


#============================================
def is_default_target(target: local_stack_control.models.ComposeTarget) -> bool:
	"""Return whether the exact target is eligible for default local bootstrap."""
	return (
		target.project == local_stack_control.models.DEFAULT_PROJECT
		and local_stack_control.local_environment.is_default_local_environment(
			target.repo_root, target.env_file
		)
	)


#============================================
def is_teaching_profile(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether a closed disposable owner selected teaching seed state."""
	return (
		isinstance(target, local_stack_control.models.DisposableComposeTarget)
		and target.owner_policy in {"live-demo-baseline", "live-demo-browser", "ui-walkthrough"}
	)


#============================================
def uses_local_teaching_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether target ownership authorizes teaching seed state."""
	return is_default_target(target_of(target)) or is_teaching_profile(target)


#============================================
def uses_local_auth_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether the selected owner may create local-file credentials and identities."""
	return is_default_target(target_of(target)) or (
		isinstance(target, local_stack_control.models.DisposableComposeTarget)
		and target.owner_policy == "ui-walkthrough"
	)


#============================================
def uses_live_demo_sysadmin_claim_context(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether the selected owner exercises Sysadmin ownership setup."""
	return (
		isinstance(target, local_stack_control.models.DisposableComposeTarget)
		and target.owner_policy == "live-demo-browser"
	)
