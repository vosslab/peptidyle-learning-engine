import assert from "node:assert/strict";
import test from "node:test";

import { routeParams, routeScopeKey } from "../src/navigation/route_params.ts";
import {
  parseBlueprintCourseReference,
  parsePublicRouteReference,
} from "../src/navigation/public_route.ts";
import { ROUTE_CONTRACT, routeContractForPathname } from "../src/route_contract.ts";

const routeCases = [
  ["courses", "/", {}, { kind: "product" }],
  ["signIn", "/sign-in", {}, { kind: "product" }],
  ["pendingCourseInvitations", "/account/course-invitations", {}, { kind: "product" }],
  ["courseAssignments", "/courses/C-1", { courseRef: "C-1" }, courseKey("C-1")],
  [
    "assignmentOverview",
    "/courses/C-1/assignments/A-1",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentAttempt",
    "/assignment-attempts/R-1",
    { assignmentAttemptRef: "R-1" },
    attemptKey("R-1"),
  ],
  [
    "assignmentAttemptSummary",
    "/assignment-attempts/R-1/summary",
    { assignmentAttemptRef: "R-1" },
    attemptKey("R-1"),
  ],
  ["library", "/library", {}, { kind: "product" }],
  ["questionDetail", "/library/7k3m9qp", { questionRef: "7k3m9qp" }, { kind: "product" }],
  ["blueprintCourses", "/blueprint-courses", {}, { kind: "product" }],
  [
    "blueprintCourseDetail",
    "/blueprint-courses/BP-1",
    { blueprintCourseRef: "BP-1" },
    { kind: "product" },
  ],
  [
    "assignmentCreate",
    "/instructor/courses/C-1/assignments/new",
    { courseRef: "C-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentWorkspaceOverview",
    "/instructor/courses/C-1/assignments/A-1",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentWorkspaceQuestions",
    "/instructor/courses/C-1/assignments/A-1/questions",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentWorkspacePolicies",
    "/instructor/courses/C-1/assignments/A-1/policies",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentWorkspaceStudentView",
    "/instructor/courses/C-1/assignments/A-1/student-view",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentWorkspaceGradingOperations",
    "/instructor/courses/C-1/assignments/A-1/grading-operations",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  [
    "assignmentPreview",
    "/instructor/courses/C-1/assignments/A-1/delivery-check",
    { courseRef: "C-1", assignmentRef: "A-1" },
    courseKey("C-1"),
  ],
  ["gradebook", "/instructor/courses/C-1/gradebook", { courseRef: "C-1" }, courseKey("C-1")],
  [
    "studentWorkInspection",
    "/instructor/courses/C-1/gradebook/students/M-1/assignments/A-1/assignment-attempts/R-1",
    { courseRef: "C-1", membershipRef: "M-1", assignmentRef: "A-1", assignmentAttemptRef: "R-1" },
    courseKey("C-1"),
  ],
  [
    "courseGradeSettings",
    "/instructor/courses/C-1/grade-settings",
    { courseRef: "C-1" },
    courseKey("C-1"),
  ],
  ["courseRoster", "/instructor/courses/C-1/students", { courseRef: "C-1" }, courseKey("C-1")],
  [
    "teachingOperations",
    "/instructor/courses/C-1/teaching-operations",
    { courseRef: "C-1" },
    courseKey("C-1"),
  ],
];

function courseKey(courseReference) {
  return { kind: "courseInstance", courseReference };
}

function attemptKey(assignmentAttemptReference) {
  return { kind: "assignmentAttempt", assignmentAttemptReference };
}

function routeById(id) {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === id);
  assert.ok(route, `route ${id} must be declared`);
  return route;
}

function scopeKeyDeclaredByRoute(route, params) {
  switch (route.ribbon.scope) {
    case "product":
      return { kind: "product" };
    case "courseInstance":
      assert.equal(typeof params.courseRef, "string", `${route.id} needs a course reference`);
      return courseKey(params.courseRef);
    case "assignmentAttempt":
      assert.equal(
        typeof params.assignmentAttemptRef,
        "string",
        `${route.id} needs an Assignment Attempt reference`,
      );
      return attemptKey(params.assignmentAttemptRef);
  }
}

const INVALID_VALUE_BY_ROUTE_PARAM = {
  courseRef: "C-0",
  assignmentRef: "A-0",
  assignmentAttemptRef: "R-0",
  membershipRef: "M-0",
  questionRef: "7K3-M9QU",
  blueprintCourseRef: "BP-0",
};

test("route parameter zipper and scope key cover every declared route", () => {
  assert.equal(routeCases.length, ROUTE_CONTRACT.length);
  for (const [id, pathname, expectedParams, expectedScopeKey] of routeCases) {
    const route = routeById(id);
    assert.equal(routeContractForPathname(pathname), route, id);
    assert.deepEqual(routeParams(route, pathname), expectedParams, id);
    assert.deepEqual(routeScopeKey(pathname), expectedScopeKey, id);
  }
});

test("scope identity and malformed scope both derive from matched Ribbon route metadata", () => {
  assert.equal(routeCases.length, ROUTE_CONTRACT.length);
  for (const [id, pathname, expectedParams] of routeCases) {
    const route = routeById(id);
    const params = routeParams(route, pathname);
    assert.deepEqual(params, expectedParams, id);
    assert.deepEqual(routeScopeKey(pathname), scopeKeyDeclaredByRoute(route, params), id);

    for (const [name, value] of Object.entries(expectedParams)) {
      const invalidValue = INVALID_VALUE_BY_ROUTE_PARAM[name];
      assert.notEqual(invalidValue, undefined, `${id} has a supported route parameter`);
      const malformedPathname = pathname.replace(value, invalidValue);
      assert.deepEqual(
        routeScopeKey(malformedPathname),
        { kind: "invalid", scope: route.ribbon.scope },
        `${id}: malformed ${name} retains its matched Ribbon scope`,
      );
    }
  }
});

test("scope identity follows a temporary declared Ribbon scope change", () => {
  const courseAssignments = routeById("courseAssignments");
  const originalRibbon = courseAssignments.ribbon;
  const originalScope = originalRibbon.scope;

  try {
    originalRibbon.scope = "product";

    assert.deepEqual(routeScopeKey("/courses/C-1"), { kind: "product" });
    assert.deepEqual(routeScopeKey("/courses/C-0"), { kind: "invalid", scope: "product" });
  } finally {
    originalRibbon.scope = originalScope;
    assert.equal(courseAssignments.ribbon, originalRibbon);
    assert.equal(courseAssignments.ribbon.scope, originalScope);
  }
});

test("static declared routes use an empty record rather than a mismatch", () => {
  const library = routeById("library");
  assert.deepEqual(routeParams(library, "/library"), {});
  assert.equal(routeParams(library, "/library/7K3-M9QP"), undefined);
});

test("a structural declared route copy zips the selected canonical pattern", () => {
  const courseAssignments = routeById("courseAssignments");
  const copiedRoute = { ...courseAssignments };
  assert.deepEqual(routeParams(copiedRoute, "/courses/C-1"), { courseRef: "C-1" });
});

test("Blueprint Course references use the shared public route parser", () => {
  assert.equal(parseBlueprintCourseReference("BP-1"), "BP-1");
  assert.equal(parsePublicRouteReference("BP-1"), "BP-1");
});

test("all declared parameter parsers reject malformed route data without changing scope", () => {
  const numericFamilies = [
    ["C", (value) => `/courses/${value}`, "courseInstance"],
    ["A", (value) => `/courses/C-1/assignments/${value}`, "courseInstance"],
    ["R", (value) => `/assignment-attempts/${value}`, "assignmentAttempt"],
    [
      "M",
      (value) =>
        `/instructor/courses/C-1/gradebook/students/${value}` +
        "/assignments/A-1/assignment-attempts/R-1",
      "courseInstance",
    ],
  ];
  for (const [prefix, pathnameFor, scope] of numericFamilies) {
    for (const suffix of [
      "X-1",
      `${prefix}-0`,
      `${prefix}-01`,
      `${prefix}-2147483648`,
      "0198e000-0000-7000-8000-000000000001",
    ]) {
      assert.deepEqual(
        routeScopeKey(pathnameFor(suffix)),
        { kind: "invalid", scope },
        `${prefix}: ${suffix}`,
      );
    }
  }

  for (const invalid of ["P-50-v3", "7K3-M9QU", "00000000-0000-0000-0000-000000000000"]) {
    assert.deepEqual(routeScopeKey(`/library/${invalid}`), { kind: "invalid", scope: "product" });
  }
  for (const invalid of [
    "B-1",
    "BP-0",
    "BP-01",
    "BP-2147483648",
    "00000000-0000-0000-0000-000000000000",
  ]) {
    assert.deepEqual(routeScopeKey(`/blueprint-courses/${invalid}`), {
      kind: "invalid",
      scope: "product",
    });
  }
});

test("every nested reference is validated before a course scope key is issued", () => {
  const base =
    "/instructor/courses/C-1/gradebook/students/M-1/assignments/A-1/assignment-attempts/R-1";
  for (const [from, to] of [
    ["M-1", "M-0"],
    ["A-1", "A-0"],
    ["R-1", "R-0"],
  ]) {
    assert.deepEqual(routeScopeKey(base.replace(from, to)), {
      kind: "invalid",
      scope: "courseInstance",
    });
  }
});

test("route-shape hostility fails closed and never supplies partial parameters", () => {
  const courseRoute = routeById("courseAssignments");
  for (const pathname of [
    "/unknown",
    "/courses/C-1/extra",
    "/courses",
    "/courses//C-1",
    "/courses/C-1/",
    "/courses/C-1?query=value",
    "/courses/C-1#fragment",
    "/%63ourses/C-1",
    "/Courses/C-1",
  ]) {
    assert.deepEqual(routeScopeKey(pathname), { kind: "invalid", scope: undefined }, pathname);
    assert.equal(routeParams(courseRoute, pathname), undefined, pathname);
  }

  assert.deepEqual(routeParams(courseRoute, "/courses/C%2F1"), { courseRef: "C%2F1" });
  assert.deepEqual(routeScopeKey("/courses/C%2F1"), {
    kind: "invalid",
    scope: "courseInstance",
  });
});
