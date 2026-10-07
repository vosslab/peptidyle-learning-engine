import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";

const COURSE_ID = "CI7K3M2QAZ";
const STUDENT_ACCOUNT_ID = "U0000035E";
const EVENT_ID = "0d6c1b7f-59ae-4c54-8b1e-490f309d31c1";

const access = {
  courseInstanceId: COURSE_ID,
  studentAccountId: STUDENT_ACCOUNT_ID,
  rosterId: "student-42",
  rosterName: "Riley Student",
  state: "activeStudent",
  audit: { eventId: EVENT_ID, occurredAt: 1_798_609_260_000 },
};

function noStoreJson(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

function recordedFetch(handler) {
  const requests = [];
  const fetchImplementation = async (input, init) => {
    const request = new Request(new URL(String(input), "https://ple.example"), init);
    requests.push(request.clone());
    return handler(request);
  };
  return { requests, fetchImplementation };
}

test("Sysadmin Student-data access sends one explicit, scoped confirmation", async () => {
  const { requests, fetchImplementation } = recordedFetch(async (request) => {
    assert.equal(request.method, "POST");
    assert.equal(
      new URL(request.url).pathname,
      `/api/sysadmin/course-instances/${COURSE_ID}/roster/student-42/student-data`,
    );
    assert.equal(new URL(request.url).search, "");
    assert.equal(request.headers.get("cache-control"), null);
    return noStoreJson(access);
  });

  const result = await createHttpApiClient({
    fetch: fetchImplementation,
  }).accessSysadminStudentData(COURSE_ID, "student-42");
  assert.deepEqual(result, access);
  assert.deepEqual(await requests[0].clone().json(), { administrativeAccessConfirmed: true });
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].credentials, "same-origin");
  assert.equal(requests.length, 1);
});

test("invalid Sysadmin Student-data target is rejected before a request", async () => {
  const { requests, fetchImplementation } = recordedFetch(async () => noStoreJson(access));
  const client = createHttpApiClient({ fetch: fetchImplementation });
  await assert.rejects(client.accessSysadminStudentData(COURSE_ID, "bad/roster"), ApiProtocolError);
  await assert.rejects(
    client.accessSysadminStudentData("bad-course", "student-42"),
    ApiProtocolError,
  );
  assert.equal(requests.length, 0);
});

test("Sysadmin Student-data client rejects a response for another roster target", async () => {
  const { requests, fetchImplementation } = recordedFetch(async () =>
    noStoreJson({ ...access, rosterId: "other-student" }),
  );
  await assert.rejects(
    createHttpApiClient({ fetch: fetchImplementation }).accessSysadminStudentData(
      COURSE_ID,
      "student-42",
    ),
    ApiProtocolError,
  );
  assert.equal(requests.length, 1);
});

test("Sysadmin Student-data client preserves the retained removed roster state", async () => {
  const removedAccess = { ...access, state: "removed" };
  const { fetchImplementation } = recordedFetch(async () => noStoreJson(removedAccess));
  const result = await createHttpApiClient({
    fetch: fetchImplementation,
  }).accessSysadminStudentData(COURSE_ID, "student-42");
  assert.deepEqual(result, removedAccess);
});
