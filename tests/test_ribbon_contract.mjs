import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import { profileRoleMayManageImage } from "../src/features/profile_avatar/profile_avatar_role.ts";
import { buildRoutePath } from "../src/ribbon/ribbon_route_path.ts";
import {
  deriveRibbonModel,
  ribbonModelAvailabilityMayAccessRoute,
} from "../src/ribbon/ribbon_contract.ts";
import { routeParams } from "../src/navigation/route_params.ts";
import {
  userRoleMayAccessRoute,
  userRoleHomePath,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { CAPABILITY_REGISTRY } from "../src/ribbon/capability_registry.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { ribbonTierTwoSchemaFor } from "../src/ribbon/ribbon_schema.ts";
import { loadAppRibbonForSsr } from "./support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "./support/ribbon_model_fixtures.ts";

const USER_ROLES = ["student", "instructor", "sysadmin"];
const LABELS = {};
const PARAMETER_VALUES = {
  courseInstanceId: "CI7K3M2QAZ",
  assessmentId: "A9D2RX5AF",
  assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
  membershipId: "00000000-0000-0000-0000-00000000000b",
  questionId: "7K3M-79QP",
  draftQuestionId: "0198e000-0000-7000-8000-000000000001",
  blueprintCourseId: "BP7K3M2QAF",
  proposalId: "e3396265-6653-4c65-bc9b-8d869c142d87",
  accountId: "U00000009",
};
const CATALOG = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG];

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

function restoreDescriptors(target, descriptors) {
  for (const key of Reflect.ownKeys(target)) {
    if (!(key in descriptors)) Reflect.deleteProperty(target, key);
  }
  Object.defineProperties(target, descriptors);
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

test("derived Ribbon controls are immutable model-owned descriptors", () => {
  const { model, controls } = controlsFor("library", "instructor");
  assert.equal(Object.isFrozen(model), true);
  for (const control of controls) {
    const catalogControl = CATALOG.find((candidate) => candidate.id === control.id);
    assert.ok(catalogControl, control.id);
    assert.equal(Object.isFrozen(control), true, control.id);
    assert.notEqual(control.destination, catalogControl.destination, control.id);
    assert.equal(Reflect.set(control.destination, "kind", "future"), false, control.id);
  }
});

test("admission withholds unavailable controls and respects declared role ceilings", () => {
  for (const route of ROUTE_CONTRACT) {
    for (const role of USER_ROLES) {
      for (const control of controlsFor(route.id, role).controls) {
        if (control.availability === "Unavailable") {
          assert.equal(control.href, undefined, `${route.id}/${role}/${control.id}`);
        }
        if (control.availability === "Available" && control.destination.kind === "route") {
          assert.equal(
            userRoleMayAccessRoute(control.destination.routeId, role),
            true,
            `${route.id}/${role}/${control.id}`,
          );
        }
      }
    }
  }
});

// Permanent contract: every signed-in role reaches the same self-owned account
// commands through the compact Profile menu. A regression would either strand a
// role from Profile or scatter Sign out back into the top bar.
test("every signed-in User Role has one accessible generic Profile end control", async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  for (const [role, routeId] of [
    ["student", "studentHome"],
    ["instructor", "instructorHome"],
    ["sysadmin", "sysadminHome"],
  ]) {
    const model = controlsFor(routeId, role).model;
    assert.deepEqual(
      model.context.accountControls,
      [
        {
          id: "profile",
          label: "Profile settings",
          availability: "Available",
          glyph: "profile",
          href: "/profile",
        },
      ],
      `${role} has the one shared Profile destination`,
    );
    const html = renderToString(() => createComponent(RealAppRibbon, { model }));
    assert.equal(
      (html.match(/data-ribbon-context-control="profile"/g) ?? []).length,
      1,
      `${role} has one Profile control`,
    );
    const profile = html.match(
      /<button[^>]*data-ribbon-context-control="profile"[^>]*>[\s\S]*?<\/button>/,
    )?.[0];
    assert.ok(profile, `${role} Profile is a button while its menu is pending`);
    assert.match(profile, /aria-label="Profile"/);
    assert.match(profile, /aria-haspopup="menu"/);
    assert.match(profile, /aria-expanded="false"/);
    assert.match(profile, /data-ribbon-profile-avatar="generic"/);
    assert.match(profile, /data-ribbon-glyph="circle-user"/);
    assert.doesNotMatch(profile, />Profile</);
    assert.doesNotMatch(html, /href="\/account-settings"/);
    assert.doesNotMatch(html, /data-ribbon-action="signOut"/);
  }
});

