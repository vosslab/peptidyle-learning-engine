// student_repeat_state.ts - append-only schema-v2 student evidence after J11/J12/J13.

import {
  closeSync,
  constants,
  fchmodSync,
  fstatSync,
  fsyncSync,
  ftruncateSync,
  lstatSync,
  openSync,
  readSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname } from "node:path";

import {
  parseInstructorSetupFragments,
  type InstructorSetupPrefix,
} from "./instructor_setup_state";

const MAX_STATE_BYTES = 4096;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

const STUDENT_CODES = {
  J1: ["visible_feedback", "visible_response", "visible_retry", "visible_submit"],
  J2: ["visible_completion", "visible_feedback", "visible_fresh_practice", "visible_submit"],
  J3: ["visible_controls_cleared", "visible_leave", "visible_resume", "visible_start"],
  J4: ["visible_back_action", "visible_completion", "visible_controls_cleared", "visible_submit"],
} as const;

type StudentJourney = keyof typeof STUDENT_CODES;
type StudentOutcomeCode = (typeof STUDENT_CODES)[StudentJourney][number];

export interface StudentRepeatFragment {
  readonly schemaVersion: 2;
  readonly journey: StudentJourney;
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId: string;
  readonly visibleOutcomeCodes: readonly StudentOutcomeCode[];
  readonly diagnostics: readonly [];
}

/** Builds one public-only student fragment after visible keyboard outcomes. */
export function passedStudentRepeatFragment(
  journey: StudentJourney,
  courseId: string,
  assignmentId: string,
  elapsedMs: number,
): StudentRepeatFragment {
  if (!UUID.test(courseId) || !UUID.test(assignmentId)) {
    throw new Error("student repeat evidence requires public UUID identifiers");
  }
  if (!Number.isSafeInteger(elapsedMs) || elapsedMs < 0 || elapsedMs > 30 * 60 * 1000) {
    throw new Error("student repeat evidence elapsed time is outside the allowed range");
  }
  return {
    schemaVersion: 2,
    journey,
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: STUDENT_CODES[journey],
    diagnostics: [],
  };
}

