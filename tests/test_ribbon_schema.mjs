import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TAB_IDS } from "../src/route_contract.ts";
import {
  hasAppendOnlyRelationshipSuffix,
  PRODUCT_TIER_ONE,
  ribbonSchemaFor,
} from "../src/ribbon/ribbon_schema.ts";

const PRODUCT_ROLES = ["instructor", "student", "sysadmin"];

test("Product Role tier-one schemas contain declared tabs and keep relationship slots append-only", () => {
  for (const role of PRODUCT_ROLES) {
    const schema = ribbonSchemaFor(role);
    assert.equal(schema, PRODUCT_TIER_ONE[role], role);
    assert.equal(hasAppendOnlyRelationshipSuffix(schema), true, role);
    for (const slot of schema) {
      assert.equal(RIBBON_TAB_IDS.includes(slot.id), true, `${role}/${slot.id}`);
    }
  }
});

test("relationship suffix validation rejects an interleaving after it has begun", () => {
  assert.equal(
    hasAppendOnlyRelationshipSuffix([
      { id: "courses", relationshipRequirement: "none" },
      { id: "questions", relationshipRequirement: "grader" },
      { id: "productAssessments", relationshipRequirement: "none" },
    ]),
    false,
  );
});

test("schema results and their slots are immutable", () => {
  const schema = ribbonSchemaFor("instructor");
  assert.equal(Object.isFrozen(schema), true);
  for (const slot of schema) assert.equal(Object.isFrozen(slot), true, slot.id);
});
