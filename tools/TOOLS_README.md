# tools scripts

`tools/` holds optional standalone utilities for people using the repository's
product, content, or data. Each command performs a direct domain task with an
understandable input and useful result. For repository engineering commands,
see [devel/DEVEL_README.md](../devel/DEVEL_README.md).

Use this folder for commands a regular user can run directly:

- Convert, validate, inspect, or transform user-supplied domain data.
- Produce a user-facing report, document, image, or other useful artifact.
- Provide an optional task beside the application's primary command.

Put primary product workflows in the application CLI. Put reusable behavior in
an explicit importable package, and keep the `tools/` file as a thin entry point.

## Placement test

Classify a script by its audience and its input/output boundary:

- A regular user supplies domain input and receives a domain result: `tools/`.
- A maintainer supplies repository source, Git state, manifests, tests, or internal fixtures and
  receives a build, release, generated source, benchmark, capture, or diagnostic: `devel/`.
- Several scripts or tests import the behavior: an importable package, with thin entry points in
  the audience-appropriate folder.

For example, HTML-to-PDF conversion and playlist validation are tools. Graphify repository maps,
dependency-pin refresh, source generation, screenshot evidence, benchmarks, and release commands
are developer work.

## Import boundary

Use `tools/` and `devel/` as entry-point directories and `tests/` for test support.
Import reusable behavior from a real package. The support-directory gate enforces
these roles across package imports and tool-to-tool imports.

The related folders have narrow, intentional exceptions:

- Vendored `devel/` tooling may use flat sibling helpers.
- Tests may use flat, same-directory helpers such as `file_utils`, because
  `tests/conftest.py` and pytest arrange that directory.

## Package layout

A large tool may delegate to an importable helper package while its command stays
as a thin `tools/` entry point. Place the repository's only native helper package
in a named folder at the repository root. Use `packages/` to group multiple native
products or packages when the extra layer provides real separation.

## Migration direction

The support-directory gate reports violations. The consumer repository's
maintainer applies the package extraction and entry-point placement in that
repository.

Move reusable behavior into the consumer's real package and leave a thin entry
point in the audience-appropriate support folder. A test that must exercise a
standalone script may instead load it by file path, as
`protein-image-grader/tests/test_copy_archive_images.py` does, keeping the
script as a standalone entry point.

The completed audit identifies these original plan-audited consumers and their
migration targets:

| Consumer repository | Module that moves into its package |
| --- | --- |
| `populous-python-nvl` | `tools.headless_runner` into `populous_game/` |
| `iptv-filters` | `tools.validate_m3u` into `iptv_filters/` |
| `marp-slides` | `tools.pptx_to_marp` into `marp_lib/` |
| `track-runner-virtual-dolly-cam` | `tools.refresh_mode_docs` into `track_runner/` |

The original four reports remain test importers, but the completed survey found
runtime drift too: `populous-python-nvl` imports `tools.headless_runner` from
runtime and smoke code, while `marp-slides` tool scripts import
`odp_to_marp`, `odp_visibility`, and `pptx_to_marp` from each other. Those
findings use the same package-plus-thin-script migration and preserve the
support-directory boundary.

## Running scripts

For Python scripts, use the repo bootstrap environment:

```bash
source source_me.sh && python3 tools/<script>.py
```

Run individual scripts with `--help` for current options. Keep command details
in script help output instead of duplicating them here.
