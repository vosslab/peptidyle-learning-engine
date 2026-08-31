// Strict browser decoder for the Instructor assignment workspace projection.

import type { SuccessorAssignmentRevisionRequired } from "../../../generated/api/SuccessorAssignmentRevisionRequired";
import type { DraftAssignmentRevisionPublicationReadiness } from "../../../generated/api/DraftAssignmentRevisionPublicationReadiness";
import type { AssignmentCapabilityViolation, AssignmentEditorDetail } from "../contracts";
import { DecodeError, decodeArray, decodeRecord, decodeString, decodeStringEnum } from "../decoder";
import {
  decodeInstructorAssignmentCurrentState,
  decodeInstructorAssignmentRevisionDefinitionLocal,
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
 * Decodes the assignment editor's deliberately narrow, revisioned projection.
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
    "assignmentRevisionDefinition",
    "currentState",
    "draftRevisionPublicationReadiness",
  ]);
  const summary = decodeAssignmentSummary(record, path, false);
  const assignmentRevisionDefinition = decodeInstructorAssignmentRevisionDefinitionLocal(
    field(record, "assignmentRevisionDefinition", path),
    `${path}.assignmentRevisionDefinition`,
  );
  const currentState = decodeInstructorAssignmentCurrentState(
    field(record, "currentState", path),
    `${path}.currentState`,
  );
  const draftRevisionPublicationReadiness = decodeDraftAssignmentRevisionPublicationReadiness(
    field(record, "draftRevisionPublicationReadiness", path),
    `${path}.draftRevisionPublicationReadiness`,
  );
  assertCurrentStateMatchesLifecycle(assignmentRevisionDefinition.lifecycle, currentState, path);
  const decoded = {
    id: summary.id,
    reference: summary.reference,
    courseId: summary.courseId,
    title: summary.title,
    entries: summary.entries,
    studentFeedbackReleaseRule: summary.studentFeedbackReleaseRule,
    policies: summary.policies,
    assignmentRevisionDefinition,
    currentState,
    draftRevisionPublicationReadiness,
  } satisfies Omit<AssignmentEditorDetail, "revision">;
  return decoded;
}

/** Decodes the exact 409 body requiring a successor Draft Assignment Revision. */
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

function assertCurrentStateMatchesLifecycle(
  lifecycle: AssignmentEditorDetail["assignmentRevisionDefinition"]["lifecycle"],
  currentState: AssignmentEditorDetail["currentState"],
  path: string,
): void {
  const currentMatchesIntent =
    (lifecycle === "draft" && currentState.state === "draft") ||
    (lifecycle === "archived" && currentState.state === "archived") ||
    (lifecycle === "closed" && currentState.state === "closed" && currentState.closedAt === null) ||
    (lifecycle === "published" &&
      (currentState.state === "scheduled" ||
        currentState.state === "open" ||
        (currentState.state === "closed" && currentState.closedAt !== null)));
  if (!currentMatchesIntent) {
    throw new DecodeError(
      `${path}.currentState`,
      "a server-derived state consistent with the stored lifecycle intent",
    );
  }
}

function decodeDraftAssignmentRevisionPublicationReadiness(
  value: unknown,
  path: string,
): DraftAssignmentRevisionPublicationReadiness {
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
  } satisfies DraftAssignmentRevisionPublicationReadiness;
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
