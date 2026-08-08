# Security model

Peptidyle keeps grading authority on the server. The browser may determine
whether a response is structurally ready to submit, but it never receives an
answer key or makes a correctness decision.

## Grading boundary

| Browser-safe surface                           | Server-only surface                |
| ---------------------------------------------- | ---------------------------------- |
| `ResponseDefinition` and `StudentResponse`     | `grading::AnswerKey`               |
| Parameter generation from a supplied seed      | Expected numeric values            |
| Response-format validation                     | Correct choice IDs and ordering    |
| Timer display and pure state transitions       | Accepted text and private rubrics  |
| Correctness and point results after disclosure | Checkers and correctness decisions |

The browser-safe model explains the input shape and public grading policy. For
example, it may reveal a numeric tolerance or that exactly two choices are
required. The expected number and the two correct choice IDs remain in
`crates/grading`.

`crates/grading` is the only answer-bearing crate. Ungraded content has no
`AnswerKey`; it does not use a browser-safe placeholder key. Native H5P remains
ungraded practice because its own evaluation runs in the browser.

`grading::grade(question, response, key)` repeats browser-safe format
validation before consulting the key. Its generic all-or-nothing checker owns
numeric exact, absolute, relative, and significant-figure comparisons;
multiple-choice set comparison; declared short-text matching; and exact
ordering. It returns only correctness and points. Partial-credit questions are
referred to a capable backend or explicit private rubric, and file uploads are
referred for manual review; the generic checker fails explicitly instead of
inventing either policy.

## Format validation

`domain::validation::validate_response_format` checks only student-controlled
structure:

- response kind matches the definition;
- numeric input is finite;
- selection count, uniqueness, and IDs are valid;
- short text fits its character limit;
- ordering is an exact permutation of the displayed items; and
- an uploaded response carries a server-issued object reference.

This function has no answer-key parameter and cannot determine correctness.
The browser calls it through `wasm_bridge::validate_response_format`; the
server repeats it before grading because client validation is a convenience,
not an authority. File size, extension, checksum, and ownership are checked by
the server against object metadata rather than trusted from the browser.

## Compile-time closure

The shipped workspace dependency closure of `wasm_bridge` is exactly:

```text
wasm_bridge
+-- domain
|   `-- question_model
`-- question_model
```

It contains no `grading` crate. `tests/test_crate_boundaries.py` resolves the
normal, build, and target-specific local dependency tables conservatively and
fails if any other workspace crate enters this closure. Including build
dependencies matters because a build script could otherwise embed secret data
without becoming a runtime dependency.

Run the closure gate with the repository Python environment:

```bash
source source_me.sh && python3 -m pytest tests/test_crate_boundaries.py
```

## Export allowlist

`tests/test_wasm_export_allowlist.mjs` builds the current bridge, processes it
with the lockfile-matched `wasm-bindgen` tooling, and compares every export name
and kind with a committed allowlist. Its disposable processed module lives
under ignored `generated/wasm-export-check/`.

The reviewed application exports are currently:

- `bridge_version`;
- `timer_verdict`;
- `validate_assignment_config`; and
- `validate_response_format`.

The allowlist also names the exact memory, table, allocator, and lifecycle
exports required by `wasm-bindgen`. A new Rust export fails the gate until a
reviewer determines that it is key-free and deliberately updates the list.
An answer-bearing export is rejected rather than added.

`timer_verdict` is safe in the browser because its inputs are already disclosed
timer policy and server timestamps, and its output cannot reveal an answer.
The server still supplies the authoritative evaluation timestamp and decides
whether to accept a submission; browser time remains display-only.

`validate_assignment_config` receives only question definitions and backend
capability declarations already shown to an instructor. Its violations name a
question version and a missing capability, never an answer or grading key. The
server independently calls the same domain function before publication.

Run the export gate directly:

```bash
node --test tests/test_wasm_export_allowlist.mjs
```

Both boundary gates run from `./check_codebase.sh`.

## Authentication and tenant derivation

Authentication uses one host-only opaque session cookie. The raw 256-bit
credential is generated from the operating-system random source, marked
HttpOnly, and never enters browser `localStorage`, logs, or PostgreSQL. The
cookie has no `Max-Age` or `Expires` attribute, so ordinary authentication is
limited to the browser session. Shared session storage contains only its
SHA-256 hash and database-authoritative creation, bounded expiration, and
revocation state.

The normal HTTPS cookie policy is `Secure; SameSite=Lax`. LTI embedding must
explicitly select `SameSite=None; Secure`, and plain HTTP requires the named
local-development mode. Credential-provider implementations own their
protocol's replay and CSRF checks; the generic login route also bounds a
presentation body to 64 KiB.

`SameSite=None` is not a CSRF defense. Ordinary browser mutations use
same-origin JSON requests and the server must not add permissive credentialed
CORS. Before embedded LTI mode is composed for production, its state-changing
requests require an origin-bound anti-CSRF mechanism in addition to the LTI
launch protocol's state, nonce, and replay validation.

The authentication cookie has no analytics, advertising, tracking, or
preference purpose. Nonessential storage, including `localStorage`, requires a
separate consent path. Persistent `remember me` behavior is not part of the
ordinary session contract and requires explicit user choice plus a
jurisdiction-specific compliance review before implementation.

Before authentication, the PostgreSQL `ple_auth` role can see only the
`auth_session` row matching the presented one-way hash. Resolving that row is
the only production path that constructs `TenantContext`; tenant values from
URLs, headers, or JSON never establish RLS context. Missing, malformed,
unknown, expired, and revoked credentials all return the same unauthenticated
response.

## Catalog publication boundary