/** Appends J1 through J4 after the exact J11/J12/J13 prefix without replacing it. */
export function appendStudentRepeatState(path: string, next: StudentRepeatFragment): void {
  if (!validStudentFragment(next)) {
    throw new Error("private student repeat state is unsafe");
  }
  const descriptor = openPrivateState(path, constants.O_RDWR);
  try {
    const metadata = fstatSync(descriptor);
    if (
      !metadata.isFile() ||
      (metadata.mode & 0o777) !== 0o600 ||
      metadata.size > MAX_STATE_BYTES
    ) {
      throw new Error("private student repeat state is unsafe");
    }
    const prefix = parseState(readAscii(descriptor, metadata.size));
    const expectedJourney = ["J1", "J2", "J3", "J4"][prefix.students.length];
    if (expectedJourney !== next.journey || !matchesSetup(prefix.instructor, next)) {
      throw new Error("private student repeat state does not match next journey");
    }
    const output = JSON.stringify([...prefix.raw, next]) + "\n";
    ftruncateSync(descriptor, 0);
    writeFileSync(descriptor, output, "ascii");
    fchmodSync(descriptor, 0o600);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

interface StudentRepeatState {
  readonly raw: readonly unknown[];
  readonly instructor: InstructorSetupPrefix;
  readonly students: readonly StudentRepeatFragment[];
}

function openPrivateState(path: string, access: number): number {
  const parentPath = dirname(path);
  const childName = basename(path);
  if (parentPath === "." || childName !== "journeys.json") {
    throw new Error("private student repeat state is unsafe");
  }
  const directory = openSync(
    parentPath,
    constants.O_RDONLY | (constants.O_DIRECTORY ?? 0) | (constants.O_NOFOLLOW ?? 0),
  );
  try {
    const parent = fstatSync(directory);
    if (!parent.isDirectory() || (parent.mode & 0o777) !== 0o700) {
      throw new Error("private student repeat state is unsafe");
    }
    const descriptor = openSync(path, access | (constants.O_NOFOLLOW ?? 0));
    const child = fstatSync(descriptor);
    const currentParent = lstatSync(parentPath);
    if (
      !child.isFile() ||
      (child.mode & 0o777) !== 0o600 ||
      !currentParent.isDirectory() ||
      currentParent.isSymbolicLink() ||
      (currentParent.mode & 0o777) !== 0o700 ||
      currentParent.dev !== parent.dev ||
      currentParent.ino !== parent.ino
    ) {
      closeSync(descriptor);
      throw new Error("private student repeat state is unsafe");
    }
    return descriptor;
  } finally {
    closeSync(directory);
  }
}

function readAscii(descriptor: number, size: number): string {
  const bytes = Buffer.alloc(size);
  if (readSync(descriptor, bytes, 0, size, 0) !== size || bytes.some((byte) => byte > 0x7f)) {
    throw new Error("private student repeat state is unsafe");
  }
  return bytes.toString("ascii");
}

function parseState(raw: string): StudentRepeatState {
  if (!raw.endsWith("\n") || raw.includes("\r") || raw.split("\n").length !== 2) {
    throw new Error("private student repeat state is unsafe");
  }
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    throw new Error("private student repeat state is unsafe");
  }
  if (raw !== JSON.stringify(value) + "\n" || !Array.isArray(value)) {
    throw new Error("private student repeat state is unsafe");
  }
  const instructor = parseInstructorSetupFragments(value.slice(0, 3));
  const students = value.slice(3);
  if (instructor === undefined || students.length > 4 || !students.every(validStudentFragment)) {
    throw new Error("private student repeat state is unsafe");
  }
  for (const [index, student] of students.entries()) {
    if (student.journey !== ["J1", "J2", "J3", "J4"][index] || !matchesSetup(instructor, student)) {
      throw new Error("private student repeat state is unsafe");
    }
  }
  return { raw: value, instructor, students };
}

function validStudentFragment(value: unknown): value is StudentRepeatFragment {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const record = value as Readonly<Record<string, unknown>>;
  if (
    !hasExactEnumerableStringKeys(record, [
      "schemaVersion",
      "journey",
      "status",
      "elapsedMs",
      "courseId",
      "assignmentId",
      "visibleOutcomeCodes",
      "diagnostics",
    ])
  ) {
    return false;
  }
  const journey = record["journey"];
  if (
    (journey !== "J1" && journey !== "J2" && journey !== "J3" && journey !== "J4") ||
    record["schemaVersion"] !== 2 ||
    record["status"] !== "PASS" ||
    !Number.isSafeInteger(record["elapsedMs"]) ||
    (record["elapsedMs"] as number) < 0 ||
    (record["elapsedMs"] as number) > 30 * 60 * 1000 ||
    !UUID.test(String(record["courseId"])) ||
    !UUID.test(String(record["assignmentId"])) ||
    !Array.isArray(record["diagnostics"]) ||
    record["diagnostics"].length !== 0 ||
    !Array.isArray(record["visibleOutcomeCodes"]) ||
    JSON.stringify(record["visibleOutcomeCodes"]) !== JSON.stringify(STUDENT_CODES[journey])
  ) {
    return false;
  }
  return true;
}

function hasExactEnumerableStringKeys(
  record: Readonly<Record<string, unknown>>,
  expected: readonly string[],
): boolean {
  const keys = Reflect.ownKeys(record);
  if (keys.length !== expected.length || keys.some((key) => typeof key !== "string")) return false;
  return expected.every((key) => {
    const descriptor = Object.getOwnPropertyDescriptor(record, key);
    return (
      descriptor !== undefined &&
      descriptor.enumerable &&
      "value" in descriptor &&
      descriptor.get === undefined &&
      descriptor.set === undefined
    );
  });
}

function matchesSetup(instructor: InstructorSetupPrefix, student: StudentRepeatFragment): boolean {
  const assignment = instructor[2];
  return (
    student.courseId === assignment.courseId && student.assignmentId === assignment.assignmentId
  );
}
