// Strict decoder for the closed Published Question Star projection.

import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";
import type { QuestionStarProjection, QuestionStarredInstructor } from "../question_star";

const MAX_VERIFIED_INSTRUCTOR_DISPLAY_NAME_SCALARS = 200;
const CONTROL_CHARACTER = /[\p{Cc}]/u;

function decodeVerifiedInstructorDisplayName(value: unknown, path: string): string {
  const displayName = decodeNonemptyString(value, path);
  if (
    displayName !== displayName.trim() ||
    CONTROL_CHARACTER.test(displayName) ||
    Array.from(displayName).length > MAX_VERIFIED_INSTRUCTOR_DISPLAY_NAME_SCALARS
  ) {
    throw new DecodeError(path, "one verified Instructor display name");
  }
  return displayName;
}

function decodeStarredInstructor(value: unknown, path: string): QuestionStarredInstructor {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["displayName"]);
  return {
    displayName: decodeVerifiedInstructorDisplayName(
      field(record, "displayName", path),
      `${path}.displayName`,
    ),
  };
}

/** Rejects every substitute identity and all Watch data from the Star projection. */
export function decodeQuestionStarProjection(
  value: unknown,
  path = "response",
): QuestionStarProjection {
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
