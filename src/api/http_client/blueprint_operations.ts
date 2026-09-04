// Strict same-origin transport for server-owned Blueprint operations.

import type { BlueprintOperationApplyIntent } from "../../../generated/api/BlueprintOperationApplyIntent";
import type { BlueprintOperationCompleted } from "../../../generated/api/BlueprintOperationCompleted";
import type { BlueprintOperationPreview } from "../../../generated/api/BlueprintOperationPreview";
import type { BlueprintOperationPreviewRequest } from "../../../generated/api/BlueprintOperationPreviewRequest";
import type { ApiClient } from "../client";
import type { BlueprintOperationsClient } from "../blueprint_operations";
import { decodeRecord } from "../decoder";
import {
  decodeBlueprintCourseReference,
  decodeBlueprintRevision,
} from "../decoders/blueprint_course";
import {
  decodeAssignmentReference,
  decodeCourseInstanceReference,
  field,
  requireOnlyFields,
} from "../decoders/shared";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const PREVIEW_PATH = "/api/blueprint-operations/preview";
const APPLY_PATH = "/api/blueprint-operations/apply";
const OPERATIONS = new Set([
  "fork_blueprint_course",
  "create_course_from_blueprint",
  "copy_course_for_new_term",
  "shift_course_dates",
  "apply_blueprint_update",
  "copy_assignment_from_blueprint",
]);

function decodeOperationRecord<T>(value: unknown, path: string, payload: string): T {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new ApiProtocolError(`API ${path} must be an operation record`);
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record);
  if (keys.length !== 2 || !keys.includes("operation") || !keys.includes(payload)) {
    throw new ApiProtocolError(`API ${path} must contain only operation and ${payload}`);
  }
  if (typeof record.operation !== "string" || !OPERATIONS.has(record.operation)) {
    throw new ApiProtocolError(`API ${path} must name a current Blueprint Course operation`);
  }
  if (
    record[payload] === null ||
    typeof record[payload] !== "object" ||
    Array.isArray(record[payload])
  ) {
    throw new ApiProtocolError(`API ${path}.${payload} must be an object`);
  }
  return value as T;
}

function decodePreviewRequest(
  value: BlueprintOperationPreviewRequest,
): BlueprintOperationPreviewRequest {
  return decodeOperationRecord<BlueprintOperationPreviewRequest>(value, "request", "request");
}

function decodeApplyIntent(value: BlueprintOperationApplyIntent): BlueprintOperationApplyIntent {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new ApiProtocolError("API apply intent must be an object");
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record);
  if (keys.length !== 1 || !keys.includes("request")) {
    throw new ApiProtocolError("API apply intent must contain only request");
  }
  decodePreviewRequest(record.request as BlueprintOperationPreviewRequest);
  return value;
}

function decodeCompleted(value: unknown, path: string): BlueprintOperationCompleted {
  const envelope = decodeOperationRecord<BlueprintOperationCompleted>(value, path, "completed");
  const completedPath = `${path}.completed`;
  const completed = decodeRecord(envelope.completed, completedPath);
  switch (envelope.operation) {
    case "fork_blueprint_course":
      requireOnlyFields(completed, completedPath, ["blueprint", "revision"]);
      return {
        operation: envelope.operation,
        completed: {
          blueprint: decodeBlueprintCourseReference(
            field(completed, "blueprint", completedPath),
            `${completedPath}.blueprint`,
          ),
          revision: decodeBlueprintRevision(
            field(completed, "revision", completedPath),
            `${completedPath}.revision`,
          ),
        },
      };
    case "create_course_from_blueprint":
    case "copy_course_for_new_term":
    case "shift_course_dates":
      requireOnlyFields(completed, completedPath, ["course"]);
      return {
        operation: envelope.operation,
        completed: {
          course: decodeCourseInstanceReference(
            field(completed, "course", completedPath),
            `${completedPath}.course`,
          ),
        },
      };
    case "apply_blueprint_update":
    case "copy_assignment_from_blueprint":
      requireOnlyFields(completed, completedPath, ["course", "assignment"]);
      return {
        operation: envelope.operation,
        completed: {
          course: decodeCourseInstanceReference(
            field(completed, "course", completedPath),
            `${completedPath}.course`,
          ),
          assignment: decodeAssignmentReference(
            field(completed, "assignment", completedPath),
            `${completedPath}.assignment`,
          ),
        },
      };
  }
}

async function post<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  body: unknown,
  decoder: (value: unknown, path: string) => T,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "POST",
    body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200)
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  return decoder(await boundedResponseJson(response, path), "response");
}

/** Creates the closed Blueprint-operations capability without coupling it to a screen model. */
export function createBlueprintOperationsClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof BlueprintOperationsClient> {
  return {
    previewBlueprintOperation: (request) =>
      post(
        fetchImplementation,
        basePath,
        PREVIEW_PATH,
        decodePreviewRequest(request),
        (value, path) => decodeOperationRecord<BlueprintOperationPreview>(value, path, "preview"),
      ),
    applyBlueprintOperation: (intent) =>
      post(fetchImplementation, basePath, APPLY_PATH, decodeApplyIntent(intent), decodeCompleted),
  };
}
