"""Cleanup, outage, replica, diagnostic, and redaction helpers for disposable stacks."""

import pathlib
import re
import uuid

import local_stack_control.browser_suite_ownership
import local_stack_control.cleanup
import local_stack_control.compose
import local_stack_control.discovery
import local_stack_control.disposable_stack_adapter
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.runtime_manifest


CONTAINER_ID_PREFIX_PATTERN = re.compile(r"^[a-f0-9]{12}$")
POSTGRESQL_ATTEMPT_COUNT_QUERIES = (
	"SELECT count(*) FROM ple_private.question_attempt WHERE question_attempt_id = :'attempt_id'::uuid",
	"SELECT count(*) FROM ple_private.assessment_attempt_saved_response WHERE question_attempt_id = :'attempt_id'::uuid",
	"SELECT count(*) FROM ple_private.grading_result WHERE question_attempt_id = :'attempt_id'::uuid",
	"SELECT count(*) FROM ple_audit.automated_grading_receipt AS receipt "
	"JOIN ple_private.grading_result AS result ON result.grading_result_id = receipt.grading_result_id "
	"WHERE result.question_attempt_id = :'attempt_id'::uuid",
)
POSTGRESQL_ATTEMPT_COUNT_RESULT_PATTERN = re.compile(
	r"[0-9]{1,10}"
	+ r"(?:\|[0-9]{1,10})" * (len(POSTGRESQL_ATTEMPT_COUNT_QUERIES) - 1)
)
EVIDENCE_LOG_TAIL_LINES = 5_000
EVIDENCE_LOG_MAX_CHARACTERS = 1_000_000


