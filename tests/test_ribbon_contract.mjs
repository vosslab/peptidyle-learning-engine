import assert from "node:assert/strict";
import test from "node:test";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import { buildRoutePath, deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { routeParams } from "../src/navigation/route_params.ts";
import {
  productRoleMayAccessRoute,
  productRoleHomePath,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { CAPABILITY_REGISTRY } from "../src/ribbon/capability_registry.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { ribbonTierTwoSchemaFor } from "../src/ribbon/ribbon_schema.ts";
import { loadAppRibbonForSsr } from "./support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "./support/ribbon_model_fixtures.ts";

const PRODUCT_ROLES = ["student", "instructor", "sysadmin"];
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
  const declaredParams = routeParams(route, pathname);
  assert.ok(declaredParams, `route ${route.id} must extract`);
  return { route, params: { ...declaredParams } };
}

function controlsFor(routeId, productRole, labels = LABELS) {
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
      ...(productRole === "student"
        ? { studentCourses: [{ id: PARAMETER_VALUES.courseInstanceId, shortName: "BCHM 355" }] }
        : {}),
    },
    { productRole },
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
    for (const role of PRODUCT_ROLES) {
      for (const control of controlsFor(route.id, role).controls) {
        if (control.availability === "Unavailable") {
          assert.equal(control.href, undefined, `${route.id}/${role}/${control.id}`);
        }
        if (control.availability === "Available" && control.destination.kind === "route") {
          assert.equal(
            productRoleMayAccessRoute(control.destination.routeId, role),
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
test("every signed-in Product Role has one accessible generic Profile end control", async () => {
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
          label: "Profile",
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
    for (const control of selected) {
      assert.equal(
        control.destination.kind === "route" && control.destination.routeId === route.id,
        true,
        `${route.id} selects only its own destination`,
      );
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
    if (!route.requiredProductRoles.includes("student")) continue;
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
});

test("global Student Tier 2 destinations stay available without Course context", () => {
  const routeState = routeStateFor("assessmentAttempt");
  const params = { assessmentAttemptId: routeState.params.assessmentAttemptId };
  const model = deriveRibbonModel(
    { route: routeState.route, params },
    { productRole: "student" },
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

test("Active Attempt links directly to the server-selected resumable Attempt", () => {
  const routeState = routeStateFor("studentHome");
  const attemptId = PARAMETER_VALUES.assessmentAttemptId;
  const model = deriveRibbonModel(
    {
      ...routeState,
      activeAttemptId: attemptId,
    },
    { productRole: "student" },
    LABELS,
  );
  const activeAttempt = model.taskAreas
    .flatMap((area) => area.controls)
    .find((control) => control.id === "activeAttempt");
  assert.ok(activeAttempt);
  assert.equal(activeAttempt.availability, "Available");
  assert.equal(
    activeAttempt.href,
    buildRoutePath("assessmentAttempt", { assessmentAttemptId: attemptId }),
  );
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

test("breadcrumb trails are canonical route projections with one current terminal", () => {
  const labels = {
    courseShortName: "BCHM 355",
    courseLongName: "Biochemistry I",
    assessmentTitle: "Problem Set 7",
    assessmentAttemptTitle: "Problem Set 7",
  };
  const cases = [
    ["courses", ["Home"]],
    ["courseAssessments", ["Home", "Biochemistry I"]],
    ["courseAppearance", ["Home", "Biochemistry I", "Appearance"]],
    ["assessmentWorkspaceQuestions", ["Home", "Biochemistry I", "Problem Set 7", "Questions"]],
    ["assessmentWorkspacePolicies", ["Home", "Biochemistry I", "Problem Set 7", "Properties"]],
    ["questionDetail", ["Home", "Question Library", "Question"]],
    ["questionDraftEditor", ["Home", "My Draft Questions", "Question"]],
    ["blueprintCourseDetail", ["Home", "My Blueprint Courses", "Blueprint Course"]],
    ["publicBlueprintSearch", ["Home", "Search Public Blueprint Courses"]],
    ["assessmentAttempt", ["Home", "Biochemistry I", "Problem Set 7", "Attempt"]],
    ["assessmentAttemptSummary", ["Home", "Biochemistry I", "Problem Set 7", "Attempt history"]],
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
      { productRole: "instructor" },
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
    { productRole: "instructor" },
    labels,
  ).breadcrumbs[1];
  assert.deepEqual(
    [courseBreadcrumb?.label, courseBreadcrumb?.compactLabel],
    ["Biochemistry I", "BCHM 355"],
    "the breadcrumb model keeps descriptive and compact Course identities together",
  );
  const malformed = ROUTE_CONTRACT.find((route) => route.id === "courseAppearance");
  assert.ok(malformed);
  const invalid = deriveRibbonModel(
    { route: malformed, params: { courseInstanceId: "C-1/not-an-id" } },
    { productRole: "instructor" },
    labels,
  );
  assert.deepEqual(
    invalid.breadcrumbs,
    [{ label: "Home", href: "/instructor", current: false }],
    "malformed scope retains only the known home without identifier copy",
  );
  const scoped = deriveRibbonModel(
    routeStateFor("courseAppearance"),
    { productRole: "instructor" },
    labels,
  );
  assert.deepEqual(Object.keys(scoped.context).sort(), [
    "accountControls",
    "productLabel",
    "signOutAction",
  ]);
  assert.equal(Object.hasOwn(scoped, "contentLayout"), false);
  assert.equal(scoped.breadcrumbs[1]?.label, "Biochemistry I");
});

test("Student Course Invitation acceptance stays outside Course breadcrumb context", () => {
  const model = deriveRibbonModel(
    routeStateFor("studentCourseInvitation"),
    { productRole: "student" },
    LABELS,
  );
  assert.deepEqual(
    model.breadcrumbs.map(({ label }) => label),
    ["Home", "Course Invitations", "Course Invitation"],
  );
});

test("deferred scope labels retain a linked human-readable current breadcrumb", () => {
  const cases = [
    ["courseAssessments", "instructor", ["Home", "Course"]],
    ["assessmentWorkspaceOverview", "instructor", ["Home", "Course", "Assessment"]],
    ["assessmentAttempt", "student", ["Home", "Attempt"]],
  ];
  for (const [routeId, productRole, expectedLabels] of cases) {
    const state = routeStateFor(routeId);
    const model = deriveRibbonModel(state, { productRole }, LABELS);
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
    assert.equal(model.breadcrumbs.at(-1).href, buildRoutePath(routeId, state.params), routeId);
    assert.equal(
      model.breadcrumbs.some(({ label }) => label.includes(PARAMETER_VALUES.assessmentAttemptRef)),
      false,
      routeId,
    );
  }
});

test("deferred assessment-attempt summary retains only its linked current breadcrumb", () => {
  const state = routeStateFor("assessmentAttemptSummary");
  assert.deepEqual(Object.keys(state.params), ["assessmentAttemptId"]);
  const model = deriveRibbonModel(state, { productRole: "student" }, LABELS);
  assert.deepEqual(model.breadcrumbs, [
    { label: "Home", href: "/student", current: false },
    {
      label: "Attempt history",
      href: "/assessment-attempts/00000000-0000-0000-0000-000000000001/summary",
      current: true,
    },
  ]);
  assert.equal(
    model.breadcrumbs.some(({ label }) => label.includes(PARAMETER_VALUES.assessmentAttemptRef)),
    false,
  );
});

test("Course breadcrumbs preserve the authored long name", () => {
  const state = routeStateFor("courseAssessments");
  for (const courseLongName of [
    "Course CI7K3M2QAZ: Molecular Biology",
    "CI7K3M2QAZ: Molecular Biology",
  ]) {
    const model = deriveRibbonModel(state, { productRole: "instructor" }, { courseLongName });
    assert.deepEqual(
      model.breadcrumbs.map(({ label }) => label),
      ["Home", courseLongName],
    );
  }
});

test("every signed-in route reserves linked breadcrumbs rooted at its role home", () => {
  for (const productRole of PRODUCT_ROLES) {
    for (const route of ROUTE_CONTRACT) {
      if (route.id === "signIn" || !productRoleMayAccessRoute(route.id, productRole)) continue;
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
        { productRole },
        {
          courseLongName: "Biochemistry I",
          assessmentTitle: "Problem Set 7",
          assessmentAttemptTitle: "Problem Set 7",
        },
      );
      assert.equal(model.breadcrumbs[0].href, productRoleHomePath(productRole), route.id);
      assert.equal(model.breadcrumbs.filter(({ current }) => current).length, 1, route.id);
      assert.equal(
        model.breadcrumbs.at(-1).href,
        route.id === "courses"
          ? productRoleHomePath(productRole)
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
