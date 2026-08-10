# QTI Mapped-Item Contract Implementation

## Status

Complete. The shared Q2 mapped-item, safe-report, and deterministic choice-ID package passed its
focused/full gates and independent P0/P1 re-review on 2026-08-09.

This package adds no Canvas or Blackboard item parser, markup converter, native flat factory,
persistence, schema, route, or UI behavior.

## Accepted boundaries

### Choice identity

`profiles/choice_ids.rs` preserves vendor IDs that already satisfy the PLE flat v1 grammar. All
preserved IDs are reserved before any invalid vendor ID is mapped. Invalid IDs use a
domain-separated SHA-256 over profile ID, item identifier, and vendor identifier, producing a
`qti_` prefix that extends deterministically when it collides. The private ordered map retains raw
vendor IDs but implements neither Debug nor serialization.

### Instructor-safe report

`profiles/report.rs` exposes only closed diagnostic codes, structural locations, and static detail
templates. A profile parser cannot interpolate an answer, vendor identifier, raw XML, object key,
archive identity, or digest into the serializable report. Accepted and rejected items are both
representable; a rejected item may have no title. Unicode-scalar limits match flat v1 for title,
prompt, and choice text.

Canvas v1 accepts only finite nonnegative declared points. Blackboard v1 accepts only its explicit
defaulted 1.0 points path and records the corresponding safe warning. Signed zero canonicalizes to
`0.0`.

### Private mapped item and integrity ownership

`profiles/mapped_item.rs` validates exact choice count and PLE IDs, unique vendor IDs, pairwise
public/private choice order, one correct binding, source/title/prompt bounds, points policy, and
closed defaults. Construction is crate-private for the profile parsers. The mapped item and its
server parts implement neither Debug nor serialization.

Detached public and combined digest computation is crate-private. The mapped item creates its own
accepted disposition and computes integrity digests only when the report has the same profile,
profile version, and mapping version. All digest values are redacted from Debug output.

## Ownership and size

- `profiles/choice_ids.rs`: 349 lines;
- `profiles/report.rs`: 305 lines;
- `profiles/mapped_item.rs`: 393 production lines;
- `profiles/mapped_item/tests.rs`: 279 lines;
- `profiles/server_parts.rs`: 48 lines.

No owner exceeds the 600-line target.

## Validation evidence

- `cargo test -p adapter_qti`: 38 unit tests, 6 fixture integration tests, and 5 compile-fail
  doctests passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked, staged, and new-file whitespace checks passed.
- Independent re-review reported PASS with no P0/P1 finding.

## Historical successor

The immediate successor was the shared strict markup projector over the ordered XML stream. Canvas
escaped HTML and Blackboard direct XML used separate entry points, one allowlisted CommonMark
projection, explicit input/token/nesting/output limits, and refusal for unknown markup, attributes,
comments, processing instructions, media, links, tables, styles, SVG, and MathML.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
