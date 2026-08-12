// v2_j5_j8_state.ts - append-only private state boundary for the J5/J8 tail.

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

import type { J5SummaryEvidence } from "./instructor_gradebook_j5";

const MAX_STATE_BYTES = 4096;
const MAX_ELAPSED_MS = 30 * 60 * 1000;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

const CODES = {
  J11: ["visible_course_created", "visible_course_opened"],
  J12: ["visible_local_student_active"],
  J13: [
    "visible_assignment_created",
    "visible_catalog_problem_selected",
    "visible_four_question_chapter_one_selection",
    "visible_mastery_policy",
  ],
  J1: ["visible_feedback", "visible_response", "visible_retry", "visible_submit"],
  J2: ["visible_completion", "visible_feedback", "visible_fresh_practice", "visible_submit"],
  J3: ["visible_controls_cleared", "visible_leave", "visible_resume", "visible_start"],
  J4: ["visible_back_action", "visible_completion", "visible_controls_cleared", "visible_submit"],
  J5: ["visible_gradebook", "visible_score_summary", "visible_two_run_history"],
  J8: ["visible_instructor_gradebook", "visible_learner_completion", "visible_shared_assignment"],
} as const;

const PREFIX_JOURNEYS = ["J11", "J12", "J13", "J1", "J2", "J3", "J4"] as const;
type PrefixJourney = (typeof PREFIX_JOURNEYS)[number];

interface V2Fragment {
  readonly schemaVersion: 2;
  readonly journey: PrefixJourney | "J5" | "J8";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId?: string;
  readonly selectedDisplayIds?: readonly [string, string, string, string];
  readonly visibleOutcomeCodes: readonly string[];
  readonly diagnostics: readonly [];
}

export interface V2J8Fragment {
  readonly schemaVersion: 2;
  readonly journey: "J8";
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId: string;
  readonly visibleOutcomeCodes: readonly [
    "visible_instructor_gradebook",
    "visible_learner_completion",
    "visible_shared_assignment",
  ];
  readonly diagnostics: readonly [];
}

let beforeChildOpenForTest: (() => void) | undefined;

/** Test-only seam proving a replaced private parent cannot pass revalidation. */
export function setV2J5J8OpenHookForTest(hook: (() => void) | undefined): void {
  beforeChildOpenForTest = hook;
}

/** Appends J5 only after the exact J11 through J4 public-ID prefix is present. */
export function appendV2J5State(path: string, next: J5SummaryEvidence): void {
  if (!validJ5(next)) throw new Error("private v2 J5/J8 state is unsafe");
  append(path, "J5", next);
}

/** Commits J5 only after the browser context has closed without an error. */
export async function closeThenAppendV2J5State(
  path: string,
  next: J5SummaryEvidence,
  closeContext: () => Promise<void>,
): Promise<void> {
  await closeContext();
  appendV2J5State(path, next);
}

/** Builds and appends J8 from the descriptor-validated J11 through J5 prefix. */
export function appendV2J8State(path: string, elapsedMs: number): void {
  if (!validElapsed(elapsedMs)) throw new Error("private v2 J5/J8 state is unsafe");
  const descriptor = openPrivateState(path);
  try {
    const prefix = parsePrefix(readCanonicalAscii(descriptor));
    if (prefix.length !== 8 || prefix[7]?.journey !== "J5") {
      throw new Error("private v2 J5/J8 state does not match next journey");
    }
    const setup = prefix[2];
    if (setup?.assignmentId === undefined) throw new Error("private v2 J5/J8 state is unsafe");
    const next = passedV2J8Tail(setup.courseId, setup.assignmentId, elapsedMs);
    writeAppend(descriptor, [...prefix, next]);
  } finally {
    closeSync(descriptor);
  }
}

/** The J8 fragment deliberately contains only cross-actor public bindings. */
export function passedV2J8Tail(
  courseId: string,
  assignmentId: string,
  elapsedMs: number,
): V2J8Fragment {
  if (!UUID.test(courseId) || !UUID.test(assignmentId) || !validElapsed(elapsedMs)) {
    throw new Error("J8 requires matching canonical public observations");
  }
  return {
    schemaVersion: 2,
    journey: "J8",
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: CODES.J8,
    diagnostics: [],
  };
}

