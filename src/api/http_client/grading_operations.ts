// Strict same-origin transport for Instructor automated-grading recovery operations.

import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { CourseId } from "../../../generated/api/CourseId";
import type { ApiClient } from "../client";
import {
  decodeGradingOperationActionReceipt,
  decodeInstructorGradingOperationReference,
  decodeGradingOperationStrongEtag,
  decodeInstructorGradingOperationsPage,
  type GradingOperationActionReceipt,
  type GradingOperationFocus,
  type GradingOperationStrongEtag,
  type InstructorGradingOperationsPage,
} from "../decoders/grading_operations";
import { decodeCursor, decodeIdentifier } from "../decoders/shared";
import { decodeStringEnum } from "../decoder";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;

function courseAssignmentPath(courseId: CourseId, assignmentId: AssignmentId): string {
  const course = decodeIdentifier(courseId, "course");
  const assignment = decodeIdentifier(assignmentId, "assignment");
  return `/api/courses/${encodedId(course)}/assignments/${encodedId(assignment)}`;
}

function listPath(
  courseId: CourseId,
  assignmentId: AssignmentId,
  focus: GradingOperationFocus,
  cursor: string | undefined,
  pageSize: number | undefined,
): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE)
  ) {
    throw new ApiProtocolError(
      "grading operations page size must be an integer from 1 through 100",
    );
  }
  const checkedFocus = decodeStringEnum(focus, "focus", ["question", "student"] as const);
  const query = new URLSearchParams({ focus: checkedFocus });
  if (cursor !== undefined) query.set("cursor", decodeCursor(cursor, "cursor"));
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  const path = `${courseAssignmentPath(courseId, assignmentId)}/grading-operations`;
  return `${path}?${query.toString()}`;
}

function retryPath(courseId: CourseId, assignmentId: AssignmentId, operation: string): string {
  const reference = decodeInstructorGradingOperationReference(operation);
  return `${courseAssignmentPath(courseId, assignmentId)}/grading-operations/${encodeURIComponent(reference)}/retry`;
}

function recalculatePath(courseId: CourseId, assignmentId: AssignmentId): string {
  return `${courseAssignmentPath(courseId, assignmentId)}/grading-operations/recalculate`;
}

function strongEtagForRevision(revision: number): string {
  return `"${revision}"`;
}

async function operationJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST";
    readonly headers?: Readonly<Record<string, string>>;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers: options.headers,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200) {
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  }
  const body = decoder(await boundedResponseJson(response, path), "response");
  return { body, response };
}

function verifyReceipt(
  receipt: GradingOperationActionReceipt,
  response: Response,
  path: string,
  operation: string | undefined,
): GradingOperationActionReceipt {
  if (operation !== undefined && receipt.operation !== operation) {
    throw new ApiProtocolError(`API response ${path} operation must match the requested operation`);
  }
  const etag = response.headers.get("etag");
  if (etag === null) throw new ApiProtocolError(`API response ${path} must include a strong ETag`);
  const checkedEtag = decodeGradingOperationStrongEtag(etag, `API response ${path} ETag`);
  const expectedEtag =
    receipt.kind === "retry"
      ? strongEtagForRevision(receipt.resulting_operation_revision)
      : strongEtagForRevision(receipt.assignment_revision);
  if (checkedEtag !== expectedEtag) {
    throw new ApiProtocolError(`API response ${path} ETag must match the returned revision`);
  }
  return receipt;
}

/** Creates the W6 recovery capability without coupling transport to a page state machine. */
export function createGradingOperationsClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "listInstructorGradingOperations"
  | "retryInstructorGradingOperation"
  | "recalculateInstructorAssignment"
> {
  return {
    listInstructorGradingOperations: async (
      courseId,
      assignmentId,
      focus: GradingOperationFocus = "question",
      cursor,
      pageSize,
    ): Promise<InstructorGradingOperationsPage> => {
      const path = listPath(courseId, assignmentId, focus, cursor, pageSize);
      const result = await operationJson(
        fetchImplementation,
        basePath,
        path,
        decodeInstructorGradingOperationsPage,
      );
      return result.body;
    },
    retryInstructorGradingOperation: async (
      courseId,
      assignmentId,
      operation,
      expectedRevision: GradingOperationStrongEtag,
    ): Promise<GradingOperationActionReceipt> => {
      const path = retryPath(courseId, assignmentId, operation);
      const revision = decodeGradingOperationStrongEtag(expectedRevision, "expectedRevision");
      const result = await operationJson(
        fetchImplementation,
        basePath,
        path,
        decodeGradingOperationActionReceipt,
        { method: "POST", headers: { "if-match": revision } },
      );
      return verifyReceipt(result.body, result.response, path, operation);
    },
    recalculateInstructorAssignment: async (
      courseId,
      assignmentId,
      expectedRevision: GradingOperationStrongEtag,
    ): Promise<GradingOperationActionReceipt> => {
      const path = recalculatePath(courseId, assignmentId);
      const revision = decodeGradingOperationStrongEtag(expectedRevision, "expectedRevision");
      const result = await operationJson(
        fetchImplementation,
        basePath,
        path,
        decodeGradingOperationActionReceipt,
        { method: "POST", headers: { "if-match": revision } },
      );
      return verifyReceipt(result.body, result.response, path, undefined);
    },
  };
}
