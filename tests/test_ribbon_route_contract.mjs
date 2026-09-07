// Semantic route-to-Ribbon presentation boundaries.

import assert from "node:assert/strict";
import test from "node:test";

import { ROUTE_CONTRACT } from "../src/route_contract.ts";
import { RIBBON_TASK_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
import { ribbonSchemaFor } from "../src/ribbon/ribbon_schema.ts";

const PRODUCT_ROLES = ["instructor", "student", "sysadmin"];

function tabExistsForAnyProductRole(scope, tab) {
  return new Set(
    PRODUCT_ROLES.flatMap((productRole) =>
      ribbonSchemaFor(scope, productRole).map((slot) => slot.id),
    ),
  ).has(tab);
}

test("declared routes select only Ribbon topology and task areas that exist", () => {
  for (const route of ROUTE_CONTRACT) {
    const { scope, tab, taskGroup } = route.ribbon;

    if (tab !== undefined) {
      if (route.requiredProductRoles.length === 0) {
        assert.equal(tabExistsForAnyProductRole(scope, tab), true, route.id);
      } else {
        for (const productRole of route.requiredProductRoles) {
          assert.equal(
            ribbonSchemaFor(scope, productRole).some((slot) => slot.id === tab),
            true,
            `${route.id}/${productRole}`,
          );
        }
      }
    }

    if (taskGroup !== undefined) {
      assert.notEqual(tab, undefined, route.id);
      const taskAreas = RIBBON_TASK_CATALOG.filter((control) => control.taskGroup === taskGroup);
      assert.notEqual(taskAreas.length, 0, route.id);
    }
  }
});
