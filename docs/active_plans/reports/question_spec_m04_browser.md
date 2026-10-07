# M04 browser authoring

## Scope completed

Native Question source now contains display and grading content only. Its closed codec rejects
Title, Description, Tags, License, Citation, and Language at the top level. Record metadata has a
separate default model and editor state, and an explicit save action uses the existing Draft
metadata endpoint and the same Edit Number as source saves. Empty Title and Description remain
valid Draft values; blank Language becomes `null`.

Draft creation sends one `{ metadata, source }` JSON envelope. Metadata GET/PUT carries the six
record fields together with General Feedback, Hint, and Worked Solution. Each save sends the saved
values for the other field group, so editing metadata does not overwrite support text and editing
support text does not overwrite unsaved record metadata. Native source saves contain no metadata.
The preview and publication review get Title and public metadata from the record state.

The backend-general-feedback-only Draft page now reads and preserves record metadata on the shared
endpoint. Authoring Playwright journeys were updated to save metadata separately before source.
Draft source autosave and incomplete-source support remain deferred to M15.

## Evidence

Passed:

- `source ./source_me.sh && node --import tsx --test tests/test_ple_question_json_authoring.mjs tests/test_ple_question_json_authoring_types.mjs` - 36 tests passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json`.
- Scoped Prettier check for the changed source, page, and focused-test files.
- Scoped `git diff --check`.
- Static search found no Question-record metadata fields in `question_json_source.ts` or
  `question_json_codec.ts`.

Pending: run the updated production-stack authoring journey and screenshot scenarios after the
coordinated M04 server/database integration. No database, API runtime, or production-browser
acceptance is claimed here.
