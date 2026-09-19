// e2e_ribbon_application_api.mjs - browser-conditioned production API factory proof.

import assert from "node:assert/strict";

import {
  PRODUCT_ROLE_FIXTURES,
  createCountingApplicationApi,
} from "../support/ribbon_test_support.ts";
import { createHttpApiClient } from "../../src/api/http_client.ts";
import {
  assessmentAttemptRouteReference,
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
  assessment: "00000000-0000-4000-8000-000000000013",
  student: "00000000-0000-4000-8000-000000000014",
  attempt: "00000000-0000-4000-8000-000000000015",
};
const resolvingClient = createHttpApiClient({
  fetch(input) {
    const pathname = typeof input === "string" ? input : new URL(input.url).pathname;
    const responseByPath = {
      "/api/course-instances/CI7K3M2QAZ/summary": {
        reference: "CI7K3M2QAZ",
        shortName: "BIO 301",
        longName: "Molecular Biology",
        term: { startDate: "2026-01-12", endDate: "2026-05-08" },
        role: "instructor",
      },
      "/api/course-instances/CI7K3M2QAZ/appearance": { theme: "grass", banner: null },
      "/api/course-instances/CI4W8QF9AD/summary": {
        reference: "CI4W8QF9AD",
        shortName: "BIO 302",
        longName: "Genetics",
        term: { startDate: "2026-08-24", endDate: "2026-12-11" },
        role: "student",
      },
      "/api/course-instances/CI4W8QF9AD/appearance": { theme: "forest", banner: null },
      "/api/navigation/00000000-0000-0000-0000-000000000001": {
        kind: "assessmentAttempt",
        courseId: identity.courseOne,
        assessmentId: identity.assessment,
        studentRecordId: identity.student,
        assessmentAttemptId: identity.attempt,
      },
      "/api/navigation/00000000-0000-0000-0000-000000000009": {
        kind: "assessment",
        courseId: identity.courseOne,
        assessmentId: identity.assessment,
      },
    };
    const payload = responseByPath[pathname];
    return Promise.resolve(
      payload === undefined
        ? new Response("not found", { status: 404, statusText: "Not Found" })
        : new Response(JSON.stringify(payload), {
            headers: { "cache-control": "no-store", "content-type": "application/json" },
          }),
    );
  },
});
const resolutionApi = createApplicationApi(resolvingClient);
const courseOne = courseInstanceRouteReference("CI7K3M2QAZ");
const courseTwo = courseInstanceRouteReference("CI4W8QF9AD");
const attemptOne = assessmentAttemptRouteReference("00000000-0000-0000-0000-000000000001");
const attemptTwo = assessmentAttemptRouteReference("00000000-0000-0000-0000-000000000002");

assert.equal(
  resolutionApi.queries.courseScope.keyFor(courseOne),
  resolutionApi.queries.courseScope.keyFor(courseOne),
);
assert.notEqual(
  resolutionApi.queries.courseScope.keyFor(courseOne),
  resolutionApi.queries.courseScope.keyFor(courseTwo),
);
assert.notEqual(
  resolutionApi.queries.courseScope.keyFor(courseOne),
  resolutionApi.queries.resolveAssessmentAttempt.keyFor(attemptOne),
);
assert.equal(
  resolutionApi.queries.resolveAssessmentAttempt.keyFor(attemptOne),
  resolutionApi.queries.resolveAssessmentAttempt.keyFor(attemptOne),
);
assert.notEqual(
  resolutionApi.queries.resolveAssessmentAttempt.keyFor(attemptOne),
  resolutionApi.queries.resolveAssessmentAttempt.keyFor(attemptTwo),
);
assert.match(resolutionApi.queries.courseScope.keyFor(courseOne), /CI7K3M2QAZ/u);
assert.match(
  resolutionApi.queries.resolveAssessmentAttempt.keyFor(attemptOne),
  /00000000-0000-0000-0000-000000000001/u,
);

assert.deepEqual(await resolutionApi.queries.courseScope(courseOne), {
  summary: {
    reference: "CI7K3M2QAZ",
    shortName: "BIO 301",
    longName: "Molecular Biology",
    term: { startDate: "2026-01-12", endDate: "2026-05-08" },
    role: "instructor",
  },
  appearance: { theme: "grass", banner: null },
});
assert.deepEqual(await resolutionApi.queries.resolveAssessmentAttempt(attemptOne), {
  courseId: identity.courseOne,
  assessmentId: identity.assessment,
  assessmentAttemptId: identity.attempt,
});
assert.deepEqual(await resolutionApi.queries.courseScope(courseTwo), {
  summary: {
    reference: "CI4W8QF9AD",
    shortName: "BIO 302",
    longName: "Genetics",
    term: { startDate: "2026-08-24", endDate: "2026-12-11" },
    role: "student",
  },
  appearance: { theme: "forest", banner: null },
});
await assert.rejects(resolutionApi.queries.resolveAssessmentAttempt("R-01"), {
  message: "Assessment Attempt reference is invalid",
});
await assert.rejects(
  resolutionApi.queries.resolveAssessmentAttempt(
    assessmentAttemptRouteReference("00000000-0000-0000-0000-000000000009"),
  ),
  { message: "Assessment Attempt reference resolved to another resource" },
);
