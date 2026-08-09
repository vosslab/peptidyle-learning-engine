# Human guidance

This file records durable project guidance from the repository owner. Apply it
alongside [AGENTS.md](../AGENTS.md) and the active implementation plan.

- Source code files should be less than 1000 lines in length; if the get longer
  than that organize and distribute the content over more files

## Plan status

- Treat `docs/active_plans/implementation_plan.md` as the source of truth for
  implementation order, architecture, contracts, security, tests, and gates.
- `docs/active_plans/m0-results.md` is concluded M0 evidence. Read it when M0
  history matters; do not treat it as an active task or reopen M0 without new
  evidence.
- Finish and validate one work package before advancing to its dependency-order
  successor.

## Retention defaults

- Ship the privacy-first course lifecycle defaults: notify after 30 days,
  archive student records after 100 days, and permanently delete them after
  365 days. Institutions may later configure their own ordered policy.
- Retain tenant-owned assignment definitions by default when student records
  archive or delete. A later archive workflow may offer an explicit owner
  choice without following references into shared published content.

## Agent-specific guidance

- Codex follows `AGENTS.md` and the repository style documents.
- `docs/CLAUDE_HOOK_USAGE_GUIDE.md` is specific to Claude tooling and does not
  govern Codex commands or file-search behavior.

## Local services

- Podman is normally running on the owner's machine.
- Use the local containers when the active work package reaches a documented
  PostgreSQL, MinIO, health, tenancy, or other container-dependent gate.
- Keep offline contract work on memory backends when its work-package gate does
  not require containers.

## Teaching and product priorities

- The product supports learning through repeated algorithmic practice. A first
  completion or a 100 percent score must not end continued practice when policy
  permits another run.
- Fresh variation is more important pedagogically than seed replay. Give every
  newly issued parameterized question instance a fresh server-owned seed;
  preserve an existing attempt's seed only for resume, re-render, audit, and
  debugging of that same instance.
- Preserve server-only grading and answer secrecy. The browser may validate
  response format but must not receive answer keys or grading implementations.
- Keep student and course records tenant-owned while published educational
  content remains shared and immutable.
- Favor behavior-focused evidence that reflects what instructors and students
  actually do over implementation-detail tests.

## Performance choices

- When measured behavior is slow, consider implementing the hot path in Rust
  or WebAssembly.
- Keep the security boundary intact when optimizing: deterministic generation,
  response-format validation, timer display, and state transitions may run in
  WebAssembly; answers, keys, and correctness decisions remain server-only.

## Score precision and display

- Use `f64` for scoring calculations and `AttemptResult` across Rust, WebAssembly,
  and browser projections. Do not replace ordinary score arithmetic with `f32`
  or a scaled-integer points model.
- Round computed current points explicitly to at most four decimal places before
  persistence. Keep PostgreSQL `NUMERIC` as the rounded storage boundary without
  forcing general Rust scoring code to use fixed-point arithmetic.
- Exact decimal command boundaries, such as a manually entered credit fraction,
  may retain up to 12 decimal places. They do not require the rest of the score
  model to use decimal arithmetic.
- Display scores and percentages with at most two decimal places and trim
  trailing zeroes. Show `8 / 10`, `8.5 / 10`, or `8.33 / 10`, never a binary
  floating-point artifact such as `8.0000000000006 / 10`.
- Choose one explicit midpoint-rounding rule before implementation and cover the
  same boundary examples in Rust and TypeScript so server and browser output
  cannot disagree.

## Software design

- Focus software design on adaptability, allowing systems to evolve with
  changing requirements and insights over time.
- Use adaptability to maintain functionality and relevance in a dynamic usage
  environment.

## Authentication storage and compliance

- Store the opaque authentication credential in one host-only HttpOnly cookie,
  not in `localStorage`. JavaScript must never be able to read the bearer
  credential.
- Use the cookie only for authentication, session security, expiration, and
  revocation needed to provide the signed-in service. Do not attach analytics,
  advertising, cross-site tracking, or unrelated preference data to it.
- Treat `localStorage` and similar browser mechanisms as storage/access
  technologies too; changing the browser API is not a way around European
  storage-consent rules.
- Classify the authentication cookie as strictly necessary only while it is
  essential to the service explicitly requested by the user and has no
  secondary purpose. Clearly disclose its name, purpose, deployment context,
  and lifetime even when prior opt-in consent is not required.
- Make ordinary authentication a browser-session cookie by default while the
  server retains an authoritative, bounded expiration. Do not assume that a
  persistent login cookie is exempt: any `remember me` behavior requires an
  explicit user choice and a jurisdiction-specific consent and legal review
  before implementation.
- Require separate consent handling before adding any nonessential browser
  storage or tracking. Recheck the deployed behavior against the target
  jurisdiction; this engineering rule is not a substitute for legal review.
- Keep the technical controls narrow: `Secure; SameSite=Lax` for ordinary
  HTTPS, explicit `SameSite=None; Secure` only for configured LTI embedding,
  explicit insecure mode only for local HTTP development, and immediate
  server-side revocation on sign-out.

The durable regulatory references for this decision are Article 5(3) of the
[consolidated EU ePrivacy Directive](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:02002L0058-20091219), the
[Article 29 Working Party Opinion 04/2012](https://ec.europa.eu/justice/article-29/documentation/opinion-recommendation/files/2012/wp194_en.pdf),
and current [ICO guidance on strictly necessary storage/access](https://ico.org.uk/for-organisations/direct-marketing-and-privacy-and-electronic-communications/guidance-on-the-use-of-storage-and-access-technologies/what-are-the-exceptions/).

## Dependency versions

- Focus on the latest versions of all code because many security bugs are being
  fixed.
- Never pin versions; `>=` version requirements are acceptable.

## Generated artifacts

- Put reproducible generated content under the repository-root `generated/`
  directory and keep that directory out of Git.
- Regenerate required artifacts through their tracked owning generator before
  builds and validation; ignored output must not become an unverified input.
- Link documentation to the tracked generator or authoritative source rather
  than to files under `generated/`, which do not exist in a clean checkout.
- Track small, deliberately reviewed golden baselines when they define a
  compatibility contract or record work evidence. These are authoritative test
  inputs rather than disposable generated build output.
- Treat `tests/fixtures/published_problem/` as reviewed cross-layer test
  evidence. Keep its fully derivative TypeScript projection under ignored
  `generated/fixtures/` and regenerate it before TypeScript validation.
