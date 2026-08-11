// visible_outcome_report.ts - canonical public-only walkthrough outcome records.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
const OUTCOMES = ["PASS", "BLOCKED", "NOT_APPLICABLE", "FAIL"] as const;
const W1_CODES = [
  "visible_completion",
  "visible_feedback",
  "visible_response",
  "visible_start",
  "visible_submit",
] as const;
const W2_CODES = [
  "visible_completion",
  "visible_feedback",
  "visible_response",
  "visible_retry",
  "visible_start",
  "visible_submit",
] as const;
const DIAGNOSTICS = ["visible-control-unavailable", "visible-state-unavailable"] as const;
const ARRANGEMENT_KEYS: Readonly<Record<string, readonly string[]>> = {
  "launcher-seeded-enrollment": [],
  "launcher-baseline-assignment": ["baselineAssignmentId"],
  "api-retry-corpus-publication": ["problemId", "versionId"],
  "api-mastery-assignment": ["courseId", "masteryAssignmentId"],
  "api-exam-assignment": ["courseId", "examAssignmentId"],
};

export type JourneyOutcome = (typeof OUTCOMES)[number];
export type W1VisibleOutcomeCode = (typeof W1_CODES)[number];
export type W2VisibleOutcomeCode = (typeof W2_CODES)[number];

interface JourneyFragmentBase {
  readonly schemaVersion: 1;
  readonly status: JourneyOutcome;
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly assignmentId: string;
  readonly diagnostics: readonly string[];
}

export interface W1JourneyFragment extends JourneyFragmentBase {
  readonly journey: "J1";
  readonly visibleOutcomeCodes: readonly W1VisibleOutcomeCode[];
}

export interface W2JourneyFragment extends JourneyFragmentBase {
  readonly journey: "J2";
  readonly visibleOutcomeCodes: readonly W2VisibleOutcomeCode[];
}

export interface J3JourneyFragment extends JourneyFragmentBase {
  readonly journey: "J3";
  readonly visibleOutcomeCodes: readonly ("visible_leave" | "visible_return" | "visible_start")[];
}

export interface J4JourneyFragment {
  readonly schemaVersion: 1;
  readonly journey: "J4";
  readonly status: JourneyOutcome;
  readonly elapsedMs: number;
  readonly courseId: string;
  readonly masteryAssignmentId: string;
  readonly examAssignmentId: string;
  readonly visibleOutcomeCodes: readonly (
    | "visible_back_action"
    | "visible_exam_closed"
    | "visible_exam_completion"
    | "visible_mastery_completion"
    | "visible_mastery_fresh_practice"
  )[];
  readonly diagnostics: readonly string[];
}

export interface J5JourneyFragment extends JourneyFragmentBase {
  readonly journey: "J5";
  readonly visibleOutcomeCodes: readonly ("visible_gradebook" | "visible_run_history")[];
}

export interface J8JourneyFragment extends JourneyFragmentBase {
  readonly journey: "J8";
  readonly visibleOutcomeCodes: readonly (
    "visible_instructor_gradebook" | "visible_learner_completion"
  )[];
}

export type JourneyFragment =
  | W1JourneyFragment
  | W2JourneyFragment
  | J3JourneyFragment
  | J4JourneyFragment
  | J5JourneyFragment
  | J8JourneyFragment;

export interface ArrangementRecord {
  readonly label: string;
  readonly publicIds: Readonly<Record<string, string>>;
}

export interface VisibleOutcomeReport {
  readonly schemaVersion: 1;
  readonly status: "PASS" | "FAIL";
  readonly masterSeed: number;
  readonly stage: "complete";
  readonly elapsedMs: number;
  readonly arrangements: readonly ArrangementRecord[];
  readonly journeys: readonly JourneyFragment[];
}

const MAX_ELAPSED_MS = 30 * 60 * 1000;
const MAX_DIAGNOSTICS = 4;
const MAX_DIAGNOSTIC_LENGTH = 96;

export function passedW1Fragment(
  courseId: string,
  assignmentId: string,
  elapsedMs: number,
): W1JourneyFragment {
  return {
    schemaVersion: 1,
    journey: "J1",
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: W1_CODES,
    diagnostics: [],
  };
}

export function passedW2Fragment(
  courseId: string,
  assignmentId: string,
  elapsedMs: number,
): W2JourneyFragment {
  return {
    schemaVersion: 1,
    journey: "J2",
    status: "PASS",
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: W2_CODES,
    diagnostics: [],
  };
}

