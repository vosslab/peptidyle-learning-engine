import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeSysadminStudentDataAccess } from "../src/api/decoders/sysadmin_student_access.ts";

const valid = {
  courseInstanceId: "CI7K3M2QAZ",
  studentAccountId: "U0000035E",
  rosterId: "student-42",
  rosterName: "Riley Student",
  state: "activeStudent",
  audit: { eventId: "0d6c1b7f-59ae-4c54-8b1e-490f309d31c1", occurredAt: 1_798_609_260_000 },
};

test("Sysadmin Student-data decoder accepts only the scoped roster projection and audit receipt", () => {
  assert.deepEqual(decodeSysadminStudentDataAccess(valid), valid);
  assert.deepEqual(decodeSysadminStudentDataAccess({ ...valid, state: "removed" }), {
    ...valid,
    state: "removed",
  });
});

test("Sysadmin Student-data decoder rejects extra student fields", () => {
  assert.throws(
    () => decodeSysadminStudentDataAccess({ ...valid, email: "private@example.edu" }),
    DecodeError,
  );
});

test("Sysadmin Student-data decoder rejects an invalid state or missing audit event", () => {
  assert.throws(() => decodeSysadminStudentDataAccess({ ...valid, state: "revoked" }), DecodeError);
  assert.throws(
    () =>
      decodeSysadminStudentDataAccess({ ...valid, audit: { occurredAt: valid.audit.occurredAt } }),
    DecodeError,
  );
});
