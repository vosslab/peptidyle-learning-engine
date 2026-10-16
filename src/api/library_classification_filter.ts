// Shared read-only Library classification identity and cascade contract.

import type { LibraryObjectSearchRequest } from "../../generated/api/LibraryObjectSearchRequest";

export type LibraryClassificationFilter = Readonly<
  Pick<
    LibraryObjectSearchRequest,
    "discipline_uuids" | "subject_uuids" | "topic_uuids" | "subtopic_uuids" | "cross_discipline"
  >
>;

export const EMPTY_LIBRARY_CLASSIFICATION_FILTER: LibraryClassificationFilter = {
  discipline_uuids: [],
  subject_uuids: [],
  topic_uuids: [],
  subtopic_uuids: [],
  cross_discipline: false,
};

export const LIBRARY_CLASSIFICATION_UUID_FIELDS = [
  "discipline_uuids",
  "subject_uuids",
  "topic_uuids",
  "subtopic_uuids",
] as const;

/** ASVS 2.2.1: client structural checks; the server owns association validation. */
export function libraryClassificationFilter(
  value: LibraryClassificationFilter,
): LibraryClassificationFilter {
  const result: LibraryClassificationFilter = {
    discipline_uuids: value.discipline_uuids,
    subject_uuids: value.subject_uuids,
    topic_uuids: value.topic_uuids,
    subtopic_uuids: value.subtopic_uuids,
    cross_discipline: value.cross_discipline,
  };
  for (const field of LIBRARY_CLASSIFICATION_UUID_FIELDS) {
    if (
      !Array.isArray(result[field]) ||
      result[field].length > 32 ||
      result[field].some(
        (uuid) =>
          typeof uuid !== "string" ||
          !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(uuid),
      )
    ) {
      throw new Error(`Library ${field} must contain at most 32 UUIDs`);
    }
  }
  if (
    typeof result.cross_discipline !== "boolean" ||
    (result.subject_uuids.length > 0 && result.discipline_uuids.length === 0) ||
    (result.topic_uuids.length > 0 && result.subject_uuids.length === 0) ||
    (result.subtopic_uuids.length > 0 && result.topic_uuids.length === 0) ||
    (result.cross_discipline &&
      (result.discipline_uuids.length === 0 || result.subject_uuids.length === 0))
  ) {
    throw new Error("Library classification requires its selected parents");
  }
  return {
    ...result,
    discipline_uuids: [
      ...new Set(result.discipline_uuids.map((uuid) => uuid.toLowerCase())),
    ].sort(),
    subject_uuids: [...new Set(result.subject_uuids.map((uuid) => uuid.toLowerCase()))].sort(),
    topic_uuids: [...new Set(result.topic_uuids.map((uuid) => uuid.toLowerCase()))].sort(),
    subtopic_uuids: [...new Set(result.subtopic_uuids.map((uuid) => uuid.toLowerCase()))].sort(),
  };
}

export function libraryClassificationChange(
  field: (typeof LIBRARY_CLASSIFICATION_UUID_FIELDS)[number],
  uuid: string | null,
): Partial<LibraryClassificationFilter> {
  const choice = uuid === null ? [] : [uuid];
  switch (field) {
    case "discipline_uuids":
      return { ...EMPTY_LIBRARY_CLASSIFICATION_FILTER, discipline_uuids: choice };
    case "subject_uuids":
      return {
        subject_uuids: choice,
        topic_uuids: [],
        subtopic_uuids: [],
        cross_discipline: false,
      };
    case "topic_uuids":
      return { topic_uuids: choice, subtopic_uuids: [] };
    case "subtopic_uuids":
      return { subtopic_uuids: choice };
  }
}

/** ASVS 1.2.2: encode only allowlisted identities through URLSearchParams. */
export function appendLibraryClassificationParameters(
  parameters: URLSearchParams,
  value: LibraryClassificationFilter,
): void {
  const filter = libraryClassificationFilter(value);
  for (const field of LIBRARY_CLASSIFICATION_UUID_FIELDS) {
    for (const uuid of filter[field]) parameters.append(field, uuid);
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
    discipline_uuids: parameters.getAll("discipline_uuids"),
    subject_uuids: parameters.getAll("subject_uuids"),
    topic_uuids: parameters.getAll("topic_uuids"),
    subtopic_uuids: parameters.getAll("subtopic_uuids"),
    cross_discipline: cross === "true",
  });
}