Every catalog route resolves the session before deriving `TenantContext`.
Request paths and bodies cannot select another tenant. Institution-visible
metadata and payloads are protected by forced PostgreSQL RLS and an exact
tenant/problem/version grant; the context-free store methods expose public
content only.

The browser supplies a workspace identifier and requested publication scope,
but never a new `ProblemId` or a backend capability declaration. The server
loads the tenant-owned draft, resolves capabilities from its trusted adapter
registry, returns the complete capability-violation list, and generates a
fresh problem identifier for a new work or fork. The store compares and locks
the same draft before committing metadata, immutable payload, visibility
grant, and draft deletion in one transaction.

Public publication requires a publisher or administrator role plus the
institution's optional review gate. Institution publication permits an
instructor, publisher, or administrator. Post-publication transitions require
both an eligible role and author ownership. Database privileges permit only
the lifecycle fields to change; published identity, scope, payload,
capabilities, metadata, authorship, and lineage cannot be updated or deleted by
the application role.

Catalog browse responses contain hot browser-safe metadata only. They expose a
backend family but no native family name, WeBWorK path, QTI package identifier,
H5P package identifier, prompt, response definition, or answer-bearing value.
Deprecated and archived versions are hidden from browse, but exact authorized
lookup remains available for historical records. Deprecation remains usable by
an exact new reference; archival additionally blocks new assignments.

## Course authorization boundary

Every course route resolves the shared session before constructing
`TenantContext`; no request may choose a tenant. A coarse instructor role may
create a course, but access to an existing course comes from a tenant-owned
`course_member` row. Tenant administrators may inspect every course through a
separate effective-authority path. Administrator authority is not a storable
membership variant, so a membership write cannot accidentally manufacture
tenant-wide access.

Course and membership tables use forced tenant RLS. Nonmembers receive the same
not-found response as absent courses, limiting identity disclosure. Students
may list and resolve assignments in their courses but receive a forbidden
response for assignment creation. Assignment writes validate every exact
problem/version reference against catalog visibility and lifecycle state; no
question payload, answer key, or grading code is copied into the course row or
returned by browse.

## Run authorization and grading boundary

Run mutations require the authenticated `UserId` stored on the enrollment;
they never infer authorization by equating that identity with `StudentId`.
Course instructors and tenant administrators may read enrollment history and
summaries, but only the enrollment owner may start or submit a run. Nonowners
receive not found so record existence is not disclosed.

Each newly issued attempt receives an operating-system-random seed. Resuming
an unresolved attempt returns its stored seed and provenance, and the store
locks the run so only one unresolved question exists at a time. Server-owned
database timestamps determine issue time, deadline, response arrival, and run
completion.

The server repeats structural response validation before calling the injected
grading backend. Submission persistence rejects malformed point values and
atomically commits the response, grade event, run and enrollment transitions,
and summary. The idempotency table is insert-only for the application role;
an exact retry returns its first committed receipt, while a changed key or
response conflicts.

Attempt responses are redacted according to the question's feedback policy.
They may contain policy-permitted correctness and points, but never an answer
key, expected value, private rubric, or checker state. Full teaching feedback
must later use an explicit sanitized disclosure DTO; it must not serialize the
server-only key as a shortcut.

## Asset delivery boundary

Browser markup carries an internal logical `AssetId`, never a bucket name,
physical key, or signed URL. `/api/assets/{id}` resolves the identifier through
the database-authoritative immutable registry. The registry accepts only a
`ProblemAsset` whose problem, version, asset, object, bucket, and category all
agree, or a tenant-matching `StudentRecord`; source packages, render caches,
and `temp-processing` objects cannot be registered for this route.

Globally public catalog assets redirect to the configured immutable CDN URL
without authentication or an object-store signing call. Institution content
and student records require the opaque HttpOnly session. Forced RLS limits the
candidate row, the store checks the authenticated user where the record has an
explicit user grant, and missing or unauthorized protected objects both return
not found. Every successful protected authorization appends an audit event
before requesting the signed URL. The event includes tenant, actor, delivery
ID, object ID, bucket, and database timestamp, but never the cookie or URL.

The object backend selects the lifetime from the typed bucket, and the route
rejects a result that exceeds 60 minutes for `content` or 5 minutes for
`student-records`. Protected redirects use `Cache-Control: no-store`,
`Pragma: no-cache`, and `Referrer-Policy: no-referrer`; public redirects use an
immutable cache policy and checksum ETag. Signed URLs are response headers
only and must not enter JSON, application logs, browser storage, or persisted
markup.

## Placement rule

Place new code according to the information it needs and the decision it
makes:

- Put response parsing and structural validation in `crates/domain` when the
  result is independent of a correct answer.
- Put expected values, accepted answers, grading rubrics, partial-credit
  weights, and correctness decisions in `crates/grading`.
- Expose a domain function through `crates/wasm` only when all inputs and
  outputs are safe for a student to inspect.
- Return correctness and points through server-controlled feedback policy;
  never return the key or checker state.

When uncertain, ask whether the value would help a student infer the correct
response before submission. If yes, it belongs on the server-only side.

## Other controls

WP-C6 proves the source and WebAssembly boundary. MOD-API-AUTH, MOD-API-CAT,
MOD-API-COURSE, MOD-API-RUN, MOD-SCHEMA, MOD-STO, and MOD-OBJ now add
authentication, catalog, course, and run authorization, PostgreSQL answer-table
grants, forced tenant row-level security, and signed object URLs. Later work
packages still add authorization for asset routes, sanitized supplied markup,
content security policy, and browser network-trace inspection. None of those
controls weakens the crate boundary established here.
