// Strict whole-result facet response for Blueprint Course discovery.
import type { BlueprintCourseFacetChoices } from "../blueprint_course";
import {
  DecodeError,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodeRecord,
} from "../decoder";
import { decodeBoundedArray, field, requireOnlyFields } from "./shared";

export function decodeBlueprintCourseCursor(value: unknown, path: string): string {
  const cursor = decodeNonemptyString(value, path);
  if (cursor.length > 16384)
    throw new DecodeError(path, "a Blueprint cursor at most 16384 characters");
  return cursor;
}

function choice(value: unknown, path: string): BlueprintCourseFacetChoices["choices"][number] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["key", "label", "count"]);
  return {
    key: decodeNonemptyString(field(record, "key", path), `${path}.key`),
    label: decodeNonemptyString(field(record, "label", path), `${path}.label`),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

export function decodeBlueprintCourseFacetChoices(
  value: unknown,
  path = "response",
): BlueprintCourseFacetChoices {
  // ASVS 1.5.2: accept only the bounded facet response shape from the service.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["choices", "selected", "nextCursor"]);
  return {
    choices: decodeBoundedArray(field(record, "choices", path), `${path}.choices`, 100, choice),
    selected: decodeBoundedArray(field(record, "selected", path), `${path}.selected`, 20, choice),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeBlueprintCourseCursor,
    ),
  };
}
