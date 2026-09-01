// Strict same-origin transport for server-owned Blueprint operations.

import type { BlueprintOperationApplyIntent } from "../../../generated/api/BlueprintOperationApplyIntent";
import type { BlueprintOperationCompleted } from "../../../generated/api/BlueprintOperationCompleted";
import type { BlueprintOperationPreview } from "../../../generated/api/BlueprintOperationPreview";
import type { BlueprintOperationPreviewRequest } from "../../../generated/api/BlueprintOperationPreviewRequest";
import type { ApiClient } from "../client";
import type { BlueprintOperationsClient } from "../blueprint_operations";
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

function decodeOperationEnvelope<T>(value: unknown, path: string, payload: string): T {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new ApiProtocolError(`API ${path} must be an operation envelope`);
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
  return decodeOperationEnvelope<BlueprintOperationPreviewRequest>(value, "request", "request");
}

function decodeApplyIntent(value: BlueprintOperationApplyIntent): BlueprintOperationApplyIntent {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new ApiProtocolError("API apply intent must be an object");
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record);
  if (keys.length !== 2 || !keys.includes("request") || !keys.includes("idempotency_key")) {
    throw new ApiProtocolError("API apply intent must contain only request and idempotency_key");
  }
  decodePreviewRequest(record.request as BlueprintOperationPreviewRequest);
  if (
    typeof record.idempotency_key !== "string" ||
    !/^[A-Za-z0-9._-]+$/u.test(record.idempotency_key) ||
    new TextEncoder().encode(record.idempotency_key).length > 128
  ) {
    throw new ApiProtocolError("API apply intent requires a bounded idempotency key");
  }
  return value;
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
        (value, path) => decodeOperationEnvelope<BlueprintOperationPreview>(value, path, "preview"),
      ),
    applyBlueprintOperation: (intent) =>
      post(fetchImplementation, basePath, APPLY_PATH, decodeApplyIntent(intent), (value, path) =>
        decodeOperationEnvelope<BlueprintOperationCompleted>(value, path, "completed"),
      ),
  };
}
