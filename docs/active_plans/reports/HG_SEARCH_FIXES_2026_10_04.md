# Human Guidance search and navigation fixes — 2026-10-04

## Scope

This change implements the agreed search-navigation behavior for the shared
Question Library and Public Blueprint Course search. The separate Blueprint
metadata and sort contract is implemented by the Blueprint search lane.

## Implemented behavior

- `RecordList` opens list links in a new tab with `rel="noopener"`. Question
  search rows therefore keep the original search page, filters, and results in
  its tab without saving a result snapshot.
- Question and Blueprint result links use canonical routes. The old return
  tokens, in-memory result copies, cursor replays, scroll restores, and
  detail-page return-token handling are removed.
- A completed Question Library search prompts before same-tab navigation leaves
  the page. An untouched search page leaves directly. Browser unload uses the
  native warning because a custom dialog cannot safely resume a reload, close,
  or address-bar navigation.
- Question Library and Blueprint Course search share the same result-display
  control with three local, non-persistent choices: Compact, List, and Visual
  boxes. Compact uses aligned dense rows for comparison; filters and sorting
  remain in the same search surface. The controls have names, pressed state,
  and brief tooltips. They do not create hover previews or store a preference.

## Validation

- `node --import tsx --test tests/test_library_classification_search.mjs` — 15 passed.
- `node --import tsx tests/test_record_list_reorder.mjs` — passed.
- `node --import tsx tests/playwright/record_list_contracts.mjs` — passed with
  the local browser harness. It includes the separate-tab and `noopener`
  contract for shared record links.
- `npx prettier --check` on changed TypeScript/JavaScript — passed.
- `git diff --check` — passed.
- The live Question Library browser scenario passes. It proves direct Ribbon
  navigation from an untouched search page, confirmation for a completed search,
  protected new-tab detail links that preserve the original query/results,
  Compact/List/Visual modes, and both Stay and Leave actions. The explicit receipt
  is `/private/tmp/hg_live_question_library_browser.log`.

Clearing the input after a search can return the default 50 rows; those rows are
still an existing result set and require confirmation before being discarded.
The direct-leave case uses an untouched page with no search or results.

## Security boundary

No search text, filters, results, cursor history, or display choice is written
to database or browser storage. Search input stays in the existing validated
query boundary; the navigation guard holds only a pending router event.
