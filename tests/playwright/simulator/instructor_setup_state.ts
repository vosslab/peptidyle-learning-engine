// instructor_setup_state.ts - private, append-only public-ID handoff for J11-J13.

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

const MAX_STATE_BYTES = 4096;
const MAX_ELAPSED_MS = 30 * 60 * 1000;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

type InstructorJourney = "J11" | "J12" | "J13";

export type InstructorSetupFragment =
  | {
      readonly schemaVersion: 2;
      readonly journey: "J11";
      readonly status: "PASS";
      readonly elapsedMs: number;
      readonly courseId: string;
      readonly visibleOutcomeCodes: readonly ["visible_course_created", "visible_course_opened"];
      readonly diagnostics: readonly [];
    }
  | {
      readonly schemaVersion: 2;
      readonly journey: "J12";
      readonly status: "PASS";
      readonly elapsedMs: number;
      readonly courseId: string;
      readonly visibleOutcomeCodes: readonly ["visible_local_student_active"];
      readonly diagnostics: readonly [];
    }
  | {
      readonly schemaVersion: 2;
      readonly journey: "J13";
      readonly status: "PASS";
      readonly elapsedMs: number;
      readonly courseId: string;
      readonly assignmentId: string;
      /** Four visible P-n-vn locators prove the canonical Chapter 1 selection. */
      readonly selectedDisplayIds: readonly [string, string, string, string];
      readonly visibleOutcomeCodes: readonly [
        "visible_assignment_created",
        "visible_catalog_problem_selected",
        "visible_four_question_chapter_one_selection",
        "visible_mastery_policy",
      ];
      readonly diagnostics: readonly [];
    };

export type InstructorSetupPrefix = readonly [
  Extract<InstructorSetupFragment, { readonly journey: "J11" }>,
  Extract<InstructorSetupFragment, { readonly journey: "J12" }>,
  Extract<InstructorSetupFragment, { readonly journey: "J13" }>,
];

const EXPECTED_JOURNEYS: readonly InstructorJourney[] = ["J11", "J12", "J13"];
const EXPECTED_CODES = {
  J11: ["visible_course_created", "visible_course_opened"],
  J12: ["visible_local_student_active"],
  J13: [
    "visible_assignment_created",
    "visible_catalog_problem_selected",
    "visible_four_question_chapter_one_selection",
    "visible_mastery_policy",
  ],
} as const;

let beforeChildOpenForTest: (() => void) | undefined;

/** Test-only seam for proving a parent replacement cannot pass the descriptor recheck. */
export function setInstructorStateOpenHookForTest(hook: (() => void) | undefined): void {
  beforeChildOpenForTest = hook;
}

/** Reads the exact schema-v2 instructor prefix through a no-follow descriptor. */
export function readInstructorSetupPrefix(path: string): readonly InstructorSetupFragment[] {
  const descriptor = openPrivateState(path, constants.O_RDONLY);
  try {
    const metadata = fstatSync(descriptor);
    if (
      !metadata.isFile() ||
      (metadata.mode & 0o777) !== 0o600 ||
      metadata.size > MAX_STATE_BYTES
    ) {
      throw new Error("private instructor setup state is unsafe");
    }
    return parsePrefix(readAscii(descriptor, metadata.size));
  } finally {
    closeSync(descriptor);
  }
}

