"""Image cleanup after a healthy ordinary local-stack replacement."""

import json
import pathlib
import re

import local_stack_control.models
import local_stack_control.process


#============================================
def _decode_json_array(
	result: local_stack_control.models.CommandResult,
	label: str,
) -> list[object]:
	"""Decode one Podman JSON array or fail the ready lifecycle closed."""
	if not result.ok():
		raise local_stack_control.models.ControllerError(
			f"{label} failed after stack readiness"
		)
	try:
		decoded = json.loads(result.stdout)
	except json.JSONDecodeError as error:
		raise local_stack_control.models.ControllerError(
			f"{label} returned invalid JSON after stack readiness"
		) from error
	if not isinstance(decoded, list):
		raise local_stack_control.models.ControllerError(
			f"{label} returned an unexpected payload after stack readiness"
		)
	return decoded


#============================================
def _active_container_image_references(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> set[str]:
	"""Return exact image references named by current or stopped containers."""
	result = runner.run(
		["podman", "ps", "--all", "--format", "json"],
		local_stack_control.process.current_environment(),
		repo_root,
	)
	entries = _decode_json_array(result, "container image inspection")
	references: set[str] = set()
	for entry in entries:
		if not isinstance(entry, dict) or not isinstance(entry.get("Image"), str):
			raise local_stack_control.models.ControllerError(
				"container image inspection returned an unexpected entry"
			)
		references.add(entry["Image"])
	return references


#============================================
def _known_image_references(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> set[str]:
	"""Return every named image reference reported by Podman."""
	result = runner.run(
		["podman", "images", "--all", "--format", "json"],
		local_stack_control.process.current_environment(),
		repo_root,
	)
	entries = _decode_json_array(result, "image-tag inspection")
	references: set[str] = set()
	for entry in entries:
		if not isinstance(entry, dict):
			raise local_stack_control.models.ControllerError(
				"image-tag inspection returned an unexpected entry"
			)
		names = entry.get("Names")
		if names is None:
			continue
		if not isinstance(names, list):
			raise local_stack_control.models.ControllerError(
				"image-tag inspection returned unexpected names"
			)
		for reference in names:
			if not isinstance(reference, str):
				raise local_stack_control.models.ControllerError(
					"image-tag inspection returned an unexpected name"
				)
			references.add(reference)
	return references


#============================================
def _disposable_project_image(reference: str) -> bool:
	"""Return whether a tag belongs to one declared disposable project."""
	project: str | None = None
	gateway_match = re.fullmatch(r"localhost/(.+)_gateway:latest", reference)
	if gateway_match is not None:
		project = gateway_match.group(1)
	else:
		application_match = re.fullmatch(
			r"localhost/peptidyle-learning-engine:(.+)", reference
		)
		if application_match is not None:
			project = application_match.group(1)
	if project is None:
		return False
	return any(
		policy.project_pattern.fullmatch(project) is not None
		for policy in local_stack_control.models.DISPOSABLE_OWNER_POLICIES
	)


#============================================
def prune_superseded_images(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> None:
	"""Remove inactive disposable tags, then every unused image."""
	active_references = _active_container_image_references(runner, repo_root)
	known_references = _known_image_references(runner, repo_root)
	stale_references = sorted(
		reference
		for reference in known_references
		if _disposable_project_image(reference)
		and reference not in active_references
	)
	for reference in stale_references:
		result = runner.run(
			["podman", "image", "rm", reference],
			local_stack_control.process.current_environment(),
			repo_root,
		)
		if not result.ok():
			raise local_stack_control.models.ControllerError(
				"disposable image-tag cleanup failed after stack readiness"
			)
	result = runner.run(
		["podman", "image", "prune", "--all", "--force"],
		local_stack_control.process.current_environment(),
		repo_root,
	)
	if not result.ok():
		raise local_stack_control.models.ControllerError(
			"unused-image cleanup failed after stack readiness"
		)
