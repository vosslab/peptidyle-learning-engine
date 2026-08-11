import assert from "node:assert/strict";
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

import {
  parseW1JourneyFragment,
  parseW2JourneyFragment,
  passedW1Fragment,
  passedW2Fragment,
  renderVisibleOutcomeReport,
} from "./playwright/simulator/visible_outcome_report.ts";
import {
  appendW2JourneyState,
  readJourneyStatePrefix,
} from "./playwright/simulator/journey_state.ts";
import { passedJ3LeaveReturnFragment } from "./playwright/simulator/j3_leave_return_fragment.ts";
import { passedJ4PolicyContrastFragment } from "./playwright/simulator/j4_policy_contrast_fragment.ts";
import { passedW4Fragment } from "./playwright/simulator/instructor_gradebook_j5.ts";
import { passedJ8CrossActorFragment } from "./playwright/simulator/j8_cross_actor_fragment.ts";
import {
  classifyFinalSurface,
  classifyPostStartSurface,
  isFinalSurfaceTerminal,
} from "./playwright/simulator/post_start_surface.ts";

const COURSE = "123e4567-e89b-12d3-a456-426614174000";
const ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174001";
const PROBLEM = "123e4567-e89b-12d3-a456-426614174002";
const VERSION = "123e4567-e89b-12d3-a456-426614174003";
const EXAM = "123e4567-e89b-12d3-a456-426614174004";

function arrangements() {
  return [
    { label: "api-exam-assignment", publicIds: { examAssignmentId: EXAM, courseId: COURSE } },
    { label: "launcher-seeded-enrollment", publicIds: {} },
    {
      label: "api-retry-corpus-publication",
      publicIds: { versionId: VERSION, problemId: PROBLEM },
    },
    { label: "launcher-baseline-assignment", publicIds: { baselineAssignmentId: ASSIGNMENT } },
    {
      label: "api-mastery-assignment",
      publicIds: { masteryAssignmentId: ASSIGNMENT, courseId: COURSE },
    },
  ];
}

function journeys() {
  const j4 = passedJ4PolicyContrastFragment(COURSE, ASSIGNMENT, EXAM, 14);
  const j5 = passedW4Fragment(COURSE, ASSIGNMENT, 15);
  return [
    passedW1Fragment(COURSE, ASSIGNMENT, 12),
    passedW2Fragment(COURSE, ASSIGNMENT, 13),
    passedJ3LeaveReturnFragment(COURSE, ASSIGNMENT, 14),
    j4,
    j5,
    passedJ8CrossActorFragment(j4, j5, 0),
  ];
}

test("W1 PASS preserves only the canonical visible milestone vocabulary", () => {
  const fragment = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  assert.deepEqual(fragment.visibleOutcomeCodes, [
    "visible_completion",
    "visible_feedback",
    "visible_response",
    "visible_start",
    "visible_submit",
  ]);
  assert.deepEqual(parseW1JourneyFragment(fragment), fragment);
});

test("W2 PASS preserves the public retry milestone without answer or grading evidence", () => {
  const fragment = passedW2Fragment(COURSE, ASSIGNMENT, 12);
  assert.deepEqual(fragment.visibleOutcomeCodes, [
    "visible_completion",
    "visible_feedback",
    "visible_response",
    "visible_retry",
    "visible_start",
    "visible_submit",
  ]);
  assert.deepEqual(parseW2JourneyFragment(fragment), fragment);
});

test("report rendering sorts arrangements and retains the public-only outcome record", () => {
  const allJourneys = journeys();
  const rendered = renderVisibleOutcomeReport(42, arrangements(), allJourneys);
  assert.equal(
    rendered,
    JSON.stringify({
      schemaVersion: 1,
      status: "PASS",
      masterSeed: 42,
      stage: "complete",
      elapsedMs: 68,
      arrangements: [
        { label: "api-exam-assignment", publicIds: { courseId: COURSE, examAssignmentId: EXAM } },
        {
          label: "api-mastery-assignment",
          publicIds: { courseId: COURSE, masteryAssignmentId: ASSIGNMENT },
        },
        {
          label: "api-retry-corpus-publication",
          publicIds: { problemId: PROBLEM, versionId: VERSION },
        },
        { label: "launcher-baseline-assignment", publicIds: { baselineAssignmentId: ASSIGNMENT } },
        { label: "launcher-seeded-enrollment", publicIds: {} },
      ],
      journeys: allJourneys,
    }) + "\n",
  );
});

