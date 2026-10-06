// Question Library content and API rules supplied to the shared search session.
import type { RecordContent, RecordFact } from "../components/record_list/record_list";
import type { QuestionSearchSort } from "../../generated/api/QuestionSearchSort";
import type {
  SearchAppliedFilter,
  SearchDefinition,
  SearchPage,
} from "../features/search/search_session";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import {
  backendLabel,
  questionLink,
  questionTypeLabel,
  webworkFormatLabel,
} from "./library_page_helpers";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  decodeLibrarySearchPage,
  normalizeQuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRow,
  type QuestionLibraryPoolRow,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryFilterCounts,
  type LibrarySearchRow,
} from "./library_page_model";

function questionDetails(row: QuestionLibraryBrowseRow): ReadonlyArray<RecordFact> {
  const details: RecordFact[] = [
    row.authors.length > 0
      ? { kind: "questionAuthors", label: "Authors", authors: row.authors }
      : { kind: "text", label: "Authors", value: "None" },
    { kind: "text", label: "Discipline", value: row.disciplineName },
  ];
  if (row.disciplineIsRetired) {
    details.push({ kind: "text", label: "Discipline status", value: "Retired discipline" });
  }
  if (row.bloom !== null) {
    details.push({
      kind: "text",
      label: "Bloom",
      value: `${row.bloom.cognitiveProcess} / ${row.bloom.knowledgeDimension}`,
    });
  }
  const format = webworkFormatLabel(row.questionFormat);
  if (format !== null) {
    details.push({ kind: "text", label: "Format", value: format });
  }
  details.push({
    kind: "questionId",
    questionTitle: row.questionTitle,
    displayId: row.displayId,
  });
  return details;
}

export function questionLibraryContent(row: QuestionLibraryBrowseRow): RecordContent {
  const summary = row.summary.trim();
  return {
    title: row.questionTitle,
    description: summary.length > 0 ? summary : undefined,
    details: questionDetails(row),
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open",
        href: questionLink(row),
        title: `Open ${row.questionTitle}`,
      },
    ],
  };
}

function poolContent(row: QuestionLibraryPoolRow): RecordContent {
  const details: RecordFact[] = [
    { kind: "text", label: "Library object", value: "Question Pool" },
    { kind: "text", label: "Owner", value: row.ownerAccountId },
    { kind: "text", label: "Members", value: String(row.memberCount) },
    { kind: "text", label: "Question Type", value: questionTypeLabel(row.questionType) },
    { kind: "text", label: "Backend", value: backendLabel(row.backend) },
    { kind: "text", label: "Calculated Pool License", value: row.license },
    { kind: "text", label: "Discipline", value: row.disciplineName },
    { kind: "text", label: "Tags", value: row.tags.length === 0 ? "None" : row.tags.join(", ") },
  ];
  if (row.disciplineIsRetired)
    details.push({ kind: "text", label: "Discipline status", value: "Retired discipline" });
  if (row.bloom !== null) {
    details.push({
      kind: "text",
      label: "Bloom",
      value: `${row.bloom.cognitiveProcess} / ${row.bloom.knowledgeDimension}`,
    });
  }
  return {
    title: row.title,
    description: row.description,
    details,
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open Question Pool",
        href: `/library/${encodeURIComponent(row.displayId)}`,
        title: `Open ${row.title}`,
      },
    ],
  };
}

export function librarySearchContent(row: LibrarySearchRow): RecordContent {
  return row.kind === "question" ? questionLibraryContent(row) : poolContent(row);
}

export function questionLibrarySearchDefinition(
  repository: QuestionLibraryBrowseRepository,
  initialQuery: QuestionLibraryBrowseQuery = EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
): SearchDefinition<
  QuestionLibraryBrowseQuery,
  LibrarySearchRow,
  QuestionLibraryFilterCounts,
  QuestionSearchSort
