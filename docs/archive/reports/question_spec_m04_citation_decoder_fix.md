# M04 Draft citation decoder alignment

The Draft metadata GET decoder now delegates non-null Question Citation validation to the shared
`decodeQuestionCitation` decoder. This keeps the browser contract aligned with canonical nullable
members, rejects a citation whose members are both null or empty, and preserves valid URL-only and
text-only citations. A null citation remains valid. Draft Title and Description behavior and the
metadata save payload and Edit Number protocol are unchanged.

Focused tests exercise GET responses with null, URL-only, text-only, and invalid empty citations.
The existing save test continues to assert the metadata payload and conditional Edit Number.

Validation passed:

- `source ./source_me.sh && node --import tsx --test tests/test_ple_question_json_authoring.mjs tests/test_ple_question_json_authoring_types.mjs` (37 tests)
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json`
- Scoped Prettier check for the changed TypeScript and test files.
- Scoped `git diff --check`.

This report covers the focused Draft browser-client decoder and mock-response tests. It does not
claim a running-server browser journey, full M04 acceptance, or broader repository checks.
