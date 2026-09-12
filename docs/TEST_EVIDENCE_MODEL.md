# Test evidence model

PLE uses evidence that matches the claim. Fast tests protect narrow stable
behavior; connected service checks protect database and external-service
boundaries; real-stack browser journeys protect visible product behavior. A
passing check in one class does not establish a claim owned by another.

[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) requires tests to earn their maintenance
cost. Keep a test only when it protects a durable behavior, authorization
boundary, evidence-integrity rule, or supported lifecycle operation. When in
doubt, remove it.

## Evidence classes

- **Permanent focused tests** are deterministic, offline tests for validation,
  decoding, serialization, error mapping, authorization decisions, and other
  stable local contracts. They avoid incidental source inventories, timing,
  random values, real services, and sleeps.
- **Permanent connected acceptance** covers durable PostgreSQL/RLS/grant and
  storage boundaries that cannot be established by a fake. It is explicit rather
  than hidden in a fast unit lane.
- **Real-stack browser acceptance** proves named visible workflows against the
  production browser artifact and disposable Live Demo. It does not substitute
  for an unrelated database or worker guarantee.
- **One-time reconstruction evidence** answers a reset-specific question, such
  as catalog disposition, base rebuild, forward-migration rehearsal, restore,
  or Graphify regeneration. Record its conclusion concisely in the changelog;
  remove the temporary probe unless it independently qualifies as a permanent
  test.
- **Independent review** evaluates the named artifact and scope at the time it
  runs. It does not silently certify later changes.

## Supported validation lanes

`./launchers/all_test.sh` is the aggregate repository front door. It runs the
owned Rust, codebase, Python, and declared service checks. Browser and
connected operations remain explicit when their owner requires them.

Every bounded work item names the commands necessary for its claim. A result is
green only when its required commands ran successfully on the material tree.
Unavailable or skipped required acceptance is recorded as incomplete, not
treated as a passing test. The changelog records final commands, environment
assumptions, and one-time results.

## Database baseline and revision model

The permanent database evidence is intentionally small:

| Protected claim | Suitable evidence |
| --- | --- |
| Runtime roles cannot change structure; schema ownership, forced RLS, grants, and capability roles are closed. | Connected database security acceptance. |
| Question and Blueprint provenance is exact; Question IDs have canonical compact storage and validation; old Student Work remains interpretable. | Focused domain tests plus connected evidence where persistence or RLS matters. |
| Current Assignment save/release, released edits, future Attempts, archive/restore, Blueprint Draft publication, and Unrelease follow their lifecycle and concurrency contracts. | Narrow owner tests at the lowest layer that proves the behavior. |
| Fresh base initialization, no-op compatible replay, and application-role verification work through supported commands. | Connected lifecycle acceptance. |
| Default installation-data provisioning converges and the explicit opt-out leaves ordinary product data unprovisioned. | Connected installation-data acceptance. |

Migration-file counts, historical checksums, module layouts, SQL body snippets,
custom splitter behavior, retired Revision projections, and broad source
inventories do not protect the resulting product. They are removed or treated
as temporary investigation evidence.

Refusal of a nonempty target without leaving partial PLE structure is a stable
initialization contract and remains connected lifecycle evidence. An intentionally
corrupted mid-manifest install and a temporary forward-migration rehearsal are
one-time cutover evidence; they do not justify permanent synthetic test machinery.

## Browser and service scope

The canonical browser suite exercises visible, accessible PLE workflows through
the same-origin application stack. Focused browser scenarios may establish their
declared interaction but do not claim all user journeys. Service oracles cover
only the particular PostgreSQL, object-store, renderer, worker, or lifecycle
boundary they name. Use an authorized second session, reload, or observer when a
visible persistence claim needs proof; do not turn private setup shortcuts into
product contracts.

## Test admission

Before adding recurring coverage, identify the regression it prevents, the
stable contract it names, its execution owner, and why an existing test does not
already establish the claim. Tests write only to owned temporary storage. A
probe that counts files, preserves a former implementation path, or measures a
one-time reconstruction belongs in ignored scratch space and is removed after
the decision it informed.