function append(path: string, expectedJourney: "J5", next: J5SummaryEvidence): void {
  const descriptor = openPrivateState(path);
  try {
    const prefix = parsePrefix(readCanonicalAscii(descriptor));
    const setup = prefix[2];
    if (
      prefix.length !== 7 ||
      setup?.assignmentId === undefined ||
      next.journey !== expectedJourney ||
      next.courseId !== setup.courseId ||
      next.assignmentId !== setup.assignmentId
    ) {
      throw new Error("private v2 J5/J8 state does not match next journey");
    }
    writeAppend(descriptor, [...prefix, next]);
  } finally {
    closeSync(descriptor);
  }
}

function openPrivateState(path: string): number {
  const parentPath = dirname(path);
  if (parentPath === "." || basename(path) !== "journeys.json") {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  const directory = openSync(
    parentPath,
    constants.O_RDONLY | (constants.O_DIRECTORY ?? 0) | (constants.O_NOFOLLOW ?? 0),
  );
  try {
    const parent = fstatSync(directory);
    if (!parent.isDirectory() || (parent.mode & 0o777) !== 0o700) {
      throw new Error("private v2 J5/J8 state is unsafe");
    }
    beforeChildOpenForTest?.();
    let descriptor: number;
    try {
      descriptor = openSync(path, constants.O_RDWR | (constants.O_NOFOLLOW ?? 0));
    } catch {
      throw new Error("private v2 J5/J8 state is unsafe");
    }
    const child = fstatSync(descriptor);
    const currentParent = lstatSync(parentPath);
    if (
      !child.isFile() ||
      (child.mode & 0o777) !== 0o600 ||
      child.size > MAX_STATE_BYTES ||
      !currentParent.isDirectory() ||
      currentParent.isSymbolicLink() ||
      (currentParent.mode & 0o777) !== 0o700 ||
      currentParent.dev !== parent.dev ||
      currentParent.ino !== parent.ino
    ) {
      closeSync(descriptor);
      throw new Error("private v2 J5/J8 state is unsafe");
    }
    return descriptor;
  } finally {
    closeSync(directory);
  }
}

function readCanonicalAscii(descriptor: number): readonly unknown[] {
  const metadata = fstatSync(descriptor);
  if (!metadata.isFile() || (metadata.mode & 0o777) !== 0o600 || metadata.size > MAX_STATE_BYTES) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  const bytes = Buffer.alloc(metadata.size);
  if (readSync(descriptor, bytes, 0, metadata.size, 0) !== metadata.size || bytes.some(notAscii)) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  const raw = bytes.toString("ascii");
  if (!raw.endsWith("\n") || raw.includes("\r") || raw.split("\n").length !== 2) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  if (raw !== JSON.stringify(parsed) + "\n" || !Array.isArray(parsed)) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  return parsed;
}

function parsePrefix(value: readonly unknown[]): readonly V2Fragment[] {
  if (value.length < 7 || value.length > 8) throw new Error("private v2 J5/J8 state is unsafe");
  const expected = value.length === 7 ? PREFIX_JOURNEYS : ([...PREFIX_JOURNEYS, "J5"] as const);
  const fragments = value.map((fragment, index) => parseFragment(fragment, expected[index]));
  if (fragments.some((fragment) => fragment === undefined)) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  const parsed = fragments as V2Fragment[];
  const setup = parsed[2];
  if (setup?.assignmentId === undefined) throw new Error("private v2 J5/J8 state is unsafe");
  if (
    !parsed.every(
      (fragment) =>
        fragment.courseId === setup.courseId &&
        (fragment.journey === "J11" ||
          fragment.journey === "J12" ||
          fragment.assignmentId === setup.assignmentId),
    )
  ) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  return parsed;
}

function parseFragment(
  value: unknown,
  journey: PrefixJourney | "J5" | undefined,
): V2Fragment | undefined {
  if (journey === undefined || !isExactDataObject(value, keysFor(journey))) return undefined;
  const record = value;
  const courseId = own(record, "courseId");
  const elapsedMs = own(record, "elapsedMs");
  if (
    own(record, "schemaVersion") !== 2 ||
    own(record, "journey") !== journey ||
    own(record, "status") !== "PASS" ||
    typeof courseId !== "string" ||
    !UUID.test(courseId) ||
    !validElapsed(elapsedMs) ||
    !sameStrings(own(record, "visibleOutcomeCodes"), CODES[journey]) ||
    !isExactEmptyArray(own(record, "diagnostics"))
  )
    return undefined;
  const assignmentId = own(record, "assignmentId");
  if (journey === "J11" || journey === "J12") {
    return {
      schemaVersion: 2,
      journey,
      status: "PASS",
      elapsedMs,
      courseId,
      visibleOutcomeCodes: CODES[journey],
      diagnostics: [],
    };
  }
  if (typeof assignmentId !== "string" || !UUID.test(assignmentId)) return undefined;
  if (journey === "J13") {
    const selectedDisplayIds = own(record, "selectedDisplayIds");
    if (!validSelectedDisplayIds(selectedDisplayIds)) return undefined;
    return {
      schemaVersion: 2,
      journey,
      status: "PASS",
      elapsedMs,
      courseId,
      assignmentId,
      selectedDisplayIds,
      visibleOutcomeCodes: CODES[journey],
      diagnostics: [],
    };
  }
  return {
    schemaVersion: 2,
    journey,
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: CODES[journey],
    diagnostics: [],
  };
}

function validJ5(value: J5SummaryEvidence): boolean {
  return isExactDataObject(value, keysFor("J5")) && parseFragment(value, "J5") !== undefined;
}

function writeAppend(descriptor: number, values: readonly unknown[]): void {
  const output = JSON.stringify(values) + "\n";
  if (Buffer.byteLength(output, "ascii") > MAX_STATE_BYTES) {
    throw new Error("private v2 J5/J8 state is unsafe");
  }
  ftruncateSync(descriptor, 0);
  writeFileSync(descriptor, output, "ascii");
  fchmodSync(descriptor, 0o600);
  fsyncSync(descriptor);
}

function keysFor(journey: PrefixJourney | "J5"): readonly string[] {
  const base = ["schemaVersion", "journey", "status", "elapsedMs", "courseId"];
  if (journey === "J13")
    return [...base, "assignmentId", "selectedDisplayIds", "visibleOutcomeCodes", "diagnostics"];
  if (journey === "J11" || journey === "J12")
    return [...base, "visibleOutcomeCodes", "diagnostics"];
  return [...base, "assignmentId", "visibleOutcomeCodes", "diagnostics"];
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

function isExactDataObject(value: unknown, expected: readonly string[]): value is object {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    Object.getPrototypeOf(value) !== Object.prototype
  )
    return false;
  const keys = Reflect.ownKeys(value);
  return (
    keys.length === expected.length &&
    keys.every((key) => typeof key === "string") &&
    expected.every((key) => {
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      return (
        descriptor?.enumerable === true &&
        "value" in (descriptor ?? {}) &&
        descriptor?.get === undefined &&
        descriptor?.set === undefined
      );
    })
  );
}

function own(value: object, key: string): unknown {
  return Object.getOwnPropertyDescriptor(value, key)?.value;
}

function sameStrings(value: unknown, expected: readonly string[]): boolean {
  return (
    isExactArray(value, expected.length) && expected.every((item, index) => value[index] === item)
  );
}

function isExactEmptyArray(value: unknown): value is readonly [] {
  return isExactArray(value, 0);
}

function isExactArray(value: unknown, length: number): value is readonly unknown[] {
  if (
    !Array.isArray(value) ||
    Object.getPrototypeOf(value) !== Array.prototype ||
    value.length !== length
  )
    return false;
  const keys = Reflect.ownKeys(value);
  if (keys.length !== length + 1 || keys[length] !== "length") return false;
  return Array.from({ length }, (_, index) => String(index)).every((key) => {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    return (
      descriptor !== undefined &&
      descriptor.enumerable &&
      "value" in descriptor &&
      descriptor.get === undefined &&
      descriptor.set === undefined
    );
  });
}

function validElapsed(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 0 &&
    value <= MAX_ELAPSED_MS
  );
}

function notAscii(byte: number): boolean {
  return byte > 0x7f;
}