export function parseW1JourneyFragment(value: unknown): W1JourneyFragment | undefined {
  const parsed = parseJourneyFragment(value, "J1", W1_CODES);
  if (parsed === undefined || !isW1Codes(parsed.visibleOutcomeCodes)) return undefined;
  return {
    schemaVersion: 1,
    journey: "J1",
    status: parsed.status,
    elapsedMs: parsed.elapsedMs,
    courseId: parsed.courseId,
    assignmentId: parsed.assignmentId,
    visibleOutcomeCodes: parsed.visibleOutcomeCodes,
    diagnostics: parsed.diagnostics,
  };
}

export function parseW2JourneyFragment(value: unknown): W2JourneyFragment | undefined {
  const parsed = parseJourneyFragment(value, "J2", W2_CODES);
  if (parsed === undefined || !isW2Codes(parsed.visibleOutcomeCodes)) return undefined;
  return {
    schemaVersion: 1,
    journey: "J2",
    status: parsed.status,
    elapsedMs: parsed.elapsedMs,
    courseId: parsed.courseId,
    assignmentId: parsed.assignmentId,
    visibleOutcomeCodes: parsed.visibleOutcomeCodes,
    diagnostics: parsed.diagnostics,
  };
}

export function parseJourneyFragments(
  value: unknown,
):
  | readonly [
      W1JourneyFragment,
      W2JourneyFragment,
      J3JourneyFragment,
      J4JourneyFragment,
      J5JourneyFragment,
      J8JourneyFragment,
    ]
  | undefined {
  if (!Array.isArray(value) || value.length !== 6) return undefined;
  const w1 = parseW1JourneyFragment(value[0]);
  const w2 = parseW2JourneyFragment(value[1]);
  const j3 = parseFixedPassFragment(value[2], "J3", [
    "visible_leave",
    "visible_return",
    "visible_start",
  ]);
  const j4 = parseJ4JourneyFragment(value[3]);
  const j5 = parseFixedPassFragment(value[4], "J5", ["visible_gradebook", "visible_run_history"]);
  const j8 = parseFixedPassFragment(value[5], "J8", [
    "visible_instructor_gradebook",
    "visible_learner_completion",
  ]);
  if (
    w1 === undefined ||
    w2 === undefined ||
    j3 === undefined ||
    j4 === undefined ||
    j5 === undefined ||
    j8 === undefined ||
    w1.courseId !== w2.courseId ||
    w1.assignmentId !== w2.assignmentId ||
    w1.courseId !== j3.courseId ||
    w1.assignmentId !== (j3 as J3JourneyFragment).assignmentId ||
    w1.courseId !== j4.courseId ||
    w1.assignmentId !== j4.masteryAssignmentId ||
    w1.courseId !== j5.courseId ||
    w1.assignmentId !== (j5 as J5JourneyFragment).assignmentId ||
    w1.courseId !== j8.courseId ||
    w1.assignmentId !== (j8 as J8JourneyFragment).assignmentId
  )
    return undefined;
  return [w1, w2, j3 as J3JourneyFragment, j4, j5 as J5JourneyFragment, j8 as J8JourneyFragment];
}

export function parseJourneyPrefix(value: unknown): readonly JourneyFragment[] | undefined {
  if (!Array.isArray(value) || value.length > 6) return undefined;
  const parsers = [
    parseW1JourneyFragment,
    parseW2JourneyFragment,
    (item: unknown): JourneyFragment | undefined =>
      parseFixedPassFragment(item, "J3", ["visible_leave", "visible_return", "visible_start"]),
    parseJ4JourneyFragment,
    (item: unknown): JourneyFragment | undefined =>
      parseFixedPassFragment(item, "J5", ["visible_gradebook", "visible_run_history"]),
    (item: unknown): JourneyFragment | undefined =>
      parseFixedPassFragment(item, "J8", [
        "visible_instructor_gradebook",
        "visible_learner_completion",
      ]),
  ];
  const output: JourneyFragment[] = [];
  for (let index = 0; index < value.length; index += 1) {
    const parsed = parsers[index]?.(value[index]);
    if (parsed === undefined) return undefined;
    output.push(parsed);
  }
  return output;
}

