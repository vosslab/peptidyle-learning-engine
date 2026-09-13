// Strict same-origin transport for Blueprint lineage metadata and immutable Revisions.

import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseSaveResponse } from "../../../generated/api/BlueprintCourseSaveResponse";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintMetadataState } from "../../../generated/api/BlueprintMetadataState";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { ApiClient } from "../client";
import type { CursorPage } from "../contracts";
import {
  decodeBlueprintCoursePage,
  decodeBlueprintCourseReference,
  decodeBlueprintCourseSaveResponse,
  decodeBlueprintCourseView,
  decodeBlueprintMetadataState,
  decodeBlueprintRevision,
  decodeBlueprintRevisionView,
  decodeCreateBlueprintCourseInput,
  decodeRenameBlueprintCourseInput,
  decodeReplaceBlueprintCourseContentInput,
} from "../decoders/blueprint_course";
import type {
  BlueprintCourseClient,
  BlueprintIdempotencyKey,
  BlueprintMetadataTransition,
  LoadedBlueprintCourse,
  BlueprintRevisionEtag,
} from "../blueprint_course";
import { ApiProtocolError, ApiRequestError, BlueprintCourseConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;
const MAX_IDEMPOTENCY_KEY_BYTES = 128;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

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

function parseRevisionEtag(value: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value) || BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong positive Revision validator`);
  }
  return value;
}

function parseMetadataEtag(value: string, path: string): string {
  const unquoted = /^"(.+)"$/u.exec(value)?.[1];
  if (unquoted === undefined || !UUID.test(unquoted)) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong opaque metadata validator`);
  }
  return value;
}

function metadataEtag(value: string, path: string): string {
  if (!UUID.test(value)) {
    throw new ApiProtocolError(`API ${path} metadata ETag must be a canonical UUID`);
  }
  return `"${value}"`;
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

async function blueprintJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: string;
    readonly parseEtag?: (value: string, path: string) => string;
    readonly idempotencyKey?: BlueprintIdempotencyKey;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined) {
    headers["if-match"] = (options.parseEtag ?? parseRevisionEtag)(
      options.etag,
      `${path} If-Match`,
    );
  }
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

function requireRevisionEtag(response: Response, revision: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || parseRevisionEtag(etag, path) !== `"${revision}"`) {
    throw new ApiProtocolError(
      `API response ${path} ETag must match its current Blueprint Revision`,
    );
  }
  return etag;
}

function requireMetadataEtag(
  response: Response,
  state: BlueprintMetadataState,
  path: string,
): string {
  const etag = response.headers.get("etag");
  const expected = metadataEtag(state.metadata_etag, path);
  if (etag === null || parseMetadataEtag(etag, path) !== expected) {
    throw new ApiProtocolError(
      `API response ${path} ETag must match its opaque metadata validator`,
    );
  }
  return etag;
}

function loadedBlueprintCourse(
  body: BlueprintCourseView,
  response: Response,
  path: string,
): LoadedBlueprintCourse {
  return {
    blueprintCourse: body,
    revisionEtag: requireRevisionEtag(response, body.current_revision.revision, path),
  };
}

function metadataTransition(
  metadata: BlueprintMetadataState,
  response: Response,
  path: string,
): BlueprintMetadataTransition {
  return { metadata, metadataEtag: requireMetadataEtag(response, metadata, path) };
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
          body: decodeCreateBlueprintCourseInput(content),
          idempotencyKey: requestKey,
          expectedStatus: 201,
        },
      );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    saveBlueprintCourse: async (
      reference,
      content,
      etag,
      requestKey,
    ): Promise<BlueprintCourseSaveResponse & { readonly revisionEtag: BlueprintRevisionEtag }> => {
      const path = blueprintPath(reference);
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseSaveResponse,
        {
          method: "PUT",
          body: decodeReplaceBlueprintCourseContentInput(content),
          etag,
          parseEtag: parseRevisionEtag,
          idempotencyKey: requestKey,
          expectedStatus: 200,
        },
      );
      return {
        ...result.body,
        revisionEtag: requireRevisionEtag(
          result.response,
          result.body.blueprintCourse.current_revision.revision,
          path,
        ),
      };
    },
    renameBlueprintCourse: async (reference, names, etag): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/metadata`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "PUT",
          body: decodeRenameBlueprintCourseInput(names),
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    getBlueprintRevision: async (reference, revision): Promise<BlueprintRevisionView> => {
      const path = `${blueprintPath(reference)}/revisions/${encodeURIComponent(decodeBlueprintRevision(revision, "revision"))}`;
      return (await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintRevisionView))
        .body;
    },
    archiveBlueprintCourse: async (
      reference,
      confirmationLongName,
      etag,
    ): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/archive`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "POST",
          body: { confirmationLongName },
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    restoreBlueprintCourse: async (reference, etag): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/restore`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "POST",
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
  };
}
