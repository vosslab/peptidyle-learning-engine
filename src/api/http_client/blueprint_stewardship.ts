// Same-origin, no-store transport; server sessions authorize each role and projection.
import type {
  BlueprintStewardshipClient,
  BlueprintPromotion,
  BlueprintWatchEvent,
} from "../blueprint_stewardship";
import { decodeBoolean } from "../decoder";
import { blueprintEditNumber, decodeBlueprintCourseId } from "../decoders/blueprint_course";
import {
  decodeBlueprintStar,
  decodeBlueprintStarredInstructors,
  decodeBlueprintWatch,
  decodeBlueprintWatchEvents,
  decodeBlueprintPromotion,
} from "../decoders/blueprint_stewardship";
import { ApiProtocolError, ApiRequestError, BlueprintCourseConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function blueprintCoursePath(blueprintCourseId: string): string {
  // ASVS 1.2.2: validated Blueprint Course IDs remain encoded as one path component.
  return encodeURIComponent(decodeBlueprintCourseId(blueprintCourseId, "id"));
}

function quotedBlueprintEditNumber(value: string): string {
  return `"${blueprintEditNumber(value, "blueprintEditNumber")}"`;
}

export function createBlueprintStewardshipClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): BlueprintStewardshipClient {
  async function request<T>(
    path: string,
    decoder: (value: unknown) => T,
    body?: unknown,
    etag?: string,
  ): Promise<{ readonly body: T; readonly response: Response }> {
    const response = await requestSameOrigin(fetchImplementation, basePath, path, {
      method: body === undefined ? "GET" : "PUT",
      body,
      headers: etag === undefined ? {} : { "if-match": quotedBlueprintEditNumber(etag) },
    });
    requireNoStore(response, path);
    if (response.status === 412) throw new BlueprintCourseConflictError(path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    if (response.status !== 200)
      throw new ApiProtocolError("Blueprint stewardship requires status 200");
    return { body: decoder(await boundedResponseJson(response, path)), response };
  }

  function path(blueprintCourseId: string, suffix: string): string {
    return `/api/course-blueprints/${blueprintCoursePath(blueprintCourseId)}/stewardship/${suffix}`;
  }

  async function promotion(
    blueprintCourseId: string,
    promoted?: boolean,
    etag?: string,
  ): Promise<BlueprintPromotion> {
    const result = await request(
      `/api/sysadmin/course-blueprints/${blueprintCoursePath(blueprintCourseId)}/promotion`,
      decodeBlueprintPromotion,
      promoted === undefined ? undefined : { promoted: decodeBoolean(promoted, "promoted") },
      etag,
    );
    const validator = result.response.headers.get("etag");
    if (validator === null || validator !== `"${result.body.blueprintEditNumber}"`)
      throw new ApiProtocolError("Blueprint promotion ETag must match its Blueprint Edit Number");
    return {
      promoted: result.body.promoted,
      blueprintEditNumber: result.body.blueprintEditNumber,
    };
  }

  return {
    getBlueprintStar: async (blueprintCourseId) =>
      (await request(path(blueprintCourseId, "star"), decodeBlueprintStar)).body,
    setBlueprintStar: async (blueprintCourseId, starred) =>
      (
        await request(path(blueprintCourseId, "star"), decodeBlueprintStar, {
          starred: decodeBoolean(starred, "starred"),
        })
      ).body,
    getBlueprintStarredInstructors: async (blueprintCourseId) =>
      (await request(path(blueprintCourseId, "starred-instructors"), decodeBlueprintStarredInstructors))
        .body,
    getBlueprintWatch: async (blueprintCourseId) =>
      (await request(path(blueprintCourseId, "watch"), decodeBlueprintWatch)).body,
    setBlueprintWatch: async (blueprintCourseId, watching) =>
      (
        await request(path(blueprintCourseId, "watch"), decodeBlueprintWatch, {
          watching: decodeBoolean(watching, "watching"),
        })
      ).body,
    getBlueprintWatchEvents: async (
      blueprintCourseId,
      limit = 25,
    ): Promise<readonly BlueprintWatchEvent[]> => {
      if (!Number.isSafeInteger(limit) || limit < 1 || limit > 100)
        throw new ApiProtocolError("Blueprint Watch event limit must be from 1 through 100");
      const result = await request(
        `${path(blueprintCourseId, "watch-events")}?${new URLSearchParams({ limit: String(limit) })}`,
        decodeBlueprintWatchEvents,
      );
      if (result.body.length > limit)
        throw new ApiProtocolError("Blueprint Watch events exceed requested limit");
      return result.body;
    },
    getBlueprintPromotion: (blueprintCourseId) => promotion(blueprintCourseId),
    setBlueprintPromotion: (blueprintCourseId, promoted, etag) =>
      promotion(blueprintCourseId, decodeBoolean(promoted, "promoted"), etag),
  };
}
