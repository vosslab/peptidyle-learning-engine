// Strict browser decoder for the Instructor assignment workspace projection.

import type { SuccessorAssignmentRevisionRequired } from "../../../generated/api/SuccessorAssignmentRevisionRequired";
import type { AssignmentReleaseValidation } from "../../../generated/api/AssignmentReleaseValidation";
import type { AssignmentCapabilityViolation, AssignmentEditorDetail } from "../contracts";
import { DecodeError, decodeArray, decodeRecord, decodeString, decodeStringEnum } from "../decoder";
import {
  decodeInstructorAssignmentAvailabilityView,
  decodeInstructorAssignmentAuthoredContentLocal,
} from "./assignment_teaching_delivery";
import { decodeAssignmentReference, decodeAssignmentSummary } from "./question_library";
import {
  decodeCapability,
  decodeEnvelopeTitle,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";

/**
 * Decodes the Assignment editor's deliberately narrow editable Assignment projection.
 * It never carries question source material or other server-only policy.
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

/** Decodes the exact 409 body requiring a successor Assignment. */
export function decodeSuccessorAssignmentRevisionRequired(
  value: unknown,
  path = "response",
): SuccessorAssignmentRevisionRequired {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["baseRevision"]);
  const baseRevisionPath = `${path}.baseRevision`;
  const baseRevision = decodeRecord(field(record, "baseRevision", path), baseRevisionPath);
  requireOnlyFields(baseRevision, baseRevisionPath, ["assignment", "revision_number"]);
  const revisionNumber = decodeString(
    field(baseRevision, "revision_number", baseRevisionPath),
    `${baseRevisionPath}.revision_number`,
  );
  if (!/^[1-9][0-9]*$/u.test(revisionNumber))
    throw new DecodeError(`${baseRevisionPath}.revision_number`, "a positive revision number");
  const decoded = {
    baseRevision: {
      assignment: decodeAssignmentReference(
        field(baseRevision, "assignment", baseRevisionPath),
        `${baseRevisionPath}.assignment`,
      ),
      revision_number: revisionNumber,
    },
  } satisfies SuccessorAssignmentRevisionRequired;
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
      assignmentAvailability.closedAt === null) ||
    (status === "released" &&
      (assignmentAvailability.state === "scheduled" ||
        assignmentAvailability.state === "available" ||
        (assignmentAvailability.state === "closed" && assignmentAvailability.closedAt !== null)));
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
      requireOnlyFields(violation, entryPath, ["title", "questionId", "capability"]);
      const decoded = {
        title: decodeEnvelopeTitle(field(violation, "title", entryPath), `${entryPath}.title`),
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