test("settled Instructor Tier 2 destinations and order stay fixed across deeper routes", () => {
  const groups = {
    courses: "instructorCourses",
    questions: "instructorQuestions",
    productAssessments: "instructorAssessments",
  };
  const rowsByTierOne = new Map();
  for (const route of ROUTE_CONTRACT) {
    const group = groups[route.ribbon.tierOneArea];
    if (group === undefined) continue;
    const model = controlsFor(route.id, "instructor").model;
    const row = model.taskAreas.map((area) => ({
      id: area.id,
      label: area.label,
      controls: area.controls.map(({ id, label, destination }) => ({ id, label, destination })),
    }));
    const controls = ribbonTierTwoSchemaFor("instructor", route.ribbon.tierOneArea).map((slot) => {
      assert.equal(slot.kind, "destination");
      return RIBBON_TASK_CATALOG.find((control) => control.id === slot.id);
    });
    assert.ok(controls.every((control) => control !== undefined));
    const expected =
      controls.length === 0
        ? []
        : [
            {
              id: controls[0].area,
              label: model.taskAreas[0]?.label,
              controls: controls.map(({ id, label, destination }) => ({ id, label, destination })),
            },
          ];
    assert.deepEqual(row, expected, `${route.id} has the complete ${route.ribbon.tierOneArea} row`);
    const selected = model.taskAreas.flatMap((area) =>
      area.controls.filter((control) => control.selected),
    );
    assert.ok(selected.length <= 1, `${route.id} selects at most one Tier 2 destination`);
    const rowControls = model.taskAreas.flatMap((area) => area.controls);
    const exactDestination = rowControls.some(
      (control) => control.destination.kind === "route" && control.destination.routeId === route.id,
    );
    if (exactDestination) {
      assert.equal(selected.length, 1, `${route.id} keeps its exact Tier 2 destination`);
      assert.equal(selected[0].destination.routeId, route.id, route.id);
    } else if (route.ribbon.tierTwoParent !== undefined) {
      assert.deepEqual(
        selected.map((control) => control.id),
        [route.ribbon.tierTwoParent],
        `${route.id} keeps its declared Tier 2 parent`,
      );
    } else {
      assert.deepEqual(selected, [], `${route.id} has no Tier 2 ancestor without a parent`);
    }
    const previous = rowsByTierOne.get(route.ribbon.tierOneArea);
    if (previous === undefined) rowsByTierOne.set(route.ribbon.tierOneArea, row);
    else assert.deepEqual(row, previous, `${route.ribbon.tierOneArea} row changed at ${route.id}`);
  }
});

test("Task Row topology does not report task-control admission", () => {
  const routeId = "assessmentWorkspaceOverview";
  const before = controlsFor(routeId, "instructor").model.taskAreas;
  const taskEntries = RIBBON_TASK_CATALOG.map((control) => CAPABILITY_REGISTRY[control.id]);
  const descriptors = taskEntries.map((entry) => [entry, Object.getOwnPropertyDescriptors(entry)]);
  try {
    for (const entry of taskEntries) {
      Object.assign(entry, {
        capability: {
          kind: "unbacked",
          reason: "Test-only all-task admission withdrawal.",
          evidence: ["tests/test_ribbon_contract.mjs"],
        },
      });
    }
    const unavailable = controlsFor(routeId, "instructor").model.taskAreas;
    assert.deepEqual(
      unavailable.map((area) => ({ id: area.id, count: area.controls.length })),
      before.map((area) => ({ id: area.id, count: area.controls.length })),
      "all unavailable task controls retain their route-declared areas and counts",
    );
    assert.equal(
      unavailable
        .flatMap((area) => area.controls)
        .every((control) => control.availability === "Unavailable"),
      true,
      "the fixture actually withdraws every task control before comparing geometry input",
    );
  } finally {
    for (const [entry, entryDescriptors] of descriptors) {
      restoreDescriptors(entry, entryDescriptors);
    }
  }
});

