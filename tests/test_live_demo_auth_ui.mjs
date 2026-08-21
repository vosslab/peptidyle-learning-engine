// WP-PROF-LD2 static browser-authentication route and safety tests.

import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

import { ApiRequestError } from "../src/api/http_client/error.ts";
import {
  isLiveDemoUnavailable,
  seededDemoDescription,
  sysadminOwnershipAvailability,
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

test("the Sysadmin setup page keeps the ownership proof out of browser persistence", () => {
  const source = fs.readFileSync("src/pages/live_demo_sysadmin_setup_page.tsx", "utf8");

  assert.match(
    source,
    /registerLiveDemoSysadminWithBrowser\(runtime\.client, proof, passkeyLabel\(\)\)/u,
  );
  assert.match(
    source,
    /finally \{\s*\/\/ The operator proof[\s\S]*?setOwnershipProof\(""\);\s*\}/u,
  );
  assert.match(source, /ownershipAttemptFailure\(error\)/u);
  assert.match(source, /Administrator setup is already complete/u);
  assert.doesNotMatch(source, /(?:localStorage|sessionStorage|console\.|window\.location)/u);
});

test("seeded account selection stays on the closed selector then shared course-picker path", () => {
  const source = fs.readFileSync("src/pages/sign_in_page.tsx", "utf8");

  assert.match(source, /runtime\.client\.listSeededDemoAccounts\(\)/u);
  assert.match(source, /runtime\.client\.selectSeededDemoAccount\(account\.persona\)/u);
  assert.match(source, /<AccountCoursePicker/u);
  assert.doesNotMatch(source, /startLiveDemoSysadminOwnership|completeLiveDemoSysadminOwnership/u);
});
