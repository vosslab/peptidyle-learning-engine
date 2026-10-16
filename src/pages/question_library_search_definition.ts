// Question Library content and API rules supplied to the shared search session.
import type { RecordContent, RecordFact } from "../components/record_list/record_list";
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
  type LibraryObjectQuestionRow,
  type QuestionLibraryPoolRow,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryFilterCounts,
  type LibrarySearchRow,
} from "./library_page_model";

function questionDetails(row: LibraryObjectQuestionRow): ReadonlyArray<RecordFact> {
  const details: RecordFact[] = [
    { kind: "text", label: "Library object", value: "Published Question" },
    { kind: "text", label: "Owner", value: row.ownerAccountId },
    { kind: "text", label: "Question Type", value: questionTypeLabel(row.questionType) },
    { kind: "text", label: "Backend", value: backendLabel(row.backend) },
    { kind: "text", label: "Question License", value: row.license ?? "Not specified" },
    { kind: "text", label: "Tags", value: row.tags.length === 0 ? "None" : row.tags.join(", ") },
    row.authors.length > 0
      ? { kind: "questionAuthors", label: "Authors", authors: row.authors }
      : { kind: "text", label: "Authors", value: "None" },
    { kind: "text", label: "Discipline", value: row.disciplineName },
  ];
  if (row.disciplineIsRetired) {
    details.push({ kind: "text", label: "Discipline status", value: "Retired discipline" });
  }
  details.push({
    kind: "text",
    label: "Bloom",
    value: `Cognitive Process: ${row.bloom?.cognitiveProcess ?? "Not assigned"}; Knowledge Dimension: ${row.bloom?.knowledgeDimension ?? "Not assigned"}`,
  });
  const format = webworkFormatLabel(row.questionFormat);
  if (format !== null) {
    details.push({ kind: "text", label: "Format", value: format });
  }
  details.push({
    kind: "questionId",
    questionTitle: row.title,
    displayId: row.displayId,
  });
  return details;
}