/** Commits J11/J12/J13 together only after every visible setup assertion has passed. */
export function commitInstructorSetupState(path: string, fragments: InstructorSetupPrefix): void {
  const descriptor = openPrivateState(path, constants.O_RDWR);
  try {
    const metadata = fstatSync(descriptor);
    if (
      !metadata.isFile() ||
      (metadata.mode & 0o777) !== 0o600 ||
      metadata.size > MAX_STATE_BYTES
    ) {
      throw new Error("private instructor setup state is unsafe");
    }
    const prefix = parsePrefix(readAscii(descriptor, metadata.size));
    if (prefix.length !== 0 || !fragments.every(validFragment)) {
      throw new Error("private instructor setup state does not match complete journey");
    }
    for (const [index, fragment] of fragments.entries()) {
      if (
        fragment.journey !== EXPECTED_JOURNEYS[index] ||
        !matchesPrefix(fragments.slice(0, index), fragment)
      ) {
        throw new Error("private instructor setup state does not match complete journey");
      }
    }
    const output = JSON.stringify(fragments) + "\n";
    ftruncateSync(descriptor, 0);
    writeFileSync(descriptor, output, { encoding: "ascii" });
    fchmodSync(descriptor, 0o600);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function openPrivateState(path: string, access: number): number {
  const parentPath = dirname(path);
  const childName = basename(path);
  if (parentPath === "." || childName !== "journeys.json") {
    throw new Error("private instructor setup state is unsafe");
  }
  const directory = openSync(
    parentPath,
    constants.O_RDONLY | (constants.O_DIRECTORY ?? 0) | (constants.O_NOFOLLOW ?? 0),
  );
  try {
    const parent = fstatSync(directory);
    if (!parent.isDirectory() || (parent.mode & 0o777) !== 0o700) {
      throw new Error("private instructor setup state is unsafe");
    }
    beforeChildOpenForTest?.();
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
      throw new Error("private instructor setup state is unsafe");
    }
    return descriptor;
  } finally {
    closeSync(directory);
  }
}

function readAscii(descriptor: number, size: number): string {
  const bytes = Buffer.alloc(size);
  if (readSync(descriptor, bytes, 0, size, 0) !== size || bytes.some((byte) => byte > 0x7f)) {
    throw new Error("private instructor setup state is unsafe");
  }
  return bytes.toString("ascii");
}

function parsePrefix(raw: string): readonly InstructorSetupFragment[] {
  if (raw === "") return [];
  if (!raw.endsWith("\n") || raw.includes("\r") || raw.split("\n").length !== 2) {
    throw new Error("private instructor setup state is unsafe");
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("private instructor setup state is unsafe");
  }
  if (raw !== JSON.stringify(parsed) + "\n") {
    throw new Error("private instructor setup state is unsafe");
  }
  const fragments = parseInstructorSetupFragments(parsed);
  if (fragments === undefined) throw new Error("private instructor setup state is unsafe");
  return fragments;
}

/** Validates the public J11/J12/J13 prefix for the later fixed student handoff. */
export function parseInstructorSetupFragments(value: unknown): InstructorSetupPrefix | undefined {
  if (!Array.isArray(value) || value.length !== 3 || !value.every(validFragment)) return undefined;
  const fragments = value as readonly InstructorSetupFragment[];
  for (const [index, fragment] of fragments.entries()) {
    if (
      fragment.journey !== EXPECTED_JOURNEYS[index] ||
      !matchesPrefix(fragments.slice(0, index), fragment)
    ) {
      return undefined;
    }
  }
  const first = fragments[0];
  const second = fragments[1];
  const third = fragments[2];
  if (first === undefined || second === undefined || third === undefined) return undefined;
  if (first.journey !== "J11" || second.journey !== "J12" || third.journey !== "J13") {
    return undefined;
  }
  return [first, second, third];
}

function validFragment(value: unknown): value is InstructorSetupFragment {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const record = value as Readonly<Record<string, unknown>>;
  const commonKeys = [
    "schemaVersion",
    "journey",
    "status",
    "elapsedMs",
    "courseId",
    "visibleOutcomeCodes",
    "diagnostics",
  ];
  const j13Keys = ["assignmentId", "selectedDisplayIds"];
  const hasKnownKeys =
    hasExactEnumerableStringKeys(record, commonKeys) ||
    hasExactEnumerableStringKeys(record, [...commonKeys, ...j13Keys]);
  if (!hasKnownKeys) return false;
  const journey = record["journey"];
  const requiredKeys = journey === "J13" ? [...commonKeys, ...j13Keys] : commonKeys;
  if (!hasExactEnumerableStringKeys(record, requiredKeys)) return false;
  if (
    record["schemaVersion"] !== 2 ||
    record["status"] !== "PASS" ||
    !Number.isSafeInteger(record["elapsedMs"]) ||
    (record["elapsedMs"] as number) < 0 ||
    (record["elapsedMs"] as number) > MAX_ELAPSED_MS ||
    !UUID.test(String(record["courseId"]))
  )
    return false;
  if (!Array.isArray(record["diagnostics"]) || record["diagnostics"].length !== 0) return false;
  if (
    !Array.isArray(record["visibleOutcomeCodes"]) ||
    JSON.stringify(record["visibleOutcomeCodes"]) !==
      JSON.stringify(EXPECTED_CODES[journey as InstructorJourney])
  )
    return false;
  if (journey === "J13")
    return (
      UUID.test(String(record["assignmentId"])) &&
      validSelectedDisplayIds(record["selectedDisplayIds"])
    );
  return journey === "J11" || journey === "J12";
}

function validSelectedDisplayIds(
  value: unknown,
): value is readonly [string, string, string, string] {
  return (
    Array.isArray(value) &&
    value.length === 4 &&
    value.every((id) => typeof id === "string" && /^P-[1-9][0-9]*-v[1-9][0-9]*$/u.test(id)) &&
    new Set(value).size === 4
  );
}

function hasExactEnumerableStringKeys(
  record: Readonly<Record<string, unknown>>,
  expectedKeys: readonly string[],
): boolean {
  const keys = Reflect.ownKeys(record);
  if (keys.length !== expectedKeys.length || keys.some((key) => typeof key !== "string")) {
    return false;
  }
  return expectedKeys.every((key) => {
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

function matchesPrefix(
  prefix: readonly InstructorSetupFragment[],
  next: InstructorSetupFragment,
): boolean {
  const first = prefix[0];
  return first === undefined || next.courseId === first.courseId;
}
