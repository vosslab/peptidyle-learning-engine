# WP-I1 visible course creation independent review

## Verdict

**ACCEPTED after repair.** The browser interaction is usable and
keyboard-complete, and the public server endpoint now enforces the matching
strict title contract before persistence. A direct authorization regression
also proves a student POST cannot create a course.

## Scope and method

This review evaluated the Fall-pilot WP-I1 course-creation task in
`docs/active_plans/peptidyle-walkthrough-plan.md` and
`docs/active_plans/workstreams/wp_i1_course_creation.md`. It used a cognitive
walkthrough of an instructor creating a course by keyboard, a strict
TypeScript/API-boundary inspection, and a role/route inspection. Email,
canonical account registration, roster activation, and assignment construction
were intentionally excluded.

## Findings

### P1: direct server callers could bypass the strict title contract (resolved)

At the original review, `src/api/decoders/catalog_course.ts:436-446` accepted
only a non-whitespace `title` field and
`src/pages/course_list_page.tsx:53-57` prevented a whitespace-only form
submission, but the server did not match that boundary. A direct authenticated
request could create a blank-title course or include ignored extra fields.

Repair verified: `CreateCourseRequest` now uses `deny_unknown_fields`, and the
handler returns `422` for a whitespace-only title before it calls the Store.
`crates/server/src/course/tests/course_creation.rs` proves both blank-title
and unknown-field rejections without persistence.

### P2: server authorization lacked a direct student-POST regression (resolved)

The implementation authorization is correct by inspection:
`crates/server/src/course/queries.rs:58-63` resolves the session and calls
`may_create_course`, while `crates/server/src/course/policy.rs:69-75` accepts
only global instructor or administrator roles. The browser test also proves a
student sees no control and makes no POST. But the only direct
`POST /api/courses` server cases in `crates/server/src/course/tests/mod.rs`
used an instructor session. Repair verified: the focused course-creation test
now proves a student POST receives `403` without persistence and an
administrator POST succeeds. The test is split into
`course_creation.rs`, keeping the course test module within the source-size
policy.

## Confirmed behavior

- `CourseCreateInput` has one field and the client calls the existing public
  `POST /api/courses` endpoint with strict input and strict course-summary
  decoding (`src/api/contracts.ts:72-74`,
  `src/api/http_client/request.ts:395-399`, and
  `src/api/decoders/catalog_course.ts:415-446`).
- Only an authenticated global instructor or administrator sees the native,
  labelled form; a learner renders neither form nor button. The production
  Playwright learner test also observes no creation POST
  (`src/pages/course_list_page.tsx:42-49,82-106` and
  `tests/playwright/course_creation.spec.ts:100-119`).
- Native label/input/form semantics permit Tab and Enter. The control becomes
  disabled while pending, the form exposes `aria-busy`, and its polite live
  region communicates pending and recoverable error state. The typed value is
  preserved on failure (`src/pages/course_list_page.tsx:51-71,83-105`).
- On success the server-returned course is inserted as a real `/courses/:id`
  link and focus moves to that link. The production browser test then opens it
  with Enter (`src/pages/course_list_page.tsx:57-60,113-122` and
  `tests/playwright/course_creation.spec.ts:50-98`).
- The course list has no course-scoped route state. `App` keys content by path,
  so entering a course and returning to `/` constructs a fresh list surface;
  a created link is not reused across course routes (`src/app.tsx:188-229` and
  `src/routes.ts:27-51`).
- No WP-I1 source introduces email, mailbox, invitation, credential, or
  identity-account behavior.

## Validation evidence

| Command | Result |
| --- | --- |
| `npx tsc --noEmit` | Passed with exit 0 and no diagnostics. |
| `npx eslint --max-warnings 0 src/pages/course_list_page.tsx src/api/contracts.ts src/api/decoders/catalog_course.ts src/api/http_client/request.ts tests/playwright/course_creation.spec.ts tests/test_http_client.mjs` | Passed with exit 0. |
| `npx prettier --check src/pages/course_list_page.tsx src/api/contracts.ts src/api/decoders/catalog_course.ts src/api/http_client/request.ts tests/playwright/course_creation.spec.ts tests/test_http_client.mjs` | Passed: `All matched files use Prettier code style!` |
| `node --import tsx --test tests/test_http_client.mjs` | Passed: 22 tests, 22 passed. |
| `npx playwright test tests/playwright/course_creation.spec.ts` | Passed: 3 tests, 3 passed. |
| `cargo test -p server_core course::tests::membership_scopes_courses_and_exact_assignment_references_survive` | Passed: 1 test passed. It establishes instructor success but not the missing student-create regression. |
| `source source_me.sh && python3 -m pytest -q tests/test_source_file_line_limit.py` | Passed: `816 passed in 0.27s`. |

## Re-review evidence

Fresh re-review of the repaired boundary passed:

| Command | Result |
| --- | --- |
| `cargo test -p server_core course::tests::course_creation` | Passed: 1 test passed. |
| `cargo test -p server_core course::tests::membership_scopes_courses_and_exact_assignment_references_survive` | Passed: 1 test passed. |
| `cargo fmt --check` | Passed with exit 0. |
| `cargo clippy -p server_core -- -D warnings` | Passed with exit 0. |
| `git diff --check` | Passed with exit 0. |

The non-canonical `node --test tests/test_http_client.mjs` invocation failed
because it does not load TypeScript source imports. The repository's prescribed
Node lane uses `--import tsx`; that canonical invocation passed above.
