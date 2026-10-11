# M05 browser type fix

Date: 2026-10-06

## Changes

- Updated Pool summary metadata decoding to require and independently decode nullable Bloom Cognitive
  Process and Knowledge Dimension values.
- Kept ordinary Pool metadata decoding on the generated enum values without unchecked type casts.
- Updated the Pool detail decoder to require the server-projected `canEditMetadata` Boolean.
- Removed the retired correction request and Edit Number decoder, plus unused decoder imports.
- Typed Assessment sorting around a complete Bloom pair. Complete pairs sort in guide order; absent
  and incomplete pairs remain last in their prior relative order.
- Added only the new M05 Bloom fields to affected Pool and Question fixtures. Existing M06 Question
  Type and current metadata fields remain in place. The M09 member decoder remains tuple-only.
- No generated files were changed.

## Focused checks

- The focused Node command ran 59 tests: 56 passed and 3 failed in the concurrent M24 search
  handoff expectations. The M05 decoder, Pool metadata, and Assessment sort cases passed.
- `npx tsc --noEmit -p tsconfig.json` reports 27 diagnostics, all outside M05 ownership.
- `npx tsc --noEmit -p tsconfig.lint.json` reports those 27 plus 10 diagnostics in Playwright
  search fixtures and screenshot scenarios, all outside M05 ownership.
- Scoped Prettier and `git diff --check` pass.

The remaining diagnostics follow the current M24 search transition. Its generated `LibraryObject`
search contracts are not present in the 416-file generated tree, and existing browser sources and
fixtures still mix the prior `QuestionSearch` and membership contracts with the new search API.
Those sources and generated outputs are owned by the M24 follow-up.

### Remaining TypeScript diagnostics

The application project reports:

- `TS2307` for missing generated `LibraryObjectSearchPage` imports in
  `src/api/application_api.tsx:8`, `src/api/client.ts:9`, `src/api/decoders/question_library.ts:20`,
  and `src/api/http_client/response.ts:4`.
- `TS2307` for missing generated `LibraryObjectSearchRequest` imports in
  `src/api/application_api.tsx:9`, `src/api/client.ts:10`, `src/api/http_client/response.ts:5`,
  `src/api/library_classification_filter.ts:3`, `src/api/question_library_repository.ts:3`, and
  `src/api/question_search_query.ts:3`.
- `TS2307` for missing generated `LibraryObjectSearchSort` imports in
  `src/pages/library_bulk_actions.tsx:22`, `src/pages/library_page_model.ts:13`, and
  `src/pages/question_library_search_definition.ts:3`.
- `TS7006` for `facet` parameters in `src/api/question_library_repository.ts` at lines 157, 163,
  164, 169, 174, 179, 184, 189, 194, 199, and 204, and for `item` at line 256. These follow from
  the unresolved generated search types.
- `TS2353` for the retired `membership` property in `src/components/question_pool_create_model.ts:172`.
- `TS2322` for `string | null` assigned to `string` in
  `src/pages/question_library_search_definition.ts:80`.

The lint project adds:

- `TS2307` for `LibraryObjectSearchPage` in `tests/playwright/e2e/ui_backbone_parity.spec.ts:9`
  and `tests/support/ribbon_shell_harness.tsx:33`, plus `LibraryObjectSearchRequest` in
  `tests/support/ribbon_shell_harness.tsx:34`.
- `TS2724` for stale `decodeQuestionSearchPage` imports in
  `tests/playwright/screenshot_corpus/scenarios_instructor_picker_screens.ts:10`,
  `tests/playwright/screenshot_corpus/scenarios_instructor_shared_search.ts:3`, and
  `tests/playwright/screenshot_corpus/scenarios_student_types.ts:7`.
- `TS7006` for untyped `item` parameters in the picker and shared-search scenarios at line 44 and
  line 41 respectively, and for `item` and `summary` in `scenarios_student_types.ts:180` and `:182`.

The three remaining Node failures are in `tests/test_library_classification_search.mjs`:

- Line 224 expects `/api/questions/search`; current M24 code requests `/api/library-objects/search`.
- Line 425 expects the removed `noPool` handoff value; current output is `null`.
- Line 452 expects the removed `all` membership value; current output is `undefined`.

No M24 source or generated contract was changed. Database and full-stack behavior were not
exercised. Connected Pool API and live browser acceptance remain pending as recorded in the
[M05 browser report](QUESTION_SPEC_M05_BROWSER.md).
