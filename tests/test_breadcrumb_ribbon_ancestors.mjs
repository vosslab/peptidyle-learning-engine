// test_breadcrumb_ribbon_ancestors.mjs - selected Ribbon ancestors stay in breadcrumb trails.

import assert from "node:assert/strict";
import test from "node:test";

import { buildRoutePath } from "../src/ribbon/ribbon_route_path.ts";
import { deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { routeParams } from "../src/navigation/route_params.ts";
import { ROUTE_CONTRACT, routeContractForPathname } from "../src/route_contract.ts";

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

function labelsFor(routeId) {
  const model = deriveRibbonModel(
    routeStateFor(routeId),
    { userRole: "instructor" },
    {
      courseShortName: "BCHM 355",
      courseLongName: "Biochemistry I",
      assessmentTitle: "Problem Set 7",
      questionTitle: "Catalytic triad",
    },
  );
  return model.breadcrumbs.map((item) => item.label);
}

test("descendant pages keep the selected Ribbon tier and collapse only identical names", () => {
  assert.deepEqual(labelsFor("courseAppearance"), [
    "Home",
    "Courses",
    "Biochemistry I",
    "Appearance",
  ]);
  assert.deepEqual(labelsFor("assessmentWorkspaceQuestions"), [
    "Home",
    "Assessments",
    "Biochemistry I",
    "Problem Set 7",
    "Questions",
  ]);
  assert.deepEqual(labelsFor("assessmentWorkspacePolicies"), [
    "Home",
    "Assessments",
    "Biochemistry I",
    "Problem Set 7",
    "Properties",
  ]);
  assert.deepEqual(labelsFor("questionDetail"), [
    "Home",
    "Questions",
    "Search Question Library",
    "Catalytic triad",
  ]);
  assert.deepEqual(labelsFor("questionDraftEditor"), [
    "Home",
    "Questions",
    "My Draft Questions",
    "Catalytic triad",
  ]);
  assert.deepEqual(labelsFor("library"), [
    "Home",
    "Questions",
    "Search Question Library",
    "Question Library",
  ]);
  const library = deriveRibbonModel(routeStateFor("library"), { userRole: "instructor" }, {});
  const search = library.breadcrumbs.find((item) => item.label === "Search Question Library");
  const page = library.breadcrumbs.find((item) => item.label === "Question Library");
  assert.equal(search?.href, page?.href);
  assert.notEqual(search?.label, page?.label);
});

test("breadcrumb trails are canonical route projections with one current terminal", () => {
  const labels = {
    courseShortName: "BCHM 355",
    courseLongName: "Biochemistry I",
    assessmentTitle: "Problem Set 7",
    assessmentAttemptTitle: "Problem Set 7",
    questionTitle: "Catalytic triad",
    blueprintCourseTitle: "Molecular Biology Blueprint",
    blueprintBreadcrumbParent: "myBlueprintCourses",
  };
  const cases = [
    ["courses", ["Home"]],
    ["courseAssessments", ["Home", "Courses", "Biochemistry I"]],
    ["courseAppearance", ["Home", "Courses", "Biochemistry I", "Appearance"]],
    ["questionDetail", ["Home", "Questions", "Search Question Library", "Catalytic triad"]],
    ["questionDraftEditor", ["Home", "Questions", "My Draft Questions", "Catalytic triad"]],
    [
      "blueprintCourseDetail",
      ["Home", "Courses", "My Blueprint Courses", "Molecular Biology Blueprint"],
    ],
    ["publicBlueprintSearch", ["Home", "Courses", "Search Public Blueprint Courses"]],
    [
      "assessmentAttempt",
      ["Home", "Coursework", "All Coursework", "Biochemistry I", "Problem Set 7", "Attempt"],
    ],
    [
      "assessmentAttemptSummary",
      ["Home", "Grades", "Attempt History", "Biochemistry I", "Problem Set 7", "Attempt history"],
    ],
  ];
  for (const [routeId, expectedLabels] of cases) {
    const routeState = routeStateFor(routeId);
    const model = deriveRibbonModel(
      routeId === "assessmentAttempt" || routeId === "assessmentAttemptSummary"
        ? {
            ...routeState,
            params: {
              ...routeState.params,
              courseInstanceId: "CI7K3M2QAZ",
              assessmentId: "A9D2RX5AF",
            },
          }
        : routeState,
      {
        userRole:
          routeId === "assessmentAttempt" || routeId === "assessmentAttemptSummary"
            ? "student"
            : "instructor",
      },
      labels,
    );
    assert.deepEqual(
      model.breadcrumbs.map((item) => item.label),
      expectedLabels,
      routeId,
    );
    assert.equal(
      model.breadcrumbs.filter((item) => item.current).length,
      expectedLabels.length === 0 ? 0 : 1,
      `${routeId} has exactly one current terminal`,
    );
    for (const item of model.breadcrumbs.filter((candidate) => candidate.href !== undefined)) {
      assert.ok(
        routeContractForPathname(item.href),
        `${routeId}:${item.label} has a declared href`,
      );
    }
  }
  const courseBreadcrumb = deriveRibbonModel(
    routeStateFor("assessmentWorkspaceQuestions"),
    { userRole: "instructor" },
    labels,
  ).breadcrumbs.find((item) => item.compactLabel === "BCHM 355");
  assert.deepEqual(
    [courseBreadcrumb?.label, courseBreadcrumb?.compactLabel],
    ["Biochemistry I", "BCHM 355"],
    "the breadcrumb model keeps descriptive and compact Course identities together",
  );
  const malformed = ROUTE_CONTRACT.find((route) => route.id === "courseAppearance");
  assert.ok(malformed);
  const invalid = deriveRibbonModel(
    { route: malformed, params: { courseInstanceId: "C-1/not-an-id" } },
    { userRole: "instructor" },
    labels,
  );
  assert.deepEqual(
    invalid.breadcrumbs,
    [{ label: "Home", href: "/instructor", current: false }],
    "malformed scope retains only the known home without identifier copy",
  );
  const scoped = deriveRibbonModel(
    routeStateFor("courseAppearance"),
    { userRole: "instructor" },
    labels,
  );
  assert.deepEqual(Object.keys(scoped.context).sort(), [
    "accountControls",
    "productLabel",
    "signOutAction",
  ]);
  assert.equal(Object.hasOwn(scoped, "contentLayout"), false);
  assert.equal(scoped.breadcrumbs[2]?.label, "Biochemistry I");
});

test("loaded Blueprint access selects its actual collection parent", () => {
  const routeState = routeStateFor("blueprintCourseDetail");
  const publicLabels = {
    blueprintCourseTitle: "Public Molecular Biology Blueprint",
    blueprintBreadcrumbParent: "publicBlueprintSearch",
  };
  const model = deriveRibbonModel(routeState, { userRole: "instructor" }, publicLabels);
  assert.deepEqual(
    model.breadcrumbs.map(({ label, href, current }) => ({ label, href, current })),
    [
      { label: "Home", href: "/instructor", current: false },
      { label: "Courses", href: "/instructor", current: false },
      {
        label: "Search Public Blueprint Courses",
        href: "/blueprint-courses/search/public",
        current: false,
      },
      {
        label: "Public Molecular Biology Blueprint",
        href: "/blueprint-courses/BP7K3M2QAF",
        current: true,
      },
    ],
  );
  for (const navigation of [{}]) {
    const directModel = deriveRibbonModel(
      routeState,
      { userRole: "instructor" },
      publicLabels,
      navigation,
    );
    assert.equal(
      directModel.breadcrumbs.find((item) => item.label === "Search Public Blueprint Courses")
        ?.href,
      "/blueprint-courses/search/public",
      "direct and malformed Blueprint Search returns use the bare collection route",
    );
  }
});

test("Student breadcrumb parents use their real collection and Course destinations", () => {
  const labels = { courseLongName: "Biochemistry I", assessmentTitle: "Problem Set 7" };
  const cases = [
    [
      "studentCourseLanding",
      ["Home", "Courses", "Biochemistry I"],
      ["/student", "/student/courses", "/student/courses/CI7K3M2QAZ"],
    ],
    [
      "studentCourseProgress",
      ["Home", "Courses", "Biochemistry I", "Progress"],
      [
        "/student",
        "/student/courses",
        "/student/courses/CI7K3M2QAZ",
        "/student/courses/CI7K3M2QAZ/progress",
      ],
    ],
    [
      "assessmentOverview",
      ["Home", "Coursework", "All Coursework", "Biochemistry I", "Problem Set 7"],
      [
        "/student",
        "/student",
        "/student",
        "/student/courses/CI7K3M2QAZ",
        "/courses/CI7K3M2QAZ/assessments/A9D2RX5AF",
      ],
    ],
    [
      "studentResponseStats",
      ["Home", "Grades", "Response Stats"],
      ["/student", "/student/grades", "/student/grades/response-stats"],
    ],
    [
      "studentAttemptHistory",
      ["Home", "Grades", "Attempt History"],
      ["/student", "/student/grades", "/student/grades/attempt-history"],
    ],
  ];
  for (const [routeId, expectedLabels, expectedHrefs] of cases) {
    const model = deriveRibbonModel(routeStateFor(routeId), { userRole: "student" }, labels);
    assert.deepEqual(
      model.breadcrumbs.map(({ label }) => label),
      expectedLabels,
      routeId,
    );
    assert.deepEqual(
      model.breadcrumbs.map(({ href }) => href),
      expectedHrefs,
      routeId,
    );
    assert.equal(model.breadcrumbs.at(-1)?.current, true, routeId);
  }
});

test("Student Course Invitation acceptance stays outside Course breadcrumb context", () => {
  const model = deriveRibbonModel(
    routeStateFor("studentCourseInvitation"),
    { userRole: "student" },
    LABELS,
  );
  assert.deepEqual(
    model.breadcrumbs.map(({ label }) => label),
    ["Home", "Course Invitations", "Course Invitation"],
  );
});

test("deferred scope labels retain a linked human-readable current breadcrumb", () => {
  const cases = [
    ["courseAssessments", "instructor", ["Home", "Courses", "Course"]],
    ["assessmentWorkspaceOverview", "instructor", ["Home", "Assessments", "Course", "Assessment"]],
    ["studentCourseLanding", "student", ["Home", "Courses", "Course"]],
    ["studentCourseProgress", "student", ["Home", "Courses", "Course", "Progress"]],
    [
      "assessmentOverview",
      "student",
      ["Home", "Coursework", "All Coursework", "Course", "Before you start"],
    ],
    ["studentResponseStats", "student", ["Home", "Grades", "Response Stats"]],
    ["assessmentAttempt", "student", ["Home", "Coursework", "All Coursework", "Attempt"]],
  ];
  for (const [routeId, userRole, expectedLabels] of cases) {
    const state = routeStateFor(routeId);
    const model = deriveRibbonModel(state, { userRole }, LABELS);
    assert.deepEqual(
      model.breadcrumbs.map(({ label }) => label),
      expectedLabels,
      routeId,
    );
    assert.deepEqual(
      model.breadcrumbs.map(({ current }) => current),
      expectedLabels.map((_, index) => index === expectedLabels.length - 1),
      routeId,
    );
    const breadcrumbParams =
      routeId === "assessmentAttempt"
        ? { courseInstanceId: state.params.courseInstanceId }
        : state.params;
    assert.equal(model.breadcrumbs.at(-1).href, buildRoutePath(routeId, breadcrumbParams), routeId);
    assert.equal(
      model.breadcrumbs.some(({ label }) => label.includes(PARAMETER_VALUES.assessmentAttemptRef)),
      false,
      routeId,
    );
  }
});

test("authored Course names survive even when they match a collection label", () => {
  const model = deriveRibbonModel(
    routeStateFor("assessmentOverview"),
    { userRole: "student" },
    {
      courseLongName: "Courses",
      assessmentTitle: "Problem Set 7",
    },
  );
  assert.deepEqual(
    model.breadcrumbs.map(({ label }) => label),
    ["Home", "Coursework", "All Coursework", "Courses", "Problem Set 7"],
  );
});

test("Attempt shortcut and current breadcrumbs preserve the selected Attempt state", () => {
  for (const [routeId, idKey, courseKey, label] of [
    ["assessmentAttempt", "activeAttemptId", "activeAttemptCourseInstanceId", "Active Attempt"],
    [
      "assessmentAttemptSummary",
      "latestFeedbackAttemptId",
      "latestFeedbackCourseInstanceId",
      "Latest Feedback",
    ],
  ]) {
    const routeState = routeStateFor(routeId);
    const attemptId = routeState.params.assessmentAttemptId;
    const model = deriveRibbonModel(
      { ...routeState, [idKey]: attemptId, [courseKey]: routeState.params.courseInstanceId },
      { userRole: "student" },
      {},
    );
    assert.deepEqual(model.breadcrumbs.find((item) => item.label === label)?.state, {
      assessmentAttemptId: attemptId,
    });
    assert.deepEqual(model.breadcrumbs.at(-1)?.state, { assessmentAttemptId: attemptId });
  }
});

test("Instructor Course descendants retain the lifecycle collection and its usable link", () => {
  for (const [lifecycle, parent, href, id] of [
    ["active", "My Active Courses", "/instructor", "myActiveCourses"],
    ["inactive", "My Inactive Courses", "/instructor/courses/inactive", "myInactiveCourses"],
  ]) {
    for (const [routeId, section] of [
      ["courseAssessments", []],
      ["courseRoster", ["Students"]],
      ["courseAppearance", ["Appearance"]],
    ]) {
      const state = routeStateFor(routeId);
      const labels = { courseLongName: "Biochemistry 301", courseLifecycleState: lifecycle };
      const model = deriveRibbonModel(state, { userRole: "instructor" }, labels);
      assert.deepEqual(
        model.breadcrumbs.map(({ label }) => label),
        ["Home", "Courses", parent, "Biochemistry 301", ...section],
        routeId,
      );
      assert.equal(model.breadcrumbs[2].href, href);
      assert.equal(model.breadcrumbs[1].href, "/instructor");
      assert.deepEqual(
        model.taskAreas
          .flatMap(({ controls }) => controls)
          .filter(({ selected }) => selected)
          .map(({ id }) => id),
        [id],
      );
      const loading = deriveRibbonModel(state, { userRole: "instructor" }, {});
      assert.equal(
        loading.breadcrumbs.some(({ label }) => label.startsWith("My ")),
        false,
      );
      assert.equal(
        loading.taskAreas.flatMap(({ controls }) => controls).some(({ selected }) => selected),
        false,
      );
    }
  }
});

test("Instructor Blueprint proposals retain the Blueprint collection above their own collection", () => {
  const model = deriveRibbonModel(
    routeStateFor("changeProposalDetail"),
    { userRole: "instructor" },
    {},
  );
  assert.deepEqual(
    model.breadcrumbs.map(({ label }) => label),
    ["Home", "Courses", "My Blueprint Courses", "My Change Proposals", "Change Proposal"],
  );
  assert.equal(model.breadcrumbs[2].href, "/blueprint-courses");
  assert.equal(model.breadcrumbs[3].href, "/blueprint-change-proposals");
});
