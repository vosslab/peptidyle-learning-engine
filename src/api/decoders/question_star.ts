// Strict decoder for the closed Published Question Star projection.

import { MAX_QUESTION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_QUESTION_TITLE_UNICODE_SCALARS";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodeRecord,
} from "../decoder";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";
import type {
  QuestionStarProjection,
  QuestionStarredInstructor,
  StarredQuestionPage,
  StarredQuestionSummary,
} from "../question_star";

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

function decodeQuestionTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (
    title !== title.trim() ||
    CONTROL_CHARACTER.test(title) ||
    Array.from(title).length > MAX_QUESTION_TITLE_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "a Question title");
  }
  return title;
}

function decodeStarredQuestionSummary(value: unknown, path: string): StarredQuestionSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "questionTitle"]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    questionTitle: decodeQuestionTitle(
      field(record, "questionTitle", path),
      `${path}.questionTitle`,
    ),
  };
}

/** Rejects Account identity and every field outside the personal Star collection. */
export function decodeStarredQuestionPage(value: unknown, path = "response"): StarredQuestionPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  const nextCursor = decodeNullable(
    field(record, "nextCursor", path),
    `${path}.nextCursor`,
    (cursor, cursorPath) => decodeNonemptyString(cursor, cursorPath),
  );
  if (nextCursor !== null && nextCursor.length > 1024) {
    throw new DecodeError(`${path}.nextCursor`, "a bounded Starred Question cursor");
  }
  return {
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeStarredQuestionSummary),
    nextCursor,
  };
}
