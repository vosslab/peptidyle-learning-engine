"""Offline behavior checks for selected renderer image preparation."""

import pathlib

import local_stack_control.models
import local_stack_control.process
import local_stack_control.renderer


class RendererImageRunner(local_stack_control.process.CommandRunner):
	"""Record the closed image operations exposed by renderer preparation."""

	def __init__(self, exists: bool) -> None:
		"""Store the selected initial image state."""
		self.exists = exists
		self.streamed: list[tuple[str, ...]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Answer only image-existence and identity observations."""
		del environment, cwd, stdin
		if argv[:3] == ["podman", "image", "exists"]:
			return local_stack_control.models.CommandResult(
				tuple(argv), 0 if self.exists else 1, "", ""
			)
		if argv[:3] == ["podman", "image", "inspect"]:
			return local_stack_control.models.CommandResult(
				tuple(argv), 0 if self.exists else 1, "a" * 64 if self.exists else "", ""
			)
		raise AssertionError(f"unexpected renderer image command: {argv}")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Record one successful build or immutable pull."""
		del environment, cwd
		self.streamed.append(tuple(argv))
		self.exists = True
		return 0


#============================================
def test_missing_selected_renderer_builds_from_the_maintained_sibling(
	tmp_path: pathlib.Path,
) -> None:
	"""Build intent reconstructs the selected local runtime image after pruning."""
	repo_root = tmp_path / "peptidyle-learning-engine"
	source = tmp_path / "webwork-pg-renderer"
	repo_root.mkdir()
	source.mkdir()
	(source / "Dockerfile").write_text("FROM scratch\n", encoding="ascii")
	runner = RendererImageRunner(False)

	identity = local_stack_control.renderer.ensure_renderer_oci_id(
		runner, repo_root, local_stack_control.renderer.LOCAL_REVIEWED_REFERENCE, {}, True
	)

	assert identity == "sha256:" + "a" * 64
	assert runner.streamed == [(
		"podman", "build", "--tag", "localhost/pg-renderer:reviewed", "--file",
		str(source / "Dockerfile"), str(source),
	)]


#============================================
def test_existing_renderer_is_reused_without_rebuilding(tmp_path: pathlib.Path) -> None:
	"""A selected existing image proceeds directly to immutable identity inspection."""
	runner = RendererImageRunner(True)
	identity = local_stack_control.renderer.ensure_renderer_oci_id(
		runner, tmp_path, local_stack_control.renderer.LOCAL_REVIEWED_REFERENCE, {}, True
	)
	assert identity == "sha256:" + "a" * 64
	assert runner.streamed == []


#============================================
def test_missing_immutable_renderer_is_pulled_by_digest(tmp_path: pathlib.Path) -> None:
	"""Build intent obtains a published renderer only through its immutable reference."""
	reference = "registry.example/renderer@sha256:" + "b" * 64
	runner = RendererImageRunner(False)
	local_stack_control.renderer.ensure_renderer_oci_id(
		runner, tmp_path, reference, {}, True
	)
	assert runner.streamed == [("podman", "pull", reference)]
