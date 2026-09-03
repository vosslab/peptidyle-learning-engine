// Strict same-origin browser transport for read-only Instructor preview operations.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { HypotheticalStudentViewScenarioRequest } from "../../../generated/api/HypotheticalStudentViewScenarioRequest";
import type { InstructorPreviewSchedulePage } from "../../../generated/api/InstructorPreviewSchedulePage";
import type { PreviewPlaneResponse } from "../../../generated/api/PreviewPlaneResponse";
import type { SelectedStudentViewScenarioRequest } from "../../../generated/api/SelectedStudentViewScenarioRequest";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import type { ApiClient } from "../client";
import type { QuestionPoolPreview } from "../contracts";
import {
  decodeHypotheticalStudentViewScenarioRequest,
  decodeInstructorPreviewSchedulePage,
  decodeQuestionPoolPreview,
  decodeQuestionPoolPreviewRequest,
  decodePreviewPlaneResponse,
  decodeSelectedStudentViewScenarioRequest,
} from "../decoders";
import {
  parseAssignmentReference,
  parseCourseInstanceReference,
} from "../../navigation/public_route";
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

function previewRoutePath(
  course: CourseInstanceReference,
  assignment: AssignmentReference,
): string {
  if (
    parseCourseInstanceReference(course) === null ||
    parseAssignmentReference(assignment) === null
  ) {
    throw new ApiProtocolError("Preview requests require exact C- and A- route references");
  }
  return `/api/courses/${encodedId(course)}/assignments/${encodedId(assignment)}`;
}

function schedulePath(
  course: CourseInstanceReference,
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

function hypotheticalStudentViewScenarioBody(
  assignment: AssignmentReference,
  revision: TeachingOperationRevision,
  request: Omit<HypotheticalStudentViewScenarioRequest, "assignment" | "revision">,
): Omit<HypotheticalStudentViewScenarioRequest, "assignment" | "revision"> {
  const parsed = decodeHypotheticalStudentViewScenarioRequest(
    { assignment, revision, ...request },
    "request",
  );
  return {
    selected_moment: parsed.selected_moment,
    modifiers: parsed.modifiers,
  };
}

function selectedStudentViewScenarioBody(
  assignment: AssignmentReference,
  revision: TeachingOperationRevision,
  request: Omit<SelectedStudentViewScenarioRequest, "assignment" | "revision">,
): Omit<SelectedStudentViewScenarioRequest, "assignment" | "revision"> {
  const parsed = decodeSelectedStudentViewScenarioRequest(
    { assignment, revision, ...request },
    "request",
  );
  return {
    selected_moment: parsed.selected_moment,
    selected_student_membership: parsed.selected_student_membership,
  };
}

export function createPreviewPlaneClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "listPreviewSchedule"
  | "constructHypotheticalStudentViewScenario"
  | "constructSelectedStudentViewScenario"
  | "previewQuestionPool"
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
    constructHypotheticalStudentViewScenario: async (
      course,
      assignment,
      revision,
      request,
    ): Promise<PreviewPlaneResponse> => {
      const path = `${previewRoutePath(course, assignment)}/student-view-scenarios/hypothetical`;
      const body = hypotheticalStudentViewScenarioBody(assignment, revision, request);
      return previewJson(fetchImplementation, basePath, path, decodePreviewPlaneResponse, {
        method: "POST",
        body,
        revision,
      });
    },
    constructSelectedStudentViewScenario: async (
      course,
      assignment,
      revision,
      request,
    ): Promise<PreviewPlaneResponse> => {
      const path = `${previewRoutePath(course, assignment)}/student-view-scenarios/selected-student`;
      const body = selectedStudentViewScenarioBody(assignment, revision, request);
      return previewJson(fetchImplementation, basePath, path, decodePreviewPlaneResponse, {
        method: "POST",
        body,
        revision,
      });
    },
    previewQuestionPool: async (
      course,
      assignment,
      revision,
      assignmentEntryId,
    ): Promise<QuestionPoolPreview> => {
      const path = `${previewRoutePath(course, assignment)}/preview-question-pool-selection`;
      const body = decodeQuestionPoolPreviewRequest({ assignmentEntryId }, "request");
      return previewJson(fetchImplementation, basePath, path, decodeQuestionPoolPreview, {
        method: "POST",
        body,
        revision,
      });
    },
  };
}
