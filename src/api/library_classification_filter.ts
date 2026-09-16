// Shared read-only Library classification identity and cascade contract.

import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";

export type LibraryClassificationFilter = Readonly<
  Pick<
    QuestionSearchRequest,
    "discipline_uuid" | "subject_uuid" | "topic_uuid" | "subtopic_uuid" | "cross_discipline"
  >
>;

export const EMPTY_LIBRARY_CLASSIFICATION_FILTER: LibraryClassificationFilter = {
  discipline_uuid: null,
  subject_uuid: null,
  topic_uuid: null,
  subtopic_uuid: null,
  cross_discipline: false,
};

export const LIBRARY_CLASSIFICATION_UUID_FIELDS = [
  "discipline_uuid",
  "subject_uuid",
  "topic_uuid",
  "subtopic_uuid",
] as const;

/** ASVS 2.1.1/2.2.1: client structural checks; the server owns association validation. */
export function libraryClassificationFilter(
  value: LibraryClassificationFilter,
): LibraryClassificationFilter {
  const result: LibraryClassificationFilter = {
    discipline_uuid: value.discipline_uuid,
    subject_uuid: value.subject_uuid,
    topic_uuid: value.topic_uuid,
    subtopic_uuid: value.subtopic_uuid,
    cross_discipline: value.cross_discipline,
  };
  for (const field of LIBRARY_CLASSIFICATION_UUID_FIELDS) {
    const uuid = result[field];
    if (uuid !== null && (typeof uuid !== "string" ||
      !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(uuid))) {
      throw new Error(`Library ${field} must be a UUID`);
    }
  }
  if (typeof result.cross_discipline !== "boolean" ||
    (result.subject_uuid !== null && result.discipline_uuid === null) ||
    (result.topic_uuid !== null && result.subject_uuid === null) ||
    (result.subtopic_uuid !== null && result.topic_uuid === null) ||
    (result.cross_discipline && (result.discipline_uuid === null || result.subject_uuid === null))) {
    throw new Error("Library classification requires its selected parents");
  }
  return result;
}

export function libraryClassificationChange(
  field: (typeof LIBRARY_CLASSIFICATION_UUID_FIELDS)[number],
  uuid: string | null,
): Partial<LibraryClassificationFilter> {
  switch (field) {
    case "discipline_uuid":
      return { ...EMPTY_LIBRARY_CLASSIFICATION_FILTER, discipline_uuid: uuid };
    case "subject_uuid":
      return { subject_uuid: uuid, topic_uuid: null, subtopic_uuid: null, cross_discipline: false };
    case "topic_uuid":
      return { topic_uuid: uuid, subtopic_uuid: null };
    case "subtopic_uuid":
      return { subtopic_uuid: uuid };
  }
}

/** ASVS 1.2.2: encode only allowlisted identities through URLSearchParams. */
export function appendLibraryClassificationParameters(
  parameters: URLSearchParams,
  value: LibraryClassificationFilter,
): void {
  const filter = libraryClassificationFilter(value);
  for (const field of LIBRARY_CLASSIFICATION_UUID_FIELDS) {
    const uuid = filter[field];
    if (uuid !== null) parameters.set(field, uuid);
  }
  if (filter.cross_discipline) parameters.set("cross_discipline", "true");
}

export function parseLibraryClassificationParameters(
  parameters: URLSearchParams,
): LibraryClassificationFilter {
  const cross = parameters.get("cross_discipline");
  if (cross !== null && cross !== "true" && cross !== "false") {
    throw new Error("Library cross_discipline must be boolean");
  }
  return libraryClassificationFilter({
    discipline_uuid: parameters.get("discipline_uuid"),
    subject_uuid: parameters.get("subject_uuid"),
    topic_uuid: parameters.get("topic_uuid"),
    subtopic_uuid: parameters.get("subtopic_uuid"),
    cross_discipline: cross === "true",
  });
}
