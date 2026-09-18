// One strict resolution boundary between visible route references and internal API identities.

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { ApiClient } from "../api/client";
import {
  parseAssessmentAttemptReference,
  parseAssessmentId,
  parseAuthoringWorkspaceReference,
} from "./public_route";

function publicReference<Reference extends string>(
  raw: string | undefined,
  prefix: string,
  label: string,
  parse: (value: string) => Reference | null,
): Reference {
  if (raw === undefined || !raw.startsWith(prefix)) {
    throw new Error(`${label} route is incomplete`);
  }
  const reference = parse(raw);
  if (reference === null) throw new Error(`${label} reference is invalid`);
  return reference;
}

function uuidRoute<Reference extends string>(
  raw: string | undefined,
  label: string,
  parse: (value: string) => Reference | null,
): Reference {
  if (raw === undefined) {
    throw new Error(`${label} route is incomplete`);
  }
  const reference = parse(raw);
  if (reference === null) throw new Error(`${label} reference is invalid`);
  return reference;
}

export interface ResolvedAssessmentAttemptIdentity {
  readonly courseInstanceId: CourseInstanceId;
  readonly assessmentId: AssessmentId;
  readonly assessmentAttemptId: AssessmentAttemptId;
}

/** Resolves a public Assessment Attempt reference to the minimum scope identity. */
export async function resolveAssessmentAttemptIdentity(
  client: ApiClient,
  raw: string | undefined,
): Promise<ResolvedAssessmentAttemptIdentity> {
  const resolved = await client.resolveNavigation(
    uuidRoute(raw, "Assessment Attempt", parseAssessmentAttemptReference),
  );
  if (resolved.kind !== "assessmentAttempt") {
    throw new Error("Assessment Attempt reference resolved to another resource");
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
    publicReference(raw, "A", "Assessment", parseAssessmentId),
  );
  if (resolved.kind !== "assessment") {
    throw new Error("Assessment reference resolved to another resource");
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
    publicReference(raw, "W", "Workspace", parseAuthoringWorkspaceReference),
  );
  if (resolved.kind !== "workspace") {
    throw new Error("Workspace reference resolved to another resource");
  }
  return resolved.workspaceId;
}
