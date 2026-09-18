// Closed, answer-free Course-wide Blueprint update discovery.

import { ASSESSMENT_TYPE_VALUES } from "../../../generated/api/AssessmentType";
import type {
  CourseAssessmentBlueprintUpdateSummary,
  CourseBlueprintUpdateReview,
} from "../assessment_release";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import { blueprintCourseReference, blueprintRevision } from "./assessment_release";
import {
  decodeAssessmentId,
  decodeAssessmentTitle,
  field,
  requireOnlyFields,
} from "./shared";

function assessmentSummary(value: unknown, path: string): CourseAssessmentBlueprintUpdateSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentReference",
    "title",
    "assessmentType",
    "matchesSource",
    "cannotApplyReason",
  ]);
  const rawReason = field(record, "cannotApplyReason", path);
  const cannotApplyReason =
    rawReason === null
      ? null
      : decodeStringEnum(rawReason, `${path}.cannotApplyReason`, [
          "retainedSourceMissing",
          "assessmentTypeMismatch",
        ]);
  const matchesSource = decodeBoolean(
    field(record, "matchesSource", path),
    `${path}.matchesSource`,
  );
  if (matchesSource && cannotApplyReason !== null) {
    throw new DecodeError(path, "an applicable matching Blueprint Assessment");
  }
  return {
    assessmentReference: decodeAssessmentId(
      field(record, "assessmentReference", path),
      `${path}.assessmentReference`,
    ),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    assessmentType: decodeStringEnum(
      field(record, "assessmentType", path),
      `${path}.assessmentType`,
      ASSESSMENT_TYPE_VALUES,
    ),
    matchesSource,
    cannotApplyReason,
  };
}

/** ASVS 1.5.2, 2.2.1, 15.3.1: only the complete, consistent public summary is accepted. */
export function decodeCourseBlueprintUpdateReview(
  value: unknown,
  path = "response",
): CourseBlueprintUpdateReview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "blueprintReference",
    "adoptedRevision",
    "sourceRevision",
    "assessments",
  ]);
  const adoptedRevision = blueprintRevision(
    field(record, "adoptedRevision", path),
    `${path}.adoptedRevision`,
  );
  const sourceRevision = blueprintRevision(
    field(record, "sourceRevision", path),
    `${path}.sourceRevision`,
  );
  const assessments = decodeArray(
    field(record, "assessments", path),
    `${path}.assessments`,
    assessmentSummary,
  );
  if (
    BigInt(sourceRevision) < BigInt(adoptedRevision) ||
    new Set(assessments.map((row) => row.assessmentReference)).size !== assessments.length
  ) {
    throw new DecodeError(path, "a current Revision and unique adopted Assessment correspondences");
  }
  return {
    blueprintReference: blueprintCourseReference(
      field(record, "blueprintReference", path),
      `${path}.blueprintReference`,
    ),
    adoptedRevision,
    sourceRevision,
    assessments,
  };
}
