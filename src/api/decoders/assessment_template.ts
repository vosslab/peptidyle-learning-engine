// Strict browser decoding for private Instructor-owned Assessment Templates.

import type { AssessmentTemplate } from "../../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateEditNumber } from "../../../generated/api/AssessmentTemplateEditNumber";
import type { AssessmentTemplateId } from "../../../generated/api/AssessmentTemplateId";
import type { AssessmentTemplateName } from "../../../generated/api/AssessmentTemplateName";
import type { AssessmentTemplateSettings } from "../../../generated/api/AssessmentTemplateSettings";
import { ASSESSMENT_TYPE_VALUES, type AssessmentType } from "../../../generated/api/AssessmentType";
import { MAX_ASSESSMENT_ATTEMPT_LIMIT } from "../../../generated/api/MAX_ASSESSMENT_ATTEMPT_LIMIT";
import { MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS } from "../../../generated/api/MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS";
import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type {
  CreateAssessmentTemplateInput,
  SaveAssessmentTemplateInput,
} from "../assessment_template";
import {
  DecodeError,
  decodeArray,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
import { decodeAssessmentActivityRules, decodeAssessmentInstructions } from "./assessment_release";
import { field, requireOnlyFields } from "./shared";

const EDGE_UNICODE_WHITE_SPACE = /^\p{White_Space}|\p{White_Space}$/u;
const MAX_ASSESSMENT_TEMPLATE_NAME_UNICODE_SCALARS = 200;

function assessmentTemplateName(value: unknown, path: string): AssessmentTemplateName {
  const decoded = decodeString(value, path);
  if (decoded.length === 0 || /^\p{White_Space}+$/u.test(decoded)) {
    throw new DecodeError(path, "an Assessment Template name containing non-whitespace text");
  }
  if (EDGE_UNICODE_WHITE_SPACE.test(decoded)) {
    throw new DecodeError(path, "an Assessment Template name without surrounding whitespace");
  }
  if (Array.from(decoded).length > MAX_ASSESSMENT_TEMPLATE_NAME_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `an Assessment Template name no longer than ${MAX_ASSESSMENT_TEMPLATE_NAME_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return decoded;
}

function assessmentTemplateId(value: unknown, path: string): AssessmentTemplateId {
  return decodeUuid(value, path);
}

function assessmentTemplateEditNumber(value: unknown, path: string): AssessmentTemplateEditNumber {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Assessment Template Edit Number");
  }
  return decoded;
}

function assessmentType(value: unknown, path: string): AssessmentType {
  return decodeStringEnum(value, path, ASSESSMENT_TYPE_VALUES);
}

function lateWorkRule(value: unknown, path: string): LateWorkRule {
  return decodeStringEnum(value, path, ["accept", "mark_late", "reject"]);
}

function optionalBoundedPositiveInteger(
  value: unknown,
  path: string,
  maximum: number,
): number | null {
  if (value === null) return null;
  const decoded = decodePositiveInteger(value, path);
  if (decoded > maximum) {
    throw new DecodeError(path, `a positive integer no greater than ${maximum}`);
  }
  return decoded;
}

function assessmentTemplateSettings(value: unknown, path: string): AssessmentTemplateSettings {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "instructions",
    "assessmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "lateWorkRule",
    "activityRules",
    "studentFeedbackReleaseRule",
  ]);
  return {
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    assessmentAttemptTimeLimitSeconds: optionalBoundedPositiveInteger(
      field(record, "assessmentAttemptTimeLimitSeconds", path),
      `${path}.assessmentAttemptTimeLimitSeconds`,
      MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    ),
    attemptLimit: optionalBoundedPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
      MAX_ASSESSMENT_ATTEMPT_LIMIT,
    ),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    activityRules: decodeAssessmentActivityRules(
      field(record, "activityRules", path),
      `${path}.activityRules`,
    ),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
  };
}

/** Decodes one full private aggregate and refuses every server-added field. */
export function decodeAssessmentTemplate(value: unknown, path = "response"): AssessmentTemplate {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "name",
    "assessmentType",
    "settings",
    "assessmentTemplateEditNumber",
  ]);
  return {
    id: assessmentTemplateId(field(record, "id", path), `${path}.id`),
    name: assessmentTemplateName(field(record, "name", path), `${path}.name`),
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    settings: assessmentTemplateSettings(field(record, "settings", path), `${path}.settings`),
    assessmentTemplateEditNumber: assessmentTemplateEditNumber(
      field(record, "assessmentTemplateEditNumber", path),
      `${path}.assessmentTemplateEditNumber`,
    ),
  };
}

/** Decodes the exact owner-scoped collection response. */
export function decodeAssessmentTemplateList(
  value: unknown,
  path = "response",
): ReadonlyArray<AssessmentTemplate> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items"]);
  return decodeArray(field(record, "items", path), `${path}.items`, decodeAssessmentTemplate);
}

/** Validates the closed create request before it crosses the browser boundary. */
export function decodeCreateAssessmentTemplateInput(
  value: unknown,
  path = "request",
): CreateAssessmentTemplateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["name", "assessmentType"]);
  return {
    name: assessmentTemplateName(field(record, "name", path), `${path}.name`),
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
  };
}

/** Validates the closed full replacement request before it crosses the browser boundary. */
export function decodeSaveAssessmentTemplateInput(
  value: unknown,
  path = "request",
): SaveAssessmentTemplateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["name", "assessmentType", "settings"]);
  return {
    name: assessmentTemplateName(field(record, "name", path), `${path}.name`),
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    settings: assessmentTemplateSettings(field(record, "settings", path), `${path}.settings`),
  };
}
