# Frontend architecture

PLE is a SolidJS single-page application backed by one Rust API and one
answer-free Rust WebAssembly facade. The browser presents server-owned course,
assignment, and Student projections. It never decides correctness, timing,
authorization, publication, or release.

This document applies [SOLID_MODEL.md](SOLID_MODEL.md),
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md),
[COLOR_CONTRAST_ACCESSIBILITY.md](COLOR_CONTRAST_ACCESSIBILITY.md), and
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md). Wire and security authority
remains in [CONTRACTS.md](CONTRACTS.md), [API_CONTRACTS.md](API_CONTRACTS.md),
and [SECURITY_MODEL.md](SECURITY_MODEL.md).

## Primary flow

```text
authenticated course context
  -> BlueprintCourse workspace or CourseInstance teaching route
  -> strict API decoder
  -> local presentation model
  -> visible edit, preview, or delivery action
  -> typed server command
  -> authoritative response and revision
```

BlueprintCourse is the reusable source surface. A published BlueprintCourse
projection is visible and reusable to every vetted Instructor; a draft is
visible only to its owner and authorized workspace collaborators. CourseInstance
pages are private to current equal Teaching Team Members and enrolled Students. Each
CourseInstance route and response carries the exact destination CourseId; its
immutable Blueprint parent and applied revision are server-owned.

## Route map

| Route                                                                  | Surface                                            | Authority                                                                             |
| ---------------------------------------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------- |
| /                                                                      | Signed-in course list                              | Account and course summaries                                                          |
| /sign-in                                                               | Deployment-gated seeded Live Demo Account selector | Authenticated Session contract                                                        |
| /courses/:courseId                                                     | CourseInstance assignments                         | Current course relationship                                                           |
| /courses/:courseId/assignments/:assignmentId                           | Assignment overview                                | Exact CourseId and assignment relationship                                            |
| `/assignment-attempts/:assignmentAttemptId` (target)                   | Assignment Attempt                                 | Issued Assignment Attempt and Student entitlement                                     |
| `/assignment-attempts/:assignmentAttemptId/summary` (target)           | Assignment Attempt summary and practice entry      | Disclosed server projection                                                           |
| /library                                                               | Question Library                                   | Vetted Instructor Question Library authority                                          |
| /blueprint-courses                                                     | BlueprintCourse workspace                          | Blueprint Course Owner/Blueprint Collaborator drafts and shared published projections |
| /blueprint-courses/:blueprintCourseRef                                 | BlueprintCourse detail/editor                      | Blueprint reference plus active session                                               |
| /workspace                                                             | My Question Drafts                                 | Workspace relationship                                                                |
| /workspace/:workspaceRef                                               | My Question Draft editor and preview               | Workspace relationship                                                                |
| /instructor/courses/:courseRef/assignments/new                         | New Assignment                                     | Current course Instructor                                                             |
| /instructor/courses/:courseRef/assignments/:assignmentRef              | Assignment home                                    | Exact CourseId and assignment                                                         |
| /instructor/courses/:courseRef/assignments/:assignmentRef/questions    | Questions                                          | Assignment revision                                                                   |
| /instructor/courses/:courseRef/assignments/:assignmentRef/policies     | Policies                                           | Assignment revision                                                                   |
| /instructor/courses/:courseRef/assignments/:assignmentRef/student-view | Instructor Student view                            | Course Instructor, answer-free                                                        |
| /instructor/courses/:courseId/gradebook                                | Gradebook                                          | Current course Instructor                                                             |
| /instructor/courses/:courseId/students                                 | Roster and enrollment                              | Current course Instructor                                                             |
| /blueprint-courses                                                     | Blueprint Course adoption and imports              | Course Instance destination authority                                                 |

Assignment Attempt screens use `/assignment-attempts/:assignmentAttemptRef` and
the canonical Assignment Attempt terms in
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).

src/routes.ts is the executable route map. Route preflight and response
decoding remain server/API responsibilities; a URL reference never grants
authority.

## Component ownership

