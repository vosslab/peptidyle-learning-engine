// Strict browser decoder for Instructor Assessment Authored Content Local.

import type { AssessmentReleaseValidation } from "../../../generated/api/AssessmentReleaseValidation";
import type { AssessmentCapabilityViolation, AssessmentEditorDetail } from "../contracts";
import { DecodeError, decodeArray, decodeRecord, decodeStringEnum } from "../decoder";
import {
  decodeInstructorAssessmentAvailabilityView,
  decodeInstructorAssessmentAuthoredContentLocal,
} from "./assessment_teaching_delivery";
import { decodeAssessmentSummary } from "./question_library";
import {
  decodeCapability,
  decodeQuestionTitle,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";

/**
 * Decodes the Assessment editor's deliberately narrow editable Instructor Assessment Authored Content Local.
 * It never carries a Question Source or other server-only policy data.
 */
export function decodeAssessmentEditorDetail(
  value: unknown,
  path = "response",
): Omit<AssessmentEditorDetail, "editNumber"> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "courseInstanceId",
    "title",
    "entries",
    "studentFeedbackReleaseRule",
    "policies",
    "assessmentStatus",
    "assessmentAuthoredContent",
    "assessmentAvailability",
    "assessmentReleaseValidation",
  ]);
  const summary = decodeAssessmentSummary(record, path, false);
  const assessmentAuthoredContent = decodeInstructorAssessmentAuthoredContentLocal(
    field(record, "assessmentAuthoredContent", path),
    `${path}.assessmentAuthoredContent`,
  );
  const assessmentAvailability = decodeInstructorAssessmentAvailabilityView(
    field(record, "assessmentAvailability", path),
    `${path}.assessmentAvailability`,
  );
  const assessmentReleaseValidation = decodeAssessmentReleaseValidation(
    field(record, "assessmentReleaseValidation", path),
    `${path}.assessmentReleaseValidation`,
  );
  const assessmentStatus = decodeStringEnum(
    field(record, "assessmentStatus", path),
    `${path}.assessmentStatus`,
    ["unreleased", "released", "closed", "archived"] as const,
  );
  assertAssessmentAvailabilityMatchesStatus(assessmentStatus, assessmentAvailability, path);
  const decoded = {
    id: summary.id,
    courseInstanceId: summary.courseInstanceId,
    title: summary.title,
    entries: summary.entries,
    studentFeedbackReleaseRule: summary.studentFeedbackReleaseRule,
    policies: summary.policies,
    assessmentStatus,
    assessmentAuthoredContent,
    assessmentAvailability,
    assessmentReleaseValidation,
  } satisfies Omit<AssessmentEditorDetail, "editNumber">;
  return decoded;
}

function assertAssessmentAvailabilityMatchesStatus(
  status: AssessmentEditorDetail["assessmentStatus"],
  assessmentAvailability: AssessmentEditorDetail["assessmentAvailability"],
  path: string,
): void {
  const currentMatchesIntent =
    (status === "unreleased" && assessmentAvailability.state === "unreleased") ||
    (status === "archived" && assessmentAvailability.state === "archived") ||
    (status === "closed" &&
      assessmentAvailability.state === "closed" &&
      assessmentAvailability.closed_at === null) ||
    (status === "released" &&
      (assessmentAvailability.state === "scheduled" ||
        assessmentAvailability.state === "available" ||
        (assessmentAvailability.state === "closed" && assessmentAvailability.closed_at !== null)));
  if (!currentMatchesIntent) {
    throw new DecodeError(
      `${path}.assessmentAvailability`,
      "a server-derived Assessment Availability View consistent with the stable Assessment Status",
    );
  }
}

function decodeAssessmentReleaseValidation(
  value: unknown,
  path: string,
): AssessmentReleaseValidation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blockingIssues"]);
  return {
    blockingIssues: decodeArray(
      field(record, "blockingIssues", path),
      `${path}.blockingIssues`,
      (issue, issuePath) => {
        const issueRecord = decodeRecord(issue, issuePath);
        requireOnlyFields(issueRecord, issuePath, ["kind"]);
        return {
          kind: decodeStringEnum(field(issueRecord, "kind", issuePath), `${issuePath}.kind`, [
            "questionsRequired",
          ] as const),
        };
      },
    ),
  } satisfies AssessmentReleaseValidation;
}

export function decodeAssessmentCapabilityViolations(
  value: unknown,
  path = "response",
): ReadonlyArray<AssessmentCapabilityViolation> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "violations"]);
  if (field(record, "error", path) !== "assessment configuration is not supported") {
    throw new DecodeError(`${path}.error`, "the assessment capability validation failure marker");
  }
  return decodeArray(
    field(record, "violations", path),
    `${path}.violations`,
    (entry, entryPath) => {
      const violation = decodeRecord(entry, entryPath);
      requireOnlyFields(violation, entryPath, ["questionTitle", "questionId", "capability"]);
      const decoded = {
        questionTitle: decodeQuestionTitle(
          field(violation, "questionTitle", entryPath),
          `${entryPath}.questionTitle`,
        ),
        questionId: decodeQuestionId(
          field(violation, "questionId", entryPath),
          `${entryPath}.questionId`,
        ),
        capability: decodeCapability(
          field(violation, "capability", entryPath),
          `${entryPath}.capability`,
        ),
      } satisfies AssessmentCapabilityViolation;
      return decoded;
    },
  );
}
