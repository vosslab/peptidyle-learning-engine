// Strict browser contracts for the roster-first Gradebook and audited Student work.

import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { QuestionAssetRendition } from "../../../generated/api/QuestionAssetRendition";
import type { CourseGradeMode } from "../../../generated/api/CourseGradeMode";
import type { CourseGradeRoundingRule } from "../../../generated/api/CourseGradeRoundingRule";
import type { CourseMembershipReference } from "../../../generated/api/CourseMembershipReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { InstructorGradingOperationReference } from "../../../generated/api/InstructorGradingOperationReference";
import type { StudentResponseInspection } from "../../../generated/api/StudentResponseInspection";
import type { StudentResponseInspectionFeedback } from "../../../generated/api/StudentResponseInspectionFeedback";
import type { QuestionPresentation } from "../../../generated/api/QuestionPresentation";
import type { AssignmentAttemptReference } from "../../../generated/api/AssignmentAttemptReference";
import type { AssignmentScoringState } from "../../../generated/api/AssignmentScoringState";
import {
  DecodeError,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeIssuedQuestionPresentation } from "./presentation_delivery";
import { decodeQuestionAssetReference } from "./question_response_format";
import { decodeInstructorGradingOperationReference } from "./grading_operations";
import {
  decodeAssignmentInspectionChoice,
  type AssignmentInspectionChoice,
} from "./gradebook_selection";
import {
  MAX_CURSOR_PAGE_ITEMS,
  decodeBoundedArray,
  decodeAssignmentTitle,
  decodeCursor,
  decodeIdentifier,
  decodeSha256,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";

const MODES = [
  "totalPoints",
  "weightedCategories",
] as const satisfies ReadonlyArray<CourseGradeMode>;
const ROUNDING_RULES = [
  "fourDecimalPlacesHalfAwayFromZero",
] as const satisfies ReadonlyArray<CourseGradeRoundingRule>;
const ASSIGNMENT_SCORING_STATES = [
  "current",
  "recalculating",
  "failed",
] as const satisfies ReadonlyArray<AssignmentScoringState>;
const COURSE_GRADE_UNAVAILABLE_REASONS = [
  "noIncludedAssignments",
  "recalculating",
  "failed",
  "emptyAfterDrop",
  "zeroPossiblePoints",
] as const;
const RELOAD_REASONS = ["schemeChanged", "rosterChanged", "filterChanged"] as const;
const MAX_PRESENTED_ITEMS = 32;
const MAX_INSPECTED_SUBMISSIONS = MAX_ASSIGNMENT_ORDERED_ENTRIES;
// One presentation permits 32 prompt blocks and at most 32 Presentation Response Items,
// each with at most 32 content blocks. The transport supplies the byte bound.
const MAX_ASSET_BINDINGS = MAX_PRESENTED_ITEMS * (MAX_PRESENTED_ITEMS + 1);
const PRESENTATION_RESPONSE_ITEM_REFERENCE = /^[0-9a-f]{4}$/u;

export type CalculatedCourseGradeOutcome =
  | {
      readonly status: "available";
      readonly score: number;
      readonly letter: string | null;
      readonly droppedAssignments: ReadonlyArray<AssignmentReference>;
      readonly totalEarned: number | null;
      readonly totalPossible: number | null;
    }
  | {
      readonly status: "unavailable";
      readonly reason: (typeof COURSE_GRADE_UNAVAILABLE_REASONS)[number];
    };

export interface CalculatedAssignmentCell {
  readonly assignment: AssignmentReference;
  readonly title: string;
  readonly included: boolean;
  readonly category: string | null;
  readonly availability: "available" | "unavailable";
  readonly selectedScore: number | null;
  readonly assignmentScoringState: AssignmentScoringState;
  readonly inspectionChoice: AssignmentInspectionChoice;
}

export interface CalculatedGradebookRow {
  readonly membership: CourseMembershipReference;
  readonly displayLabel: string;
  readonly outcome: CalculatedCourseGradeOutcome;
  readonly assignmentCells: ReadonlyArray<CalculatedAssignmentCell>;
}

export interface AssignmentScoringSnapshot {
  readonly assignment: AssignmentReference;
  readonly generation: number;
  readonly assignmentScoringState: AssignmentScoringState;
}

export type CalculatedGradebookResult =
  | {
      readonly kind: "page";
      readonly schemeRevision: number;
      readonly rosterRevision: number;
      readonly mode: CourseGradeMode;
      readonly rounding: CourseGradeRoundingRule;
      readonly observationTime: number;
      readonly assignmentScoringSnapshots: ReadonlyArray<AssignmentScoringSnapshot>;
      readonly nextCursor: string | null;
      readonly rows: ReadonlyArray<CalculatedGradebookRow>;
    }
  | { readonly kind: "reloadRequired"; readonly reason: (typeof RELOAD_REASONS)[number] };

export type CalculatedGradebookFilter =
  | { readonly kind: "all" }
  | { readonly kind: "assignment"; readonly assignment: AssignmentReference }
  | { readonly kind: "student"; readonly membership: CourseMembershipReference }
  | { readonly kind: "operation"; readonly operation: InstructorGradingOperationReference };

export interface CalculatedGradebookQuery {
  readonly cursor?: string;
  readonly pageSize?: number;
  readonly filter?: CalculatedGradebookFilter;
}

export type InspectedSubmissionEvidence =
  | {
      readonly kind: "issuedPresentation";
      readonly question: QuestionPresentation;
      readonly questionAssetRenditions: ReadonlyArray<QuestionAssetRendition>;
      readonly issuedPresentationChecksum: string;
    }
  | { readonly kind: "presentationNotApplicable" };

export interface InspectedStudentSubmission {
  readonly submittedAt: number;
  readonly evidence: InspectedSubmissionEvidence;
  readonly scoringGeneration: number;
  readonly feedback: StudentResponseInspectionFeedback;
  readonly response: StudentResponseInspection;
  readonly assignmentScoringState: AssignmentScoringState;
}

export interface InspectedStudentWorkDetail {
  readonly course: CourseInstanceReference;
  readonly membership: CourseMembershipReference;
  readonly assignment: AssignmentReference;
  readonly assignmentAttempt: AssignmentAttemptReference;
  /** Current roster presentation label; never immutable evidence or an audit fact. */
  readonly studentDisplayLabel: string;
  /** Current assignment presentation title; never immutable evidence or an audit fact. */
  readonly assignmentTitle: string;
  readonly submissions: ReadonlyArray<InspectedStudentSubmission>;
  readonly returnContext: InspectedStudentWorkReturnContext;
}

export type InspectedStudentWorkReturnContext =
  | {
      readonly kind: "gradebook";
      readonly course: CourseInstanceReference;
      readonly membership: CourseMembershipReference;
      readonly assignment: AssignmentReference;
      readonly focus: {
        readonly kind: "gradebookCell";
        readonly membership: CourseMembershipReference;
        readonly assignment: AssignmentReference;
      };
    }
  | {
      readonly kind: "gradingOperation";
      readonly course: CourseInstanceReference;
      readonly membership: CourseMembershipReference;
      readonly assignment: AssignmentReference;
      readonly operation: InstructorGradingOperationReference;
      readonly focus: {
        readonly kind: "gradingOperationControl";
        readonly membership: CourseMembershipReference;
        readonly assignment: AssignmentReference;
        readonly operation: InstructorGradingOperationReference;
      };
    };

function closed(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, keys);
  for (const key of keys) field(record, key, path);
  return record;
}

