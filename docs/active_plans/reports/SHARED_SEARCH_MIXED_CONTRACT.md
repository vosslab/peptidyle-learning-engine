# M10 mixed Library search contract

This is a read-only implementation handoff for M10 in the
[active plan](../read-docs-active-plans-active-shared-sea-peaceful-mountain.md).
It describes the smallest change that makes the existing Library search API
capable of returning Questions and Pools together. M12 changes the Library
definition to request both kinds; M10 itself preserves today's visible Library
behavior.

## Wire and request contract

Keep `GET /api/questions/search` as the one Library page endpoint. Add two
closed query fields to the existing strict `QuestionSearchQuery` and generated
`QuestionSearchRequest`:

| Field | Values | Omitted/default at M10 | Meaning |
| --- | --- | --- | --- |
| `kind` | `both`, `questions`, `pools` | `questions` | Result kinds admitted to the one query. |
| `membership` | `noPool`, `all` | `all` | Question-row membership predicate. It has no effect on Pool rows. |

`questions` plus `all` deliberately preserves the old endpoint: callers that
do not know about M10 continue to receive all matching Questions. The M10
Library definition must explicitly continue to send that pair. M12 can switch
its starting query to `both` plus `noPool` without a transport redesign.

Add nullable `owner_account_id` using the existing canonical `AccountId`
syntax. It must be part of the same normalized query as `kind` and
`membership`, not a client-side filter. The SQL applies it to Question current
owner and Pool `owner_account_id`; it does not reuse Question authorship.
M10 needs no display-name lookup or account-search control: M12 can provide a
control only when an existing identity presentation contract supports it.

For the response, retain the existing question-only `QuestionSearchResult`
unchanged: it is also the exact nested Question payload used outside Library
search. Add a generated tagged `LibrarySearchResult` for page items instead.
Use the public tag `kind` and reject unknown tags and fields in the TypeScript
decoder:

```text
LibrarySearchResult =
  { kind: "question", question: QuestionSearchResult, ownerAccountId }
| { kind: "pool", pool: QuestionPoolLibrarySummary }

```

Reuse `QuestionPoolLibrarySummary` for the Pool payload, including its existing
metadata, edit numbers, owner, Type, Backend, calculated license, member count,
and Bloom assignment.

The Rust payload should name the variants `Question` and `Pool`, with
`#[serde(tag = "kind", rename_all = "camelCase")]`; ts-rs then generates the
matching discriminated TypeScript union. Keep Question-only source resolution,
capabilities, revision tuple, and statistics inside the nested `question`
payload. The nested `pool` payload uses Pool-owned metadata and its calculated
license. Do not create an optional-fields record: it permits invalid mixed rows
to reach the browser and leaves a future third kind ambiguous.

The page stays `{ items, nextCursor, facets }`. Add one compact category-count
object to its facets, with exactly `questionsInNoPool`, `questionsInPool`, and
`pools`. Each number counts the authorized candidates after every other active
filter, but before the `kind` selector, membership selector, cursor, or page
size. Question-only constraints are hard filters: when Author, Used in my
courses, capability, or authored-by-current-account is active, the Pool arm is
empty and the `pools` category count is zero. Thus a default `both` plus
`noPool` search can still display its matching Questions-in-a-Pool count as an
available alternate selection, while a Question-only constraint does not leave
unrelated Pools visible. Existing facets retain their current semantics over
the active final result predicate. A Pool must not be represented by the current SQL NULL
`published_question_id` facet carrier; use a separate explicit sentinel or a
one-row facet relation so a real Pool cannot be discarded.

Keep the visible `backends` facet mixed, because Backend is a valid Pool-owned
filter. Add an internal-only `question_backends` facet beside it in
`QuestionLibrarySearchFacets`; it counts the same eligible Question branch
before pagination. `crates/server/src/question_library/facets.rs` must derive
the public capability counts from `question_backends`, never mixed `backends`.
Pools do not advertise Question adapter capabilities, so a Pool backend count
must not inflate a capability count. Do not expose `question_backends` in the
generated browser facet DTO.

## Query, cursor, and lookup boundary

Extend the private/public `ple_api.search_question_library_entries` pair in
`schemas/base_schema/50_functions/question_library_operations.sql`. Each arm
first applies only predicates that belong to that kind, then `UNION ALL`s a
shared normalized row shape. Calculate facets from the eligible set and apply
the cursor and `LIMIT page_size + 1` only after the union. The existing
`question_pool_member` uniqueness and member index support an `EXISTS` test by
`published_question_id`; that test deliberately ranges over every Pool,
including Assessment forks.

Question arm rules:

- `membership=noPool` means `NOT EXISTS` a member row for the Question in any
  Pool; `all` does not add a membership predicate.
- Author, course-use, capability, and authored-by-current-account predicates
  are Question-only constraints: their presence excludes every Pool candidate.
- Text or Tag evidence in a member matches its Question row under `all`; it
  never makes the containing Pool match.

Pool arm rules:

- Text, classification, Tags, Type, Backend, license, Bloom, and owner use
  the Pool's own stored values only.
- Pool `newest` is `question_pool.created_at`; Question `newest` remains its
  publication timestamp.

The server's existing text parser remains the sole parser. It produces typed
terms before the store call; SQL applies each term to the matching Pool-owned
title, description, classification, or Tags field. A parsed `author:` term is
Question-only and eliminates Pool candidates, just like the Author filter. No
term is ignored on the Pool arm and the client never filters decoded rows. A
member title, description, or Tag is never Pool text evidence.

Widen `QuestionLibrarySearchCursorPosition` and its wire envelope so both
variants carry the same final identity type: the globally reserved public ID,
not a nullable Question ID. The exact two keysets are:

