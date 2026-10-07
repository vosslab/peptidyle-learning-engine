# M26 Sysadmin server implementation

## Implemented contract

The server exposes one protected Student-data operation:

```text
POST /api/sysadmin/course-instances/{courseInstanceId}/roster/{rosterId}/student-data
Content-Type: application/json

{"administrativeAccessConfirmed": true}
```

An authenticated Sysadmin session is required. The acting Account is resolved from the session and
never accepted from the request body. A missing or false confirmation fails before the Store call.
The database function repeats the confirmation and active Sysadmin checks, resolves the exact
Course roster profile to its Student Account, and returns the named roster projection only after
writing `sysadmin_student_data_accessed` to the immutable `ple_audit.course_roster_event` table in
the same transaction.

Success is `200` with `courseInstanceId`, `studentAccountId`, `rosterId`, `rosterName`, `state`, and
an `audit` receipt containing the immutable event UUID and transaction time in Unix milliseconds.
Roster state is `activeStudent`, `invitationPending`, or `removed`. Responses are no-store.
Anonymous and non-Sysadmin requests and missing Course/roster targets are concealed as `404`;
missing/false confirmation returns `422`; direct `GET` is refused with `405`.

The projection contains no Student Work, email address, assessment content, or answer data. Normal
Course Instructor authorization remains based on actual Course membership; platform administration
is checked only in the Sysadmin Student-data operation.

## Schema change

The Instructor-issued support capability registry, use receipts, resource/result enums, triggers,
policies, grants, indexes, API operations, and read routes are retired. The roster audit writer
accepts the new event kind and returns the written event UUID. The registry schema-layer include
files remain as empty comments so existing layer assembly stays stable.

## Evidence

- `cargo test -p learning-data-access --features postgres support_capability --lib`: passed, 1 test.
- `cargo check -p server_core`: passed.
- `source ./source_me.sh && python3 -m pytest tests/test_schema_table_shape.py -q`: passed, 1 test.
- `schema_style/check_schema_style.py`: clean.
- `bash -n tests/e2e/e2e_live_demo_support_capability.sh`: passed.
- Scoped `git diff --check`: passed.

The disposable PostgreSQL catalog proof and Live Demo route proof are pending the coordinated fresh
database run. The Live Demo script now checks confirmation, role denial, exact target resolution,
event actor/target, direct-GET rejection, retired grant endpoint, and registry absence. No runtime
service operation was performed in this implementation lane.

## Security guidance

The server and database enforce input validation and current role authorization at trusted layers
(ASVS 2.2.2, 8.3.1). Reading the protected projection and recording its event occur in one database
transaction (ASVS 2.3.3). The event records the actor, target, Course, event kind, and time without
copying the Student roster name or record payload into the audit table (ASVS 16.2.1, 16.2.5).
