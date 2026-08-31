# Data contracts

This is PLE's cross-cutting data-contract lookup. It identifies what a datum
means, who controls it, where it may appear, how long it survives, and which
document owns the detailed contract. It does not replace schemas, Rust types,
routes, migrations, or the active implementation plan.

Read [CONTRACTS.md](CONTRACTS.md) for the contract register and
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) for the durable rationale. The
[active implementation plan](active_plans/implementation_plan.md) remains the
authority for scope, ordering, and acceptance.

## Status and authority

The labels below prevent a useful design from being mistaken for a shipped
browser or database feature.

| Label                              | Meaning                                                                                  |
| ---------------------------------- | ---------------------------------------------------------------------------------------- |
| **Implemented**                    | Production code and its named behavior evidence exist.                                   |
| **Current compatibility contract** | A supported wire or storage shape still used while a replacement is planned.             |
| **Reserved**                       | A documented future boundary; do not rely on it as an available feature.                 |
| **Fail closed**                    | Missing, foreign, malformed, stale, or contradictory input is refused.                   |
| **Authoritative**                  | The component that decides the value; a copy elsewhere is only a projection or evidence. |

An identifier is not authority. The server derives global identity, `AuthenticatedSession`,
exact course/Student/workspace relationships, permissions, and grading backend
from an authenticated request and the stored attempt. The browser can provide a
value for validation, but cannot establish its meaning by naming it.

## Contract vocabulary

| Term           | Meaning                                                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Answer-bearing | An answer, key, private rubric, tolerance, provider field/value map, or material that can reveal or calculate correctness. |
| Browser-safe   | Permitted in a projection for an authorized browser; never a grant of authority.                                           |
| Evidence       | Immutable or append-only material explaining an accepted outcome.                                                          |
| Projection     | A bounded view for one authorized reader or route.                                                                         |
| Presentation   | The exact answer-free question state shown for one issued attempt.                                                         |
| Typed key      | A constructed object-store key, rather than a browser-supplied storage path.                                               |

## Data taxonomy

The table is intentionally broad. Follow the owner link for exact fields,
formats, database relations, and recovery procedures.

