// Strict decoder for the closed Published Question Star projection.

import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodeRecord,
} from "../decoder";
import { decodeAccountId, field, requireOnlyFields } from "./shared";
import { decodeLibrarySearchClassification, decodeQuestionSearchResult } from "./question_library";
import { decodeQuestionSearchFacets } from "./question_type_facets";
import { validateCanonicalPublicId } from "../../question_id";
import type {
  QuestionStarProjection,
  QuestionStarredInstructor,
  StarredQuestionPage,
  StarredQuestionSummary,
} from "../question_star";

const MAX_INSTRUCTOR_DISPLAY_NAME_SCALARS = 200;
const CONTROL_CHARACTER = /[\p{Cc}]/u;

function decodeInstructorDisplayName(value: unknown, path: string): string {
  const displayName = decodeNonemptyString(value, path);
  if (
    displayName !== displayName.trim() ||
    CONTROL_CHARACTER.test(displayName) ||
    Array.from(displayName).length > MAX_INSTRUCTOR_DISPLAY_NAME_SCALARS
  ) {
    throw new DecodeError(path, "one Instructor display name");
  }
  return displayName;
}

function decodeStarredInstructor(value: unknown, path: string): QuestionStarredInstructor {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["displayName", "accountId"]);
  const accountId = field(record, "accountId", path);
  if (typeof accountId !== "string" || validateCanonicalPublicId("account", accountId) === null)
    throw new DecodeError(`${path}.accountId`, "a canonical Instructor Account ID");
  return {
    displayName: decodeInstructorDisplayName(
      field(record, "displayName", path),
      `${path}.displayName`,
    ),
    accountId,
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

function decodeStarredQuestionSummary(value: unknown, path: string): StarredQuestionSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "question",
    "ownerAccountId",
    "classification",
    "starredAtMicros",
  ]);
  const starredAtMicros = decodeNonnegativeInteger(
    field(record, "starredAtMicros", path),
    `${path}.starredAtMicros`,
  );
  if (!Number.isSafeInteger(starredAtMicros))
    throw new DecodeError(`${path}.starredAtMicros`, "a safe microsecond timestamp");
  return {
    question: decodeQuestionSearchResult(field(record, "question", path), `${path}.question`),
    ownerAccountId: decodeAccountId(
      field(record, "ownerAccountId", path),
      `${path}.ownerAccountId`,
    ),
    classification: decodeLibrarySearchClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    starredAtMicros,
  };
}

/** Rejects Account identity and every field outside the personal Star collection. */
export function decodeStarredQuestionPage(value: unknown, path = "response"): StarredQuestionPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor", "facets"]);
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
    facets: decodeQuestionSearchFacets(field(record, "facets", path), `${path}.facets`),
  };
}
