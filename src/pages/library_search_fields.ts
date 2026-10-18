import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import type { RecordField } from "../components/data_sheet/record_fields";
import type { LibrarySearchRow } from "./library_page_model";
import { questionTypeLabel } from "./library_page_helpers";

export type LibraryRecordSortKey = "title" | "kind" | "discipline" | "questionType" | "published";

/** Shared first columns for Library pages and embedded Library pickers. */
export const librarySearchFields: ReadonlyArray<
  RecordField<LibrarySearchRow, LibraryRecordSortKey, LibraryFacetField>
> = [
  {
    id: "title",
    label: "Title",
    role: "identity",
    sortKey: "title",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "18rem" },
  },
  {
    id: "kind",
    label: "Library object",
    sortKey: "kind",
    filterKey: "kind",
    value: (row) => (row.kind === "question" ? "Published Question" : "Question Pool"),
    defaultColumn: { priority: 1, width: "10rem" },
  },
  {
    id: "discipline",
    label: "Discipline",
    sortKey: "discipline",
    filterKey: "disciplineUuids",
    value: (row) => row.classification.disciplineName,
    defaultColumn: { priority: 2, width: "10rem" },
  },
  {
    id: "type",
    label: "Question Type",
    sortKey: "questionType",
    filterKey: "questionTypes",
    value: (row) => questionTypeLabel(row.questionType),
    defaultColumn: { priority: 3, width: "10rem" },
  },
  {
    id: "subject",
    label: "Subject",
    filterKey: "subjectUuids",
    value: (row) => row.classification.subjectName,
    defaultColumn: { priority: 4, width: "10rem" },
  },
  {
    id: "topic",
    label: "Topic",
    filterKey: "topicUuids",
    value: (row) => row.classification.topicName,
    defaultColumn: { priority: 5, width: "10rem", visible: false },
  },
  {
    id: "subtopic",
    label: "Subtopic",
    filterKey: "subtopicUuids",
    value: (row) => row.classification.subtopicName,
    defaultColumn: { priority: 6, width: "10rem", visible: false },
  },
  {
    id: "backend",
    label: "Backend",
    filterKey: "backends",
    value: (row) => row.backend,
    defaultColumn: { priority: 7, width: "8rem", visible: false },
  },
  {
    id: "tags",
    label: "Tags",
    filterKey: "tags",
    value: (row) => row.tags,
    defaultColumn: { priority: 8, width: "10rem", visible: false },
  },
  {
    id: "authors",
    label: "Authors",
    filterKey: "authorNames",
    value: (row) =>
      row.kind === "question" ? row.authors.map((author) => author.displayName) : [],
    defaultColumn: { priority: 9, width: "10rem", visible: false },
  },
  {
    id: "capability",
    label: "Capability",
    filterKey: "capabilities",
    value: (row) => (row.kind === "question" ? row.capabilities : []),
    defaultColumn: { priority: 10, width: "10rem", visible: false },
  },
  {
    id: "license",
    label: "License",
    filterKey: "questionLicenses",
    value: (row) => row.license,
    defaultColumn: { priority: 11, width: "9rem", visible: false },
  },
  {
    id: "bloomProcess",
    label: "Bloom Cognitive Process",
    filterKey: "bloomCognitiveProcesses",
    value: (row) => row.bloom?.cognitiveProcess ?? null,
    defaultColumn: { priority: 12, width: "10rem", visible: false },
  },
  {
    id: "bloomKnowledge",
    label: "Bloom Knowledge Dimension",
    filterKey: "bloomKnowledgeDimensions",
    value: (row) => row.bloom?.knowledgeDimension ?? null,
    defaultColumn: { priority: 13, width: "10rem", visible: false },
  },
];

/** A fixed collection can reveal/sort Kind, but cannot change its scope. */
export const fixedKindLibrarySearchFields: typeof librarySearchFields = librarySearchFields.map(
  (field) =>
    field.id === "kind"
      ? {
          ...field,
          filterKey: undefined,
          defaultColumn: { ...field.defaultColumn, visible: false },
        }
      : field,
);
