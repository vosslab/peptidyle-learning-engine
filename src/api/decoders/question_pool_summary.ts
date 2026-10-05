// Shared strict decoders for published Question Pool metadata and summary rows.

import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolMetadata } from "../../../generated/api/QuestionPoolMetadata";
import type { QuestionType } from "../../../generated/api/QuestionType";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { decodeBloomClassificationView } from "./bloom_classification";
import { decodeAccountId, decodeQuestionId, field, requireOnlyFields } from "./shared";

function decodePoolQuestionType(value: unknown, path: string): QuestionType {
  return decodeStringEnum(value, path, [
    "multipleChoice",
    "multipleAnswer",
    "fillInBlank",
    "multipleFillInBlank",
    "numeric",
    "matching",
    "ordering",
    "hotspot",
  ] as const);
}

function decodePoolBackend(value: unknown, path: string): QuestionBackend {
  return decodeStringEnum(value, path, ["ple", "webwork", "imathas"] as const);
}

function decodePoolLicense(value: unknown, path: string): QuestionLicense {
  return decodeStringEnum(value, path, ["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"] as const);
}

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
  const tags = decodeArray(field(record, "tags", path), `${path}.tags`, (item, itemPath) =>
    decodeQuestionPoolText(item, itemPath, 120),
  );
  if (new Set(tags).size !== tags.length) throw new DecodeError(`${path}.tags`, "unique Pool Tags");
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

/** Strict metadata-only Pool row used by the Pool list and mixed Library page. */
export function decodeQuestionPoolLibrarySummary(
  value: unknown,
  path: string,
): QuestionPoolLibrarySummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionPoolId",
    "ownerAccountId",
    "questionType",
    "backend",
    "license",
    "questionPoolEditNumber",
    "questionPoolMetadataEditNumber",
    "metadata",
    "memberCount",
    "bloom",
  ]);
  return {
    metadata: decodeQuestionPoolMetadata(field(record, "metadata", path), `${path}.metadata`),
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    ownerAccountId: decodeAccountId(
      field(record, "ownerAccountId", path),
      `${path}.ownerAccountId`,
    ),
    questionType: decodePoolQuestionType(
      field(record, "questionType", path),
      `${path}.questionType`,
    ),
    backend: decodePoolBackend(field(record, "backend", path), `${path}.backend`),
    license: decodePoolLicense(field(record, "license", path), `${path}.license`),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
    ),
    questionPoolMetadataEditNumber: decodePositiveInteger(
      field(record, "questionPoolMetadataEditNumber", path),
      `${path}.questionPoolMetadataEditNumber`,
    ),
    memberCount: decodePositiveInteger(field(record, "memberCount", path), `${path}.memberCount`),
    bloom: decodeNullable(
      field(record, "bloom", path),
      `${path}.bloom`,
      decodeBloomClassificationView,
    ),
  };
}

export { decodePoolBackend, decodePoolLicense, decodePoolQuestionType };