test("Instructor Product routes reserve owner-ordered task groups despite unavailable entries", () => {
  const courses = controlsFor("instructorHome", "instructor").model;
  const questions = controlsFor("library", "instructor").model;
  const assessments = controlsFor("assessmentsDueSoon", "instructor").model;
  assert.deepEqual(
    courses.tabs.map((control) => control.label),
    ["Courses", "Questions", "Assessments"],
  );
  assert.deepEqual(
    courses.taskAreas.flatMap((area) => area.controls).map((control) => control.label),
    [
      "My Blueprint Courses",
      "My Active Courses",
      "My Inactive Courses",
      "Search Public Blueprint Courses",
    ],
  );
  assert.deepEqual(
    questions.taskAreas.flatMap((area) => area.controls).map((control) => control.label),
    [
      "My Questions",
      "My Draft Questions",
      "Starred",
      "Watched",
      "Search Question Library",
      "Browse Question Library",
    ],
  );
  assert.deepEqual(
    assessments.taskAreas.flatMap((area) => area.controls).map((control) => control.label),
    ["Assessments Due Soon", "My Assessment Templates"],
  );
  for (const control of courses.taskAreas.flatMap((area) => area.controls)) {
    if (control.availability === "Unavailable") assert.equal(control.href, undefined, control.id);
  }
  for (const control of questions.taskAreas.flatMap((area) => area.controls)) {
    if (control.availability === "Unavailable") assert.equal(control.href, undefined, control.id);
  }
});

test("frequent Instructor teaching tasks stay linked while future tasks stay unusable", () => {
  const courses = controlsFor("instructorHome", "instructor").model;
  const questions = controlsFor("library", "instructor").model;
  const assessments = controlsFor("assessmentsDueSoon", "instructor").model;
  assert.deepEqual(
    courses.tabs.map((control) => control.label),
    ["Courses", "Questions", "Assessments"],
  );
  const linked = [courses, questions, assessments].flatMap((model) =>
    model.taskAreas.flatMap((area) => area.controls),
  );
  for (const control of linked) {
    assert.equal(control.availability, "Available", control.id);
    assert.match(control.href ?? "", /^\//, control.id);
  }
  const futureIds = ["teachingOperations", "blueprintUpdates", "courseSetup", "gradeSettings"];
  for (const futureId of futureIds) {
    const entry = RIBBON_TASK_CATALOG.find((candidate) => candidate.id === futureId);
    assert.ok(entry, futureId);
    assert.equal(entry.destination.kind, "future", futureId);
    assert.equal(
      linked.some((control) => control.id === futureId && control.href !== undefined),
      false,
      futureId,
    );
  }
});

test("Ribbon has one plain brand anchor rather than a separate product-name treatment", async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const html = renderToString(() =>
    createComponent(RealAppRibbon, { model: M6_RIBBON_FIXTURES.courseInstructor }),
  );
  const brand = html.match(/<a[^>]*class="ple-app-ribbon__brand"[^>]*>([\s\S]*?)<\/a>/)?.[0];
  assert.ok(brand, "the Context Row renders a plain brand anchor");
  assert.match(brand, /href="\/"/);
  assert.match(brand, /aria-label="Peptidyle home"/);
  assert.doesNotMatch(brand, /data-ribbon-(?:control|pending)=/);
  assert.equal(
    (html.match(/ple-app-ribbon__brand-word/g) ?? []).length,
    1,
    "exactly one element in the Ribbon carries the product wordmark",
  );
  assert.doesNotMatch(html, /ple-app-ribbon__product-name/);
});

test("top bar order is logo, role badge, Tier 1, then Light/Dark and Profile", async () => {
  const RealAppRibbon = await loadAppRibbonForSsr();
  const html = renderToString(() =>
    createComponent(RealAppRibbon, {
      model: M6_RIBBON_FIXTURES.courseInstructor,
      displayMode: "light",
      onSwitchDisplayMode: () => undefined,
    }),
  );
  const brand = html.indexOf("ple-app-ribbon__brand");
  const role = html.indexOf("ple-app-ribbon__user-role");
  const tabs = html.indexOf('aria-label="Ribbon tabs"');
  const mode = html.indexOf("ple-app-ribbon__display-mode-toggle");
  const profile = html.indexOf('data-ribbon-context-control="profile"');
  assert.ok(brand >= 0 && role > brand, "logo precedes the role badge");
  assert.ok(tabs > role, "Tier 1 follows the role badge");
  assert.ok(mode > tabs && profile > mode, "Light/Dark precedes the Profile control");
  const phone = readFileSync(new URL("../src/ribbon/app_ribbon.css", import.meta.url), "utf8");
  const phoneRule = phone.slice(phone.indexOf("@media (max-width: 40rem)"));
  assert.doesNotMatch(phoneRule, /\.ple-app-ribbon__user-role\s*\{[^}]*order:/s);
  assert.match(phoneRule, /\.ple-app-ribbon__brand-word\s*\{[^}]*display:\s*none;/s);
});

