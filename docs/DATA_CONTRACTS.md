# Data contracts

This is PLE's cross-cutting data-contract lookup. It identifies what a datum
means, who controls it, where it may appear, how long it survives, and which
document owns the detailed contract. It does not replace schemas, Rust types,
routes, migrations, or the active implementation plan.

Read [CONTRACTS.md](CONTRACTS.md) for the contract register,
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) for the durable rationale, and
[ROADMAP.md](ROADMAP.md) for release direction and acceptance.

## Status and authority

The labels below prevent a useful design from being mistaken for a shipped
browser or database feature.

| Label                              | Meaning                                                                                              |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **Implemented**                    | Production code and its named behavior evidence exist.                                               |
| **Current compatibility contract** | A supported wire or storage shape still used while a replacement is planned.                         |
| **Reserved**                       | A documented future boundary; do not rely on it as an available feature.                             |
| **Fail closed**                    | Missing, foreign, malformed, stale, or contradictory input is refused.                               |
| **Authoritative**                  | The component that decides the value; a copy elsewhere is only a derived representation or evidence. |

An identifier is not authority. The server derives global identity, `AuthenticatedSession`,
exact course/Student/workspace relationships, permissions, and grading backend
from an authenticated request and the stored attempt. The browser can provide a
value for validation, but cannot establish its meaning by naming it.

## Contract vocabulary

| Term           | Meaning                                                                                                                                 |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Answer-bearing | An Answer Key, private rubric, tolerance, provider field/value map, or Question Grading Input that can reveal or calculate correctness. |
| Browser-safe   | Permitted in data returned to an authorized browser; never a grant of authority.                                                        |
| Evidence       | Immutable or append-only record explaining an accepted outcome.                                                                         |
| Reader result  | Bounded data returned to one authorized reader or route.                                                                                |
| Presentation   | The exact answer-free question state shown for one issued attempt.                                                                      |
| Typed key      | A constructed object-store key, rather than a browser-supplied storage path.                                                            |

## Data taxonomy

The table is intentionally broad. Follow the owner link for exact fields,
formats, database relations, and recovery procedures.