function optionalField(record: Record<string, unknown>, key: string): unknown {
  return Object.prototype.hasOwnProperty.call(record, key) ? record[key] : undefined;
}

function publicReference(value: unknown, path: string, prefix: "A" | "C" | "M" | "R"): string {
  const reference = decodeString(value, path);
  const pattern = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  const numericPart = reference.slice(prefix.length + 1);
  if (!pattern.test(reference) || Number(numericPart) > 2_147_483_647) {
    throw new DecodeError(path, `a canonical ${prefix}- reference`);
  }
  return reference;
}

function positiveSafeInteger(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 1) throw new DecodeError(path, "a positive browser-safe integer");
  return decoded;
}

function boundedDisplayLabel(value: unknown, path: string): string {
  const label = decodeString(value, path);
  if (
    label.trim() !== label ||
    label.length === 0 ||
    Array.from(label).length > MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS
  ) {
    throw new DecodeError(
      path,
      `trimmed text of 1 to ${MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS} Unicode scalars`,
    );
  }
  return label;
}

function nullableFiniteNumber(value: unknown, path: string): number | null {
  return decodeNullable(value, path, decodeFiniteNumber);
}

function decodeCourseGradeOutcome(value: unknown, path: string): CalculatedCourseGradeOutcome {
  const record = decodeRecord(value, path);
  const status = decodeString(field(record, "status", path), `${path}.status`);
  if (status === "available") {
    requireOnlyFields(record, path, [
      "status",
      "score",
      "letter",
      "droppedAssignments",
      "totalEarned",
      "totalPossible",
    ]);
    return {
      status,
      score: decodeFiniteNumber(field(record, "score", path), `${path}.score`),
      letter: decodeNullable(field(record, "letter", path), `${path}.letter`, decodeString),
      droppedAssignments: decodeBoundedArray(
        field(record, "droppedAssignments", path),
        `${path}.droppedAssignments`,
        MAX_ASSIGNMENT_ORDERED_ENTRIES,
        (item, itemPath) => publicReference(item, itemPath, "A"),
      ),
      totalEarned: nullableFiniteNumber(field(record, "totalEarned", path), `${path}.totalEarned`),
      totalPossible: nullableFiniteNumber(
        field(record, "totalPossible", path),
        `${path}.totalPossible`,
      ),
    };
  }
  if (status === "unavailable") {
    requireOnlyFields(record, path, ["status", "reason"]);
    return {
      status,
      reason: decodeStringEnum(
        field(record, "reason", path),
        `${path}.reason`,
        COURSE_GRADE_UNAVAILABLE_REASONS,
      ),
    };
  }
  throw new DecodeError(`${path}.status`, "available or unavailable");
}

