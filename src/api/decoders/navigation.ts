import type { NavigationResolution } from "../../../generated/api/NavigationResolution";
import { DecodeError, decodeRecord } from "../decoder";
import { decodeIdentifier, field, kind, requireOnlyFields } from "./shared";

/** Strict decoder for the authenticated public-reference lookup boundary. */
export function decodeNavigationResolution(
  value: unknown,
  path = "response",
): NavigationResolution {
  const record = decodeRecord(value, path);
  switch (kind(record, path)) {
    case "course":
      requireOnlyFields(record, path, ["kind", "courseInstanceId"]);
      return {
        kind: "course",
        courseInstanceId: decodeIdentifier(
          field(record, "courseInstanceId", path),
          `${path}.courseInstanceId`,
        ),
      };
    case "assessment":
      requireOnlyFields(record, path, ["kind", "courseInstanceId", "assessmentId"]);
      return {
        kind: "assessment",
        courseInstanceId: decodeIdentifier(
          field(record, "courseInstanceId", path),
          `${path}.courseInstanceId`,
        ),
        assessmentId: decodeIdentifier(field(record, "assessmentId", path), `${path}.assessmentId`),
      };
    case "assessmentAttempt":
      requireOnlyFields(record, path, [
        "kind",
        "courseInstanceId",
        "assessmentId",
        "studentRecordId",
        "assessmentAttemptId",
      ]);
      return {
        kind: "assessmentAttempt",
        courseInstanceId: decodeIdentifier(
          field(record, "courseInstanceId", path),
          `${path}.courseInstanceId`,
        ),
        assessmentId: decodeIdentifier(field(record, "assessmentId", path), `${path}.assessmentId`),
        studentRecordId: decodeIdentifier(
          field(record, "studentRecordId", path),
          `${path}.studentRecordId`,
        ),
        assessmentAttemptId: decodeIdentifier(
          field(record, "assessmentAttemptId", path),
          `${path}.assessmentAttemptId`,
        ),
      };
    case "workspace":
      requireOnlyFields(record, path, ["kind", "workspaceId"]);
      return {
        kind: "workspace",
        workspaceId: decodeIdentifier(field(record, "workspaceId", path), `${path}.workspaceId`),
      };
    default:
      throw new DecodeError(`${path}.kind`, "course, assessment, assessmentAttempt, or workspace");
  }
}
