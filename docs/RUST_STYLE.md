# Rust style

Language model and human guide to Rust in this repo. It applies to every `.rs` file
and every crate here.

Rust style here means: let the toolchain settle formatting, use the type system to
make invalid states unrepresentable, propagate errors as values, and keep `unsafe`
rare, small, and audited.

A concise definition:

> Idiomatic Rust for this repo is rustfmt-formatted, Clippy-clean, `Result`-first
> code. Errors propagate with `?` and are handled at a boundary. Invariants live in
> types, not in repeated runtime checks. Public API shape is designed with `pub use`
> rather than mirroring the private module tree. `unsafe` is wrapped in a safe
> abstraction, kept small, and checked with Miri. Tests live in `#[cfg(test)]`
> modules for units and in `tests/` for integration.

Repo-wide conventions live in `docs/REPO_STYLE.md`. Its core philosophies are cited
by name throughout this document.

## Common misconceptions

Agents coming from Python get these wrong. Each links to a section below.

- **Tabs are a Python rule, not a Rust rule.** `.rs` files are four-space,
  rustfmt-formatted. See section 1.
- **`pub use` re-export facades are good Rust.** `docs/PYTHON_STYLE.md` bans
  re-export facades in `__init__.py`; Rust endorses them in `lib.rs`. See section 5.
- **"Avoid try/except" does not become "avoid `Result`".** The Rust analogue is
  "`?` propagates, you handle at the boundary". See section 6.
- **A blanket `.unwrap()` to silence the compiler is the Rust form of
  `dict.get(key, fallback)`.** Both paper over missing data. See sections 8 and 9.
- **`clone()` is not a borrow-checker fix.** It is a deliberate cost. See section 10.
- **`unsafe` does not disable the borrow checker.** It unlocks exactly five
  operations. See section 12.

## 1. Let rustfmt and Clippy own formatting

Rust has an official style guide, and rustfmt implements it. Its guiding principles
are, in priority order, readability, aesthetics, specifics such as version-control
friendliness, and ease of application.[^style-principles] The default formatting
conventions are spaces rather than tabs, four spaces per indentation level, and a
maximum line width of 100 characters.[^style-index]

This is the one place where Rust style deliberately parts company with
`docs/PYTHON_STYLE.md`. The tabs-not-spaces rule in that document is a Python rule
about `.py` files. For `.rs` files, rustfmt output is authoritative. Do not fight it,
do not hand-format around it, and do not add a `rustfmt.toml` that diverges from
community defaults without a recorded reason.

rustfmt reformats code to the community style, and many collaborative projects use it
precisely to prevent arguments about which style to use.[^book-appd] That is the core
philosophy **Focus on important issues** in one sentence: formatting is bikeshedding, so
delegate it to a tool and spend the argument budget on design.

Two gates, both auto-allowed commands in this environment:

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

Run both before reporting work complete. `--check` fails without rewriting files,
which is what a gate should do; use plain `cargo fmt` to actually format.
`-D warnings` promotes every lint to an error so warnings cannot accumulate.

Two more tools from the same appendix:[^book-appd]

- `cargo fix` applies rustfix suggestions for compiler warnings that have an
  unambiguous correction, and also drives edition migration.
- `rust-analyzer` is the community-recommended language server, giving completion,
  jump-to-definition, and inline errors in an editor.

Good rule:

> rustfmt decides layout. Clippy decides idiom. You decide design.

## 2. Where Rust differs from the Python guide

`docs/PYTHON_STYLE.md` is the sibling guide for the other main language in these
repos. Three of its rules do not transfer, and misapplying them produces bad Rust.
The differences are stated here explicitly so nobody has to guess.

| Python rule | Rust position |
| --- | --- |
| Always indent with tabs | Four spaces, rustfmt-enforced (section 1) |
| No re-export facades in `__init__.py` | `pub use` in `lib.rs` is idiomatic (section 5) |
| Avoid try/except | `?` propagates; handle at the boundary (section 6) |

