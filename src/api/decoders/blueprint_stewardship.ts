// ASVS 1.5.2, 2.2.1, 8.2.3: reject identities and Watch facts outside their exact projection.
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import type {
  BlueprintStarProjection,
  BlueprintStarredInstructor,
  BlueprintWatchProjection,
  BlueprintWatchEvent,
} from "../blueprint_stewardship";
import { metadataEtag } from "./blueprint_course";
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
        throw new DecodeError(`${path}.displayName`, "one verified Instructor display name");
      return { displayName };
    },
  );
}

export function decodeBlueprintWatch(value: unknown, path = "response"): BlueprintWatchProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["watching"]);
  return { watching: decodeBoolean(field(record, "watching", path), `${path}.watching`) };
}

export function decodeBlueprintWatchEvents(
  value: unknown,
  path = "response",
): readonly BlueprintWatchEvent[] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["events"]);
  return decodeBoundedArray(field(record, "events", path), `${path}.events`, 100, (value, path) => {
    const event = decodeRecord(value, path);
    requireOnlyFields(event, path, ["kind", "occurredAt"]);
    const occurredAt = decodeNonnegativeInteger(
      field(event, "occurredAt", path),
      `${path}.occurredAt`,
    );
    if (Number.isNaN(new Date(occurredAt).getTime()))
      throw new DecodeError(`${path}.occurredAt`, "a valid millisecond timestamp");
    return {
      kind: decodeStringEnum(field(event, "kind", path), `${path}.kind`, [
        "revision",
        "published",
        "archived",
        "restored",
      ]),
      occurredAt,
    };
  });
}

export function decodeBlueprintPromotion(
  value: unknown,
  path = "response",
): { readonly promoted: boolean; readonly metadataEtag: string } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["promoted", "metadataEtag"]);
  return {
    promoted: decodeBoolean(field(record, "promoted", path), `${path}.promoted`),
    metadataEtag: metadataEtag(field(record, "metadataEtag", path), `${path}.metadataEtag`),
  };
}
