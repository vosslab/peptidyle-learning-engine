// Exhaustive presentation metadata evidence for the frozen route contract.

import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TAB_IDS, ROUTE_CONTRACT } from "../src/route_contract.ts";

const EXPECTED_RIBBON_BY_ROUTE_ID = {
  courses: { scope: "product", tab: "courses", contentLayout: "reading" },
  signIn: { scope: "product", contentLayout: "reading" },
  pendingCourseInvitations: { scope: "product", contentLayout: "reading" },
  courseAssignments: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  assignmentOverview: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  assignmentAttempt: {
    scope: "assignmentAttempt",
    tab: "attempt",
    taskGroup: "assignmentAttempt",
    contentLayout: "reading",
  },
  assignmentAttemptSummary: {
    scope: "assignmentAttempt",
    tab: "attempt",
    taskGroup: "assignmentAttempt",
    contentLayout: "reading",
  },
  library: {
    scope: "product",
    tab: "questionLibrary",
    taskGroup: "questionLibrary",
    contentLayout: "fullWidth",
  },
  questionDetail: {
    scope: "product",
    tab: "questionLibrary",
    taskGroup: "questionLibrary",
    contentLayout: "reading",
  },
  blueprintCourses: { scope: "product", tab: "blueprintCourses", contentLayout: "reading" },
  blueprintCourseDetail: { scope: "product", tab: "blueprintCourses", contentLayout: "reading" },
  assignmentCreate: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  assignmentWorkspaceOverview: {
    scope: "courseInstance",
    tab: "assignments",
    taskGroup: "assignment",
    contentLayout: "fullWidth",
  },
  assignmentWorkspaceQuestions: {
    scope: "courseInstance",
    tab: "assignments",
    taskGroup: "assignment",
    contentLayout: "fullWidth",
  },
  assignmentWorkspacePolicies: {
    scope: "courseInstance",
    tab: "assignments",
    taskGroup: "assignment",
    contentLayout: "fullWidth",
  },
  assignmentWorkspaceStudentView: {
    scope: "courseInstance",
    tab: "assignments",
    taskGroup: "assignment",
    contentLayout: "fullWidth",
  },
  assignmentWorkspaceGradingOperations: {
    scope: "courseInstance",
    tab: "assignments",
    taskGroup: "assignment",
    contentLayout: "fullWidth",
  },
  assignmentPreview: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  gradebook: { scope: "courseInstance", tab: "gradebook", contentLayout: "fullWidth" },
  studentWorkInspection: { scope: "courseInstance", tab: "gradebook", contentLayout: "reading" },
  courseGradeSettings: {
    scope: "courseInstance",
    tab: "courseSetup",
    taskGroup: "courseSetup",
    contentLayout: "reading",
  },
  courseRoster: { scope: "courseInstance", tab: "students", contentLayout: "fullWidth" },
  teachingOperations: {
    scope: "courseInstance",
    tab: "teachingOperations",
    contentLayout: "reading",
  },
};

const PRODUCT_TABS = new Set(["courses", "questionLibrary", "blueprintCourses"]);
const COURSE_INSTANCE_TABS = new Set([
  "assignments",
  "students",
  "gradebook",
  "teachingOperations",
  "courseSetup",
]);

test("every declared route owns exactly its selected Ribbon presentation", () => {
  const expectedRouteIds = Object.keys(EXPECTED_RIBBON_BY_ROUTE_ID).sort();
  assert.equal(expectedRouteIds.length, 23);
  assert.deepEqual(ROUTE_CONTRACT.map((route) => route.id).sort(), expectedRouteIds);
  for (const route of ROUTE_CONTRACT) {
    assert.deepEqual(route.ribbon, EXPECTED_RIBBON_BY_ROUTE_ID[route.id], route.id);
  }
});

test("Ribbon scopes, tabs, and task groups remain valid presentation combinations", () => {
  const taskGroupByTab = {
    questionLibrary: "questionLibrary",
    assignments: "assignment",
    courseSetup: "courseSetup",
    attempt: "assignmentAttempt",
  };
  for (const route of ROUTE_CONTRACT) {
    const { scope, tab, taskGroup, contentLayout } = route.ribbon;
    assert.ok(["reading", "fullWidth"].includes(contentLayout), route.id);
    if (scope === "product") {
      assert.ok(tab === undefined || PRODUCT_TABS.has(tab), route.id);
    } else if (scope === "courseInstance") {
      assert.ok(tab === undefined || COURSE_INSTANCE_TABS.has(tab), route.id);
    } else {
      assert.equal(tab, "attempt", route.id);
    }
    if (taskGroup !== undefined) {
      assert.equal(taskGroupByTab[tab], taskGroup, route.id);
    }
  }
});

test("Context Controls and mechanical full-width routes retain their exact boundaries", () => {
  for (const id of ["signIn", "pendingCourseInvitations"]) {
    assert.equal(EXPECTED_RIBBON_BY_ROUTE_ID[id].tab, undefined, id);
    assert.equal(EXPECTED_RIBBON_BY_ROUTE_ID[id].taskGroup, undefined, id);
  }
  assert.deepEqual(
    ROUTE_CONTRACT.filter((route) => route.ribbon.contentLayout === "fullWidth")
      .map((route) => route.id)
      .sort(),
    [
      "assignmentWorkspaceGradingOperations",
      "assignmentWorkspaceOverview",
      "assignmentWorkspacePolicies",
      "assignmentWorkspaceQuestions",
      "assignmentWorkspaceStudentView",
      "courseRoster",
      "gradebook",
      "library",
    ],
  );
});

test("Instructor Accounts is catalog-ready but not selected by a current route", () => {
  assert.deepEqual(RIBBON_TAB_IDS, [
    "courses",
    "questionLibrary",
    "blueprintCourses",
    "assignments",
    "students",
    "gradebook",
    "teachingOperations",
    "blueprintUpdates",
    "courseSetup",
    "attempt",
    "instructorAccounts",
  ]);
  assert.ok(RIBBON_TAB_IDS.includes("instructorAccounts"));
  assert.equal(
    ROUTE_CONTRACT.some((route) => route.ribbon.tab === "instructorAccounts"),
    false,
  );
});