```text
title ascending: stored title COLLATE "C" ASC, public_id ASC
newest:          sort_timestamp DESC, public_id ASC
```

Use the same timestamp precision that SQL returns and Rust binds (integer epoch
milliseconds in the current cursor); do not round Pool `created_at` differently
from Question `published_at`. Public-ID reservation across Question and Pool
lineages makes the second key unique. Version the opaque cursor because its
serialized position changes, and retain the digest over the complete normalized
request, including `kind`, `membership`, and owner. This prevents a cursor for
one branch or membership choice from being reused for another.

Add `GET /api/library/{id}/kind` (or the exact `library` route group used by
M11) as an authenticated Library-read endpoint. It accepts the existing
canonical public-ID syntax and resolves the actual visible Published Question
or Pool under the same Instructor read authority as search. The public-ID
reservation table remains useful for global cursor identity, but it is not a
lookup result: a reserved draft or failed creation must not resolve. Return a
strict `{ kind: "question" | "questionPool" }` only for an accessible Library object;
otherwise return the normal not-found/forbidden result without exposing object
kind. Keep this lookup next to `crates/server/src/question_library.rs`; it is a
routing decision, not a second client-side search.

## Existing change points

| Boundary | M10 change |
| --- | --- |
| `crates/question_model/src/question_search.rs` | Query enums/defaults, complete cursor-bound filter digest, generated `LibrarySearchResult`/Pool payload and category counts; preserve `QuestionSearchResult`. |
| `crates/learning-data-access/src/question_library.rs` | Store request, mixed store entry enum, cursor identity, and facets. |
| `crates/learning-data-access/src/postgres/question_library/search.rs` | Bind the two new filters, decode the explicit facet carrier and both row variants, make the next position from the last mixed row. |
| `schemas/base_schema/50_functions/question_library_operations.sql` | One authorization-preserving combined SQL query and kind lookup function/grant. |
| `crates/server/src/question_library/query.rs`, `paging.rs`, and `question_library.rs` | Strict query parsing/defaults, cursor version/position, route, and branch Question source/statistics work away from Pool rows. |
| `src/api/question_search_query.ts` | Allowlist and validate `kind`, `membership`, and owner; serialize defaults explicitly when the page needs a stable replayable request. |
| `src/api/decoders/question_library.ts` | Strict `kind` switch with per-variant `requireOnlyFields`, then strict category-count decoding. |
| `src/api/question_library_repository.ts` | Map the discriminated generated response into the Library adapter. Keep M10's definition question-only; do not add M12 controls here. |

## Connected test seams

Add the SQL/store oracle cases in
`crates/learning-data-access/tests/blueprint_course_postgres/question_library.rs`.
Its existing tied-title, 65-row fixture is the right place to add Pool rows,
an Assessment-fork-only member, tied title and timestamp values, and both sort
and membership traversals. Assert that every public ID occurs exactly once.

Keep transport defaults, unknown enum rejection, cursor query binding/version,
kind lookup authorization, source-read avoidance for Pool rows, and
Question-only statistics in `crates/server/src/question_library/tests.rs`.
The existing `PageOnlyLibrary` route test is the natural regression seam for
ensuring a Pool page does not call `entries_to_summaries` or page statistics for
Pool IDs. That file is already large: put the complete mixed-search route
fixture and its tests in a sibling `crates/server/src/question_library/mixed_search_tests.rs`
module, leaving focused cursor/unit coverage in `tests.rs`.

The connected cases must also prove that an active Author, course-use,
capability, or authored-by-current-account constraint returns no Pool rows and
sets `pools` to zero; that `author:` does the same; and that Pool-only text and
Tag matching never traverses Pool members. The kind-lookup route must reject a
reserved-but-not-visible ID and must not identify an inaccessible object.
Add a mixed-backend fixture proving that visible backend facets include both
kinds while capability facets derive from Question rows only.

Use `tests/test_library_classification_search.mjs` for request serialization,
strict mixed-response decoder fixtures, and adapter mapping. Include rejected
unknown tags, missing required Pool fields, surplus variant fields, and an
omitted `kind`/`membership` request that serializes or normalizes to the M10
Questions/all behavior. Regenerated `generated/api/` files are contract output,
not hand-edited test fixtures.

## Bounded ownership split

**Backend owner:** SQL function and grants; question-model/store enums and
mixed types; SQLx binding/decoding; server normalization, cursor, kind lookup,
and connected Rust tests; regenerate API types. This owner is responsible for
the combined-query correctness and before/after `EXPLAIN (ANALYZE, BUFFERS)`
and HTTP baseline comparison recorded in `SHARED_SEARCH_SPEED.md`.

**TypeScript adapter owner:** after regenerated types land, update only
`question_search_query.ts`, `decoders/question_library.ts`,
`question_library_repository.ts`, and their focused MJS contract fixtures.
Keep the M10 Library definition on `kind=questions, membership=all`; leave M12
filter controls, mixed-row presentation, and links to their later milestones.
The existing `decodeQuestionSearchResult` remains for Question-only embedded
payloads in `src/api/decoders/blueprint_course.ts` and
`src/api/decoders/question_pool_library.ts`; add a separate strict
`decodeLibrarySearchResult` only for search-page items. Every Library adapter
fixture that reads an item summary must first narrow `item.kind` and then use
`item.question.summary`.

`crates/question_model/src/question_search.rs` is also near the repository's
file-size limit. Put the new Library-kind, membership, category-count, and
mixed-result contract in a focused sibling module such as
`question_search/library_mixed.rs`, re-exported from the current module, rather
than extending the existing file past 1,000 lines.

The handoff point is the generated tagged page type plus endpoint path and
strict JSON examples from the backend owner. That keeps SQL/Rust and the
browser adapter independent without duplicating a provisional mixed-row model.
