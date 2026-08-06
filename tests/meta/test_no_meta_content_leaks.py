"""CONTENT leak detector for shipped Markdown.

test_no_meta_leaks.py protects ROUTING: which files ship. This module protects
CONTENT: what a legitimately-shipped Markdown file's prose talks about. A doc
that ships to every consumer must not instruct those consumers to look at a path
only this template repo has (for example `tests/meta/e2e/run_all.sh`).

Scope is plan-driven, not directory-driven: the union of every `.md` file that
compute_propagation_plan() reports for any repo type, resolved back to its
template source. A doc that stops shipping stops being checked, and a newly
shipped doc is covered with no edit here.

FORBIDDEN VOCABULARY -- derived from repolib.model, then pruned. The pruning is
deliberate; do not "helpfully" re-add an excluded entry.

META_DIRS entries EXCLUDED from the content check:
  __pycache__, .git       Generic to every repo. A shipped doc explaining what a
                          clean-up script removes names them legitimately.
  tools                   Ambiguous. ROOT tools/ is template infrastructure, but
                          templates/<type>/tools/ ships to a consumer's tools/,
                          so consumers DO have tools/ and may be told about it
                          (test_no_meta_leaks.py documents the same nuance in
                          TYPED_OVERLAY_ALLOWED_META_DIRS). Resolved by
                          forbidding the specific this-repo-only file
                          tools/detect_repo_type.py while allowing bare tools/.
  docs/active_plans,      Meta for ROUTING only: this repo's own plan files must
  docs/archive            not ship. The FOLDER CONVENTION is consumer-facing and
                          is documented on purpose in docs/REPO_STYLE.md, so
                          naming these paths in prose is correct.

META_FILES is NOT reusable as a content vocabulary. Most of it (README.md,
VERSION, .gitignore, REPO_TYPE, docs/CHANGELOG.md) is "meta" only in the routing
sense -- the propagator must not overwrite a consumer's copy -- and every
consumer HAS those files, so mentioning them in prose is legitimate. Only the
two entries that are genuinely this-repo-only are forbidden here:
propagate_style_guides.py and reset_repo.py.
"""

import os
import re

import pytest

import file_utils
import repolib.files
import repolib.model

TEMPLATE_ROOT = file_utils.get_repo_root()

# Every consumer type plus the 'unknown' pseudo-type, matching test_no_meta_leaks.py.
REPO_TYPES = repolib.model.REPO_TYPE_ORDER + ('unknown',)

# Plan buckets whose entries name a shipped file.
PLAN_BUCKETS = ('overwrite_files', 'noexist_files', 'merge_files', 'test_files', 'devel_files')

# META_DIRS entries that are generic or consumer-facing in prose (see module docstring).
CONTENT_ALLOWED_META_DIRS = frozenset({
	'__pycache__',
	'.git',
	'tools',
	'docs/active_plans',
	'docs/archive',
})

# The two META_FILES entries that exist only in this template repo.
FORBIDDEN_META_FILENAMES = ('propagate_style_guides.py', 'reset_repo.py')

# Template infrastructure named by path rather than by directory, since bare
# tools/ is allowed above but this specific file ships nowhere.
FORBIDDEN_META_PATHS = ('tools/detect_repo_type.py',)

# No file is exempt. This check found that docs/TODO.md shipped to every
# consumer, overwriting each repo's own backlog with this template's; that was
# settled as a ROUTING fix (docs/TODO.md is now in meta_files) rather than an
# exemption here. Keep this set empty: a content leak in a shipped doc is a bug
# in the doc or in its routing, and an exemption would hide both.
CONTENT_CHECK_EXEMPT: frozenset[str] = frozenset()


