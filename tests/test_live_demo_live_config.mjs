import assert from "node:assert/strict";
import test from "node:test";

import { liveDemoInputsFromEnvironment } from "../tests/playwright/browser_suite_live_config.ts";

const proof = "A".repeat(42) + "E";

function validInput(overrides = {}) {
  return {
    schemaVersion: 2,
    scenarioId: "live_demo",
    namespace: "bs1-0123456789ab-live_demo",
    baseUrl: "https://localhost:55123/",
    personas: ["elena_instructor", "mary_student", "avery_student", "morgan_sysadmin"],
    baselineReads: [
      "base_course",
      "genetics_practice_course",
      "mary_completed_run",
      "jack_open_run",
      "published_peptide_assignment",
    ],
    sysadminRequirement: "unclaimed",
    visibleObservation: "avery_teaching_team_access_after_reauthentication",
    sysadminOwnershipProof: proof,
    ...overrides,
  };
}

function parse(contents) {
  return liveDemoInputsFromEnvironment(
    { PLE_LIVE_DEMO_BROWSER_REQUIRED: "1", PLE_LIVE_DEMO_BROWSER_INPUT_FILE: "/private/input" },
    () => contents,
    () => {},
  );
}

test("live-demo parser accepts the exact V2 private TLS-origin ABI", () => {
  const inputs = parse(JSON.stringify(validInput()));
  assert.equal(inputs?.baseUrl, "https://localhost:55123/");
  assert.equal(inputs?.namespace, "bs1-0123456789ab-live_demo");
});

test("live-demo parser rejects extensions, omissions, noncanonical JSON, and mismatched scenario fields", () => {
  const extra = validInput({ extra: true });
  assert.throws(() => parse(JSON.stringify(extra)));
  const missing = validInput();
  delete missing.baselineReads;
  assert.throws(() => parse(JSON.stringify(missing)));
  assert.throws(() => parse(`${JSON.stringify(validInput())}\n`));
  const reordered = validInput();
  const reorderedContents = JSON.stringify({ baseUrl: reordered.baseUrl, ...reordered });
  assert.throws(() => parse(reorderedContents));
  assert.throws(() => parse(JSON.stringify(validInput({ namespace: "bs1-0123456789ab-other" }))));
  assert.throws(() => parse(JSON.stringify(validInput({ baseUrl: "https://127.0.0.1:55123/" }))));
});

test("live-demo parser binds the ownership proof to the unclaimed first-claim transition", () => {
  const claimed = validInput({ sysadminRequirement: "claimed" });
  assert.throws(() => parse(JSON.stringify(claimed)));
  const syntheticClaimed = validInput({
    scenarioId: "synthetic_claimed",
    namespace: "bs1-0123456789ab-synthetic_claimed",
    sysadminRequirement: "claimed",
  });
  delete syntheticClaimed.sysadminOwnershipProof;
  assert.equal(parse(JSON.stringify(syntheticClaimed))?.scenarioId, "synthetic_claimed");
  assert.throws(() => parse(JSON.stringify({ ...syntheticClaimed, sysadminOwnershipProof: null })));
  const unclaimed = validInput();
  delete unclaimed.sysadminOwnershipProof;
  assert.throws(() => parse(JSON.stringify(unclaimed)));
  assert.throws(() => parse(JSON.stringify(validInput({ sysadminOwnershipProof: "short" }))));
  assert.throws(() =>
    parse(JSON.stringify(validInput({ sysadminOwnershipProof: `${proof.slice(0, -1)}F` }))),
  );
  assert.throws(() => parse(JSON.stringify(validInput({ serviceReceipt: null }))));
});

test("V2 parser accepts only closed service receipt identifiers", () => {
  const base = validInput();
  const valid = {
    schemaVersion: base.schemaVersion,
    scenarioId: base.scenarioId,
    namespace: base.namespace,
    baseUrl: base.baseUrl,
    personas: base.personas,
    baselineReads: base.baselineReads,
    sysadminRequirement: base.sysadminRequirement,
    visibleObservation: base.visibleObservation,
    serviceReceipt: "worker_completion",
    sysadminOwnershipProof: base.sysadminOwnershipProof,
  };
  assert.equal(parse(JSON.stringify(valid))?.serviceReceipt, "worker_completion");
  for (const serviceReceipt of [null, 9, "", "invalid receipt", "unknown_receipt"]) {
    assert.throws(() => parse(JSON.stringify(validInput({ serviceReceipt }))));
  }
});

test("V2 parser accepts only the owner-declared gateway fault transition", () => {
  const base = validInput({
    scenarioId: "learner_gateway_recovery",
    namespace: "bs1-0123456789ab-learner_gateway_recovery",
    sysadminRequirement: "not_required",
  });
  delete base.sysadminOwnershipProof;
  const valid = {
    schemaVersion: base.schemaVersion,
    scenarioId: base.scenarioId,
    namespace: base.namespace,
    baseUrl: base.baseUrl,
    personas: base.personas,
    baselineReads: base.baselineReads,
    sysadminRequirement: base.sysadminRequirement,
    visibleObservation: base.visibleObservation,
    faultTransition: "gateway_submit_outage",
  };
  assert.equal(parse(JSON.stringify(valid))?.faultTransition, "gateway_submit_outage");
  assert.throws(() => parse(JSON.stringify({ ...valid, faultTransition: "renderer_outage" })));
});

test("V2 parser admits the full visual corpus input while retaining a bounded private ABI", () => {
  const base = validInput({
    scenarioId: "learner_delivery",
    namespace: "bs1-0123456789ab-learner_delivery",
    personas: ["elena_instructor", "mary_student"],
    baselineReads: ["base_course"],
    sysadminRequirement: "not_required",
    visibleObservation: "learner_delivery",
  });
  delete base.sysadminOwnershipProof;
  const artifacts = Array.from({ length: 64 }, (_, index) => ({
    artifactId: `capture_${index + 1}`,
    stateId: `state_${index + 1}`,
  }));
  const input = { ...base, screenshotCapture: { version: 1, artifacts } };
  assert.equal(parse(JSON.stringify(input))?.screenshotCapture?.artifacts.length, 64);
  assert.throws(() =>
    parse(
      JSON.stringify({
        ...base,
        screenshotCapture: {
          version: 1,
          artifacts: [...artifacts, { artifactId: "capture_65", stateId: "state_65" }],
        },
      }),
    ),
  );
  assert.throws(() =>
    parse(
      JSON.stringify({
        ...base,
        screenshotCapture: {
          version: 1,
          artifacts: [...artifacts.slice(0, 63), artifacts[0]],
        },
      }),
    ),
  );
});
