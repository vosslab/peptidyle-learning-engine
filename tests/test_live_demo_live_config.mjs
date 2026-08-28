import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { liveDemoInputsFromEnvironment } from "../tests/playwright/browser_suite_live_config.ts";

function input(overrides = {}) {
  return {
    schemaVersion: 2,
    scenarioId: "direct_role_entry",
    namespace: "bs1-0123456789ab-direct_role_entry",
    baseUrl: "https://localhost:55001/",
    personas: ["morgan_sysadmin"],
    baselineReads: ["genetics_practice_course"],
    visibleObservation: "direct_sysadmin_passkey_reauthentication",
    ...overrides,
  };
}

function parse(value) {
  return liveDemoInputsFromEnvironment(
    {
      PLE_LIVE_DEMO_BROWSER_REQUIRED: "1",
      PLE_LIVE_DEMO_BROWSER_INPUT_FILE: "/private/input.json",
    },
    () => JSON.stringify(value),
    () => undefined,
  );
}

test("live-demo parser accepts direct role input without claim material", () => {
  const parsed = parse(input());
  assert.equal(parsed?.scenarioId, "direct_role_entry");
});

test("live-demo parser accepts descriptive ASCII observation evidence", () => {
  const observation =
    "approved instructors fork Alpha, correct its schedule, apply controlled updates, " +
    "preserve local divergence, and roll over a fresh course";
  assert.equal(parse(input({ visibleObservation: observation }))?.visibleObservation, observation);
  assert.throws(() => parse(input({ visibleObservation: "" })));
  assert.throws(() => parse(input({ visibleObservation: "   " })));
  assert.throws(() => parse(input({ visibleObservation: "visible evidence \u2192 complete" })));
});

test("live-demo parser rejects retired role-claim fields", () => {
  assert.throws(() => parse(input({ sysadminRequirement: "claimed" })));
});

test("live-demo parser binds its namespace, origin, optional service evidence, and screenshot projection", () => {
  const parsed = parse(
    input({
      serviceReceipt: "renderer_delivery",
      faultTransition: "gateway_submit_outage",
      screenshotCapture: {
        version: 1,
        artifacts: [{ artifactId: "account_security", stateId: "passkey_ready" }],
      },
    }),
  );
  assert.deepEqual(parsed?.screenshotCapture, {
    version: 1,
    artifacts: [{ artifactId: "account_security", stateId: "passkey_ready" }],
  });
  assert.equal(
    parse(input({ faultTransition: "deterministic_grader_exception" }))?.faultTransition,
    "deterministic_grader_exception",
  );
  for (const invalid of [
    input({ namespace: "bs1-0123456789ab-other" }),
    input({ baseUrl: "http://localhost:55001/" }),
    input({ serviceReceipt: "unknown" }),
    input({ faultTransition: "unknown" }),
    input({ screenshotCapture: { version: 2, artifacts: [] } }),
    input({
      screenshotCapture: {
        version: 1,
        artifacts: [
          { artifactId: "one", stateId: "state" },
          { artifactId: "one", stateId: "other" },
        ],
      },
    }),
  ]) {
    assert.throws(() => parse(invalid));
  }
});

test("live-demo parser rejects missing and unknown fields", () => {
  const missing = input();
  delete missing.personas;
  assert.throws(() => parse(missing));
  assert.throws(() => parse(input({ unexpected: true })));
});

test("live-demo parser accepts only a bounded regular private input file with exact mode", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-live-input-"));
  const path = join(directory, "input.json");
  const environment = {
    PLE_LIVE_DEMO_BROWSER_REQUIRED: "1",
    PLE_LIVE_DEMO_BROWSER_INPUT_FILE: path,
  };
  try {
    writeFileSync(path, JSON.stringify(input()), { mode: 0o600 });
    chmodSync(path, 0o600);
    assert.equal(liveDemoInputsFromEnvironment(environment)?.scenarioId, "direct_role_entry");
    chmodSync(path, 0o644);
    assert.throws(() => liveDemoInputsFromEnvironment(environment));
    chmodSync(path, 0o600);
    writeFileSync(path, "x".repeat(16_385), { mode: 0o600 });
    assert.throws(() => liveDemoInputsFromEnvironment(environment));
    const link = join(directory, "input-link.json");
    symlinkSync(path, link);
    assert.throws(() =>
      liveDemoInputsFromEnvironment({ ...environment, PLE_LIVE_DEMO_BROWSER_INPUT_FILE: link }),
    );
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});
