// Strict browser decoder for Instructor Assignment Authored Content Local.

import type { AssignmentReleaseValidation } from "../../../generated/api/AssignmentReleaseValidation";
import type { AssignmentCapabilityViolation, AssignmentEditorDetail } from "../contracts";
import { DecodeError, decodeArray, decodeRecord, decodeStringEnum } from "../decoder";
import {
  decodeInstructorAssignmentAvailabilityView,
  decodeInstructorAssignmentAuthoredContentLocal,
} from "./assignment_teaching_delivery";
import { decodeAssignmentSummary } from "./question_library";
import {
  decodeCapability,
  decodeQuestionTitle,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";

/**
 * Decodes the Assignment editor's deliberately narrow editable Instructor Assignment Authored Content Local.
 * It never carries a Question Source or other server-only policy data.
 */
export function decodeAssignmentEditorDetail(
  value: unknown,
  path = "response",
): Omit<AssignmentEditorDetail, "revision"> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "reference",
    "courseId",
    "title",
    "entries",
    "studentFeedbackReleaseRule",
    "policies",
    "assignmentStatus",
    "assignmentAuthoredContent",
    "assignmentAvailability",
    "assignmentReleaseValidation",
  ]);
  const summary = decodeAssignmentSummary(record, path, false);
  const assignmentAuthoredContent = decodeInstructorAssignmentAuthoredContentLocal(
    field(record, "assignmentAuthoredContent", path),
    `${path}.assignmentAuthoredContent`,
  );
  const assignmentAvailability = decodeInstructorAssignmentAvailabilityView(
    field(record, "assignmentAvailability", path),
    `${path}.assignmentAvailability`,
  );
  const assignmentReleaseValidation = decodeAssignmentReleaseValidation(
    field(record, "assignmentReleaseValidation", path),
    `${path}.assignmentReleaseValidation`,
  );
  const assignmentStatus = decodeStringEnum(
    field(record, "assignmentStatus", path),
    `${path}.assignmentStatus`,
    ["unreleased", "released", "closed", "archived"] as const,
  );
  assertAssignmentAvailabilityMatchesStatus(assignmentStatus, assignmentAvailability, path);
  const decoded = {
    id: summary.id,
    reference: summary.reference,
    courseId: summary.courseId,
    title: summary.title,
    entries: summary.entries,
    studentFeedbackReleaseRule: summary.studentFeedbackReleaseRule,
    policies: summary.policies,
    assignmentStatus,
    assignmentAuthoredContent,
    assignmentAvailability,
    assignmentReleaseValidation,
  } satisfies Omit<AssignmentEditorDetail, "revision">;
  return decoded;
}

function assertAssignmentAvailabilityMatchesStatus(
  status: AssignmentEditorDetail["assignmentStatus"],
  assignmentAvailability: AssignmentEditorDetail["assignmentAvailability"],
  path: string,
): void {
  const currentMatchesIntent =
    (status === "unreleased" && assignmentAvailability.state === "unreleased") ||
    (status === "archived" && assignmentAvailability.state === "archived") ||
    (status === "closed" &&
      assignmentAvailability.state === "closed" &&
      assignmentAvailability.closed_at === null) ||
    (status === "released" &&
      (assignmentAvailability.state === "scheduled" ||
        assignmentAvailability.state === "available" ||
        (assignmentAvailability.state === "closed" && assignmentAvailability.closed_at !== null)));
  if (!currentMatchesIntent) {
    throw new DecodeError(
      `${path}.assignmentAvailability`,
      "a server-derived Assignment Availability View consistent with the stable Assignment Status",
    );
  }
}

function decodeAssignmentReleaseValidation(
  value: unknown,
  path: string,
): AssignmentReleaseValidation {
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
  } satisfies AssignmentReleaseValidation;
}

export function decodeAssignmentCapabilityViolations(
  value: unknown,
  path = "response",
): ReadonlyArray<AssignmentCapabilityViolation> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "violations"]);
  if (field(record, "error", path) !== "assignment configuration is not supported") {
    throw new DecodeError(`${path}.error`, "the assignment capability validation failure marker");
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
      } satisfies AssignmentCapabilityViolation;
      return decoded;
    },
  );
}