Everything else carries over unchanged. In particular the philosophy layer is
identical: **fix the design, not the symptom**, do not hide bugs behind defaults, and
prefer the durable fix (**long-term over short-term**). Rust simply gives you sharper
tools for obeying those rules, because the compiler can enforce most of them.

## 3. Naming conventions

Rust naming follows RFC 430, catalogued in the Rust API Guidelines.[^api-naming]

| Item | Convention |
| --- | --- |
| Crates and modules | `snake_case` |
| Types, traits, enum variants | `UpperCamelCase` |
| Functions, methods, local variables | `snake_case` |
| Macros | `snake_case!` |
| Statics and constants | `SCREAMING_SNAKE_CASE` |
| Type parameters | Single uppercase letter, for example `T` |
| Lifetimes | Short lowercase, for example `'a` |

In `UpperCamelCase`, an acronym counts as one word: write `Uuid`, not `UUID`.

Conversion methods carry cost information in their prefix, and readers rely on
it.[^api-naming]

- `as_` is a free conversion between borrowed views, for example `str::as_bytes`.
- `to_` is a potentially expensive conversion, typically borrowed to owned, for
  example `str::to_lowercase`.
- `into_` consumes the receiver, for example `String::into_bytes`.

Picking the wrong prefix is a correctness-adjacent bug: it misreports allocation and
ownership to every caller.

File naming follows `docs/REPO_STYLE.md`: lowercase ASCII, underscores between words,
no CamelCase in filenames. `CamelCase` is reserved for type names inside the file.

## 4. Crate and module layout

A package holds one or more crates: at most one library crate at `src/lib.rs`, and
any number of binary crates at `src/main.rs` and `src/bin/*.rs`. Modules form a tree
rooted at the crate root, and everything is private to its parent by default until
marked `pub`, as described in the Rust book chapter
[Control Scope and Privacy with Modules](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html).

Practical rules:

- Start every module private. Add `pub` when an outside caller genuinely needs the
  item, not preemptively. Privacy is the cheapest form of **design for adaptability**:
  a private item can be changed without breaking anyone.
- Split a module into its own file when it grows past comfortable reading length
  ([Separating Modules into Different Files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)).
  Prefer `src/thing.rs`, or
  `src/thing/mod.rs` plus submodules when the module has children.
- Keep the tree shallow. This matches `docs/REPO_STYLE.md` on repository structure:
  prefer small single-purpose units, and avoid deep nesting.
- Use `use` to bring the parent of a function into scope rather than the function
  itself, so call sites read `module::function(...)` and the origin stays visible
  ([Bringing Paths Into Scope with the use Keyword](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html)).
  Bring types, traits, structs, and enums in
  by full name. This is the same instinct as the `docs/PYTHON_STYLE.md` preference for
  `import os` over `from os import path`, and here it is also the community idiom.
- Do not use the glob operator `use foo::*` outside a `#[cfg(test)]` module, where
  `use super::*;` is the accepted convention
  ([Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)).

For multi-crate projects, use a Cargo workspace so member crates share one
`Cargo.lock` and one `target/` directory
([Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)). Reach for a
workspace when crates are separately meaningful, not merely to create folders.

## 5. Shape the public API with `pub use`

The internal structure that makes sense while writing a crate is often inconvenient
for callers. A caller should not have to write
`use my_crate::some_module::another_module::UsefulType;` to reach a type that is
central to the crate. Re-export with `pub use` instead, producing a
public structure that differs from the private one, without rearranging internals
(the Rust book, [Publishing a Crate to Crates.io](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html),
section "Exporting a Convenient Public API with `pub use`").

```rust
// src/lib.rs
pub mod kinds;
pub mod utils;

// Flatten the two items callers actually reach for.
pub use crate::kinds::PrimaryColor;
pub use crate::utils::mix;
```

This is the explicit exception to the `docs/PYTHON_STYLE.md` rule banning re-export
facades in `__init__.py`. That rule exists because Python's `__init__.py` is a file
coders do not inspect when hunting bugs, so logic hidden there disguises problems.
Rust's `lib.rs` is the opposite: it is the crate root, the first file a reader opens,
and the generated `cargo doc` front page reflects exactly what it re-exports. Do not
carry the Python prohibition into Rust.

