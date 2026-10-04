// Semantic route-to-Ribbon presentation boundaries.

import assert from "node:assert/strict";
import test from "node:test";

import { profileRoleMayManageImage } from "../src/features/profile_avatar/profile_avatar_role.ts";
import {
  userRoleHomePath,
  userRoleHomeRouteId,
  userRoleMayAccessRoute,
  ROUTE_CONTRACT,
} from "../src/route_contract.ts";
import { deriveRibbonModel } from "../src/ribbon/ribbon_contract.ts";
import { ribbonSchemaFor } from "../src/ribbon/ribbon_schema.ts";

const USER_ROLES = ["instructor", "student", "sysadmin"];

function tierOneAreaExistsForUserRole(userRole, tierOneArea) {
  return ribbonSchemaFor(userRole).some((slot) => slot.id === tierOneArea);
}

test("declared routes select only Ribbon topology and task areas that exist", () => {
  for (const route of ROUTE_CONTRACT) {
    assert.equal(Object.hasOwn(route.ribbon, "contentLayout"), false, route.id);
    assert.ok(route.pageLayout === undefined || route.pageLayout === "fullWidth", route.id);
    const { tierOneArea } = route.ribbon;
    assert.deepEqual(
      Object.keys(route.ribbon)
        .filter((key) => key !== "tierTwoParent")
        .sort(),
      ["scope", "tierOneArea"],
      route.id,
    );
    if (route.ribbon.tierTwoParent !== undefined) {
      assert.ok(
        [
          "searchQuestionLibrary",
          "myDraftQuestions",
          "myBlueprintCourses",
          "allCoursework",
          "studentAttemptHistory",
        ].includes(route.ribbon.tierTwoParent),
        route.id,
      );
    }

    if (tierOneArea !== "account") {
      const applicableRoles =
        route.requiredUserRoles.length === 0 ? USER_ROLES : route.requiredUserRoles;
      for (const userRole of applicableRoles) {
        assert.equal(
          tierOneAreaExistsForUserRole(userRole, tierOneArea),
          true,
          `${route.id}/${userRole}`,
        );
      }
    }
  }
});

// Permanent contract: each authenticated User Role keeps one explicit home
// destination. Student home opens Coursework; its Courses tab opens the Course list.
test("each User Role has an explicit selected home route", () => {
  for (const userRole of USER_ROLES) {
    const routeId = userRoleHomeRouteId(userRole);
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
    assert.ok(route, userRole);
    assert.equal(route.path, userRoleHomePath(userRole), userRole);
    assert.deepEqual(route.requiredUserRoles, [userRole], userRole);
    assert.equal(route.ribbon.scope, "product", userRole);
    const selectedHomeArea = userRole === "student" ? "coursework" : "courses";
    assert.equal(route.ribbon.tierOneArea, selectedHomeArea, userRole);
    assert.equal(
      ribbonSchemaFor(userRole).some((slot) => slot.id === "courses"),
      true,
      userRole,
    );
    const model = deriveRibbonModel({ route, params: {} }, { userRole }, {});
    const homeTab = model.tabs.find((control) => control.id === selectedHomeArea);
    assert.equal(homeTab?.selected, true, userRole);
    assert.equal(homeTab?.href, route.path, userRole);
  }
});

// Permanent contract: the account menu has exactly two authenticated-self
// destinations for every User Role. A failure means restoring the common
// route contract, rather than creating a role-specific account route.
test("Profile is the only common authenticated-self Account preference route", () => {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === "profile");
  assert.ok(route, "profile");
  assert.equal(route.path, "/profile");
  assert.deepEqual(route.requiredUserRoles, ["student", "instructor", "sysadmin"]);
  assert.equal(route.ribbon.scope, "product");
  assert.equal(route.ribbon.tierOneArea, "account");
  assert.equal(
    ROUTE_CONTRACT.some((candidate) => candidate.path === "/account-settings"),
    false,
  );
});

test("student route access and staff image management follow User Role", () => {
  assert.equal(profileRoleMayManageImage("student"), false);
  assert.equal(profileRoleMayManageImage("instructor"), true);
  assert.equal(profileRoleMayManageImage("sysadmin"), true);
  for (const routeId of ["courseAppearance", "questionDrafts", "questionDraftEditor"]) {
    assert.equal(userRoleMayAccessRoute(routeId, "student"), false, routeId);
    assert.equal(userRoleMayAccessRoute(routeId, "instructor"), true, routeId);
  }
  assert.equal(userRoleMayAccessRoute("profile", "student"), true);
});
