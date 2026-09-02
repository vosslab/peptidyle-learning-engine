// e2e_wasm_bridge.mjs - WP-F2/WP-C6 gate: Rust exports are callable from Node.
//
// This is the whole point of the WASM path. `crates/domain` logic has to
// produce identical results on the server and in the browser, and this test is
// the first link in that chain: compile Rust to wasm32, generate glue with
// wasm-bindgen, load it in a JavaScript runtime, call it, and compare against
// a value Rust owns.
//
// It lives in tests/e2e/ rather than tests/ because it needs a real build
// artifact on disk, which puts it outside the fast pytest lane by the rule in
// docs/E2E_TESTS.md.
//
// Run: ./pipeline/build_wasm.sh && node tests/e2e/e2e_wasm_bridge.mjs

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

const bridgePath = path.join(repoRoot, "dist_wasm", "node", "ple_bridge.js");

if (!fs.existsSync(bridgePath)) {
  console.error(`FAIL: ${bridgePath} missing.`);
  console.error("Build it first: ./pipeline/build_wasm.sh");
  process.exit(1);
}

const bridge = await import(bridgePath);

// One committed, answer-free fixture set drives the native Rust, generated Node,
// and real-browser Wasm checks. It belongs to the Wasm package instead of
// `tests/fixtures/` because it is part of this boundary's durable contract.
const { cases: pleQuestionJsonParityCases } = JSON.parse(
  fs.readFileSync(
    path.join(repoRoot, "crates", "wasm", "ple_question_json_response_format_fixture_set.json"),
    "utf8",
  ),
);

for (const parityCase of pleQuestionJsonParityCases) {
  const check = JSON.parse(
    bridge.validate_response_format(
      JSON.stringify(parityCase.definition),
      JSON.stringify(parityCase.response),
    ),
  );
  assert.deepEqual(
    check,
    parityCase.expectedCheck,
    `ple-question-json-v2 Node parity: ${parityCase.name}`,
  );
}

const repeatedCase = pleQuestionJsonParityCases.find(
  ({ name }) => name === "ple-question-json-v2-matching-full-permutation",
);
assert.ok(repeatedCase, "ple-question-json-v2 repeated-call test case is present");
const firstRepeatedCheck = bridge.validate_response_format(
  JSON.stringify(repeatedCase.definition),
  JSON.stringify(repeatedCase.response),
);
const secondRepeatedCheck = bridge.validate_response_format(
  JSON.stringify(repeatedCase.definition),
  JSON.stringify(repeatedCase.response),
);
assert.equal(
  secondRepeatedCheck,
  firstRepeatedCheck,
  "ple-question-json-v2 format validation is stateless",
);

assert.throws(
  () => bridge.validate_response_format("{", "{}"),
  /^invalid Question Response Format:/u,
  "malformed public input keeps its documented JavaScript error category",
);

// The Rust side returns its own CARGO_PKG_VERSION. Comparing against the
// version in crates/wasm/Cargo.toml proves the value crossed the boundary
// rather than being produced by the glue.
const cargoToml = fs.readFileSync(path.join(repoRoot, "crates", "wasm", "Cargo.toml"), "utf8");
const inheritsWorkspaceVersion = /version\.workspace\s*=\s*true/.test(cargoToml);
const workspaceToml = fs.readFileSync(path.join(repoRoot, "Cargo.toml"), "utf8");
const versionSource = inheritsWorkspaceVersion ? workspaceToml : cargoToml;
const versionMatch = versionSource.match(/^version\s*=\s*"([^"]+)"/m);
assert.ok(versionMatch, "could not read the crate version from Cargo.toml");
const expectedVersion = versionMatch[1];

const actualVersion = bridge.bridge_version();
assert.equal(
  actualVersion,
  expectedVersion,
  `bridge_version() returned ${actualVersion}, expected ${expectedVersion}`,
);

const check = JSON.parse(
  bridge.validate_response_format(
    JSON.stringify({ kind: "shortText", matchMode: "normalized", maxLength: 2 }),
    JSON.stringify({ kind: "shortText", text: "abc" }),
  ),
);
assert.deepEqual(check, {
  issues: [{ kind: "textTooLong", maxLength: 2, actualLength: 3 }],
});

const questionAttemptTimingDecision = JSON.parse(
  bridge.question_attempt_timing_decision(
    JSON.stringify({
      policy: { kind: "limited", seconds: 60, graceSeconds: 2 },
      timer: { issuedAt: 1_000, deadline: 10_000, submittedAt: 11_500 },
      evaluatedAt: 11_500,
      pauseExtensionMillis: 0,
    }),
  ),
);
assert.equal(questionAttemptTimingDecision, "submittedWithinGrace");

const fixture = JSON.parse(
  fs.readFileSync(
    path.join(repoRoot, "tests", "fixtures", "published_question", "fixture_set.json"),
    "utf8",
  ),
);
const capabilityViolations = JSON.parse(
  bridge.validate_assignment_config(
    JSON.stringify({
      questions: [
        {
          question: fixture.publishedQuestionRevision,
          questionBackendCapabilities: [],
        },
      ],
      requiredCapabilities: [],
    }),
  ),
);
assert.deepEqual(
  capabilityViolations.map((violation) => violation.capability),
  ["serverGrading"],
);

const draftPreview = JSON.parse(
  bridge.preview_ple_draft(
    JSON.stringify({
      workspace: "00000000-0000-0000-0000-000000000001",
      backendLocator: { backend: "ple" },
      title: "Fixture",
      prompt: [{ kind: "text", markdown: "Value {{value}}" }],
      response: { kind: "shortText", matchMode: "normalized", maxLength: 20 },
      questionVariationRule: {
        kind: "seeded",
        generator: { id: "fixture", version: "1" },
        parameters: { value: { kind: "fixed", value: "safe" } },
      },
    }),
    JSON.stringify(4),
  ),
);
assert.deepEqual(draftPreview, {
  kind: "ready",
  preview: {
    workspace: "00000000-0000-0000-0000-000000000001",
    seed: 4,
    title: "Fixture",
    prompt: [{ kind: "text", markdown: "Value safe" }],
    response: { kind: "shortText", matchMode: "normalized", maxLength: 20 },
  },
});

const presentation = {
  questionRevision: { questionId: "ABC-DEFG", revisionNumber: 1 },
  seed: 42,
  presentationNonce: "11111111111111111111111111111111",
  title: "Peptide bond",
  prompt: [{ kind: "text", markdown: "Which group forms the peptide bond?" }],
  response: {
    kind: "singleChoice",
    choices: [
      { id: "cfdf", body: [{ kind: "text", markdown: "Amino group" }] },
      { id: "6603", body: [{ kind: "text", markdown: "Carboxyl group" }] },
    ],
  },
};
const presentationToken = "pd1_q2fE1ezXCkT6_yd7zeqkCQ";
assert.equal(
  bridge.verify_presentation_descriptor(
    JSON.stringify(presentation),
    JSON.stringify([]),
    presentationToken,
  ),
  true,
  "Wasm must reproduce the native Rust presentation vector",
);
assert.equal(
  bridge.verify_presentation_descriptor(
    JSON.stringify({ ...presentation, title: "Changed title" }),
    JSON.stringify([]),
    presentationToken,
  ),
  false,
  "Wasm must reject a visible presentation mutation",
);

console.log(
  `PASS: WASM bridge ${actualVersion} returned format, timer, capability, draft preview, and presentation results`,
);
