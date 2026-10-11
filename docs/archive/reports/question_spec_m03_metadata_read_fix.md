# M03 current metadata read correction

Status: the Rust and browser read contract, editor initialization, and canonical SQL reader
functions are corrected. Connected PostgreSQL execution remains pending with the exclusive
`fresh_database_gate` owner.

## Finding

The Sysadmin detail page offered ordinary metadata editing, but opening it called the bounded
current shared-metadata reader, which required an Instructor both in the server and in SQL. The
reader also returned the metadata Edit Number, Tags, and classification while the editor copied
Title and Description from the older page-detail response. If another editor changed the Question
before this editor opened, the new Edit Number could authorize saving those old Title and
Description values over the newer metadata.

## Correction

The single current shared-metadata read now supplies Title, Description, Tags, classification, and
the metadata Edit Number as one snapshot. The detail editor initializes every editable field from
that response. The server read uses the existing Question Library reader-session policy, which
allows Instructors and Sysadmins and conceals the route from other roles. The ordinary metadata
save and bulk update role checks are unchanged.

The generated `PublishedQuestionSharedMetadata` contract and strict browser decoder include the two
text fields. The decoder is named `decodeCurrentQuestionSharedMetadata`, and the browser method is
named `getCurrentQuestionSharedMetadata` at its interface, implementation, and callers. The bounded
transport route and bulk update method remain the existing API.

## Focused evidence

- `source ./source_me.sh && cargo tsgen` completed and regenerated the DTO.
- `source ./source_me.sh && node --import tsx --test tests/test_library_classification_search.mjs`:
  16 passed, including a current-snapshot client test.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` passed.
- `source ./source_me.sh && python3 schema_style/check_schema_style.py --source-dir schemas/base_schema`
  passed with `clean`.
- Scoped Rust formatting, Prettier for touched browser files, and `git diff --check` passed.
- The connected `question_revision_metadata` case asserts Sysadmin current read values and counter,
  a different Instructor's Library read access while its metadata write remains Forbidden, and
  Student denial. It also checks the ordinary Sysadmin metadata save.

## Runtime status

The exclusive gate captured a fresh v2 schema install, then failed an earlier M26 confirmed-read
selector and cleaned up before reaching M03. Therefore the new SQL and connected M03 role assertions
have not run against PostgreSQL. The source change is ready for the next coherent gate. Because the
return table changed, its one-time live overlay must drop the API wrapper before the private
function, recreate the canonical definitions, and restore the existing grants. The function
identity, security-definer boundary, and grants themselves remain unchanged.
