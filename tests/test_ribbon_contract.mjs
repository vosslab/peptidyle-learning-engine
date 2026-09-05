import assert from "node:assert/strict";
import test from "node:test";

import {
  buildRoutePath,
  deriveRibbonModel,
  ribbonModelAvailabilityMayAccessRoute,
} from "../src/ribbon/ribbon_contract.ts";
import { routeParams, routeScopeKey } from "../src/navigation/route_params.ts";
import {
  productRoleMayAccessRoute,
  routeContractForPathname,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { CAPABILITY_REGISTRY, ribbonAvailability } from "../src/ribbon/capability_registry.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { PRODUCT_ROLE_FIXTURES, createDeferredResolution } from "./support/ribbon_test_support.ts";

const PRODUCT_ROLES = ["student", "instructor", "sysadmin"];
const SCOPES = ["product", "courseInstance", "assignmentAttempt"];
const BUILD_ROUTE_PATH_FAILS_CLOSED_TEST_NAME =
  "buildRoutePath fails closed for unknown, incomplete, surplus, malformed, and " +
  "smuggled input";
const TRUTHFULNESS_REGISTRY_TEST_NAME =
  "current truthfulness registry yields no Checking controls and never exceeds route role access";
const RELATIONSHIP_ADMISSION_TEST_NAME =
  "registry-owned relationship admission can check without moving schema-owned positions";
const DERIVED_CONTROL_PARAMETERS_TEST_NAME =
  "every derived control receives its required parameters from its declared source route";
const BACK_TO_ASSIGNMENTS_WITHHELD_TEST_NAME =
  "back-to-assignments stays withheld when a test-only backed capability " +
  "lacks its source parameter";
const LABELS = Object.freeze({
  accountLabel: "Neil Voss",
  courseTitle: "Molecular Biology",
  assignmentTitle: "Promoter logic",
  assignmentAttemptTitle: "Promoter logic practice",
  assignmentAttemptProgress: "Question 12 of 12",
});

const PARAMETER_VALUES = Object.freeze({
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
  membershipRef: "M-1",
  questionRef: "7K3M9QP",
  blueprintCourseRef: "BP-1",
});

const EXPECTED_SCHEMAS = Object.freeze({
  product: {
    instructor: ["courses", "questionLibrary", "blueprintCourses"],
    student: ["courses"],
    sysadmin: ["courses", "instructorAccounts"],
  },
  courseInstance: {
    instructor: [
      "assignments",
      "students",
      "gradebook",
      "teachingOperations",
      "blueprintUpdates",
      "courseSetup",
    ],
    student: ["assignments"],
    sysadmin: ["teachingOperations"],
  },
  assignmentAttempt: {
    instructor: [],
    student: ["attempt"],
    sysadmin: [],
  },
});

function paramsForRoute(route) {
  const values = {};
  for (const segment of route.path.split("/")) {
    if (!segment.startsWith(":")) continue;
    const name = segment.slice(1);
    values[name] = PARAMETER_VALUES[name];
  }
  return values;
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

function modelFor(routeId, productRole) {
  const result = deriveRibbonModel(routeStateFor(routeId), { productRole }, LABELS);
  return result;
}

function declaredParamNames(route) {
  return route.path
    .split("/")
    .filter((segment) => segment.startsWith(":"))
    .map((segment) => segment.slice(1));
}

test("buildRoutePath round trips all 23 declared routes with canonical public references", () => {
  assert.equal(ROUTE_CONTRACT.length, 23);
  for (const route of ROUTE_CONTRACT) {
    const pathname = buildRoutePath(route.id, paramsForRoute(route));
    assert.ok(pathname, route.id);
    assert.equal(pathname.includes("?"), false, route.id);
    assert.equal(pathname.includes("#"), false, route.id);
    const extracted = routeParams(route, pathname);
    assert.ok(extracted, route.id);
    assert.deepEqual(Object.keys(extracted).sort(), declaredParamNames(route).sort(), route.id);
  }
  assert.equal(buildRoutePath("questionDetail", { questionRef: "7k3m9qp" }), "/library/7K3-M9QP");
});

test(BUILD_ROUTE_PATH_FAILS_CLOSED_TEST_NAME, () => {
  assert.equal(buildRoutePath("unknown", {}), undefined);
  assert.equal(buildRoutePath("courseAssignments", {}), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1", extra: "x" }), undefined);
  assert.equal(buildRoutePath("courses", { extra: "x" }), undefined);
  assert.equal(buildRoutePath("courses", []), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-0" }), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1/gradebook" }), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1%2Fgradebook" }), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1?next=/" }), undefined);
  assert.equal(buildRoutePath("courseAssignments", { courseRef: "C-1#next" }), undefined);
  assert.equal(
    buildRoutePath("assignmentOverview", { courseRef: "C-1", assignmentRef: 1 }),
    undefined,
  );
  assert.equal(buildRoutePath("questionDetail", { questionRef: "7K3%2FM9QP" }), undefined);
  assert.equal(
    buildRoutePath("questionDetail", { questionRef: "7K3-M9QP", extra: undefined }),
    undefined,
  );
});