export function questionLibraryContent(row: LibraryObjectQuestionRow): RecordContent {
  const summary = row.description.trim();
  return {
    title: row.title,
    description: summary.length > 0 ? summary : undefined,
    details: questionDetails(row),
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open",
        href: questionLink(row),
        title: `Open ${row.title}`,
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
  details.push({
    kind: "text",
    label: "Bloom",
    value: `Cognitive Process: ${row.bloom?.cognitiveProcess ?? "Not assigned"}; Knowledge Dimension: ${row.bloom?.knowledgeDimension ?? "Not assigned"}`,
  });
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

/** Criteria that can narrow a Library result set independently of result order or display. */
export function hasLibrarySearchCriteria(query: QuestionLibraryBrowseQuery): boolean {
  return (
    query.search.length > 0 ||
    query.kind !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.kind ||
    query.questions !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.questions ||
    (query.ownerAccountId !== null && query.ownerAccountId.length > 0) ||
    query.authorship !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.authorship ||
    query.discipline_uuids.length > 0 ||
    query.subject_uuids.length > 0 ||
    query.topic_uuids.length > 0 ||
    query.subtopic_uuids.length > 0 ||
    query.authorNames.length > 0 ||
    query.backends.length > 0 ||
    query.tags.length > 0 ||
    query.subjects.length > 0 ||
    query.topics.length > 0 ||
    query.bloomCognitiveProcesses.length > 0 ||
    query.bloomKnowledgeDimensions.length > 0 ||
    query.questionTypes.length > 0 ||
    query.capabilities.length > 0 ||
    query.questionLicenses.length > 0
  );
}

export function questionLibrarySearchDefinition(
  repository: QuestionLibraryBrowseRepository,
  initialQuery: QuestionLibraryBrowseQuery = EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  predefined = false,
  fixedKind?: QuestionLibraryBrowseQuery["kind"],
  fixedOwnerAccountId?: string,
): SearchDefinition<QuestionLibraryBrowseQuery, LibrarySearchRow, QuestionLibraryFilterCounts> {
  return {
    initialQuery,
    // ASVS 8.2.2: every UI query transition retains the signed-in owner scope.
    cleanup: (query) =>
      normalizeQuestionLibraryBrowseQuery({
        ...query,
        kind: fixedKind ?? query.kind,
        ownerAccountId: fixedOwnerAccountId ?? query.ownerAccountId,
      }),
    isEligible: (query) => predefined || hasLibrarySearchCriteria(query),
    getText: (query) => query.search,
    setText: (query, search) => ({ ...query, search }),
    rowId: (row) => row.displayId,
    content: librarySearchContent,
    appliedFilters: (query): ReadonlyArray<SearchAppliedFilter<QuestionLibraryBrowseQuery>> => {
      const filters: SearchAppliedFilter<QuestionLibraryBrowseQuery>[] = [];
      if (fixedKind === undefined && query.kind !== "both") {
        filters.push({
          id: "kind",
          label: `Show: ${query.kind === "questions" ? "Questions" : "Question Pools"}`,
          clear: (value) => ({ ...value, kind: "both" }),
        });
      }
      if (query.kind !== "pools" && query.questions === "inNoPool") {
        filters.push({
          id: "questions",
          label: "Questions: in no Pool",
          clear: (value) => ({ ...value, questions: "all" }),
        });
      }
      if (fixedOwnerAccountId === undefined && query.ownerAccountId !== null) {
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
      const multiValueFilters: ReadonlyArray<readonly [keyof QuestionLibraryBrowseQuery, string]> =
        [
          ["authorNames", "Author"],
          ["backends", "Backend"],
          ["tags", "Tag"],
          ["questionTypes", "Question type"],
          ["questionLicenses", "License"],
          ["capabilities", "Capability"],
        ];
      for (const [field, name] of multiValueFilters) {
        const value = query[field];
        if (Array.isArray(value) && value.length > 0) {
          filters.push({
            id: field,
            label: `${name}: ${value.join(", ")}`,
            clear: (current: QuestionLibraryBrowseQuery) => ({ ...current, [field]: [] }),
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
      if (query.discipline_uuids.length > 0)
        filters.push({
          id: "discipline",
          label: "Discipline filter",
          clear: (value) => ({
            ...value,
            discipline_uuids: [],
            subject_uuids: [],
            topic_uuids: [],
            subtopic_uuids: [],
            cross_discipline: false,
          }),
        });
      if (query.subject_uuids.length > 0)
        filters.push({
          id: "classification-subject",
          label: "Classification subject",
          clear: (value) => ({
            ...value,
            subject_uuids: [],
            topic_uuids: [],
            subtopic_uuids: [],
            cross_discipline: false,
          }),
        });
      if (query.topic_uuids.length > 0)
        filters.push({
          id: "classification-topic",
          label: "Classification topic",
          clear: (value) => ({ ...value, topic_uuids: [], subtopic_uuids: [] }),
        });
      if (query.subtopic_uuids.length > 0)
        filters.push({
          id: "classification-subtopic",
          label: "Classification subtopic",
          clear: (value) => ({ ...value, subtopic_uuids: [] }),
        });
      if (query.cross_discipline)
        filters.push({
          id: "cross-discipline",
          label: "Include cross-discipline questions",
          clear: (value) => ({ ...value, cross_discipline: false }),
        });
      if (query.bloomCognitiveProcesses.length > 0)
        filters.push({
          id: "bloom-cognitive-process",
          label: `Bloom cognitive process: ${query.bloomCognitiveProcesses.join(", ")}`,
          clear: (value) => ({ ...value, bloomCognitiveProcesses: [] }),
        });
      if (query.bloomKnowledgeDimensions.length > 0)
        filters.push({
          id: "bloom-knowledge-dimension",
          label: `Bloom knowledge dimension: ${query.bloomKnowledgeDimensions.join(", ")}`,
          clear: (value) => ({ ...value, bloomKnowledgeDimensions: [] }),
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
