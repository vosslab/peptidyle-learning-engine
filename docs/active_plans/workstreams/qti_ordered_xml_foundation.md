# QTI Ordered XML Foundation

## Status

Complete. The additive ordered-content and namespace foundation passed focused/full adapter gates
and independent P0/P1 review on 2026-08-09.

This package prepares the shared hostile-input XML tree for exact Canvas and Blackboard item
parsers. It adds no vendor grammar, item mapping, persistence, schema, route, or UI behavior.

## Contract

The crate-private XML tree now retains two complementary views:

- the existing aggregate text and child tree used by the generic compatibility importer; and
- an ordered borrowed content stream for profile parsers, preserving text, raw CDATA, child
  positions, comments, and processing instructions.

Elements and attributes retain their prefixes and resolved namespace URIs. Default and prefixed
namespace declarations are inherited and may be lexically shadowed. Unprefixed attributes remain
outside the default namespace, and unresolved prefixes remain explicit with no namespace binding so
a vendor profile can refuse them. Namespace declarations use the XMLNS namespace rather than being
mistaken for ordinary item attributes.

The new ordered path does not change the generic importer's historical text normalization. Ordinary
XML text is decoded once, CDATA remains distinguishable in the ordered stream, and the legacy
aggregate text behavior remains locked by an exact public-output regression test.

## Ownership and size

- `crates/adapters/qti/src/xml.rs`: 552 lines, including the bounded parser and focused tests;
- `crates/adapters/qti/src/parser/tests.rs`: one exact generic-output regression.

All new accessors and content types are crate-private. Future Canvas and Blackboard parsers must use
them through their focused profile modules; no browser or persistence contract can construct or
serialize the XML tree.

## Validation evidence

- `cargo test -p adapter_qti`: 26 unit tests, 6 fixture integration tests, and one compile-fail
  doctest passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked, staged, and new-file whitespace checks passed.
- Independent review reported PASS with no P0/P1 finding.

## Historical successor

The immediate successor added the server-only mapped-item capability, safe per-item report, and
deterministic vendor-to-PLE choice-ID mapping before markup conversion and vendor parsers. Private
answer and identifier mappings remained non-serializable and non-debuggable.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