#============================================
def require_declared_outage_snapshot(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Require a complete snapshot to describe only the selected labelled project."""
	project = disposable.target.project
	if snapshot.project != project:
		raise local_stack_control.models.ControllerError(
			"declared outage snapshot does not match its selected project"
		)
	resources = (*snapshot.containers, *snapshot.volumes, *snapshot.networks)
	if any(resource.project != project for resource in resources):
		raise local_stack_control.models.ControllerError(
			"declared outage snapshot contains a foreign or malformed resource"
		)
	if disposable.owner_policy == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		if (
			disposable.target.project != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
			or disposable.project_prefix != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		):
			raise local_stack_control.models.ControllerError(
				"live-demo browser outage has an invalid fixed project selection"
			)
		local_stack_control.browser_suite_ownership.require_live_demo_browser_ownership(snapshot)


#============================================
def declared_outage_stop_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.ServiceStopPlan:
	"""Select exactly one running policy-declared service from one owned snapshot."""
	require_declared_outage_snapshot(disposable, snapshot)
	service = local_stack_control.disposable_stack_adapter.outage_service(disposable)
	selected = tuple(container for container in snapshot.containers if container.service == service)
	if len(selected) != 1 or not selected[0].running:
		raise local_stack_control.models.ControllerError(
			"declared outage requires exactly one running labelled service instance"
		)
	argv, _environment = local_stack_control.disposable_stack_adapter.outage_stop_command(disposable)
	return local_stack_control.models.ServiceStopPlan(
		project=disposable.target.project,
		service=service,
		argv=tuple(argv),
	)


#============================================
def persistent_scope(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> tuple[tuple[str, ...], tuple[str, ...]]:
	"""Return order-independent exact-project persistent resource identities."""
	result = (
		tuple(sorted(resource.name for resource in snapshot.volumes)),
		tuple(sorted(resource.name for resource in snapshot.networks)),
	)
	return result


#============================================
def unrelated_containers(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> tuple[local_stack_control.models.ContainerResource, ...]:
	"""Return the complete unchanged-container comparison scope outside one service."""
	result = tuple(sorted(
		(container for container in snapshot.containers if container.service != service),
		key=lambda container: container.id,
	))
	return result


#============================================
def require_declared_outage_stopped(
	disposable: local_stack_control.models.DisposableComposeTarget,
	before: local_stack_control.models.ProjectSnapshot,
	after: local_stack_control.models.ProjectSnapshot,
	plan: local_stack_control.models.ServiceStopPlan,
) -> None:
	"""Prove only the selected declared service changed from running to stopped."""
	require_declared_outage_snapshot(disposable, before)
	require_declared_outage_snapshot(disposable, after)
	if plan.project != disposable.target.project or plan.service != local_stack_control.disposable_stack_adapter.outage_service(disposable):
		raise local_stack_control.models.ControllerError(
			"declared outage plan does not match its selected policy"
		)
	expected_argv, _environment = local_stack_control.disposable_stack_adapter.outage_stop_command(disposable)
	if plan.argv != tuple(expected_argv):
		raise local_stack_control.models.ControllerError(
			"declared outage plan does not match its closed command"
		)
	if persistent_scope(before) != persistent_scope(after):
		raise local_stack_control.models.ControllerError(
			"declared outage changed labelled persistent resource scope"
		)
	if unrelated_containers(before, plan.service) != unrelated_containers(after, plan.service):
		raise local_stack_control.models.ControllerError(
			"declared outage changed an unrelated labelled container"
		)
	stopped = tuple(
		container for container in after.containers if container.service == plan.service
	)
	selected_before = tuple(
		container for container in before.containers if container.service == plan.service
	)
	if len(selected_before) != 1 or not selected_before[0].running:
		raise local_stack_control.models.ControllerError(
			"declared outage preselection is not exactly one running labelled service"
		)
	if len(stopped) != 1 or stopped[0].running or stopped[0].id != selected_before[0].id:
		raise local_stack_control.models.ControllerError(
			"declared outage did not leave exactly one labelled service instance stopped"
		)


#============================================
def stop_declared_outage_service(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.DeclaredOutageStop:
	"""Stop one policy-declared service and prove its exact labelled postcondition."""
	before = local_stack_control.disposable_stack_adapter.require_current_resource_capability(
		runner, disposable
	)
	plan = declared_outage_stop_plan(disposable, before)
	environment = local_stack_control.disposable_stack_adapter.compose_environment(disposable)
	result = runner.stream(list(plan.argv), environment, disposable.target.repo_root)
	if result != 0:
		raise local_stack_control.models.ControllerError("declared outage stop command failed")
	after = local_stack_control.disposable_stack_adapter.require_current_resource_capability(
		runner, disposable
	)
	require_declared_outage_stopped(disposable, before, after, plan)
	completed = local_stack_control.models.DeclaredOutageStop(plan.project, plan.service)
	return completed


#============================================
def owned_project_images(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[str, ...]:
	"""Return policy-derived image *tags* removable after an empty resource proof.

	The closed policy owns these generated tags, not the underlying image ID.  A
	plain ``podman image rm <tag>`` consequently leaves a shared image and any
	default-project tag intact.  Callers must prove that the labelled resource
	snapshot is empty before using this result; image cleanup never substitutes
	for Compose cleanup.
	"""
	policy = local_stack_control.disposable_stack_adapter.disposable_policy(disposable)
	images: list[str] = []
	application_image: str | None = None
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		application_image = local_stack_control.disposable_stack_adapter.live_demo_profile_policy(
			disposable
		).application_image
	if application_image is not None:
		images.append(application_image)
	if policy.removes_gateway_image:
		images.append(f"localhost/{disposable.target.project}_gateway:latest")
	return tuple(images)


#============================================
def require_empty_post_cleanup_snapshot(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Fail closed when Compose cleanup leaves any labelled resource behind."""
	remaining = len(snapshot.containers) + len(snapshot.volumes) + len(snapshot.networks)
	if remaining != 0:
		raise local_stack_control.models.ControllerError(
			"disposable cleanup left labelled resources; retained the owned image"
		)


#============================================
def replica_stop_container(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
	id_prefix: str,
) -> local_stack_control.models.ContainerResource:
	"""Resolve one running replica strictly within its typed labelled project."""
	policy = local_stack_control.disposable_stack_adapter.disposable_policy(disposable)
	stoppable_service = None
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		stoppable_service = local_stack_control.disposable_stack_adapter.live_demo_profile_policy(disposable).stoppable_service
	if stoppable_service is None or service != stoppable_service:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot stop the requested service"
		)
	if CONTAINER_ID_PREFIX_PATTERN.fullmatch(id_prefix) is None:
		raise local_stack_control.models.ControllerError(
			"replica container prefix must be twelve lowercase hexadecimal characters"
		)
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"replica snapshot does not match its owned project"
		)
	running = tuple(
		container
		for container in snapshot.containers
		if container.service == service and container.running
	)
	matches = tuple(container for container in running if container.id.startswith(id_prefix))
	if len(running) < 2 or len(matches) != 1:
		raise local_stack_control.models.ControllerError(
			"replica stop requires one matching instance within at least two running API replicas"
		)
	return matches[0]


