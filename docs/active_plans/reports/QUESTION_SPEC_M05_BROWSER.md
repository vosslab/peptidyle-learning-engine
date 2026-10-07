# M05 browser metadata and Assessment ordering

Status: The browser and Node slice is implemented. Generated-contract TypeScript validation and
connected Pool API behavior remain pending the coordinated M05 backend update.

## Behavior

The Question detail editor reads both Bloom dimensions with its ordinary current metadata
snapshot and saves them with the exact `PublishedQuestionRevisionTuple` and existing
`metadataEditNumber`. Each dimension has its own optional selector. Changing one leaves the other
at its loaded value, including `NULL`.

Pool detail uses its ordinary current metadata client and editor. The replacement includes Title,
Description, Topic, Subtopic, Tags, and both independently nullable Bloom dimensions. Its
`QuestionPoolMetadataEditNumber` remains distinct from the tuple-set Edit Number. The page shows the
editor only when the server-projected `canEditMetadata` affordance is true.

Bloom display names both dimensions and shows `Not assigned` independently. Library filters already
support each dimension separately. Assessment sorting orders complete pairs in guide order, then
keeps absent or incomplete classifications last in their previous relative order.

The dedicated Bloom correction browser client, receipts, decoder, 412 error, and detail editor were
removed. Bloom edits now use ordinary metadata replacement and concurrency.

## Evidence

Passed:

- Focused Node coverage: 59/59 passed across Bloom decoding, ordinary Question metadata save/read,
  Pool metadata read/write, Library search, Assessment sorting, and related Question/Pool decoders.
- The scoped current-metadata Playwright harness passed for Owner and Sysadmin projections. It
  verifies a one-dimension Question change keeps the other `NULL` in the ordinary save request.
- Prettier completed for the changed TS/TSX/Node files.

Pending:

- Generated contracts for nullable Bloom view fields, Question metadata fields, Pool metadata
  requests/receipts, and Pool `canEditMetadata` are owned by the coordinated backend update. Run
  application and lint-scope TypeScript checks after that generation.
- The Pool client/editor has not yet run against the connected API or a real Pool detail response.
- The Playwright harness is a controlled component harness, not connected browser acceptance.
