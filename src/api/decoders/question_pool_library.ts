// Strict browser decoders for reusable published Question Pool reads.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolMetadata } from "../../../generated/api/QuestionPoolMetadata";
import type { QuestionPoolRevisionMemberView } from "../../../generated/api/QuestionPoolRevisionMemberView";
import type { QuestionPoolRevisionReference } from "../../../generated/api/QuestionPoolRevisionReference";
import type { QuestionPoolRevisionView } from "../../../generated/api/QuestionPoolRevisionView";
import type { ReusableQuestionView } from "../../../generated/api/ReusableQuestionView";
import type { QuestionPoolLibraryPage } from "../question_pool_library";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
  decodeString,
  decodeUuid,
} from "../decoder";
import { decodeQuestionSearchResult } from "./question_library";
import {
  decodeBoundedArray,
  decodeCursor,
  decodeQuestionId,
  decodeQuestionRevisionReference,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_PAGE_SIZE = 100;

/** ASVS 2.2.1: match canonical server-owned Pool text constraints. */
export function decodeQuestionPoolText(value: unknown, path: string, maximum: number): string {
  const text = decodeString(value, path);
  if (
    text.length === 0 ||
    text.startsWith(" ") ||
    text.endsWith(" ") ||
    Array.from(text).length > maximum ||
    Array.from(text).some((character) => {
      const code = character.codePointAt(0)!;
      return code < 32 || (code >= 127 && code <= 159);
    })
  ) {
    throw new DecodeError(
      path,
      `ASCII-space-trimmed, control-free Pool text of 1 to ${maximum} Unicode scalars`,
    );
  }
  return text;
}

/** ASVS 1.5.2: Pool metadata is independent of member Question metadata. */
export function decodeQuestionPoolMetadata(value: unknown, path: string): QuestionPoolMetadata {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "description",
    "disciplineUuid",
    "disciplineName",
    "disciplineIsRetired",
    "subjectUuid",
    "topicUuid",
    "subtopicUuid",
    "tags",
  ]);
  const tags = decodeArray(field(record, "tags", path), `${path}.tags`, (value, tagPath) =>
    decodeQuestionPoolText(value, tagPath, 120),
  );
  if (new Set(tags).size !== tags.length) {
    throw new DecodeError(`${path}.tags`, "unique Pool Tags");
  }
  const topicUuid = decodeNullable(
    field(record, "topicUuid", path),
    `${path}.topicUuid`,
    decodeUuid,
  );
  const subtopicUuid = decodeNullable(
    field(record, "subtopicUuid", path),
    `${path}.subtopicUuid`,
    decodeUuid,
  );
  if (subtopicUuid !== null && topicUuid === null) {
    throw new DecodeError(path, "descendants with their selected parents");
  }
  return {
    title: decodeQuestionPoolText(field(record, "title", path), `${path}.title`, 512),
    description: decodeQuestionPoolText(
      field(record, "description", path),
      `${path}.description`,
      4000,
    ),
    disciplineUuid: decodeUuid(field(record, "disciplineUuid", path), `${path}.disciplineUuid`),
    disciplineName: decodeQuestionPoolText(
      field(record, "disciplineName", path),
      `${path}.disciplineName`,
      120,
    ),
    disciplineIsRetired: decodeBoolean(
      field(record, "disciplineIsRetired", path),
      `${path}.disciplineIsRetired`,
    ),
    subjectUuid: decodeUuid(field(record, "subjectUuid", path), `${path}.subjectUuid`),
    topicUuid,
    subtopicUuid,
    tags,
  };
}

function decodeQuestionPoolRevisionReference(
  value: unknown,
  path: string,
): QuestionPoolRevisionReference {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "revisionNumber"]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    revisionNumber: decodePositiveInteger(
      field(record, "revisionNumber", path),
      `${path}.revisionNumber`,
    ),
  };
}

function decodeQuestionPoolLibrarySummary(
  value: unknown,
  path: string,
): QuestionPoolLibrarySummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolRevision", "metadata", "memberCount"]);
  return {
    metadata: decodeQuestionPoolMetadata(field(record, "metadata", path), `${path}.metadata`),
    questionPoolRevision: decodeQuestionPoolRevisionReference(
      field(record, "questionPoolRevision", path),
      `${path}.questionPoolRevision`,
    ),
    memberCount: decodePositiveInteger(field(record, "memberCount", path), `${path}.memberCount`),
  };
}

function decodeReusableQuestionView(value: unknown, path: string): ReusableQuestionView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "question_library", "selection_availability"]);
  return {
    reference: decodeQuestionRevisionReference(
      field(record, "reference", path),
      `${path}.reference`,
    ),
    question_library: decodeQuestionSearchResult(
      field(record, "question_library", path),
      `${path}.question_library`,
    ),
    selection_availability: decodeStringEnum(
      field(record, "selection_availability", path),
      `${path}.selection_availability`,
      ["available", "retained"],
    ),
  };
}

/** Strictly decodes one ordered exact member for Pool and Assessment-fork readers. */
export function decodeQuestionPoolRevisionMemberView(
  value: unknown,
  path: string,
): QuestionPoolRevisionMemberView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["memberPosition", "questionRevision", "question"]);
  return {
    memberPosition: decodeNonnegativeInteger(
      field(record, "memberPosition", path),
      `${path}.memberPosition`,
    ),
    questionRevision: decodeQuestionRevisionReference(
      field(record, "questionRevision", path),
      `${path}.questionRevision`,
      true,
    ),
    question: decodeReusableQuestionView(field(record, "question", path), `${path}.question`),
  };
}

/** ASVS 1.5.2 and 2.2.1: accepts only the closed published-Pool list shape. */
export function decodeQuestionPoolLibraryPage(
  value: unknown,
  path = "response",
): QuestionPoolLibraryPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_PAGE_SIZE,
      decodeQuestionPoolLibrarySummary,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}

/** ASVS 1.5.2 and 2.2.3: validates exact pins and immutable member order. */
export function decodeQuestionPoolRevisionView(
  value: unknown,
  path = "response",
): QuestionPoolRevisionView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolRevision", "metadata", "members"]);
  const members = decodeBoundedArray(
    field(record, "members", path),
    `${path}.members`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    decodeQuestionPoolRevisionMemberView,
  );
  if (members.length === 0) {
    throw new DecodeError(`${path}.members`, "a nonempty published Question Pool");
  }
  for (const [index, member] of members.entries()) {
    if (member.memberPosition !== index) {
      throw new DecodeError(
        `${path}.members[${index}].memberPosition`,
        "its zero-based array position",
      );
    }
  }
  return {
    metadata: decodeQuestionPoolMetadata(field(record, "metadata", path), `${path}.metadata`),
    questionPoolRevision: decodeQuestionPoolRevisionReference(
      field(record, "questionPoolRevision", path),
      `${path}.questionPoolRevision`,
    ),
    members,
  };
}
