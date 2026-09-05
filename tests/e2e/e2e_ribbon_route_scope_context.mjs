// e2e_ribbon_route_scope_context.mjs - browser-condition integration for the Ribbon scope owner.

import assert from "node:assert/strict";
import test from "node:test";

import { createRoot, createSignal } from "solid-js";

import {
  createDeferredResolution,
  walkPathnamesThroughMountedApp,
} from "../support/ribbon_test_support.ts";
import {
  assignmentAttemptScreenData,
  assignmentAttemptSummaryData,
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
  const courseResolvers = new Map();
  const attemptResolvers = new Map();
  const courseViews = new Map();
  const screens = new Map();
  const summaries = new Map();
  const deferred = (map, key, label) => {
    if (map.has(key)) throw new Error(`redundant ${label} query for ${key}`);
    const value = createDeferredResolution();
    map.set(key, value);
    return value;
  };
  return {
    courseResolvers,
    attemptResolvers,
    courseViews,
    screens,
    summaries,
    queries: {
      resolveCourse(reference) {
        return deferred(courseResolvers, reference, "Course resolution").promise;
      },
      resolveAssignmentAttempt(reference) {
        return deferred(attemptResolvers, reference, "Attempt resolution").promise;
      },
      courseScope(courseId) {
        return deferred(courseViews, courseId, "Course scope").promise;
      },
      assignmentAttemptScreen(attemptId) {
        return deferred(screens, attemptId, "Attempt screen").promise;
      },
      assignmentAttemptSummary(attemptId) {
        return deferred(summaries, attemptId, "Attempt summary").promise;
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

test("controller owns pending resolution before any consumer reads its data", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/courses/C-1");
  assert.deepEqual(app.controller.identity(), { kind: "courseInstance", courseReference: "C-1" });
  assert.ok(fixture.courseResolvers.has("C-1"));
  fixture.courseResolvers.get("C-1").resolve({ courseId: "course-one" });
  await nextTurn();
  assert.ok(fixture.courseViews.has("course-one"));
  const course = courseRouteData("C-1");
  fixture.courseViews.get("course-one").resolve(course);
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
        resolveCourse: queryFunction(fixture.queries.resolveCourse, "test-resolve-course"),
        resolveAssignmentAttempt: queryFunction(
          fixture.queries.resolveAssignmentAttempt,
          "test-resolve-attempt",
        ),
        courseScope: queryFunction(fixture.queries.courseScope, "test-course-scope"),
        assignmentAttemptScreen: queryFunction(
          fixture.queries.assignmentAttemptScreen,
          "test-attempt-screen",
        ),
        assignmentAttemptSummary: queryFunction(
          fixture.queries.assignmentAttemptSummary,
          "test-attempt-summary",
        ),
      },
    };
    const { mountRouteScopeProviderHarness } = await loadRouteScopeProviderHarness();
    const app = mountRouteScopeProviderHarness(applicationApi, "/courses/C-1");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseReference: "C-1" },
      data: undefined,
    });
    fixture.courseResolvers.get("C-1").resolve({ courseId: "course-one" });
    await nextTurn();
    const courseOne = courseRouteData("C-1");
    fixture.courseViews.get("course-one").resolve(courseOne);
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseReference: "C-1" },
      data: { kind: "course", course: courseOne },
    });
    app.navigate("/courses/C-2");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "courseInstance", courseReference: "C-2" },
      data: undefined,
    });
    fixture.courseResolvers.get("C-2").resolve({ courseId: "course-two" });
    await nextTurn();
    app.navigate("/courses/C-1");
    await nextTurn();
    fixture.courseViews.get("course-two").resolve(courseRouteData("C-2"));
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "course", course: courseOne });
    app.navigate("/assignment-attempts/R-1");
    await nextTurn();
    assert.deepEqual(app.latest(), {
      identity: { kind: "assignmentAttempt", assignmentAttemptReference: "R-1" },
      data: undefined,
    });
    fixture.attemptResolvers.get("R-1").resolve({ assignmentAttemptId: "attempt-one" });
    await nextTurn();
    const screen = assignmentAttemptScreenData("C-1");
    fixture.screens.get("attempt-one").resolve(screen);
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "assignmentAttempt", screen });
    app.navigate("/courses/C-1");
    await nextTurn();
    assert.deepEqual(app.latest().data, { kind: "course", course: courseOne });
    assert.equal(app.mounts(), 1);
    app.dispose();
  },
);

test("stable controller retains separate Attempt views", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/assignment-attempts/R-1");
  fixture.attemptResolvers.get("R-1").resolve({ assignmentAttemptId: "attempt-one" });
  await nextTurn();
  const screen = assignmentAttemptScreenData("C-1");
  fixture.screens.get("attempt-one").resolve(screen);
  await nextTurn();
  assert.deepEqual(app.controller.data(), { kind: "assignmentAttempt", screen });
  app.navigate("/assignment-attempts/R-1/summary");
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  const summary = assignmentAttemptSummaryData("C-1");
  fixture.summaries.get("attempt-one").resolve(summary);
  await nextTurn();
  assert.deepEqual(app.controller.data(), {
    kind: "assignmentAttemptSummary",
    response: summary,
  });
  app.dispose();
});

test("shared transition driver retains one owner through scoped and unscoped routes", async () => {
  const fixture = createDeferredQueries();
  const app = mountedController(fixture.queries, "/courses/C-1");
  await walkPathnamesThroughMountedApp({
    pathnames: ["/courses/C-1/assignments/A-1", "/courses/C-2", "/library", "/courses/C-01"],
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
  app.navigate("/courses/C-01");
  assert.deepEqual(app.controller.identity(), { kind: "invalid", scope: "courseInstance" });
  app.navigate("/courses/C-2");
  fixture.courseResolvers.get("C-2").reject(new Error("refused"));
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  app.navigate("/library");
  app.navigate("/courses/C-2");
  await nextTurn();
  assert.equal(app.controller.data(), undefined);
  app.dispose();
});