test("the renderer rejects duplicate arrangements and non-PASS J1 outcome vocabulary", () => {
  const fragment = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  assert.equal(
    renderVisibleOutcomeReport(
      42,
      arrangements().map((arrangement) =>
        arrangement.label === "api-exam-assignment"
          ? { ...arrangement, label: "api-mastery-assignment" }
          : arrangement,
      ),
      [fragment, passedW2Fragment(COURSE, ASSIGNMENT, 12)],
    ),
    undefined,
  );
  assert.equal(
    parseW1JourneyFragment({ ...fragment, status: "BLOCKED", diagnostics: [] }),
    undefined,
  );
  assert.equal(parseW1JourneyFragment({ ...fragment, status: "FAIL", diagnostics: [] }), undefined);
  assert.deepEqual(
    parseW1JourneyFragment({
      ...fragment,
      status: "FAIL",
      visibleOutcomeCodes: ["visible_start"],
      diagnostics: ["visible-control-unavailable"],
    })?.status,
    "FAIL",
  );
});

test("the direct renderer rejects answer-like records and uppercase public identifiers", () => {
  const fragment = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  const correctChoice = arrangements().map((arrangement) =>
    arrangement.label === "api-mastery-assignment"
      ? { ...arrangement, publicIds: { courseId: COURSE, correctChoice: ASSIGNMENT } }
      : arrangement,
  );
  const uppercase = arrangements().map((arrangement) =>
    arrangement.label === "api-exam-assignment"
      ? { ...arrangement, publicIds: { courseId: COURSE.toUpperCase(), examAssignmentId: EXAM } }
      : arrangement,
  );
  assert.equal(
    renderVisibleOutcomeReport(42, correctChoice, [
      fragment,
      passedW2Fragment(COURSE, ASSIGNMENT, 12),
    ]),
    undefined,
  );
  assert.equal(
    renderVisibleOutcomeReport(42, uppercase, [fragment, passedW2Fragment(COURSE, ASSIGNMENT, 12)]),
    undefined,
  );
});

