// WP-INST-T2 strict same-origin browser transport behavior tests.

import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";

const course = "0198e000-0000-7000-8000-000000000001";
const assignment = "0198e000-0000-7000-8000-000000000002";

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

test("course-group creation requires 201, matching strong ETag, and safe Location", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      assert.equal(request.method, "POST");
      assert.equal(
        await request.text(),
        '{"title":"Section A","purpose":"section","members":["M-1"]}',
      );
      return jsonResponse(
        { reference: "G-1", title: "Section A", purpose: "section", revision: "2", memberCount: 1 },
        201,
        {
          etag: '"2"',
          location: `/api/courses/${course}/groups/G-1`,
        },
      );
    }),
  });

  const created = await client.createCourseGroup(course, {
    title: "Section A",
    purpose: "section",
    members: ["M-1"],
  });
  assert.equal(created.reference, "G-1");
  assert.equal(created.revision, "2");
});

test("M2 mutation sends canonical If-Match and rejects an ETag mismatch", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      assert.equal(request.method, "PUT");
      assert.equal(request.headers.get("if-match"), '"7"');
      assert.equal(await request.text(), '{"offsetSeconds":3600}');
      return jsonResponse({ revision: "8" }, 200, { etag: '"9"' });
    }),
  });

  await assert.rejects(
    client.putGroupScheduleOffset(course, assignment, "G-1", { offsetSeconds: 3600 }, "7"),
    ApiProtocolError,
  );
});

test("retention uses generated responses and keeps non-successes coarse", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      if (new URL(request.url).pathname.endsWith("/retention")) {
        assert.equal(request.method, "GET");
        return jsonResponse(
          { state: "active", assignmentDefinitions: "retain", revision: "4" },
          200,
          { etag: '"4"' },
        );
      }
      assert.equal(request.headers.get("if-match"), '"4"');
      return jsonResponse({ private: "not exposed" }, 412);
    }),
  });

  const current = await client.getCourseRetention(course);
  assert.equal(current.revision, "4");
  await assert.rejects(
    client.archiveCourseRetention(course, { assignmentDefinitions: "retain" }, "4"),
    (error) => error instanceof ApiRequestError && error.status === 412,
  );
});

test("safe picker reads percent-encode pagination and decode only bounded projections", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      const url = new URL(request.url);
      assert.equal(request.method, "GET");
      assert.equal(request.headers.get("content-type"), null);
      if (url.pathname.endsWith("/co-instructor-targets")) {
        assert.equal(url.search, "?query=Dr.+A%26B&after=cursor%2Fone&size=2");
        return jsonResponse({
          targets: [
            {
              account: { reference: "U-1", display: "Dr. Ada" },
              approval: { state: "approved", revision: "3" },
            },
          ],
          nextCursor: "2",
        });
      }
      assert.equal(url.pathname, `/api/courses/${course}/student-targets`);
      assert.equal(url.search, "?after=2%2Fnext&size=1");
      return jsonResponse({
        students: [{ reference: "M-1", display: "Learner One", role: "student", status: "active" }],
        nextCursor: null,
      });
    }),
  });

  const targets = await client.searchCourseCoInstructorTargets(course, "Dr. A&B", "cursor/one", 2);
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

test("Sysadmin candidate search serializes one bounded display-label query", async () => {
  const client = createHttpApiClient({
    fetch: createTeachingFetch(async (request) => {
      const url = new URL(request.url);
      assert.equal(url.pathname, "/api/teaching/instructor-approval-candidates");
      assert.equal(url.search, "?query=Avery+%26+Co&after=next%2Fpage&size=20");
      assert.equal(request.method, "GET");
      assert.equal(request.headers.get("content-type"), null);
      return jsonResponse({
        candidates: [
          {
            account: { reference: "U-8", display: "Avery Student" },
            approval: { state: "unapproved", revision: null },
          },
        ],
        nextCursor: null,
      });
    }),
  });

  const page = await client.searchSysadminInstructorCandidates({
    query: "Avery & Co",
    after: "next/page",
    size: 20,
  });
  assert.equal(page.candidates[0]?.account.reference, "U-8");
});
