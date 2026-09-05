// Ribbon catalog tests validate navigation truthfulness and canonical information architecture.

import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TAB_IDS, ROUTE_CONTRACT } from "../src/route_contract.ts";
import { TAB_CATALOG, RIBBON_TASK_CATALOG } from "../src/ribbon/ribbon_catalog.ts";

// This is the approved Ribbon information architecture. Its order is the
// visible order; values are deliberately exact so future catalog edits cannot
// silently recategorize, fabricate, or resize a destination.
const EXPECTED_CONTROL_METADATA = [
  [
    "tab",
    "courses",
    "Courses",
    { kind: "route", routeId: "courses" },
    [],
    "primary",
    "critical",
    "standard",
    true,
    false,
  ],
  [
    "tab",
    "questionLibrary",
    "Question Library",
    { kind: "route", routeId: "library" },
    [],
    "primary",
    "critical",
    "standard",
    true,
    false,
  ],
  [
    "tab",
    "blueprintCourses",
    "Blueprint Courses",
    { kind: "route", routeId: "blueprintCourses" },
    [],
    "supporting",
    "normal",
    "compact",
    false,
    false,
  ],
  [
    "tab",
    "assignments",
    "Assignments",
    { kind: "route", routeId: "courseAssignments" },
    ["courseRef"],
    "primary",
    "critical",
    "standard",
    true,
    false,
  ],
  [
    "tab",
    "students",
    "Students",
    { kind: "route", routeId: "courseRoster" },
    ["courseRef"],
    "supporting",
    "normal",
    "standard",
    true,
    false,
  ],
  [
    "tab",
    "gradebook",
    "Gradebook",
    { kind: "route", routeId: "gradebook" },
    ["courseRef"],
    "primary",
    "critical",
    "standard",
    true,
    false,
  ],
  [
    "tab",
    "teachingOperations",
    "Teaching Operations",
    { kind: "route", routeId: "teachingOperations" },
    ["courseRef"],
    "supporting",
    "normal",
    "standard",
    false,
    false,
  ],
  [
    "tab",
    "blueprintUpdates",
    "Blueprint Updates",
    { kind: "future", futureId: "blueprintUpdates" },
    ["courseRef"],
    "supporting",
    "normal",
    "compact",
    false,
    false,
  ],
  [
    "tab",
    "courseSetup",
    "Course Setup",
    { kind: "future", futureId: "courseSetup" },
    ["courseRef"],
    "supporting",
    "normal",
    "compact",
    true,
    false,
  ],
  [
    "tab",
    "attempt",
    "Attempt",
    { kind: "route", routeId: "assignmentAttempt" },
    ["assignmentAttemptRef"],
    "primary",
    "critical",
    "standard",
    true,
    false,
  ],
  [
    "tab",
    "instructorAccounts",
    "Instructor Accounts",
    { kind: "future", futureId: "instructorAccounts" },
    [],
    "primary",
    "critical",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "allQuestions",
    "All Questions",
    { kind: "route", routeId: "library" },
    [],
    "questionLibrary",
    "questionDestinations",
    "primary",
    "critical",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "myQuestions",
    "My Questions",
    { kind: "future", futureId: "myQuestions" },
    [],
    "questionLibrary",
    "questionDestinations",
    "primary",
    "critical",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "myQuestionDrafts",
    "My Question Drafts",
    { kind: "future", futureId: "myQuestionDrafts" },
    [],
    "questionLibrary",
    "questionDestinations",
    "supporting",
    "normal",
    "standard",
    true,
    false,
  ],
  [
    "task",
    "starred",
    "Starred",
    { kind: "future", futureId: "starredQuestions" },
    [],
    "questionLibrary",
    "questionRelationships",
    "supporting",
    "normal",
    "compact",
    true,
    true,
  ],
  [
    "task",
    "watched",
    "Watched",
    { kind: "future", futureId: "watchedQuestions" },
    [],
    "questionLibrary",
    "questionRelationships",
    "supporting",
    "normal",
    "compact",
    true,
    true,
  ],
  [
    "task",
    "assignmentOverview",
    "Overview",
    { kind: "route", routeId: "assignmentWorkspaceOverview" },
    ["courseRef", "assignmentRef"],
    "assignment",
    "assignment",
    "primary",
    "critical",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "assignmentQuestions",
    "Questions",
    { kind: "route", routeId: "assignmentWorkspaceQuestions" },
    ["courseRef", "assignmentRef"],
    "assignment",
    "assignment",
    "primary",
    "critical",
    "standard",
    true,
    false,
  ],
  [
    "task",
    "assignmentPolicies",
    "Policies",
    { kind: "route", routeId: "assignmentWorkspacePolicies" },
    ["courseRef", "assignmentRef"],
    "assignment",
    "assignment",
    "supporting",
    "normal",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "assignmentGradingOperations",
    "Grading Operations",
    { kind: "route", routeId: "assignmentWorkspaceGradingOperations" },
    ["courseRef", "assignmentRef"],
    "assignment",
    "assignment",
    "supporting",
    "normal",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "assignmentStudentView",
    "Student View",
    { kind: "route", routeId: "assignmentWorkspaceStudentView" },
    ["courseRef", "assignmentRef"],
    "assignment",
    "assignment",
    "supporting",
    "normal",
    "standard",
    true,
    false,
  ],
  [
    "task",
    "gradeSettings",
    "Grade Settings",
    { kind: "route", routeId: "courseGradeSettings" },
    ["courseRef"],
    "courseSetup",
    "courseSetup",
    "primary",
    "critical",
    "standard",
    false,
    false,
  ],
  [
    "task",
    "appearance",
    "Appearance",
    { kind: "future", futureId: "courseAppearance" },
    ["courseRef"],
    "courseSetup",
    "courseSetup",
    "supporting",
    "normal",
    "standard",
    true,
    false,
  ],
  [
    "task",
    "backToAssignments",
    "Back to Assignments",
    { kind: "route", routeId: "courseAssignments" },
    ["courseRef"],
    "assignmentAttempt",
    "assignmentAttempt",
    "primary",
    "critical",
    "standard",
    true,
    true,
  ],
];

