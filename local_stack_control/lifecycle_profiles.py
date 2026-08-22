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
		and (
			target.owner_policy == "live-demo-baseline"
			or (
				target.owner_policy == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
				and target.live_demo_profile is not None
			)
		)
	)


#============================================
def uses_local_teaching_state(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether target ownership authorizes teaching seed state."""
	return is_default_target(target_of(target)) or is_teaching_profile(target)


#============================================
def uses_live_demo_sysadmin_claim_context(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> bool:
	"""Return whether the selected owner exercises Sysadmin ownership setup."""
	return (
		isinstance(target, local_stack_control.models.DisposableComposeTarget)
		and target.owner_policy == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
		and target.live_demo_profile is not None
	)


#============================================
def expected_long_running_count(
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> int:
	"""Return the closed profile-owned instance count for one required service."""
	policy = selected_live_demo_profile_policy(target)
	if policy is None:
		return 1
	for selected_service, count in policy.service_replica_counts:
		if selected_service == service:
			return count
	return 1


#============================================
def selected_live_demo_profile_policy(
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.LiveDemoProfilePolicy | None:
	"""Return the closed profile policy carried by one typed lifecycle target."""
	if not isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return None
	if target.owner_policy != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		return None
	if target.live_demo_profile is None:
		raise local_stack_control.models.ControllerError(
			"live-demo target does not declare a supported profile"
		)
	policy = local_stack_control.models.live_demo_profile_policy(
		target.live_demo_profile
	)
	return policy


#============================================
def application_scale_arguments(
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	services: tuple[str, ...],
) -> tuple[str, ...]:
	"""Return lifecycle-owned Compose scale arguments for selected services."""
	arguments: list[str] = []
	for service in services:
		count = expected_long_running_count(target, service)
		if count > 1:
			arguments.extend(("--scale", f"{service}={count}"))
	return tuple(arguments)


#============================================
def recreate_arguments(
	target: local_stack_control.models.ComposeTarget
	| local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> list[str]:
	"""Build one service recreate command with closed profile scaling."""
	arguments = ["up", "-d", "--force-recreate", "--no-deps"]
	arguments.extend(application_scale_arguments(target, (service,)))
	arguments.append(service)
	return arguments
