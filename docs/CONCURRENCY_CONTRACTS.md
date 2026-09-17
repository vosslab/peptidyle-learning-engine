# Concurrency contracts

Concurrency protects current state and immutable evidence; it does not create
new product revisions or lifecycle states. Product meaning comes from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

## Authority model

| Boundary                     | Concurrency authority                                                                                |
| ---------------------------- | ---------------------------------------------------------------------------------------------------- |
| Authenticated database work  | One protected transaction with server-installed Account context and exact relationship checks        |
| Draft Question               | Current Edit Number or equivalent compare-and-swap precondition                                      |
| Assessment                   | Current Edit Number or equivalent; not an Assessment Revision                                        |
| Blueprint content            | Expected current Blueprint Revision; a meaningful Save creates one next immutable Revision           |
| Blueprint metadata/lifecycle | Independent current metadata precondition; no Blueprint Revision                                     |
| Assessment Attempt           | One authoritative open Attempt per applicable start/resume operation                                 |
| Response save and submission | Serialization at the Attempt boundary; saved response wins before submission or is refused afterward |
| Assessment Unrelease         | Serialization at the Assessment root with complete Student Work deletion                             |
| Background service           | Exact target plus bounded lease only when a decided operation needs asynchronous execution           |

## Transaction rules

- Install authenticated Account context and perform the protected operation in
  the same database transaction.
- Recheck Account state and the exact Course, Student, workspace, or
  Blueprint-owner relationship inside the transaction.
- Retry only a whole idempotent owner operation after a retryable database
  abort. Do not replay individual statements with stale facts.
- Browser retries use the same logical target and precondition; they do not mint
  a new record merely because the response was lost.

## Current-state edits

A meaningful Draft Question or Assessment save advances its current Edit Number
once. A no-op leaves it unchanged. A stale value conflicts and requires a
reload; it never overwrites newer content.

Edit Numbers are not history. Only an explicit changed Blueprint Save creates
an immutable Blueprint Revision. A no-op Save returns the current Revision with
no new row. Blueprint name and lifecycle changes use current metadata and do
not create content Revisions.

Published Question source changes create one next immutable Question Revision
under the stable Question ID. Publication serializes against the Draft state it
validated so a stale request cannot publish a different source.

A Question or Pool Bloom Classification uses the exact Revision's independent
classification Edit Number as a whole-pair CAS precondition. The command always
contains both dimensions, including an unchanged dimension. The transaction
locks the exact pair and checks the expected number before deciding whether the
pair is a no-op: a stale request returns `412`, even when its requested pair
matches current values. A current changed pair advances the number once; a
current no-op keeps it. Neither outcome creates or substitutes a content
Revision.

The correction client does not automatically retry or merge after `412`. It
reloads the same exact Revision's current pair, retains the submitted draft for
comparison, and requires an explicit Instructor Save to issue a later command.

## Attempt convergence

Starting or resuming Student work returns the authoritative open Assessment
Attempt allowed by policy. Concurrent requests cannot create unintended
parallel Attempts.

Response save and whole-Assessment submission serialize on that Attempt:

- a valid complete response committed first is included in submission;
- a submission committed first closes the Attempt and refuses a later save;
- explicit and deadline submission converge on the same submitted Attempt; and
- repeating a completed submission returns the existing outcome.

Submission deduplication applies to the whole Assessment Attempt. Internal
response rows may have identities, but they remain evidence beneath that
Attempt.

## Credit and point-value races

The Question Backend's credit fraction is immutable. Current Assessment
Question point values are read when a score is calculated. A concurrent point
edit can change the next score projection but never changes the response or
regrades it. Human Guidance does not require scoring generations or a
background score-rebuild lifecycle.

## Assessment Unrelease

Unrelease locks the Assessment before deleting its Attempt roots. It verifies
the Released state, current precondition when used, exact title confirmation,
and equal co-Instructor authority. It commits the transition to Unreleased and
the complete Student Work deletion together. A concurrent save or submission
either commits first and is included in the deletion, or observes the
unreleased/missing target and cannot recreate the work.

## Background operations

The expired-Attempt submitter and retention checker use durable targets and
idempotent decisions. A bounded public-asset operation may also use an opaque
lease; a stale lease cannot commit.

Job infrastructure alone does not authorize grading queues, regrading,
recovery-state machines, generalized audit streams, snapshots, or compatibility
work. When PLE requests a grading outcome, the Question Backend returns it
without a deferred grading state.

## Cross-system writes

Database and object/provider effects cannot generally share one transaction.
Use a narrow operation-specific order:

1. validate authorization and record the intended exact target;
2. perform the bounded external effect;
3. verify the result; and
4. atomically publish only the verified database state.

Retries must be safe for that exact target. Never use a bucket listing,
provider response, queue message, or browser claim as target authority.

## Locking discipline

Lock the highest shared owner before its children and keep a stable order across
operations. For Student Work, that normally means Assessment, Attempt, then
selected Question/response rows. For Course membership, lock the Course or
roster root before the relationship row. Keep network calls outside database
locks when doing so does not weaken the operation's exact-target guarantee.

## Review checklist

For every competing write, identify the authoritative owner, precondition,
lock order, idempotency identity, lost-response behavior, stale-work fence, and
evidence that two real concurrent operations converge. Do not call a new enum
or receipt a concurrency solution unless the product actually needs that
object.