#============================================
def require_lowercase_uuid(value: str, label: str) -> str:
	"""Require a lowercase UUID before forming a PostgreSQL variable."""
	# ASVS 1.2.4 and 2.2.1: only typed UUID values may reach the fixed SQL statement.
	if not isinstance(value, str):
		raise local_stack_control.models.ControllerError(f"{label} must be a lowercase UUID")
	try:
		parsed = uuid.UUID(value)
	except ValueError as error:
		raise local_stack_control.models.ControllerError(
			f"{label} must be a lowercase UUID"
		) from error
	if str(parsed) != value:
		raise local_stack_control.models.ControllerError(f"{label} must be a lowercase UUID")
	return value


#============================================
def postgresql_count_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	attempt_id: str,
) -> tuple[list[str], dict[str, str], str]:
	"""Form the replica profile's one fixed scoped durability-count query."""
	# ASVS 8.2.2: this adapter exposes only the one oracle-scoped data projection.
	if disposable.owner_policy != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"PostgreSQL count is limited to the fixed replica profile"
		)
	profile = local_stack_control.disposable_stack_adapter.live_demo_profile_policy(disposable)
	if (
		profile.profile is not local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
		or "postgresql_count" not in profile.child_capabilities
	):
		raise local_stack_control.models.ControllerError(
			"PostgreSQL count is limited to the fixed replica profile"
		)
	attempt = require_lowercase_uuid(attempt_id, "PostgreSQL count attempt")
	values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	postgres_user = values.get("POSTGRES_USER")
	postgres_database = values.get("POSTGRES_DB")
	if not postgres_user or not postgres_database:
		raise local_stack_control.models.ControllerError(
			"PostgreSQL count target omits its database selection"
		)
	# ASVS 1.2.4: the attempt identifier remains a psql parameter; SQL identifiers are fixed above.
	sql = "SELECT " + ",".join(
		f"({query})" for query in POSTGRESQL_ATTEMPT_COUNT_QUERIES
	) + ";\n"
	argv = local_stack_control.compose.compose_argv(
		disposable.target,
		[
			"exec",
			"-T",
			"postgres",
			"psql",
			"-v",
			"ON_ERROR_STOP=1",
			"-v",
			f"attempt_id={attempt}",
			"-U",
			postgres_user,
			"-d",
			postgres_database,
			"-tA",
			"-F",
			"|",
		],
	)
	return argv, local_stack_control.disposable_stack_adapter.compose_environment(disposable), sql


#============================================
def require_replica_stopped(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	container_id: str,
) -> None:
	"""Prove the selected replica stopped while another API remains available."""
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"replica post-stop snapshot does not match its owned project"
		)
	selected = tuple(container for container in snapshot.containers if container.id == container_id)
	running_peers = tuple(
		container
		for container in snapshot.containers
		if container.service == "api" and container.running and container.id != container_id
	)
	if len(selected) != 1 or selected[0].running or len(running_peers) < 1:
		raise local_stack_control.models.ControllerError(
			"replica stop did not leave the selected API stopped with a running peer"
		)