test("malformed matched scoped URLs retain their declared data-free Ribbon schema", () => {
  const fixtures = [
    {
      pathname: "/courses/C-0",
      scope: "courseInstance",
      schema: EXPECTED_SCHEMAS.courseInstance.instructor,
    },
    {
      pathname: "/assignment-attempts/R-0",
      scope: "assignmentAttempt",
      schema: EXPECTED_SCHEMAS.assignmentAttempt.instructor,
    },
  ];

  for (const fixture of fixtures) {
    const route = routeContractForPathname(fixture.pathname);
    assert.ok(route, `${fixture.pathname} remains a matched declared route`);
    const params = routeParams(route, fixture.pathname);
    assert.ok(params, `${fixture.pathname} retains only raw declared parameters`);
    assert.deepEqual(routeScopeKey(fixture.pathname), { kind: "invalid", scope: fixture.scope });

    const model = deriveRibbonModel(
      { route, params },
      { productRole: "instructor" },
      { accountLabel: LABELS.accountLabel },
    );
    assert.equal(model.scope, fixture.scope, `${fixture.pathname} preserves declared scope`);
    assert.deepEqual(
      model.tabs.map((tab) => tab.id),
      fixture.schema,
      `${fixture.pathname} preserves declared tab schema`,
    );
    assert.equal(
      model.context.scopeLabel,
      undefined,
      `${fixture.pathname} has no fabricated title`,
    );
    assert.equal(
      model.context.assignmentLabel,
      undefined,
      `${fixture.pathname} has no fabricated assignment label`,
    );
    for (const control of [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)]) {
      assert.equal(control.availability, "Unavailable", `${fixture.pathname}/${control.id}`);
      assert.equal(control.href, undefined, `${fixture.pathname}/${control.id}`);
    }
  }
});

test("model preserves the exact designed 3 by 3 topology before admission", () => {
  const routeForScope = {
    product: "courses",
    courseInstance: "courseAssignments",
    assignmentAttempt: "assignmentAttempt",
  };
  for (const scope of SCOPES) {
    for (const productRole of PRODUCT_ROLES) {
      const model = modelFor(routeForScope[scope], productRole);
      assert.equal(model.scope, scope);
      assert.deepEqual(
        model.tabs.map((tab) => tab.id),
        EXPECTED_SCHEMAS[scope][productRole],
        `${scope}/${productRole}`,
      );
      assert.equal(Object.isFrozen(model), true, `${scope}/${productRole} model`);
      assert.equal(Object.isFrozen(model.tabs), true, `${scope}/${productRole} tabs`);
      assert.equal(Object.isFrozen(model.context), true, `${scope}/${productRole} context`);
      assert.equal(
        Object.isFrozen(model.context.signOutAction),
        true,
        `${scope}/${productRole} sign out action`,
      );
      for (const control of [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)]) {
        assert.equal(Object.isFrozen(control), true, `${scope}/${productRole}/${control.id}`);
        assert.equal(
          Object.isFrozen(control.destination),
          true,
          `${scope}/${productRole}/${control.id} destination`,
        );
        const catalogControl = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG].find(
          (candidate) => candidate.id === control.id,
        );
        assert.ok(catalogControl, `${control.id} catalog entry`);
        assert.notEqual(
          control.destination,
          catalogControl.destination,
          `${scope}/${productRole}/${control.id} destination is a model-owned copy`,
        );
        assert.equal(
          Reflect.set(control.destination, "kind", "future"),
          false,
          `${scope}/${productRole}/${control.id} destination cannot mutate`,
        );
      }
      for (const area of model.taskAreas) {
        assert.equal(Object.isFrozen(area), true, `${scope}/${productRole}/${area.id} area`);
        assert.equal(
          Object.isFrozen(area.controls),
          true,
          `${scope}/${productRole}/${area.id} controls`,
        );
      }
    }
  }
});

