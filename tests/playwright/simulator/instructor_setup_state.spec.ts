import { chmodSync, mkdirSync, mkdtempSync, renameSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import {
  commitInstructorSetupState,
  parseInstructorSetupFragments,
  readInstructorSetupPrefix,
  setInstructorStateOpenHookForTest,
  type InstructorSetupFragment,
} from "./instructor_setup_state";

const courseId = "123e4567-e89b-12d3-a456-426614174000";
const assignmentId = "123e4567-e89b-12d3-a456-426614174001";
const problemId = "123e4567-e89b-12d3-a456-426614174002";
const versionId = "123e4567-e89b-12d3-a456-426614174003";

function statePath(): string {
  const directory = mkdtempSync(join(tmpdir(), "ple-instructor-state-"));
  chmodSync(directory, 0o700);
  const path = join(directory, "journeys.json");
  writeFileSync(path, "", { encoding: "ascii", mode: 0o600 });
  return path;
}

function j11(): Extract<InstructorSetupFragment, { readonly journey: "J11" }> {
  return {
    schemaVersion: 2,
    journey: "J11",
    status: "PASS",
    elapsedMs: 1,
    courseId,
    visibleOutcomeCodes: ["visible_course_created", "visible_course_opened"],
    diagnostics: [],
  };
}

function j12(): Extract<InstructorSetupFragment, { readonly journey: "J12" }> {
  return {
    schemaVersion: 2,
    journey: "J12",
    status: "PASS",
    elapsedMs: 2,
    courseId,
    visibleOutcomeCodes: ["visible_local_student_active"],
    diagnostics: [],
  };
}

function j13(): Extract<InstructorSetupFragment, { readonly journey: "J13" }> {
  return {
    schemaVersion: 2,
    journey: "J13",
    status: "PASS",
    elapsedMs: 3,
    courseId,
    assignmentId,
    problemId,
    versionId,
    visibleOutcomeCodes: [
      "visible_assignment_created",
      "visible_catalog_problem_selected",
      "visible_mastery_policy",
    ],
    diagnostics: [],
  };
}

test("schema-v2 private state commits only the complete J11/J12/J13 public-ID handoff", () => {
  const path = statePath();
  expect(readInstructorSetupPrefix(path)).toEqual([]);
  commitInstructorSetupState(path, [j11(), j12(), j13()]);
  expect(readInstructorSetupPrefix(path)).toHaveLength(3);
});

test("a post-J12 visible failure cannot leave a partial protected-state handoff", () => {
  const path = statePath();
  const heldFragments = [j11(), j12()] as const;
  expect(heldFragments).toHaveLength(2);
  // J13 failed before the sole commit boundary, so no fragment is written.
  expect(readInstructorSetupPrefix(path)).toEqual([]);
});

test("schema-v2 state rejects private identity fields, symlinks, and reordered fragments", () => {
  const path = statePath();
  writeFileSync(
    path,
    `[{"schemaVersion":2,"journey":"J11","status":"PASS","elapsedMs":1,"courseId":"${courseId}","learnerAlias":"private","visibleOutcomeCodes":["visible_course_created","visible_course_opened"],"diagnostics":[]}]\n`,
    { encoding: "ascii", mode: 0o600 },
  );
  expect(() => readInstructorSetupPrefix(path)).toThrow("unsafe");

  const linkedPath = statePath();
  const link = `${linkedPath}.link`;
  symlinkSync(linkedPath, link);
  expect(() => readInstructorSetupPrefix(link)).toThrow("unsafe");

  const reorderedPath = statePath();
  expect(() => commitInstructorSetupState(reorderedPath, [j11(), j11() as never, j13()])).toThrow(
    "complete journey",
  );
});

test("schema-v2 state rejects duplicate JSON, upper-case IDs, oversized files, and parent replacement", () => {
  const duplicatePath = statePath();
  writeFileSync(
    duplicatePath,
    `[{"schemaVersion":2,"journey":"J11","journey":"J11","status":"PASS","elapsedMs":1,"courseId":"${courseId}","visibleOutcomeCodes":["visible_course_created","visible_course_opened"],"diagnostics":[]}]\n`,
    { encoding: "ascii", mode: 0o600 },
  );
  expect(() => readInstructorSetupPrefix(duplicatePath)).toThrow("unsafe");

  const uppercasePath = statePath();
  writeFileSync(
    uppercasePath,
    `[{"schemaVersion":2,"journey":"J11","status":"PASS","elapsedMs":1,"courseId":"${courseId.toUpperCase()}","visibleOutcomeCodes":["visible_course_created","visible_course_opened"],"diagnostics":[]}]\n`,
    { encoding: "ascii", mode: 0o600 },
  );
  expect(() => readInstructorSetupPrefix(uppercasePath)).toThrow("unsafe");

  const oversizedPath = statePath();
  writeFileSync(oversizedPath, "x".repeat(4097), { encoding: "ascii", mode: 0o600 });
  expect(() => readInstructorSetupPrefix(oversizedPath)).toThrow("unsafe");

  const racedPath = statePath();
  const originalDirectory = racedPath.slice(0, racedPath.lastIndexOf("/"));
  const replacementDirectory = `${originalDirectory}-replacement`;
  const movedDirectory = `${originalDirectory}-moved`;
  mkdirSync(replacementDirectory);
  chmodSync(replacementDirectory, 0o700);
  setInstructorStateOpenHookForTest(() => {
    renameSync(originalDirectory, movedDirectory);
    renameSync(replacementDirectory, originalDirectory);
    writeFileSync(racedPath, "", { encoding: "ascii", mode: 0o600 });
  });
  expect(() => readInstructorSetupPrefix(racedPath)).toThrow("unsafe");
  setInstructorStateOpenHookForTest(undefined);
});

test("schema-v2 parser rejects inherited, hidden, and symbol fields in a public handoff", () => {
  const inherited: Record<string, unknown> = { ...j11() };
  delete inherited["courseId"];
  Object.setPrototypeOf(inherited, { courseId });
  expect(parseInstructorSetupFragments([inherited, j12(), j13()])).toBeUndefined();

  const hidden = { ...j11() };
  Object.defineProperty(hidden, "hidden", { value: "x", enumerable: false });
  expect(parseInstructorSetupFragments([hidden, j12(), j13()])).toBeUndefined();

  const symbol = { ...j11(), [Symbol("private")]: "x" };
  expect(parseInstructorSetupFragments([symbol, j12(), j13()])).toBeUndefined();

  const accessor = { ...j11() } as Record<string, unknown>;
  Object.defineProperty(accessor, "courseId", {
    enumerable: true,
    get: () => courseId,
  });
  expect(parseInstructorSetupFragments([accessor, j12(), j13()])).toBeUndefined();
});

test("schema-v2 parser rejects an elapsed time beyond the bounded walkthrough window", () => {
  const overdue = { ...j11(), elapsedMs: 30 * 60 * 1000 + 1 };
  expect(parseInstructorSetupFragments([overdue, j12(), j13()])).toBeUndefined();
});
