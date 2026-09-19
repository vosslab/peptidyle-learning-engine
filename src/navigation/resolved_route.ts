// One strict resolution boundary between visible route IDs and internal API identities.

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { ApiClient } from "../api/client";
import {
  parseAssessmentAttemptId,
  parseAssessmentId,
  parseAuthoringWorkspaceId,
} from "./public_route";

function publicIdRoute<Id extends string>(
  raw: string | undefined,
  prefix: string,
  label: string,
  parse: (value: string) => Id | null,
): Id {
  if (raw === undefined || !raw.startsWith(prefix)) {
    throw new Error(`${label} route is incomplete`);
  }
  const id = parse(raw);
  if (id === null) throw new Error(`${label} ID is invalid`);
  return id;
}

function uuidRoute<Id extends string>(
  raw: string | undefined,
  label: string,
  parse: (value: string) => Id | null,
): Id {
  if (raw === undefined) {
    throw new Error(`${label} route is incomplete`);
  }
  const id = parse(raw);
  if (id === null) throw new Error(`${label} ID is invalid`);
  return id;
}

export interface ResolvedAssessmentAttemptIdentity {
  readonly courseInstanceId: CourseInstanceId;
  readonly assessmentId: AssessmentId;
  readonly assessmentAttemptId: AssessmentAttemptId;
}

/** Resolves a public Assessment Attempt ID to the minimum scope identity. */
export async function resolveAssessmentAttemptIdentity(
  client: ApiClient,
  raw: string | undefined,
): Promise<ResolvedAssessmentAttemptIdentity> {
  const resolved = await client.resolveNavigation(
    uuidRoute(raw, "Assessment Attempt", parseAssessmentAttemptId),
  );
  if (resolved.kind !== "assessmentAttempt") {
    throw new Error("Assessment Attempt ID resolved to another resource");
  }
  return Object.freeze({
    courseInstanceId: resolved.courseInstanceId,
    assessmentId: resolved.assessmentId,
    assessmentAttemptId: resolved.assessmentAttemptId,
  });
}

export async function resolveAssessmentRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<{
  readonly courseInstanceId: CourseInstanceId;
  readonly assessmentId: AssessmentId;
}> {
  const resolved = await client.resolveNavigation(
    publicIdRoute(raw, "A", "Assessment", parseAssessmentId),
  );
  if (resolved.kind !== "assessment") {
    throw new Error("Assessment ID resolved to another resource");
  }
  return resolved;
}

export async function resolveAssessmentAttemptRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<AssessmentAttemptId> {
  return (await resolveAssessmentAttemptIdentity(client, raw)).assessmentAttemptId;
}

export async function resolveWorkspaceRoute(
  client: ApiClient,
  raw: string | undefined,
): Promise<WorkspaceId> {
  const resolved = await client.resolveNavigation(
    uuidRoute(raw, "Authoring Workspace", parseAuthoringWorkspaceId),
  );
  if (resolved.kind !== "workspace") {
    throw new Error("Authoring Workspace ID resolved to another resource");
  }
  return resolved.workspaceId;
}