test(TRUTHFULNESS_REGISTRY_TEST_NAME, () => {
  for (const productRole of PRODUCT_ROLES) {
    for (const route of ROUTE_CONTRACT) {
      const model = modelFor(route.id, productRole);
      for (const control of [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)]) {
        assert.notEqual(
          control.availability,
          "Checking",
          `${route.id}/${productRole}/${control.id}`,
        );
        assert.equal(
          control.availability,
          "Unavailable",
          `${route.id}/${productRole}/${control.id}`,
        );
        assert.equal(control.href, undefined, `${route.id}/${productRole}/${control.id}`);
        assert.equal(ribbonModelAvailabilityMayAccessRoute(control, productRole), true);
        if (control.availability === "Available" && control.destination.kind === "route") {
          assert.equal(productRoleMayAccessRoute(control.destination.routeId, productRole), true);
        }
      }
    }
  }
});

test("derivation is synchronous while a deferred API request remains unresolved", async () => {
  const fixtures = [
    ["student", "assignmentAttempt"],
    ["instructor", "assignmentWorkspaceQuestions"],
    ["sysadmin", "courses"],
  ];
  for (const [productRole, routeId] of fixtures) {
    const deferred = createDeferredResolution();
    let requested = false;
    let settled = false;
    const client = createHttpApiClient({
      fetch: (input, init) => {
        requested = true;
        assert.equal(input, "/api/courses", `${productRole} starts the harmless course read`);
        assert.equal(init?.method, "GET", `${productRole} preserves the client request method`);
        return deferred.promise;
      },
    });
    const networkOutcome = client.listCourses().then(
      () => "success",
      (error) => error,
    );
    void networkOutcome.then(() => {
      settled = true;
    });
    assert.equal(requested, true, `${productRole} invokes the injected fake transport`);

    const fixture = PRODUCT_ROLE_FIXTURES[productRole];
    const before = deriveRibbonModel(
      routeStateFor(routeId),
      { productRole: fixture.productRole },
      LABELS,
    );
    const beforeSlots = [
      ...before.tabs.map((control) => control.id),
      ...before.taskAreas.flatMap((area) => area.controls.map((control) => control.id)),
    ];
    assert.equal(
      settled,
      false,
      `${productRole} deferred fixture remains unresolved during derivation`,
    );
    assert.equal(
      typeof before.then,
      "undefined",
      `${productRole} returns a model, never a thenable`,
    );

    deferred.resolve(new Response("not found", { status: 404, statusText: "Not Found" }));
    const outcome = await networkOutcome;
    assert.notEqual(outcome, "success", `${productRole} consumes the expected harmless read error`);
    assert.equal(settled, true, `${productRole} deferred API request settles after release`);
    const after = deriveRibbonModel(
      routeStateFor(routeId),
      { productRole: fixture.productRole },
      LABELS,
    );
    const afterSlots = [
      ...after.tabs.map((control) => control.id),
      ...after.taskAreas.flatMap((area) => area.controls.map((control) => control.id)),
    ];
    assert.deepEqual(
      afterSlots,
      beforeSlots,
      `${productRole} topology is independent of deferred work`,
    );
  }
});

test(RELATIONSHIP_ADMISSION_TEST_NAME, () => {
  const entry = CAPABILITY_REGISTRY.assignments;
  const originalDescriptors = Object.getOwnPropertyDescriptors(entry);
  const before = modelFor("courseAssignments", "instructor");
  const beforeSlots = before.tabs.map((control) => control.id);
  try {
    Object.assign(entry, {
      relationshipRequirement: "grader",
      capability: {
        kind: "backed",
        clientMethod: "test-only capability fixture",
        serverEvidence: { kind: "noServerCall", justification: "test-only capability fixture" },
        evidence: ["tests/test_ribbon_contract.mjs"],
      },
    });
    const checking = modelFor("courseAssignments", "instructor");
    assert.deepEqual(
      checking.tabs.map((control) => control.id),
      beforeSlots,
    );
    assert.equal(
      checking.tabs.find((control) => control.id === "assignments")?.availability,
      "Checking",
    );
  } finally {
    for (const key of Reflect.ownKeys(entry)) {
      if (!(key in originalDescriptors)) Reflect.deleteProperty(entry, key);
    }
    Object.defineProperties(entry, originalDescriptors);
  }
  const restored = modelFor("courseAssignments", "instructor");
  assert.deepEqual(
    restored.tabs.map((control) => control.id),
    beforeSlots,
  );
  assert.equal(
    restored.tabs.every((control) => control.availability === "Unavailable"),
    true,
  );
});

