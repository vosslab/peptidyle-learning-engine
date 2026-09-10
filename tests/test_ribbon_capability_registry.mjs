import assert from "node:assert/strict";
import test from "node:test";

import { productRoleMayAccessRoute, ROUTE_CONTRACT } from "../src/route_contract.ts";
import {
  CAPABILITY_REGISTRY,
  createRibbonCapabilityEntry,
  isRibbonAvailabilityVisible,
  isRibbonEntryVisible,
  ribbonAvailability,
} from "../src/ribbon/capability_registry.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";

const PRODUCT_ROLES = ["instructor", "student", "sysadmin"];
const RESOLVED_ALLOW = { kind: "resolved", allowed: true };
const RESOLVED_DENY = { kind: "resolved", allowed: false };
const OUTSTANDING = { kind: "outstanding" };
const CATALOG = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG];

function backedEntry(relationshipRequirement = "none") {
  const control = CATALOG.find(
    (candidate) =>
      candidate.destination.kind === "route" && candidate.destination.routeId === "library",
  );
  assert.ok(control, "a declared route supplies the synthetic backed entry");
  return createRibbonCapabilityEntry(
    control.id,
    {
      kind: "backed",
      clientMethod: "SyntheticApi.read",
      serverEvidence: { kind: "registeredHandler", handler: "synthetic_router" },
      evidence: ["tests/test_ribbon_capability_registry.mjs"],
    },
    relationshipRequirement,
  );
}

test("capability construction rejects incomplete proofs and backing a future identity", () => {
  assert.throws(
    () => createRibbonCapabilityEntry("blueprintUpdates", backedEntry().capability),
    Error,
  );
  assert.throws(
    () =>
      createRibbonCapabilityEntry("questionLibrary", {
        kind: "backed",
        clientMethod: " ",
        serverEvidence: { kind: "registeredHandler", handler: "router" },
        evidence: ["test"],
      }),
    Error,
  );
});

test("registry and catalog agree on each declared navigation destination", () => {
  const routeIds = new Set(ROUTE_CONTRACT.map(({ id }) => id));
  for (const control of CATALOG) {
    const entry = CAPABILITY_REGISTRY[control.id];
    assert.ok(entry, control.id);
    assert.deepEqual(entry.destination, control.destination, control.id);
    if (control.destination.kind !== "future") {
      assert.equal(entry.routeId, control.destination.routeId, control.id);
      assert.equal(routeIds.has(entry.routeId), true, control.id);
    } else {
      assert.equal(entry.routeId, undefined, control.id);
    }
  }
  for (const entry of Object.values(CAPABILITY_REGISTRY)) {
    assert.ok(
      CATALOG.some((control) => control.id === entry.id),
      entry.id,
    );
  }
});

test("availability applies capability, route role, and relationship precedence", () => {
  const entry = backedEntry("grader");
  assert.equal(ribbonAvailability(entry, "student", OUTSTANDING), "Unavailable");
  assert.equal(ribbonAvailability(entry, "instructor", OUTSTANDING), "Checking");
  assert.equal(ribbonAvailability(entry, "instructor", RESOLVED_DENY), "Unavailable");
  assert.equal(ribbonAvailability(entry, "instructor", RESOLVED_ALLOW), "Available");
});

test("Checking remains withheld and Available never exceeds the route role ceiling", () => {
  const checking = backedEntry("grader");
  assert.equal(isRibbonAvailabilityVisible("Checking"), false);
  assert.equal(isRibbonEntryVisible(checking, "instructor", OUTSTANDING), false);
  for (const role of PRODUCT_ROLES) {
    for (const entry of Object.values(CAPABILITY_REGISTRY)) {
      if (ribbonAvailability(entry, role, RESOLVED_ALLOW) !== "Available") continue;
      assert.ok(entry.routeId, entry.id);
      assert.equal(productRoleMayAccessRoute(entry.routeId, role), true, entry.id);
    }
  }
});
