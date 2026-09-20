// Strict browser decoding for retained Assessment Blueprint update review.

import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import type {
  AssessmentBlueprintUpdateContent,
  AssessmentBlueprintUpdateEntry,
  AssessmentBlueprintUpdateReview,
  ApplyAssessmentBlueprintUpdateInput,
} from "../assessment_release";
import { DecodeError, decodePositiveInteger, decodeRecord, decodeStringEnum } from "../decoder";
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
import {
  assessmentType,
  decodeAssessmentActivityRules,
  decodeAssessmentInstructions,
  decodeLiveAssessmentWorkspace,
  editNumber,
  lateWorkRule,
  optionalPositiveInteger,
  pointValue,
  poolSelectionRule,
} from "./assessment_release";
import { blueprintRevisionTuple } from "./blueprint_course";
import { decodeQuestionAttemptLimit, decodeQuestionAttemptTimeLimit } from "./question_model";
import {
  decodeAssessmentTitle,
  decodeBoundedArray,
  decodeQuestionId,
  decodeQuestionRevisionTuple,
  field,
  requireOnlyFields,
} from "./shared";

export function decodeApplyAssessmentBlueprintUpdateInput(
  value: unknown,
  path = "input",
): ApplyAssessmentBlueprintUpdateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "expectedSourceBlueprintRevisionTuple",
    "expectedAssessmentEditNumber",
  ]);
  return {
    expectedSourceBlueprintRevisionTuple: blueprintRevisionTuple(
      field(record, "expectedSourceBlueprintRevisionTuple", path),
      `${path}.expectedSourceBlueprintRevisionTuple`,
    ),
    expectedAssessmentEditNumber: editNumber(
      field(record, "expectedAssessmentEditNumber", path),
      `${path}.expectedAssessmentEditNumber`,
    ),
  };
}

function blueprintUpdateEntry(value: unknown, path: string): AssessmentBlueprintUpdateEntry {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ]);
  const sharedFields = ["kind", "scoringRule", "questionAttemptLimit", "questionAttemptTimeLimit"];
  requireOnlyFields(
    record,
    path,
    kind === "fixedQuestion"
      ? [...sharedFields, "questionRevisionTuple", "pointsPossible"]
      : [
          ...sharedFields,
          "questionPoolId",
          "questionPoolEditNumber",
          "selectionCount",
          "pointsPerItem",
          "selectionRule",
        ],
  );
  const settings = {
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ]),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
      true,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
      true,
    ),
  };
  if (kind === "fixedQuestion")
    return {
      kind,
      ...settings,
      questionRevisionTuple: decodeQuestionRevisionTuple(
        field(record, "questionRevisionTuple", path),
        `${path}.questionRevisionTuple`,
        true,
      ),
      pointsPossible: pointValue(field(record, "pointsPossible", path), `${path}.pointsPossible`),
    };
  const selectionCount = decodePositiveInteger(
    field(record, "selectionCount", path),
    `${path}.selectionCount`,
  );
  if (selectionCount > 4_294_967_295)
    throw new DecodeError(`${path}.selectionCount`, "a positive u32");
  return {
    kind,
    ...settings,
    selectionCount,
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
    pointsPerItem: pointValue(field(record, "pointsPerItem", path), `${path}.pointsPerItem`),
    selectionRule: poolSelectionRule(field(record, "selectionRule", path), `${path}.selectionRule`),
  };
}

function blueprintUpdateContent(value: unknown, path: string): AssessmentBlueprintUpdateContent {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentType",
    "title",
    "instructions",
    "defaults",
    "entries",
  ]);
  const defaultsPath = `${path}.defaults`;
  const defaults = decodeRecord(field(record, "defaults", path), defaultsPath);
  requireOnlyFields(defaults, defaultsPath, [
    "assessment_attempt_time_limit_seconds",
    "assessment_attempt_limit",
    "late_work_rule",
    "activity_rules",
    "student_feedback_release_rule",
  ]);
  return {
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    entries: decodeBoundedArray(
      field(record, "entries", path),
      `${path}.entries`,
      MAX_ASSESSMENT_ORDERED_ENTRIES,
      blueprintUpdateEntry,
    ),
    defaults: {
      assessment_attempt_time_limit_seconds: optionalPositiveInteger(
        field(defaults, "assessment_attempt_time_limit_seconds", defaultsPath),
        `${defaultsPath}.assessment_attempt_time_limit_seconds`,
      ),
      assessment_attempt_limit: optionalPositiveInteger(
        field(defaults, "assessment_attempt_limit", defaultsPath),
        `${defaultsPath}.assessment_attempt_limit`,
      ),
      late_work_rule: lateWorkRule(
        field(defaults, "late_work_rule", defaultsPath),
        `${defaultsPath}.late_work_rule`,
      ),
      activity_rules: decodeAssessmentActivityRules(
        field(defaults, "activity_rules", defaultsPath),
        `${defaultsPath}.activity_rules`,
      ),
      student_feedback_release_rule: decodeStudentFeedbackReleaseRule(
        field(defaults, "student_feedback_release_rule", defaultsPath),
        `${defaultsPath}.student_feedback_release_rule`,
      ),
    },
  };
}

/** ASVS 1.5.2, 2.2.1: accept only the closed, internally consistent review. */
export function decodeAssessmentBlueprintUpdateReview(
  value: unknown,
  path = "response",
): AssessmentBlueprintUpdateReview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment",
    "sourceBlueprintRevisionTuple",
    "proposed",
    "cannotApplyReason",
  ]);
  const rawProposed = field(record, "proposed", path);
  const rawReason = field(record, "cannotApplyReason", path);
  const assessment = decodeLiveAssessmentWorkspace(
    field(record, "assessment", path),
    `${path}.assessment`,
  );
  const proposed =
    rawProposed === null ? null : blueprintUpdateContent(rawProposed, `${path}.proposed`);
  const cannotApplyReason =
    rawReason === null
      ? null
      : decodeStringEnum(rawReason, `${path}.cannotApplyReason`, [
          "retainedSourceMissing",
          "assessmentTypeMismatch",
        ]);
  if (
    (proposed === null) !== (cannotApplyReason === "retainedSourceMissing") ||
    (proposed !== null &&
      (proposed.assessmentType !== assessment.assessmentType) !==
        (cannotApplyReason === "assessmentTypeMismatch")) ||
    assessment.origin.kind !== "adopted"
  ) {
    throw new DecodeError(path, "a consistent retained Assessment Blueprint update review");
  }
  return {
    assessment,
    proposed,
    cannotApplyReason,
    sourceBlueprintRevisionTuple: blueprintRevisionTuple(
      field(record, "sourceBlueprintRevisionTuple", path),
      `${path}.sourceBlueprintRevisionTuple`,
    ),
  };
}