| Data family                                | Authoritative owner                                                                | Browser visibility                                          | Persistence                                                                                                                            | On invalid or unavailable data                                            | Detailed authority                                                             |
| ------------------------------------------ | ---------------------------------------------------------------------------------- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Durable identities                         | Server-issued typed IDs                                                            | Only IDs needed by the authorized route                     | Question Library, course, Assignment Attempt, Question Attempt, object, and evidence rows                                              | Refuse malformed, foreign, or type-confused IDs                           | [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md)                                 |
| Draft source                               | Authorized instructor workspace                                                    | Never Student-visible; bounded Instructor projection only   | Workspace, source object, revision history                                                                                             | Refuse unauthorized, unpublished, or stale revision access                | [QUESTION_MODEL.md](QUESTION_MODEL.md)                                         |
| Published question                         | Publication transition and immutable version                                       | Answer-free render only                                     | Published Question Version records and immutable source assets                                                                         | Refuse missing version, unsupported public shape, or altered provenance   | [QUESTION_MODEL.md](QUESTION_MODEL.md)                                         |
| Assignment Attempt and timing              | Issuance service and stored Assignment Attempt state                               | Question Attempt ID plus permitted state summary            | Exact Course, Student Record, Assignment Attempt, Question Attempt, timer, Submission receipt, and Question Attempt Source Record rows | Conceal or refuse foreign state; reject completed or expired transitions  | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md)                             |
| Render presentation                        | Trusted backend reproducing version and seed                                       | Prompt, public assets, response shape, presentation binding | Question Attempt Source Record and private replay state                                                                                | Refuse inconsistent reproduction or unsupported render                    | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                   |
| Student response                           | Student, only within an issued attempt                                             | Request body supplied by student                            | Append-only submission evidence and idempotency receipt                                                                                | Structural/membership failure receives no grade                           | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                   |
| Grade and feedback                         | Server-only adapter, grader, and disclosure policy                                 | Only policy-permitted result and feedback                   | Result, protected feedback, score, and summary rows                                                                                    | Do not disclose private material; failed grading does not invent a result | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md)                             |
| Answer-bearing material                    | Private question definition, adapter, or grader                                    | Never                                                       | Protected database/object/provider state                                                                                               | Refuse if unavailable, malformed, or not authorized for the backend       | [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)                               |
| Account, session, and relationship records | Global session resolver, exact membership/ownership records, PostgreSQL forced RLS | Authorized projections only                                 | Global accounts plus course, Student, workspace, and capability relationships                                                          | A resolved Account and required relationship authorize access             | [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#authority-relationships) |
| Binary objects and assets                  | Object record plus typed-key object store                                          | Logical asset ID or authorized bytes only                   | Object record, immutable object, integrity metadata                                                                                    | Check scope and digest; refuse a mismatch                                 | [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md)                               |
| Caches and prefetch reservations           | Authoritative origin remains the database/backend                                  | Safe reusable render or no browser visibility               | Cache rows, reservation state, and metrics                                                                                             | Expire, re-render, or promote atomically; never grade a reservation       | [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md)                             |
| Jobs, leases, and aggregates               | Worker lease and generation fence                                                  | Coarse progress or aggregate projection only                | Job, ledger, aggregate, and audit rows                                                                                                 | Stale lease/generation cannot commit; cleanup remains retryable           | [FAILURE_RECOVERY.md](FAILURE_RECOVERY.md)                                     |
| Upload responses                           | No accepted Student-upload path yet                                                | No capability, object key, or signed URL                    | No Student-upload object is accepted                                                                                                   | **Fail closed** before submission mutation                                | Dedicated upload contract remains future work                                  |

## Visibility rules

The following rules apply across every table row:

- Browser-safe data is a read projection, not an instruction to trust a later
  browser request.
- Answer-bearing data never enters a student render payload, public asset,
  browser cache, analytics event, or generic worker payload.
- An Account, course, Student, or workspace ID in a path, header, JSON body, or
  cache key does not establish authority; authenticated server context and the
  exact stored relationship do.
- A storage key is constructed from typed server state. Browsers use logical
  delivery identifiers, not raw object-store paths.
- Private provider state, including WeBWorK upstream field/value mappings,
  stays behind the PLE server boundary.
- File uploads deliberately remain refused until the reserved capability
  contract is implemented and accepted.

## Assessment boundary

The student-facing assessment exchange has two different payload sizes and
trust levels:

| Exchange       | Current status                     | Browser receives or sends                                                             | Server derives or retains                                                                              | Owner                                                        |
| -------------- | ---------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ |
| Render         | **Implemented** foundation         | Answer-free prompt, public assets, widget shape, and safe presentation metadata       | Key, rubric, backend provenance, private replay mapping, and policy authority                          | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Submit         | **Current compatibility contract** | Question Attempt route, idempotency key, and tagged `StudentResponse`                 | Account, Student, exact course/Assignment Attempt, version, seed, backend, policy, and expected family | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Compact submit | **Reserved** cutover               | Attempt route, idempotency key, presentation digest, and Question-Type-minimal answer | The Question Type and all attempt-owned context                                                        | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |

`kind` is needed in the current render payload so the browser can select the
correct widget. It remains on the current compatibility submission wire. The
reserved compact submission contract omits it because the attempt already
selects the expected response schema.

Presentation binding is a consistency check, not authorization. The current
foundation uses a server-stored nonce and full digest, plus compact rendered
item IDs that are unique within one presentation. CRC16 can detect accidental
stale or mismatched visible state after uniqueness is enforced; it cannot
authenticate a student or defend against a malicious client. See
[IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) for identifier roles and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for the payload
strategy.

## Persistence and recovery

PLE records the accepted outcome, rather than trusting a client claim that a
request completed. An idempotency key associates a retry with its first
committed submission receipt. Compare-and-set revisions, leases, and scoring
generations stop stale concurrent work from overwriting newer state.

Immutable published versions and provenance let trusted backends reproduce the
issued question. Object bytes are separate from their authoritative metadata;
integrity checks and typed keys prevent an object path from becoming a second
authority. Retention uses ledgers and manifests so incomplete cleanup remains
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
| Question Types, publication, answer-free envelope                           | [QUESTION_MODEL.md](QUESTION_MODEL.md)                                                                               |
| Attempt issuance, grade lifecycle, mastery behavior                         | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) and [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md)  |
| Render/response payloads, CRC presentation IDs, WeBWorK boundary            | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                                                         |
| HTTP routes, session boundary, idempotency                                  | [API_CONTRACTS.md](API_CONTRACTS.md)                                                                                 |
| Identity names, UUIDs, capability versus identifier                         | [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md)                                                                       |
| Forced RLS, authenticated Account context, roles, concealed access failures | [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security) and [SECURITY_MODEL.md](SECURITY_MODEL.md) |
| Tables, migrations, and operational database layout                         | [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md)                                                                       |
| Object lifecycle, assets, integrity, retention                              | [OBJECT_STORAGE.md](OBJECT_STORAGE.md) and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md)                          |
| Data sensitivity and permitted projections                                  | [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md)                                                                     |
| Cache authority, immutable reuse, and safe prefetch                         | [CACHING_AND_PREFETCH.md](CACHING_AND_PREFETCH.md)                                                                   |
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
