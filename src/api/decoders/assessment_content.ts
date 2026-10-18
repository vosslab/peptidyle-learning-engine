// Assessment content requests, entry summaries, and activity policy decoding.
import type { AssessmentEntryAvailability } from "../../../generated/api/AssessmentEntryAvailability";
import type { FixedQuestionAssessmentEntrySummary as FixedQuestionAssessmentEntry } from "../../../generated/api/FixedQuestionAssessmentEntrySummary";
import type { AssessmentEntrySummary } from "../../../generated/api/AssessmentEntrySummary";
import type { AssessmentEntryScoringRule } from "../../../generated/api/AssessmentEntryScoringRule";
import type { QuestionPoolAssessmentEntrySummary as QuestionPoolAssessmentEntry } from "../../../generated/api/QuestionPoolAssessmentEntrySummary";
import type { AssessmentSummary } from "../../../generated/api/AssessmentSummary";
import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import type { AssessmentActivityRules } from "../../../generated/api/AssessmentActivityRules";
import type { AssessmentContentInput, AssessmentEditorEntryInput } from "../contracts";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { MAX_ASSESSMENT_QUESTION_POOL_ITEMS } from "../../../generated/api/MAX_ASSESSMENT_QUESTION_POOL_ITEMS";
import {
  decodeAssessmentId,
  decodeAssessmentTitle,
  decodeBoundedArray,
  decodeCourseInstanceId,
  decodeIdentifier,
  decodeQuestionBackendCapabilities,
  decodeQuestionId,
  decodeQuestionTitle,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeQuestionAttemptLimit, decodeQuestionAttemptTimeLimit } from "./question_model";
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";

function decodeAssessmentActivityRules(
  value: unknown,
  path: string,
  strict = false,
): AssessmentActivityRules {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "partialCreditEnabled",
      "questionVariationRule",
      "assessmentQuestionOrderRule",
    ]);
  }
  const decoded = {
    partialCreditEnabled: decodeBoolean(
      field(record, "partialCreditEnabled", path),
      `${path}.partialCreditEnabled`,
    ),
    questionVariationRule: decodeStringEnum(
      field(record, "questionVariationRule", path),
      `${path}.questionVariationRule`,
      ["reuseVariation", "newVariation"],
    ),
    assessmentQuestionOrderRule: decodeStringEnum(
      field(record, "assessmentQuestionOrderRule", path),
      `${path}.assessmentQuestionOrderRule`,
      ["authoredOrder", "shuffled"],
    ),
  } satisfies AssessmentActivityRules;
  return decoded;
}

function decodeAssessmentPointValue(value: unknown, path: string): AssessmentPointValue {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]{0,9})(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative decimal with at most four places");
  }
  const [whole = "0"] = decoded.split(".", 1);
  if (BigInt(whole) > 1_000_000_000n) {
    throw new DecodeError(path, "an assessment point value in the supported range");
  }
  return decoded;
}

function decodeFixedQuestionAssessmentEntry(
  value: unknown,
  path: string,
): FixedQuestionAssessmentEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "kind",
    "id",
    "questionId",
    "questionTitle",
    "backend",
    "capabilities",
    "pointsPossible",
    "availability",
    "scoringRule",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    questionTitle: decodeQuestionTitle(
      field(record, "questionTitle", path),
      `${path}.questionTitle`,
    ),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "ple",
      "webwork",
      "imathas",
    ]),
    capabilities: decodeQuestionBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    pointsPossible: decodeAssessmentPointValue(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
  };
}

