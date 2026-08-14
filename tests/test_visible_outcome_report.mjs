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
import { passedStudentCompletionPolicyEvidence } from "./playwright/simulator/student_completion_policy_evidence.ts";
import { passedStudentLeaveResumeEvidence } from "./playwright/simulator/student_leave_resume_evidence.ts";
import { passedW4Fragment } from "./playwright/simulator/instructor_gradebook_j5.ts";
import { passedJ8CrossActorFragment } from "./playwright/simulator/j8_cross_actor_fragment.ts";
import {
  classifyFinalSurface,
  classifyPostStartSurface,
  isFinalSurfaceTerminal,
} from "./playwright/simulator/post_start_surface.ts";

const COURSE = "C-42";
const ASSIGNMENT = "A-73";
const EXAM_ASSIGNMENT = "A-74";
const PROBLEM = "123e4567-e89b-12d3-a456-426614174002";
const VERSION = "123e4567-e89b-12d3-a456-426614174003";
const BASELINE_ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174004";

function arrangements() {
  return [
    {
      label: "api-exam-assignment",
      publicIds: { examAssignmentReference: EXAM_ASSIGNMENT, courseReference: COURSE },
    },
    { label: "launcher-seeded-enrollment", publicIds: {} },
    {
      label: "api-retry-corpus-publication",
      publicIds: { versionId: VERSION, problemId: PROBLEM },
    },
    {
      label: "launcher-baseline-assignment",
      publicIds: { baselineAssignmentId: BASELINE_ASSIGNMENT },
    },
    {
      label: "api-mastery-assignment",
      publicIds: { masteryAssignmentReference: ASSIGNMENT, courseReference: COURSE },
    },
  ];
}

function journeys() {
  const j4 = passedStudentCompletionPolicyEvidence(COURSE, ASSIGNMENT, EXAM_ASSIGNMENT, 14);
  const j5 = passedW4Fragment(COURSE, ASSIGNMENT, 15);
  return [
    passedW1Fragment(COURSE, ASSIGNMENT, 12),
    passedW2Fragment(COURSE, ASSIGNMENT, 13),
    passedStudentLeaveResumeEvidence(COURSE, ASSIGNMENT, 14),
    j4,
    j5,
    passedJ8CrossActorFragment(j4, j5, 0),
  ];
}

test("the first learner run preserves only its canonical visible outcomes", () => {
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

test("the retry run preserves visible outcomes without answer or grading evidence", () => {
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
        {
          label: "api-exam-assignment",
          publicIds: { courseReference: COURSE, examAssignmentReference: EXAM_ASSIGNMENT },
        },
        {
          label: "api-mastery-assignment",
          publicIds: { courseReference: COURSE, masteryAssignmentReference: ASSIGNMENT },
        },
        {
          label: "api-retry-corpus-publication",
          publicIds: { problemId: PROBLEM, versionId: VERSION },
        },
        {
          label: "launcher-baseline-assignment",
          publicIds: { baselineAssignmentId: BASELINE_ASSIGNMENT },
        },
        { label: "launcher-seeded-enrollment", publicIds: {} },
      ],
      journeys: allJourneys,
    }) + "\n",
  );
});

test("the renderer rejects duplicate arrangements and invalid first-run outcomes", () => {
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

test("the direct renderer rejects answer-like records and invalid public references", () => {
  const fragment = passedW1Fragment(COURSE, ASSIGNMENT, 12);
  const correctChoice = arrangements().map((arrangement) =>
    arrangement.label === "api-mastery-assignment"
      ? { ...arrangement, publicIds: { courseReference: COURSE, correctChoice: ASSIGNMENT } }
      : arrangement,
  );
  const uppercase = arrangements().map((arrangement) =>
    arrangement.label === "api-exam-assignment"
      ? {
          ...arrangement,
          publicIds: {
            courseReference: COURSE.toLowerCase(),
            examAssignmentReference: EXAM_ASSIGNMENT,
          },
        }
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

test("post-start classification accepts visible run and fresh-practice surfaces", () => {
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

test("completion classification distinguishes visible transient and terminal outcomes", () => {
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

test("the renderer requires complete ordered public journey evidence", () => {
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

test("retry state handoff rejects unsafe state without following a replacement symlink", () => {
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

test("cross-actor state reading rejects noncanonical state and unsafe parent metadata", () => {
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
