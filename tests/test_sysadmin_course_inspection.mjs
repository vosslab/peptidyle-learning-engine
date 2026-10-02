import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";

const COURSE_ID = "CI7K3M2QAZ";

const inspection = {
  id: COURSE_ID,
  shortName: "Cell",
  longName: "Cell Biology",
  term: { startDate: "2026-08-15", endDate: "2026-12-15" },
  lifecycleState: "inactive",
  retentionLifecycleState: "archived",
  instructorDisplayNames: ["Ada Lovelace"],
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

test("sysadmin installation course inspection shows instructor and status", async () => {
  const { requests, fetchImplementation } = recordedFetch(async (request) => {
    const url = new URL(request.url);
    if (url.pathname === "/api/sysadmin/courses" && request.method === "GET") {
      assert.equal(url.searchParams.get("pageSize"), "50");
      return noStoreJson({ courses: [inspection], nextCursor: '{"course":"next"}' });
    }
    if (url.pathname === "/api/sysadmin/courses/search" && request.method === "POST") {
      assert.equal(url.search, "");
      const body = await request.json();
      assert.equal(body.query, "Ada Lovelace");
      assert.equal(body.pageSize, 100);
      assert.equal(typeof body.cursor, "string");
      return noStoreJson({ courses: [inspection], nextCursor: null });
    }
    if (url.pathname === `/api/sysadmin/courses/${COURSE_ID}` && request.method === "GET") {
      return noStoreJson(inspection);
    }
    return noStoreJson({ message: "missing" }, 404);
  });
  const client = createHttpApiClient({ fetch: fetchImplementation });
  const page = await client.listInstallationCourses("   ", null, 50);
  assert.equal(requests[0].method, "GET");
  assert.equal(new URL(requests[0].url).pathname, "/api/sysadmin/courses");
  assert.deepEqual(page.courses[0].instructorDisplayNames, ["Ada Lovelace"]);
  assert.equal(page.courses[0].lifecycleState, "inactive");
  assert.equal(page.courses[0].retentionLifecycleState, "archived");
  assert.equal(page.nextCursor, '{"course":"next"}');

  const searchCursor = '{"query":"Ada Lovelace","pageSize":100}';
  await client.listInstallationCourses("Ada Lovelace", searchCursor, 100);
  assert.equal(requests[1].method, "POST");
  assert.equal(new URL(requests[1].url).pathname, "/api/sysadmin/courses/search");
  assert.equal(new URL(requests[1].url).search, "");
  assert.deepEqual(await requests[1].clone().json(), {
    query: "Ada Lovelace",
    cursor: searchCursor,
    pageSize: 100,
  });

  const loaded = await client.loadInstallationCourse(COURSE_ID);
  assert.equal(loaded.longName, "Cell Biology");
  assert.deepEqual(loaded.instructorDisplayNames, ["Ada Lovelace"]);
  assert.equal(requests.length, 3);

  const revealing = recordedFetch(async () =>
    noStoreJson({
      courses: [{ ...inspection, studentEmail: "ada@university.edu" }],
      nextCursor: null,
    }),
  );
  await assert.rejects(
    createHttpApiClient({ fetch: revealing.fetchImplementation }).listInstallationCourses(
      "",
      null,
      50,
    ),
    DecodeError,
  );
  await assert.rejects(
    async () => client.listInstallationCourses(`${"A".repeat(201)}`, null, 50),
    ApiProtocolError,
  );
  assert.equal(requests.length, 3);
});
