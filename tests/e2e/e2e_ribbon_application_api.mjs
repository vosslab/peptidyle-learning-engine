// e2e_ribbon_application_api.mjs - browser-conditioned production API factory proof.

import assert from "node:assert/strict";

import {
  PRODUCT_ROLE_FIXTURES,
  createCountingApplicationApi,
} from "../support/ribbon_test_support.ts";
import { createHttpApiClient } from "../../src/api/http_client.ts";
import {
  assignmentAttemptRouteReference,
  courseInstanceRouteReference,
} from "../../src/navigation/public_route.ts";

// The router's browser entry only needs this history surface while query identities are created.
const history = {
  state: { _depth: 0 },
  length: 1,
  replaceState(state) {
    this.state = state;
  },
};
globalThis.window = { history };

// The router starts its maintenance interval while its browser entry is imported.
// Keep that fixture-owned timer from holding this assertion script open without
// changing how assertion failures or later promise rejections reach Node.
const nativeSetInterval = globalThis.setInterval;
globalThis.setInterval = function unrefRouterMaintenanceInterval(...arguments_) {
  const interval = nativeSetInterval(...arguments_);
  interval.unref?.();
  return interval;
};
let createApplicationApi;
try {
  ({ createApplicationApi } = await import("../../src/api/application_api.tsx"));
} finally {
  globalThis.setInterval = nativeSetInterval;
}
const harness = createCountingApplicationApi(
  createApplicationApi,
  PRODUCT_ROLE_FIXTURES.instructor,
);
const session = await harness.applicationApi.client.getSession();

assert.equal(session.account.productRole, "instructor");
assert.equal(harness.countRequests("/api/auth/session"), 1);

const identity = {
  courseOne: "00000000-0000-4000-8000-000000000011",
  courseTwo: "00000000-0000-4000-8000-000000000012",
  assignment: "00000000-0000-4000-8000-000000000013",
  student: "00000000-0000-4000-8000-000000000014",
  attempt: "00000000-0000-4000-8000-000000000015",
};
const resolvingClient = createHttpApiClient({
  fetch(input) {
    const pathname = typeof input === "string" ? input : new URL(input.url).pathname;
    const responseByPath = {
      "/api/navigation/C-1": { kind: "course", courseId: identity.courseOne },
      "/api/navigation/C-2": { kind: "course", courseId: identity.courseTwo },
      "/api/navigation/R-1": {
        kind: "assignmentAttempt",
        courseId: identity.courseOne,
        assignmentId: identity.assignment,
        studentRecordId: identity.student,
        assignmentAttemptId: identity.attempt,
      },
      "/api/navigation/C-9": {
        kind: "assignment",
        courseId: identity.courseOne,
        assignmentId: identity.assignment,
      },
      "/api/navigation/R-9": {
        kind: "assignment",
        courseId: identity.courseOne,
        assignmentId: identity.assignment,
      },
    };
    const payload = responseByPath[pathname];
    return Promise.resolve(
      payload === undefined
        ? new Response("not found", { status: 404, statusText: "Not Found" })
        : new Response(JSON.stringify(payload), {
            headers: { "content-type": "application/json" },
          }),
    );
  },
});
const resolutionApi = createApplicationApi(resolvingClient);
const courseOne = courseInstanceRouteReference("C-1");
const courseTwo = courseInstanceRouteReference("C-2");
const attemptOne = assignmentAttemptRouteReference("R-1");
const attemptTwo = assignmentAttemptRouteReference("R-2");

assert.equal(
  resolutionApi.queries.resolveCourse.keyFor(courseOne),
  resolutionApi.queries.resolveCourse.keyFor(courseOne),
);
assert.notEqual(
  resolutionApi.queries.resolveCourse.keyFor(courseOne),
  resolutionApi.queries.resolveCourse.keyFor(courseTwo),
);
assert.notEqual(
  resolutionApi.queries.resolveCourse.keyFor(courseOne),
  resolutionApi.queries.resolveAssignmentAttempt.keyFor(attemptOne),
);
assert.equal(
  resolutionApi.queries.resolveAssignmentAttempt.keyFor(attemptOne),
  resolutionApi.queries.resolveAssignmentAttempt.keyFor(attemptOne),
);
assert.notEqual(
  resolutionApi.queries.resolveAssignmentAttempt.keyFor(attemptOne),
  resolutionApi.queries.resolveAssignmentAttempt.keyFor(attemptTwo),
);
assert.match(resolutionApi.queries.resolveCourse.keyFor(courseOne), /C-1/u);
assert.match(resolutionApi.queries.resolveAssignmentAttempt.keyFor(attemptOne), /R-1/u);

assert.deepEqual(await resolutionApi.queries.resolveCourse(courseOne), {
  courseId: identity.courseOne,
});
assert.deepEqual(await resolutionApi.queries.resolveAssignmentAttempt(attemptOne), {
  courseId: identity.courseOne,
  assignmentId: identity.assignment,
  assignmentAttemptId: identity.attempt,
});
await resolutionApi.queries.resolveCourse(courseOne);
assert.deepEqual(await resolutionApi.queries.resolveCourse(courseTwo), {
  courseId: identity.courseTwo,
});
await assert.rejects(resolutionApi.queries.resolveCourse("C-01"), {
  message: "Course reference is invalid",
});
await assert.rejects(resolutionApi.queries.resolveCourse(courseInstanceRouteReference("C-9")), {
  message: "Course Instance reference resolved to another resource",
});
await assert.rejects(resolutionApi.queries.resolveAssignmentAttempt("R-01"), {
  message: "Assignment Attempt reference is invalid",
});
await assert.rejects(
  resolutionApi.queries.resolveAssignmentAttempt(assignmentAttemptRouteReference("R-9")),
  { message: "Assignment Attempt reference resolved to another resource" },
);
