// Closed Course metadata boundary: Subject remains optional, unlike Library metadata.
import type { CourseClassification } from "../../../generated/api/CourseClassification";
import { DecodeError, decodeArray, decodeRecord, decodeString, decodeUuid } from "../decoder";
import { field, requireOnlyFields } from "./shared";

export function decodeCourseClassification(
  value: unknown,
  path = "classification",
): CourseClassification {
  // ASVS 1.5.2, 2.2.1, 2.2.3: validate exact identities and parent presence before use.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "disciplineUuid",
    "subjectUuid",
    "topicUuid",
    "subtopicUuid",
    "tags",
  ]);
  function optionalUuid(name: string): string | null {
    const value = field(record, name, path);
    return value === null ? null : decodeUuid(value, `${path}.${name}`);
  }
  const subjectUuid = optionalUuid("subjectUuid");
  const topicUuid = optionalUuid("topicUuid");
  const subtopicUuid = optionalUuid("subtopicUuid");
  if (
    (topicUuid !== null && subjectUuid === null) ||
    (subtopicUuid !== null && topicUuid === null)
  ) {
    throw new DecodeError(path, "descendants with their selected parents");
  }
  const tags = decodeArray(field(record, "tags", path), `${path}.tags`, (value, tagPath) => {
    const text = decodeString(value, tagPath);
    if (
      text.length === 0 ||
      Array.from(text).length > 120 ||
      text.startsWith(" ") ||
      text.endsWith(" ") ||
      /\p{Cc}/u.test(text)
    ) {
      throw new DecodeError(tagPath, "a trimmed Tag of 1 through 120 characters without controls");
    }
    return text;
  });
  if (new Set(tags).size !== tags.length) throw new DecodeError(`${path}.tags`, "unique Tags");
  return {
    disciplineUuid: decodeUuid(field(record, "disciplineUuid", path), `${path}.disciplineUuid`),
    subjectUuid,
    topicUuid,
    subtopicUuid,
    tags,
  };
}
