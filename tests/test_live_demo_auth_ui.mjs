// WP-PROF-LD3 direct seeded-role sign-in UI contract tests.

import assert from "node:assert/strict";
import test from "node:test";

import { ApiRequestError } from "../src/api/http_client/error.ts";
import { isLiveDemoUnavailable, seededDemoDescription } from "../src/pages/live_demo_auth_model.ts";
import { routeContractForPathname } from "../src/route_contract.ts";

test("the public sign-in route remains available while the special setup route is retired", () => {
  assert.equal(routeContractForPathname("/sign-in")?.id, "signIn");
  assert.equal(routeContractForPathname("/live-demo/sysadmin-setup"), undefined);
});

test("live-demo account absence remains a deployment absence and persona copy is positive", () => {
  assert.equal(
    isLiveDemoUnavailable(new ApiRequestError(404, "/api/auth/live-demo/accounts")),
    true,
  );
  assert.equal(
    isLiveDemoUnavailable(new ApiRequestError(503, "/api/auth/live-demo/accounts")),
    false,
  );
  assert.match(seededDemoDescription("morganSysadmin"), /seeded Sysadmin account/u);
  assert.match(seededDemoDescription("morganSysadmin"), /administrator tools/u);
});
