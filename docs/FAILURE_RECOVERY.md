# Failure and recovery contract

PLE preserves authoritative state and gives the user a bounded next action.
Failure handling does not create a second product lifecycle. Current product
meaning comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

## Outcome rule

Every effectful request ends in one of three conditions:

| Condition | Meaning | Client behavior |
| --- | --- | --- |
| Committed | The durable effect is known to exist | Show current state; a replay returns the same outcome |
| Rejected | Authorization, validation, lifecycle, or stale-state rules refused the effect | Correct the visible issue or reload before another attempt |
| Indeterminate | An external effect may have happened but was not safely confirmed | Reconcile the exact operation identity before retrying |

Do not report success before the authoritative commit. Do not invent a result
from a timeout, provider error, missing cache entry, or worker message.

## Request errors

- Authentication failure does not reveal a protected target.
- Authorization failure for a foreign Student or Course record is
  non-enumerating where appropriate.
- Invalid or unknown input is rejected before mutation.
- A stale Edit Number or lifecycle precondition conflicts and requires reload.
- A retryable database abort retries only the complete idempotent owner
  operation.
- A dependency outage preserves input/evidence and returns a bounded unavailable
  state; it does not count as an incorrect Student answer.

Error responses containing protected context use `no-store` and omit Answer
Keys, responses, grades, backend state, credentials, object paths, and raw
provider output.

## Assessment Attempt recovery

The Student saves complete Question responses while the Assessment Attempt is
open. A lost save response can be retried against the same Attempt and Question
position. Reload restores only responses the server actually saved.

The Student submits the whole Assessment Attempt. Explicit submission and
automatic deadline submission converge on one submitted Attempt and finalize
the same saved responses. A lost success response is recovered by
reading or repeating that same Attempt transition.

If the browser remains disconnected through the deadline, the server submits
the whole Attempt, finalizes the complete responses saved before expiry, and
leaves other positions unanswered. This ordinary deadline behavior is the
Assessment recovery path;
there is no separate Student-visible recovery state machine.

## Question Backend failure

The backend owns rendering, response interpretation, grading, feedback, and
opaque state. PLE preserves the response and exact backend binding when an
operation fails. It never converts unavailability to zero credit or lets the
browser grade.

Human Guidance does not define a public pending-grading status, Instructor
Retry button, regrading operation, mutable result, or generic grading worker.
If a real backend cannot return its immutable credit fraction in the ordinary
operation, the required internal continuation and its failure semantics remain
a backend-specific unresolved design until explicitly accepted.

## Replica and cache continuity

API replicas carry no correctness-bearing process memory. PostgreSQL, typed
object storage, and the responsible backend boundary hold durable state. A
surviving replica can resume an authorized open Attempt from those authorities.

Cache misses rerender or refetch only through the authorized owner. Cache
mismatches fail closed. A cache never supplies Student ownership, saved-work
truth, submission state, timing, credit, or feedback policy.

## Object and provider effects

For an effect outside PostgreSQL, record and verify the exact target rather than
assuming a timeout means success or failure. Publish the database-visible state
only after the effect is verified. A retry is scoped to that same target and is
safe if it repeats.

Generic object cleanup cannot become authority to delete Student records.
Assessment Unrelease and Course retention own their exact deletion boundaries.

## Retention failure

The retention process checks the stored final Assessment deadline and later
Student activity, sends required Instructor notice, removes FERPA-protected
records from normal interfaces, keeps them recoverable during the retention
period, and permanently deletes them at expiry.

Each pass is idempotent. A partial failure must not advance the reported stage
past completed work, shorten the clock, duplicate material notices, or expose
archived records in ordinary interfaces. Exact job/event/receipt machinery is
an implementation choice, not a required product model.

## Restore and schema refusal

A restored database or object set is accepted only after structural,
authorization, integrity, and release compatibility checks succeed. Restoration
does not bypass retention or revive permanently deleted Student records.

An incompatible schema fails startup clearly. Do not add an undocumented
compatibility layer that silently changes product meaning.

## Diagnostics

Diagnostics identify the operation class, safe correlation, time, and bounded
failure category. They exclude credentials, private source, answers, Student
responses, grades, raw backend payloads, and FERPA-linked exports. Operational
logs have their own bounded retention and never serve as an undeclared Student
record archive.

## Change checklist

For a new failure path, identify the authoritative state, how the caller learns
whether it committed, the safe retry identity, what is preserved, what the user
sees, and which real boundary test proves it. Add recovery machinery only for a
demonstrated failure mode.
