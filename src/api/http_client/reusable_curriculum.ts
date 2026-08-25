// Strict same-origin transport for instructor reusable curricula.

import type { AlphaCourseDefinitionInput } from "../../../generated/api/AlphaCourseDefinitionInput";
import type { AlphaCourseReference } from "../../../generated/api/AlphaCourseReference";
import type { AlphaCourseSummaryView } from "../../../generated/api/AlphaCourseSummaryView";
import type { BlueprintDefinitionInput } from "../../../generated/api/BlueprintDefinitionInput";
import type { BlueprintReference } from "../../../generated/api/BlueprintReference";
import type { BlueprintSummaryView } from "../../../generated/api/BlueprintSummaryView";
import type { ApiClient } from "../client";
import type { CursorPage } from "../contracts";
import {
  decodeAlphaCourseDefinitionInput,
  decodeAlphaCoursePage,
  decodeAlphaCourseReference,
  decodeAlphaCourseView,
  decodeBlueprintDefinitionInput,
  decodeBlueprintPage,
  decodeBlueprintReference,
  decodeBlueprintView,
} from "../decoders/reusable_curriculum";
import type {
  ReusableCurriculumClient,
  ReusableCurriculumEtag,
  RevisionedAlphaCourse,
  RevisionedBlueprint,
} from "../reusable_curriculum";
import { ApiProtocolError, ApiRequestError, ReusableCurriculumConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;

function pagePath(path: string, cursor: string | undefined, pageSize: number | undefined): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE)
  ) {
    throw new ApiProtocolError("curriculum page size must be an integer from 1 through 100");
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

function requireMatchingEtag(response: Response, revision: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || parseStrongEtag(etag, path) !== `"${revision}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its revision`);
  }
  return etag;
}

function requestRevision(value: ReusableCurriculumEtag, path: string): string {
  return parseStrongEtag(value, `${path} If-Match`);
}

function blueprintPath(value: BlueprintReference): string {
  const reference = decodeBlueprintReference(value, "blueprint");
  return `/api/course-blueprints/${encodeURIComponent(reference)}`;
}

function alphaPath(value: AlphaCourseReference): string {
  const reference = decodeAlphaCourseReference(value, "alpha");
  return `/api/alpha-courses/${encodeURIComponent(reference)}`;
}

async function curriculumJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: ReusableCurriculumEtag;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined) headers["if-match"] = requestRevision(options.etag, path);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new ReusableCurriculumConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.expectedStatus !== undefined && response.status !== options.expectedStatus) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.expectedStatus}`);
  }
  const body = decoder(await boundedResponseJson(response, path), "response");
  return { body, response };
}

async function deleteCurriculum(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  etag: ReusableCurriculumEtag,
): Promise<void> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "DELETE",
    headers: { "if-match": requestRevision(etag, path) },
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new ReusableCurriculumConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204 || (await response.text()).length !== 0) {
    throw new ApiProtocolError(`API response ${path} must use an empty 204 response`);
  }
}

/** Creates the complete reusable-curriculum capability without coupling it to a screen model. */
export function createReusableCurriculumClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof ReusableCurriculumClient> {
  return {
    listBlueprints: async (cursor, pageSize): Promise<CursorPage<BlueprintSummaryView>> => {
      const path = pagePath("/api/course-blueprints", cursor, pageSize);
      const result = await curriculumJson(fetchImplementation, basePath, path, decodeBlueprintPage);
      return result.body;
    },
    getBlueprint: async (reference): Promise<RevisionedBlueprint> => {
      const path = blueprintPath(reference);
      const result = await curriculumJson(fetchImplementation, basePath, path, decodeBlueprintView);
      return {
        blueprint: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    createBlueprint: async (definition: BlueprintDefinitionInput): Promise<RevisionedBlueprint> => {
      const path = "/api/course-blueprints";
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintView,
        {
          method: "POST",
          body: decodeBlueprintDefinitionInput(definition),
          expectedStatus: 201,
        },
      );
      return {
        blueprint: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    replaceBlueprint: async (reference, definition, etag): Promise<RevisionedBlueprint> => {
      const path = blueprintPath(reference);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintView,
        {
          method: "PUT",
          body: decodeBlueprintDefinitionInput(definition),
          etag,
          expectedStatus: 200,
        },
      );
      return {
        blueprint: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    deleteBlueprint: (reference, etag) =>
      deleteCurriculum(fetchImplementation, basePath, blueprintPath(reference), etag),
    listAlphaCourses: async (cursor, pageSize): Promise<CursorPage<AlphaCourseSummaryView>> => {
      const path = pagePath("/api/alpha-courses", cursor, pageSize);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeAlphaCoursePage,
      );
      return result.body;
    },
    getAlphaCourse: async (reference): Promise<RevisionedAlphaCourse> => {
      const path = alphaPath(reference);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeAlphaCourseView,
      );
      return {
        alpha: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    createAlphaCourse: async (
      definition: AlphaCourseDefinitionInput,
    ): Promise<RevisionedAlphaCourse> => {
      const path = "/api/alpha-courses";
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeAlphaCourseView,
        {
          method: "POST",
          body: decodeAlphaCourseDefinitionInput(definition),
          expectedStatus: 201,
        },
      );
      return {
        alpha: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    replaceAlphaCourse: async (reference, definition, etag): Promise<RevisionedAlphaCourse> => {
      const path = alphaPath(reference);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeAlphaCourseView,
        {
          method: "PUT",
          body: decodeAlphaCourseDefinitionInput(definition),
          etag,
          expectedStatus: 200,
        },
      );
      return {
        alpha: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
  };
}
