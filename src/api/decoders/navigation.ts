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
      requireOnlyFields(record, path, ["kind", "courseId"]);
      return {
        kind: "course",
        courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
      };
    case "assignment":
      requireOnlyFields(record, path, ["kind", "courseId", "assignmentId"]);
      return {
        kind: "assignment",
        courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
        assignmentId: decodeIdentifier(field(record, "assignmentId", path), `${path}.assignmentId`),
      };
    case "assignmentAttempt":
      requireOnlyFields(record, path, [
        "kind",
        "courseId",
        "assignmentId",
        "studentRecordId",
        "assignmentAttemptId",
      ]);
      return {
        kind: "assignmentAttempt",
        courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
        assignmentId: decodeIdentifier(field(record, "assignmentId", path), `${path}.assignmentId`),
        studentRecordId: decodeIdentifier(
          field(record, "studentRecordId", path),
          `${path}.studentRecordId`,
        ),
        assignmentAttemptId: decodeIdentifier(
          field(record, "assignmentAttemptId", path),
          `${path}.assignmentAttemptId`,
        ),
      };
    case "workspace":
      requireOnlyFields(record, path, ["kind", "workspaceId"]);
      return {
        kind: "workspace",
        workspaceId: decodeIdentifier(field(record, "workspaceId", path), `${path}.workspaceId`),
      };
    default:
      throw new DecodeError(`${path}.kind`, "course, assignment, assignmentAttempt, or workspace");
  }
}
