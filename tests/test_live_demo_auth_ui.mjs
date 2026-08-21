// WP-PROF-LD2 static browser-authentication route and safety tests.

import assert from "node:assert/strict";
import test from "node:test";

import { ApiRequestError } from "../src/api/http_client/error.ts";
import {
  clearSysadminOwnershipProof,
  isLiveDemoUnavailable,
  seededDemoDescription,
  sysadminCourseFailure,
  sysadminOwnershipFailure,
  sysadminOwnershipAvailability,
  sysadminSetupBusyMessage,
  sysadminSetupErrorMessage,
  sysadminSetupFormVisible,
  sysadminSetupRetry,
} from "../src/pages/live_demo_auth_model.ts";
import { rolesMayAccessRoute, routeContractForPathname } from "../src/route_contract.ts";
import { courseThemeRouteRequest } from "../src/features/course_appearance/course_theme_route.ts";

test("the operator-discovered Sysadmin setup route remains anonymous and globally themed", () => {
  const route = routeContractForPathname("/live-demo/sysadmin-setup");

  assert.equal(route?.id, "liveDemoSysadminSetup");
  assert.deepEqual(route?.requiredRoles, []);
  assert.equal(rolesMayAccessRoute("liveDemoSysadminSetup", ["student"]), true);
  assert.deepEqual(courseThemeRouteRequest("/live-demo/sysadmin-setup"), { kind: "global" });
});

test("live-demo availability keeps 404 absent and maps safe ownership states", () => {
  assert.equal(
    isLiveDemoUnavailable(new ApiRequestError(404, "/api/auth/live-demo/accounts")),
    true,
  );
  assert.equal(
    isLiveDemoUnavailable(new ApiRequestError(503, "/api/auth/live-demo/accounts")),
    false,
  );
  assert.equal(sysadminOwnershipAvailability(true), "ready");
  assert.equal(sysadminOwnershipAvailability(false), "complete");
  assert.match(seededDemoDescription("elenaInstructor"), /seeded account/u);
});

test("post-claim course failures close ownership setup and focus a course-list retry", () => {
  const failedCourseLoad = sysadminCourseFailure("list");
  const failedCourseSelection = sysadminCourseFailure("select");

  assert.equal(sysadminSetupFormVisible(failedCourseLoad), false);
  assert.equal(sysadminSetupRetry(failedCourseLoad), "courses");
  assert.match(sysadminSetupErrorMessage(failedCourseLoad), /passkey is ready/u);
  assert.equal(sysadminSetupRetry(failedCourseSelection), "courses");
  assert.match(sysadminSetupErrorMessage(failedCourseSelection), /Reload your course list/u);
  assert.equal(clearSysadminOwnershipProof(), "");
});

test("only ownership failures reopen the claim form, while a stale claim is terminal", () => {
  const invalidProof = sysadminOwnershipFailure(
    new ApiRequestError(403, "/api/auth/live-demo/sysadmin/ownership"),
  );
  const alreadyClaimed = sysadminOwnershipFailure(
    new ApiRequestError(409, "/api/auth/live-demo/sysadmin/ownership"),
  );

  assert.equal(sysadminSetupFormVisible(invalidProof), true);
  assert.equal(sysadminSetupFormVisible(alreadyClaimed), false);
  assert.equal(
    sysadminSetupBusyMessage({ kind: "courses-busy", message: "Opening your account..." }),
    "Opening your account...",
  );
});
