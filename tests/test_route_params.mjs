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
      "/courses/CIABCDEFGS/assessments/AABCDEFG8",
      { courseRef: "CIABCDEFGS", assessmentRef: "AABCDEFG8" },
      courseKey("CIABCDEFGS"),
    ],
    [
      "assessmentWorkspacePolicies",
      "/instructor/courses/CIABCDEFGS/assessments/AABCDEFG8/properties",
      { courseRef: "CIABCDEFGS", assessmentRef: "AABCDEFG8" },
      courseKey("CIABCDEFGS"),
    ],
    [
      "assessmentAttempt",
      "/assessment-attempts/00000000-0000-0000-0000-000000000001",
      { assessmentAttemptRef: "00000000-0000-0000-0000-000000000001" },
      attemptKey("00000000-0000-0000-0000-000000000001"),
    ],
  ];
  for (const [id, pathname, expectedParams, expectedScopeKey] of routeCases) {
    const route = routeById(id);
    assert.equal(routeContractForPathname(pathname), route, id);
    assert.deepEqual(routeParams(route, pathname), expectedParams, id);
    assert.deepEqual(routeScopeKey(pathname), expectedScopeKey, id);
  }
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assignments/A9D2RX5AF/policies"),
    undefined,
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/policies"),
    undefined,
  );
});

test("retired per-presentation submission paths are not declared routes", () => {
  const pathname =
    "/courses/CIABCDEFGS/assessments/AABCDEFG8/presentations/0123456789abcdef0123456789abcdef";
  assert.equal(routeContractForPathname(pathname), undefined);
  assert.deepEqual(routeScopeKey(pathname), { kind: "invalid", scope: undefined });
});

test("scope identity follows a temporary declared Ribbon scope change", () => {
  const courseAssessments = routeById("courseAssessments");
  const originalRibbon = courseAssessments.ribbon;
  const originalScope = originalRibbon.scope;

  try {
    originalRibbon.scope = "product";

    assert.deepEqual(routeScopeKey("/courses/CIABCDEFGS"), { kind: "product" });
    assert.deepEqual(routeScopeKey("/courses/CIABCDEFG"), { kind: "invalid", scope: "product" });
  } finally {
    originalRibbon.scope = originalScope;
    assert.equal(courseAssessments.ribbon, originalRibbon);
    assert.equal(courseAssessments.ribbon.scope, originalScope);
  }
});

test("static declared routes use an empty record rather than a mismatch", () => {
  const library = routeById("library");
  assert.deepEqual(routeParams(library, "/library"), {});
  assert.equal(routeParams(library, "/library/7K3M-79QP"), undefined);
});

test("a structural declared route copy zips the selected canonical pattern", () => {
  const courseAssessments = routeById("courseAssessments");
  const copiedRoute = { ...courseAssessments };
  assert.deepEqual(routeParams(copiedRoute, "/courses/CIABCDEFGS"), { courseRef: "CIABCDEFGS" });
});

test("Blueprint Course references use the shared public route parser", () => {
  assert.equal(parseBlueprintCourseReference("BPABCDEFGJ"), "BPABCDEFGJ");
  assert.equal(parsePublicRouteReference("BPABCDEFGJ"), "BPABCDEFGJ");
});

test("route-shape hostility fails closed and never supplies partial parameters", () => {
  const courseRoute = routeById("courseAssessments");
  for (const pathname of [
    "/unknown",
    "/courses/CIABCDEFGS/extra",
    "/courses",
    "/courses//CIABCDEFGS",
    "/courses/CIABCDEFGS/",
    "/courses/CIABCDEFGS?query=value",
    "/courses/CIABCDEFGS#fragment",
    "/%63ourses/CIABCDEFGS",
    "/Courses/CIABCDEFGS",
  ]) {
    assert.deepEqual(routeScopeKey(pathname), { kind: "invalid", scope: undefined }, pathname);
    assert.equal(routeParams(courseRoute, pathname), undefined, pathname);
  }

  assert.deepEqual(routeParams(courseRoute, "/courses/CI%2FABCDEFGS"), {
    courseRef: "CI%2FABCDEFGS",
  });
  assert.deepEqual(routeScopeKey("/courses/CI%2FABCDEFGS"), {
    kind: "invalid",
    scope: "courseInstance",
  });
});
