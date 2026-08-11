// v2_visible_outcome_report.spec.ts - hostile validation fixtures for the isolated v2 boundary.

import { expect, test } from "@playwright/test";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  renameSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

import {
  parseV2WalkthroughState,
  passedV2J8Fragment,
  readV2WalkthroughState,
  renderV2VisibleOutcomeReport,
  setV2StateOpenHookForTest,
} from "./v2_visible_outcome_report";

const COURSE_ID = "123e4567-e89b-12d3-a456-426614174000";
const ASSIGNMENT_ID = "123e4567-e89b-12d3-a456-426614174001";
const PROBLEM_ID = "123e4567-e89b-12d3-a456-426614174002";
const VERSION_ID = "123e4567-e89b-12d3-a456-426614174003";

function validState(): unknown {
  return [
    fragment("J11", ["visible_course_created", "visible_course_opened"]),
    fragment("J12", ["visible_local_student_active"]),
    fragment("J13", [
      "visible_assignment_created",
      "visible_catalog_problem_selected",
      "visible_mastery_policy",
    ]),
    fragment("J1", ["visible_feedback", "visible_response", "visible_retry", "visible_submit"]),
    fragment("J2", [
      "visible_completion",
      "visible_feedback",
      "visible_fresh_practice",
      "visible_submit",
    ]),
    fragment("J3", [
      "visible_controls_cleared",
      "visible_leave",
      "visible_resume",
      "visible_start",
    ]),
    fragment("J4", [
      "visible_back_action",
      "visible_completion",
      "visible_controls_cleared",
      "visible_submit",
    ]),
    fragment("J5", ["visible_gradebook", "visible_score_summary", "visible_two_run_history"]),
    fragment("J8", [
      "visible_instructor_gradebook",
      "visible_learner_completion",
      "visible_shared_assignment",
    ]),
  ];
}

function fragment(
  journey: string,
  visibleOutcomeCodes: readonly string[],
): Record<string, unknown> {
  const base: Record<string, unknown> = {
    schemaVersion: 2,
    journey,
    status: "PASS",
    elapsedMs: 1,
    courseId: COURSE_ID,
    visibleOutcomeCodes,
    diagnostics: [],
  };
  if (journey === "J13") {
    base["assignmentId"] = ASSIGNMENT_ID;
    base["problemId"] = PROBLEM_ID;
    base["versionId"] = VERSION_ID;
  } else if (journey !== "J11" && journey !== "J12") {
    base["assignmentId"] = ASSIGNMENT_ID;
  }
  return base;
}

test("v2 state renders exactly the corrected public no-email pilot report", () => {
  const state = parseV2WalkthroughState(validState());
  expect(state).toBeDefined();
  if (state === undefined) throw new Error("test fixture must parse");
  const rendered = renderV2VisibleOutcomeReport(42, state);
  expect(rendered).toBeDefined();
  expect(rendered).not.toMatch(
    /123e4567|100%|learnerName|studentName|runId|email|problemId|versionId/iu,
  );
  const report = JSON.parse(rendered ?? "") as unknown;
  expect(report).toEqual({
    schemaVersion: 2,
    status: "PASS",
    masterSeed: 42,
    stage: "complete",
    elapsedMs: 9,
    arrangements: [{ label: "api-retry-corpus-publication" }],
    journeys: [
      {
        journey: "J11",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: ["visible_course_created", "visible_course_opened"],
        diagnostics: [],
      },
      {
        journey: "J12",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: ["visible_local_student_active"],
        diagnostics: [],
      },
      {
        journey: "J13",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_assignment_created",
          "visible_catalog_problem_selected",
          "visible_mastery_policy",
        ],
        diagnostics: [],
      },
      {
        journey: "J1",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_feedback",
          "visible_response",
          "visible_retry",
          "visible_submit",
        ],
        diagnostics: [],
      },
      {
        journey: "J2",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_completion",
          "visible_feedback",
          "visible_fresh_practice",
          "visible_submit",
        ],
        diagnostics: [],
      },
      {
        journey: "J3",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_controls_cleared",
          "visible_leave",
          "visible_resume",
          "visible_start",
        ],
        diagnostics: [],
      },
      {
        journey: "J4",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_back_action",
          "visible_completion",
          "visible_controls_cleared",
          "visible_submit",
        ],
        diagnostics: [],
      },
      {
        journey: "J5",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_gradebook",
          "visible_score_summary",
          "visible_two_run_history",
        ],
        diagnostics: [],
      },
      {
        journey: "J8",
        status: "PASS",
        elapsedMs: 1,
        visibleOutcomeCodes: [
          "visible_instructor_gradebook",
          "visible_learner_completion",
          "visible_shared_assignment",
        ],
        diagnostics: [],
      },
    ],
  });
});

test("v2 state rejects reordered, cross-bound, and score-bearing evidence", () => {
  const reordered = validState() as unknown[];
  [reordered[0], reordered[1]] = [reordered[1], reordered[0]];
  expect(parseV2WalkthroughState(reordered)).toBeUndefined();

  const mismatched = validState() as Record<string, unknown>[];
  (mismatched[7] as Record<string, unknown>)["assignmentId"] =
    "123e4567-e89b-12d3-a456-426614174004";
  expect(parseV2WalkthroughState(mismatched)).toBeUndefined();

  const scoreBearing = validState() as Record<string, unknown>[];
  (scoreBearing[7] as Record<string, unknown>)["score"] = "100%";
  expect(parseV2WalkthroughState(scoreBearing)).toBeUndefined();
});