```text
API runtime provider
- Wasm facade and runtime status
- Router
  - App shell and route error boundary
    - Route resource
      - Feature/page state
        - Answer-free presentation or Question Response Control
```

src/api/application_api.tsx creates one typed Application API. src/api/http_client/
contains same-origin transport. src/api/decoders/ converts unknown JSON into
strict local types. src/features/ owns capability workflows; src/pages/ owns
route composition; src/components/ owns reusable presentation and keyboard
interaction. The production browser consumes dist/ through the HTTPS gateway;
browser-free fixtures are test seams only.

## BlueprintCourse workspace

The reusable source client is owned by:

- src/api/blueprint_course.ts: one BlueprintCourse client contract.
- src/api/http_client/blueprint_course.ts: list, get, create, replace, and
  lifecycle requests with strong ETags and no-store reads.
- src/api/decoders/blueprint_course.ts: strict nested-tree decoder.
- src/features/blueprint_course/: one list/detail/editor workspace and
  local draft model.
- src/pages/blueprint_course_route_page.tsx and
  src/pages/blueprint_course_detail_route_page.tsx: route composition.

The workspace displays ordered modules and ordered assignments. Its problem
picker source descriptor contains one Blueprint reference and normalized module
and assignment positions. A one-assignment selection is a bounded projection
of the same tree, not another source type. The editor keeps draft input locally,
sends a complete definition with the observed revision, and preserves the local
draft after a stale or invalid response.

A published projection contains only answer-free definitions, reviewed Question Authorship,
public Question IDs, safe Question Library summaries, current publication state, and
disclosed evidence context. It contains no answer key, private source,
response, grading payload, internal UUID, email, Student, or CourseInstance
record.

## Blueprint-operation transport boundary

The Blueprint-operation transport is separate from the reusable source client:

- src/api/blueprint_operations.ts: source, destination, operation, preview, and
  receipt types.
- src/api/http_client/blueprint_operations.ts: no-store preview and idempotent
  apply requests.
- src/features/blueprint_operations/: the operation-workflow stylesheet.

No Blueprint-operation page or server route is mounted yet.

The future page loads one BlueprintCourse source and asks the Instructor to
choose the destination operation:

| Operation                      | Result                                                            |
| ------------------------------ | ----------------------------------------------------------------- |
| Fork BlueprintCourse           | Independent AccountId-owned BlueprintCourse with source lineage   |
| Copy Assignment from Blueprint | Selected nested assignment in an exact existing Course Instance   |
| Create Course from Blueprint   | New Course Instance with one immutable Blueprint parent/revision  |
| Copy Course for New Term       | New teaching instance without Student or issued state             |
| Shift Course Dates             | Atomic date resolution when no issued work makes it ineligible    |
| Fast-forward or selected copy  | Update untouched import or create a new assignment when divergent |

Every preview binds source reference, observed revision, target CourseId where
applicable, term, time zone, and idempotency evidence. The server resolves
relative calendar-day and local-wall-clock values, reports DST corrections,
and returns an apply command derived from the accepted preview. New
BlueprintCourse assignments appear in daughter CourseInstances as unreleased;
the Instructor explicitly releases them. The browser never silently overwrites
delivery edits or releases upstream additions.

The former paired product-level route/client/UI names are SD1 migration inputs
only. They are not accepted route aliases, decoder variants, or UI branches.

## Client contract

| Concern         | Frontend rule                                                                  |
| --------------- | ------------------------------------------------------------------------------ |
| API access      | Every route uses the typed API runtime and same-origin client.                 |
| Decoding        | Decode from unknown with closed, bounded, field-by-field decoders.             |
| Mutation        | Send strong revision evidence and operation-specific idempotency keys.         |
| Pagination      | Use cursor contracts; never create an offset-based fallback.                   |
| Cache           | Use no-store for private projections and previews; do not cache authority.     |
| Generated types | Consume generated/api/ output derived from Rust; do not hand-edit it.          |
| Errors          | Preserve entered draft/response where the contract permits recovery.           |
| Authority       | Treat server authorization, revision, schedule, release, and grading as final. |

