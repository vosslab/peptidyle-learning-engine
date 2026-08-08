# Problem identity and lifecycle

How a question is identified, and what happens to it over time (MOD-ID,
WP-C2). The types live in `crates/question_model/src/identity.rs` and
`crates/question_model/src/lifecycle.rs`.

## The rule in one sentence

A draft lives in an instructor workspace and has no `ProblemId`; publishing is
the transition that assigns one; a published version is immutable thereafter.

Everything below follows from that sentence.

## Four identifiers

| Type | Names | Scope |
| --- | --- | --- |
| `WorkspaceId` | An instructor workspace | Tenant-owned |
| `ProblemId` | A published problem, across all its versions | Shared content |
| `VersionId` | One immutable version of a problem | Shared content |
| `AssetId` | A stored image, figure, or source package | Shared content |

They are distinct newtypes over `Uuid`, so a function expecting one refuses the
others at compile time. The bug this prevents is passing a draft's identifier
where published content is expected, which would let unpublished material reach
a course.

All four are UUIDv7. Two properties matter: the value is random enough that a
catalog number reveals nothing about how many problems exist, and it is
time-ordered enough to index well. Sequential identifiers would leak volume and
invite enumeration.

Minting sits behind the `generate` feature. The server enables it; the
WebAssembly bridge leaves it off, so the browser bundle has no way to create an
identifier. `from_uuid` stays available for rehydrating values read back from
storage.

## Lifecycle states

`Lifecycle` has five one-way states:

| State | Holds a `ProblemId` | Catalog browse | New assignments | Exact resolution |
| --- | --- | --- | --- | --- |
| `Draft` | No | No | No | No |
| `Validated` | No | No | No | No |
| `Published` | Yes | Yes | Yes | Yes |
| `Deprecated` | Yes | No | Yes, by exact reference | Yes |
| `Archived` | Yes | No | No | Yes |

"Is this a draft" is answered by the absence of a `ProblemId`, not by a stored
flag. A flag can disagree with the identifier beside it; an absent value
cannot.

Deprecation and archival preserve historical references because deletion would
break the record. A deprecated version disappears from discovery but remains
assignable by an exact reference; archival additionally blocks new references.
Deprecation carries an author explanation, and archival retains it.

## Transitions

Every change passes through one fallible function:

```rust
pub fn apply(
    state: Lifecycle,
    event: LifecycleEvent,
) -> Result<Lifecycle, LifecycleError>
```

Legal moves:

| From | Event | To |
| --- | --- | --- |
| `Draft` | `Validate` | `Validated` |
| `Validated` | `Publish { problem }` | `Published`, carrying the server-minted `ProblemId` |
| `Published` | `Deprecate { reason }` | `Deprecated` |
| `Deprecated` | `Archive` | `Archived` |

Anything else returns `LifecycleError::IllegalTransition`; an empty
deprecation explanation returns `LifecycleError::EmptyDeprecationReason`.
There is no restore transition. Correcting published content means publishing
a new immutable version and deprecating the superseded one when appropriate.

The caller places the minted identifier in the publish event rather than the
function creating one internally. That keeps minting server-side, where the
`generate` feature is on, while leaving the pure transition callable in
key-free clients.

## Version ownership and forks

A problem has a nonempty author set and a linear version chain. An author may
publish one successor whose `previousVersion` points to the version it revises.
The store locks that base version and refuses a second successor, so conflicting
branches cannot silently form under one `ProblemId`.

A third party creates a new `ProblemId` instead. Its first version records the
exact source in `derivedFrom`, preserving attribution and license lineage
without granting write access to another author's chain.

## Why immutability pays for itself

An assignment references `(ProblemId, VersionId)`, not just a problem. So:

- Improving a problem creates a new version and leaves every course that
  already assigned the old one delivering exactly what it delivered before.
- A grade stays auditable, because the exact version a student saw still
  exists.
- One published version serves thousands of courses with no copying, which is
  what makes the shared catalog work at all.

The cost is that "edit a published problem" is not an operation. Publishing a
new version is, and instructors are told which of their assignments reference
an older one.

## Related documents

- [QUESTION_MODEL.md](QUESTION_MODEL.md): the types these identifiers live in.
- [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md): the shared-content versus
  tenant-owned split that decides which identifiers carry a tenant.
- [active_plans/implementation_plan.md](active_plans/implementation_plan.md):
  publication governance and retention.