test("v2 state rejects hidden, symbol, inherited, and accessor input", () => {
  const hidden = validState() as Record<string, unknown>[];
  Object.defineProperty(hidden[0] ?? {}, "hidden", { value: "forbidden" });
  expect(parseV2WalkthroughState(hidden)).toBeUndefined();

  const symbolBearing = validState() as Record<string, unknown>[];
  Object.defineProperty(symbolBearing[1] ?? {}, Symbol("forbidden"), {
    value: "forbidden",
    enumerable: true,
  });
  expect(parseV2WalkthroughState(symbolBearing)).toBeUndefined();

  const inherited = validState() as Record<string, unknown>[];
  Object.setPrototypeOf(inherited[2] ?? {}, { assignmentId: ASSIGNMENT_ID });
  expect(parseV2WalkthroughState(inherited)).toBeUndefined();

  const accessor = validState() as Record<string, unknown>[];
  Object.defineProperty(accessor[3] ?? {}, "courseId", { enumerable: true, get: () => COURSE_ID });
  expect(parseV2WalkthroughState(accessor)).toBeUndefined();
});

test("renderer revalidates direct hostile inputs and bounds the master seed", () => {
  const hidden = validState() as Record<string, unknown>[];
  Object.defineProperty(hidden[0] ?? {}, "hidden", { value: "forbidden" });
  expect(renderV2VisibleOutcomeReport(42, hidden)).toBeUndefined();
  expect(renderV2VisibleOutcomeReport(4_294_967_296, validState())).toBeUndefined();
  expect(renderV2VisibleOutcomeReport(-1, validState())).toBeUndefined();
});

test("J8 builder carries only public identifiers and its closed cross-actor vocabulary", () => {
  expect(passedV2J8Fragment(COURSE_ID, ASSIGNMENT_ID, 12)).toEqual({
    schemaVersion: 2,
    journey: "J8",
    status: "PASS",
    elapsedMs: 12,
    courseId: COURSE_ID,
    assignmentId: ASSIGNMENT_ID,
    visibleOutcomeCodes: [
      "visible_instructor_gradebook",
      "visible_learner_completion",
      "visible_shared_assignment",
    ],
    diagnostics: [],
  });
});

test("v2 protected-state reader rejects unsafe mode and symlink paths", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-v2-state-"));
  try {
    chmodSync(directory, 0o700);
    const statePath = join(directory, "journeys.json");
    writeFileSync(statePath, JSON.stringify(validState()) + "\n", {
      encoding: "ascii",
      mode: 0o600,
    });
    expect(readV2WalkthroughState(statePath).fragments).toHaveLength(9);

    chmodSync(statePath, 0o644);
    expect(() => readV2WalkthroughState(statePath)).toThrow(
      "private v2 walkthrough state is unsafe",
    );
    chmodSync(statePath, 0o600);

    const targetPath = join(directory, "state-target.json");
    writeFileSync(targetPath, JSON.stringify(validState()) + "\n", {
      encoding: "ascii",
      mode: 0o600,
    });
    unlinkSync(statePath);
    symlinkSync(targetPath, statePath);
    expect(() => readV2WalkthroughState(statePath)).toThrow(
      "private v2 walkthrough state is unsafe",
    );
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("v2 protected-state reader rejects hostile bytes, replacement, and renderer leakage", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-v2-state-hostile-"));
  try {
    chmodSync(directory, 0o700);
    const statePath = join(directory, "journeys.json");
    const canonical = JSON.stringify(validState()) + "\n";
    writeFileSync(statePath, canonical, { encoding: "ascii", mode: 0o600 });

    chmodSync(directory, 0o755);
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");
    chmodSync(directory, 0o700);

    writeFileSync(statePath, Buffer.from([0x5b, 0x80, 0x5d, 0x0a]), { mode: 0o600 });
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");

    writeFileSync(statePath, "x".repeat(4097), { encoding: "ascii", mode: 0o600 });
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");

    writeFileSync(statePath, canonical.replace("\n", "\r\n"), { encoding: "ascii", mode: 0o600 });
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");

    writeFileSync(statePath, `${canonical}\n`, { encoding: "ascii", mode: 0o600 });
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");

    writeFileSync(
      statePath,
      canonical.replace('"schemaVersion":2,', '"journey":"J11","schemaVersion":2,'),
      { encoding: "ascii", mode: 0o600 },
    );
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");

    const duplicate = canonical.replace(
      '"schemaVersion":2,',
      '"schemaVersion":2,"schemaVersion":2,',
    );
    writeFileSync(statePath, duplicate, { encoding: "ascii", mode: 0o600 });
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");

    const child = spawnSync(
      process.execPath,
      ["--import", "tsx", "tests/e2e/ui_walkthrough_v2_report.ts"],
      {
        cwd: process.cwd(),
        encoding: "utf8",
        env: {
          ...process.env,
          PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE: statePath,
          PLE_UI_WALKTHROUGH_MASTER_SEED: "42",
        },
      },
    );
    expect(child.status).not.toBe(0);
    expect(child.stdout).toBe("");

    writeFileSync(statePath, canonical, { encoding: "ascii", mode: 0o600 });
    const replacementDirectory = `${directory}-replacement`;
    const movedDirectory = `${directory}-moved`;
    mkdirSync(replacementDirectory, { mode: 0o700 });
    setV2StateOpenHookForTest(() => {
      renameSync(directory, movedDirectory);
      renameSync(replacementDirectory, directory);
      writeFileSync(statePath, canonical, { encoding: "ascii", mode: 0o600 });
    });
    expect(() => readV2WalkthroughState(statePath)).toThrow("unsafe");
    setV2StateOpenHookForTest(undefined);
  } finally {
    setV2StateOpenHookForTest(undefined);
    rmSync(directory, { recursive: true, force: true });
    rmSync(`${directory}-moved`, { recursive: true, force: true });
    rmSync(`${directory}-replacement`, { recursive: true, force: true });
  }
});
