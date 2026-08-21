import assert from "node:assert/strict";
import test from "node:test";

import { liveDemoInputsFromEnvironment } from "../tests/playwright/live_demo_live_config.ts";
import { liveModeActivationFromEnvironment } from "../tests/playwright/live_mode_activation.ts";

const proof = "A".repeat(42) + "E";
const valid = JSON.stringify({
  schemaVersion: 1,
  baseUrl: "https://localhost:55123/",
  sysadminOwnershipProof: proof,
});

test("live-demo parser accepts the exact private TLS-origin ABI", () => {
  const inputs = liveDemoInputsFromEnvironment(
    { PLE_LIVE_DEMO_BROWSER_REQUIRED: "1", PLE_LIVE_DEMO_BROWSER_INPUT_FILE: "/private/input" },
    () => valid,
    () => {},
  );
  assert.equal(inputs?.baseUrl, "https://localhost:55123/");
});

test("live-demo parser rejects a non-localhost origin and malformed proof", () => {
  assert.throws(() =>
    liveDemoInputsFromEnvironment(
      { PLE_LIVE_DEMO_BROWSER_REQUIRED: "1", PLE_LIVE_DEMO_BROWSER_INPUT_FILE: "/private/input" },
      () =>
        JSON.stringify({
          schemaVersion: 1,
          baseUrl: "https://127.0.0.1:55123/",
          sysadminOwnershipProof: proof,
        }),
      () => {},
    ),
  );
  assert.throws(() =>
    liveDemoInputsFromEnvironment(
      { PLE_LIVE_DEMO_BROWSER_REQUIRED: "1", PLE_LIVE_DEMO_BROWSER_INPUT_FILE: "/private/input" },
      () =>
        JSON.stringify({
          schemaVersion: 1,
          baseUrl: "https://localhost:55123/",
          sysadminOwnershipProof: "short",
        }),
      () => {},
    ),
  );
});

test("live modes are mutually exclusive", () => {
  assert.throws(() =>
    liveModeActivationFromEnvironment({
      PLE_WEBWORK_LIVE_REQUIRED: "1",
      PLE_LIVE_DEMO_BROWSER_REQUIRED: "1",
    }),
  );
});