#============================================
def compose_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	arguments: list[str],
) -> tuple[list[str], dict[str, str]]:
	"""Form one policy-authorized Compose invocation for a proven target."""
	policy = local_stack_control.disposable_stack_adapter.disposable_policy(disposable)
	local_stack_control.disposable_stack_adapter.require_safe_compose_arguments(arguments)
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		profile = local_stack_control.disposable_stack_adapter.live_demo_profile_policy(disposable)
		if profile.profile is local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE:
			if "database_baseline_oracle" not in profile.child_capabilities:
				raise local_stack_control.models.ControllerError("live-demo profile capability is invalid")
			is_postgres_ready = arguments == [
				"exec", "-T", "postgres", "pg_isready", "-U", "ple_e2e_migrator", "-d", "postgres",
			]
			is_postgres_psql = len(arguments) >= 5 and arguments[:4] == ["exec", "-T", "postgres", "psql"]
			is_migrator_build = arguments == ["--profile", "migration", "build", "database-migrator"]
			is_migrator_initialize = arguments == [
				"--profile", "migration", "run", "--rm", "--no-deps",
				"database-migrator", "database", "initialize",
			]
			if arguments != ["up", "-d", "postgres"] and not (
				is_postgres_ready or is_postgres_psql or is_migrator_build or is_migrator_initialize
			):
				raise local_stack_control.models.ControllerError(
					"database baseline Compose commands are limited to PostgreSQL startup, readiness, psql, and canonical initialization"
				)
			if disposable.acceptance_runtime_workspace is not None:
				local_stack_control.runtime_manifest.require_database_baseline_compose_password(
					disposable.acceptance_runtime_workspace
				)
		elif profile.profile is local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE:
			if "course_appearance_cross_store_oracle" not in profile.child_capabilities:
				raise local_stack_control.models.ControllerError("live-demo profile capability is invalid")
			is_postgres_ready = arguments[:4] == ["exec", "-T", "postgres", "pg_isready"]
			is_minio_ready = arguments == ["exec", "-T", "minio", "mc", "ready", "local"]
			is_postgres_psql = len(arguments) >= 5 and arguments[:4] == ["exec", "-T", "postgres", "psql"]
			is_bucket_initialization = arguments == [
				"--profile", "course-appearance-initialization",
				"run", "--rm", "-T", "createbuckets",
			]
			is_migrator_build = arguments == ["--profile", "migration", "build", "database-migrator"]
			is_migrator_initialize = arguments == [
				"--profile", "migration", "run", "--rm", "--no-deps",
				"database-migrator", "database", "initialize",
			]
			if arguments != ["up", "-d", "postgres", "minio"] and not (
				is_postgres_ready or is_minio_ready or is_postgres_psql or is_bucket_initialization
				or is_migrator_build or is_migrator_initialize
			):
				raise local_stack_control.models.ControllerError(
					"cross-store Compose commands are limited to startup, readiness, bucket creation, PostgreSQL psql, and canonical initialization"
				)
			if disposable.acceptance_runtime_workspace is not None:
				local_stack_control.runtime_manifest.require_course_appearance_cross_store_compose_credentials(
					disposable.acceptance_runtime_workspace
				)
		else:
			raise local_stack_control.models.ControllerError(
				"this fixed live-demo profile cannot use generic Compose commands"
			)
	elif not policy.allows_generic_compose:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot use generic Compose commands"
		)
	argv = local_stack_control.compose.compose_argv(disposable.target, arguments)
	environment = local_stack_control.disposable_stack_adapter.compose_environment(disposable)
	return argv, environment


#============================================
def evidence_log_service(
	disposable: local_stack_control.models.DisposableComposeTarget,
	receipt_claim: str,
) -> str:
	"""Resolve one receipt claim to its policy-owned service without a generic selector."""
	if not isinstance(receipt_claim, str):
		raise local_stack_control.models.ControllerError("evidence receipt claim is invalid")
	policy = local_stack_control.disposable_stack_adapter.disposable_policy(disposable)
	mapping: tuple[tuple[str, str], ...] = ()
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		mapping = local_stack_control.disposable_stack_adapter.live_demo_profile_policy(disposable).evidence_log_services
	claims = tuple(item[0] for item in mapping)
	if len(claims) != len(set(claims)):
		raise local_stack_control.models.ControllerError("evidence receipt policy is invalid")
	for claim, service in mapping:
		if claim == receipt_claim:
			return service
	raise local_stack_control.models.ControllerError(
		"this disposable owner cannot read the requested evidence receipt"
	)


