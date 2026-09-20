"""The default stack and Live Demo must not both run the same service."""

import local_stack_control.models
import local_stack_control.service_singletons


#============================================
def running_container(
	service: str,
	project: str,
	name: str,
) -> local_stack_control.models.ContainerResource:
	"""One running labelled long-running service instance."""
	return local_stack_control.models.ContainerResource(
		id=name,
		names=(name,),
		project=project,
		service=service,
		state="running",
		running=True,
		exit_code=None,
		health="healthy",
		image="local-image",
		ports=(),
	)


#============================================
def snapshot(
	project: str,
	containers: tuple[local_stack_control.models.ContainerResource, ...],
) -> local_stack_control.models.ProjectSnapshot:
	"""One labelled project inventory."""
	return local_stack_control.models.ProjectSnapshot(
		project=project,
		containers=containers,
		volumes=(),
		networks=(),
	)


#============================================
def test_one_live_demo_postgres_is_allowed() -> None:
	"""The ordinary Live Demo may run one postgres."""
	project = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	inventory = (
		snapshot(project, (running_container("postgres", project, "ple-live-demo-browser_postgres_1"),)),
	)
	violations = local_stack_control.service_singletons.singleton_violations(
		inventory, project, lambda service: 1
	)
	assert violations == ()


#============================================
def test_default_stack_and_live_demo_postgres_together_are_refused() -> None:
	"""Two developer projects must not both run postgres."""
	live = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	default = local_stack_control.models.DEFAULT_PROJECT
	inventory = (
		snapshot(live, (running_container("postgres", live, "ple-live-demo-browser_postgres_1"),)),
		snapshot(default, (running_container("postgres", default, "containers_postgres_1"),)),
	)
	violations = local_stack_control.service_singletons.singleton_violations(
		inventory, live, lambda service: 1
	)
	joined = " ".join(violations)
	assert "postgres" in joined
	assert "containers_postgres_1" in joined
