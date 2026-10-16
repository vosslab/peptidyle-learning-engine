import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import type { LibraryObjectSearchSort } from "../../generated/api/LibraryObjectSearchSort";
import type { ContentClassificationClient } from "../api/content_classification";
import type {
  RecordFilterChoices,
  RecordListQuery,
  RecordSort,
} from "../components/record_list/record_fields";
import type { LibraryRecordSortKey } from "./library_search_fields";
import type {
  QuestionLibraryBrowseQuery,
  QuestionLibraryBrowseRepository,
} from "./library_page_model";

const libraryOrders: ReadonlyArray<{
  readonly order: LibraryObjectSearchSort;
  readonly sort: RecordSort<LibraryRecordSortKey>;
}> = [
  { order: "titleAscending", sort: { key: "title", direction: "ascending" } },
  { order: "titleDescending", sort: { key: "title", direction: "descending" } },
  { order: "kindAscending", sort: { key: "kind", direction: "ascending" } },
  { order: "kindDescending", sort: { key: "kind", direction: "descending" } },
  { order: "disciplineAscending", sort: { key: "discipline", direction: "ascending" } },
  { order: "disciplineDescending", sort: { key: "discipline", direction: "descending" } },
  { order: "questionTypeAscending", sort: { key: "questionType", direction: "ascending" } },
  { order: "questionTypeDescending", sort: { key: "questionType", direction: "descending" } },
  { order: "publishedNewest", sort: { key: "published", direction: "descending" } },
];

function values(
  query: QuestionLibraryBrowseQuery,
): ReadonlyMap<LibraryFacetField, ReadonlyArray<string>> {
  return new Map<LibraryFacetField, ReadonlyArray<string>>([
    ["kind", query.kind === "both" ? [] : [query.kind]],
    ["disciplineUuids", query.discipline_uuids],
    ["subjectUuids", query.subject_uuids],
    ["topicUuids", query.topic_uuids],
    ["subtopicUuids", query.subtopic_uuids],
    ["questionTypes", query.questionTypes],
    ["backends", query.backends],
    ["tags", query.tags],
    ["authorNames", query.authorNames],
    ["capabilities", query.capabilities],
    ["questionLicenses", query.questionLicenses],
    ["bloomCognitiveProcesses", query.bloomCognitiveProcesses],
    ["bloomKnowledgeDimensions", query.bloomKnowledgeDimensions],
  ]);
}

function hasParent(query: QuestionLibraryBrowseQuery, field: LibraryFacetField): boolean {
  if (field === "subjectUuids") return query.discipline_uuids.length > 0;
  if (field === "topicUuids") return query.subject_uuids.length > 0;
  if (field === "subtopicUuids") return query.topic_uuids.length > 0;
  return true;
}

function prerequisite(field: LibraryFacetField): string {
  if (field === "subjectUuids") return "Choose a Discipline first.";
  if (field === "topicUuids") return "Choose a Subject first.";
  return "Choose a Topic first.";
}

async function retainChildren(
  query: QuestionLibraryBrowseQuery,
  field: LibraryFacetField,
  selected: ReadonlyArray<string>,
  client: ContentClassificationClient,
): Promise<QuestionLibraryBrowseQuery> {
  if (field !== "disciplineUuids" && field !== "subjectUuids" && field !== "topicUuids")
    return query;
  const children =
    field === "disciplineUuids"
      ? query.subject_uuids
      : field === "subjectUuids"
        ? query.topic_uuids
        : query.subtopic_uuids;
  const parents = selected;
  const load =
    field === "disciplineUuids"
      ? client.listSubjects
      : field === "subjectUuids"
        ? client.listTopics
        : client.listSubtopics;
  const allowed = new Set(
    children.length === 0
      ? []
      : (await Promise.all(parents.map((uuid) => load(uuid)))).flat().map((item) => item.uuid),
  );
  if (field === "disciplineUuids") {
    const subject_uuids = query.subject_uuids.filter((uuid) => allowed.has(uuid));
    const next = {
      ...query,
      subject_uuids,
      cross_discipline: subject_uuids.length > 0 && query.cross_discipline,
    };
    return retainChildren(next, "subjectUuids", subject_uuids, client);
  }
  if (field === "subjectUuids") {
    const topic_uuids = query.topic_uuids.filter((uuid) => allowed.has(uuid));
    return retainChildren({ ...query, topic_uuids }, "topicUuids", topic_uuids, client);
  }
  return { ...query, subtopic_uuids: query.subtopic_uuids.filter((uuid) => allowed.has(uuid)) };
}

