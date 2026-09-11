import assert from "node:assert/strict";
import test from "node:test";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import { buildRoutePath, deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { routeParams } from "../src/navigation/route_params.ts";
import {
  productRoleMayAccessRoute,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { CAPABILITY_REGISTRY } from "../src/ribbon/capability_registry.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { loadAppRibbonForSsr } from "./support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "./support/ribbon_model_fixtures.ts";

const PRODUCT_ROLES = ["student", "instructor", "sysadmin"];
const LABELS = {};
const PARAMETER_VALUES = {
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
  membershipRef: "M-1",
  questionRef: "7K3M9QP",
  draftQuestionRef: "D-1",
  blueprintCourseRef: "BP-1",
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
  const params = routeParams(route, pathname);
  assert.ok(params, `route ${route.id} must extract`);
  return { route, params };
}

function controlsFor(routeId, productRole) {
  const model = deriveRibbonModel(routeStateFor(routeId), { productRole }, LABELS);
  return { model, controls: [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)] };
}

function restoreDescriptors(target, descriptors) {
  for (const key of Reflect.ownKeys(target)) {
    if (!(key in descriptors)) Reflect.deleteProperty(target, key);
  }
  Object.defineProperties(target, descriptors);
}

test("declared routes build and extract a canonical public pathname", () => {
  for (const route of ROUTE_CONTRACT) {
    const pathname = buildRoutePath(route.id, paramsForRoute(route));
    assert.ok(pathname, route.id);
    const extracted = routeParams(route, pathname);
    assert.ok(extracted, route.id);
    assert.equal(buildRoutePath(route.id, extracted), pathname, route.id);
  }
});

test("route construction fails closed for incomplete, surplus, and malformed input", () => {
  assert.equal(buildRoutePath("unknown", {}), undefined);
  assert.equal(buildRoutePath("courseAssignments", {}), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1", extra: "x" }), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1/gradebook" }), undefined);
  assert.equal(buildRoutePath("questionDetail", { questionRef: "7K3%2FM9QP" }), undefined);
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

test("Instructor Ribbon has one Product Role plate and an accessible icon-only Profile end control", async () => {
  const model = controlsFor("courses", "instructor").model;
  assert.deepEqual(model.context.accountControls, [
    {
      id: "profile",
      label: "Profile",
      availability: "Available",
      glyph: "profile",
      href: "/profile",
    },
  ]);
  const RealAppRibbon = await loadAppRibbonForSsr();
  const html = renderToString(() => createComponent(RealAppRibbon, { model }));
  assert.match(html, /data-ribbon-context-control="profile"/);
  assert.equal(
    (html.match(/ple-app-ribbon__product-role/g) ?? []).length,
    1,
    "the Product Role is represented by one role plate",
  );
  const profile = html.match(
    /<a[^>]*data-ribbon-context-control="profile"[^>]*>[\s\S]*?<\/a>/,
  )?.[0];
  assert.ok(profile, "Instructor Ribbon has a Profile end control");
  assert.match(profile, /aria-label="Profile"/);
  assert.doesNotMatch(profile, />Profile</);
  assert.ok(
    html.indexOf('data-ribbon-action="signOut"') <
      html.indexOf('data-ribbon-context-control="profile"'),
    "Profile follows Sign out at the account end",
  );
  const profileRoute = ROUTE_CONTRACT.find((route) => route.id === "instructorProfile");
  assert.deepEqual(profileRoute?.ribbon, { scope: "product", contentLayout: "reading" });
});

test("Appearance admits only the Instructor Course Setup task and preserves its route", async () => {
  const instructor = controlsFor("courseAppearance", "instructor");
  const RealAppRibbon = await loadAppRibbonForSsr();
  const instructorHtml = renderToString(() =>
    createComponent(RealAppRibbon, { model: instructor.model }),
  );
  assert.match(
    instructorHtml,
    /href="\/instructor\/courses\/C-1\/appearance"[^>]*data-ribbon-control="appearance"/,
  );
  for (const role of ["student", "sysadmin"]) {
    const model = controlsFor("courseAppearance", role).model;
    const control = [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)].find(
      (candidate) => candidate.id === "appearance",
    );
    assert.equal(
      control?.availability,
      "Unavailable",
      `${role} cannot receive Appearance admission`,
    );
    const html = renderToString(() => createComponent(RealAppRibbon, { model }));
    assert.doesNotMatch(
      html,
      /data-ribbon-control="appearance"/,
      `${role} Ribbon omits Appearance`,
    );
  }
});

test("Task Row topology is exactly the declared task-group topology for every route", () => {
  for (const route of ROUTE_CONTRACT) {
    for (const role of PRODUCT_ROLES) {
      const model = deriveRibbonModel(routeStateFor(route.id), { productRole: role }, LABELS);
      const instructorProductTaskGroup = [
        "instructorCourses",
        "instructorQuestions",
        "instructorAssignments",
      ].includes(route.ribbon.taskGroup);
      assert.equal(
        model.taskAreas.length > 0,
        route.ribbon.taskGroup !== undefined &&
          (!instructorProductTaskGroup || role === "instructor"),
        `${route.id}/${role}: Task Row topology follows the route contract`,
      );
    }
  }
});