test("task rows preserve catalog order, area group labels, and route identity selection", () => {
  const questionModel = modelFor("questionDetail", "instructor");
  assert.deepEqual(
    questionModel.taskAreas.map((area) => [
      area.id,
      area.label,
      area.controls.map((control) => control.id),
    ]),
    [
      [
        "questionDestinations",
        "Question destinations",
        ["allQuestions", "myQuestions", "myQuestionDrafts"],
      ],
      ["questionRelationships", "Question relationships", ["starred", "watched"]],
    ],
  );
  assert.equal(questionModel.tabs.find((tab) => tab.id === "questionLibrary")?.selected, true);
  assert.equal(
    questionModel.taskAreas.flatMap((area) => area.controls).every((task) => !task.selected),
    true,
    "a route inside Question Library does not select a task whose destination is another route",
  );

  const assignmentModel = modelFor("assignmentWorkspaceQuestions", "instructor");
  assert.deepEqual(
    assignmentModel.taskAreas.flatMap((area) => area.controls.map((control) => control.id)),
    RIBBON_TASK_CATALOG.filter((task) => task.taskGroup === "assignment").map((task) => task.id),
  );
  assert.equal(
    assignmentModel.taskAreas
      .flatMap((area) => area.controls)
      .find((task) => task.id === "assignmentQuestions")?.selected,
    true,
  );
});

test("context routes have no selected tab and content layout follows route metadata", () => {
  for (const routeId of ["signIn", "pendingCourseInvitations"]) {
    const model = modelFor(routeId, "student");
    assert.equal(
      model.tabs.some((tab) => tab.selected),
      false,
      routeId,
    );
    assert.equal(model.taskAreas.length, 0, routeId);
  }
  assert.equal(modelFor("library", "instructor").contentLayout, "fullWidth");
  assert.equal(modelFor("questionDetail", "instructor").contentLayout, "reading");
  const context = modelFor("assignmentAttempt", "student").context;
  assert.equal(context.productLabel, "Student");
  assert.equal(context.scopeLabel, undefined);
  assert.equal(context.assignmentLabel, "Promoter logic practice");
  assert.equal(context.assignmentAttemptProgress, "Question 12 of 12");
  assert.deepEqual(context.signOutAction, { kind: "action", id: "signOut", label: "Sign out" });
});

test("Back to Assignments remains unconstructible from Assignment Attempt route state", () => {
  const model = modelFor("assignmentAttempt", "student");
  const back = model.taskAreas
    .flatMap((area) => area.controls)
    .find((task) => task.id === "backToAssignments");
  assert.ok(back);
  assert.equal(back.availability, "Unavailable");
  assert.equal(back.href, undefined);
});

test(DERIVED_CONTROL_PARAMETERS_TEST_NAME, () => {
  const intentionalExceptions = [];
  for (const route of ROUTE_CONTRACT) {
    const routeState = routeStateFor(route.id);
    for (const productRole of PRODUCT_ROLES) {
      const model = deriveRibbonModel(routeState, { productRole }, LABELS);
      for (const control of [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)]) {
        const catalogControl = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG].find(
          (candidate) => candidate.id === control.id,
        );
        assert.ok(catalogControl, `${route.id}/${productRole}/${control.id} catalog entry`);
        const missingParams = catalogControl.requiredParams.filter(
          (name) => routeState.params[name] === undefined,
        );
        if (missingParams.length === 0) continue;

        intentionalExceptions.push({ routeId: route.id, productRole, controlId: control.id });
        assert.equal(control.id, "backToAssignments", `${route.id}/${productRole}`);
        assert.equal(route.ribbon.scope, "assignmentAttempt", `${route.id}/${productRole}`);
        assert.deepEqual(missingParams, ["courseRef"], `${route.id}/${productRole}`);
        assert.equal(control.availability, "Unavailable", `${route.id}/${productRole}`);
        assert.equal(control.href, undefined, `${route.id}/${productRole}`);
      }
    }
  }
  assert.deepEqual(intentionalExceptions, [
    { routeId: "assignmentAttempt", productRole: "student", controlId: "backToAssignments" },
    {
      routeId: "assignmentAttempt",
      productRole: "instructor",
      controlId: "backToAssignments",
    },
    {
      routeId: "assignmentAttempt",
      productRole: "sysadmin",
      controlId: "backToAssignments",
    },
    {
      routeId: "assignmentAttemptSummary",
      productRole: "student",
      controlId: "backToAssignments",
    },
    {
      routeId: "assignmentAttemptSummary",
      productRole: "instructor",
      controlId: "backToAssignments",
    },
    {
      routeId: "assignmentAttemptSummary",
      productRole: "sysadmin",
      controlId: "backToAssignments",
    },
  ]);
});

