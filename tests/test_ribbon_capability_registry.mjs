// Ribbon capability registry tests prove truthful UI admission without claiming
// service authorization.

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

function backedEntry({ routeId = "library", relationshipRequirement = "none" } = {}) {
  const control = CATALOG.find(
    (candidate) =>
      candidate.destination.kind === "route" && candidate.destination.routeId === routeId,
  );
  assert.ok(control, `synthetic backed entry needs catalog route ${routeId}`);
  return createRibbonCapabilityEntry(
    control.id,
    {
      kind: "backed",
      clientMethod: "SyntheticApi.read",
      serverEvidence: { kind: "registeredHandler", handler: "synthetic_router" },
      evidence: ["tests/test_ribbon_capability_registry.mjs::backedEntry"],
    },
    relationshipRequirement,
  );
}

test("catalog-joined construction rejects every incomplete capability proof", () => {
  const validBacked = {
    kind: "backed",
    clientMethod: "SyntheticApi.read",
    serverEvidence: { kind: "registeredHandler", handler: "synthetic_router" },
    evidence: ["tests/test_ribbon_capability_registry.mjs::validBacked"],
  };
  const invalidDeclarations = [
    ["future backed destination", "blueprintUpdates", validBacked],
    ["blank backed client method", "questionLibrary", { ...validBacked, clientMethod: " " }],
    ["empty backed evidence", "questionLibrary", { ...validBacked, evidence: [] }],
    ["blank backed evidence item", "questionLibrary", { ...validBacked, evidence: [" "] }],
    [
      "blank registered handler",
      "questionLibrary",
      {
        ...validBacked,
        serverEvidence: { kind: "registeredHandler", handler: " " },
      },
    ],
    [
      "blank no-server-call justification",
      "questionLibrary",
      {
        ...validBacked,
        serverEvidence: { kind: "noServerCall", justification: " " },
      },
    ],
    [
      "blank unbacked reason",
      "questionLibrary",
      { kind: "unbacked", reason: " ", evidence: ["test::unbacked"] },
    ],
    [
      "empty unbacked evidence",
      "questionLibrary",
      { kind: "unbacked", reason: "future", evidence: [] },
    ],
    [
      "blank unbacked evidence item",
      "questionLibrary",
      { kind: "unbacked", reason: "future", evidence: [" "] },
    ],
  ];

  for (const [description, id, capability] of invalidDeclarations) {
    assert.throws(() => createRibbonCapabilityEntry(id, capability), Error, description);
  }
});

test("catalog-joined construction accepts registered-handler and no-server-call proofs", () => {
  const registeredHandler = backedEntry();
  assert.equal(registeredHandler.capability.kind, "backed");

  const noServerCall = createRibbonCapabilityEntry("questionLibrary", {
    kind: "backed",
    clientMethod: "SyntheticUi.open",
    serverEvidence: {
      kind: "noServerCall",
      justification: "The interaction changes only local presentation state.",
    },
    evidence: ["tests/test_ribbon_capability_registry.mjs::noServerCall"],
  });
  assert.equal(noServerCall.capability.kind, "backed");
  assert.equal(noServerCall.capability.serverEvidence.kind, "noServerCall");
});

test("registry is total over exactly the 24 unique catalog destinations", () => {
  const catalogIds = CATALOG.map(({ id }) => id);
  assert.equal(catalogIds.length, 24);
  assert.equal(new Set(catalogIds).size, catalogIds.length);
  assert.deepEqual(Object.keys(CAPABILITY_REGISTRY).sort(), [...catalogIds].sort());
  assert.equal("createAssignment" in CAPABILITY_REGISTRY, false);
  assert.equal("assignmentAttemptProgress" in CAPABILITY_REGISTRY, false);
  assert.equal("context" in CAPABILITY_REGISTRY, false);
});