function parseFixedPassFragment(
  value: unknown,
  journey: "J3" | "J5" | "J8",
  codes: readonly string[],
): JourneyFragment | undefined {
  const parsed = parseJourneyFragment(value, journey, codes);
  if (
    parsed === undefined ||
    parsed.status !== "PASS" ||
    !sameStrings(parsed.visibleOutcomeCodes, codes)
  )
    return undefined;
  return parsed as JourneyFragment;
}

function parseJ4JourneyFragment(value: unknown): J4JourneyFragment | undefined {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "schemaVersion",
      "journey",
      "status",
      "elapsedMs",
      "courseId",
      "masteryAssignmentId",
      "examAssignmentId",
      "visibleOutcomeCodes",
      "diagnostics",
    ])
  )
    return undefined;
  const codes = [
    "visible_back_action",
    "visible_exam_closed",
    "visible_exam_completion",
    "visible_mastery_completion",
    "visible_mastery_fresh_practice",
  ];
  const parsedCodes = parseCodes(ownValue(value, "visibleOutcomeCodes"), codes);
  if (
    ownValue(value, "schemaVersion") !== 1 ||
    ownValue(value, "journey") !== "J4" ||
    ownValue(value, "status") !== "PASS" ||
    !isElapsed(ownValue(value, "elapsedMs")) ||
    !isUuid(ownValue(value, "courseId")) ||
    !isUuid(ownValue(value, "masteryAssignmentId")) ||
    !isUuid(ownValue(value, "examAssignmentId")) ||
    !sameStrings(parsedCodes ?? [], codes) ||
    !Array.isArray(ownValue(value, "diagnostics")) ||
    (ownValue(value, "diagnostics") as unknown[]).length !== 0
  )
    return undefined;
  return value as unknown as J4JourneyFragment;
}

interface ParsedJourneyFragment extends JourneyFragmentBase {
  readonly journey: "J1" | "J2" | "J3" | "J5" | "J8";
  readonly visibleOutcomeCodes: readonly string[];
}

function parseJourneyFragment(
  value: unknown,
  journey: "J1" | "J2" | "J3" | "J5" | "J8",
  allowedCodes: readonly string[],
): ParsedJourneyFragment | undefined {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "schemaVersion",
      "journey",
      "status",
      "elapsedMs",
      "courseId",
      "assignmentId",
      "visibleOutcomeCodes",
      "diagnostics",
    ])
  )
    return undefined;
  const schemaVersion = ownValue(value, "schemaVersion");
  const actualJourney = ownValue(value, "journey");
  const status = ownValue(value, "status");
  const elapsedMs = ownValue(value, "elapsedMs");
  const courseId = ownValue(value, "courseId");
  const assignmentId = ownValue(value, "assignmentId");
  const diagnostics = ownValue(value, "diagnostics");
  const visibleOutcomeCodes = ownValue(value, "visibleOutcomeCodes");
  if (
    schemaVersion !== 1 ||
    actualJourney !== journey ||
    !isOutcome(status) ||
    !isElapsed(elapsedMs) ||
    !isUuid(courseId) ||
    !isUuid(assignmentId) ||
    !isStringArray(diagnostics) ||
    diagnostics.length > MAX_DIAGNOSTICS ||
    diagnostics.some((diagnostic) => !isDiagnostic(diagnostic))
  )
    return undefined;
  const codes = parseCodes(visibleOutcomeCodes, allowedCodes);
  if (
    codes === undefined ||
    (status !== "PASS" && status !== "FAIL") ||
    (status === "PASS" && (diagnostics.length !== 0 || !sameStrings(codes, allowedCodes))) ||
    (status === "FAIL" && (diagnostics.length !== 1 || sameStrings(codes, allowedCodes)))
  )
    return undefined;
  return {
    schemaVersion: 1,
    journey,
    status,
    elapsedMs,
    courseId,
    assignmentId,
    visibleOutcomeCodes: codes,
    diagnostics: [...diagnostics].sort(),
  };
}

