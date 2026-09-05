// One strict resolution boundary between visible route references and internal API identities.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { ApiClient } from "../api/client";
import {
  parseAssignmentAttemptReference,
  parseAssignmentReference,
  parseAuthoringWorkspaceReference,
  parseCourseInstanceReference,
} from "./public_route";

function publicReference<Reference extends string>(
  raw: string | undefined,
  prefix: string,
  label: string,
  parse: (value: string) => Reference | null,
): Reference {
  if (raw === undefined || !raw.startsWith(`${prefix}-`)) {
    throw new Error(`${label} route is incomplete`);
  }
  const reference = parse(raw);
  if (reference === null) throw new Error(`${label} reference is invalid`);
  return reference;
}

export interface ResolvedCourseIdentity {
  readonly courseId: CourseId;
}

export interface ResolvedAssignmentAttemptIdentity {
  readonly courseId: CourseId;
  readonly assignmentId: AssignmentId;
  readonly assignmentAttemptId: AssignmentAttemptId;
}

/** Resolves a public Course Instance reference to the minimum scope identity. */
export async function resolveCourseIdentity(
  client: ApiClient,
  raw: string | undefined,
): Promise<ResolvedCourseIdentity> {
  const resolved = await client.resolveNavigation(
    publicReference(raw, "C", "Course", parseCourseInstanceReference),
  );
  if (resolved.kind !== "course")
    throw new Error("Course Instance reference resolved to another resource");
  return Object.freeze({ courseId: resolved.courseId });
}

/** Resolves a public Assignment Attempt reference to the minimum scope identity. */
export async function resolveAssignmentAttemptIdentity(
  client: ApiClient,
  raw: string | undefined,
): Promise<ResolvedAssignmentAttemptIdentity> {
  const resolved = await client.resolveNavigation(
    publicReference(raw, "R", "Assignment Attempt", parseAssignmentAttemptReference),
  );
  if (resolved.kind !== "assignmentAttempt") {
    throw new Error("Assignment Attempt reference resolved to another resource");
  }
  return Object.freeze({
    courseId: resolved.courseId,
    assignmentId: resolved.assignmentId,
    assignmentAttemptId: resolved.assignmentAttemptId,
  });
}

export async function resolveCourseRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<CourseId> {
  return (await resolveCourseIdentity(client, raw)).courseId;
}

export async function resolveAssignmentRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<{ readonly courseId: CourseId; readonly assignmentId: AssignmentId }> {
  const resolved = await client.resolveNavigation(
    publicReference(raw, "A", "Assignment", parseAssignmentReference),
  );
  if (resolved.kind !== "assignment") {
    throw new Error("Assignment reference resolved to another resource");
  }
  return resolved;
}

export async function resolveAssignmentAttemptRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<AssignmentAttemptId> {
  return (await resolveAssignmentAttemptIdentity(client, raw)).assignmentAttemptId;
}

export async function resolveWorkspaceRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<WorkspaceId> {
  const resolved = await client.resolveNavigation(
    publicReference(raw, "W", "Workspace", parseAuthoringWorkspaceReference),
  );
  if (resolved.kind !== "workspace") {
    throw new Error("Workspace reference resolved to another resource");
  }
  return resolved.workspaceId;
}