function decodeAssignmentCell(value: unknown, path: string): CalculatedAssignmentCell {
  const record = closed(value, path, [
    "assignment",
    "title",
    "included",
    "category",
    "availability",
    "selectedScore",
    "assignmentScoringState",
    "inspectionChoice",
  ]);
  return {
    assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
    title: boundedDisplayLabel(field(record, "title", path), `${path}.title`),
    included: decodeBoolean(field(record, "included", path), `${path}.included`),
    category: decodeNullable(field(record, "category", path), `${path}.category`, decodeIdentifier),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "unavailable",
    ] as const),
    selectedScore: nullableFiniteNumber(
      field(record, "selectedScore", path),
      `${path}.selectedScore`,
    ),
    assignmentScoringState: decodeStringEnum(
      field(record, "assignmentScoringState", path),
      `${path}.assignmentScoringState`,
      ASSIGNMENT_SCORING_STATES,
    ),
    inspectionChoice: decodeAssignmentInspectionChoice(
      field(record, "inspectionChoice", path),
      `${path}.inspectionChoice`,
    ),
  };
}

function decodeGradebookRow(value: unknown, path: string): CalculatedGradebookRow {
  const record = closed(value, path, ["membership", "displayLabel", "outcome", "assignmentCells"]);
  return {
    membership: publicReference(field(record, "membership", path), `${path}.membership`, "M"),
    displayLabel: boundedDisplayLabel(field(record, "displayLabel", path), `${path}.displayLabel`),
    outcome: decodeCourseGradeOutcome(field(record, "outcome", path), `${path}.outcome`),
    assignmentCells: decodeBoundedArray(
      field(record, "assignmentCells", path),
      `${path}.assignmentCells`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      decodeAssignmentCell,
    ),
  };
}

function decodeAssignmentScoringSnapshot(value: unknown, path: string): AssignmentScoringSnapshot {
  const record = closed(value, path, ["assignment", "generation", "assignmentScoringState"]);
  return {
    assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
    generation: positiveSafeInteger(field(record, "generation", path), `${path}.generation`),
    assignmentScoringState: decodeStringEnum(
      field(record, "assignmentScoringState", path),
      `${path}.assignmentScoringState`,
      ASSIGNMENT_SCORING_STATES,
    ),
  };
}

