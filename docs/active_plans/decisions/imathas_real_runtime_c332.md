# C332 iMathAS real-runtime decision

Status: proposed for manager approval before implementation.

## Decision

C332 uses a small, PLE-maintained PHP bridge module co-deployed with a normal,
currently supported IMathAS PHP and MySQL installation.  It is not described as
an IMathAS upstream API and it does not require upstream to expose one.  The
bridge is the concrete implementation of the existing fixed private
`v1/imathas/{snapshot,render,launch,result,proxy}` adapter protocol.

The bridge uses IMathAS's shipped `assess2/AssessStandalone.php`,
`QuestionGenerator`, and `ScoreEngine` to render and evaluate.  It owns the
session state, submitted answer data, feedback, and score calculation.  PLE
continues to own only authorization, the exact Question Revision selected for
an Attempt, immutable source/evidence persistence, and final score projection.
PLE must not parse PG-like controls, answer names, rendered HTML, feedback, or
grading rules.

The exact Question Revision source is a required immutable source snapshot,
not a software-dependency snapshot.  At publish/import, bridge `snapshot`
canonicalizes the answer-bearing IMathAS Question data and required local asset
facts, stores that exact source bundle under its digest in bridge-owned
immutable storage, and returns the same bytes for PLE to retain as the Question
Revision source.  At each launch, the bridge resolves the supplied source digest
from that archive, recomputes the digest, and fails closed if it is missing or
mismatched.  It passes the verified archived source data directly to
`AssessStandalone::setQuestionData`, rather than letting the runtime use a
current mutable `imas_questionset` record.  Thus later source mutation or
deletion cannot change a resumed Attempt.  C332b must replace the adapter's
current live-item snapshot fetch during launch with this archive-availability
check; an item reference remains import provenance, not the execution target.
This satisfies source integrity without retaining PHP, MySQL, or IMathAS
version history.  Dependency resolution follows the repository's latest-first
policy; a tested current resolution is deployment evidence, not a historical
runtime pin.

## Browser and trust boundary

The browser receives one same-origin PLE launch URL only.  PLE creates its
server-held iMathAS session through the bridge, then serves a dedicated PLE
document/proxy route for that one Attempt position.  The browser never receives
an IMathAS endpoint, cookie, bridge handle, launch JWT, source snapshot, result
token, or score.

The PLE document route forwards only the exact restored opaque bridge session.
It accepts a bounded `GET` for the rendered activity and a bounded `POST` for
the activity submission; PLE treats both bodies and returned document as opaque.
The bridge, not PLE, owns the form fields, control names, DOM, JavaScript,
feedback, response state, and scoring.  The bridge must generate its activity
document so its form and supported runtime requests return to those two fixed
PLE routes.  It must serve only its current supported local runtime and
source-bundle local assets through explicit closed routes.  No browser input
can select an upstream host, path, header, redirect, cookie, or external URL;
there is no generic reverse proxy or direct cross-origin iframe.

Each bridge launch consumes PLE's signed short-lived launch assertion once and
creates a fresh opaque handle.  Its private state binds that handle to the
bridge session, PLE attempt, exact Question Revision, normalized iMathAS seed,
source checksum, and PLE challenge/binding checksum.  The bridge signs its
terminal result with its deployment secret.  PLE retrieves that result only
server-to-server, and the existing verifier must reject a missing, expired,
replayed, wrong-question, wrong-challenge, wrong-binding, or invalid score
before the immutable credit fraction is stored.  A browser `postMessage` or
ordinary marker submission is never a grade.

The initial `imathas_remote_grading_v1` profile supports only the bridge's
documented local runtime and snapshot asset set.  A source requiring an
unapproved remote resource or an unsupported bridge capability is rejected at
source acceptance; it is not given a fallback URL or a partially native PLE
implementation.  This is an engineering capability boundary, not a new product
question.  Later support for another IMathAS capability is an additive bridge
implementation with the same boundaries, not a new generic transport.

## Evidence from the official runtime

The official `drlippman/IMathAS` repository at reviewed commit
`c4bff0fe507a19dfd968dca7c772524428e6b5e8` describes IMathAS as a PHP/MySQL
web assessment system and documents LTI, not the adapter's private API.  Its
`assess2/AssessStandalone.php` accepts supplied Question data and state, calls
`QuestionGenerator` for rendering and `ScoreEngine` for scoring.  Therefore a
co-deployed bridge can execute the official runtime without pretending that
`/v1/imathas/*` is upstream functionality.  The official `displayq2.php` and
`QuestionGenerator.php` also demonstrate that ordinary execution otherwise
loads mutable `imas_questionset` rows, which is why launch-time checksum
revalidation and explicit supplied snapshot data are required.

## Atomic implementation order

| Task | Owner boundary | Outcome | Temporary proof |
| --- | --- | --- | --- |
| C332a | Bridge module and deployment composition | A real PHP bridge boots IMathAS and implements the six fixed private operations with HMAC authentication, size limits, no redirects/cookies in responses, and redacted errors. | Start a self-owned PHP/MySQL bridge, invoke every operation with valid and hostile request cases, then remove the fixture. |
| C332b | Bridge snapshot and local assets | Canonical source/asset serialization is deterministic and archived by digest; launch revalidates the archived bundle and uses supplied verified data with `AssessStandalone`. | Import one algorithmic IMathAS fixture; prove the archived bytes render/grade, then mutate or delete the live source and prove resume still uses the archived Question. Prove missing/tampered archive fails before execution. Remove. |
| C332c | Bridge session/result | One-time launch creates a server-held session; opaque POST updates only that session; terminal result is signed and binds every stated fact. | Two Accounts/two Attempts plus replay, expired, wrong binding, and forged-result matrix. Remove. |
| C332d | PLE server composition and document routes | The existing adapter is composed into the live server; authorized Student gets only the fixed PLE launch/document routes; all other users, positions, methods, and malformed body sizes are concealed or refused. | Fresh self-owned PG17/bridge/server HTTP matrix. Retain only a narrow stable authorization/result-authenticity test if every PYTEST_STYLE question passes. |
| C332e | Browser integration | The real iframe completes a bridge-owned interaction, receives bridge feedback, and PLE finalization stores only the verified immutable credit fraction. | Self-owned browser journey with network assertions: no IMathAS URL/cookie/token/source/score in PLE DTOs or browser storage; remove. |

C332 cannot be marked complete until C332a-e pass, C333 proves the shared
complex-backend boundary, and C362's backend-ownership closure includes the
real iMathAS path.  Existing recorded transports and ignored loopback tests are
not completion evidence and should be removed or converted only when they test
one stable external contract.

## Explicit non-decisions

This decision does not define H5P, Pool selection, generic external-resource
support, an upstream IMathAS API, a PLE parser for IMathAS HTML, a direct
browser IMathAS session, or a retained historical PHP/MySQL/IMathAS runtime.
