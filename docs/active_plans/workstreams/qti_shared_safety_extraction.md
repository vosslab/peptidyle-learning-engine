# QTI Shared Safety Extraction

## Status

Complete. The behavior-preserving archive/XML extraction passed focused gates and independent P0/P1
review on 2026-08-09.

This package establishes the shared hostile-input boundary required by the Canvas and Blackboard
parsers. It adds no vendor grammar, mapping, persistence, route, schema, or UI behavior.

## Ownership

- `crates/adapters/qti/src/archive.rs` owns bounded ZIP reading: archive, entry, file, and expanded
  size limits; normalized relative paths; enclosure and symlink refusal; duplicate rejection; and
  bounded actual reads.
- `crates/adapters/qti/src/xml.rs` owns the bounded XML tree: UTF-8, DTD/entity refusal, token/node/
  depth limits, duplicate attributes, pending tags, balanced names, a single root, and text
  normalization.
- `crates/adapters/qti/src/parser.rs` retains the existing generic grammar and QTI compatibility
  behavior. It supplies its unchanged `imsmanifest.xml`, `items/`, and `assets/` allowlist to the
  shared archive reader.

The shared archive result is an opaque crate-private type with read-only `get` and `paths` methods.
Profile-specific predicates run only after the non-bypassable core path, enclosure, and link checks.
The XML node and traversal APIs are also crate-private. No browser, persistence, or public adapter
surface can construct a validated archive or XML tree.

## Size result

- `archive.rs`: 211 lines;
- `xml.rs`: 269 lines;
- generic `parser.rs`: 613 lines, reduced from 976.

The parent remains below the 1,000-line hard owner limit and will not absorb vendor parsers. A later
asset-specific split may take it below the 600-line target without coupling that cleanup to profile
behavior.

## Validation evidence

- `cargo test -p adapter_qti`: 23 unit tests, 6 fixture integration tests, and one compile-fail
  doctest passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked, staged, and new-file whitespace checks passed.
- Independent review reported PASS with no P0/P1 finding.

## Historical successor

The immediate successors were the Canvas and Blackboard parsers. They used the same bounded
archive/XML foundation with separate exact entry grammars and kept vendor rules in separate profile
modules without widening the generic importer allowlist.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
