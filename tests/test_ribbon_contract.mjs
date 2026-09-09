import assert from "node:assert/strict";
import test from "node:test";

import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import { buildRoutePath, deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { routeParams } from "../src/navigation/route_params.ts";
import { productRoleMayAccessRoute, ROUTE_CONTRACT } from "../src/route_contract.ts";
import { CAPABILITY_REGISTRY } from "../src/ribbon/capability_registry.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { loadAppRibbonForSsr } from "./support/ribbon_component_ssr.ts";
import { M6_RIBBON_FIXTURES } from "./support/ribbon_model_fixtures.ts";

const PRODUCT_ROLES = ["student", "instructor", "sysadmin"];
const LABELS = { accountLabel: "Neil Voss" };
const PARAMETER_VALUES = {
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
  membershipRef: "M-1",
  questionRef: "7K3M9QP",
  draftQuestionRef: "D-1",
  blueprintCourseRef: "BP-1",
  presentationNonce: "0123456789abcdef0123456789abcdef",
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

test("Task Row topology is exactly the declared task-group topology for every route", () => {
  for (const route of ROUTE_CONTRACT) {
    for (const role of PRODUCT_ROLES) {
      const model = deriveRibbonModel(routeStateFor(route.id), { productRole: role }, LABELS);
      assert.equal(
        model.taskAreas.length > 0,
        route.ribbon.taskGroup !== undefined,
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
