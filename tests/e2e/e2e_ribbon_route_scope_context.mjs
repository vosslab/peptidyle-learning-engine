// e2e_ribbon_route_scope_context.mjs - browser-condition integration for the Ribbon scope owner.

import assert from "node:assert/strict";
import test from "node:test";

import { createRoot, createSignal } from "solid-js";

import {
  createDeferredResolution,
  walkPathnamesThroughMountedApp,
} from "../support/ribbon_test_support.ts";
import {
  assignmentAttemptContext,
  assignmentAttemptHistoryData,
  courseRouteData,
} from "../support/route_scope_provider_fixtures.ts";
import { loadRouteScopeProviderHarness } from "../support/route_scope_provider_bundle.ts";
import { createRouteScopeController } from "../../src/ribbon/route_scope_controller.ts";

const nextTurn = () => new Promise((resolve) => setImmediate(resolve));
let createApplicationApi;
let createHttpApiClient;
let useRouteScopeData;
let useRouteScopeIdentity;
const history = {
  state: { _depth: 0 },
  length: 1,
  replaceState(state) {
    this.state = state;
  },
};
globalThis.window = { history };
const nativeSetInterval = globalThis.setInterval;
globalThis.setInterval = function unrefRouterMaintenanceInterval(...arguments_) {
  const interval = nativeSetInterval(...arguments_);
  interval.unref?.();
  return interval;
};
try {
  ({ createApplicationApi } = await import("../../src/api/application_api.tsx"));
  ({ createHttpApiClient } = await import("../../src/api/http_client.ts"));
  ({ useRouteScopeData, useRouteScopeIdentity } =
    await import("../../src/ribbon/route_scope_context.tsx"));
} finally {
  globalThis.setInterval = nativeSetInterval;
}

function createDeferredQueries() {
  const attemptResolvers = new Map();
  const courseViews = new Map();
  const attemptContexts = new Map();
  const histories = new Map();
  const deferred = (map, key, label) => {
    if (map.has(key)) throw new Error(`redundant ${label} query for ${key}`);
    const value = createDeferredResolution();
    map.set(key, value);
    return value;
  };
  return {
    attemptResolvers,
    courseViews,
    attemptContexts,
    histories,
    queries: {
      resolveAssignmentAttempt(assessmentAttemptId) {
        return deferred(attemptResolvers, assessmentAttemptId, "Attempt resolution").promise;
      },
      courseScope(courseInstanceId) {
        return deferred(courseViews, courseInstanceId, "Course scope").promise;
      },
      assessmentAttemptScope(assessmentAttemptId) {
        return deferred(attemptContexts, assessmentAttemptId, "Attempt context").promise;
      },
      assessmentAttemptHistory(assessmentAttemptId) {
        return deferred(histories, assessmentAttemptId, "Attempt history").promise;
      },
    },
  };
}

function mountedController(queries, initialPathname) {
  let dispose;
  let setPathname;
  let controller;
  let shellMounts = 0;
  createRoot((disposeRoot) => {
    dispose = disposeRoot;
    const [pathname, setPathnameSignal] = createSignal(initialPathname);
    setPathname = setPathnameSignal;
    controller = createRouteScopeController(pathname, queries);
    shellMounts += 1;
  });
  return {
    controller,
    navigate: (pathname) => setPathname(pathname),
    shellMounts: () => shellMounts,
    dispose,
  };
}

function queryFunction(handler, key) {
  return Object.assign(handler, {
    key,
    keyFor: (...arguments_) => `${key}:${arguments_.join(":")}`,
  });
}

test("scope hooks fail explicitly outside their provider", () => {
  createRoot(() => {
    assert.throws(
      () => useRouteScopeIdentity(),
      /RouteScopeProvider is missing from the application shell/u,
    );
    assert.throws(
      () => useRouteScopeData(),
      /RouteScopeProvider is missing from the application shell/u,
    );
  });
});

test("controller begins the direct Course Instance scope read before any consumer reads its data", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/courses/CI7K3M2QAZ");
  assert.deepEqual(app.controller.identity(), {
    kind: "courseInstance",
    courseInstanceId: "CI7K3M2QAZ",
  });
  assert.ok(fixture.courseViews.has("CI7K3M2QAZ"));
  const course = courseRouteData("CI7K3M2QAZ");
  fixture.courseViews.get("CI7K3M2QAZ").resolve(course);
  await nextTurn();
  assert.deepEqual(app.controller.data(), { kind: "course", course });
  app.dispose();
});