def forbidden_patterns() -> list[tuple[str, re.Pattern]]:
	"""
	Build the (token, compiled pattern) list for the content check.

	The directory half is derived from repolib.model.META_DIRS minus the pruned
	set, so adding a new meta directory to the manifest extends this check with
	no edit here. Each directory token must appear as a path segment (trailing
	slash required), which keeps prose words like 'metadata' from matching.

	Returns:
		list[tuple[str, re.Pattern]]: Human-readable token and its matcher.
	"""
	# Left boundary rejects word characters and hyphens, so 'template-meta/' and
	# 'submeta/' do not match while 'tests/meta/' does.
	boundary = r'(?<![\w-])'
	patterns = []
	for meta_dir in sorted(repolib.model.META_DIRS):
		if meta_dir in CONTENT_ALLOWED_META_DIRS:
			continue
		token = f'{meta_dir}/'
		patterns.append((token, re.compile(boundary + re.escape(token))))
	for meta_name in FORBIDDEN_META_FILENAMES + FORBIDDEN_META_PATHS:
		patterns.append((meta_name, re.compile(boundary + re.escape(meta_name))))
	return patterns


PATTERNS = forbidden_patterns()


def find_meta_mentions(text: str) -> list[tuple[int, str]]:
	"""
	Report every meta-path mention in a block of Markdown text.

	Args:
		text (str): Full file content to scan.

	Returns:
		list[tuple[int, str]]: (line number starting at 1, offending token) pairs.
	"""
	# One pass per line so the report can name the exact line number.
	mentions = []
	for line_number, line in enumerate(text.splitlines(), start=1):
		for token, pattern in PATTERNS:
			if pattern.search(line):
				mentions.append((line_number, token))
	return mentions


def collect_shipped_markdown() -> list[str]:
	"""
	Collect the template-relative source path of every shipped Markdown file.

	Takes the union across every repo type's propagation plan, then resolves each
	plan entry back to the template source it was copied from, so the CONTENT of
	the source file is what gets scanned.

	Returns:
		list[str]: Sorted template-root-relative paths of shipped .md sources.
	"""
	# Union across repo types; a doc shipped to several types is scanned once.
	sources = set()
	for repo_type in REPO_TYPES:
		plan = repolib.files.compute_propagation_plan(TEMPLATE_ROOT, repo_type)
		for bucket in PLAN_BUCKETS:
			for entry in plan.get(bucket, []):
				if not entry.endswith('.md'):
					continue
				source = repolib.model.find_source_for_bucket(
					TEMPLATE_ROOT, bucket, entry, repo_type
				)
				if source is None:
					continue
				source_rel = os.path.relpath(source, TEMPLATE_ROOT)
				if source_rel in CONTENT_CHECK_EXEMPT:
					continue
				sources.add(source_rel)
	return sorted(sources)


SHIPPED_MARKDOWN = collect_shipped_markdown()


@pytest.mark.parametrize('source_rel', SHIPPED_MARKDOWN, ids=SHIPPED_MARKDOWN)
def test_shipped_markdown_names_no_meta_path(source_rel: str) -> None:
	"""A shipped Markdown file must not point consumers at a template-meta path."""
	with open(os.path.join(TEMPLATE_ROOT, source_rel), encoding='utf-8') as handle:
		text = handle.read()
	mentions = find_meta_mentions(text)
	# Name file, line, and token so the fix needs no follow-up grep.
	detail = '; '.join(f'{source_rel}:{line} names {token!r}' for line, token in mentions)
	assert not mentions, (
		f"META content leak in shipped doc: {detail}. "
		"Restate the rule from the consumer's point of view, at the path the consumer has."
	)


def test_checker_catches_synthetic_violation() -> None:
	"""The checker must actually fire on a meta path, not silently match nothing."""
	mentions = find_meta_mentions('run the harness:\nbash tests/meta/foo.sh\n')
	assert (2, 'meta/') in mentions


def test_checker_allows_consumer_facing_prose() -> None:
	"""Legitimate consumer prose (README.md, tools/, docs/archive/) must not fire."""
	text = 'See README.md, run tools/build.py, close plans into docs/archive/.\n'
	assert find_meta_mentions(text) == []


def test_checker_ignores_word_prefixed_matches() -> None:
	"""A hyphenated or word-joined lookalike is not a path segment."""
	assert find_meta_mentions('the template-meta/ idea and metadata handling\n') == []


def test_shipped_markdown_set_is_not_empty() -> None:
	"""A plan that resolves no Markdown would make every case above vacuous."""
	assert SHIPPED_MARKDOWN
