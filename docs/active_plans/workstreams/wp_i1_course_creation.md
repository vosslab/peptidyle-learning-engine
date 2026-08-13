# WP-I1 visible course creation

## Scope

This work package adds only the instructor-visible course-creation surface for
the Fall pilot. It uses the existing authenticated `POST /api/courses` route.
Email, account creation, roster activation, assignment construction, runner
work, and any authorization-policy expansion are outside this package. The
existing course-create handler owns one matching request-boundary repair:
unknown fields and whitespace-only titles are rejected before persistence.

## Contract

- The browser submits only `{ "title": string }` through a strict request
  decoder; the server accepts the same exact, non-whitespace request shape and
  returns only a strict public course summary response.
- Only sessions with the approved Instructor or Sysadmin role render the
  native labelled form and Create course button.
- A direct student POST receives `403` and persists no course.
- A recoverable error retains the typed title and announces recovery status.
- Successful creation inserts the server-returned course and focuses its real
  Open course link. Native form and link keyboard behavior provide the primary
  keyboard path.

## Validation

- Run the focused HTTP client and Playwright course-creation tests.
- Run direct server request regressions for blank titles, unknown fields, and
  student authorization with a no-persistence assertion.
- Run strict TypeScript, ESLint, Prettier, ASCII, Markdown-link, source-line,
  and diff checks before integration.

## Review repair

The independent review found that the browser's strict title decoder was not
yet mirrored by the production route. The server request model now denies
unknown fields and the handler rejects whitespace-only titles before it calls
the Store. A separate focused Rust test proves those two rejections, a student
POST returns `403` without persistence, and a Sysadmin can still create
a course. The test lives in `course_creation.rs` so the broader course test
module remains below the repository source-file limit.
