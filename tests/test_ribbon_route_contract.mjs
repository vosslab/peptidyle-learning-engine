// Semantic route-to-Ribbon presentation boundaries.

import assert from "node:assert/strict";
import test from "node:test";

import {
  productRoleHomePath,
  productRoleHomeRouteId,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { ribbonSchemaFor } from "../src/ribbon/ribbon_schema.ts";

const PRODUCT_ROLES = ["instructor", "student", "sysadmin"];

function tierOneAreaExistsForProductRole(productRole, tierOneArea) {
  return ribbonSchemaFor(productRole).some((slot) => slot.id === tierOneArea);
}

test("declared routes select only Ribbon topology and task areas that exist", () => {
  for (const route of ROUTE_CONTRACT) {
    assert.equal(Object.hasOwn(route.ribbon, "contentLayout"), false, route.id);
    assert.ok(route.pageLayout === undefined || route.pageLayout === "fullWidth", route.id);
    const { tierOneArea } = route.ribbon;
    assert.deepEqual(Object.keys(route.ribbon).sort(), ["scope", "tierOneArea"]);

    if (tierOneArea !== "account") {
      const applicableRoles =
        route.requiredProductRoles.length === 0 ? PRODUCT_ROLES : route.requiredProductRoles;
      for (const productRole of applicableRoles) {
        assert.equal(
          tierOneAreaExistsForProductRole(productRole, tierOneArea),
          true,
          `${route.id}/${productRole}`,
        );
      }
    }
  }
});

// Permanent contract: each authenticated Product Role keeps one explicit home
// destination. Student home opens Coursework; its Courses tab opens the Course list.
test("each Product Role has an explicit selected home route", () => {
  for (const productRole of PRODUCT_ROLES) {
    const routeId = productRoleHomeRouteId(productRole);
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
    assert.ok(route, productRole);
    assert.equal(route.path, productRoleHomePath(productRole), productRole);
    assert.deepEqual(route.requiredProductRoles, [productRole], productRole);
    assert.equal(route.ribbon.scope, "product", productRole);
    const selectedHomeArea = productRole === "student" ? "coursework" : "courses";
    assert.equal(route.ribbon.tierOneArea, selectedHomeArea, productRole);
    assert.equal(
      ribbonSchemaFor(productRole).some((slot) => slot.id === "courses"),
      true,
      productRole,
    );
    const model = deriveRibbonModel({ route, params: {} }, { productRole }, {});
    const homeTab = model.tabs.find((control) => control.id === selectedHomeArea);
    assert.equal(homeTab?.selected, true, productRole);
    assert.equal(homeTab?.href, route.path, productRole);
  }
});

// Permanent contract: the account menu has exactly two authenticated-self
// destinations for every Product Role. A failure means restoring the common
// route contract, rather than creating a role-specific account route.
test("Profile is the only common authenticated-self Account preference route", () => {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === "profile");
  assert.ok(route, "profile");
  assert.equal(route.path, "/profile");
  assert.deepEqual(route.requiredProductRoles, ["student", "instructor", "sysadmin"]);
  assert.equal(route.ribbon.scope, "product");
  assert.equal(route.ribbon.tierOneArea, "account");
  assert.equal(
    ROUTE_CONTRACT.some((candidate) => candidate.path === "/account-settings"),
    false,
  );
});