test("Task Row topology does not report task-control admission", () => {
  const routeId = "assignmentWorkspaceOverview";
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
  const courses = controlsFor("courses", "instructor").model;
  const questions = controlsFor("library", "instructor").model;
  assert.deepEqual(
    courses.tabs.map((control) => control.label),
    ["Courses", "Questions", "Assignments"],
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

test("Student Assignment Access and its Course landing retain the role-owned Assignments tab", () => {
  const assignmentAccess = controlsFor("assignmentOverview", "student").model;
  const courseLanding = controlsFor("studentCourseLanding", "student").model;
  const expectedTab = [
    {
      id: "studentAssignments",
      label: "Assignments",
      destination: { kind: "route", routeId: "studentCourseLanding" },
      availability: "Available",
      selected: true,
      href: "/student/courses/C-1",
      role: "primary",
      priority: "critical",
      presentation: "standard",
      iconBearing: true,
      iconOnlySafe: false,
    },
  ];
  assert.deepEqual(assignmentAccess.tabs, expectedTab);
  assert.deepEqual(courseLanding.tabs, expectedTab);
});

test("missing source parameters withhold a backed destination without changing its position", () => {
  const entry = CAPABILITY_REGISTRY.backToAssignments;
  const descriptors = Object.getOwnPropertyDescriptors(entry);
  const before = controlsFor("assignmentAttempt", "student").controls.map(({ id }) => id);
  try {
    Object.assign(entry, {
      relationshipRequirement: "none",
      capability: {
        kind: "backed",
        clientMethod: "TestApi.read",
        serverEvidence: { kind: "noServerCall", justification: "Test-only admission boundary." },
        evidence: ["tests/test_ribbon_contract.mjs"],
      },
    });
    const controls = controlsFor("assignmentAttempt", "student").controls;
    const back = controls.find((control) => control.id === entry.id);
    assert.deepEqual(
      controls.map(({ id }) => id),
      before,
    );
    assert.equal(back?.href, undefined);
  } finally {
    restoreDescriptors(entry, descriptors);
  }
});

test("relationship admission may check without moving schema-owned positions", () => {
  const entry = CAPABILITY_REGISTRY.assignments;
  const descriptors = Object.getOwnPropertyDescriptors(entry);
  const before = controlsFor("courseAssignments", "instructor").controls.map(({ id }) => id);
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
    const controls = controlsFor("courseAssignments", "instructor").controls;
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
    assignmentTitle: "Problem Set 7",
    assignmentAttemptTitle: "Problem Set 7",
  };
  const cases = [
    ["courses", []],
    ["courseAssignments", ["Courses", "Biochemistry I"]],
    ["courseAppearance", ["Courses", "Biochemistry I", "Appearance"]],
    ["assignmentWorkspaceQuestions", ["Courses", "Biochemistry I", "Problem Set 7", "Questions"]],
    ["questionDetail", ["Questions", "Question Library", "Question"]],
    ["questionDraftEditor", ["Questions", "My Draft Questions", "Draft Question"]],
    ["blueprintCourseDetail", ["Courses", "My Blueprint Courses", "Blueprint Course"]],
    ["assignmentAttempt", ["Courses", "Biochemistry I", "Problem Set 7", "Assignment attempt"]],
  ];
  for (const [routeId, expectedLabels] of cases) {
    const routeState = routeStateFor(routeId);
    const model = deriveRibbonModel(
      routeId === "assignmentAttempt"
        ? {
            ...routeState,
            params: { ...routeState.params, courseRef: "C-1", assignmentRef: "A-1" },
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
  const malformed = ROUTE_CONTRACT.find((route) => route.id === "courseAppearance");
  assert.ok(malformed);
  const invalid = deriveRibbonModel(
    { route: malformed, params: { courseRef: "C-1/not-a-reference" } },
    { productRole: "instructor" },
    labels,
  );
  assert.deepEqual(invalid.breadcrumbs, [], "malformed scope fails closed without identifier copy");
  assert.equal(
    invalid.breadcrumbPreludeReserved,
    true,
    "declared deep-route geometry remains stable",
  );

  const scoped = deriveRibbonModel(
    routeStateFor("courseAppearance"),
    { productRole: "instructor" },
    labels,
  );
  assert.equal(scoped.context.scopeLabel, "BCHM 355");
  assert.equal(scoped.breadcrumbs[1]?.label, "Biochemistry I");
});

test("Student Course Invitation acceptance stays outside Course breadcrumb context", () => {
  const model = deriveRibbonModel(
    routeStateFor("studentCourseInvitation"),
    { productRole: "student" },
    LABELS,
  );
  assert.deepEqual(model.breadcrumbs, []);
  assert.equal(model.breadcrumbPreludeReserved, false);
});
