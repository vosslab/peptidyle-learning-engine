# Pagination backend scale-oracle review

## Verdict

**ACCEPTED WITH A DOCUMENTATION CORRECTION.** The new MemoryStore conformance
fixture creates the right 51-record boundary and proves bounded 50- and
17-row traversals, exact union, stable order, no duplicates, terminal
completion, and foreign-tenant concealment. `PageSize` is expressly a maximum,
not an exact fill requirement, so the existing `<= page_size` assertion is the
correct store-level boundary. The only P2 is that the module's cross-backend
wording exceeds the sole registered driver, `MemoryStore`.

## Findings

| Priority | Finding | Evidence | Required correction |
| --- | --- | --- | --- |
| P2 | "Cross-backend" is not an honest description of current execution coverage. | The module doc says "Cross-backend" (`pagination_scale.rs:1`), but the only invocation is in `memory_store_conforms` with `MemoryStore::default()` (`drivers.rs:4-8`). No PostgreSQL conformance driver calls this generic helper. | Rename the module/documentation to MemoryStore scale conformance, or add a deliberately provisioned PostgreSQL conformance driver and only then retain the cross-backend claim. Do not imply a live PostgreSQL result from this offline fixture. |

There are no P1 findings. The requested size is a maximum under
`PageSize`/`Page<T>` (`src/pagination.rs:31-94`), not a promise that every
nonterminal page is filled. With 51 fixtures, returning all records in one
page fails the `<= 50` and `<= 17` checks; returning a terminal partial page
early fails exact set equality; and a cursor cycle fails the seen-cursor
assertion. The terminal `None` is necessarily observed before the collectors
return their exact expected sets.

## Confirmed properties

| Requirement | Result | Evidence |
| --- | --- | --- |
| Exact record union | PASS | 51 distinct assignment and enrollment IDs are created; the returned sets must equal the expected sets (`pagination_scale.rs:57-109`). |
| 50- and 17-row bounds | PASS | Every returned assignment and gradebook page is bounded by the requested size (`pagination_scale.rs:156`, `211`); 51 fixtures force continuation for both requested maximum sizes. |
| Stable returned-item order | PASS | Both traversals assert strict assignment or `(assignment, enrollment)` ordering across every collected item (`pagination_scale.rs:157-169`, `212-224`). |
| No duplicate returned records | PASS | Both identities are inserted into sets with a failing duplicate assertion. |
| Progress and terminal cursor | PASS | Repeated continuation tokens fail; exact 51-row set equality can return only after a terminal `None`, so a premature terminal or a repeated cursor cannot satisfy the fixture (`pagination_scale.rs:171-179`, `226-234`). |
| Foreign-tenant concealment | PASS | Assignment and gradebook calls under a separate authenticated tenant must return `StoreError::NotFound` (`pagination_scale.rs:111-125`). |
| Destructive/live database behavior | PASS | The new driver constructs an in-process `MemoryStore`; it does not create a pool, load a URL, start Podman, run migrations, or touch a PostgreSQL database (`drivers.rs:4-8`). |
| Generic helper reuse | PARTIAL | The helper is generic over `Store + CatalogStore`, so a future backend driver can reuse it, but no such driver exists today. |

## Validation run

- PASS: `cargo fmt --all -- --check`
- PASS: `cargo check -p learning-data-access --tests`
- PASS: `cargo clippy -p learning-data-access --tests -- -D warnings`
- PASS: `cargo test -p learning-data-access --test conformance memory_store_conforms` (1 passed)
- PASS: `git diff --check`

No live PostgreSQL, Podman, migrations, volumes, or staging operations were
performed.