test("Student Tier 2 choices and order stay fixed across routes and Course context", () => {
  const expectedTaskRows = {
    courses: ["studentCourse:CI7K3M2QAZ"],
    coursework: ["allCoursework", "dueSoon", "completedCoursework", "activeAttempt"],
    grades: [
      "studentScores",
      "studentResponseStats",
      "studentAttemptHistory",
      "studentLatestFeedback",
    ],
  };
  for (const route of ROUTE_CONTRACT) {
    if (!route.requiredUserRoles.includes("student")) continue;
    const expected = expectedTaskRows[route.ribbon.tierOneArea];
    if (expected === undefined) continue;
    const model = controlsFor(route.id, "student").model;
    const row = model.taskAreas.flatMap((area) => area.controls.map((control) => control.id));
    assert.deepEqual(row, expected, `${route.id} retains its fixed Student Tier 2 row`);
    for (const control of model.taskAreas.flatMap((area) => area.controls)) {
      if (control.id.startsWith("studentCourse:")) {
        assert.equal(control.href, "/student/courses/CI7K3M2QAZ", control.id);
      } else if (control.id === "activeAttempt" || control.id === "studentLatestFeedback") {
        assert.equal(control.href, undefined, control.id);
      } else {
        assert.match(control.href ?? "", /^\/student(?:\/|$)/, control.id);
      }
    }
  }
  assert.equal(
    routeStateFor("assessmentAttemptSummary").route.ribbon.tierOneArea,
    "grades",
    "submitted Attempt results keep the Grades navigation context",
  );
});

