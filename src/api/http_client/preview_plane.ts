// Strict same-origin browser transport for the read-only WP-INST-T3 preview plane.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseReference } from "../../../generated/api/CourseReference";
import type { DerivedPreviewSubjectRequest } from "../../../generated/api/DerivedPreviewSubjectRequest";
import type { InstructorPreviewSchedulePage } from "../../../generated/api/InstructorPreviewSchedulePage";
import type { PreviewPlaneResponse } from "../../../generated/api/PreviewPlaneResponse";
import type { SyntheticPreviewSubjectRequest } from "../../../generated/api/SyntheticPreviewSubjectRequest";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import type { ApiClient } from "../client";
import type { PoolDrawPreview } from "../contracts";
import {
  decodeDerivedPreviewSubjectRequest,
  decodeInstructorPreviewSchedulePage,
  decodePoolDrawPreview,
  decodePoolDrawPreviewRequest,
  decodePreviewPlaneResponse,
  decodeSyntheticPreviewSubjectRequest,
} from "../decoders";
import { parseAssignmentReference, parseCourseReference } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError, PreviewPlaneConflictError } from "./error";
import { encodedId, requestPath, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

type JsonDecoder<T> = (value: unknown, path?: string) => T;

function strongRevision(value: TeachingOperationRevision, path: string): string {
  if (!/^[1-9][0-9]*$/u.test(value) || BigInt(value) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`${path} needs a canonical positive revision`);
  }
  return `"${value}"`;
}

function previewRoutePath(course: CourseReference, assignment: AssignmentReference): string {
  if (parseCourseReference(course) === null || parseAssignmentReference(assignment) === null) {
    throw new ApiProtocolError("Preview requests require exact C- and A- route references");
  }
  return `/api/courses/${encodedId(course)}/assignments/${encodedId(assignment)}`;
}

function schedulePath(
  course: CourseReference,
  assignment: AssignmentReference,
  cursor: string | undefined,
  pageSize: number | undefined,
): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 100)
  ) {
    throw new ApiProtocolError("Preview schedule page size must be an integer from 1 through 100");
  }
  const query = new URLSearchParams();
  if (cursor !== undefined) {
    if (cursor.length === 0 || cursor.length > 512) {
      throw new ApiProtocolError("Preview schedule cursor must be a bounded opaque value");
    }
    query.set("after", cursor);
  }
  if (pageSize !== undefined) query.set("size", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${previewRoutePath(course, assignment)}/preview-schedule${suffix}`;
}

async function previewJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: JsonDecoder<T>,
  options: {
    readonly method?: "GET" | "POST";
    readonly body?: unknown;
    readonly revision: TeachingOperationRevision;
  },
): Promise<T> {
  const headers: Record<string, string> = {
    accept: "application/json",
    "if-match": strongRevision(options.revision, "Preview request"),
  };
  if (options.body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new PreviewPlaneConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decoder(await boundedResponseJson(response, path), "response");
}

function syntheticBody(
  assignment: AssignmentReference,
  revision: TeachingOperationRevision,
  request: Omit<SyntheticPreviewSubjectRequest, "assignment" | "revision">,
): Omit<SyntheticPreviewSubjectRequest, "assignment" | "revision"> {
  const parsed = decodeSyntheticPreviewSubjectRequest(
    { assignment, revision, ...request },
    "request",
  );
  return {
    selectedMoment: parsed.selectedMoment,
    modifiers: parsed.modifiers,
  };
}

function derivedBody(
  assignment: AssignmentReference,
  revision: TeachingOperationRevision,
  request: Omit<DerivedPreviewSubjectRequest, "assignment" | "revision">,
): Omit<DerivedPreviewSubjectRequest, "assignment" | "revision"> {
  const parsed = decodeDerivedPreviewSubjectRequest(
    { assignment, revision, ...request },
    "request",
  );
  return { selectedMoment: parsed.selectedMoment, membership: parsed.membership };
}

export function createPreviewPlaneClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "listPreviewSchedule"
  | "constructSyntheticPreview"
  | "constructDerivedPreview"
  | "previewPoolDraw"
> {
  return {
    listPreviewSchedule: (
      course,
      assignment,
      revision,
      cursor,
      pageSize,
    ): Promise<InstructorPreviewSchedulePage> =>
      previewJson(
        fetchImplementation,
        basePath,
        schedulePath(course, assignment, cursor, pageSize),
        decodeInstructorPreviewSchedulePage,
        { revision },
      ),
    constructSyntheticPreview: async (
      course,
      assignment,
      revision,
      request,
    ): Promise<PreviewPlaneResponse> => {
      const path = `${previewRoutePath(course, assignment)}/preview-subjects/synthetic`;
      const body = syntheticBody(assignment, revision, request);
      return previewJson(fetchImplementation, basePath, path, decodePreviewPlaneResponse, {
        method: "POST",
        body,
        revision,
      });
    },
    constructDerivedPreview: async (
      course,
      assignment,
      revision,
      request,
    ): Promise<PreviewPlaneResponse> => {
      const path = `${previewRoutePath(course, assignment)}/preview-subjects/derived`;
      const body = derivedBody(assignment, revision, request);
      return previewJson(fetchImplementation, basePath, path, decodePreviewPlaneResponse, {
        method: "POST",
        body,
        revision,
      });
    },
    previewPoolDraw: async (
      course,
      assignment,
      revision,
      groupPosition,
    ): Promise<PoolDrawPreview> => {
      const path = `${previewRoutePath(course, assignment)}/preview-pool-draw`;
      const body = decodePoolDrawPreviewRequest({ groupPosition }, "request");
      return previewJson(fetchImplementation, basePath, path, decodePoolDrawPreview, {
        method: "POST",
        body,
        revision,
      });
    },
  };
}
