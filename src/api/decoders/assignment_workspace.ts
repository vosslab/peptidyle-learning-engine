// Strict browser decoder for the Instructor assignment workspace projection.

import type { AssignmentContentIssuedWorkConflict } from "../../../generated/api/AssignmentContentIssuedWorkConflict";
import type { AssignmentPublicationReadiness } from "../../../generated/api/AssignmentPublicationReadiness";
import type { AssignmentCapabilityViolation, AssignmentEditorDetail } from "../contracts";
import { DecodeError, decodeArray, decodeRecord, decodeStringEnum } from "../decoder";
import {
  decodeInstructorAssignmentCurrentState,
  decodeInstructorAssignmentTeachingSettingsLocal,
} from "./assignment_teaching_delivery";
import { decodeAssignmentSummary } from "./question_library";
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
    "disclosurePolicy",
    "policies",
    "teachingSettings",
    "currentState",
    "publicationReadiness",
  ]);
  const summary = decodeAssignmentSummary(record, path, false);
  const teachingSettings = decodeInstructorAssignmentTeachingSettingsLocal(
    field(record, "teachingSettings", path),
    `${path}.teachingSettings`,
  );
  const currentState = decodeInstructorAssignmentCurrentState(
    field(record, "currentState", path),
    `${path}.currentState`,
  );
  const publicationReadiness = decodeAssignmentPublicationReadiness(
    field(record, "publicationReadiness", path),
    `${path}.publicationReadiness`,
  );
  assertCurrentStateMatchesLifecycle(teachingSettings.lifecycle, currentState, path);
  const decoded = {
    id: summary.id,
    reference: summary.reference,
    courseId: summary.courseId,
    title: summary.title,
    entries: summary.entries,
    disclosurePolicy: summary.disclosurePolicy,
    policies: summary.policies,
    teachingSettings,
    currentState,
    publicationReadiness,
  } satisfies Omit<AssignmentEditorDetail, "revision">;
  return decoded;
}

/** Decodes the exact 409 body that says issued Student work blocks content save. */
export function decodeAssignmentContentIssuedWorkConflict(
  value: unknown,
  path = "response",
): AssignmentContentIssuedWorkConflict {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["kind"]);
  const decoded = {
    kind: decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
      "issuedStudentWork",
    ] as const),
  } satisfies AssignmentContentIssuedWorkConflict;
  return decoded;
}

function assertCurrentStateMatchesLifecycle(
  lifecycle: AssignmentEditorDetail["teachingSettings"]["lifecycle"],
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

function decodeAssignmentPublicationReadiness(
  value: unknown,
  path: string,
): AssignmentPublicationReadiness {
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
  } satisfies AssignmentPublicationReadiness;
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
