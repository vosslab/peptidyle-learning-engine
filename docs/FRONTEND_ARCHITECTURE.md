# Frontend architecture

PLE is a SolidJS single-page application backed by a same-origin Rust API and
an answer-free Rust WebAssembly facade. The browser presents authorized,
server-owned course, assignment, and Student Work data. The server owns
authorization, release, availability, timing, publication, grading, and
retained evidence interpretation.

[CONTRACTS.md](CONTRACTS.md) and [API_CONTRACTS.md](API_CONTRACTS.md) own the
wire contract. [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns the
meaning of current state and immutable Revisions. This document describes the
browser implementation boundary.

## Application structure

```text
src/routes.ts and src/route_contract.ts
  -> route access boundary and application shell
    -> page composition in src/pages/
      -> feature workflows in src/features/
        -> typed same-origin API client in src/api/
          -> strict decoding of server JSON
```

[src/routes.ts](../src/routes.ts) derives every product route from the single
[src/route_contract.ts](../src/route_contract.ts) declaration. A route's
product-role check is presentation admission only; each request independently
receives server authorization. Opaque route references locate a candidate and
never grant access.

[src/api/application_api.tsx](../src/api/application_api.tsx) provides one
typed client and router-owned query identities. [src/api/client.ts](../src/api/client.ts)
is the browser-safe client shape. [src/api/http_client/](../src/api/http_client/)
owns same-origin transport, no-store response handling, and HTTP error mapping.
The decoders in [src/api/decoders/](../src/api/decoders/) accept closed,
bounded JSON shapes before pages use them. Rust owns serialized field spelling;
the TypeScript types in `generated/api/` are derived from Rust contract roots.

## Product routes

The executable route contract includes these user-facing areas:

| Area | Routes | Browser responsibility |
| --- | --- | --- |
| Account and invitations | `/`, `/sign-in`, `/profile`, invitation routes | Session-oriented navigation and account-owned actions |
| Student delivery | Student Course landing, Assignment access, `R-*` Attempt and summary routes | Answer-free start, resume, response, submission, and disclosed history views |
| Question authoring | `/library`, Question detail, `/authoring/drafts` | Library discovery and private Draft editing/publication |
| Blueprint Courses | `/blueprint-courses` and detail route | Browse, owner Draft editing, and explicit publication |
| Course teaching | Course, Assignment workspace, roster, Gradebook, appearance, grade settings, operations | Current Course configuration and Instructor workflows |
| System support | Instructor-account and scoped-support roster routes | Bounded Sysadmin tools |

The exact paths, route parameters, product-role admission, ribbon state, and
layout remain in [src/route_contract.ts](../src/route_contract.ts). A browser
route does not exist for a capability merely because its API transport exists.

## Current Assignment workflow

The Assignment workspace is one current mutable aggregate. Its browser client
is [src/api/assignment_release.ts](../src/api/assignment_release.ts), with
transport in [src/api/http_client/assignment_release.ts](../src/api/http_client/assignment_release.ts)
and page composition in [src/pages/assignment_workspace/](../src/pages/assignment_workspace/).

- Saves, inline changes, release, and Unrelease carry the quoted Assignment
  Edit Number ETag. A `412` leaves the local draft available for reload or
  correction.
- Assignment Entries and picker rows carry exact `QuestionRevisionReference`
  pins. The workspace describes future Attempt configuration; it does not
  create an Assignment Revision.
- Release validation precedes the explicit release action. A successful release
  returns the current Assignment and its next ETag.
- The policies page displays a Released Assignment's aggregate Unrelease
  impact and requires the current title before submitting the destructive
  transition. The returned receipt exposes aggregate deletion counts only.
- A later accepted released save affects future Attempts. Existing Attempt
  pages read the server's retained Attempt and Issued Question evidence,
  including exact Question Revision and presentation binding.

`Assignment Revision`, Assignment snapshot, and successor-revision browser
types, routes, decoders, and pages are absent.

## Published content and Blueprint Courses

Question and Blueprint availability belong to their stable lineages. The
browser availability clients in [src/api/question_availability.ts](../src/api/question_availability.ts)
and [src/api/blueprint_course.ts](../src/api/blueprint_course.ts) use an exact
Availability Edit Number ETag. Archive includes title confirmation; restore
uses the Edit Number. Ordinary discovery lists Available lineages, while an
authorized exact immutable Revision read remains available after archive.

A Blueprint Course create response contains its stable lineage and private
current Draft, with no Blueprint Revision. The owner-facing workflow in
[src/features/blueprint_course/](../src/features/blueprint_course/) saves that
Draft with its Draft ETag and publishes it explicitly. Publishing returns an
immutable `BlueprintRevisionReference`; the Draft remains the owner working
copy. The browser retains a local Draft across recoverable validation or
concurrency errors. This is a single-owner Draft workflow; collaboration is
not a browser capability.

Blueprint Revision content is answer-free reusable course structure. A Course
Assignment exposes `BlueprintAssignmentSource`: the stable Blueprint Assignment
Reference plus its exact Blueprint Revision Reference. It is provenance, not a
third Revision model.

## Student Work boundary

The Attempt route presents one issued question at a time and keeps only the
current response state needed for recovery. It sends typed response and
submission commands and renders only server-authorized status and feedback.
[src/wasm/index.ts](../src/wasm/index.ts) is the sole browser import boundary
for generated Wasm helpers; those helpers provide answer-free validation and
formatting, never authorization or grading.

The client bundle and browser storage exclude answer keys, grading input,
private source/object locations, undisclosed feedback, and authority. The
server decides whether an Attempt can start or resume and reconstructs an
existing Attempt from retained evidence rather than mutable Assignment state.

## Browser behavior

- Every private API read uses the typed same-origin client and no-store
  response handling.
- Mutations use their applicable ETag and idempotency identity. The client
  preserves input on recoverable conflicts or dependency errors.
- Cursor lists use their declared cursor contracts; pages do not synthesize an
  offset fallback.
- Local storage contains only consent-appropriate preferences. Session storage
  is limited to explicitly requested in-progress response recovery and clears
  at the documented boundary. Credentials and protected academic records are
  not browser storage.
- Pages keep semantic labels, visible validation, keyboard-equivalent actions,
  focus recovery, and answer-free error messages. Accessibility authorities are
  [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md) and
  [COLOR_CONTRAST_ACCESSIBILITY.md](COLOR_CONTRAST_ACCESSIBILITY.md).

## Verification

Keep focused TypeScript and Node coverage for strict decoding, route/reference
binding, ETag behavior, Draft and response recovery, and answer-free DTOs.
Use real-stack browser acceptance for visible authoring, teaching, delivery,
and destructive-workflow journeys. [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md)
classifies permanent checks separately from one-time implementation evidence.
