// v2_visible_outcome_report.ts - closed, redacted report contract for the no-email pilot.

import { closeSync, constants, fstatSync, lstatSync, openSync, readSync } from "node:fs";
import { basename, dirname } from "node:path";

const MAX_ELAPSED_MS = 30 * 60 * 1000;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

const JOURNEY_CODES = {
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

const JOURNEYS = ["J11", "J12", "J13", "J1", "J2", "J3", "J4", "J5", "J8"] as const;
const MAX_MASTER_SEED = 4_294_967_295;

let beforeV2ChildOpenForTest: (() => void) | undefined;

export type V2Journey = (typeof JOURNEYS)[number];
export type V2VisibleOutcomeCode = (typeof JOURNEY_CODES)[V2Journey][number];

/** Test-only seam proving that a parent replacement cannot pass descriptor revalidation. */
export function setV2StateOpenHookForTest(hook: (() => void) | undefined): void {
  beforeV2ChildOpenForTest = hook;
}

export interface V2JourneyFragment {
  readonly schemaVersion: 2;
  readonly journey: V2Journey;
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId?: string;
  readonly selectedDisplayIds?: readonly [string, string, string, string];
  readonly visibleOutcomeCodes: readonly V2VisibleOutcomeCode[];
  readonly diagnostics: readonly [];
}

export interface V2WalkthroughState {
  readonly fragments: readonly [
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
    V2JourneyFragment,
  ];
}

interface PublicJourneyRow {
  readonly journey: V2Journey;
  readonly status: "PASS";
  readonly elapsedMs: number;
  readonly visibleOutcomeCodes: readonly V2VisibleOutcomeCode[];
  readonly diagnostics: readonly [];
}

interface PublicV2Report {
  readonly schemaVersion: 2;
  readonly status: "PASS";
  readonly masterSeed: number;
  readonly stage: "complete";
  readonly elapsedMs: number;
  readonly arrangements: readonly [{ readonly label: "launcher-chapter-one-genetics" }];
  readonly journeys: readonly PublicJourneyRow[];
}

/**
 * Validates a complete in-memory schema-v2 state. It intentionally accepts no
 * inherited, hidden, symbol, or accessor properties at this boundary.
 */
export function parseV2WalkthroughState(value: unknown): V2WalkthroughState | undefined {
  if (!isExactArray(value, 9)) return undefined;
  const fragments: V2JourneyFragment[] = [];
  for (const [index, journey] of JOURNEYS.entries()) {
    const fragment = parseFragment(value[index], journey);
    if (fragment === undefined) return undefined;
    fragments.push(fragment);
  }
  if (!matchesPublicBindings(fragments)) return undefined;
  const first = fragments[0];
  const second = fragments[1];
  const third = fragments[2];
  const fourth = fragments[3];
  const fifth = fragments[4];
  const sixth = fragments[5];
  const seventh = fragments[6];
  const eighth = fragments[7];
  const ninth = fragments[8];
  if (
    first === undefined ||
    second === undefined ||
    third === undefined ||
    fourth === undefined ||
    fifth === undefined ||
    sixth === undefined ||
    seventh === undefined ||
    eighth === undefined ||
    ninth === undefined
  )
    return undefined;
  return { fragments: [first, second, third, fourth, fifth, sixth, seventh, eighth, ninth] };
}

/** Reads a canonical v2 state only through a mode-checked, no-follow descriptor. */
export function readV2WalkthroughState(path: string): V2WalkthroughState {
  const descriptor = openPrivateState(path);
  try {
    const metadata = fstatSync(descriptor);
    if (!metadata.isFile() || (metadata.mode & 0o777) !== 0o600 || metadata.size > 4096) {
      throw new Error("private v2 walkthrough state is unsafe");
    }
    const bytes = Buffer.alloc(metadata.size);
    if (
      readSync(descriptor, bytes, 0, metadata.size, 0) !== metadata.size ||
      bytes.some(notAscii)
    ) {
      throw new Error("private v2 walkthrough state is unsafe");
    }
    const raw = bytes.toString("ascii");
    if (!raw.endsWith("\n") || raw.includes("\r") || raw.split("\n").length !== 2) {
      throw new Error("private v2 walkthrough state is unsafe");
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      throw new Error("private v2 walkthrough state is unsafe");
    }
    if (JSON.stringify(parsed) + "\n" !== raw)
      throw new Error("private v2 walkthrough state is unsafe");
    const state = parseV2WalkthroughState(parsed);
    if (state === undefined) throw new Error("private v2 walkthrough state is unsafe");
    return state;
  } finally {
    closeSync(descriptor);
  }
}

/** Renders only public, non-identifying journey evidence in canonical JSON. */
export function renderV2VisibleOutcomeReport(
  masterSeed: number,
  value: unknown,
): string | undefined {
  if (!Number.isSafeInteger(masterSeed) || masterSeed < 0 || masterSeed > MAX_MASTER_SEED)
    return undefined;
  const fragments = isExactDataObject(value, ["fragments"]) ? ownValue(value, "fragments") : value;
  const state = parseV2WalkthroughState(fragments);
  if (state === undefined) return undefined;
  const journeys = state.fragments.map(toPublicJourneyRow);
  const elapsedMs = journeys.reduce(totalElapsed, 0);
  if (elapsedMs > MAX_ELAPSED_MS * JOURNEYS.length) return undefined;
  const report: PublicV2Report = {
    schemaVersion: 2,
    status: "PASS",
    masterSeed,
    stage: "complete",
    elapsedMs,
    arrangements: [{ label: "launcher-chapter-one-genetics" }],
    journeys,
  };
  const rendered = JSON.stringify(report) + "\n";
  return rendered;
}

function parseFragment(value: unknown, journey: V2Journey): V2JourneyFragment | undefined {
  const expectedKeys = keysForJourney(journey);
  if (!isExactDataObject(value, expectedKeys)) return undefined;
  const schemaVersion = ownValue(value, "schemaVersion");
  const status = ownValue(value, "status");
  const elapsedMs = ownValue(value, "elapsedMs");
  const courseId = ownValue(value, "courseId");
  const receivedJourney = ownValue(value, "journey");
  if (
    schemaVersion !== 2 ||
    receivedJourney !== journey ||
    status !== "PASS" ||
    !validElapsed(elapsedMs) ||
    typeof courseId !== "string" ||
    !UUID.test(courseId) ||
    !isExactStringArray(ownValue(value, "visibleOutcomeCodes"), JOURNEY_CODES[journey]) ||
    !isExactEmptyArray(ownValue(value, "diagnostics"))
  )
    return undefined;

  if (journey === "J13") {
    const assignmentId = ownValue(value, "assignmentId");
    const selectedDisplayIds = ownValue(value, "selectedDisplayIds");
    if (!validUuid(assignmentId) || !validSelectedDisplayIds(selectedDisplayIds)) return undefined;
    return createFragment(journey, elapsedMs, courseId, assignmentId, selectedDisplayIds);
  }
  if (journey === "J11" || journey === "J12") return createFragment(journey, elapsedMs, courseId);
  const assignmentId = ownValue(value, "assignmentId");
  if (!validUuid(assignmentId)) return undefined;
  return createFragment(journey, elapsedMs, courseId, assignmentId);
}

function createFragment(
  journey: V2Journey,
  elapsedMs: number,
  courseId: string,
  assignmentId?: string,
  selectedDisplayIds?: readonly [string, string, string, string],
): V2JourneyFragment {
  const base = {
    schemaVersion: 2 as const,
    journey,
    status: "PASS" as const,
    elapsedMs,
    courseId,
    visibleOutcomeCodes: JOURNEY_CODES[journey],
    diagnostics: [] as const,
  };
  if (journey === "J13" && assignmentId !== undefined && selectedDisplayIds !== undefined) {
    return { ...base, assignmentId, selectedDisplayIds };
  }
  if (assignmentId !== undefined) return { ...base, assignmentId };
  return base;
}

function keysForJourney(journey: V2Journey): readonly string[] {
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

function matchesPublicBindings(fragments: readonly V2JourneyFragment[]): boolean {
  const setup = fragments[2];
  if (setup === undefined || setup.assignmentId === undefined) return false;
  const courseId = setup.courseId;
  const assignmentId = setup.assignmentId;
  return fragments.every(matchesSetupBinding);

  function matchesSetupBinding(fragment: V2JourneyFragment): boolean {
    if (fragment.courseId !== courseId) return false;
    return (
      fragment.journey === "J11" ||
      fragment.journey === "J12" ||
      fragment.assignmentId === assignmentId
    );
  }
}

function toPublicJourneyRow(fragment: V2JourneyFragment): PublicJourneyRow {
  return {
    journey: fragment.journey,
    status: fragment.status,
    elapsedMs: fragment.elapsedMs,
    visibleOutcomeCodes: fragment.visibleOutcomeCodes,
    diagnostics: fragment.diagnostics,
  };
}

function totalElapsed(total: number, journey: PublicJourneyRow): number {
  return total + journey.elapsedMs;
}

function isExactDataObject(value: unknown, expectedKeys: readonly string[]): value is object {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  if (Object.getPrototypeOf(value) !== Object.prototype) return false;
  const keys = Reflect.ownKeys(value);
  if (keys.length !== expectedKeys.length || keys.some((key) => typeof key !== "string"))
    return false;
  return expectedKeys.every((key) => {
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

function ownValue(value: object, key: string): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  return descriptor?.value;
}

function isExactArray(value: unknown, length: number): value is readonly unknown[] {
  if (
    !Array.isArray(value) ||
    Object.getPrototypeOf(value) !== Array.prototype ||
    value.length !== length
  ) {
    return false;
  }
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

function isExactStringArray(value: unknown, expected: readonly string[]): boolean {
  if (!isExactArray(value, expected.length)) return false;
  return expected.every((item, index) => value[index] === item);
}

function isExactEmptyArray(value: unknown): value is readonly [] {
  return isExactArray(value, 0);
}

function validElapsed(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 0 &&
    value <= MAX_ELAPSED_MS
  );
}

function validUuid(value: unknown): value is string {
  return typeof value === "string" && UUID.test(value);
}

function openPrivateState(path: string): number {
  const parentPath = dirname(path);
  if (parentPath === "." || basename(path) !== "journeys.json") {
    throw new Error("private v2 walkthrough state is unsafe");
  }
  const directory = openSync(
    parentPath,
    constants.O_RDONLY | (constants.O_DIRECTORY ?? 0) | (constants.O_NOFOLLOW ?? 0),
  );
  try {
    const parent = fstatSync(directory);
    if (!parent.isDirectory() || (parent.mode & 0o777) !== 0o700) {
      throw new Error("private v2 walkthrough state is unsafe");
    }
    beforeV2ChildOpenForTest?.();
    let descriptor: number;
    try {
      descriptor = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
    } catch {
      throw new Error("private v2 walkthrough state is unsafe");
    }
    const currentParent = lstatSync(parentPath);
    const child = fstatSync(descriptor);
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
      throw new Error("private v2 walkthrough state is unsafe");
    }
    return descriptor;
  } finally {
    closeSync(directory);
  }
}

function notAscii(byte: number): boolean {
  return byte > 0x7f;
}