The rule survives in spirit, though. A `pub use` line is an API decision, not a
convenience dumping ground:

- Re-export the small set of items that are genuinely the crate's front door.
- Do not put implementation logic, conditional imports, or runtime lookup tables in
  `lib.rs`. That part of the Python rule does transfer.
- Every `pub use` is a public commitment. Removing one is a breaking change.

## 6. Errors are values, and `?` is the propagation tool

Rust splits failure into unrecoverable (`panic!`) and recoverable (`Result<T, E>`).
Returning `Result` is the good default choice
when defining a function that might fail, because it hands the calling code the
decision. Choosing to panic makes that decision on the caller's behalf, and there is
no way back from it
([To panic! or Not to panic!](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html)).

`docs/PYTHON_STYLE.md` says to avoid `try`/`except`. Do not translate that into
"avoid `Result`". The Python rule targets a control-flow construct that swallows
context and encourages broad catches. Rust's `Result` is the opposite: it is a value
in the type signature, visible to every caller, impossible to ignore silently. The
correct Rust translation of the Python rule is:

> `?` propagates. You handle at the boundary.

```rust
// Propagate: the function's signature already advertises that it can fail.
fn load_config(path: &Path) -> Result<Config, ConfigError> {
	let text = std::fs::read_to_string(path)?;
	let config = toml::from_str(&text)?;
	Ok(config)
}
```

In practice:

- Use `?` for the whole call chain. Do not write a `match` whose only job is to
  return `Err(e)` unchanged.
- Handle errors at exactly one boundary per program: `main`, a request handler, a
  task entry point. That boundary decides how to report and whether to exit.
- Do not swallow an error into a default value. Silently substituting a fallback is
  precisely the pattern `docs/PYTHON_STYLE.md` forbids under DO NOT HIDE BUGS WITH
  DEFAULTS, and it is just as wrong in Rust. `unwrap_or_default()` on a genuinely
  failable operation hides the bug.
- Reserve `panic!` for a broken invariant, a violated function contract, or a state
  your code cannot continue from. A contract violation always
  indicates a caller-side bug, calling code has no reasonable way to recover,
  and such contracts belong in the API documentation
  ([To panic! or Not to panic!](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html),
  section "Guidelines for Error Handling").
- When failure is expected in normal operation, for example malformed parser input or
  an HTTP rate-limit response, return `Result`.

## 7. Choose the error type by crate role

The `thiserror` versus `anyhow` question has one clean answer, and it turns on
whether callers need to match on your errors.

