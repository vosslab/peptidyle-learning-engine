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
- `crates/adapters/qti/src/parser_stub.rs` retains the existing generic grammar and QTI compatibility
  behavior. It supplies its unchanged `imsmanifest.xml`, `items/`, and `assets/` allowlist to the
  shared archive reader.

The shared archive result is an opaque crate-private type with read-only `get` and `paths` methods.
Profile-specific predicates run only after the non-bypassable core path, enclosure, and link checks.
The XML node and traversal APIs are also crate-private. No browser, persistence, or public adapter
surface can construct a validated archive or XML tree.

## Size result

- `archive.rs`: 211 lines;
- `xml.rs`: 269 lines;
- generic `parser_stub.rs`: 613 lines, reduced from 976.

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

## Next package

Canvas and Blackboard parsers may now use the same bounded archive/XML foundation with separate
exact entry grammars. They must keep their vendor rules in separate profile modules and must not
widen the generic importer allowlist.