/** Request-only entry shape: the server owns display metadata and all internal identities. */
function decodeAssessmentContentEntry(value: unknown, path: string): AssessmentEditorEntryInput {
  const record = decodeRecord(value, path);
  const entryKind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ] as const);
  if (entryKind === "fixedQuestion") {
    requireOnlyFields(record, path, [
      "kind",
      "questionId",
      "pointsPossible",
      "availability",
      "scoringRule",
      "questionAttemptLimit",
      "questionAttemptTimeLimit",
    ]);
    return {
      kind: "fixedQuestion",
      questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
      pointsPossible: decodeAssessmentPointValue(
        field(record, "pointsPossible", path),
        `${path}.pointsPossible`,
      ),
      availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
        "available",
        "retired",
      ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
      scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
        "normal",
        "fullCredit",
        "extraCredit",
        "excluded",
      ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
      questionAttemptLimit: decodeQuestionAttemptLimit(
        field(record, "questionAttemptLimit", path),
        `${path}.questionAttemptLimit`,
      ),
      questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
        field(record, "questionAttemptTimeLimit", path),
        `${path}.questionAttemptTimeLimit`,
      ),
    };
  }
  requireOnlyFields(record, path, [
    "kind",
    "questionIds",
    "availability",
    "scoringRule",
    "selectionCount",
    "pointsPerItem",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  const questionIds = decodeBoundedArray(
    field(record, "questionIds", path),
    `${path}.questionIds`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    decodeQuestionId,
  );
  if (new Set(questionIds).size !== questionIds.length)
    throw new DecodeError(`${path}.questionIds`, "unique Question IDs");
  const selectionCount = decodePositiveInteger(
    field(record, "selectionCount", path),
    `${path}.selectionCount`,
  );
  if (selectionCount > questionIds.length)
    throw new DecodeError(
      `${path}.selectionCount`,
      "a value no greater than the Question Pool Item count",
    );
  return {
    kind: "questionPool",
    questionIds,
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
    selectionCount,
    pointsPerItem: decodeAssessmentPointValue(
      field(record, "pointsPerItem", path),
      `${path}.pointsPerItem`,
    ),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
  };
}

function decodeAssessmentContentEntries(
  value: unknown,
  path: string,
): ReadonlyArray<AssessmentEditorEntryInput> {
  const entries = decodeBoundedArray(
    value,
    path,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    decodeAssessmentContentEntry,
  );
  const questionPoolItemCount = entries.reduce(
    (total, entry) => total + (entry.kind === "questionPool" ? entry.questionIds.length : 0),
    0,
  );
  if (questionPoolItemCount > MAX_ASSESSMENT_QUESTION_POOL_ITEMS)
    throw new DecodeError(
      path,
      `no more than ${MAX_ASSESSMENT_QUESTION_POOL_ITEMS} Question Pool Item Question IDs`,
    );
  return entries;
}

/** Strict request decoder for the Questions-owned title and ordered content slice. */
export function decodeAssessmentContentInput(
  value: unknown,
  path = "response",
): AssessmentContentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "entries"]);
  return {
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    entries: decodeAssessmentContentEntries(field(record, "entries", path), `${path}.entries`),
  };
}

function decodeQuestionPoolAssessmentEntry(
  value: unknown,
  path: string,
): QuestionPoolAssessmentEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "kind",
    "id",
    "questionPoolId",
    "questionPoolEditNumber",
    "availability",
    "scoringRule",
    "selectionCount",
    "pointsPerItem",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
    selectionCount: decodePositiveInteger(
      field(record, "selectionCount", path),
      `${path}.selectionCount`,
    ),
    pointsPerItem: decodeAssessmentPointValue(
      field(record, "pointsPerItem", path),
      `${path}.pointsPerItem`,
    ),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
  };
}

export function decodeAssessmentEntry(value: unknown, path: string): AssessmentEntrySummary {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ] as const);
  if (kind === "fixedQuestion") {
    return { kind, ...decodeFixedQuestionAssessmentEntry(value, path) };
  }
  return { kind, ...decodeQuestionPoolAssessmentEntry(value, path) };
}

export function decodeAssessmentSummary(
  value: unknown,
  path = "response",
  strict = false,
): AssessmentSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "id",
      "courseInstanceId",
      "title",
      "entries",
      "studentFeedbackReleaseRule",
      "policies",
    ]);
  }
  const decoded = {
    id: decodeAssessmentId(field(record, "id", path), `${path}.id`),
    courseInstanceId: decodeCourseInstanceId(
      field(record, "courseInstanceId", path),
      `${path}.courseInstanceId`,
    ),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    entries: decodeArray(field(record, "entries", path), `${path}.entries`, decodeAssessmentEntry),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    policies: decodeAssessmentActivityRules(
      field(record, "policies", path),
      `${path}.policies`,
      strict,
    ),
  } satisfies AssessmentSummary;
  return decoded;
}