/** The Library page owns the full query; RecordList only presents these values and choices. */
export function libraryRecordQuery(props: {
  readonly query: () => QuestionLibraryBrowseQuery;
  readonly apply: (
    query:
      | QuestionLibraryBrowseQuery
      | ((current: QuestionLibraryBrowseQuery) => QuestionLibraryBrowseQuery | undefined),
  ) => Promise<unknown>;
  /** Shared search revision for accepted full-query replacements. */
  readonly replacementRevision: () => number;
  readonly repository: QuestionLibraryBrowseRepository;
  readonly classificationClient: ContentClassificationClient;
  readonly onError?: (message: string | null) => void;
}): RecordListQuery<LibraryRecordSortKey, LibraryFacetField> {
  const hierarchyFields = [
    "disciplineUuids",
    "subjectUuids",
    "topicUuids",
    "subtopicUuids",
  ] as const;
  const hierarchyRevisions = [0, 0, 0, 0];
  const pending: Array<() => Promise<void>> = [];
  let running = false;

  function enqueue(operation: (isCurrent: () => boolean) => Promise<void>): void {
    const replacementRevision = props.replacementRevision();
    const isCurrent = (): boolean => replacementRevision === props.replacementRevision();
    pending.push(async (): Promise<void> => {
      if (!isCurrent()) return;
      try {
        await operation(isCurrent);
      } catch (error) {
        if (isCurrent()) throw error;
      }
    });
    if (running) return;
    running = true;
    void (async (): Promise<void> => {
      while (pending.length > 0) {
        try {
          await pending.shift()!();
        } catch {
          props.onError?.("Could not update library filters. Try again.");
        }
      }
      running = false;
    })();
  }

  function sameHierarchy(
    left: QuestionLibraryBrowseQuery,
    right: QuestionLibraryBrowseQuery,
  ): boolean {
    return (
      left.discipline_uuids.join("\0") === right.discipline_uuids.join("\0") &&
      left.subject_uuids.join("\0") === right.subject_uuids.join("\0") &&
      left.topic_uuids.join("\0") === right.topic_uuids.join("\0") &&
      left.subtopic_uuids.join("\0") === right.subtopic_uuids.join("\0") &&
      left.cross_discipline === right.cross_discipline
    );
  }

  function change(field: LibraryFacetField, selected: ReadonlyArray<string>): void {
    props.onError?.(null);
    const hierarchyIndex = hierarchyFields.indexOf(field as (typeof hierarchyFields)[number]);
    if (hierarchyIndex >= 0)
      for (let index = hierarchyIndex; index < hierarchyRevisions.length; index++)
        hierarchyRevisions[index] = (hierarchyRevisions[index] ?? 0) + 1;
    const changeRevision = hierarchyIndex >= 0 ? hierarchyRevisions[hierarchyIndex] : 0;
    enqueue(async (isCurrent): Promise<void> => {
      if (hierarchyIndex >= 0 && changeRevision !== hierarchyRevisions[hierarchyIndex]) return;
      const original = props.query();
      if (!hasParent(original, field) && selected.length > 0) return;
      let next: QuestionLibraryBrowseQuery;
      switch (field) {
        case "kind":
          next = {
            ...original,
            kind:
              selected.length === 1 && selected[0] === "questions"
                ? "questions"
                : selected.length === 1 && selected[0] === "pools"
                  ? "pools"
                  : "both",
          };
          break;
        case "disciplineUuids":
          next = { ...original, discipline_uuids: [...selected] };
          break;
        case "subjectUuids":
          next = {
            ...original,
            subject_uuids: [...selected],
            cross_discipline: selected.length > 0 && original.cross_discipline,
          };
          break;
        case "topicUuids":
          next = { ...original, topic_uuids: [...selected] };
          break;
        case "subtopicUuids":
          next = { ...original, subtopic_uuids: [...selected] };
          break;
        case "questionTypes":
          next = { ...original, questionTypes: selected };
          break;
        case "backends":
          next = { ...original, backends: selected };
          break;
        case "tags":
          next = { ...original, tags: selected };
          break;
        case "authorNames":
          next = { ...original, authorNames: selected };
          break;
        case "capabilities":
          next = { ...original, capabilities: selected };
          break;
        case "questionLicenses":
          next = { ...original, questionLicenses: selected };
          break;
        case "bloomCognitiveProcesses":
          next = {
            ...original,
            bloomCognitiveProcesses:
              selected as QuestionLibraryBrowseQuery["bloomCognitiveProcesses"],
          };
          break;
        case "bloomKnowledgeDimensions":
          next = {
            ...original,
            bloomKnowledgeDimensions:
              selected as QuestionLibraryBrowseQuery["bloomKnowledgeDimensions"],
          };
          break;
        default:
          return;
      }
      let retained: QuestionLibraryBrowseQuery;
      try {
        retained = await retainChildren(next, field, selected, props.classificationClient);
      } catch {
        if (
          isCurrent() &&
          (hierarchyIndex < 0 || changeRevision === hierarchyRevisions[hierarchyIndex])
        )
          props.onError?.("Could not update classification filters. Try again.");
        return;
      }
      if (!isCurrent()) return;
      if (hierarchyIndex >= 0 && changeRevision !== hierarchyRevisions[hierarchyIndex]) return;
      if (hierarchyIndex >= 0 && !sameHierarchy(original, props.query())) return;
      await props.apply((latest) => {
        if (!isCurrent()) return undefined;
        if (hierarchyIndex >= 0 && changeRevision !== hierarchyRevisions[hierarchyIndex])
          return undefined;
        if (hierarchyIndex >= 0 && !sameHierarchy(original, latest)) return undefined;
        return {
          ...latest,
          ...(hierarchyIndex >= 0
            ? {
                discipline_uuids: retained.discipline_uuids,
                subject_uuids: retained.subject_uuids,
                topic_uuids: retained.topic_uuids,
                subtopic_uuids: retained.subtopic_uuids,
                cross_discipline: retained.cross_discipline,
              }
            : {}),
          kind: field === "kind" ? retained.kind : latest.kind,
          questionTypes: field === "questionTypes" ? retained.questionTypes : latest.questionTypes,
          backends: field === "backends" ? retained.backends : latest.backends,
          tags: field === "tags" ? retained.tags : latest.tags,
          authorNames: field === "authorNames" ? retained.authorNames : latest.authorNames,
          capabilities: field === "capabilities" ? retained.capabilities : latest.capabilities,
          questionLicenses:
            field === "questionLicenses" ? retained.questionLicenses : latest.questionLicenses,
          bloomCognitiveProcesses:
            field === "bloomCognitiveProcesses"
              ? retained.bloomCognitiveProcesses
              : latest.bloomCognitiveProcesses,
          bloomKnowledgeDimensions:
            field === "bloomKnowledgeDimensions"
              ? retained.bloomKnowledgeDimensions
              : latest.bloomKnowledgeDimensions,
        };
      });
    });
  }

  return {
    kind: "paged",
    sort: {
      get value(): RecordSort<LibraryRecordSortKey> | null {
        return libraryOrders.find((entry) => entry.order === props.query().sort)?.sort ?? null;
      },
      onChange: (sort): void => {
        const order = libraryOrders.find(
          (entry) => entry.sort.key === sort.key && entry.sort.direction === sort.direction,
        )?.order;
        if (order === undefined || order === props.query().sort) return;
        props.onError?.(null);
        enqueue(async (isCurrent): Promise<void> => {
          await props.apply((current) => {
            if (!isCurrent() || order === current.sort) return undefined;
            return { ...current, sort: order };
          });
        });
      },
      choices: [
        { label: "Recently published", sort: { key: "published", direction: "descending" } },
      ],
    },
    filters: {
      get values(): ReadonlyMap<LibraryFacetField, ReadonlyArray<string>> {
        return values(props.query());
      },
      onChange: change,
      onClear: (): void => {
        for (let index = 0; index < hierarchyRevisions.length; index++)
          hierarchyRevisions[index] = (hierarchyRevisions[index] ?? 0) + 1;
        props.onError?.(null);
        enqueue(async (isCurrent): Promise<void> => {
          await props.apply((current) =>
            isCurrent()
              ? {
                  ...current,
                  kind: "both",
                  discipline_uuids: [],
                  subject_uuids: [],
                  topic_uuids: [],
                  subtopic_uuids: [],
                  cross_discipline: false,
                  questionTypes: [],
                  backends: [],
                  tags: [],
                  authorNames: [],
                  capabilities: [],
                  subjects: [],
                  topics: [],
                  questionLicenses: [],
                  bloomCognitiveProcesses: [],
                  bloomKnowledgeDimensions: [],
                }
              : undefined,
          );
        });
      },
      choices: async (field, search, cursor, signal): Promise<RecordFilterChoices> => {
        const current = props.query();
        if (!hasParent(current, field))
          return {
            choices: [],
            selected: [],
            nextCursor: null,
            unavailableReason: prerequisite(field),
          };
        const page = await props.repository.listFacetChoices(field, current, search, cursor);
        if (signal.aborted) return { choices: [], selected: [], nextCursor: null };
        return page;
      },
    },
  };
}
