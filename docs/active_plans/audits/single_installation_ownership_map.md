# Single-installation authorization ownership map

## Purpose

PLE has one installation, global accounts, shared published content, private
workspaces, and exact course records. This map assigns one durable owner and
one server-side authorization predicate to every former installation-scoped
operation family. It is the Phase 0 authority for the baseline schema and the
contract cutover.

## Baseline construction decision

The clean-installation baseline is authored directly in `schemas/migrations/`
as the domain-ordered `2026082901` through `2026082932` design and the
ownership decisions in this map. It contains one definition for every live
relation, function, policy, grant, and acceptance witness; its ordered file
list follows the dependency graph recorded with the schema work. A clean
volume applies only this baseline.

## Ownership map

| Family | Durable owner | Server-derived identity | Predicate at the protected boundary |
| --- | --- | --- | --- |
| Actor identity | Active session record | `ActorContext { user_id, session_id }` resolved from the opaque credential | Session exists, is active, and binds its one immutable account role. The browser supplies neither identity value. |
| Catalog publication and discovery | Global published `QuestionId` and immutable version | `ActorContext.user_id` | `approved_instructor(user_id, now)` for discovery, reuse, improvement, and publication. Public projections remain answer-free. |
| Course and Student records | Exact `CourseId`, membership episode, and enrollment | `ActorContext.user_id` | Current Instructor membership plus approval for teaching operations; current Student membership plus matching enrollment ownership for Student operations. |
| Private authoring | `WorkspaceId` and current workspace relationship | `ActorContext.user_id` | Current owner or collaborator relation authorizes draft, source, import, preview, and publication actions. |
| Jobs and workers | Locked `JobId`, typed target, generation, handler family, and current lease | Worker-only lease capability | The broker checks job kind, typed target, current generation, unexpired lease, and handler-family grant before every preparation, read, write, and finalization. |
| Objects and delivery | Typed catalog, workspace, course-record, or provider object key | `ActorContext.user_id` or worker lease | The key names its real parent. Delivery checks the current catalog, workspace, course, enrollment, or lease predicate; an object identifier alone grants nothing. |
| Exports and retention | Exact export, assignment, course, stage, and generation | `ActorContext.user_id` or worker lease | Export creation checks current course-Instructor authority. Retention work checks its locked course/stage/generation and lease before it changes records or delivery. |
| Sysadmin support | Narrow audited support operation with its exact target | `ActorContext.user_id` | Sysadmin role permits only the named support broker. The broker records the action and applies its operation-specific course or account predicate. |

## Observer and support findings

The baseline's Course Observer grant names one course and permits named assignment
completion plus thresholded anonymous aggregate grades, never individual Student
scores. A Student Observer grant names one Student record and course; the guidance
supplies the FERPA-waiver assumption, so the auditable grant is the authority record
rather than a separate consent artifact. Both grants carry their own issue and
revocation fields. Their protected queries resolve the actor from the session and
require an active exact grant.

The historical support implementation demonstrates a narrow operation-specific
broker rather than ambient Sysadmin course access. The baseline carries that shape
forward as a support capability with actor, exact course and optional Student target,
purpose, allowed operation, issued time, expiry, revocation, and append-only use
evidence. The broker accepts the capability only for its named operation and target.

Candidate durable authorization cases are: revoked or fabricated observer grants
cannot disclose records; a Course Observer receives completion and thresholded
aggregate evidence without individual scores; Student Observer reads resolve only
the named Student; ordinary Sysadmin access remains unavailable; and an unexpired
capability permits only its recorded support operation.

## Worker target decision

`JobPayload` currently carries closed handler kinds and several exact
identifiers, but `Export { delivery_object }` and `Import { source_object }`
do not name a complete durable authorization parent. A baseline job row must
therefore store a server-resolved, immutable typed target alongside its payload:

| Job family | Required locked target |
| --- | --- |
| Accepted submission, assignment recalculation, item analysis, and auto-submit | `CourseId`, `AssignmentId` or `QuestionAttemptId`, and the relevant generation |
| Retention | `CourseId`, stage, and schedule generation |
| Catalog render and asset publication | Exact immutable `ProblemVersionRef` |
| Export | `ExportId`, `CourseId`, frozen manifest, and expected artifact identities |
| Import | `WorkspaceId` or explicit provider registration plus immutable source object |
| QTI import | `WorkspaceId`, import identity, and immutable source object |

The enqueue transaction resolves this target from current authorized records,
stores it with the closed handler kind and generation fence, and creates the
job. A claim returns the locked target and a fresh opaque lease token. Worker
brokers compare the claimed kind, target, generation, and lease token before
each state transition. Retry and cancellation keep the same target; a new
generation creates new work rather than widening an existing claim.

## Verification boundary

The baseline database acceptance suite proves that a foreign target, stale
generation, mismatched handler kind, expired lease, or client-supplied scope
value cannot read, write, dispatch, or finalize work. Course and workspace
acceptance cases prove the corresponding human authorization predicates.