| Data category                              | Authoritative owner                                                                | Browser visibility                                               | Persistence                                                                                                                                                          | On invalid or unavailable data                                                                           | Detailed authority                                                             |
| ------------------------------------------ | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Durable identities                         | Server-issued typed IDs                                                            | Only IDs needed by the authorized route                          | Question Library, course, Assignment Attempt, Question Attempt, object, and evidence rows                                                                            | Refuse malformed, foreign, or type-confused IDs                                                          | [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md)                                 |
| Draft Question                             | Authorized Instructor workspace                                                    | Bounded Instructor result only                                   | Separate mutable Draft Question metadata and current draft source object                                                                                             | Refuse unauthorized, expired, unpublished, or stale Edit Number access                                   | [QUESTION_MODEL.md](QUESTION_MODEL.md)                                         |
| Published Question                         | Publication transition, stable lineage metadata, and immutable Question Revision   | Answer-free render only                                          | Separate Published Question metadata, Question Revision records, immutable author-declared educational Question Type, and immutable Question Revision source objects | Refuse missing revision, unsupported public shape, or altered Source Object Checksum                     | [QUESTION_MODEL.md](QUESTION_MODEL.md)                                         |
| Assignment Attempt and timing              | Issuance service and stored Assignment Attempt state                               | Question Attempt ID plus permitted state summary                 | Exact Course, Student Record, Assignment Attempt, Question Attempt, timer, Submission receipt, and Question Attempt Reproduction Details                             | Conceal or refuse foreign state; reject completed or expired transitions                                 | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md)                             |
| Render presentation                        | Trusted backend reproducing version and seed                                       | Typed PLE-native presentation or exact backend-owned document    | Question Attempt Reproduction Details plus the attempt-bound backend document where applicable                                                                       | Refuse inconsistent reproduction or unsupported render                                                   | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                   |
| Student response                           | Student, only within an active issued Attempt                                      | Typed PLE-native response or bounded backend-owned ordered pairs | Replaceable working save until whole-Attempt finalization; immutable internal Question Submission after acceptance                                                   | Structural, ownership, lifecycle, or presentation failure writes no accepted response                    | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                   |
| Grade and feedback                         | Server-only adapter, grader, and disclosure policy                                 | Only policy-permitted result and feedback                        | Result, protected feedback, score, and summary rows                                                                                                                  | Do not disclose an Answer Key or private Question Grading Input; failed grading does not invent a result | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md)                             |
| Private grading records                    | Private Draft Question, adapter, or grader                                         | Never                                                            | Protected database/object/provider state                                                                                                                             | Refuse if unavailable, malformed, or not authorized for the backend                                      | [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)                               |
| Account, session, and relationship records | Global session resolver, exact membership/ownership records, PostgreSQL forced RLS | Authorized projections only                                      | Global accounts plus course, Student, workspace, and capability relationships                                                                                        | A resolved Account and required relationship authorize access                                            | [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#authority-relationships) |
| Binary objects and assets                  | Object record plus typed-key object store                                          | Logical asset ID or authorized bytes only                        | Object record, immutable object, integrity metadata                                                                                                                  | Check scope and Object Checksum; refuse a mismatch                                                       | [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md)                               |
| Caches and retained presentations          | Authoritative origin remains the database/backend                                  | Safe reusable render or no browser visibility                    | Immutable render objects, retained Attempt presentation evidence, and metrics                                                                                        | Refuse mismatches; never grade or submit from a cache entry                                              | [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md)                             |
| Background work and public assets          | Narrow server-owned expiry/deferred-completion pass; exact public-asset Job target | No browser-visible lifecycle                                     | Immutable credit outcome, public-asset Job/lease, and audit records                                                                                                  | No Student or Instructor grading action; stale public-asset lease cannot commit                          | [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md)                                     |

## Visibility rules

The following rules apply across every table row:

- Browser-safe data is a reader result, not an instruction to trust a later
  browser request.
- Answer-bearing data never enters a student render payload, public asset,
  browser cache, analytics event, or generic worker payload.
- An Account, course, Student, or workspace ID in a path, header, JSON body, or
  cache key does not establish authority; authenticated server context and the
  exact stored relationship do.
- A storage key is constructed from typed server state. Browsers use logical
  delivery identifiers, not raw object-store paths.
- Private provider state stays behind the PLE server boundary. The WeBWorK E1 path is stateless:
  it holds no replay mapping or renderer session state.

## Assessment boundary

The student-facing assessment exchange has two different payload sizes and
trust levels:

| Exchange         | Current status  | Browser receives or sends                                                                                 | Server derives or retains                                                                                                       | Owner                                                        |
| ---------------- | --------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Render           | **Implemented** | Answer-free presentation or exact backend-owned document, selected by Assignment Attempt and position     | Account, Student Record, exact Course and Attempt, immutable Question Revision, seed, backend, policy, and presentation binding | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Save response    | **Implemented** | Assignment Attempt, position, and tagged `StudentResponse`, including bounded backend-owned ordered pairs | Exact issued Question Attempt and presentation mapping; later valid saves replace only the active working response              | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Finalize Attempt | **Implemented** | Bodyless whole-Attempt request or no browser request at deadline                                          | Explicit finalization requires every response; expiry auto-submits saved responses and closes missing positions unanswered      | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Completed result | **Implemented** | Policy-permitted credit-derived result and feedback                                                       | Immutable accepted response, backend credit fraction, current-point score, and private evidence                                 | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |

`kind` selects a PLE-native Question Response Control and a closed Student Response shape. A
backend-owned document selects the generic capture component instead; its controls do not identify
an educational Question Type. The immutable author-declared Question Type belongs to the Published
Question Revision and supports PLE discovery and labeling.

For a backend-owned response, the 64 KiB shared bound covers only the raw UTF-8 canonical ordered
pair payload captured from the browser form. It is distinct from the WeBWorK adapter's 256 KiB
PG/PGML source limit, 1 MiB renderer document/envelope bound, and asset limits.

Presentation binding is a consistency check, not authorization. The current
foundation uses a server-stored nonce and complete Question Presentation Checksum, plus compact rendered
item IDs that are unique within one presentation. CRC16 can detect accidental
stale or mismatched visible state after uniqueness is enforced; it cannot
authenticate a student or defend against a malicious client. See
[IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) for identifier roles and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for the payload
strategy.

## Persistence and recovery

PLE records the accepted whole-Attempt outcome rather than trusting a client claim that a request
completed. Explicit finalization and expiry auto-submission converge on one immutable Assignment
Submission. Each accepted saved response receives an immutable backend credit fraction; score
readers apply current point values. The narrow background pass performs only abandoned expiry
finalization and backend-specific deferred completion, without a public grading lifecycle.

Immutable published versions and Question Attempt Reproduction Details let trusted backends reproduce the
issued question. Object bytes are separate from their authoritative metadata;
integrity checks and typed keys prevent an object path from becoming a second
authority. Retention uses Course Retention Jobs with current Job Leases, Course
Retention Events, and Object Cleanup Manifests so incomplete cleanup remains
observable and retryable rather than silently partial.

Detailed rules are owned by [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md),
[FAILURE_RECOVERY.md](FAILURE_RECOVERY.md),
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md), and
[OBJECT_STORAGE.md](OBJECT_STORAGE.md).

