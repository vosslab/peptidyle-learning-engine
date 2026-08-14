import type { NavigationResolution } from "../contracts";
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
    case "run":
      requireOnlyFields(record, path, ["kind", "runId"]);
      return {
        kind: "run",
        runId: decodeIdentifier(field(record, "runId", path), `${path}.runId`),
      };
    case "workspace":
      requireOnlyFields(record, path, ["kind", "workspaceId"]);
      return {
        kind: "workspace",
        workspaceId: decodeIdentifier(field(record, "workspaceId", path), `${path}.workspaceId`),
      };
    default:
      throw new DecodeError(`${path}.kind`, "course, assignment, run, or workspace");
  }
}
