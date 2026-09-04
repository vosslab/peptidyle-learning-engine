// Live Demo seeded-account sign-in UI contract tests.

import assert from "node:assert/strict";
import test from "node:test";

import { ApiRequestError } from "../src/api/http_client/error.ts";
import {
  isLiveDemoUnavailable,
  seededDemoAvailabilityStatus,
  seededDemoDescription,
} from "../src/pages/live_demo_auth_model.ts";
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

test("degraded seeded entry names only availability and retains usable choices", () => {
  assert.equal(
    seededDemoAvailabilityStatus(1),
    "One demo Account is unavailable. The available choices remain usable.",
  );
  assert.equal(
    seededDemoAvailabilityStatus(3),
    "3 demo Accounts are unavailable. The available choices remain usable.",
  );
});