export function decodeCalculatedGradebookResult(
  value: unknown,
  path = "response",
): CalculatedGradebookResult {
  const record = decodeRecord(value, path);
  const resultKind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (resultKind === "reloadRequired") {
    requireOnlyFields(record, path, ["kind", "reason"]);
    return {
      kind: resultKind,
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, RELOAD_REASONS),
    };
  }
  if (resultKind !== "page") {
    throw new DecodeError(`${path}.kind`, "page or reloadRequired");
  }
  requireOnlyFields(record, path, [
    "kind",
    "schemeRevision",
    "rosterRevision",
    "mode",
    "rounding",
    "observationTime",
    "assignmentScoringSnapshots",
    "nextCursor",
    "rows",
  ]);
  const nextCursorValue = optionalField(record, "nextCursor");
  return {
    kind: resultKind,
    schemeRevision: positiveSafeInteger(
      field(record, "schemeRevision", path),
      `${path}.schemeRevision`,
    ),
    rosterRevision: positiveSafeInteger(
      field(record, "rosterRevision", path),
      `${path}.rosterRevision`,
    ),
    mode: decodeStringEnum(field(record, "mode", path), `${path}.mode`, MODES),
    rounding: decodeStringEnum(field(record, "rounding", path), `${path}.rounding`, ROUNDING_RULES),
    observationTime: decodeTimestamp(
      field(record, "observationTime", path),
      `${path}.observationTime`,
    ),
    assignmentScoringSnapshots: decodeBoundedArray(
      field(record, "assignmentScoringSnapshots", path),
      `${path}.assignmentScoringSnapshots`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      decodeAssignmentScoringSnapshot,
    ),
    nextCursor:
      nextCursorValue === undefined ? null : decodeCursor(nextCursorValue, `${path}.nextCursor`),
    rows: decodeBoundedArray(
      field(record, "rows", path),
      `${path}.rows`,
      MAX_CURSOR_PAGE_ITEMS,
      decodeGradebookRow,
    ),
  };
}

function presentationResponseItemReference(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!PRESENTATION_RESPONSE_ITEM_REFERENCE.test(decoded)) {
    throw new DecodeError(path, "a four-character lowercase Presentation Response Item Reference");
  }
  return decoded;
}

function inspectedText(value: unknown, path: string): string {
  return decodeString(value, path);
}

function decodeInspectedResponse(value: unknown, path: string): StudentResponseInspection {
  const record = decodeRecord(value, path);
  const responseKind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (responseKind) {
    case "numeric":
      requireOnlyFields(record, path, ["kind", "value"]);
      return {
        kind: responseKind,
        value: decodeFiniteNumber(field(record, "value", path), `${path}.value`),
      };
    case "multipleChoice":
      requireOnlyFields(record, path, ["kind", "selected"]);
      return {
        kind: responseKind,
        selected: decodeBoundedArray(
          field(record, "selected", path),
          `${path}.selected`,
          MAX_PRESENTED_ITEMS,
          presentationResponseItemReference,
        ),
      };
    case "shortText":
      requireOnlyFields(record, path, ["kind", "text"]);
      return {
        kind: responseKind,
        text: inspectedText(field(record, "text", path), `${path}.text`),
      };
    case "multiBlank":
      requireOnlyFields(record, path, ["kind", "answers"]);
      return {
        kind: responseKind,
        answers: decodeBoundedArray(
          field(record, "answers", path),
          `${path}.answers`,
          MAX_PRESENTED_ITEMS,
          (item, itemPath) => {
            const answer = closed(item, itemPath, ["slot", "text"]);
            return {
              slot: presentationResponseItemReference(
                field(answer, "slot", itemPath),
                `${itemPath}.slot`,
              ),
              text: inspectedText(field(answer, "text", itemPath), `${itemPath}.text`),
            };
          },
        ),
      };
    case "matching":
      requireOnlyFields(record, path, ["kind", "matches"]);
      return {
        kind: responseKind,
        matches: decodeBoundedArray(
          field(record, "matches", path),
          `${path}.matches`,
          MAX_PRESENTED_ITEMS,
          (item, itemPath) => {
            const pair = closed(item, itemPath, ["prompt", "choice"]);
            return {
              prompt: presentationResponseItemReference(
                field(pair, "prompt", itemPath),
                `${itemPath}.prompt`,
              ),
              choice: presentationResponseItemReference(
                field(pair, "choice", itemPath),
                `${itemPath}.choice`,
              ),
            };
          },
        ),
      };
    case "ordering":
      requireOnlyFields(record, path, ["kind", "order"]);
      return {
        kind: responseKind,
        order: decodeBoundedArray(
          field(record, "order", path),
          `${path}.order`,
          MAX_PRESENTED_ITEMS,
          presentationResponseItemReference,
        ),
      };
    case "hotspot":
      requireOnlyFields(record, path, ["kind", "selectedRegions"]);
      return {
        kind: responseKind,
        selectedRegions: decodeBoundedArray(
          field(record, "selectedRegions", path),
          `${path}.selectedRegions`,
          MAX_PRESENTED_ITEMS,
          presentationResponseItemReference,
        ),
      };
    case "imathasQuestionBackend":
      requireOnlyFields(record, path, ["kind", "completion"]);
      return {
        kind: responseKind,
        completion: decodeStringEnum(field(record, "completion", path), `${path}.completion`, [
          "submissionRecorded",
        ] as const),
      };
    default:
      throw new DecodeError(`${path}.kind`, "a known inspected Student response kind");
  }
}

