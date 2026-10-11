# Fresh database correction

## Correction

The M26 catalog fixture constructed a Course roster profile as `ple_private_owner`. That table
forces row-level security, while its construction policy and `INSERT` grant are assigned to
`ple_api_owner`. The fixture now uses that authorized role for the synthetic profile insert. The
subsequent assertions still call the confirmed, session-authorized Sysadmin read, verify its exact
removed-roster projection, and check that one audit event was recorded.

The Blueprint parent-column comments now explain the existing paired-NULL constraint: NULL parent
fields identify a root Blueprint Course that was not forked; populated fields identify the exact
immediate source Blueprint Revision. No schema behavior changed in this correction.

## Evidence and status

- Policy and grant inspection confirms `ple_api_owner` is authorized to construct the fixture row;
  `ple_private_owner` is not the intended FORCE RLS construction role.
- The catalog fixture retains the authenticated Sysadmin, explicit confirmation, exact roster
  record, membership non-mutation, and same-transaction audit assertions.
- Source-only `schema_style/check_schema_style.py` passes with no findings. Its default run reads
  the stale checked-in catalog snapshot and reports one old missing-comment finding; rerunning with
  `--source-dir schemas/base_schema` from outside the repository loads the working SQL and passes.
- Scoped `git diff --check` passes.
- The exclusive `fresh_database_gate` must rerun the baseline-installed schema, replay, connected
  role checks, and M26 catalog fixture. This report does not claim runtime acceptance.
