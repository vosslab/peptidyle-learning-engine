// Same-origin, no-store transport; server sessions authorize each role and projection.
import type {
  BlueprintStewardshipClient,
  BlueprintPromotion,
  BlueprintWatchEvent,
} from "../blueprint_stewardship";
import { decodeBoolean } from "../decoder";
import { decodeBlueprintCourseReference, metadataEtag } from "../decoders/blueprint_course";
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

function referencePath(reference: string): string {
  // ASVS 1.2.2: validated references remain encoded as one path component.
  return encodeURIComponent(decodeBlueprintCourseReference(reference));
}

function strongMetadataEtag(value: string): string {
  if (!value.startsWith('"') || !value.endsWith('"'))
    throw new ApiProtocolError("Blueprint promotion requires a strong metadata ETag");
  metadataEtag(value.slice(1, -1), "metadataEtag");
  return value;
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
      headers: etag === undefined ? {} : { "if-match": strongMetadataEtag(etag) },
    });
    requireNoStore(response, path);
    if (response.status === 412) throw new BlueprintCourseConflictError(path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    if (response.status !== 200)
      throw new ApiProtocolError("Blueprint stewardship requires status 200");
    return { body: decoder(await boundedResponseJson(response, path)), response };
  }

  function path(reference: string, suffix: string): string {
    return `/api/course-blueprints/${referencePath(reference)}/stewardship/${suffix}`;
  }

  async function promotion(
    reference: string,
    promoted?: boolean,
    etag?: string,
  ): Promise<BlueprintPromotion> {
    const result = await request(
      `/api/sysadmin/course-blueprints/${referencePath(reference)}/promotion`,
      decodeBlueprintPromotion,
      promoted === undefined ? undefined : { promoted: decodeBoolean(promoted, "promoted") },
      etag,
    );
    const validator = result.response.headers.get("etag");
    if (validator === null || strongMetadataEtag(validator) !== `"${result.body.metadataEtag}"`)
      throw new ApiProtocolError("Blueprint promotion ETag must match its metadata validator");
    return { promoted: result.body.promoted, metadataEtag: validator };
  }

  return {
    getBlueprintStar: async (reference) =>
      (await request(path(reference, "star"), decodeBlueprintStar)).body,
    setBlueprintStar: async (reference, starred) =>
      (
        await request(path(reference, "star"), decodeBlueprintStar, {
          starred: decodeBoolean(starred, "starred"),
        })
      ).body,
    getBlueprintStarredInstructors: async (reference) =>
      (await request(path(reference, "starred-instructors"), decodeBlueprintStarredInstructors))
        .body,
    getBlueprintWatch: async (reference) =>
      (await request(path(reference, "watch"), decodeBlueprintWatch)).body,
    setBlueprintWatch: async (reference, watching) =>
      (
        await request(path(reference, "watch"), decodeBlueprintWatch, {
          watching: decodeBoolean(watching, "watching"),
        })
      ).body,
    getBlueprintWatchEvents: async (
      reference,
      limit = 25,
    ): Promise<readonly BlueprintWatchEvent[]> => {
      if (!Number.isSafeInteger(limit) || limit < 1 || limit > 100)
        throw new ApiProtocolError("Blueprint Watch event limit must be from 1 through 100");
      const result = await request(
        `${path(reference, "watch-events")}?${new URLSearchParams({ limit: String(limit) })}`,
        decodeBlueprintWatchEvents,
      );
      if (result.body.length > limit)
        throw new ApiProtocolError("Blueprint Watch events exceed requested limit");
      return result.body;
    },
    getBlueprintPromotion: (reference) => promotion(reference),
    setBlueprintPromotion: (reference, promoted, etag) =>
      promotion(reference, decodeBoolean(promoted, "promoted"), strongMetadataEtag(etag)),
  };
}