function paramsInPath(path) {
  return [...path.matchAll(/:([A-Za-z]+Ref)/g)].map((match) => match[1]);
}

test("catalog exactly implements the approved Ribbon metadata contract", () => {
  const actual = [
    ...TAB_CATALOG.map((control) => [
      "tab",
      control.id,
      control.label,
      control.destination,
      control.requiredParams,
      control.role,
      control.priority,
      control.presentation,
      control.iconBearing,
      control.iconOnlySafe,
    ]),
    ...RIBBON_TASK_CATALOG.map((control) => [
      "task",
      control.id,
      control.label,
      control.destination,
      control.requiredParams,
      control.taskGroup,
      control.area,
      control.role,
      control.priority,
      control.presentation,
      control.iconBearing,
      control.iconOnlySafe,
    ]),
  ];
  assert.deepEqual(actual, EXPECTED_CONTROL_METADATA);
  assert.deepEqual(
    TAB_CATALOG.map(({ id }) => id),
    RIBBON_TAB_IDS,
  );
});

test("task catalog preserves canonical task groups, areas, labels, and adjacency", () => {
  assert.deepEqual(
    RIBBON_TASK_CATALOG.filter(({ taskGroup }) => taskGroup === "questionLibrary").map(
      ({ area }) => area,
    ),
    [
      "questionDestinations",
      "questionDestinations",
      "questionDestinations",
      "questionRelationships",
      "questionRelationships",
    ],
  );
});

test("route destinations are declared routes with exactly their needed pattern parameters", () => {
  const routesById = new Map(ROUTE_CONTRACT.map((route) => [route.id, route]));
  for (const control of [...TAB_CATALOG, ...RIBBON_TASK_CATALOG]) {
    if (control.destination.kind !== "route") continue;
    const route = routesById.get(control.destination.routeId);
    assert.ok(route, `${control.id} must name a declared route`);
    assert.deepEqual(control.requiredParams, paramsInPath(route.path), control.id);
  }
});

test("future destinations remain declared identities rather than fabricated routes", () => {
  const routeIds = new Set(ROUTE_CONTRACT.map((route) => route.id));
  for (const control of [...TAB_CATALOG, ...RIBBON_TASK_CATALOG]) {
    if (control.destination.kind !== "future") continue;
    assert.equal(routeIds.has(control.destination.futureId), false, control.id);
    assert.equal("routeId" in control.destination, false, control.id);
  }
  assert.equal(TAB_CATALOG.find(({ id }) => id === "courseSetup")?.destination.kind, "future");
  assert.equal(
    RIBBON_TASK_CATALOG.find(({ id }) => id === "myQuestions")?.destination.kind,
    "future",
  );
  assert.deepEqual(RIBBON_TASK_CATALOG.find(({ id }) => id === "myQuestionDrafts")?.destination, {
    kind: "future",
    futureId: "myQuestionDrafts",
  });
});

test("catalog controls are navigation only and retain independent density metadata", () => {
  for (const control of [...TAB_CATALOG, ...RIBBON_TASK_CATALOG]) {
    assert.ok(["primary", "supporting"].includes(control.role), control.id);
    assert.ok(["critical", "normal"].includes(control.priority), control.id);
    assert.ok(["standard", "compact"].includes(control.presentation), control.id);
    assert.equal(typeof control.iconBearing, "boolean", control.id);
    assert.equal(typeof control.iconOnlySafe, "boolean", control.id);
    assert.equal(control.iconOnlySafe && !control.iconBearing, false, control.id);
    assert.equal("operation" in control, false, control.id);
  }
  assert.equal(
    RIBBON_TASK_CATALOG.some(({ label }) => label === "Create Assignment"),
    false,
  );
  assert.equal(
    TAB_CATALOG.some(({ label }) => ["Account", "Profile"].includes(label)),
    false,
  );
  assert.equal(
    RIBBON_TASK_CATALOG.some(({ label }) => ["Account", "Profile"].includes(label)),
    false,
  );
});
