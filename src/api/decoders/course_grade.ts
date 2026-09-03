// Strict browser decoder for the instructor-only course-grade API.

import type { CourseGradeAssignmentSetting } from "../../../generated/api/CourseGradeAssignmentSetting";
import type { CourseGradeAssignmentView } from "../../../generated/api/CourseGradeAssignmentView";
import type { CourseGradeOutcomeView } from "../../../generated/api/CourseGradeOutcomeView";
import type { CourseGradeScheme } from "../../../generated/api/CourseGradeScheme";
import type { CourseGradeSchemeUpdateView } from "../../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseGradeSchemeView } from "../../../generated/api/CourseGradeSchemeView";
import type { CourseGradebookTotalsView } from "../../../generated/api/CourseGradebookTotalsView";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeField,
  decodeFiniteNumber,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";

const MODES = ["totalPoints", "weightedCategories"] as const;
const ROUNDING = ["fourDecimalPlacesHalfAwayFromZero"] as const;
const REASONS = [
  "noIncludedAssignments",
  "recalculating",
  "failed",
  "emptyAfterDrop",
  "zeroPossiblePoints",
] as const;

function closed(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  for (const key of Object.keys(record))
    if (!keys.includes(key)) throw new DecodeError(`${path}.${key}`, "a known field");
  for (const key of keys) decodeField(record, key, path);
  return record;
}

function trimmedText(value: unknown, path: string, maximum: number): string {
  const text = decodeString(value, path);
  const count = Array.from(text).length;
  if (text.trim() !== text || count < 1 || count > maximum)
    throw new DecodeError(path, `trimmed text of 1 to ${maximum} Unicode scalars`);
  return text;
}

function position(value: unknown, path: string): number {
  const parsed = decodeSafeInteger(value, path);
  if (parsed < 0 || parsed > 10_000) throw new DecodeError(path, "a position from 0 to 10000");
  return parsed;
}

function category(value: unknown, path: string): CourseGradeScheme["categories"][number] {
  const record = closed(value, path, [
    "id",
    "title",
    "position",
    "weightBasisPoints",
    "dropLowest",
  ]);
  const weightBasisPoints = decodeSafeInteger(
    record.weightBasisPoints,
    `${path}.weightBasisPoints`,
  );
  const dropLowest = decodeSafeInteger(record.dropLowest, `${path}.dropLowest`);
  if (weightBasisPoints < 1 || weightBasisPoints > 10_000)
    throw new DecodeError(`${path}.weightBasisPoints`, "a weight from 1 to 10000 basis points");
  if (dropLowest < 0 || dropLowest > 10_000)
    throw new DecodeError(`${path}.dropLowest`, "a nonnegative bounded drop count");
  return {
    id: decodeUuid(record.id, `${path}.id`),
    title: trimmedText(record.title, `${path}.title`, 200),
    position: position(record.position, `${path}.position`),
    weightBasisPoints,
    dropLowest,
  };
}

function letterBand(value: unknown, path: string): CourseGradeScheme["letterBands"][number] {
  const record = closed(value, path, ["label", "minimumBasisPoints"]);
  const minimumBasisPoints = decodeSafeInteger(
    record.minimumBasisPoints,
    `${path}.minimumBasisPoints`,
  );
  if (minimumBasisPoints < 0 || minimumBasisPoints > 10_000)
    throw new DecodeError(`${path}.minimumBasisPoints`, "a threshold from 0 to 10000 basis points");
  return { label: trimmedText(record.label, `${path}.label`, 32), minimumBasisPoints };
}

