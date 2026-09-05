// test_ribbon_schema.mjs - Stable topology evidence for Ribbon schemas.

import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TAB_IDS } from "../src/route_contract.ts";
import {
  hasAppendOnlyRelationshipSuffix,
  RIBBON_SCOPES,
  ribbonSchemaFor,
} from "../src/ribbon/ribbon_schema.ts";

const EXPECTED_SCHEMAS = {
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
};

const PRODUCT_ROLES = ["instructor", "student", "sysadmin"];

test("every scope and immutable Product Role pair has its exact designed schema", () => {
  for (const scope of RIBBON_SCOPES) {
    for (const productRole of PRODUCT_ROLES) {
      const schema = ribbonSchemaFor(scope, productRole);
      assert.deepEqual(
        schema.map((slot) => slot.id),
        EXPECTED_SCHEMAS[scope][productRole],
        `${scope}/${productRole}`,
      );
    }
  }
});

test("each schema position is a declared Ribbon tab and currently needs no relationship", () => {
  for (const scope of RIBBON_SCOPES) {
    for (const productRole of PRODUCT_ROLES) {
      for (const slot of ribbonSchemaFor(scope, productRole)) {
        assert.ok(RIBBON_TAB_IDS.includes(slot.id), `${scope}/${productRole}/${slot.id}`);
        assert.equal(slot.relationshipRequirement, "none");
      }
    }
  }
});

test("relationship-narrowed positions form an append-only suffix", () => {
  const validSuffix = [
    { id: "courses", relationshipRequirement: "none" },
    { id: "questionLibrary", relationshipRequirement: "courseObserver" },
    { id: "blueprintCourses", relationshipRequirement: "grader" },
  ];
  const invalidInterleaving = [
    { id: "courses", relationshipRequirement: "none" },
    { id: "questionLibrary", relationshipRequirement: "courseObserver" },
    { id: "blueprintCourses", relationshipRequirement: "none" },
  ];

  for (const scope of RIBBON_SCOPES) {
    for (const productRole of PRODUCT_ROLES) {
      assert.equal(hasAppendOnlyRelationshipSuffix(ribbonSchemaFor(scope, productRole)), true);
    }
  }
  assert.equal(hasAppendOnlyRelationshipSuffix(validSuffix), true);
  assert.equal(hasAppendOnlyRelationshipSuffix(invalidInterleaving), false);
});

test("schemas are immutable and independent between calls and pairs", () => {
  const instructorProduct = ribbonSchemaFor("product", "instructor");
  const instructorProductAgain = ribbonSchemaFor("product", "instructor");
  const studentProduct = ribbonSchemaFor("product", "student");

  assert.strictEqual(instructorProduct, instructorProductAgain);
  assert.notStrictEqual(instructorProduct, studentProduct);
  assert.equal(Object.isFrozen(instructorProduct), true);
  assert.equal(Object.isFrozen(instructorProduct[0]), true);
  assert.throws(() => instructorProduct.push(instructorProduct[0]), TypeError);
  assert.deepEqual(
    ribbonSchemaFor("product", "instructor").map((slot) => slot.id),
    EXPECTED_SCHEMAS.product.instructor,
  );
});