function decodeQuestionAssetRendition(value: unknown, path: string): QuestionAssetRendition {
  const record = closed(value, path, [
    "questionAsset",
    "renditionChecksum",
    "intrinsicWidth",
    "intrinsicHeight",
  ]);
  const dimension = (item: unknown, itemPath: string): number | null =>
    decodeNullable(item, itemPath, (candidate, candidatePath) => {
      const decoded = positiveSafeInteger(candidate, candidatePath);
      if (decoded > 4_294_967_295) throw new DecodeError(candidatePath, "a positive u32");
      return decoded;
    });
  return {
    questionAsset: decodeQuestionAssetReference(
      field(record, "questionAsset", path),
      `${path}.questionAsset`,
    ),
    renditionChecksum: decodeSha256(
      field(record, "renditionChecksum", path),
      `${path}.renditionChecksum`,
    ),
    intrinsicWidth: dimension(field(record, "intrinsicWidth", path), `${path}.intrinsicWidth`),
    intrinsicHeight: dimension(field(record, "intrinsicHeight", path), `${path}.intrinsicHeight`),
  };
}

function decodeEvidence(value: unknown, path: string): InspectedSubmissionEvidence {
  const record = decodeRecord(value, path);
  const evidenceKind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (evidenceKind === "presentationNotApplicable") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind: evidenceKind };
  }
  if (evidenceKind !== "issuedPresentation") {
    throw new DecodeError(`${path}.kind`, "issuedPresentation or presentationNotApplicable");
  }
  requireOnlyFields(record, path, ["kind", "presentation", "issuedPresentationChecksum"]);
  const presentationPath = `${path}.presentation`;
  const presentation = closed(field(record, "presentation", path), presentationPath, [
    "presentation",
    "questionAssetRenditions",
  ]);
  return {
    kind: evidenceKind,
    question: decodeIssuedQuestionPresentation(
      field(presentation, "presentation", presentationPath),
      `${presentationPath}.presentation`,
    ),
    questionAssetRenditions: decodeBoundedArray(
      field(presentation, "questionAssetRenditions", presentationPath),
      `${presentationPath}.questionAssetRenditions`,
      MAX_ASSET_BINDINGS,
      decodeQuestionAssetRendition,
    ),
    issuedPresentationChecksum: decodeSha256(
      field(record, "issuedPresentationChecksum", path),
      `${path}.issuedPresentationChecksum`,
    ),
  };
}

function decodeScoreFeedback(value: unknown, path: string): StudentResponseInspectionFeedback {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["correctness", "pointsEarned", "pointsPossible"]);
  const correctness = optionalField(record, "correctness");
  const pointsEarned = optionalField(record, "pointsEarned");
  const pointsPossible = optionalField(record, "pointsPossible");
  return {
    ...(correctness === undefined
      ? {}
      : { correctness: decodeBoolean(correctness, `${path}.correctness`) }),
    ...(pointsEarned === undefined
      ? {}
      : { pointsEarned: decodeFiniteNumber(pointsEarned, `${path}.pointsEarned`) }),
    ...(pointsPossible === undefined
      ? {}
      : { pointsPossible: decodeFiniteNumber(pointsPossible, `${path}.pointsPossible`) }),
  };
}

function decodeSubmission(value: unknown, path: string): InspectedStudentSubmission {
  const record = closed(value, path, [
    "submittedAt",
    "evidence",
    "scoringGeneration",
    "feedback",
    "response",
    "assignmentScoringState",
  ]);
  return {
    submittedAt: decodeTimestamp(field(record, "submittedAt", path), `${path}.submittedAt`),
    evidence: decodeEvidence(field(record, "evidence", path), `${path}.evidence`),
    scoringGeneration: positiveSafeInteger(
      field(record, "scoringGeneration", path),
      `${path}.scoringGeneration`,
    ),
    feedback: decodeScoreFeedback(field(record, "feedback", path), `${path}.feedback`),
    response: decodeInspectedResponse(field(record, "response", path), `${path}.response`),
    assignmentScoringState: decodeStringEnum(
      field(record, "assignmentScoringState", path),
      `${path}.assignmentScoringState`,
      ASSIGNMENT_SCORING_STATES,
    ),
  };
}

