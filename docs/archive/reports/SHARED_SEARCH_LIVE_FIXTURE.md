# Shared search Live Demo fixture

`tests/e2e/e2e_live_demo_question_library.sh --api` creates this one durable
Live Demo fixture only when a Pool with the exact title is absent:

| Pool title | Exact member |
| --- | --- |
| `Shared Search Live Verification Pool` | `Genetic disorders: Which one?`, at the current published revision returned by the Instructor-only Question search |

The test first searches `kind=pools` for that exact title. It creates a
one-member Pool through `POST /api/question-pools` only when no row exists,
then requires exactly one matching Pool and exactly one member. Repeated runs
reuse the same Pool. The test intentionally does not delete it because the
public Library API has no Pool-delete command.

This is an intentional, small Live Demo dataset delta. It supplies one actual
Pool and one actual Pool member for mixed Library acceptance: `both/noPool`
shows the Pool while hiding its member, `both/all` shows both, and `pools`
returns the Pool alone. The separate eight Pilot checks explicitly request
`kind=questions&membership=all`, so their source-controlled Question contract
does not depend on the default membership display.
