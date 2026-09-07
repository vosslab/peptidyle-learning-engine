import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TAB_IDS } from "../src/route_contract.ts";
import {
  hasAppendOnlyRelationshipSuffix,
  RIBBON_SCOPES,
  ribbonSchemaFor,
} from "../src/ribbon/ribbon_schema.ts";

const PRODUCT_ROLES = ["instructor", "student", "sysadmin"];

test("schemas contain declared tabs and keep relationship-specific slots in an append-only suffix", () => {
  for (const scope of RIBBON_SCOPES) {
    for (const role of PRODUCT_ROLES) {
      const schema = ribbonSchemaFor(scope, role);
      assert.equal(hasAppendOnlyRelationshipSuffix(schema), true, `${scope}/${role}`);
      for (const slot of schema) {
        assert.equal(RIBBON_TAB_IDS.includes(slot.id), true, `${scope}/${role}/${slot.id}`);
      }
    }
  }
});

test("relationship suffix validation rejects an interleaving after it has begun", () => {
  assert.equal(
    hasAppendOnlyRelationshipSuffix([
      { id: "courses", relationshipRequirement: "none" },
      { id: "questionLibrary", relationshipRequirement: "grader" },
      { id: "blueprintCourses", relationshipRequirement: "none" },
    ]),
    false,
  );
});

test("schema results and their slots are immutable", () => {
  const schema = ribbonSchemaFor("product", "instructor");
  assert.equal(Object.isFrozen(schema), true);
  for (const slot of schema) assert.equal(Object.isFrozen(slot), true, slot.id);
});
