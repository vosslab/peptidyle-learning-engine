// Strict browser decoder for the Policies workspace's server-owned validation envelope.

import type { AssignmentPoliciesValidationFailure } from "../../../generated/api/AssignmentPoliciesValidationFailure";
import type { AssignmentPoliciesValidationIssue } from "../../../generated/api/AssignmentPoliciesValidationIssue";
import type { AssignmentReleaseIssue } from "../../../generated/api/AssignmentReleaseIssue";
import { DecodeError, decodeRecord, decodeStringEnum } from "../decoder";
import { decodeAssignmentWorkingCopyDefinitionValidationFailure } from "./assignment_teaching_delivery";
import {
  decodeBoundedArray,
  decodeCapability,
  decodeEnvelopeTitle,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_POLICY_VALIDATION_ISSUES = 100;

function decodeAssignmentReleaseIssue(value: unknown, path: string): AssignmentReleaseIssue {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["kind"]);
  return {
    kind: decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
      "questionsRequired",
    ] as const),
  } satisfies AssignmentReleaseIssue;
}

function decodePolicyValidationIssue(
  value: unknown,
  path: string,
): AssignmentPoliciesValidationIssue {
  const record = decodeRecord(value, path);
  const issueKind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "assignmentWorkingCopyDefinition",
    "configuration",
    "capability",
    "assignmentReleaseRequirements",
  ] as const);
  switch (issueKind) {
    case "assignmentWorkingCopyDefinition":
      requireOnlyFields(record, path, ["kind", "correction"]);
      return {
        kind: issueKind,
        correction: decodeAssignmentWorkingCopyDefinitionValidationFailure(
          field(record, "correction", path),
          `${path}.correction`,
        ),
      } satisfies AssignmentPoliciesValidationIssue;
    case "configuration":
      requireOnlyFields(record, path, ["kind", "reason"]);
      return {
        kind: issueKind,
        reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
          "selectedQuestionVariantsWithQuestionPools",
        ] as const),
      } satisfies AssignmentPoliciesValidationIssue;
    case "capability":
      requireOnlyFields(record, path, ["kind", "title", "questionId", "capability"]);
      return {
        kind: issueKind,
        title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
        questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
        capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
      } satisfies AssignmentPoliciesValidationIssue;
    case "assignmentReleaseRequirements": {
      requireOnlyFields(record, path, ["kind", "blockingIssues"]);
      const blockingIssues = decodeBoundedArray(
        field(record, "blockingIssues", path),
        `${path}.blockingIssues`,
        MAX_POLICY_VALIDATION_ISSUES,
        decodeAssignmentReleaseIssue,
      );
      if (blockingIssues.length === 0) {
        throw new DecodeError(`${path}.blockingIssues`, "a nonempty blocking issue list");
      }
      return {
        kind: issueKind,
        blockingIssues,
      } satisfies AssignmentPoliciesValidationIssue;
    }
  }
}

/** Decodes only the bounded, closed 422 envelope owned by Policies save. */
export function decodeAssignmentPoliciesValidationFailure(
  value: unknown,
  path = "response",
): AssignmentPoliciesValidationFailure {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "issues"]);
  const decoded = {
    error: decodeStringEnum(field(record, "error", path), `${path}.error`, [
      "assignmentPoliciesInvalid",
    ] as const),
    issues: decodeBoundedArray(
      field(record, "issues", path),
      `${path}.issues`,
      MAX_POLICY_VALIDATION_ISSUES,
      decodePolicyValidationIssue,
    ),
  } satisfies AssignmentPoliciesValidationFailure;
  if (decoded.issues.length === 0) {
    throw new DecodeError(`${path}.issues`, "a nonempty validation issue list");
  }
  return decoded;
}