test("Student Coursework keeps its collective Tier 1 label without an Attempt-only row", () => {
  const courseLanding = controlsFor("studentCourseLanding", "student").model;
  const attempt = controlsFor("assessmentAttempt", "student", {
    courseShortName: "BCHM 355",
    courseLongName: "Biochemistry 301",
  }).model;
  const courseList = controlsFor("studentCourses", "student").model;
  const courseControl = (model) =>
    model.taskAreas
      .flatMap((area) => area.controls)
      .find((control) => control.id.startsWith("studentCourse:"));
  assert.equal(courseControl(courseLanding)?.selected, true);
  assert.equal(courseControl(courseList)?.selected, false);
  assert.equal(
    attempt.breadcrumbs.some((item) => item.label === "Biochemistry 301"),
    true,
    "an Attempt breadcrumb identifies its explicit Course context",
  );
  const attemptTaskControls = attempt.taskAreas.flatMap((area) => area.controls);
  assert.deepEqual(
    courseLanding.tabs.map(({ id, label, href }) => ({ id, label, href })),
    [
      { id: "coursework", label: "Coursework", href: "/student" },
      { id: "grades", label: "Grades", href: "/student/grades" },
      { id: "courses", label: "Courses", href: "/student/courses" },
    ],
  );
  assert.equal(courseLanding.tabs.find((tab) => tab.id === "courses")?.selected, true);
  assert.deepEqual(
    attemptTaskControls.map((control) => control.label),
    ["All Coursework", "Due Soon", "Completed", "Active Attempt"],
  );
  assert.equal(
    attemptTaskControls.at(-1)?.availability,
    "Unavailable",
    "the fixed shortcut stays disabled when no resumable Attempt is reported",
  );
  const unlabeledAttempt = controlsFor("assessmentAttempt", "student").model;
  const studentLabels = [
    ...unlabeledAttempt.tabs.map((tab) => tab.label),
    ...unlabeledAttempt.taskAreas.flatMap((area) => area.controls.map((control) => control.label)),
    ...unlabeledAttempt.breadcrumbs.map((item) => item.label),
  ];
  assert.equal(
    studentLabels.some((label) => label.includes("Assessment")),
    false,
    "Student Ribbon labels use Coursework language",
  );
  assert.equal(
    unlabeledAttempt.breadcrumbs.some((item) => item.label === "Coursework"),
    true,
    "a Student Attempt breadcrumb falls back to Coursework",
  );
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
  assert.deepEqual(
    controls.map((control) => control.id),
    ["allCoursework", "dueSoon", "completedCoursework", "activeAttempt"],
  );
  assert.deepEqual(
    controls.map((control) => control.href),
    ["/student", "/student/due-soon", "/student/completed", undefined],
  );
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

test("relationship admission may check without moving schema-owned positions", () => {
  const entry = CAPABILITY_REGISTRY.myActiveCourses;
  const descriptors = Object.getOwnPropertyDescriptors(entry);
  const before = controlsFor("courseAssessments", "instructor").controls.map(({ id }) => id);
  try {
    Object.assign(entry, {
      relationshipRequirement: "grader",
      capability: {
        kind: "backed",
        clientMethod: "TestApi.read",
        serverEvidence: { kind: "noServerCall", justification: "Test-only admission boundary." },
        evidence: ["tests/test_ribbon_contract.mjs"],
      },
    });
    const controls = controlsFor("courseAssessments", "instructor").controls;
    assert.deepEqual(
      controls.map(({ id }) => id),
      before,
    );
    assert.equal(controls.find((control) => control.id === entry.id)?.availability, "Checking");
  } finally {
    restoreDescriptors(entry, descriptors);
  }
});

test("deferred assessment-attempt summary keeps Attempt History and its review crumb", () => {
  const state = routeStateFor("assessmentAttemptSummary");
  assert.deepEqual(Object.keys(state.params), ["courseInstanceId", "assessmentAttemptId"]);
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
  assert.equal(
    model.breadcrumbs.some(({ label }) => label.includes(PARAMETER_VALUES.assessmentAttemptRef)),
    false,
  );
});

const STUDENT_PAGE_COMPONENTS = {
  assessmentAttempt: ["AssessmentAttemptPage", "src/pages/assessment_attempt_page.tsx"],
  assessmentAttemptSummary: [
    "AssessmentAttemptSummaryPage",
    "src/pages/assessment_attempt_summary_page.tsx",
  ],
  assessmentOverview: ["AssessmentOverviewPage", "src/pages/assessment_overview_page.tsx"],
  studentAttemptHistory: [
    "StudentAttemptHistoryPage",
    "src/pages/student_course_attempt_history_page.tsx",
  ],
  studentCompleted: ["StudentCompletedPage", "src/pages/student_course_landing_page.tsx"],
  studentCourseInvitation: [
    "StudentCourseInvitationPage",
    "src/pages/student_course_invitation_page.tsx",
  ],
  studentCourseInvitations: [
    "StudentCourseInvitationsPage",
    "src/pages/student_course_invitations_page.tsx",
  ],
  studentCourseLanding: ["StudentCourseLandingPage", "src/pages/student_course_landing_page.tsx"],
  studentCourseProgress: [
    "StudentCourseProgressPage",
    "src/pages/student_course_progress_page.tsx",
  ],
  studentCourses: ["StudentCoursesPage", "src/pages/student_courses_page.tsx"],
  studentDueSoon: ["StudentDueSoonPage", "src/pages/student_course_landing_page.tsx"],
  studentHome: ["StudentHomePage", "src/pages/role_home_pages.tsx"],
  studentResponseStats: [
    "StudentResponseStatsPage",
    "src/pages/student_course_response_stats_page.tsx",
  ],
  studentScores: ["StudentScoresPage", "src/pages/student_course_grades_page.tsx"],
};

test("Student navigation and pages should contain only Student interfaces and capabilities.", () => {
  const repositoryRoot = new URL("../", import.meta.url);
  const routesSource = readFileSync(new URL("src/routes.ts", repositoryRoot), "utf8");
  const boundarySource = readFileSync(
    new URL("src/route_access_boundary.tsx", repositoryRoot),
    "utf8",
  );
  assert.match(
    boundarySource,
    /return userRoleMayAccessRoute\(route\.id, state\.session\.account\.userRole\)/,
  );
  assert.match(boundarySource, /This page is not available to this account/);
  assert.match(routesSource, /withRouteAccessBoundary\(route, routeComponents\[route\.id\]\)/);
  const studentRoutes = ROUTE_CONTRACT.filter(
    (route) => route.requiredUserRoles.length === 1 && route.requiredUserRoles[0] === "student",
  );
  assert.deepEqual(
    studentRoutes.map((route) => route.id).sort(),
    Object.keys(STUDENT_PAGE_COMPONENTS).sort(),
  );
  const instructorCapability =
    /Unrelease|Create Question Pool|Instructor Accounts|type="file"|Gradebook|Question Library|Deactivate Instructor/u;
  for (const route of studentRoutes) {
    assert.equal(userRoleMayAccessRoute(route.id, "student"), true, route.id);
    assert.equal(userRoleMayAccessRoute(route.id, "instructor"), false, route.id);
    assert.equal(userRoleMayAccessRoute(route.id, "sysadmin"), false, route.id);
    const [componentName, componentPath] = STUDENT_PAGE_COMPONENTS[route.id];
    assert.match(routesSource, new RegExp(`^  ${route.id}: ${componentName},$`, "m"), route.id);
    const importPath = componentPath.replace(/^src\//, "./").replace(/\.tsx$/, "");
    assert.match(
      routesSource,
      new RegExp(`\\b${componentName}\\b[\\s\\S]*?${importPath.replaceAll(".", "\\.")}`),
      route.id,
    );
    const pageSource = readFileSync(new URL(componentPath, repositoryRoot), "utf8");
    if (componentName === "StudentHomePage") {
      assert.match(
        pageSource,
        /export function StudentHomePage\(\): JSX\.Element \{\s*return <StudentAllCourseworkPage \/>;\s*\}/,
      );
    } else {
      assert.equal(instructorCapability.test(pageSource), false, componentPath);
    }
    const state = routeStateFor(route.id);
    const model = deriveRibbonModel(
      {
        ...state,
        params: {
          ...state.params,
          courseInstanceId: PARAMETER_VALUES.courseInstanceId,
          assessmentId: PARAMETER_VALUES.assessmentId,
        },
        studentCourses: [{ id: PARAMETER_VALUES.courseInstanceId, shortName: "BCHM 355" }],
        activeAttemptId: PARAMETER_VALUES.assessmentAttemptId,
        latestFeedbackAttemptId: PARAMETER_VALUES.assessmentAttemptId,
      },
      { userRole: "student" },
      { assessmentAttemptTitle: "Weekly enzyme kinetics", courseShortName: "BCHM 355" },
    );
    const controls = [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)];
    for (const control of controls) {
      assert.equal(ribbonModelAvailabilityMayAccessRoute(control, "student"), true, control.id);
      assert.equal(control.label.includes("Question Library"), false, control.id);
      assert.equal(control.label.includes("Instructor Accounts"), false, control.id);
      assert.equal(control.label.includes("Gradebook"), false, control.id);
    }
    const hrefs = [
      ...controls.map((control) => control.href),
      ...model.breadcrumbs.map((item) => item.href),
      ...model.context.accountControls.map((control) => control.href),
    ].filter((href) => href !== undefined);
    for (const href of hrefs) {
      const destination = routeContractForPathname(href);
      assert.ok(destination, `${route.id} ${href}`);
      const studentOnly =
        destination.requiredUserRoles.length === 1 &&
        destination.requiredUserRoles[0] === "student";
      assert.equal(studentOnly || destination.id === "profile", true, `${route.id} ${href}`);
    }
  }
  assert.equal(profileRoleMayManageImage("student"), false);
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

  const instructorHome = trail("instructorHome", "instructor", {});
  assert.deepEqual(
    instructorHome.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "My Active Courses"],
  );
  assert.deepEqual(
    instructorHome.breadcrumbs.map((item) => item.href),
    ["/instructor", "/instructor", "/instructor"],
  );
  assert.deepEqual(selectedTaskIds(instructorHome), ["myActiveCourses"]);

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

  const blueprint = trail("blueprintCourseDetail", "instructor", {
    ...labels,
    blueprintBreadcrumbParent: "myBlueprintCourses",
  });
  assert.deepEqual(
    blueprint.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "My Blueprint Courses", "Molecular Biology Blueprint"],
  );
  assert.deepEqual(selectedTaskIds(blueprint), ["myBlueprintCourses"]);

  const unknownBlueprint = trail("blueprintCourseDetail", "instructor", labels);
  assert.deepEqual(selectedTaskIds(unknownBlueprint), []);
  assert.deepEqual(
    unknownBlueprint.breadcrumbs.map((item) => item.label),
    ["Home", "Courses", "Molecular Biology Blueprint"],
  );

  const question = trail("questionDetail", "instructor", labels);
  assert.deepEqual(
    question.breadcrumbs.map((item) => item.label),
    ["Home", "Questions", "Search Question Library", "Catalytic triad"],
  );
  assert.deepEqual(selectedTaskIds(question), ["searchQuestionLibrary"]);

  const library = trail("library", "instructor", {});
  const searchCrumb = library.breadcrumbs.find((item) => item.label === "Search Question Library");
  const pageCrumb = library.breadcrumbs.find((item) => item.label === "Question Library");
  assert.equal(searchCrumb?.href, "/library");
  assert.equal(pageCrumb?.href, searchCrumb?.href);
  assert.notEqual(searchCrumb?.label, pageCrumb?.label);

  const workspace = trail("assessmentWorkspacePolicies", "instructor", labels);
  assert.deepEqual(
    workspace.breadcrumbs.map((item) => item.label),
    ["Home", "Assessments", "Biochemistry I", "Problem Set 7", "Properties"],
  );
  assert.deepEqual(selectedTaskIds(workspace), []);
  assert.equal(workspace.tabs.find((tab) => tab.selected)?.id, "productAssessments");

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

  const profile = trail("profile", "instructor", {});
  assert.deepEqual(
    profile.breadcrumbs.map((item) => item.label),
    ["Home", "Profile settings"],
  );
  assert.equal(
    profile.tabs.some((tab) => tab.selected),
    false,
  );

  const inspection = trail("sysadminCourseInspection", "sysadmin", {});
  assert.deepEqual(
    inspection.breadcrumbs.map((item) => item.label),
    ["Home", "Courses"],
  );
  assert.equal(inspection.breadcrumbs.at(-1)?.href, "/sysadmin/courses");

  const malformed = ROUTE_CONTRACT.find((route) => route.id === "courseRoster");
  const invalid = deriveRibbonModel(
    { route: malformed, params: { courseInstanceId: "not-a-course" } },
    { userRole: "instructor" },
    { ...labels, courseLifecycleState: "active" },
  );
  assert.deepEqual(invalid.breadcrumbs, [{ label: "Home", href: "/instructor", current: false }]);
});

