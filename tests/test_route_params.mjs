import assert from "node:assert/strict";
import test from "node:test";

import { routeParams, routeScopeKey } from "../src/navigation/route_params.ts";
import {
  parseBlueprintCourseReference,
  parsePublicRouteReference,
} from "../src/navigation/public_route.ts";
import { ROUTE_CONTRACT, routeContractForPathname } from "../src/route_contract.ts";

function courseKey(courseReference) {
  return { kind: "courseInstance", courseReference };
}

function attemptKey(assessmentAttemptReference) {
  return { kind: "assessmentAttempt", assessmentAttemptReference };
}

function routeById(id) {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === id);
  assert.ok(route, `route ${id} must be declared`);
  return route;
}

// Permanent contract: these are the external Assessment routes users and
// support staff can communicate. A regression could silently restore generic
// Assignment navigation or make the canonical Properties task unaddressable.
test("canonical Assessment routes extract opaque references and reject the retired generic path", () => {
  const routeCases = [
    [
      "assessmentOverview",
      "/courses/CI7K3M2Q/assessments/A9D2RX5",
      { courseRef: "CI7K3M2Q", assessmentRef: "A9D2RX5" },
      courseKey("CI7K3M2Q"),
    ],
    [
      "assessmentWorkspacePolicies",
      "/instructor/courses/CI7K3M2Q/assessments/A9D2RX5/properties",
      { courseRef: "CI7K3M2Q", assessmentRef: "A9D2RX5" },
      courseKey("CI7K3M2Q"),
    ],
    [
      "assessmentAttempt",
      "/assessment-attempts/R-1",
      { assessmentAttemptRef: "R-1" },
      attemptKey("R-1"),
    ],
  ];
  for (const [id, pathname, expectedParams, expectedScopeKey] of routeCases) {
    const route = routeById(id);
    assert.equal(routeContractForPathname(pathname), route, id);
    assert.deepEqual(routeParams(route, pathname), expectedParams, id);
    assert.deepEqual(routeScopeKey(pathname), expectedScopeKey, id);
  }
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2Q/assignments/A9D2RX5/policies"),
    undefined,
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2Q/assessments/A9D2RX5/policies"),
    undefined,
  );
});

test("retired per-presentation submission paths are not declared routes", () => {
  const pathname =
    "/courses/CI7K3M2Q/assessments/A9D2RX5/presentations/0123456789abcdef0123456789abcdef";
  assert.equal(routeContractForPathname(pathname), undefined);
  assert.deepEqual(routeScopeKey(pathname), { kind: "invalid", scope: undefined });
});

test("scope identity follows a temporary declared Ribbon scope change", () => {
  const courseAssessments = routeById("courseAssessments");
  const originalRibbon = courseAssessments.ribbon;
  const originalScope = originalRibbon.scope;

  try {
    originalRibbon.scope = "product";

    assert.deepEqual(routeScopeKey("/courses/CI7K3M2Q"), { kind: "product" });
    assert.deepEqual(routeScopeKey("/courses/CI7K3M2"), { kind: "invalid", scope: "product" });
  } finally {
    originalRibbon.scope = originalScope;
    assert.equal(courseAssessments.ribbon, originalRibbon);
    assert.equal(courseAssessments.ribbon.scope, originalScope);
  }
});

test("static declared routes use an empty record rather than a mismatch", () => {
  const library = routeById("library");
  assert.deepEqual(routeParams(library, "/library"), {});
  assert.equal(routeParams(library, "/library/7K3M-X9QP"), undefined);
});

test("a structural declared route copy zips the selected canonical pattern", () => {
  const courseAssessments = routeById("courseAssessments");
  const copiedRoute = { ...courseAssessments };
  assert.deepEqual(routeParams(copiedRoute, "/courses/CI7K3M2Q"), { courseRef: "CI7K3M2Q" });
});

test("Blueprint Course references use the shared public route parser", () => {
  assert.equal(parseBlueprintCourseReference("BP7K3M2Q"), "BP7K3M2Q");
  assert.equal(parsePublicRouteReference("BP7K3M2Q"), "BP7K3M2Q");
});

test("route-shape hostility fails closed and never supplies partial parameters", () => {
  const courseRoute = routeById("courseAssessments");
  for (const pathname of [
    "/unknown",
    "/courses/CI7K3M2Q/extra",
    "/courses",
    "/courses//CI7K3M2Q",
    "/courses/CI7K3M2Q/",
    "/courses/CI7K3M2Q?query=value",
    "/courses/CI7K3M2Q#fragment",
    "/%63ourses/CI7K3M2Q",
    "/Courses/CI7K3M2Q",
  ]) {
    assert.deepEqual(routeScopeKey(pathname), { kind: "invalid", scope: undefined }, pathname);
    assert.equal(routeParams(courseRoute, pathname), undefined, pathname);
  }

  assert.deepEqual(routeParams(courseRoute, "/courses/CI%2F7K3M2Q"), { courseRef: "CI%2F7K3M2Q" });
  assert.deepEqual(routeScopeKey("/courses/CI%2F7K3M2Q"), {
    kind: "invalid",
    scope: "courseInstance",
  });
});