function decodeReturnContext(
  value: unknown,
  path: string,
): InspectedStudentWorkDetail["returnContext"] {
  const record = decodeRecord(value, path);
  const contextKind = decodeString(field(record, "kind", path), `${path}.kind`);
  const focusPath = `${path}.focus`;
  if (contextKind === "gradebook") {
    requireOnlyFields(record, path, ["kind", "course", "membership", "assignment", "focus"]);
    const focus = closed(field(record, "focus", path), focusPath, [
      "kind",
      "membership",
      "assignment",
    ]);
    if (field(focus, "kind", focusPath) !== "gradebookCell") {
      throw new DecodeError(`${focusPath}.kind`, "gradebookCell");
    }
    return {
      kind: contextKind,
      course: publicReference(field(record, "course", path), `${path}.course`, "C"),
      membership: publicReference(field(record, "membership", path), `${path}.membership`, "M"),
      assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
      focus: {
        kind: "gradebookCell",
        membership: publicReference(
          field(focus, "membership", focusPath),
          `${focusPath}.membership`,
          "M",
        ),
        assignment: publicReference(
          field(focus, "assignment", focusPath),
          `${focusPath}.assignment`,
          "A",
        ),
      },
    };
  }
  if (contextKind !== "gradingOperation") {
    throw new DecodeError(`${path}.kind`, "gradebook or gradingOperation");
  }
  requireOnlyFields(record, path, [
    "kind",
    "course",
    "membership",
    "assignment",
    "operation",
    "focus",
  ]);
  const focus = closed(field(record, "focus", path), focusPath, [
    "kind",
    "membership",
    "assignment",
    "operation",
  ]);
  if (field(focus, "kind", focusPath) !== "gradingOperationControl") {
    throw new DecodeError(`${focusPath}.kind`, "gradingOperationControl");
  }
  return {
    kind: contextKind,
    course: publicReference(field(record, "course", path), `${path}.course`, "C"),
    membership: publicReference(field(record, "membership", path), `${path}.membership`, "M"),
    assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
    operation: decodeInstructorGradingOperationReference(
      field(record, "operation", path),
      `${path}.operation`,
    ),
    focus: {
      kind: "gradingOperationControl",
      membership: publicReference(
        field(focus, "membership", focusPath),
        `${focusPath}.membership`,
        "M",
      ),
      assignment: publicReference(
        field(focus, "assignment", focusPath),
        `${focusPath}.assignment`,
        "A",
      ),
      operation: decodeInstructorGradingOperationReference(
        field(focus, "operation", focusPath),
        `${focusPath}.operation`,
      ),
    },
  };
}

export function decodeInspectedStudentWorkDetail(
  value: unknown,
  path = "response",
): InspectedStudentWorkDetail {
  const record = closed(value, path, [
    "course",
    "membership",
    "assignment",
    "assignmentAttempt",
    "studentDisplayLabel",
    "assignmentTitle",
    "submissions",
    "returnContext",
  ]);
  const detail: InspectedStudentWorkDetail = {
    course: publicReference(field(record, "course", path), `${path}.course`, "C"),
    membership: publicReference(field(record, "membership", path), `${path}.membership`, "M"),
    assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
    assignmentAttempt: publicReference(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
      "R",
    ),
    studentDisplayLabel: boundedDisplayLabel(
      field(record, "studentDisplayLabel", path),
      `${path}.studentDisplayLabel`,
    ),
    assignmentTitle: decodeAssignmentTitle(
      field(record, "assignmentTitle", path),
      `${path}.assignmentTitle`,
    ),
    submissions: decodeBoundedArray(
      field(record, "submissions", path),
      `${path}.submissions`,
      MAX_INSPECTED_SUBMISSIONS,
      decodeSubmission,
    ),
    returnContext: decodeReturnContext(
      field(record, "returnContext", path),
      `${path}.returnContext`,
    ),
  };
  const context = detail.returnContext;
  const identityMismatch =
    context.course !== detail.course ||
    context.membership !== detail.membership ||
    context.assignment !== detail.assignment ||
    context.focus.membership !== detail.membership ||
    context.focus.assignment !== detail.assignment;
  const operationMismatch =
    context.kind === "gradingOperation" && context.operation !== context.focus.operation;
  if (identityMismatch || operationMismatch) {
    throw new DecodeError(`${path}.returnContext`, "the inspected Student-work identity");
  }
  return detail;
}