test("J1 uses the platform keyboard for rendered local sign-in", () => {
  const source = readFileSync("tests/playwright/ui_walkthrough_keyboard_j1.spec.ts", "utf8");
  assert.doesNotMatch(source, /\.click\(/u);
  assert.match(source, /simulator\/keyboard_walkthrough/u);
  assert.match(source, /tabTo\(page, credentialInput\)/u);
  assert.match(source, /tabTo\(page, signIn\)/u);
  assert.match(source, /expect\(assignmentLink\)\.toBeVisible\(\)/u);
  assert.match(source, /tabTo\(page, assignmentLink, "backward"\)/u);
  assert.match(source, /locator\('input\[type="radio"\]:visible'\)/u);
  assert.match(source, /expect\(radios\)\.toHaveCount\(2\)/u);
  assert.match(source, /expect\(radios\.nth\(0\)\)\.not\.toBeChecked\(\)/u);
  assert.match(source, /expect\(radios\.nth\(1\)\)\.not\.toBeChecked\(\)/u);
  assert.match(source, /tabTo\(page, response, "backward"\)/u);
  assert.match(source, /getByRole\("button", \{ name: "Start another practice run" \}\)/u);
  assert.doesNotMatch(source, /for \(let visibleAttempt/u);
  assert.doesNotMatch(source, /Keep practicing with a fresh variation/u);
  assert.match(source, /keyboard\.press\("Enter"\)/u);
});

test("J2 uses only rendered keyboard controls and never inspects answers or feedback text", () => {
  const source = readFileSync("tests/playwright/ui_walkthrough_keyboard_j2.spec.ts", "utf8");
  assert.doesNotMatch(source, /\.click\(/u);
  assert.doesNotMatch(source, /correct|answerKey|feedback\.text|innerText/u);
  assert.match(source, /chooseAndSubmit\(page, 0\)/u);
  assert.match(source, /chooseAndSubmit\(page, 1\)/u);
  assert.match(source, /waitForPostStartSurface\(/u);
  assert.match(source, /getByRole\("button", \{ name: "Start another practice run" \}\)/u);
  assert.doesNotMatch(source, /Keep practicing with a fresh variation/u);
  assert.match(source, /retrySurface !== "run"/u);
  assert.match(source, /timeout: 30_000/u);
  assert.match(source, /test\.setTimeout\(90_000\)/u);
  assert.match(source, /finalSurface === "run"/u);
  assert.match(source, /final fresh-practice control did not appear in time/u);
  assert.match(source, /waitForFinalSurface\(page\)/u);
  assert.match(source, /final Feedback or Continue did not leave/u);
  assert.match(source, /isFinalSurfaceTerminal\(surface\)/u);
  assert.match(source, /getByRole\("heading", \{ name: "Run complete", exact: true \}\)/u);
  assert.match(
    source,
    /locator\("\.question-card"\)\s*\.getByRole\("heading", \{ name: "Feedback"/u,
  );
  assert.match(source, /if \(!isPollTimeout\(error\)\) throw error/u);
  assert.match(source, /expect\(assignmentLink\)\.toBeVisible\(\)/u);
  assert.match(source, /tabTo\(page, assignmentLink, "backward"\)/u);
  assert.match(source, /locator\('input\[type="radio"\]:visible'\)/u);
  assert.match(source, /expect\(radios\)\.toHaveCount\(2\)/u);
  assert.match(source, /expect\(radios\.nth\(0\)\)\.not\.toBeChecked\(\)/u);
  assert.match(source, /choiceIndex === 0 \? "forward" : "backward"/u);
  const helper = readFileSync("tests/playwright/simulator/keyboard_walkthrough.ts", "utf8");
  assert.match(helper, /direction: TabDirection = "forward"/u);
  assert.match(helper, /"Shift\+Tab"/u);
});

test("J2 accepts both rendered Mastery post-start surfaces and fails on an inline error", () => {
  assert.equal(
    classifyPostStartSurface({ radios: 2, freshPractice: false, inlineErrors: 0 }),
    "run",
  );
  assert.equal(
    classifyPostStartSurface({ radios: 0, freshPractice: true, inlineErrors: 0 }),
    "fresh-practice",
  );
  assert.equal(
    classifyPostStartSurface({ radios: 0, freshPractice: false, inlineErrors: 1 }),
    "error",
  );
  assert.equal(
    classifyPostStartSurface({ radios: 0, freshPractice: false, inlineErrors: 0 }),
    "pending",
  );
});

test("J2 final surface classifier distinguishes visible non-fresh outcomes", () => {
  const base = {
    radios: 0,
    freshPractice: false,
    inlineErrors: 0,
    continueVisible: false,
    feedbackVisible: false,
    neutralComplete: false,
    closedComplete: false,
  };
  assert.equal(classifyFinalSurface({ ...base, freshPractice: true }), "fresh-practice");
  assert.equal(classifyFinalSurface({ ...base, radios: 2 }), "run");
  assert.equal(classifyFinalSurface({ ...base, inlineErrors: 1 }), "error");
  assert.equal(classifyFinalSurface({ ...base, feedbackVisible: true }), "feedback");
  assert.equal(classifyFinalSurface({ ...base, neutralComplete: true }), "neutral");
  assert.equal(classifyFinalSurface({ ...base, closedComplete: true }), "closed");
  assert.equal(classifyFinalSurface(base), "pending");
  assert.equal(isFinalSurfaceTerminal("feedback"), false);
  assert.equal(isFinalSurfaceTerminal("pending"), false);
  assert.equal(isFinalSurfaceTerminal("neutral"), false);
  assert.equal(isFinalSurfaceTerminal("fresh-practice"), true);
});

test("the validator fails closed on answer text, selectors, and incomplete PASS evidence", () => {
  const fragment = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  assert.equal(parseW1JourneyFragment({ ...fragment, answer: "nitrogen" }), undefined);
  assert.equal(
    parseW1JourneyFragment({ ...fragment, diagnostics: ["button-submit-answer"] }),
    undefined,
  );
  assert.equal(
    parseW1JourneyFragment({ ...fragment, visibleOutcomeCodes: ["visible_start"] }),
    undefined,
  );
});

test("the renderer requires exactly ordered J1 through J8 fragments", () => {
  const w1 = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  const w2 = passedW2Fragment(COURSE, ASSIGNMENT, 13);
  assert.equal(renderVisibleOutcomeReport(42, arrangements(), [w1]), undefined);
  assert.equal(
    renderVisibleOutcomeReport(42, arrangements(), [w2, w1, ...journeys().slice(2)]),
    undefined,
  );
  assert.equal(
    renderVisibleOutcomeReport(42, arrangements(), [
      w1,
      { ...w2, answer: "x" },
      ...journeys().slice(2),
    ]),
    undefined,
  );
});

test("the renderer rejects null, hidden, symbol, and answer-like hostile records without throwing", () => {
  const w1 = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  const w2 = passedW2Fragment(COURSE, ASSIGNMENT, 13);
  const hidden = { ...w1 };
  Object.defineProperty(hidden, "answer", { enumerable: false, value: "x" });
  const symbolArrangement = arrangements();
  symbolArrangement[0][Symbol("private")] = "x";
  assert.doesNotThrow(() => renderVisibleOutcomeReport(42, null, null));
  assert.equal(renderVisibleOutcomeReport(42, [null], [w1, w2]), undefined);
  assert.equal(renderVisibleOutcomeReport(42, arrangements(), [hidden, w2]), undefined);
  assert.equal(renderVisibleOutcomeReport(42, symbolArrangement, [w1, w2]), undefined);
});

test("J2 state handoff rejects unsafe state without following a replacement symlink", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-w2-state-"));
  const state = join(directory, "journeys.json");
  const outside = join(directory, "outside.json");
  try {
    chmodSync(directory, 0o700);
    writeFileSync(state, JSON.stringify([passedW1Fragment(COURSE, ASSIGNMENT, 12)]) + "\n", {
      encoding: "ascii",
      mode: 0o600,
    });
    appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13));
    assert.match(readFileSync(state, "ascii"), /"J2"/u);
    writeFileSync(outside, "outside", { encoding: "ascii", mode: 0o600 });
    unlinkSync(state);
    symlinkSync(outside, state);
    assert.throws(() => appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13)));
    assert.equal(readFileSync(outside, "ascii"), "outside");
    unlinkSync(state);
    writeFileSync(state, "[]\n", { encoding: "ascii", mode: 0o600 });
    assert.throws(() => appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13)));
    writeFileSync(state, '["\u00e9"]\n', { encoding: "utf8", mode: 0o600 });
    assert.throws(() => appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13)));
    writeFileSync(state, "x".repeat(4097), { encoding: "ascii", mode: 0o600 });
    assert.throws(() => appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13)));
    chmodSync(state, 0o644);
    assert.throws(() => appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13)));
    chmodSync(state, 0o600);
    chmodSync(directory, 0o755);
    assert.throws(() => appendW2JourneyState(state, passedW2Fragment(COURSE, ASSIGNMENT, 13)));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("the J8 reader rejects noncanonical state and unsafe parent metadata", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-j8-state-"));
  const state = join(directory, "journeys.json");
  try {
    chmodSync(directory, 0o700);
    writeFileSync(state, "[ ]\n", { encoding: "ascii", mode: 0o600 });
    assert.throws(() => readJourneyStatePrefix(state));
    writeFileSync(state, JSON.stringify([passedW1Fragment(COURSE, ASSIGNMENT, 1)]) + "\n", {
      encoding: "ascii",
      mode: 0o600,
    });
    chmodSync(directory, 0o755);
    assert.throws(() => readJourneyStatePrefix(state));
    chmodSync(directory, 0o700);
    unlinkSync(state);
    symlinkSync("outside.json", state);
    assert.throws(() => readJourneyStatePrefix(state));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("the fixed final renderer accepts canonical state and leaks nothing for hostile state", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-report-child-"));
  const state = join(directory, "journeys.json");
  const run = () =>
    spawnSync(
      process.execPath,
      ["node_modules/tsx/dist/cli.mjs", "tests/e2e/ui_walkthrough_report.ts"],
      {
        cwd: process.cwd(),
        encoding: "utf8",
        env: {
          ...process.env,
          PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE: state,
          PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON: JSON.stringify(arrangements()),
          PLE_UI_WALKTHROUGH_MASTER_SEED: "42",
        },
      },
    );
  try {
    chmodSync(directory, 0o700);
    writeFileSync(state, JSON.stringify(journeys()) + "\n", { encoding: "ascii", mode: 0o600 });
    const valid = run();
    assert.equal(valid.status, 0);
    assert.match(valid.stdout, /"journey":"J8"/u);
    unlinkSync(state);
    symlinkSync("outside.json", state);
    const hostile = run();
    assert.notEqual(hostile.status, 0);
    assert.equal(hostile.stdout, "");
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
