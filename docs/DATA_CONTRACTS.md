# Data contracts

This is PLE's cross-cutting data-contract lookup. Product meaning comes from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). This document adds implementation detail
only where that detail is compatible with Human Guidance; a current code or
schema name is evidence about implementation, not authority to preserve an old
product model.

Read [CONTRACTS.md](CONTRACTS.md) for the contract register,
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) for durable rationale, and
[ROADMAP.md](ROADMAP.md) for delivery direction.

## Contract vocabulary

| Term | Meaning |
| --- | --- |
| Answer-bearing | An Answer Key, private rubric, tolerance, or backend value that can reveal or calculate correctness. |
| Browser-safe | Permitted in data returned to an authorized browser; never a grant of authority. |
| Evidence | The minimum durable record needed to explain a saved response or accepted outcome. |
| Presentation | The answer-free Question state shown for one Assessment Attempt. |
| Typed key | A server-constructed object-store key, not a browser-supplied path. |

An identifier is not authority. The server derives the Account, product role,
Course membership, Blueprint ownership, Student record, Attempt, and Question
Backend from authenticated context and stored relationships.

## Data taxonomy

| Data category | Authority and persistence | Browser boundary | Detailed authority |
| --- | --- | --- | --- |
| Global Account | Server identity and immutable product role | Authorized account projection only | [USER_ROLES.md](USER_ROLES.md) |
| Course relationship | Stored Course membership or Blueprint ownership | Only the relationship needed for the current page | [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) |
| Draft Question | Authorized Instructor workspace; mutable and unpublished | Bounded Instructor result only | [QUESTION_MODEL.md](QUESTION_MODEL.md) |
| Published Question | Stable identity plus immutable Question Revisions | Answer-free render and permitted metadata | [QUESTION_MODEL.md](QUESTION_MODEL.md) |
| Question Pool | Stable identity plus immutable Pool Revisions | Published discovery data and Assessment selection evidence | [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) |
| Blueprint Course | Stable identity plus immutable changed-content Revisions | Visibility follows Private, Public, or Archived state | [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) |
| Canonical Blueprint JSON | Complete Blueprint comparison and exchange representation; not primary persistence | Authorized import, export, comparison, and proposal review only | [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) |
| Course Instance Assessment | Current Course configuration; no Assessment Revision family | Instructor editor or Student Coursework projection | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) |
| Assessment Attempt | Course, Student, timing, saved responses, and whole-Assessment submission state | The authorized Student's current Attempt and permitted result | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) |
| Student response | Student-authored response saved while an Attempt is open | Typed native response or bounded backend-owned form data | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Credit and feedback | Question Backend's immutable credit fraction and permitted feedback | Hidden until the applicable disclosure point | [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md) |
| Binary object | Object metadata plus typed server key | Logical asset ID or authorized bytes only | [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md) |
| FERPA-protected Course record | Course Instance retention policy | Removed from normal interfaces at archive; recoverable only during retention | [RETENTION_POLICY.md](RETENTION_POLICY.md) |

## Assessment exchange

An Assessment Attempt is the submission boundary.

| Exchange | Browser action | Server responsibility |
| --- | --- | --- |
| Render | Request one Question in the open Attempt | Select the Assessment Question and exact backend-owned state; return an answer-free presentation. |
| Save response | Save a complete response while the Attempt is open | Validate ownership and shape, replace the working response, and retain the minimum evidence needed to interpret it. |
| Incomplete response | Leave a response incomplete | Do not save it as a complete response and do not grade it. |
| Submit Assessment | Submit the whole Attempt | Close the Attempt and finalize all saved responses together as Student Work. |
| Read result | Request a permitted result after submission | Apply disclosure policy to the immutable credit fraction and feedback; calculate score from current Question point values. |

The Question Backend owns rendering, response interpretation, grading,
feedback, and backend-specific state. PLE treats the rendered presentation and
state as opaque, stores the returned credit fraction without changing it, and
must not add a second parser for backend controls. Backend evaluation may occur
before whole-Assessment submission, but no Student-visible grading outcome is
created by saving a Question response.

The native PLE Question JSON format is private, unpublished, unversioned, and
strictly validated. Author JavaScript is isolated and untrusted; server grading
remains authoritative.

## Evidence and retention

Retain only evidence needed to identify the exact Question or Pool revision,
the Assessment selection, backend state needed to interpret the response, the
saved response, immutable credit fraction, and disclosure state. Human Guidance
does not require rendered-page snapshots, software-version snapshots, a public
grading-receipt model, or a general historical replay service.

The final Assessment deadline starts the Course retention clock; later Student
activity resets it. Archive, recovery during the retention period, permanent
deletion, and Course inactivity follow [RETENTION_POLICY.md](RETENTION_POLICY.md).

Background processing is limited to product behavior Human Guidance actually
requires, including expired-Attempt submission and idempotent retention checks,
plus operation-specific implementation work such as public-asset publication. It
is not authority to invent an asynchronous grading lifecycle, generalized audit
machinery, or compatibility states.

## Visibility rules

- Answer-bearing data never enters a Student render payload, public asset,
  browser cache, analytics event, or generic worker payload.
- A path, header, JSON field, cache key, or public ID cannot establish access.
- Private backend state stays behind the PLE server boundary.
- Browser caches and retained presentations are never grading or authorization
  authorities.
- Sysadmin support access to FERPA-protected records is scoped, deliberate, and
  recorded; the Sysadmin role has no ambient FERPA access.

## Owner directory

| Need | Start here |
| --- | --- |
| Product intent | [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) |
| Cross-module contract state | [CONTRACTS.md](CONTRACTS.md) |
| Question publication and native/backend boundaries | [QUESTION_MODEL.md](QUESTION_MODEL.md) and [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md) |
| Assessment lifecycle and payloads | [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) and [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) |
| Identity | [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) |
| Authorization and FERPA access | [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md), [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md), and [SECURITY_MODEL.md](SECURITY_MODEL.md) |
| Storage and object integrity | [OBJECT_STORAGE.md](OBJECT_STORAGE.md) and [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md) |
| Student-record retention | [RETENTION_POLICY.md](RETENTION_POLICY.md) |
| Evidence claims | [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) |

## Change rule

When implementation conflicts with Human Guidance, record the implementation
gap and update the implementation in a separate code task. Do not rewrite this
document to make an obsolete route, table, test, or fixture look like current
product intent.
