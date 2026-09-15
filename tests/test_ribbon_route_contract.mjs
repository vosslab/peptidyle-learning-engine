// Semantic route-to-Ribbon presentation boundaries.

import assert from "node:assert/strict";
import test from "node:test";

import {
  productRoleHomePath,
  productRoleHomeRouteId,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
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

// Permanent contract: each authenticated Product Role keeps one explicit home
// destination. A regression would strand a role at an ambiguous shared root.
test("each Product Role has an explicit selected Courses home route", () => {
  for (const productRole of PRODUCT_ROLES) {
    const routeId = productRoleHomeRouteId(productRole);
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
    assert.ok(route, productRole);
    assert.equal(route.path, productRoleHomePath(productRole), productRole);
    assert.deepEqual(route.requiredProductRoles, [productRole], productRole);
    assert.equal(route.ribbon.scope, "product", productRole);
    assert.equal(route.ribbon.tab, "courses", productRole);
    assert.equal(
      ribbonSchemaFor("product", productRole).some((slot) => slot.id === "courses"),
      true,
      productRole,
    );
    const model = deriveRibbonModel({ route, params: {} }, { productRole }, {});
    const courses = model.tabs.find((control) => control.id === "courses");
    assert.equal(courses?.selected, true, productRole);
    assert.equal(courses?.href, route.path, productRole);
  }
});

// Permanent contract: the account menu has exactly two authenticated-self
// destinations for every Product Role. A failure means restoring the common
// route contract, rather than creating a role-specific account route.
test("Profile and Account settings are common authenticated-self routes", () => {
  for (const [id, path] of [
    ["profile", "/profile"],
    ["accountSettings", "/account-settings"],
  ]) {
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === id);
    assert.ok(route, id);
    assert.equal(route.path, path, id);
    assert.deepEqual(route.requiredProductRoles, ["student", "instructor", "sysadmin"], id);
    assert.equal(route.ribbon.scope, "product", id);
    assert.equal(route.ribbon.tab, undefined, id);
  }
});
