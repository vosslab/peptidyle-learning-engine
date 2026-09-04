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
| [graphify_context_lib.py](graphify_context_lib.py) | Load artifacts and format orientation. |
| [graphify_docs_lib.py](graphify_docs_lib.py) | Render a browsable repository map. |
| [graphify_prune_tests.py](graphify_prune_tests.py) | Remove Rust tests before clustering. |
| [graphify_clean_svg.py](graphify_clean_svg.py) | Shrink an exported SVG figure. |

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

## Repository mapping with Graphify

[graphify_map_repo.py](graphify_map_repo.py) builds a queryable map of this
repository and writes agent orientation to `graphify-out/MANAGER_CONTEXT.md`.
Read that file before exploring an unfamiliar repository: it names the major
areas, the architectural hubs, the cross-area connectors, and the map size.

Build or refresh the map, then read the orientation:

```bash
source source_me.sh && python3 devel/graphify_map_repo.py
source source_me.sh && python3 devel/graphify_map_repo.py --context
```

Force a full refresh only when needed. Use local Ollama when the Claude allowance is exhausted:

```bash
source source_me.sh && python3 devel/graphify_map_repo.py --fresh
source source_me.sh && python3 devel/graphify_map_repo.py --fresh --ollama
```

Prefer targeted Graphify traversal over a broad repository sweep:

```bash
graphify query "<question>" --budget 1500
graphify explain "<symbol_or_path>"
graphify affected "<symbol_or_path>" --depth 2
```

### Cleaned map SVG

`--svg` writes the lightweight `docs/GRAPHIFY_map.svg` from an existing map:

```bash
source source_me.sh && python3 devel/graphify_map_repo.py --svg
```

The wrapper leaves Graphify's full export in generated `graphify-out/` and copies
only the cleaned SVG to `docs/`. The cleaner strips unreadable per-symbol labels
but preserves the community legend, so the result shows cluster shape and scale
rather than source-level detail. Graphify renders the export with matplotlib,
which is optional; an unavailable export leaves no SVG output.

`graphify-out/` is generated output and stays out of Git. The cleaned SVG
describes the repository where it was generated, so it is never shared between
repositories. Scope comes from `.graphifyignore`.

### Rust test symbols

Graphify's Rust extractor indexes `#[cfg(test)] mod tests` contents as
production symbols. Because those modules live inside `src/*.rs`, no ignore rule
can exclude them without dropping the production code beside them.

In a repository with a `Cargo.toml`, a fresh build therefore extracts without
clustering, removes those symbols from `graph.json`, and clusters what remains,
so community detection and hub ranking never see the test suite. The run reports
how many nodes and links it removed.

Incremental updates do not prune, because re-clustering renumbers communities
and would strand the stored labels. Orientation still filters test symbols out
of what it prints, which covers updates and other languages' inline test
conventions.

## Running scripts

For Python scripts, use the repo bootstrap environment:

```bash
source source_me.sh && python3 devel/<script>.py
```

Run individual scripts with `--help` for current options. Keep command details
in script help output instead of duplicating them here.
