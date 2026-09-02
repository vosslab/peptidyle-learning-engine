# WP-A1 native retry corpus

## Scope

- Package: WP-A1, native retry-corpus arrangement.
- Owner: TypeScript/API test engineer.
- Status: independently ACCEPTED through the accepted M3 live integration.
- Files: `tests/playwright/simulator/retry_corpus.ts` and its focused spec.

## Contract

- A fresh isolated Playwright API request context authenticates an instructor only
  in memory, creates one fresh workspace UUID, and disposes the context on every path.
- The authoring request sends a private `pleQuestionJson` single-choice source with
  unlimited attempts, immediate full feedback, and untimed timing. A named WP-V1
  RNG stream selects its content without putting the answer in returned metadata.
  In every private variant, the first choice is incorrect and the second is correct,
  so J1/J2 can use one stable visible retry sequence without grading reconstruction.
- Save requires HTTP 200 and one strong positive decimal i64 ETag. Publication sends
  that exact ETag, requires HTTP 201, and is never automatically retried after an
  ambiguous outcome.
- A safe Question Details GET by canonical Question ID is inspected for answer-bearing fields. The
  returned retry-corpus result has only that Question ID, its public Question title, and arrangement
  label; it never includes a credential, cookie, source, answer key, QuestionId, or QuestionRevisionNumber.
- This package creates no account, enrollment, course, membership, SQL fixture, or cleanup action.

## Evidence

- Focused Playwright behavior tests cover login, exact request paths and headers,
  retry-capable policy, positive-i64 ETag rejection, public-payload secrecy including nested
  mixed-case singular/plural answer-bearing aliases, staged errors,
  redaction of transport/disposal failures, no second publish, and disposal.
- Independent offline review accepted the request contract, strict ETag handling,
  answer-bearing detail-payload rejection, and redaction boundary.
- The accepted M3 runner invoked this module through the fixed arranger without
  Playwright global setup. Its two clean same-seed manager passes and independent
  live pass are recorded in [m3_arrangement_integration.md](m3_arrangement_integration.md)
  and its [M3 review](../audits/m3_arrangement_integration_review.md).