- **Library crates: a concrete error type.** Callers must be able to branch on the
  failure mode, so define an enum, one variant per distinguishable failure, and
  implement `std::error::Error`. `thiserror` derives that boilerplate without adding
  anything to your public type signature. Preserve the cause with `#[from]` or
  `#[source]` so the chain survives.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("config file not readable: {path}")]
	Unreadable { path: PathBuf, #[source] source: std::io::Error },
	#[error("config file is not valid TOML")]
	Malformed(#[from] toml::de::Error),
}
```

- **Binary crates: an opaque error type.** Nothing downstream matches on the error,
  so a boxed dynamic error is fine. `anyhow::Result<T>` plus `.context("...")` gives
  a readable failure chain for the user with far less code.

Do not push `anyhow` into a library's public signatures. That forces every downstream
caller to give up structured matching, which they cannot recover on their own. That
is a **long-term over short-term** call: `thiserror` costs more today and stops the
API from calcifying around a decision the caller cannot undo.

Add either crate to `Cargo.toml` explicitly. `docs/REPO_STYLE.md` requires all
dependencies to be declared, not worked around.

## 8. When `unwrap` and `expect` are acceptable

There are exactly three situations where panicking beats returning a `Result`
([To panic! or Not to panic!](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html),
section "Examples, Prototype Code, and Tests"):

- **Examples.** Full error handling would obscure the concept being illustrated.
- **Prototypes.** `unwrap` and `expect` leave visible markers for the places you have
  not yet decided how to handle.
- **Tests.** A failed call should fail the test, and `panic!` is how a test reports
  failure.

There is a fourth case: when you have more information than the compiler. When
surrounding logic guarantees an `Ok` that the compiler cannot see, `expect` is
correct, and the message should record why
([To panic! or Not to panic!](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html),
section "When You Have More Information Than the Compiler").

```rust
let home: IpAddr = "127.0.0.1"
	.parse()
	.expect("Hardcoded IP address should be valid");
```

Write the `expect` message as the assertion that must hold, not as a lament. Prefer
`expect("<invariant that guarantees Ok>")` over `unwrap()` everywhere outside tests,
because the message is the documentation of the reasoning.

What is not acceptable: reaching for `.unwrap()` because the compiler complained and
you wanted the error to go away. That is the Rust form of writing
`config.get("name", "Unknown")` to make a `KeyError` stop happening. Both trade a
loud failure at the real site for a quiet wrong answer somewhere later. **Fix the
design, not the symptom**: if the value can legitimately be absent, model that in the
signature and propagate it.

## 9. Encode invalid states out of existence

This is the most important section in this guide, and it is the Rust expression of
the core philosophy **fix the design, not the symptom**.

Scattering validation checks through every function is verbose
and annoying, and Rust's type system can do those checks for you. A parameter
typed `u32` cannot be negative. A parameter typed `T` rather than `Option<T>` cannot
be absent, so the function body has one case instead of two, and code trying to pass
nothing will not compile.

Take it one step further with a custom type whose constructor is the only way in
([To panic! or Not to panic!](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html),
section "Custom Types for Validation"):

```rust
pub struct Guess {
	value: i32,
}

impl Guess {
	/// Returns None when the value is outside 1..=100.
	pub fn new(value: i32) -> Option<Guess> {
		if !(1..=100).contains(&value) {
			return None;
		}
		Some(Guess { value })
	}

	pub fn value(&self) -> i32 {
		self.value
	}
}
```

The field is private, so no code outside this module can construct or mutate a
`Guess` that is out of range. Every downstream function taking a `Guess` gets the
range invariant for free and never re-checks it. This is the newtype pattern, which
also serves type safety and abstraction
([Advanced Types](https://doc.rust-lang.org/book/ch20-03-advanced-types.html)) and lets you
implement external traits on external types
([Advanced Traits](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html)).

Apply it whenever a bare primitive is carrying a rule:

- `UserId(u64)` rather than a `u64` that must not be zero.
- `Email(String)` rather than a `String` that must contain an at-sign.
- `NonEmpty<Vec<T>>` rather than a `Vec<T>` that callers must remember to check.

Good rule:

> If two call sites re-check the same condition, the condition belongs in a type.

The payoff is **design for adaptability**. When the rule changes, it changes in one
constructor, and the compiler finds every construction site.

## 10. Ownership, borrowing, and lifetimes

Ownership is the design surface, not an obstacle to route around.

- Borrow by default. Take `&T` when you only read, `&mut T` when you mutate, and `T`
  when the function genuinely consumes the value.
- Prefer slice parameters over owned collections: `&str` over `&String`, `&[T]` over
  `&Vec<T>`. This accepts strictly more callers at no cost
  ([The Slice Type](https://doc.rust-lang.org/book/ch04-03-slices.html)).
- The rules of references still bind everywhere: at any time you may have either one
  mutable reference or any number of immutable ones, and references must always be
  valid ([References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)).
- Reach for `clone()` deliberately, with a reason. A clone that exists only to escape
  a borrow error is a symptom, and the fix is usually to shorten the borrow, split the
  struct, or restructure the call. A clone that exists because two owners genuinely
  need the data is fine. Say which one it is when it is not obvious.
- `Rc<T>` for shared ownership on one thread, `Arc<T>` across threads, and interior
  mutability (`RefCell<T>`, `Mutex<T>`) only when shared mutation is genuinely
  required
  ([Rc, the Reference Counted Smart Pointer](https://doc.rust-lang.org/book/ch15-04-rc.html),
  [RefCell and the Interior Mutability Pattern](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html),
  [Shared-State Concurrency](https://doc.rust-lang.org/book/ch16-03-shared-state.html)).
  Each of these moves a check from compile time to run
  time; that trade should be conscious.
- Do not write lifetime annotations the elision rules already supply. The three
  elision rules cover the overwhelming majority of function signatures
  ([Validating References with Lifetimes](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)).
  Annotate when the compiler asks, and when it does, treat the
  question as a design question about which input the output borrows from.

## 11. Prefer iterators, `match`, and `let...else`

- Prefer iterator adapters over manual index loops. They remove the index-arithmetic
  bug class and read as a pipeline
  ([Processing a Series of Items with Iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html)).
  A published benchmark of loops against iterators finds iterators fast enough that
  clarity should decide
  ([Performance in Loops vs. Iterators](https://doc.rust-lang.org/book/ch13-04-performance.html)).
  If a hot path
  matters, follow the core philosophy **use the scientific method** and measure with a
  benchmark rather than assuming.
- Keep adapter chains readable. A chain doing real work across several lines should be
  a named function, the same judgment `docs/PYTHON_STYLE.md` applies to `lambda`.
- `match` is exhaustive, and that is a feature. When you add an enum variant, the
  compiler shows you every site that must change. Use a catch-all `_` arm only when
  the remaining variants truly are interchangeable; a `_` arm silently absorbs every
  future variant and forfeits the exhaustiveness guarantee
  ([The match Control Flow Construct](https://doc.rust-lang.org/book/ch06-02-match.html),
  sections "Matches Are Exhaustive" and "Catch-All Patterns and the _ Placeholder").
- Use `let...else` to keep the happy path unindented
  ([Concise Control Flow with if let and let...else](https://doc.rust-lang.org/book/ch06-03-if-let.html)).
  This is the direct Rust answer to rightward drift, which the
  official style guide lists as a specific concern.[^style-principles]

```rust
let Some(config) = load_optional_config()? else {
	return Ok(Defaults::new());
};
// config is in scope, unindented, for the rest of the function.
```

- Use `if let` when one variant matters and the rest do not need naming.

## 12. Keep `unsafe` small, wrapped, and checked

`unsafe` unlocks exactly five operations that safe Rust forbids: dereferencing a raw
pointer, calling an unsafe function or method, accessing or modifying a mutable static,
implementing an unsafe trait, and accessing union fields. It does not turn off the
borrow checker or any other safety check
([Unsafe Rust](https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html)).

Repo policy:

- Default to no `unsafe`. Most application code never needs it.
- When it is needed, keep the blocks small. The reasoning is practical: because
  memory-safety errors must be inside an `unsafe` block, small blocks make the search
  space small.
- Wrap `unsafe` in a safe abstraction that exposes a safe API, so `unsafe` does not
  leak into every call site. Parts of the standard library are exactly this: audited
  safe abstractions over unsafe code.
- Write a `// SAFETY:` comment above every `unsafe` block stating the invariant that
  makes it sound. An `unsafe` block whose invariant nobody wrote down cannot be
  reviewed.
- Check it with Miri, the official tool for detecting undefined behavior. Unlike the
  borrow checker, Miri is dynamic: it runs the program or its test suite and reports
  violations ([Unsafe Rust](https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html),
  section "Using Miri to Check Unsafe Code").

```bash
rustup +nightly component add miri
cargo +nightly miri test
```

- For anything beyond a small wrapper, read the Rustonomicon, the official guide to
  unsafe Rust.[^nomicon]

Good rule:

> Every `unsafe` block has a written invariant, a safe wrapper, and a Miri run.

## 13. Document the public API

Documentation comments use `///` before the item, or `//!` inside the item for
module- and crate-level docs, and support Markdown. `cargo doc` runs rustdoc and
writes HTML into `target/doc` (the Rust book,
[Publishing a Crate to Crates.io](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html),
section "Making Useful Documentation Comments").

```rust
/// Adds one to the number given.
///
/// # Examples
///
/// ```
/// let answer = my_crate::add_one(5);
/// assert_eq!(6, answer);
/// ```
pub fn add_one(x: i32) -> i32 {
	x + 1
}
```

In practice:

- Document every `pub` item. A public item without docs is an undocumented promise.
- Use the conventional sections: `# Examples`, `# Errors` for what the `Err` variants
  mean, `# Panics` for the contract whose violation panics, and `# Safety` for
  `unsafe` functions. Section 6 requires the panic contract to be documented; this is
  where it goes.
- Code blocks in doc comments are compiled and run by `cargo test`. That makes
  examples executable documentation that cannot silently rot. Prefer a real example
  over prose describing one.
- `cargo doc --open` is the fastest review of whether the public API reads well from
  the outside.

This is the same instinct as the `docs/REPO_STYLE.md` documentation rules: keep docs
current, and remove or replace stale ones. Rust just gives you a test runner for them.

## 14. Test layout

The Rust community splits tests into two categories. Unit tests are small and focused,
test one module in isolation, and can reach private interfaces. Integration tests are
entirely external to the library, use only the public API, and exercise several
modules together
([Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)).

**Unit tests** live in the same file as the code, in a module named `tests` annotated
`#[cfg(test)]`. The attribute means the test code compiles only under `cargo test`,
not `cargo build`, which saves compile time and keeps tests out of the shipped
artifact.

```rust
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn adds_two() {
		assert_eq!(add_two(2), 4);
	}
}
```

**Integration tests** live in a top-level `tests/` directory next to `src/`. Cargo
compiles each file there as its own crate, and they need no `#[cfg(test)]` because
they are already outside the build. Shared helpers go in
`tests/common/mod.rs` so Cargo does not treat them as a test crate.

Note the deliberate parallel with `docs/E2E_TESTS.md`: `tests/` at the repo root is
the slow outer tier there too. In a Rust repo, `cargo test` runs unit tests,
integration tests, and doc tests in one pass, so keep individual tests fast enough
that the whole pass stays worth running.

Testing guidance that carries over from `docs/PYTEST_STYLE.md`: assert on behavior,
not on incidental facts. Do not assert on collection sizes, hardcoded defaults, or
function names when the real subject is the behavior.

## 15. Binary crates: thin `main`, logic in a library

When `main` starts growing, split the program into `src/main.rs` and `src/lib.rs` and
move the logic to `lib.rs`. What remains in `main` should be limited to calling the
argument parsing, assembling configuration, calling a `run` function in the library,
and handling the error that `run` returns
([Refactoring to Improve Modularity and Error Handling](https://doc.rust-lang.org/book/ch12-03-improving-error-handling-and-modularity.html)).

The decisive reason: `main` cannot be tested directly, so moving logic
out of it is what makes the program testable, and what is left in `main` is short
enough to verify by reading. That
is the same reasoning behind the `docs/PYTHON_STYLE.md` rule that `main()` is a
backbone calling single-task subfunctions.

Send errors to standard error, not standard output, using `eprintln!`. A CLI whose
error text lands in a redirected output file is a broken CLI
([Writing Error Messages to Standard Error](https://doc.rust-lang.org/book/ch12-06-writing-to-stderr-instead-of-stdout.html)).

```rust
fn main() {
	let config = Config::build(std::env::args()).unwrap_or_else(|err| {
		eprintln!("Problem parsing arguments: {err}");
		std::process::exit(1);
	});

	if let Err(e) = my_crate::run(config) {
		eprintln!("Application error: {e}");
		std::process::exit(1);
	}
}
```

Argparse minimalism from `docs/PYTHON_STYLE.md` applies unchanged to `clap`: add a
flag only when users change it between runs. Hardcode timeouts, buffer sizes, and
retry counts.

## 16. Editions, profiles, and dependencies

**Editions.** Roughly every three years the Rust team packages accumulated changes as
an edition. The `edition` key in `Cargo.toml` selects it; if absent, 2015 is assumed
for backward compatibility. Every compiler supports every prior edition and can link
crates of different editions together, so an edition choice is per-crate and does not
constrain dependencies
([Appendix E: Editions](https://doc.rust-lang.org/book/appendix-05-editions.html)).
Use the current edition for new crates. Migrate
existing crates with `cargo fix --edition`.

**Profiles.** Cargo has a `dev` profile for `cargo build` and a `release` profile for
`cargo build --release`, with defaults of `opt-level = 0` and `opt-level = 3`
respectively; override selectively under `[profile.*]` in `Cargo.toml`
([Customizing Builds with Release Profiles](https://doc.rust-lang.org/book/ch14-01-release-profiles.html)).
Change a profile only with a measured
reason, per **use the scientific method**.

**Dependencies.** `docs/REPO_STYLE.md` requires dependencies to be declared rather
than worked around. In Rust that means every crate you use appears in `Cargo.toml`
with a version requirement, and `Cargo.lock` is committed for binary crates so builds
are reproducible. Keep the dependency set small and justified; each one is API surface
you did not write but now maintain.

**Derives.** Prefer the standard derivable traits over hand-written impls where they
fit: `Debug` for programmer output, `PartialEq` and `Eq`, `PartialOrd` and `Ord`,
`Clone` and `Copy`, `Hash`, and `Default`
([Appendix C: Derivable Traits](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html)).
A hand-written impl of one of
these should have a reason, because a derive cannot drift out of sync with the fields.

## 17. Compact checklist

- `cargo fmt --check` and `cargo clippy -- -D warnings` both pass.
- Formatting is rustfmt's, not hand-tuned; the Python tabs rule was not applied.
- Naming follows RFC 430, including `as_` / `to_` / `into_` cost prefixes.
- Modules are private by default and the tree is shallow.
- `lib.rs` shapes the public API with `pub use` and contains no logic.
- Fallible functions return `Result`; `?` propagates; one boundary handles.
- Library errors are a concrete enum; binary errors may be opaque.
- Every `unwrap` or `expect` outside tests has a documented invariant.
- Invariants live in types, not in repeated runtime checks.
- Borrows are preferred to clones, and each clone has a reason.
- `match` arms are exhaustive; `_` is used only where future variants are equivalent.
- `unsafe` is absent, or small, wrapped, `// SAFETY:`-commented, and Miri-checked.
- Every `pub` item has a doc comment with a runnable example.
- Unit tests are `#[cfg(test)]`; integration tests are in `tests/`.
- `main` is thin, logic is in the library, and errors go to stderr.
- The edition is current and every dependency is declared.

## References

Every source cited here is freely readable online. Three carry most of the weight:

- [The Rust Programming Language](https://doc.rust-lang.org/book/), the official book,
  linked by chapter page. Section names are quoted so the passage is easy to find on the
  page; the guidance above is paraphrased.
- [The Rust Style Guide](https://doc.rust-lang.org/stable/style-guide/), the normative
  source for formatting.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), the normative
  source for public API design and naming.

[^style-principles]: The Rust Style Guide, "Guiding principles and rationale." https://doc.rust-lang.org/stable/style-guide/principles.html
[^style-index]: The Rust Style Guide, formatting conventions (4 spaces, no tabs, 100-character maximum width). https://doc.rust-lang.org/stable/style-guide/
[^api-naming]: Rust API Guidelines, "Naming" (RFC 430 casing conventions and conversion prefixes). https://rust-lang.github.io/api-guidelines/naming.html
[^book-appd]: The Rust Programming Language, Appendix D, "Useful Development Tools" (rustfmt, rustfix, Clippy, rust-analyzer). https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html
[^nomicon]: The Rustonomicon, the official guide to unsafe Rust. https://doc.rust-lang.org/nomicon/
