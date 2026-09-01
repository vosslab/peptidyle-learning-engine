// WP-INST-T2 strict same-origin browser transport behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";

const course = "0198e000-0000-7000-8000-000000000001";

function jsonResponse(value, status = 200, headers = {}) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json; charset=utf-8",
      ...headers,
    },
  });
}

function createTeachingFetch(handler) {
  return async (input, init) => {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    return await handler(request);
  };
}

test("retention uses generated responses and keeps non-successes coarse", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      if (new URL(request.url).pathname.endsWith("/retention")) {
        assert.equal(request.method, "GET");
        return jsonResponse({ state: "active", assignmentContent: "retain", revision: "4" }, 200, {
          etag: '"4"',
        });
      }
      assert.equal(request.headers.get("if-match"), '"4"');
      return jsonResponse({ private: "not exposed" }, 412);
    }),
  });

  const current = await client.getCourseRetention(course);
  assert.equal(current.revision, "4");
  await assert.rejects(
    client.archiveCourseRetention(course, { assignmentContent: "retain" }, "4"),
    (error) => error instanceof ApiRequestError && error.status === 412,
  );
});

test("safe picker reads percent-encode pagination and decode only bounded projections", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      const url = new URL(request.url);
      assert.equal(request.method, "GET");
      assert.equal(request.headers.get("content-type"), null);
      if (url.pathname.endsWith("/instructor-course-invitation-targets")) {
        assert.equal(url.search, "?query=Dr.+A%26B&after=cursor%2Fone&size=2");
        return jsonResponse({
          targets: [
            {
              account: { reference: "U-1", display: "Dr. Ada" },
            },
          ],
          nextCursor: "2",
        });
      }
      assert.equal(url.pathname, `/api/courses/${course}/student-targets`);
      assert.equal(url.search, "?after=2%2Fnext&size=1");
      return jsonResponse({
        students: [{ reference: "M-1", display: "Student One", role: "student", status: "active" }],
        nextCursor: null,
      });
    }),
  });

  const targets = await client.searchInstructorCourseInvitationTargets(
    course,
    "Dr. A&B",
    "cursor/one",
    2,
  );
  assert.equal(targets.targets[0]?.account.reference, "U-1");
  const students = await client.listCourseStudentTargets(course, "2/next", 1);
  assert.equal(students.students[0]?.reference, "M-1");
});

test("safe picker rejects a response that omits no-store", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(
      async () =>
        new Response(JSON.stringify({ students: [], nextCursor: null }), {
          headers: { "content-type": "application/json" },
        }),
    ),
  });

  await assert.rejects(client.listCourseStudentTargets(course), ApiProtocolError);
});