test(
  [
    "actual provider composition keeps immediate identity",
    "and direct union data through route changes",
  ].join(" "),
  async () => {
    const fixture = createDeferredQueries();
    const base = createApplicationApi(
      createHttpApiClient({ fetch: () => Promise.reject(new Error("unused")) }),
    );
    const applicationApi = {
      ...base,
      queries: {
        ...base.queries,
        resolveAssignmentAttempt: queryFunction(
          fixture.queries.resolveAssignmentAttempt,
          "test-resolve-attempt",
        ),
        courseScope: queryFunction(fixture.queries.courseScope, "test-course-scope"),
        assessmentAttemptScope: queryFunction(
          fixture.queries.assessmentAttemptScope,
          "test-attempt-context",
        ),
        assessmentAttemptHistory: queryFunction(
          fixture.queries.assessmentAttemptHistory,
          "test-attempt-history",
        ),
      },
    };
    const { mountRouteScopeProviderHarness } = await loadRouteScopeProviderHarness();
    const app = mountRouteScopeProviderHarness(applicationApi, "/courses/CI7K3M2QAZ");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseInstanceId: "CI7K3M2QAZ" },
      data: undefined,
    });
    const courseOne = courseRouteData("CI7K3M2QAZ");
    fixture.courseViews.get("CI7K3M2QAZ").resolve(courseOne);
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseInstanceId: "CI7K3M2QAZ" },
      data: { kind: "course", course: courseOne },
    });
    app.navigate("/courses/CI4W8QF9AD");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseInstanceId: "CI4W8QF9AD" },
      data: undefined,
    });
    app.navigate("/courses/CI7K3M2QAZ");
    await nextTurn();
    fixture.courseViews.get("CI4W8QF9AD").resolve(courseRouteData("CI4W8QF9AD"));
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "course", course: courseOne });
    app.navigate("/assessment-attempts/00000000-0000-0000-0000-000000000001");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: {
        kind: "assessmentAttempt",
        assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
      },
      data: undefined,
    });
    assert.ok(fixture.attemptContexts.has("00000000-0000-0000-0000-000000000001"));
    assert.equal(fixture.attemptResolvers.has("00000000-0000-0000-0000-000000000001"), false);
    const context = assignmentAttemptContext("CI7K3M2QAZ");
    fixture.attemptContexts.get("00000000-0000-0000-0000-000000000001").resolve(context);
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "assessmentAttempt", context });
    app.navigate("/courses/CI7K3M2QAZ");
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "course", course: courseOne });
    assert.equal(app.mounts(), 1);
    app.dispose();
  },
);

test("stable controller retains separate Attempt views", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(
    fixture.queries,
    "/assessment-attempts/00000000-0000-0000-0000-000000000001",
  );
  assert.ok(fixture.attemptContexts.has("00000000-0000-0000-0000-000000000001"));
  const context = assignmentAttemptContext("CI7K3M2QAZ");
  fixture.attemptContexts.get("00000000-0000-0000-0000-000000000001").resolve(context);
  await nextTurn();
  assert.deepEqual(app.controller.data(), { kind: "assessmentAttempt", context });
  app.navigate("/assessment-attempts/00000000-0000-0000-0000-000000000001/summary");
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  assert.ok(fixture.histories.has("00000000-0000-0000-0000-000000000001"));
  const history = assignmentAttemptHistoryData("CI7K3M2QAZ");
  fixture.histories.get("00000000-0000-0000-0000-000000000001").resolve(history);
  await nextTurn();
  assert.deepEqual(app.controller.data(), {
    kind: "assessmentAttemptHistory",
    history,
  });
  app.dispose();
});

test("active Student Attempt retry replaces only its rejected R-reference context request", async () => {
  const attempts = [];
  const app = mountedController(
    {
      assessmentAttemptScope() {
        const deferred = createDeferredResolution();
        attempts.push(deferred);
        return deferred.promise;
      },
    },
    "/assessment-attempts/00000000-0000-0000-0000-000000000001",
  );
  assert.equal(attempts.length, 1);
  attempts[0].reject(new Error("temporary context failure"));
  await nextTurn();
  assert.equal(app.controller.loadState(), "rejected");
  assert.equal(app.controller.data(), undefined);
  app.controller.retry();
  assert.equal(attempts.length, 2);
  attempts[1].resolve(assignmentAttemptContext("CI7K3M2QAZ"));
  await nextTurn();
  assert.equal(app.controller.loadState(), "resolved");
  assert.deepEqual(app.controller.data(), {
    kind: "assessmentAttempt",
    context: assignmentAttemptContext("CI7K3M2QAZ"),
  });
  app.dispose();
});

test("shared transition driver retains one owner through scoped and unscoped routes", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/courses/CI7K3M2QAZ");
  await walkPathnamesThroughMountedApp({
    pathnames: [
      "/courses/CI7K3M2QAZ/assessments/A9D2RX5AF",
      "/courses/CI4W8QF9AD",
      "/library",
      "/courses/CI7K3M2",
    ],
    mount: () => ({ navigate: app.navigate }),
  });
  assert.equal(app.shellMounts(), 1);
  assert.deepEqual(app.controller.identity(), { kind: "invalid", scope: "courseInstance" });
});

test("Product, invalid, and rejected entries stay data-free without retrying", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/library");
  assert.deepEqual(app.controller.identity(), { kind: "product" });
  assert.equal(app.controller.data(), undefined);
  app.navigate("/courses/CI7K3M2");
  assert.deepEqual(app.controller.identity(), { kind: "invalid", scope: "courseInstance" });
  app.navigate("/courses/CI4W8QF9AD");
  fixture.courseViews.get("CI4W8QF9AD").reject(new Error("refused"));
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  app.navigate("/library");
  app.navigate("/courses/CI4W8QF9AD");
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  app.dispose();
});
