import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TAB_IDS, ROUTE_CONTRACT } from "../src/route_contract.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";

const CATALOG = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG];

function paramsInPath(path) {
  return [...path.matchAll(/:([A-Za-z]+Ref)/g)].map((match) => match[1]);
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

test("catalog controls are navigation descriptors with independently valid density metadata", () => {
  for (const control of CATALOG) {
    assert.ok(control.id.length > 0, "catalog control has an identity");
    assert.ok(control.label.trim().length > 0, control.id);
    assert.ok(["primary", "supporting"].includes(control.role), control.id);
    assert.ok(["critical", "normal"].includes(control.priority), control.id);
    assert.ok(["standard", "compact"].includes(control.presentation), control.id);
    assert.equal(control.iconOnlySafe && !control.iconBearing, false, control.id);
    assert.equal("operation" in control, false, control.id);
  }
});

test("schema tab identities have one corresponding tab catalog descriptor", () => {
  for (const id of RIBBON_TAB_IDS) {
    assert.equal(TAB_CATALOG.filter((control) => control.id === id).length, 1, id);
  }
});