test("registry joins the declared catalog and never invents a route", () => {
  const routeIds = new Set(ROUTE_CONTRACT.map(({ id }) => id));
  for (const control of CATALOG) {
    const entry = CAPABILITY_REGISTRY[control.id];
    assert.equal(entry.label, control.label, control.id);
    assert.deepEqual(entry.destination, control.destination, control.id);
    if (control.destination.kind === "route") {
      assert.equal(entry.routeId, control.destination.routeId, control.id);
      assert.ok(routeIds.has(entry.routeId), control.id);
    } else {
      assert.equal(entry.routeId, undefined, control.id);
    }
  }
});

test("all current catalog destinations remain honestly unbacked with reviewable evidence", () => {
  for (const entry of Object.values(CAPABILITY_REGISTRY)) {
    assert.equal(entry.capability.kind, "unbacked", entry.id);
    assert.ok(entry.capability.reason.length > 0, entry.id);
    assert.ok(entry.capability.evidence.length > 0, entry.id);
    assert.equal(entry.relationshipRequirement, "none", entry.id);
    for (const role of PRODUCT_ROLES) {
      assert.notEqual(ribbonAvailability(entry, role, OUTSTANDING), "Checking", entry.id);
    }
  }
});

test("Teaching Operations evidence names its current mounted page export", () => {
  const teachingOperations = CAPABILITY_REGISTRY.teachingOperations;
  assert.ok(
    teachingOperations.capability.evidence.includes(
      "src/pages/teaching_operations_page.tsx::TeachingOperationsPage",
    ),
  );
});

test("My Question Drafts remains one closed future identity outside route admission", () => {
  const drafts = CAPABILITY_REGISTRY.myQuestionDrafts;
  assert.deepEqual(drafts.destination, { kind: "future", futureId: "myQuestionDrafts" });
  assert.equal("routeId" in drafts, false);
  assert.equal(drafts.capability.kind, "unbacked");
  assert.equal(ribbonAvailability(drafts, "instructor", RESOLVED_ALLOW), "Unavailable");
  assert.equal(isRibbonEntryVisible(drafts, "instructor", RESOLVED_ALLOW), false);
  assert.equal(CATALOG.filter(({ id }) => id === "myQuestionDrafts").length, 1);
});

test("availability applies capability, route role, and relationship precedence in order", () => {
  const unbacked = CAPABILITY_REGISTRY.questionLibrary;
  assert.equal(ribbonAvailability(unbacked, "instructor", OUTSTANDING), "Unavailable");

  const instructorOnly = backedEntry();
  assert.equal(ribbonAvailability(instructorOnly, "student", OUTSTANDING), "Unavailable");

  const relationshipBacked = backedEntry({ relationshipRequirement: "grader" });
  assert.equal(ribbonAvailability(relationshipBacked, "instructor", OUTSTANDING), "Checking");
  assert.equal(ribbonAvailability(relationshipBacked, "instructor", RESOLVED_DENY), "Unavailable");
  assert.equal(ribbonAvailability(relationshipBacked, "instructor", RESOLVED_ALLOW), "Available");
  assert.equal(ribbonAvailability(instructorOnly, "instructor", OUTSTANDING), "Available");
});

test("Checking is explicitly withheld rather than rendered", () => {
  const entry = backedEntry({ relationshipRequirement: "grader" });
  assert.equal(isRibbonAvailabilityVisible("Checking"), false);
  assert.equal(isRibbonEntryVisible(entry, "instructor", OUTSTANDING), false);
  assert.equal(isRibbonAvailabilityVisible("Unavailable"), false);
  assert.equal(isRibbonAvailabilityVisible("Available"), true);
});

test("Available is always beneath the declared Product Role route ceiling", () => {
  for (const role of PRODUCT_ROLES) {
    for (const entry of Object.values(CAPABILITY_REGISTRY)) {
      if (ribbonAvailability(entry, role, RESOLVED_ALLOW) !== "Available") continue;
      assert.notEqual(entry.routeId, undefined, entry.id);
      assert.equal(productRoleMayAccessRoute(entry.routeId, role), true, entry.id);
    }
  }
});