def evidence_log_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	receipt_claim: str,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> tuple[list[str], dict[str, str]]:
	"""Read one claim-selected, label-resolved running container with bounded logs."""
	service = evidence_log_service(disposable, receipt_claim)
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"evidence-log snapshot does not match its selected project"
		)
	selected = tuple(
		container
		for container in snapshot.containers
		if container.service == service and container.running
	)
	if len(selected) != 1:
		raise local_stack_control.models.ControllerError(
			"evidence receipt requires exactly one running labelled service container"
		)
	argv = ["podman", "logs", "--tail", str(EVIDENCE_LOG_TAIL_LINES), selected[0].id]
	return argv, local_stack_control.disposable_stack_adapter.compose_environment(disposable)


#============================================
def cleanup_plan(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.CleanupPlan:
	"""Discover labels and form the only destructive operation this adapter owns."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	plan = local_stack_control.cleanup.disposable_cleanup_plan(
		disposable,
		snapshot,
	)
	return plan


#============================================
def require_mutating_capability(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ProjectSnapshot:
	"""Prove runner capability against all current labels before a mutation."""
	return local_stack_control.disposable_stack_adapter.require_current_resource_capability(
		runner, disposable
	)


#============================================
def require_current_resource_capability(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ProjectSnapshot:
	"""Prove the manifest capability against every current project resource."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)
	return snapshot


#============================================
def require_capability_snapshot(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Require every existing resource to bind the runner capability digest."""
	local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)


#============================================
def diagnostic_services(
	disposable: local_stack_control.models.DisposableComposeTarget,
	services: tuple[str, ...],
) -> tuple[str, ...]:
	"""Validate the bounded service set exposed by replica diagnostics."""
	policy = local_stack_control.disposable_stack_adapter.disposable_policy(disposable)
	if policy.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"diagnostics are not available for this disposable owner"
		)
	allowed = set(local_stack_control.disposable_stack_adapter.live_demo_profile_policy(disposable).diagnostic_services)
	if len(services) == 0 or len(set(services)) != len(services) or not set(services) <= allowed:
		raise local_stack_control.models.ControllerError(
			"replica diagnostics require unique api or gateway services"
		)
	return services


#============================================
def diagnostic_commands(
	disposable: local_stack_control.models.DisposableComposeTarget,
	services: tuple[str, ...],
) -> tuple[list[str], list[str]]:
	"""Form the two bounded replica diagnostic commands without generic authority."""
	selected = diagnostic_services(disposable, services)
	status = local_stack_control.compose.compose_argv(disposable.target, ["ps"])
	logs = local_stack_control.compose.compose_argv(
		disposable.target,
		["logs", "--no-color", "--tail", "80", *selected],
	)
	return status, logs


#============================================
def private_environment_values(env_file: pathlib.Path) -> tuple[str, ...]:
	"""Read only values needed to redact private disposable diagnostics."""
	local_stack_control.env_file.require_mutation_env_file(env_file)
	values: list[str] = []
	for line in env_file.read_text(encoding="utf-8").splitlines():
		stripped = line.strip()
		if stripped == "" or stripped.startswith("#"):
			continue
		value = line.split("=", 1)[1]
		if len(value) >= 4:
			values.append(value)
	return tuple(values)


#============================================
def redact_diagnostics(text: str, private_values: tuple[str, ...]) -> str:
	"""Return bounded diagnostic text without values from the private env file."""
	return redact_private_values(text, private_values)[-4_000:]


#============================================
def redact_evidence_logs(text: str, private_values: tuple[str, ...]) -> str:
	"""Return the bounded evidence log with every private environment value removed."""
	return redact_private_values(text, private_values)[-EVIDENCE_LOG_MAX_CHARACTERS:]


#============================================
def redact_private_values(text: str, private_values: tuple[str, ...]) -> str:
	"""Remove private environment values and credential-bearing PostgreSQL URLs."""
	redacted = text
	for value in sorted(set(private_values), key=len, reverse=True):
		redacted = redacted.replace(value, "[redacted]")
	redacted = re.sub(r"postgres://[^@\s]+@", "postgres://[redacted]@", redacted)
	return redacted