export function renderVisibleOutcomeReport(
  masterSeed: number,
  arrangements: unknown,
  journeys: unknown,
): string | undefined {
  if (!isUint32(masterSeed) || !Array.isArray(arrangements) || arrangements.length !== 5)
    return undefined;
  const parsedJourneys = parseJourneyFragments(journeys);
  const parsedArrangements = arrangements.map(parseArrangement);
  if (parsedJourneys === undefined) return undefined;
  const completeArrangements: ArrangementRecord[] = [];
  for (const arrangement of parsedArrangements) {
    if (arrangement === undefined) return undefined;
    completeArrangements.push(arrangement);
  }
  if (
    new Set(completeArrangements.map((arrangement) => arrangement.label)).size !==
      arrangements.length ||
    !sameStrings(
      completeArrangements.map((arrangement) => arrangement.label).sort(),
      Object.keys(ARRANGEMENT_KEYS).sort(),
    )
  )
    return undefined;
  const canonicalArrangements = completeArrangements
    .map((arrangement) => ({
      label: arrangement.label,
      publicIds: Object.fromEntries(
        Object.entries(arrangement.publicIds).sort(([left], [right]) => left.localeCompare(right)),
      ),
    }))
    .sort((left, right) => left.label.localeCompare(right.label));
  const elapsedMs = parsedJourneys.reduce((total, journey) => total + journey.elapsedMs, 0);
  if (!isElapsed(elapsedMs)) return undefined;
  const report: VisibleOutcomeReport = {
    schemaVersion: 1,
    status: parsedJourneys.every((journey) => journey.status === "PASS") ? "PASS" : "FAIL",
    masterSeed,
    stage: "complete",
    elapsedMs,
    arrangements: canonicalArrangements,
    journeys: parsedJourneys,
  };
  return JSON.stringify(report) + "\n";
}

function parseArrangement(value: unknown): ArrangementRecord | undefined {
  if (!isRecord(value) || !hasExactKeys(value, ["label", "publicIds"])) return undefined;
  const label = ownValue(value, "label");
  const publicIds = ownValue(value, "publicIds");
  if (typeof label !== "string" || !isRecord(publicIds)) return undefined;
  if (!Object.prototype.hasOwnProperty.call(ARRANGEMENT_KEYS, label)) return undefined;
  const expectedKeys = ARRANGEMENT_KEYS[label];
  if (expectedKeys === undefined || !hasExactKeys(publicIds, expectedKeys)) return undefined;
  const parsedPublicIds: Record<string, string> = {};
  for (const key of expectedKeys) {
    const identifier = ownValue(publicIds, key);
    if (!isUuid(identifier)) return undefined;
    parsedPublicIds[key] = identifier;
  }
  return { label, publicIds: parsedPublicIds };
}

function parseCodes(
  value: unknown,
  allowedCodes: readonly string[],
): readonly string[] | undefined {
  if (!isStringArray(value)) return undefined;
  const sorted = [...value].sort();
  if (new Set(sorted).size !== sorted.length || sorted.some((code) => !allowedCodes.includes(code)))
    return undefined;
  return sorted;
}

function isW1Codes(value: readonly string[]): value is readonly W1VisibleOutcomeCode[] {
  return value.every((code): code is W1VisibleOutcomeCode =>
    W1_CODES.includes(code as W1VisibleOutcomeCode),
  );
}

function isW2Codes(value: readonly string[]): value is readonly W2VisibleOutcomeCode[] {
  return value.every((code): code is W2VisibleOutcomeCode =>
    W2_CODES.includes(code as W2VisibleOutcomeCode),
  );
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function hasExactKeys(value: Readonly<Record<string, unknown>>, keys: readonly string[]): boolean {
  const actualKeys = Reflect.ownKeys(value);
  if (actualKeys.length !== keys.length) return false;
  const stringKeys: string[] = [];
  for (const key of actualKeys) {
    if (typeof key !== "string") return false;
    stringKeys.push(key);
  }
  const enumerable = stringKeys.every((key) => {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    return descriptor?.enumerable === true && "value" in descriptor;
  });
  return enumerable && sameStrings(stringKeys.sort(), [...keys].sort());
}
function ownValue(value: Readonly<Record<string, unknown>>, key: string): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  return descriptor !== undefined && descriptor.enumerable && "value" in descriptor
    ? descriptor.value
    : undefined;
}
function sameStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}
function isOutcome(value: unknown): value is JourneyOutcome {
  return typeof value === "string" && OUTCOMES.includes(value as JourneyOutcome);
}
function isElapsed(value: unknown): value is number {
  return (
    typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= MAX_ELAPSED_MS
  );
}
function isUint32(value: number): boolean {
  return Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
}
function isUuid(value: unknown): value is string {
  return typeof value === "string" && UUID.test(value);
}
function isStringArray(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}
function isDiagnostic(value: string): boolean {
  return (
    value.length <= MAX_DIAGNOSTIC_LENGTH &&
    DIAGNOSTICS.includes(value as (typeof DIAGNOSTICS)[number])
  );
}
