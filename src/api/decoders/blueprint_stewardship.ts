// ASVS 1.5.2, 2.2.1, 8.2.3: reject identities and Watch facts outside their exact projection.
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import type {
  BlueprintStarProjection,
  BlueprintStarredInstructor,
  BlueprintWatchProjection,
  BlueprintWatchPage,
} from "../blueprint_stewardship";
import { blueprintEditNumber } from "./blueprint_course";
import { decodeBoundedArray, field, requireOnlyFields } from "./shared";

export function decodeBlueprintStar(value: unknown, path = "response"): BlueprintStarProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["starCount", "viewerHasStarred"]);
  return {
    starCount: decodeNonnegativeInteger(field(record, "starCount", path), `${path}.starCount`),
    viewerHasStarred: decodeBoolean(
      field(record, "viewerHasStarred", path),
      `${path}.viewerHasStarred`,
    ),
  };
}

export function decodeBlueprintStarredInstructors(
  value: unknown,
  path = "response",
): readonly BlueprintStarredInstructor[] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["starredInstructors"]);
  return decodeArray(
    field(record, "starredInstructors", path),
    `${path}.starredInstructors`,
    (value, path) => {
      const instructor = decodeRecord(value, path);
      requireOnlyFields(instructor, path, ["displayName"]);
      const displayName = decodeNonemptyString(
        field(instructor, "displayName", path),
        `${path}.displayName`,
      );
      if (
        displayName !== displayName.trim() ||
        /[\p{Cc}]/u.test(displayName) ||
        Array.from(displayName).length > 200
      )
        throw new DecodeError(`${path}.displayName`, "one Instructor display name");
      return { displayName };
    },
  );
}

export function decodeBlueprintWatch(value: unknown, path = "response"): BlueprintWatchProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["watching"]);
  return { watching: decodeBoolean(field(record, "watching", path), `${path}.watching`) };
}

export function decodeBlueprintWatchEvents(value: unknown, path = "response"): BlueprintWatchPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "items",
    "pageNumber",
    "pageSize",
    "totalCount",
    "eventKindFacets",
  ]);
  const pageNumber = decodeNonnegativeInteger(
    field(record, "pageNumber", path),
    `${path}.pageNumber`,
  );
  const pageSize = decodeNonnegativeInteger(field(record, "pageSize", path), `${path}.pageSize`);
  const totalCount = decodeSafeInteger(field(record, "totalCount", path), `${path}.totalCount`);
  if (pageNumber < 1) throw new DecodeError(`${path}.pageNumber`, "a positive page number");
  if (pageSize < 1 || pageSize > 100)
    throw new DecodeError(`${path}.pageSize`, "a page size from 1 through 100");
  if (totalCount < 0) throw new DecodeError(`${path}.totalCount`, "a nonnegative result count");
  const items = decodeBoundedArray(
    field(record, "items", path),
    `${path}.items`,
    100,
    (value, path) => {
      const event = decodeRecord(value, path);
      requireOnlyFields(event, path, ["eventId", "kind", "occurredAt"]);
      const occurredAt = decodeNonnegativeInteger(
        field(event, "occurredAt", path),
        `${path}.occurredAt`,
      );
      if (Number.isNaN(new Date(occurredAt).getTime()))
        throw new DecodeError(`${path}.occurredAt`, "a valid millisecond timestamp");
      return {
        eventId: decodeUuid(field(event, "eventId", path), `${path}.eventId`),
        kind: decodeStringEnum(field(event, "kind", path), `${path}.kind`, [
          "revision",
          "archived",
          "restored",
        ]),
        occurredAt,
      };
    },
  );
  if (
    items.length > pageSize ||
    (totalCount === 0 && pageNumber !== 1) ||
    (totalCount > 0 && (items.length === 0 || pageNumber > Math.ceil(totalCount / pageSize)))
  ) {
    throw new DecodeError(path, "a consistent numbered Blueprint Watch page");
  }
  const facetKinds = ["revision", "archived", "restored"] as const;
  const eventKindFacets = decodeBoundedArray(
    field(record, "eventKindFacets", path),
    `${path}.eventKindFacets`,
    facetKinds.length,
    (value, facetPath) => {
      const facet = decodeRecord(value, facetPath);
      requireOnlyFields(facet, facetPath, ["value", "count"]);
      return {
        value: decodeStringEnum(field(facet, "value", facetPath), `${facetPath}.value`, facetKinds),
        count: decodeNonnegativeInteger(field(facet, "count", facetPath), `${facetPath}.count`),
      };
    },
  );
  if (new Set(eventKindFacets.map((facet) => facet.value)).size !== eventKindFacets.length)
    throw new DecodeError(`${path}.eventKindFacets`, "unique event kinds");
  return { items, pageNumber, pageSize, totalCount, eventKindFacets };
}

export function decodeBlueprintPromotion(
  value: unknown,
  path = "response",
): { readonly promoted: boolean; readonly blueprintEditNumber: string } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["promoted", "blueprintEditNumber"]);
  return {
    promoted: decodeBoolean(field(record, "promoted", path), `${path}.promoted`),
    blueprintEditNumber: blueprintEditNumber(
      field(record, "blueprintEditNumber", path),
      `${path}.blueprintEditNumber`,
    ),
  };
}
