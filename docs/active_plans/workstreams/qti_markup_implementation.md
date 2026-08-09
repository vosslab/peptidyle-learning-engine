# QTI Strict Markup Implementation

## Status

Complete. The shared Q2 markup projector passed focused/full gates and independent P0/P1 re-review
on 2026-08-09.

This package adds no Canvas or Blackboard structural item parser, mapped-item policy change, native
flat factory, Store/schema behavior, route, or UI.

## Accepted boundary

### Canvas

Canvas `mattext` may contain an escaped HTML fragment. The XML layer has already decoded ordinary XML
text once; CDATA remains raw. The projector joins only ordered text/CDATA under a cumulative byte
limit, then uses the `html5ever` 0.39 tokenizer for exactly one HTML entity/token layer. It does not
use the forgiving HTML DOM/tree builder. XML children, comments, processing instructions, malformed
or recovery-dependent HTML, duplicate attributes, and unbalanced structures refuse.

### Blackboard

Blackboard content follows the ordered XML tree directly and is never reparsed as HTML. Elements
must remain in the exact QTI item or XHTML namespace. The container may carry only its matching
default XMLNS declaration; descendants may carry no attributes or namespace changes. Comments,
processing instructions, foreign/unresolved namespaces, and invalid ordered-child references refuse.

### Shared projection

The allowlist is `p`, `div`, `br`, `strong`, `b`, `em`, `i`, `code`, `ul`, `ol`, and `li`. Links,
tables, styles, classes/IDs/events, `u`, `sub`, `sup`, images, audio, video, SVG, MathML, and every
unknown element refuse rather than being stripped.

Accepted markup emits escaped deterministic CommonMark: block separation, hard line breaks, strong/
emphasis, bounded inline-code fences, unordered list markers, and source-order numbered markers.
Pretty-printed list/block indentation does not change output, leading whitespace cannot create an
accidental CommonMark code block, and semantic spacing between inline siblings remains one space.

`MarkupLimits` bounds input bytes, retained tokens, nesting, and Unicode output characters. The HTML
sink never retains more than its token budget. Canvas input is checked before every append, and the
renderer writes through a character-counting bounded writer so entity escaping, recursion, lists,
code fences, and blocks cannot create oversized intermediate strings.

## Dependency and ownership

- workspace `html5ever >=0.39.0`, enabled only by `adapter_qti`;
- `profiles/markup.rs`: 479 lines;
- `profiles/markup/renderer.rs`: 264 lines;
- `profiles/markup/tests.rs`: 270 lines.

The dependency is used only through its tokenizer API. The crate reports MIT OR Apache-2.0 licensing
and requires Rust 1.71, below the workspace Rust 1.96 baseline. No Wasm/browser dependency edge was
added. The private module has one temporary, narrowly documented dead-code allowance until the next
Canvas/Blackboard parser package consumes its entry points.

## Validation evidence

- `cargo test -p adapter_qti`: 46 unit tests, 6 fixture integration tests, and 5 compile-fail
  doctests passed.
- `cargo clippy -p adapter_qti --all-targets --all-features -- -D warnings` passed.
- `cargo fmt --check` passed.
- `python3 -m pytest -q tests/test_crate_boundaries.py`: 5 passed.
- Tracked and staged whitespace checks passed.
- Independent re-review reported PASS with no P0/P1 finding.

## Next package

Implement one exact vendor structural parser at a time, beginning with Canvas QTI 1.2. It must reuse
the bounded archive/XML/markup/choice/mapped-item contracts and retain per-item refusal rather than
widening this shared projector.