> {
  return {
    initialQuery,
    cleanup: normalizeQuestionLibraryBrowseQuery,
    getText: (query) => query.search,
    setText: (query, search) => ({ ...query, search }),
    rowId: (row) => row.displayId,
    content: librarySearchContent,
    sort: {
      label: "Order results",
      options: [
        { value: "titleAscending", label: "Title (A-Z)" },
        { value: "publishedNewest", label: "Recently published" },
      ],
      get: (query) => query.sort,
      set: (query, sort): QuestionLibraryBrowseQuery => ({ ...query, sort }),
    },
    appliedFilters: (query): ReadonlyArray<SearchAppliedFilter<QuestionLibraryBrowseQuery>> => {
      const filters: SearchAppliedFilter<QuestionLibraryBrowseQuery>[] = [];
      if (query.kind !== "both") {
        filters.push({
          id: "kind",
          label: `Show: ${query.kind === "questions" ? "Questions" : "Question Pools"}`,
          clear: (value) => ({ ...value, kind: "both" }),
        });
      }
      if (query.kind !== "pools" && query.membership === "noPool") {
        filters.push({
          id: "membership",
          label: "Question membership: Questions in no Pool",
          clear: (value) => ({ ...value, membership: "all" }),
        });
      }
      if (query.ownerAccountId !== null) {
        filters.push({
          id: "owner",
          label: `Owner: ${query.ownerAccountId}`,
          clear: (value) => ({ ...value, ownerAccountId: null }),
        });
      }
      if (query.search)
        filters.push({
          id: "search",
          label: `Search: ${query.search}`,
          clear: (value: QuestionLibraryBrowseQuery) => ({ ...value, search: "" }),
        });
      const scalarFilters: ReadonlyArray<readonly [keyof QuestionLibraryBrowseQuery, string]> = [
        ["authorName", "Author"],
        ["backend", "Backend"],
        ["tag", "Tag"],
        ["questionType", "Question type"],
        ["questionLicense", "License"],
        ["capability", "Capability"],
      ];
      for (const [field, name] of scalarFilters) {
        const value = query[field];
        if (typeof value === "string" && value) {
          filters.push({
            id: field,
            label: `${name}: ${value}`,
            clear: (current: QuestionLibraryBrowseQuery) => ({ ...current, [field]: null }),
          });
        }
      }
      if (query.subjects.length > 0)
        filters.push({
          id: "subjects",
          label: `Subject: ${query.subjects.join(", ")}`,
          clear: (value: QuestionLibraryBrowseQuery) => ({ ...value, subjects: [], topics: [] }),
        });
      if (query.topics.length > 0)
        filters.push({
          id: "topics",
          label: `Topic: ${query.topics.join(", ")}`,
          clear: (value: QuestionLibraryBrowseQuery) => ({ ...value, topics: [] }),
        });
      if (query.discipline_uuid !== null)
        filters.push({
          id: "discipline",
          label: "Discipline filter",
          clear: (value) => ({
            ...value,
            discipline_uuid: null,
            subject_uuid: null,
            topic_uuid: null,
            subtopic_uuid: null,
            cross_discipline: false,
          }),
        });
      if (query.subject_uuid !== null)
        filters.push({
          id: "classification-subject",
          label: "Classification subject",
          clear: (value) => ({
            ...value,
            subject_uuid: null,
            topic_uuid: null,
            subtopic_uuid: null,
            cross_discipline: false,
          }),
        });
      if (query.topic_uuid !== null)
        filters.push({
          id: "classification-topic",
          label: "Classification topic",
          clear: (value) => ({ ...value, topic_uuid: null, subtopic_uuid: null }),
        });
      if (query.subtopic_uuid !== null)
        filters.push({
          id: "classification-subtopic",
          label: "Classification subtopic",
          clear: (value) => ({ ...value, subtopic_uuid: null }),
        });
      if (query.cross_discipline)
        filters.push({
          id: "cross-discipline",
          label: "Include cross-discipline questions",
          clear: (value) => ({ ...value, cross_discipline: false }),
        });
      if (query.bloomCognitiveProcess !== null)
        filters.push({
          id: "bloom-cognitive-process",
          label: `Bloom cognitive process: ${query.bloomCognitiveProcess}`,
          clear: (value) => ({ ...value, bloomCognitiveProcess: null }),
        });
      if (query.bloomKnowledgeDimension !== null)
        filters.push({
          id: "bloom-knowledge-dimension",
          label: `Bloom knowledge dimension: ${query.bloomKnowledgeDimension}`,
          clear: (value) => ({ ...value, bloomKnowledgeDimension: null }),
        });
      return filters;
    },
    selection: { maximum: MAX_BULK_QUESTION_METADATA_ITEMS },
    fetchPage: async (
      query,
      cursor,
      pageSize,
    ): Promise<SearchPage<LibrarySearchRow, QuestionLibraryFilterCounts>> => {
      // ASVS 2.2.1: validate transport data before supplying rows to the shared UI.
      const page = decodeLibrarySearchPage(await repository.search(query, cursor, pageSize));
      return {
        items: page.items,
        nextCursor: page.nextCursor,
        filterCounts: { aggregates: page.aggregates, facetTruncation: page.facetTruncation },
      };
    },
  };
}
