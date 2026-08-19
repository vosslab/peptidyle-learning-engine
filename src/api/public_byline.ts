// Browser boundary for the reviewed public-attribution wire contract.

import type { PublicByline } from "../../generated/api/PublicByline";
import { DecodeError, decodeArray, decodeRecord, decodeString } from "./decoder";
import { field, requireOnlyFields } from "./decoders/shared";

const MAX_PUBLIC_AUTHOR_NAMES = 16;
const MAX_PUBLIC_AUTHOR_NAME_SCALARS = 120;
const CONTROL_CHARACTER = /[\p{Cc}]/u;

function isValidPublicAuthorName(name: string): boolean {
  return (
    name.length > 0 &&
    name === name.trim() &&
    !CONTROL_CHARACTER.test(name) &&
    Array.from(name).length <= MAX_PUBLIC_AUTHOR_NAME_SCALARS
  );
}

/** Returns the exact reviewed-attribution value or null for locally correctable author input. */
export function parseReviewedPublicByline(text: string): PublicByline | null {
  const names = text.split("\n").map((name) => name.trim());
  if (
    names.length === 0 ||
    names.length > MAX_PUBLIC_AUTHOR_NAMES ||
    names.some((name) => !isValidPublicAuthorName(name)) ||
    new Set(names).size !== names.length
  ) {
    return null;
  }
  return { names };
}

/** Validates an already-shaped publication command before it crosses the HTTP boundary. */
export function isPublicByline(value: PublicByline): boolean {
  return (
    value.names.length > 0 &&
    value.names.length <= MAX_PUBLIC_AUTHOR_NAMES &&
    value.names.every(isValidPublicAuthorName) &&
    new Set(value.names).size === value.names.length
  );
}

/** Strictly decodes the generated `{ names }` public wire shape. */
export function decodePublicByline(value: unknown, path: string): PublicByline {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["names"]);
  const names = decodeArray(field(record, "names", path), `${path}.names`, decodeString);
  if (!isPublicByline({ names })) {
    throw new DecodeError(path, "one to sixteen distinct reviewed author names");
  }
  return { names };
}