test("every signed-in route reserves linked breadcrumbs rooted at its role home", () => {
  for (const userRole of USER_ROLES) {
    for (const route of ROUTE_CONTRACT) {
      if (route.id === "signIn" || !userRoleMayAccessRoute(route.id, userRole)) continue;
      const state = routeStateFor(route.id);
      const model = deriveRibbonModel(
        {
          ...state,
          params: {
            ...state.params,
            courseInstanceId: PARAMETER_VALUES.courseInstanceId,
            assessmentId: PARAMETER_VALUES.assessmentId,
          },
        },
        { userRole },
        {
          courseLongName: "Biochemistry I",
          assessmentTitle: "Problem Set 7",
          assessmentAttemptTitle: "Problem Set 7",
        },
      );
      assert.equal(model.breadcrumbs[0].href, userRoleHomePath(userRole), route.id);
      assert.equal(model.breadcrumbs.filter(({ current }) => current).length, 1, route.id);
      assert.equal(
        model.breadcrumbs.at(-1).href,
        route.id === "courses"
          ? userRoleHomePath(userRole)
          : buildRoutePath(route.id, paramsForRoute(route)),
        route.id,
      );
      for (const breadcrumb of model.breadcrumbs) {
        assert.ok(routeContractForPathname(breadcrumb.href), route.id);
        assert.ok(breadcrumb.label.trim(), route.id);
        assert.equal(
          Object.values(PARAMETER_VALUES).some((value) => breadcrumb.label.includes(value)),
          false,
          route.id,
        );
      }
    }
  }
});
