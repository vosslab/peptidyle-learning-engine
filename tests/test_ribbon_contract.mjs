import assert from "node:assert/strict";
import test from "node:test";

import { buildRoutePath } from "../src/ribbon/ribbon_route_path.ts";
import { deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { routeParams } from "../src/navigation/route_params.ts";
import {
  userRoleMayAccessRoute,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
const LABELS = {};
const PARAMETER_VALUES = {
  courseInstanceId: "CI7K3M2QAZ",
  assessmentId: "A9D2RX5AF",
  assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
  questionId: "7K3M-79QP",
};

function paramsForRoute(route) {
  return Object.fromEntries(
    route.path
      .split("/")
      .filter((segment) => segment.startsWith(":"))
      .map((segment) => {
        const name = segment.slice(1);
        return [name, PARAMETER_VALUES[name]];
      }),
  );
}

function routeStateFor(routeId) {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
  assert.ok(route, `route ${routeId} must exist`);
  const pathname = buildRoutePath(route.id, paramsForRoute(route));
  assert.ok(pathname, `route ${route.id} must build`);
  const historyState =
    route.id === "assessmentAttempt" || route.id === "assessmentAttemptSummary"
      ? { assessmentAttemptId: PARAMETER_VALUES.assessmentAttemptId }
      : undefined;
  const declaredParams = routeParams(route, pathname, historyState);
  assert.ok(declaredParams, `route ${route.id} must extract`);
  return { route, params: { ...declaredParams } };
}

function controlsFor(routeId, userRole, labels = LABELS) {
  const routeState = routeStateFor(routeId);
  const params =
    routeId === "assessmentAttempt" || routeId === "assessmentAttemptSummary"
      ? {
          ...routeState.params,
          courseInstanceId: PARAMETER_VALUES.courseInstanceId,
          assessmentId: PARAMETER_VALUES.assessmentId,
        }
      : routeState.params;
  const model = deriveRibbonModel(
    {
      ...routeState,
      params,
      ...(userRole === "student"
        ? { studentCourses: [{ id: PARAMETER_VALUES.courseInstanceId, shortName: "BCHM 355" }] }
        : {}),
    },
    { userRole },
    labels,
  );
  return { model, controls: [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)] };
}

test("route construction fails closed for incomplete, surplus, and malformed input", () => {
  assert.equal(buildRoutePath("unknown", {}), undefined);
  assert.equal(buildRoutePath("courseAssessments", {}), undefined);
  assert.equal(
    buildRoutePath("courseAssessments", { courseInstanceId: "CI7K3M2QAZ", extra: "x" }),
    undefined,
  );
  assert.equal(
    buildRoutePath("courseAssessments", { courseInstanceId: "CI7K3M2QAZ/gradebook" }),
    undefined,
  );
  assert.equal(buildRoutePath("questionDetail", { questionId: "7K3%2FM9QP" }), undefined);
});

test("admission withholds unavailable controls and respects declared role ceilings", () => {
  for (const [routeId, role] of [
    ["assessmentAttempt", "student"],
    ["library", "instructor"],
    ["sysadminHome", "sysadmin"],
  ]) {
    for (const control of controlsFor(routeId, role).controls) {
      if (control.availability === "Unavailable") {
        assert.equal(control.href, undefined, `${routeId}/${role}/${control.id}`);
      }
      if (control.availability === "Available" && control.destination.kind === "route") {
        assert.equal(
          userRoleMayAccessRoute(control.destination.routeId, role),
          true,
          `${routeId}/${role}/${control.id}`,
        );
      }
    }
  }
});

test("global Student Tier 2 destinations stay available without Course context", () => {
  const routeState = routeStateFor("assessmentAttempt");
  const params = { assessmentAttemptId: routeState.params.assessmentAttemptId };
  const model = deriveRibbonModel(
    { route: routeState.route, params },
    { userRole: "student" },
    LABELS,
  );
  const controls = model.taskAreas.flatMap((area) => area.controls);
  for (const [id, href] of [
    ["allCoursework", "/student"],
    ["dueSoon", "/student/due-soon"],
    ["completedCoursework", "/student/completed"],
  ]) {
    const control = controls.find((candidate) => candidate.id === id);
    assert.deepEqual([control?.availability, control?.href], ["Available", href], id);
  }
  const activeAttempt = controls.find((control) => control.id === "activeAttempt");
  assert.deepEqual([activeAttempt?.availability, activeAttempt?.href], ["Unavailable", undefined]);
});

test("Student Attempt shortcuts use each server-selected Attempt and Course pair", () => {
  const routeState = routeStateFor("studentHome");
  const attemptId = PARAMETER_VALUES.assessmentAttemptId;
  const activeCourseInstanceId = "CI7K3M2QAZ";
  const feedbackCourseInstanceId = "CI6F2R8TA0";
  const model = deriveRibbonModel(
    {
      ...routeState,
      params: { ...routeState.params, courseInstanceId: PARAMETER_VALUES.courseInstanceId },
      activeAttemptId: attemptId,
      activeAttemptCourseInstanceId: activeCourseInstanceId,
      latestFeedbackAttemptId: attemptId,
      latestFeedbackCourseInstanceId: feedbackCourseInstanceId,
    },
    { userRole: "student" },
    LABELS,
  );
  const activeAttempt = model.taskAreas
    .flatMap((area) => area.controls)
    .find((control) => control.id === "activeAttempt");
  assert.ok(activeAttempt);
  assert.equal(activeAttempt.availability, "Available");
  assert.equal(
    activeAttempt.href,
    buildRoutePath("assessmentAttempt", { courseInstanceId: activeCourseInstanceId }),
  );
  assert.deepEqual(activeAttempt.state, { assessmentAttemptId: attemptId });
  const latestFeedbackModel = deriveRibbonModel(
    {
      ...routeStateFor("studentAttemptHistory"),
      params: { courseInstanceId: PARAMETER_VALUES.courseInstanceId },
      latestFeedbackAttemptId: attemptId,
      latestFeedbackCourseInstanceId: feedbackCourseInstanceId,
    },
    { userRole: "student" },
    LABELS,
  );
  const latestFeedback = latestFeedbackModel.taskAreas
    .flatMap((area) => area.controls)
    .find((control) => control.id === "studentLatestFeedback");
  assert.ok(latestFeedback);
  assert.equal(latestFeedback.availability, "Available");
  assert.equal(
    latestFeedback.href,
    buildRoutePath("assessmentAttemptSummary", { courseInstanceId: feedbackCourseInstanceId }),
  );
  assert.deepEqual(latestFeedback.state, { assessmentAttemptId: attemptId });
});

test("deferred assessment-attempt summary keeps Attempt History and its review crumb", () => {
  const state = routeStateFor("assessmentAttemptSummary");
  const model = deriveRibbonModel(state, { userRole: "student" }, LABELS);
  assert.deepEqual(model.breadcrumbs, [
    { label: "Home", href: "/student", current: false },
    { label: "Grades", href: "/student/grades", current: false },
    { label: "Attempt History", href: "/student/grades/attempt-history", current: false },
    {
      label: "Attempt history",
      href: "/courses/CI7K3M2QAZ/review",
      current: true,
      state: { assessmentAttemptId: state.params.assessmentAttemptId },
    },
  ]);
});

test("Student routes admit Students and withhold Instructor capabilities", () => {
  assert.equal(userRoleMayAccessRoute("assessmentAttempt", "student"), true);
  assert.equal(userRoleMayAccessRoute("assessmentAttempt", "instructor"), false);
  assert.equal(userRoleMayAccessRoute("assessmentAttempt", "sysadmin"), false);
  assert.equal(userRoleMayAccessRoute("library", "student"), false);
  assert.equal(userRoleMayAccessRoute("instructorAccounts", "student"), false);
  assert.equal(userRoleMayAccessRoute("gradebook", "student"), false);
});

test("Course breadcrumbs preserve the authored long name", () => {
  const state = routeStateFor("courseAssessments");
  for (const courseLongName of [
    "Course CI7K3M2QAZ: Molecular Biology",
    "CI7K3M2QAZ: Molecular Biology",
  ]) {
    const model = deriveRibbonModel(state, { userRole: "instructor" }, { courseLongName });
    assert.deepEqual(
      model.breadcrumbs.map(({ label }) => label),
      ["Home", "Courses", courseLongName],
    );
  }
});

test("hierarchy breadcrumbs keep ancestors and collapse only identical adjacent names", () => {
  const labels = {
    courseShortName: "BCHM 355",
    courseLongName: "Biochemistry I",
    assessmentTitle: "Problem Set 7",
    assessmentAttemptTitle: "Problem Set 7",
    questionTitle: "Catalytic triad",
    libraryObjectTitle: "Catalytic triad",
    blueprintCourseTitle: "Molecular Biology Blueprint",
  };
  const selectedTaskIds = (model) =>
    model.taskAreas.flatMap((area) =>
      area.controls.filter((control) => control.selected).map((control) => control.id),
    );
  const trail = (routeId, userRole, contextLabels, routeState = routeStateFor(routeId)) => {
    const model = deriveRibbonModel(routeState, { userRole }, contextLabels);
    assert.equal(model.breadcrumbs.filter((item) => item.current).length, 1, routeId);
    assert.equal(model.breadcrumbs.at(-1)?.current, true, routeId);
    for (const item of model.breadcrumbs) {
      assert.equal(typeof item.href, "string", `${routeId}:${item.label}`);
      assert.ok(routeContractForPathname(item.href), `${routeId}:${item.label}`);
    }
    return model;
  };

  const activeRoster = trail("courseRoster", "instructor", {
    ...labels,
    courseLifecycleState: "active",
  });
  assert.deepEqual(
    activeRoster.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "My Active Courses", "Biochemistry I", "Students"],
  );
  assert.deepEqual(selectedTaskIds(activeRoster), ["myActiveCourses"]);

  const inactiveRoster = trail("courseRoster", "instructor", {
    ...labels,
    courseLifecycleState: "inactive",
  });
  assert.deepEqual(
    inactiveRoster.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "My Inactive Courses", "Biochemistry I", "Students"],
  );
  assert.deepEqual(selectedTaskIds(inactiveRoster), ["myInactiveCourses"]);

  const unresolvedRoster = trail("courseRoster", "instructor", labels);
  assert.deepEqual(selectedTaskIds(unresolvedRoster), []);
  assert.equal(
    unresolvedRoster.breadcrumbs.some((item) => item.label.startsWith("My ")),
    false,
  );

  const question = trail("questionDetail", "instructor", labels);
  assert.deepEqual(
    question.breadcrumbs.map((item) => item.label),
    ["Home", "Questions", "Browse Question Library", "Catalytic triad"],
  );
  assert.deepEqual(selectedTaskIds(question), ["browseQuestionLibrary"]);

  const library = trail("library", "instructor", {});
  const searchCrumb = library.breadcrumbs.find((item) => item.label === "Search Question Library");
  const pageCrumb = library.breadcrumbs.find((item) => item.label === "Question Library");
  assert.equal(searchCrumb?.href, "/library");
  assert.equal(pageCrumb?.href, searchCrumb?.href);
  assert.notEqual(searchCrumb?.label, pageCrumb?.label);

  const progress = trail("studentCourseProgress", "student", labels, {
    ...routeStateFor("studentCourseProgress"),
    studentCourses: [{ id: PARAMETER_VALUES.courseInstanceId, shortName: "BCHM 355" }],
  });
  assert.deepEqual(
    progress.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "Biochemistry I", "Progress"],
  );
  assert.equal(
    progress.breadcrumbs.find((item) => item.label === "Biochemistry I")?.compactLabel,
    "BCHM 355",
  );
  assert.deepEqual(selectedTaskIds(progress), [
    `studentCourse:${PARAMETER_VALUES.courseInstanceId}`,
  ]);

  const progressNamedLikePage = trail(
    "studentCourseProgress",
    "student",
    { ...labels, courseLongName: "Progress", courseShortName: "PROG" },
    {
      ...routeStateFor("studentCourseProgress"),
      studentCourses: [{ id: PARAMETER_VALUES.courseInstanceId, shortName: "PROG" }],
    },
  );
  assert.deepEqual(
    progressNamedLikePage.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "Progress"],
  );
  assert.equal(
    progressNamedLikePage.breadcrumbs.at(-1)?.href,
    buildRoutePath("studentCourseProgress", routeStateFor("studentCourseProgress").params),
  );
  assert.notEqual(
    progressNamedLikePage.breadcrumbs.at(-1)?.href,
    progressNamedLikePage.taskAreas
      .flatMap((area) => area.controls)
      .find((control) => control.selected)?.href,
  );

  const attemptState = routeStateFor("assessmentAttempt");
  const attempt = trail("assessmentAttempt", "student", labels, {
    ...attemptState,
    params: {
      ...attemptState.params,
      courseInstanceId: PARAMETER_VALUES.courseInstanceId,
      assessmentId: PARAMETER_VALUES.assessmentId,
    },
  });
  assert.deepEqual(
    attempt.breadcrumbs.map((item) => item.label),
    ["Home", "Coursework", "All Coursework", "Biochemistry I", "Problem Set 7", "Attempt"],
  );
  assert.deepEqual(selectedTaskIds(attempt), ["allCoursework"]);

  const activeAttempt = trail("assessmentAttempt", "student", labels, {
    ...attemptState,
    params: {
      ...attemptState.params,
      courseInstanceId: PARAMETER_VALUES.courseInstanceId,
      assessmentId: PARAMETER_VALUES.assessmentId,
    },
    activeAttemptId: attemptState.params.assessmentAttemptId,
    activeAttemptCourseInstanceId: PARAMETER_VALUES.courseInstanceId,
  });
  assert.deepEqual(selectedTaskIds(activeAttempt), ["activeAttempt"]);
  assert.equal(
    activeAttempt.breadcrumbs.some((item) => item.label === "All Coursework"),
    false,
  );

  const summary = trail("assessmentAttemptSummary", "student", labels);
  assert.deepEqual(selectedTaskIds(summary), ["studentAttemptHistory"]);
  const summaryState = routeStateFor("assessmentAttemptSummary");
  const latestFeedback = trail("assessmentAttemptSummary", "student", labels, {
    ...summaryState,
    latestFeedbackAttemptId: summaryState.params.assessmentAttemptId,
    latestFeedbackCourseInstanceId: PARAMETER_VALUES.courseInstanceId,
  });
  assert.deepEqual(selectedTaskIds(latestFeedback), ["studentLatestFeedback"]);

  const malformed = ROUTE_CONTRACT.find((route) => route.id === "courseRoster");
  const invalid = deriveRibbonModel(
    { route: malformed, params: { courseInstanceId: "not-a-course" } },
    { userRole: "instructor" },
    { ...labels, courseLifecycleState: "active" },
  );
  assert.deepEqual(invalid.breadcrumbs, [{ label: "Home", href: "/instructor", current: false }]);
});
