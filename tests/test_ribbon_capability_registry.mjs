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
      createRibbonCapabilityEntry("questions", {
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

test("unbacked Instructor Product destinations remain unavailable without invented links", () => {
  for (const id of [
    "myActiveCourses",
    "myInactiveCourses",
    "searchPublicBlueprintCourses",
    "myQuestions",
    "starred",
    "watched",
    "assignmentTemplates",
  ]) {
    const entry = CAPABILITY_REGISTRY[id];
    assert.equal(entry.capability.kind, "unbacked", id);
    assert.equal(ribbonAvailability(entry, "instructor", RESOLVED_ALLOW), "Unavailable", id);
  }
});

test("Product Assignments enters the backed Assignments Due Soon reader", () => {
  const productAssignments = CAPABILITY_REGISTRY.productAssignments;
  assert.equal(productAssignments.capability.kind, "backed");
  assert.equal(productAssignments.routeId, "assignmentsDueSoon");
  assert.equal(productAssignments.capability.clientMethod, "ApiClient.listAssignmentsDueSoon");
  assert.deepEqual(productAssignments.capability.serverEvidence, {
    kind: "registeredHandler",
    handler: "crates/server/src/assignment_release.rs::assignment_release_router",
  });
  assert.equal(ribbonAvailability(productAssignments, "instructor", RESOLVED_ALLOW), "Available");
});

test("Assignments Due Soon is backed by the bounded cross-Course Assignment reader", () => {
  const dueSoon = CAPABILITY_REGISTRY.assignmentsDueSoon;
  assert.equal(dueSoon.capability.kind, "backed");
  assert.equal(dueSoon.capability.clientMethod, "ApiClient.listAssignmentsDueSoon");
  assert.deepEqual(dueSoon.capability.serverEvidence, {
    kind: "registeredHandler",
    handler: "crates/server/src/assignment_release.rs::assignment_release_router",
  });
  assert.equal(ribbonAvailability(dueSoon, "instructor", RESOLVED_ALLOW), "Available");
});

test("Student Attempt navigation is backed by the registered Assignment delivery handler", () => {
  const attempt = CAPABILITY_REGISTRY.attempt;
  assert.equal(attempt.capability.kind, "backed");
  assert.equal(attempt.capability.clientMethod, "ApiClient.startLiveAssignment");
  assert.deepEqual(attempt.capability.serverEvidence, {
    kind: "registeredHandler",
    handler: "crates/server/src/assignment_delivery.rs::assignment_delivery_router",
  });
  assert.equal(ribbonAvailability(attempt, "student", RESOLVED_ALLOW), "Available");
});

test("Student Attempt return uses the existing Student Assignment Access boundary", () => {
  const backToAssignment = CAPABILITY_REGISTRY.backToAssignments;
  assert.equal(backToAssignment.capability.kind, "backed");
  assert.equal(backToAssignment.capability.clientMethod, "ApiClient.getLiveAssignmentAccess");
  assert.deepEqual(backToAssignment.capability.serverEvidence, {
    kind: "registeredHandler",
    handler: "crates/server/src/assignment_delivery.rs::assignment_delivery_router",
  });
  assert.equal(ribbonAvailability(backToAssignment, "student", RESOLVED_ALLOW), "Available");
});
