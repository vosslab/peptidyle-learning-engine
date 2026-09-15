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
      resolveAssignmentAttempt(reference) {
        return deferred(attemptResolvers, reference, "Attempt resolution").promise;
      },
      courseScope(reference) {
        return deferred(courseViews, reference, "Course scope").promise;
      },
      assessmentAttemptScope(reference) {
        return deferred(attemptContexts, reference, "Attempt context").promise;
      },
      assessmentAttemptHistory(reference) {
        return deferred(histories, reference, "Attempt history").promise;
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
  const app = mountedController(fixture.queries, "/courses/CI7K3M2Q");
  assert.deepEqual(app.controller.identity(), {
    kind: "courseInstance",
    courseReference: "CI7K3M2Q",
  });
  assert.ok(fixture.courseViews.has("CI7K3M2Q"));
  const course = courseRouteData("CI7K3M2Q");
  fixture.courseViews.get("CI7K3M2Q").resolve(course);
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
    const app = mountRouteScopeProviderHarness(applicationApi, "/courses/CI7K3M2Q");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseReference: "CI7K3M2Q" },
      data: undefined,
    });
    const courseOne = courseRouteData("CI7K3M2Q");
    fixture.courseViews.get("CI7K3M2Q").resolve(courseOne);
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseReference: "CI7K3M2Q" },
      data: { kind: "course", course: courseOne },
    });
    app.navigate("/courses/CI4W8QF9");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseReference: "CI4W8QF9" },
      data: undefined,
    });
    app.navigate("/courses/CI7K3M2Q");
    await nextTurn();
    fixture.courseViews.get("CI4W8QF9").resolve(courseRouteData("CI4W8QF9"));
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "course", course: courseOne });
    app.navigate("/assessment-attempts/R-1");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "assessmentAttempt", assessmentAttemptReference: "R-1" },
      data: undefined,
    });
    assert.ok(fixture.attemptContexts.has("R-1"));
    assert.equal(fixture.attemptResolvers.has("R-1"), false);
    const context = assignmentAttemptContext("CI7K3M2Q");
    fixture.attemptContexts.get("R-1").resolve(context);
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "assessmentAttempt", context });
    app.navigate("/courses/CI7K3M2Q");
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "course", course: courseOne });
    assert.equal(app.mounts(), 1);
    app.dispose();
  },
);

test("stable controller retains separate Attempt views", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/assessment-attempts/R-1");
  assert.ok(fixture.attemptContexts.has("R-1"));
  const context = assignmentAttemptContext("CI7K3M2Q");
  fixture.attemptContexts.get("R-1").resolve(context);
  await nextTurn();
  assert.deepEqual(app.controller.data(), { kind: "assessmentAttempt", context });
  app.navigate("/assessment-attempts/R-1/summary");
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  assert.ok(fixture.histories.has("R-1"));
  const history = assignmentAttemptHistoryData("CI7K3M2Q");
  fixture.histories.get("R-1").resolve(history);
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
    "/assessment-attempts/R-1",
  );
  assert.equal(attempts.length, 1);
  attempts[0].reject(new Error("temporary context failure"));
  await nextTurn();
  assert.equal(app.controller.loadState(), "rejected");
  assert.equal(app.controller.data(), undefined);
  app.controller.retry();
  assert.equal(attempts.length, 2);
  attempts[1].resolve(assignmentAttemptContext("CI7K3M2Q"));
  await nextTurn();
  assert.equal(app.controller.loadState(), "resolved");
  assert.deepEqual(app.controller.data(), {
    kind: "assessmentAttempt",
    context: assignmentAttemptContext("CI7K3M2Q"),
  });
  app.dispose();
});

test("shared transition driver retains one owner through scoped and unscoped routes", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/courses/CI7K3M2Q");
  await walkPathnamesThroughMountedApp({
    pathnames: [
      "/courses/CI7K3M2Q/assessments/A9D2RX5",
      "/courses/CI4W8QF9",
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
  app.navigate("/courses/CI4W8QF9");
  fixture.courseViews.get("CI4W8QF9").reject(new Error("refused"));
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  app.navigate("/library");
  app.navigate("/courses/CI4W8QF9");
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  app.dispose();
});
