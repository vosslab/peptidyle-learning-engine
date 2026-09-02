# devel scripts

`devel/` holds engineering commands for highly technical maintainers working on
the repository itself. These commands may require source-tree knowledge, Git,
development dependencies, or internal fixtures. For regular-user utilities, see
[tools/TOOLS_README.md](../tools/TOOLS_README.md).

Use this folder for repository lifecycle and engineering work:

- Git, version, release, and changelog maintenance.
- Dependency refresh, environment setup, builds, packaging, and source generation.
- Lint, benchmark, probe, diagnostic, screenshot, and engineering-evidence commands.
- Documentation repair, repository hygiene, and developer helpers shared by propagation.

Put regular-user domain utilities in `tools/`, primary workflows in the
application CLI, shared test helpers in `tests/`, and reusable behavior in an
importable package.

## Placement test

Ask what the command consumes and produces. Repository source, Git state,
manifests, internal fixtures, builds, releases, generated source, benchmarks,
captures, and diagnostics indicate `devel/`. User-supplied domain data and a
directly useful domain result indicate `tools/`.

## Import boundary

Use `tools/`, `devel/`, and `tests/` as entry-point or test-support directories.
Import reusable behavior from a real package. Vendored `devel/` tooling may use
flat sibling helpers such as `changelog_lib`, `version_lib`, and `version_files`.
Place one native helper package in a named root-level folder; use `packages/` to
group multiple native products or packages. The support-directory gate enforces
these roles. See
[tools/TOOLS_README.md](../tools/TOOLS_README.md) for the full boundary and
consumer migration direction.

## Current root scripts

| File | Kind of work |
| --- | --- |
| [bump_version.py](bump_version.py) | Preview and save repo version changes; enter `patch` for the next patch release. |
| [version_lib.py](version_lib.py) | Shared version parsing and normalization behavior. |
| [version_files.py](version_files.py) | Discover and update files that carry version metadata. |
| [changelog_lib.py](changelog_lib.py) | Shared parser and helpers for changelog tools. |
| [commit_changelog.py](commit_changelog.py) | Draft a commit message from new changelog entries. |
| [query_changelog.py](query_changelog.py) | Search active and archived changelog entries. |
| [rotate_changelog.py](rotate_changelog.py) | Move old changelog day blocks into archive files. |
| [flatten_broken_md_links.py](flatten_broken_md_links.py) | Repair or flatten broken Markdown links. |
| [dist_clean.sh](dist_clean.sh) | Remove build artifacts, caches, and dependency installs. |
| [graphify_map_repo.py](graphify_map_repo.py) | Build repository maps and manager orientation for technical maintenance. |

## Propagated devel scripts

Some developer tools arrive by propagation and appear in `devel/` when this repo's
`REPO_TYPE` calls for them.

`devel/make_release.py` ships to the `scripted`, `compiled`, and `other` families, including
their descendants (`python`, `pypi`, `rust`, and `swift`). It prepares a GitHub source release:
CalVer freshness check, free-tag check, committed `LICENSE.<SPDX>` verification,
zip and tgz archive build with byte-level checks of every license, LLM-prompt generation for
the release description, optional `docs/RELEASE_HISTORY.md` and `docs/NEWS.md` updates,
and printed `git tag` + `gh release create` commands. Use `--dry-run` to preview or
`--write` to update doc files. See [docs/REPO_STYLE.md](../docs/REPO_STYLE.md) versioning
section for the full flow.

Other propagated devel tools are type-specific, so a repo receives only the ones
matching its `REPO_TYPE`. Examples include Python release publishing helpers and
TypeScript setup/rendering helpers.

## Running scripts

For Python scripts, use the repo bootstrap environment:

```bash
source source_me.sh && python3 devel/<script>.py
```

Run individual scripts with `--help` for current options. Keep command details
in script help output instead of duplicating them here.
