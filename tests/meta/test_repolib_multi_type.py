"""Behavioral tests for multi-type REPO_TYPE markers (comma-separated, and 'all')."""

import os
import pathlib

import file_utils
import repolib.files
import repolib.model


class TestEffectiveTypeChainUnion:
	"""effective_type_chain() over a comma marker unions and orders single-token chains."""

	def test_union_covers_each_declared_family(self) -> None:
		"""The multi-type chain is a superset of each single-token chain it declares."""
		combined = repolib.model.effective_type_chain('python,rust')
		python_only = repolib.model.effective_type_chain('python')
		rust_only = repolib.model.effective_type_chain('rust')

		assert set(python_only) <= set(combined)
		assert set(rust_only) <= set(combined)

	def test_declaration_order_preserved(self) -> None:
		"""'python' precedes 'rust' when declared in that order."""
		combined = repolib.model.effective_type_chain('python,rust')

		assert combined.index('python') < combined.index('rust')

	def test_dedupe_idempotent(self) -> None:
		"""Repeating the same token in a marker changes nothing."""
		assert repolib.model.effective_type_chain('python,python') == repolib.model.effective_type_chain('python')

	def test_all_covers_every_known_type(self) -> None:
		"""'all' expands to every declared token in KNOWN_REPO_TYPES except 'all' itself."""
		combined = set(repolib.model.effective_type_chain('all'))
		expected_tokens = repolib.model.KNOWN_REPO_TYPES - {repolib.model.LANG_ALL}

		assert expected_tokens <= combined


class TestValidateMarker:
	"""validate_marker() drops unknown tokens but degrades only when nothing is known."""

	def test_typo_preserves_the_valid_half(self) -> None:
		"""A marker with one good and one bad token routes on the good token alone."""
		canonical = repolib.model.validate_marker('python,pyhton', 'test-repo')
		chain = repolib.model.effective_type_chain(canonical)

		assert 'python' in chain
		assert repolib.model.LANG_OTHER not in chain

	def test_wholly_invalid_marker_degrades_to_other(self) -> None:
		"""A marker with no recognized token routes as 'other'."""
		canonical = repolib.model.validate_marker('pyhton', 'test-repo')

		assert canonical == repolib.model.LANG_OTHER


class TestFindSourceForBucketPrecedence:
	"""First-declared-type-wins precedence, proven at actual source resolution."""

	def _build_tree(self, tmp_path: pathlib.Path) -> str:
		# Same relative doc path exists under both a python and a rust overlay.
		root = tmp_path / "template"
		python_docs = root / "templates" / "python" / "docs"
		rust_docs = root / "templates" / "rust" / "docs"
		python_docs.mkdir(parents=True)
		rust_docs.mkdir(parents=True)
		(python_docs / "SHARED.md").write_text('python version\n', encoding='utf-8')
		(rust_docs / "SHARED.md").write_text('rust version\n', encoding='utf-8')
		return str(root)

	def test_python_first_wins(self, tmp_path: pathlib.Path) -> None:
		"""'python,rust' resolves the collision to the python overlay copy."""
		root = self._build_tree(tmp_path)

		source = repolib.model.find_source_for_bucket(root, 'overwrite_files', 'docs/SHARED.md', 'python,rust')

		assert source == os.path.join(root, 'templates', 'python', 'docs', 'SHARED.md')

	def test_rust_first_wins(self, tmp_path: pathlib.Path) -> None:
		"""'rust,python' resolves the same collision to the rust overlay copy."""
		root = self._build_tree(tmp_path)

		source = repolib.model.find_source_for_bucket(root, 'overwrite_files', 'docs/SHARED.md', 'rust,python')

		assert source == os.path.join(root, 'templates', 'rust', 'docs', 'SHARED.md')


class TestGitignoreBlockUnion:
	"""compute_propagation_plan()'s gitignore_block unions every declared type's lines."""

	def test_typed_blocks_union_across_declared_types(self) -> None:
		"""A 'python,rust' plan's gitignore_block is a superset of each single-type block."""
		template_root = file_utils.get_repo_root()

		combined_plan = repolib.files.compute_propagation_plan(template_root, 'python,rust')
		python_plan = repolib.files.compute_propagation_plan(template_root, 'python')
		rust_plan = repolib.files.compute_propagation_plan(template_root, 'rust')

		assert set(python_plan['gitignore_block']) <= set(combined_plan['gitignore_block'])
		assert set(rust_plan['gitignore_block']) <= set(combined_plan['gitignore_block'])

	def test_all_keeps_typed_blocks(self) -> None:
		"""'all' still carries every typed block into gitignore_block.

		This property already held before the multi-type rewrite (the old 'all'
		recursion loaded each child's typed block too); it guards against the
		rewrite losing the union, not against a gap being closed here.
		"""
		template_root = file_utils.get_repo_root()

		all_plan = repolib.files.compute_propagation_plan(template_root, 'all')
		python_plan = repolib.files.compute_propagation_plan(template_root, 'python')

		assert set(python_plan['gitignore_block']) <= set(all_plan['gitignore_block'])


class TestMergeGitignoreBlocksMultiType:
	"""merge_gitignore_blocks() writes one managed block per declared type, stably."""

	def test_writes_both_blocks_and_is_idempotent(self, tmp_path: pathlib.Path) -> None:
		"""A 'python,rust' merge yields both headers, and a second run changes nothing."""
		template_root = file_utils.get_repo_root()
		repo_dir = tmp_path / "repo"
		repo_dir.mkdir()
		context = repolib.model.PropagateContext(
			source_dir=str(repo_dir),
			template_root=template_root,
			repo_name=None,
			dry_run=False,
			initial_setup=False,
			auto_discover=False,
			write_marker=False,
		)

		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python,rust', template_root, context)
		first_content = (repo_dir / ".gitignore").read_bytes()
		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python,rust', template_root, context)
		second_content = (repo_dir / ".gitignore").read_bytes()

		assert b'# === PYTHON ===' in first_content and b'# === RUST ===' in first_content
		assert first_content == second_content

	def test_single_type_output_is_stable(self, tmp_path: pathlib.Path) -> None:
		"""A single-type marker still writes UNIVERSAL plus its one block, idempotently."""
		template_root = file_utils.get_repo_root()
		repo_dir = tmp_path / "repo"
		repo_dir.mkdir()
		context = repolib.model.PropagateContext(
			source_dir=str(repo_dir),
			template_root=template_root,
			repo_name=None,
			dry_run=False,
			initial_setup=False,
			auto_discover=False,
			write_marker=False,
		)

		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python', template_root, context)
		first_content = (repo_dir / ".gitignore").read_bytes()
		repolib.files.merge_gitignore_blocks(str(repo_dir), 'python', template_root, context)
		second_content = (repo_dir / ".gitignore").read_bytes()

		assert b'# === PYTHON ===' in first_content and b'# === RUST ===' not in first_content
		assert first_content == second_content


class TestSpacedBlock:
	"""spaced_block() normalizes a managed block's trailing blank line."""

	def test_collapses_existing_trailing_blanks(self) -> None:
		"""An older block with several trailing blanks converges to exactly one."""
		assert repolib.files.spaced_block(['a', 'b', '', '', '']) == ['a', 'b', '']

	def test_adds_one_trailing_blank(self) -> None:
		"""A block with no trailing blank gains exactly one."""
		assert repolib.files.spaced_block(['a', 'b']) == ['a', 'b', '']
