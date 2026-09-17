// Strict decoders for Pool endorsements and private own-Watch state.

import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";
import type {
  QuestionPoolStarProjection,
  QuestionPoolStarredInstructor,
  QuestionPoolWatchProjection,
} from "../question_pool_stewardship";

function decodeStarredInstructor(value: unknown, path: string): QuestionPoolStarredInstructor {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["displayName"]);
  const displayName = decodeString(field(record, "displayName", path), `${path}.displayName`);
  // Canonical SQL btrim(text) strips ASCII spaces only. Preserve valid NBSP
  // and all other approved non-control Unicode scalars without normalization.
  if (
    displayName.length === 0 ||
    displayName.startsWith(" ") ||
    displayName.endsWith(" ") ||
    /[\p{Cc}]/u.test(displayName) ||
    Array.from(displayName).length > 200
  ) {
    throw new DecodeError(`${path}.displayName`, "one verified Instructor display name");
  }
  return { displayName };
}

/** ASVS 15.3.1: reject substitute identities and every Watch field. */
export function decodeQuestionPoolStarProjection(
  value: unknown,
  path = "response",
): QuestionPoolStarProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["starCount", "viewerHasStarred", "starredInstructors"]);
  const starredInstructors = decodeArray(
    field(record, "starredInstructors", path),
    `${path}.starredInstructors`,
    decodeStarredInstructor,
  );
  const starCount = decodeNonnegativeInteger(field(record, "starCount", path), `${path}.starCount`);
  if (starredInstructors.length > starCount) {
    throw new DecodeError(path, "a Star projection with no more display names than Stars");
  }
  return {
    starCount,
    viewerHasStarred: decodeBoolean(
      field(record, "viewerHasStarred", path),
      `${path}.viewerHasStarred`,
    ),
    starredInstructors,
  };
}

/** Accepts only the current Instructor's boolean, never other-watchers data. */
export function decodeQuestionPoolWatchProjection(
  value: unknown,
  path = "response",
): QuestionPoolWatchProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["watching"]);
  return { watching: decodeBoolean(field(record, "watching", path), `${path}.watching`) };
}
