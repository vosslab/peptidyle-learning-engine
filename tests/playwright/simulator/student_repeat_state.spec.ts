import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import { commitInstructorSetupState, type InstructorSetupPrefix } from "./instructor_setup_state";
import {
  appendStudentRepeatState,
  passedStudentRepeatFragment,
  type StudentRepeatFragment,
} from "./student_repeat_state";

const courseReference = "C-42";
const assignmentReference = "A-73";

function statePath(): string {
  const directory = mkdtempSync(join(tmpdir(), "ple-student-repeat-state-"));
  chmodSync(directory, 0o700);
  const path = join(directory, "journeys.json");
  writeFileSync(path, "", { encoding: "ascii", mode: 0o600 });
  return path;
}

function setupPrefix(): InstructorSetupPrefix {
  return [
    {
      schemaVersion: 2,
      journey: "J11",
      status: "PASS",
      elapsedMs: 1,
      courseReference,
      visibleOutcomeCodes: ["visible_course_created", "visible_course_opened"],
      diagnostics: [],
    },
    {
      schemaVersion: 2,
      journey: "J12",
      status: "PASS",
      elapsedMs: 2,
      courseReference,
      visibleOutcomeCodes: ["visible_local_student_active"],
      diagnostics: [],
    },
    {
      schemaVersion: 2,
      journey: "J13",
      status: "PASS",
      elapsedMs: 3,
      courseReference,
      assignmentReference,
      selectedDisplayIds: ["7K3-M9QP", "ABC-123T", "PEP-T1D3", "GEN-E42K"],
      visibleOutcomeCodes: [
        "visible_assignment_created",
        "visible_catalog_problem_selected",
        "visible_four_question_chapter_one_selection",
        "visible_mastery_policy",
      ],
      diagnostics: [],
    },
  ];
}

test("schema-v2 student evidence appends J1 through J4 after the exact instructor prefix", () => {
  const path = statePath();
  commitInstructorSetupState(path, setupPrefix());
  for (const journey of ["J1", "J2", "J3", "J4"] as const) {
    appendStudentRepeatState(
      path,
      passedStudentRepeatFragment(journey, courseReference, assignmentReference, 4),
    );
  }
  const fragments: unknown = JSON.parse(readFileSync(path, "ascii"));
  expect(fragments).toHaveLength(7);
});

test("student evidence rejects a reordered journey, foreign assignment, and unsafe state metadata", () => {
  const path = statePath();
  commitInstructorSetupState(path, setupPrefix());
  expect(() =>
    appendStudentRepeatState(
      path,
      passedStudentRepeatFragment("J2", courseReference, assignmentReference, 4),
    ),
  ).toThrow("next journey");
  expect(() =>
    appendStudentRepeatState(path, passedStudentRepeatFragment("J1", courseReference, "A-74", 4)),
  ).toThrow("next journey");
  chmodSync(path, 0o644);
  expect(() =>
    appendStudentRepeatState(
      path,
      passedStudentRepeatFragment("J1", courseReference, assignmentReference, 4),
    ),
  ).toThrow("unsafe");
});

test("student append rejects accessor, hidden, symbol, and inherited caller fragments", () => {
  const path = statePath();
  commitInstructorSetupState(path, setupPrefix());
  const canonical = passedStudentRepeatFragment("J1", courseReference, assignmentReference, 4);
  const accessor = { ...canonical } as Record<string, unknown>;
  Object.defineProperty(accessor, "assignmentReference", {
    enumerable: true,
    get: () => assignmentReference,
  });
  expect(() =>
    appendStudentRepeatState(path, accessor as unknown as StudentRepeatFragment),
  ).toThrow("unsafe");

  const hidden = { ...canonical } as Record<string | symbol, unknown>;
  Object.defineProperty(hidden, "hidden", { value: "x", enumerable: false });
  expect(() => appendStudentRepeatState(path, hidden as unknown as StudentRepeatFragment)).toThrow(
    "unsafe",
  );

  const symbol = { ...canonical, [Symbol("private")]: "x" };
  expect(() => appendStudentRepeatState(path, symbol)).toThrow("unsafe");

  const inherited = Object.create(canonical) as StudentRepeatFragment;
  expect(() => appendStudentRepeatState(path, inherited)).toThrow("unsafe");
});