## Owner directory

Use the narrowest owner document for a design or implementation decision:

| Need                                                                        | Start here                                                                                                           |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Contract state, module owner, acceptance evidence                           | [CONTRACTS.md](CONTRACTS.md)                                                                                         |
| Question Types, publication, answer-free Question Presentation              | [QUESTION_MODEL.md](QUESTION_MODEL.md)                                                                               |
| Attempt issuance, grade lifecycle, mastery behavior                         | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) and [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md)  |
| Render/response payloads, CRC presentation IDs, WeBWorK boundary            | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                                                         |
| HTTP routes, session boundary, and repeated-operation identities            | [API_CONTRACTS.md](API_CONTRACTS.md)                                                                                 |
| Identity names, UUIDs, capability versus identifier                         | [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md)                                                                       |
| Forced RLS, authenticated Account context, roles, concealed access failures | [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security) and [SECURITY_MODEL.md](SECURITY_MODEL.md) |
| Tables, migrations, and operational database layout                         | [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md)                                                                       |
| Object lifecycle, assets, integrity, retention                              | [OBJECT_STORAGE.md](OBJECT_STORAGE.md) and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md)                          |
| Data sensitivity and permitted projections                                  | [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)                                                                     |
| Cache authority, immutable reuse, and retained presentations                | [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md)                                                                   |
| Retries, races, provider failures, and repair evidence                      | [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md)                                                                           |
| Why a boundary exists and what is intentionally deferred                    | [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md)                                                                           |
| What a test or one-time probe proves                                        | [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md)                                                                     |

## Change checklist

When a data contract changes:

1. Change the authoritative Rust type, migration, route, or adapter contract
   first; this document never substitutes for source.
2. Update the narrow owner document and [CONTRACTS.md](CONTRACTS.md) when the
   public or cross-module boundary changes.
3. Mark a not-yet-accepted behavior **Reserved** rather than implying that it
   is already live.
4. Add behavior-focused evidence appropriate to the boundary. Keep temporary
   rebuild or measurement probes out of the permanent suite unless they meet
   [PYTEST_STYLE.md](PYTEST_STYLE.md)'s permanence criteria.
5. Update this lookup only when its taxonomy, visibility rule, or owner
   directory changes.
