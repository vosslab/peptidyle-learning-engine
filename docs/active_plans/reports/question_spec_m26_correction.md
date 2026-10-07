# M26 roster response and contract correction

## Correction

The handwritten Sysadmin Student-data response type and strict decoder now accept all three
backend roster states: `invitationPending`, `activeStudent`, and `removed`. A removed roster entry
is a retained record that remains valid for this audited administrative read. Focused tests cover
decoder acceptance and client preservation of `removed` while keeping the response field set
closed.

`API_CONTRACTS.md` now documents the confirmed, session-authorized single-record route and its
same-transaction audit receipt. `CONTRACTS.md` states that the operation leaves Course membership
unchanged and does not use Instructor grants. Both documents keep coordinated runtime acceptance
pending.

## Rechecked M26 contract

Human Guidance requires Sysadmin authority over Course and Student records, confirmation before
FERPA-sensitive Student-data access, and audit recording. M26 specifies confirmation before
protected retrieval and replaces Instructor-grant requirements. The server implementation report
defines the request as `{"administrativeAccessConfirmed":true}`, derives the acting Account from an
authenticated Sysadmin session, fails missing or false confirmation before storage, and commits
the exact roster projection and audit event in one transaction. It lists all three retained roster
states and says this does not change Course membership or Instructor authorization. The browser
type, decoder, API contract, and module contract now match those facts.

The coordinated fresh-database/catalog and Live Demo checks remain pending as recorded by the
server implementation owner. This correction does not claim runtime acceptance.

## Verification

- `source ./source_me.sh && node --import tsx --test tests/test_sysadmin_student_access_decoder.mjs tests/test_sysadmin_student_access_client.mjs`: 8 passed, 0 failed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json`: passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.lint.json`: passed.
- `source ./source_me.sh && python3 -m pytest tests/test_markdown_links.py -q`: 459 passed.
- Scoped `git diff --check`: passed.

No server, schema, or runtime service operation was performed in this correction lane.