function decodeScheme(value: unknown, path: string): CourseGradeScheme {
  const record = closed(value, path, ["mode", "rounding", "categories", "letterBands"]);
  const mode = decodeStringEnum(record.mode, `${path}.mode`, MODES);
  const categories = decodeArray(record.categories, `${path}.categories`, category);
  const letterBands = decodeArray(record.letterBands, `${path}.letterBands`, letterBand);
  if (
    new Set(categories.map((item) => item.id)).size !== categories.length ||
    !categories.every((item, index) => item.position === index)
  )
    throw new DecodeError(
      `${path}.categories`,
      "unique category IDs with no out-of-sequence category positions",
    );
  if (mode === "totalPoints" && categories.length !== 0)
    throw new DecodeError(`${path}.categories`, "no categories for total points");
  if (
    mode === "weightedCategories" &&
    (categories.length === 0 ||
      categories.reduce((total, item) => total + item.weightBasisPoints, 0) !== 10_000)
  )
    throw new DecodeError(`${path}.categories`, "nonempty weights totaling 10000 basis points");
  if (new Set(letterBands.map((item) => item.label)).size !== letterBands.length)
    throw new DecodeError(`${path}.letterBands`, "unique letter labels");
  for (let index = 1; index < letterBands.length; index += 1)
    if (letterBands[index - 1]!.minimumBasisPoints <= letterBands[index]!.minimumBasisPoints)
      throw new DecodeError(`${path}.letterBands`, "strictly descending thresholds");
  return {
    mode,
    rounding: decodeStringEnum(record.rounding, `${path}.rounding`, ROUNDING),
    categories,
    letterBands,
  };
}

function decodeAssignment(
  value: unknown,
  path: string,
  includeTitle: true,
): CourseGradeAssignmentView;
function decodeAssignment(
  value: unknown,
  path: string,
  includeTitle: false,
): CourseGradeAssignmentSetting;
function decodeAssignment(
  value: unknown,
  path: string,
  includeTitle: boolean,
): CourseGradeAssignmentView | CourseGradeAssignmentSetting {
  const keys = includeTitle
    ? ["assignment", "title", "included", "category", "position"]
    : ["assignment", "included", "category", "position"];
  const record = closed(value, path, keys);
  const categoryId = decodeNullable(record.category, `${path}.category`, decodeUuid);
  const assignmentPosition = decodeNullable(record.position, `${path}.position`, position);
  if ((categoryId === null) !== (assignmentPosition === null))
    throw new DecodeError(path, "paired category and position values");
  const decoded = {
    assignment: decodeUuid(record.assignment, `${path}.assignment`),
    included: decodeBoolean(record.included, `${path}.included`),
    category: categoryId,
    position: assignmentPosition,
  };
  return includeTitle
    ? { ...decoded, title: trimmedText(record.title, `${path}.title`, 200) }
    : decoded;
}

function validateAssignments(
  assignments: ReadonlyArray<CourseGradeAssignmentView | CourseGradeAssignmentSetting>,
  parsedScheme: CourseGradeScheme,
  path: string,
  isWrite: boolean,
): void {
  if (new Set(assignments.map((item) => item.assignment)).size !== assignments.length)
    throw new DecodeError(path, "unique assignment IDs");
  const positions = new Map<string, number[]>();
  for (const assignment of assignments) {
    if (parsedScheme.mode === "totalPoints" && assignment.category !== null)
      throw new DecodeError(path, "unmapped total-points assignments");
    if (
      parsedScheme.mode === "weightedCategories" &&
      isWrite &&
      assignment.included &&
      assignment.category === null
    )
      throw new DecodeError(path, "included weighted assignments mapped to a category");
    if (assignment.category === null) continue;
    if (!parsedScheme.categories.some((item) => item.id === assignment.category))
      throw new DecodeError(path, "assignments mapped to known categories");
    const categoryPositions = positions.get(assignment.category) ?? [];
    categoryPositions.push(assignment.position!);
    positions.set(assignment.category, categoryPositions);
  }
  for (const categoryPositions of positions.values()) {
    categoryPositions.sort((left, right) => left - right);
    if (!categoryPositions.every((item, index) => item === index))
      throw new DecodeError(
        path,
        "no out-of-sequence category positions across mapped assignments",
      );
  }
}

