# Plan: Ribbon destination completion

## Context

The Questions Ribbon already names My Questions and Starred, but both remain future destinations.
Current code distinguishes their backend work: `QuestionSearchAuthorship::AuthoredByCurrentAccount`
already exists and is enforced by Question Library summaries, while question stars exist as
stewardship data but the Question search request has no current-account-starred filter.

The repository fast gate is red. Stabilization must complete before this destination work begins.

## Objectives

- Deliver My Questions as an existing authored-by-current-account Question Library query.
- Deliver Starred as a separate current-account-starred Question Library query with its own data-access boundary.
- Keep the two Ribbon destinations visually similar while preserving their different backend meaning.

## Design philosophy

Reuse the Question Library's established result surface and filters; distinguish authored ownership
from personal stars in the query contract. Treating Starred as a client-side filter is rejected
because a library page does not necessarily hold every matching Question.

## Scope

- Route My Questions to a library query with `AuthoredByCurrentAccount` and no new backend filter.
- Add a closed `starred_by_current_account` filter, SQL predicate, API/grant path, decoder/type
  propagation, and Ribbon destination for Starred.
- Measure the Starred query with seeded data and add an index only when `EXPLAIN` demonstrates need.
- Preserve the current question-star write/read stewardship behavior.

## Non-goals

- Do not combine authored and starred results into one destination.
- Do not add saved searches, tags, notifications, or a new collection workflow.
- Do not add an index merely because a new filter exists.
- Do not execute while stabilization remains unresolved.

## Current state summary

`crates/question_model/src/question_search.rs` defines `AuthoredByCurrentAccount`; the Question
Library summary path already recognizes it. `ple_data.question_star` stores the current instructor's
stars, and `40_indexes.sql` already indexes star records for instructor/time/question access. The
Questions Ribbon catalog currently represents `starred` as a future destination.

## Approach

1. After stabilization, make My Questions a route/catalog wiring package that creates the existing
   authored query and reuses the library results surface.
2. Add one closed search facet for `starred_by_current_account`, propagate it from Rust contract to
   generated TypeScript, SQL query predicate, API, and grant. The predicate must bind the current
   authenticated account, never a client-provided account ID.
3. Use the same library page result behavior for the Starred route, with an explicit destination
   label and filter state.
4. Run `EXPLAIN` on representative seeded Starred data. Keep the present index if it supports the
   plan; otherwise add only the measured index required by the query.

## Critical files

- `crates/question_model/src/question_search.rs` and `crates/server/src/question_library/`.
- `schemas/base_schema/50_functions/`, `schemas/base_schema/70_grants/`, and `40_indexes.sql`.
- `src/pages/library_page_model.ts`, Question Library API/decoder files, and `src/ribbon/ribbon_catalog.ts`.

## Acceptance criteria and gates

- My Questions returns only published Questions authored by the signed-in account through the
  existing authorship contract.
- Starred returns only Questions starred by the signed-in account through a server-enforced predicate
  and grant.
- Neither route accepts another account's identity as a filter input.
- My Questions and Starred preserve ordinary Question Library result navigation and return state.
- An index change has an attached seeded-data `EXPLAIN` comparison; absent evidence means no index change.
- Fast checks, focused query/API checks, authorization checks, and `git diff --check` pass.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Authorship and starring are conflated | Wrong collection membership | Shared query field is proposed | query-contract owner | Keep two named predicates and route filters |
| Star filter leaks another instructor's collection | Authorization defect | Predicate binds a request value | database owner | Bind only session account identity and test denial |
| Speculative index adds write cost | Unneeded schema complexity | No `EXPLAIN` evidence | database owner | Retain existing indexes unless measurement requires one |

## Verification

The fast gate must first be green. Then test both routes as two instructors with disjoint authored
and starred sets, verify the generated contract and grants, record `EXPLAIN` evidence if an index is
proposed, and inspect the rendered Ribbon destination/result labels.
