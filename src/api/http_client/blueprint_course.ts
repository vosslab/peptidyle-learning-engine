// Strict same-origin transport for the Blueprint lineage, Draft, and publication lifecycle.

import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintRevisionReference } from "../../../generated/api/BlueprintRevisionReference";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { ApiClient } from "../client";
import type { CursorPage } from "../contracts";
import {
  decodeBlueprintAvailabilityTransition,
  decodeBlueprintCoursePage,
  decodeBlueprintCourseReference,
  decodeBlueprintCourseView,
  decodeBlueprintPublication,
  decodeBlueprintRevision,
  decodeBlueprintRevisionView,
  decodeCreateBlueprintCourseContentInput,
  decodeReplaceBlueprintCourseContentInput,
} from "../decoders/blueprint_course";
import type {
  BlueprintAvailabilityTransition,
  BlueprintCourseClient,
  BlueprintIdempotencyKey,
  LoadedBlueprintCourse,
} from "../blueprint_course";
import { ApiProtocolError, ApiRequestError, BlueprintCourseConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;
const MAX_IDEMPOTENCY_KEY_BYTES = 128;

function pagePath(path: string, cursor: string | undefined, pageSize: number | undefined): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE)
  ) {
    throw new ApiProtocolError("Blueprint Course page size must be an integer from 1 through 100");
  }
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", cursor);
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${path}${suffix}`;
}

function parseStrongEtag(value: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value) || BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong positive numeric validator`);
  }
  return value;
}

function idempotencyKey(value: BlueprintIdempotencyKey, path: string): string {
  if (
    value.length === 0 ||
    value.length > MAX_IDEMPOTENCY_KEY_BYTES ||
    !Array.from(value).every((character) => {
      const code = character.codePointAt(0);
      return code !== undefined && code >= 0x21 && code <= 0x7e;
    })
  ) {
    throw new ApiProtocolError(
      `API ${path} Idempotency-Key must be 1 through 128 visible ASCII bytes`,
    );
  }
  return value;
}

function blueprintPath(value: BlueprintCourseReference): string {
  return `/api/course-blueprints/${encodeURIComponent(decodeBlueprintCourseReference(value, "blueprint"))}`;
}

function requireMatchingEtag(response: Response, editNumber: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || parseStrongEtag(etag, path) !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its qualified Edit Number`);
  }
  return etag;
}

async function blueprintJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: string;
    readonly idempotencyKey?: BlueprintIdempotencyKey;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined)
    headers["if-match"] = parseStrongEtag(options.etag, `${path} If-Match`);
  if (options.idempotencyKey !== undefined)
    headers["idempotency-key"] = idempotencyKey(options.idempotencyKey, path);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new BlueprintCourseConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.expectedStatus !== undefined && response.status !== options.expectedStatus) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.expectedStatus}`);
  }
  return { body: decoder(await boundedResponseJson(response, path), "response"), response };
}

function loadedBlueprintCourse(
  body: BlueprintCourseView,
  response: Response,
  path: string,
): LoadedBlueprintCourse {
  return {
    blueprintCourse: body,
    draftEtag:
      body.draft === null ? undefined : requireMatchingEtag(response, body.draft.edit_number, path),
  };
}

function availabilityTransition(
  body: {
    readonly availability: BlueprintAvailabilityTransition["availability"];
    readonly editNumber: string;
  },
  response: Response,
  path: string,
): BlueprintAvailabilityTransition {
  return { ...body, etag: requireMatchingEtag(response, body.editNumber, path) };
}

/** Creates the complete Blueprint Course capability without coupling it to a screen model. */
export function createBlueprintCourseClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof BlueprintCourseClient> {
  return {
    listBlueprintCourses: async (
      cursor,
      pageSize,
    ): Promise<CursorPage<BlueprintCourseSummaryView>> => {
      const path = pagePath("/api/course-blueprints", cursor, pageSize);
      return (await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintCoursePage))
        .body;
    },
    getBlueprintCourse: async (reference): Promise<LoadedBlueprintCourse> => {
      const path = blueprintPath(reference);
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
      );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    createBlueprintCourse: async (content, requestKey): Promise<LoadedBlueprintCourse> => {
      const path = "/api/course-blueprints";
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        {
          method: "POST",
          body: decodeCreateBlueprintCourseContentInput(content),
          idempotencyKey: requestKey,
          expectedStatus: 201,
        },
      );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    saveBlueprintDraft: async (
      reference,
      content,
      etag,
      requestKey,
    ): Promise<LoadedBlueprintCourse> => {
      const path = `${blueprintPath(reference)}/draft`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        {
          method: "PUT",
          body: decodeReplaceBlueprintCourseContentInput(content),
          etag,
          idempotencyKey: requestKey,
          expectedStatus: 200,
        },
      );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    publishBlueprintDraft: async (
      reference,
      etag,
      requestKey,
    ): Promise<BlueprintRevisionReference> => {
      const path = `${blueprintPath(reference)}/publish`;
      return (
        await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintPublication, {
          method: "POST",
          etag,
          idempotencyKey: requestKey,
          expectedStatus: 200,
        })
      ).body;
    },
    getBlueprintRevision: async (reference, revision): Promise<BlueprintRevisionView> => {
      const path = `${blueprintPath(reference)}/revisions/${encodeURIComponent(decodeBlueprintRevision(revision, "revision"))}`;
      return (await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintRevisionView))
        .body;
    },
    archiveBlueprintCourse: async (
      reference,
      confirmationTitle,
      etag,
    ): Promise<BlueprintAvailabilityTransition> => {
      const path = `${blueprintPath(reference)}/archive`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintAvailabilityTransition,
        {
          method: "POST",
          body: { confirmationTitle },
          etag,
          expectedStatus: 200,
        },
      );
      return availabilityTransition(result.body, result.response, path);
    },
    restoreBlueprintCourse: async (reference, etag): Promise<BlueprintAvailabilityTransition> => {
      const path = `${blueprintPath(reference)}/restore`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintAvailabilityTransition,
        {
          method: "POST",
          etag,
          expectedStatus: 200,
        },
      );
      return availabilityTransition(result.body, result.response, path);
    },
  };
}
