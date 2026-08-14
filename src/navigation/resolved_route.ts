// One strict resolution boundary between visible route references and internal API identities.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { RunId } from "../../generated/api/RunId";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { ApiClient } from "../api/client";
import { parsePublicRouteReference, type PublicRouteReference } from "./public_route";

function publicReference(
  raw: string | undefined,
  prefix: string,
  label: string,
): PublicRouteReference {
  if (raw === undefined || !raw.startsWith(`${prefix}-`)) {
    throw new Error(`${label} route is incomplete`);
  }
  const reference = parsePublicRouteReference(raw);
  if (reference === null) throw new Error(`${label} reference is invalid`);
  return reference;
}

export async function resolveCourseRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<CourseId> {
  const resolved = await client.resolveNavigation(publicReference(raw, "C", "Course"));
  if (resolved.kind !== "course") throw new Error("Course reference resolved to another resource");
  return resolved.courseId;
}

export async function resolveAssignmentRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<{ readonly courseId: CourseId; readonly assignmentId: AssignmentId }> {
  const resolved = await client.resolveNavigation(publicReference(raw, "A", "Assignment"));
  if (resolved.kind !== "assignment") {
    throw new Error("Assignment reference resolved to another resource");
  }
  return resolved;
}

export async function resolveRunRoute(client: ApiClient, raw: string | undefined): Promise<RunId> {
  const resolved = await client.resolveNavigation(publicReference(raw, "R", "Run"));
  if (resolved.kind !== "run") throw new Error("Run reference resolved to another resource");
  return resolved.runId;
}

export async function resolveWorkspaceRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<WorkspaceId> {
  const resolved = await client.resolveNavigation(publicReference(raw, "W", "Workspace"));
  if (resolved.kind !== "workspace") {
    throw new Error("Workspace reference resolved to another resource");
  }
  return resolved.workspaceId;
}