Rust Serde owns serialized spelling. The generated TypeScript projection is a
derivative of Rust contract roots through
crates/project-tools/src/tsgen.rs. Authored TypeScript owns only transport
adapters, strict decoding, and presentation models.

## Student and delivery boundary

The Assignment Attempt page keeps one editable response and one idempotency key for the
current Assignment Attempt. It may use answer-free Wasm for format hints and timing
display. It never stores keys, private envelopes, unreleased Student Feedback, or
provider state, and never derives correctness or completion.

After accepted submission, the browser clears the response and polls an
answer-free status projection. It does not resubmit known-accepted work.
Student Feedback, score, item correctness, solution, class statistics, late status,
and Student Feedback Release are redacted or exposed only by the server's current policy.

CourseInstance pages use exact CourseId and Student relationship context.
Instructor Student view is informational and creates no Student work. Gradebook,
inspection, roster, and assignment data never come from a public
BlueprintCourse projection.

## WebAssembly facade

src/wasm/index.ts is the only browser import boundary for generated Wasm glue.
It exposes answer-free validation and formatting operations through typed
lower-camel-case helpers. It may validate response format, timing inputs,
assignment capability configuration, and presentation descriptors. It cannot
import grading, receive answers or keys, or make an authorization or release
decision. If Wasm is unavailable, the documented server validation fallback
is used only for non-authoritative format/timing help.

## Browser persistence

| Storage        | Allowed data                                                                 | Clear boundary                                       |
| -------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------- |
| localStorage   | Nonessential preferences after applicable consent                            | Reset or consent withdrawal                          |
| sessionStorage | Explicitly requested in-progress response recovery                           | Submit success, Assignment Attempt exit, or sign-out |
| neither        | Session tokens, keys, grades, undisclosed feedback, CourseInstance authority | Never stored                                         |

A BlueprintCourse draft remains in component/page state until the typed
repository reports success or a recoverable conflict. It is not a hidden
browser authority. Course appearance and authorization are loaded from
server-owned Course Route View data, not browser storage.

## Errors and accessibility

- Keep the shell and navigation visible during loading.
- Explain empty states with the next available action.
- Preserve local draft or response data for recoverable failures.
- Move focus to an error heading, then to the relevant retry or correction.
- Use semantic labels, fieldsets, legends, live validation text, and
  aria-invalid with visible explanations.
- Keep keyboard and pointer paths equivalent; no mouse-only picker or dialog.
- Keep primary targets at least 44 CSS pixels tall and focus contrast at the
  repository target.
- Keep Student layouts usable at maintained laptop, portrait, and narrow-phone
  profiles without horizontal overflow.
- Use the fixed 1280 by 800 desktop evidence profile for Instructor routes.

## Security rules

- The client bundle contains no answer-bearing generated type or private
  grading contract.
- Authentication and role preflight happen before protected data decoding.
- References locate BlueprintCourse or CourseInstance records; they never
  carry authority.
- Mutations use same-origin requests, strong revisions, and typed idempotency.
- Browser logs contain no response text, answer, key, undisclosed feedback,
  grades, email, UUID, or FERPA record.
- Supplied markup is sanitized server-side; Question Backends remain behind
  their server-owned transports.
- A public BlueprintCourse read never exposes CourseInstance delivery or
  Student state.

## Validation gates

The browser evidence hierarchy is:

1. Permanent TypeScript/Node checks for strict decoders, client behavior,
   route/reference binding, local draft preservation, and answer-free DTOs.
2. Production HTTPS Playwright for visible BlueprintCourse authoring, nested
   picker reuse, Fork Blueprint Course, Copy Assignment from Blueprint, Create Course from Blueprint, DST
   correction, unreleased propagation and explicit release, Apply Blueprint Update,
   and divergence recovery.
3. Screenshot publication and semantic visual review for hierarchy, focus,
   contrast, privacy, recovery, and source/destination clarity.
4. Human and independent architecture/security/HCI review where required.

Graphify and source/route inventories are one-time implementation evidence,
not permanent tests. A focused docs gate may validate Markdown links, ASCII,
and whitespace. Per [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md), an unrun
required runtime, database, browser, or human gate remains open.
