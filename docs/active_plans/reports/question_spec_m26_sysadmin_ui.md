# M26 Sysadmin Student-data access UI

Status: Implemented; focused browser-client, decoder, and TypeScript checks passed. The
privileged Live Demo screenshot journey is registered and handed to the runtime owner.

The Sysadmin Course inspection page now accepts one Course-scoped roster ID. It shows the
administrative-work confirmation before enabling access, sends no request on Cancel, and displays
only the roster ID, Course access state, and audit receipt after a successful read. The request
uses the server-owned `administrativeAccessConfirmed: true` field and strict no-store transport.
The Sysadmin home no longer links to the scoped roster-support grant surface.

## Evidence

- `source ./source_me.sh && node --import tsx --test tests/test_sysadmin_course_inspection.mjs tests/test_sysadmin_student_access_client.mjs tests/test_sysadmin_student_access_decoder.mjs`:
  passed, 8 tests, 0 failures. Coverage includes the existing Course inspection privacy projection,
  explicit POST body and exact Course/roster route, pre-request target validation, strict response
  identity/field decoding, and audit receipt validation.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json`: passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.lint.json`: passed, including the
  screenshot-corpus scenario types.
- `source ./source_me.sh && npx prettier --check src/pages/sysadmin_course_inspection_page.tsx src/pages/role_home_pages.tsx src/api/client.ts src/api/http_client.ts src/api/sysadmin_student_access.ts src/api/http_client/sysadmin_student_access.ts src/api/decoders/sysadmin_student_access.ts tests/test_sysadmin_course_inspection.mjs tests/test_sysadmin_student_access_client.mjs tests/test_sysadmin_student_access_decoder.mjs tests/playwright/screenshot_corpus/scenarios_sysadmin.ts`:
  passed.
- `git diff --check`: passed.
- `tests/playwright/screenshot_corpus/scenarios_sysadmin.ts` now declares
  `sysadmin_student_data_confirmation`. It captures the checked confirmation using the seeded
  Course and then asserts Cancel clears the lookup without sending `/student-data`. The scenario
  was handed to `m01_runtime_luna` for execution in the exclusive privileged browser lane; runtime
  evidence is pending.

No Student-data request was made during these checks. The server route remains the authorization
and audit boundary; runtime success-path acceptance belongs to the M26 server and privileged
browser checks.
