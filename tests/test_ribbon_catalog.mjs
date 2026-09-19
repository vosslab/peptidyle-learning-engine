import assert from "node:assert/strict";
import test from "node:test";

import { ROUTE_CONTRACT } from "../src/route_contract.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";

const CATALOG = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG];

function paramsInPath(path) {
  return [...path.matchAll(/:([A-Za-z][A-Za-z0-9]*)/g)].map((match) => match[1]);
}

test("catalog route destinations name declared routes and their complete parameter boundary", () => {
  const routesById = new Map(ROUTE_CONTRACT.map((route) => [route.id, route]));
  for (const control of CATALOG) {
    if (control.destination.kind !== "route") continue;
    const route = routesById.get(control.destination.routeId);
    assert.ok(route, control.id);
    assert.deepEqual(control.requiredParams, paramsInPath(route.path), control.id);
  }
});

test("catalog future destinations remain identities rather than fabricated routes", () => {
  const routeIds = new Set(ROUTE_CONTRACT.map((route) => route.id));
  for (const control of CATALOG) {
    if (control.destination.kind !== "future") continue;
    assert.equal(routeIds.has(control.destination.futureId), false, control.id);
    assert.equal("routeId" in control.destination, false, control.id);
  }
});

test("Course list choices use the declared active and inactive Instructor routes", () => {
  const destinations = new Map(
    RIBBON_TASK_CATALOG.map((control) => [control.id, control.destination]),
  );
  assert.deepEqual(destinations.get("myActiveCourses"), {
    kind: "route",
    routeId: "instructorHome",
  });
  assert.deepEqual(destinations.get("myInactiveCourses"), {
    kind: "route",
    routeId: "instructorInactiveCourses",
  });
});
