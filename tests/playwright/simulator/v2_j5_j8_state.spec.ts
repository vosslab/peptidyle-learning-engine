// v2_j5_j8_state.spec.ts - hostile tests for the isolated J5/J8 append tail.

import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import { commitInstructorSetupState, type InstructorSetupPrefix } from "./instructor_setup_state";
import { passedJ5SummaryEvidence } from "./instructor_gradebook_j5";
import { appendStudentRepeatState, passedStudentRepeatFragment } from "./student_repeat_state";
import {
  appendV2J5State,
  appendV2J8State,
  closeThenAppendV2J5State,
  setV2J5J8OpenHookForTest,
} from "./v2_j5_j8_state";
import { parseV2WalkthroughState } from "./v2_visible_outcome_report";

const COURSE_ID = "123e4567-e89b-12d3-a456-426614174000";
const ASSIGNMENT_ID = "123e4567-e89b-12d3-a456-426614174001";
const PROBLEM_ID = "123e4567-e89b-12d3-a456-426614174002";
const VERSION_ID = "123e4567-e89b-12d3-a456-426614174003";

function statePath(): string {
  const directory = mkdtempSync(join(tmpdir(), "ple-v2-j5-j8-state-"));
  chmodSync(directory, 0o700);
  const path = join(directory, "journeys.json");
  writeFileSync(path, "", { encoding: "ascii", mode: 0o600 });
  return path;
}

function setup(): InstructorSetupPrefix {
  return [
    {
      schemaVersion: 2,
      journey: "J11",
      status: "PASS",
      elapsedMs: 1,
      courseId: COURSE_ID,
      visibleOutcomeCodes: ["visible_course_created", "visible_course_opened"],
      diagnostics: [],
    },
    {
      schemaVersion: 2,
      journey: "J12",
      status: "PASS",
      elapsedMs: 1,
      courseId: COURSE_ID,
      visibleOutcomeCodes: ["visible_local_student_active"],
      diagnostics: [],
    },
    {
      schemaVersion: 2,
      journey: "J13",
      status: "PASS",
      elapsedMs: 1,
      courseId: COURSE_ID,
      assignmentId: ASSIGNMENT_ID,
      problemId: PROBLEM_ID,
      versionId: VERSION_ID,
      visibleOutcomeCodes: [
        "visible_assignment_created",
        "visible_catalog_problem_selected",
        "visible_mastery_policy",
      ],
      diagnostics: [],
    },
  ];
}

function throughJ4(path: string): void {
  commitInstructorSetupState(path, setup());
  for (const journey of ["J1", "J2", "J3", "J4"] as const) {
    appendStudentRepeatState(
      path,
      passedStudentRepeatFragment(journey, COURSE_ID, ASSIGNMENT_ID, 1),
    );
  }
}

test("J5 and J8 append only after the exact public schema-v2 prefix", () => {
  const path = statePath();
  throughJ4(path);
  appendV2J5State(path, passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 2));
  appendV2J8State(path, 3);
  const fragments: unknown = JSON.parse(readFileSync(path, "ascii"));
  expect(parseV2WalkthroughState(fragments)).toBeDefined();
  expect(fragments).toHaveLength(9);
  const tailKeys = (fragments as Record<string, unknown>[])
    .slice(7)
    .flatMap((fragment) => Object.keys(fragment));
  expect(tailKeys).not.toEqual(expect.arrayContaining(["score", "title", "learnerId", "runId"]));
});

test("J5 rejects wrong sequencing, foreign public IDs, and hostile caller data", () => {
  const path = statePath();
  throughJ4(path);
  expect(() => appendV2J8State(path, 1)).toThrow("next journey");
  expect(() => appendV2J5State(path, passedJ5SummaryEvidence(COURSE_ID, PROBLEM_ID, 1))).toThrow(
    "next journey",
  );

  const hostile = { ...passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1) } as Record<
    string,
    unknown
  >;
  Object.defineProperty(hostile, "assignmentId", { enumerable: true, get: () => ASSIGNMENT_ID });
  expect(() =>
    appendV2J5State(path, hostile as unknown as ReturnType<typeof passedJ5SummaryEvidence>),
  ).toThrow("unsafe");

  const hidden = { ...passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1) } as Record<
    string,
    unknown
  >;
  Object.defineProperty(hidden, "private", { value: "x", enumerable: false });
  expect(() =>
    appendV2J5State(path, hidden as unknown as ReturnType<typeof passedJ5SummaryEvidence>),
  ).toThrow("unsafe");

  const symbol = {
    ...passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1),
    [Symbol("private")]: "x",
  };
  expect(() => appendV2J5State(path, symbol)).toThrow("unsafe");

  const inherited = Object.create(
    passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1),
  ) as ReturnType<typeof passedJ5SummaryEvidence>;
  expect(() => appendV2J5State(path, inherited)).toThrow("unsafe");

  const arrayAccessor = {
    ...passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1),
    visibleOutcomeCodes: ["visible_gradebook", "visible_score_summary", "visible_two_run_history"],
  };
  Object.defineProperty(arrayAccessor.visibleOutcomeCodes, "0", {
    enumerable: true,
    get: () => "visible_gradebook",
  });
  expect(() => appendV2J5State(path, arrayAccessor)).toThrow("unsafe");
});

test("J8 child returns no stdout and refuses an unsafe or replaced private path", () => {
  const path = statePath();
  throughJ4(path);
  appendV2J5State(path, passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1));
  chmodSync(path, 0o644);
  const child = spawnSync(
    process.execPath,
    ["--import", "tsx", "tests/e2e/ui_walkthrough_v2_cross_actor.ts"],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: {
        ...process.env,
        PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE: path,
        PLE_UI_WALKTHROUGH_J8_ELAPSED_MS: "1",
      },
    },
  );
  expect(child.status).not.toBe(0);
  expect(child.stdout).toBe("");
  chmodSync(path, 0o600);

  const directory = path.slice(0, path.lastIndexOf("/"));
  const moved = `${directory}-moved`;
  setV2J5J8OpenHookForTest(() => renameSync(directory, moved));
  expect(() => appendV2J8State(path, 1)).toThrow("unsafe");
  setV2J5J8OpenHookForTest(undefined);
});

test("J5 leaves the exact J11 through J4 prefix untouched when context closure fails", async () => {
  const path = statePath();
  throughJ4(path);
  const original = readFileSync(path, "ascii");
  await expect(
    closeThenAppendV2J5State(path, passedJ5SummaryEvidence(COURSE_ID, ASSIGNMENT_ID, 1), () =>
      Promise.reject(new Error("context close failed")),
    ),
  ).rejects.toThrow("context close failed");
  expect(readFileSync(path, "ascii")).toBe(original);
});
