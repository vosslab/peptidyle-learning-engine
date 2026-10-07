# M04 Published Question Summary Decoder

The shared Question metadata decoder now accepts a nullable `language`. A supplied nonempty
language remains unchanged. Invalid non-string values still fail decoding. Published Question
Title, Description, and citation validation remains strict; strict metadata decoding also continues
to reject unknown fields.

The focused Question Summary decoder test covers null language, preservation of a supplied
language, invalid language types, whitespace-only Title and Description rejection, unknown metadata
fields, and malformed empty citations.

Validation passed:

- `source ./source_me.sh && node --import tsx --test tests/test_question_summary_latest_revision_decoder.mjs` (4 tests)
- `npx prettier --write src/api/decoders/shared.ts tests/test_question_summary_latest_revision_decoder.mjs`
- `git diff --check -- src/api/decoders/shared.ts tests/test_question_summary_latest_revision_decoder.mjs`

The generated TypeScript `QuestionMetadata` declaration still types `language` as required text
until the coordinated contract generation step. This change does not run TypeScript checks against
that stale declaration. Fresh specification and quality reviews both flagged this expected
integration dependency; post-generation TypeScript checking and review remain pending. This report
covers only the shared summary decoder slice and does not claim M04 acceptance. No runtime,
database, browser, or full-stack gate was run.
