import assert from "node:assert/strict";
import test from "node:test";

import {
  browserTestServerEnabled,
  productionBrowserUse,
  testIgnoreFromEnvironment,
} from "../playwright.config.ts";

test("ordinary browser configuration uses the helper origin without TLS bypass", () => {
  const configured = productionBrowserUse(undefined, undefined);
  assert.deepEqual(configured, {
    baseURL: "http://127.0.0.1:4173",
    ignoreHTTPSErrors: false,
  });
});

test("live-demo browser configuration uses the owner-created HTTPS origin", () => {
  const configured = productionBrowserUse({ baseUrl: "https://localhost:55123/" }, undefined);
  assert.deepEqual(configured, {
    baseURL: "https://localhost:55123/",
    ignoreHTTPSErrors: true,
  });
});

test("WebWork browser configuration keeps its private origin without TLS bypass", () => {
  const configured = productionBrowserUse(undefined, { baseUrl: "https://localhost:55124/" });
  assert.deepEqual(configured, {
    baseURL: "https://localhost:55124/",
    ignoreHTTPSErrors: false,
  });
});

test("live-demo activation leaves the fixed real scenario available without a helper server", () => {
  const environment = { PLE_LIVE_DEMO_BROWSER_REQUIRED: "1" };
  const ignored = testIgnoreFromEnvironment(environment);
  assert.equal(browserTestServerEnabled(environment), false);
  assert.equal(ignored.includes("**/e2e/live_demo.spec.ts"), false);
});