test(BACK_TO_ASSIGNMENTS_WITHHELD_TEST_NAME, () => {
  const entry = CAPABILITY_REGISTRY.backToAssignments;
  const originalDescriptors = Object.getOwnPropertyDescriptors(entry);
  const sourceRouteIds = ["assignmentAttempt", "assignmentAttemptSummary"];
  const slotsBefore = sourceRouteIds.map((routeId) => {
    const model = modelFor(routeId, "student");
    return [
      ...model.tabs.map((control) => control.id),
      ...model.taskAreas.flatMap((area) => area.controls.map((control) => control.id)),
    ];
  });
  try {
    Object.assign(entry, {
      relationshipRequirement: "none",
      capability: {
        kind: "backed",
        clientMethod: "test-only back-to-assignments capability",
        serverEvidence: {
          kind: "noServerCall",
          justification: "test-only source-parameter regression fixture",
        },
        evidence: ["tests/test_ribbon_contract.mjs"],
      },
    });
    assert.equal(entry.capability.kind, "backed");
    assert.equal(entry.relationshipRequirement, "none");
    assert.equal(productRoleMayAccessRoute(entry.routeId, "student"), true);
    assert.equal(
      ribbonAvailability(entry, "student", { kind: "resolved", allowed: true }),
      "Available",
    );

    for (const [index, routeId] of sourceRouteIds.entries()) {
      const routeState = routeStateFor(routeId);
      assert.deepEqual(Object.keys(routeState.params), ["assignmentAttemptRef"], routeId);
      assert.equal(routeState.params.courseRef, undefined, routeId);
      const model = deriveRibbonModel(routeState, { productRole: "student" }, LABELS);
      const slotsDuring = [
        ...model.tabs.map((control) => control.id),
        ...model.taskAreas.flatMap((area) => area.controls.map((control) => control.id)),
      ];
      assert.deepEqual(slotsDuring, slotsBefore[index], `${routeId} slot order`);
      const back = model.taskAreas
        .flatMap((area) => area.controls)
        .find((control) => control.id === "backToAssignments");
      assert.ok(back, routeId);
      assert.equal(back.availability, "Unavailable", routeId);
      assert.equal(back.href, undefined, routeId);
    }
  } finally {
    for (const key of Reflect.ownKeys(entry)) {
      if (!(key in originalDescriptors)) Reflect.deleteProperty(entry, key);
    }
    Object.defineProperties(entry, originalDescriptors);
  }

  for (const routeId of sourceRouteIds) {
    const restored = modelFor(routeId, "student");
    const controls = [...restored.tabs, ...restored.taskAreas.flatMap((area) => area.controls)];
    assert.equal(
      controls.every((control) => control.availability === "Unavailable"),
      true,
      routeId,
    );
    const back = controls.find((control) => control.id === "backToAssignments");
    assert.ok(back, routeId);
    assert.equal(back.href, undefined, routeId);
  }
});

test("every catalog route destination declares exactly its target route parameter set", () => {
  for (const control of [...TAB_CATALOG, ...RIBBON_TASK_CATALOG]) {
    if (control.destination.kind !== "route") continue;
    const target = ROUTE_CONTRACT.find((route) => route.id === control.destination.routeId);
    assert.ok(target, control.id);
    assert.deepEqual(
      [...control.requiredParams].sort(),
      declaredParamNames(target).sort(),
      control.id,
    );
  }
});