export function decodeCourseGradeSchemeView(
  value: unknown,
  path = "response",
): CourseGradeSchemeView {
  const record = closed(value, path, ["scheme", "assignments"]);
  const parsedScheme = decodeScheme(record.scheme, `${path}.scheme`);
  const assignments = decodeArray(record.assignments, `${path}.assignments`, (entry, entryPath) =>
    decodeAssignment(entry, entryPath, true),
  );
  validateAssignments(assignments, parsedScheme, `${path}.assignments`, false);
  return { scheme: parsedScheme, assignments };
}

export function decodeCourseGradeSchemeUpdateView(
  value: unknown,
  path = "request",
): CourseGradeSchemeUpdateView {
  const record = closed(value, path, ["scheme", "assignments"]);
  const parsedScheme = decodeScheme(record.scheme, `${path}.scheme`);
  const assignments = decodeArray(record.assignments, `${path}.assignments`, (entry, entryPath) =>
    decodeAssignment(entry, entryPath, false),
  );
  validateAssignments(assignments, parsedScheme, `${path}.assignments`, true);
  return { scheme: parsedScheme, assignments };
}

function decodeOutcome(
  value: unknown,
  mode: CourseGradeScheme["mode"],
  path: string,
): CourseGradeOutcomeView {
  const record = decodeRecord(value, path);
  const status = decodeStringEnum(decodeField(record, "status", path), `${path}.status`, [
    "available",
    "unavailable",
  ] as const);
  if (status === "unavailable")
    return {
      status,
      reason: decodeStringEnum(
        closed(record, path, ["status", "reason"]).reason,
        `${path}.reason`,
        REASONS,
      ),
    };
  const available = closed(record, path, [
    "status",
    "score",
    "letter",
    "droppedAssignmentIds",
    "totalEarned",
    "totalPossible",
  ]);
  const droppedAssignmentIds = decodeArray(
    available.droppedAssignmentIds,
    `${path}.droppedAssignmentIds`,
    decodeUuid,
  );
  if (new Set(droppedAssignmentIds).size !== droppedAssignmentIds.length)
    throw new DecodeError(`${path}.droppedAssignmentIds`, "unique assignment IDs");
  const totalEarned = decodeNullable(
    available.totalEarned,
    `${path}.totalEarned`,
    decodeFiniteNumber,
  );
  const totalPossible = decodeNullable(
    available.totalPossible,
    `${path}.totalPossible`,
    decodeFiniteNumber,
  );
  if (
    mode === "totalPoints" &&
    (totalEarned === null ||
      totalPossible === null ||
      totalPossible <= 0 ||
      droppedAssignmentIds.length !== 0)
  )
    throw new DecodeError(path, "a complete total-points outcome");
  if (mode === "weightedCategories" && (totalEarned !== null || totalPossible !== null))
    throw new DecodeError(path, "a weighted outcome without point totals");
  return {
    status,
    score: decodeFiniteNumber(available.score, `${path}.score`),
    letter: decodeNullable(available.letter, `${path}.letter`, (entry, entryPath) =>
      trimmedText(entry, entryPath, 32),
    ),
    droppedAssignmentIds,
    totalEarned,
    totalPossible,
  };
}

export function decodeCourseGradebookTotalsView(
  value: unknown,
  path = "response",
): CourseGradebookTotalsView {
  const record = closed(value, path, ["mode", "rounding", "rows"]);
  const mode = decodeStringEnum(record.mode, `${path}.mode`, MODES);
  const rows = decodeArray(record.rows, `${path}.rows`, (entry, entryPath) => {
    const row = closed(entry, entryPath, ["displayName", "outcome"]);
    return {
      displayName: trimmedText(row.displayName, `${entryPath}.displayName`, 200),
      outcome: decodeOutcome(row.outcome, mode, `${entryPath}.outcome`),
    };
  });
  return { mode, rounding: decodeStringEnum(record.rounding, `${path}.rounding`, ROUNDING), rows };
}
