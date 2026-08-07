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

## Draft versus published

`Lifecycle` has three states:

| State | Holds a `ProblemId` | Appears in the catalog | New assignments |
| --- | --- | --- | --- |
| `Draft` | No | No | No |
| `Published` | Yes | Yes | Yes |
| `Withdrawn` | Yes | No | No |

"Is this a draft" is answered by the absence of a `ProblemId`, not by a stored
flag. A flag can disagree with the identifier beside it; an absent value
cannot.

Withdrawal exists because deletion would break the record. A course that
assigned a problem mid-term keeps working after the problem is withdrawn, and
the withdrawal only stops it appearing to new assignments.

## Transitions

Every change passes through one fallible function:

```rust
pub fn apply(
    state: Lifecycle,
    event: LifecycleEvent,
    minted: ProblemId,
) -> Result<Lifecycle, LifecycleError>
```

Legal moves:

| From | Event | To |
| --- | --- | --- |
| `Draft` | `Publish` | `Published`, carrying the minted `ProblemId` |
| `Published` | `Withdraw` | `Withdrawn` |
| `Withdrawn` | `Restore` | `Published` |

Anything else returns `LifecycleError::IllegalTransition`. The common case is
republishing a published version: that version is immutable, so a change means
publishing a *new* version.

The caller supplies the minted identifier rather than the function creating one
internally. That keeps minting server-side, where the `generate` feature is on,
while leaving `apply` callable from the browser so an editor can preview what a
transition would do.

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
