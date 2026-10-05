// question_picker_model.ts - reusable, answer-free Question Picker contracts.

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionType } from "../../../generated/api/QuestionType";
import type { RecordContent } from "../../components/record_list/record_list";
import type { SearchDefinition, SearchPage } from "../search/search_session";
import type { QuestionDetailsPromptView } from "../../../generated/api/QuestionDetailsPromptView";
import type { QuestionResponsePreview } from "../../../generated/api/QuestionResponsePreview";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import { validateCanonicalQuestionIdSyntax } from "../../question_id";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { BlueprintAssessmentSource } from "../../../generated/api/BlueprintAssessmentSource";
import type { QuestionFormat } from "../../../generated/api/QuestionFormat";
import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  decodeQuestionLibraryBrowsePage,
  normalizeQuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowsePage,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRow,
} from "../../pages/library_page_model";

/** The largest selection any current D2 consumer can request. */
export const MAX_QUESTION_PICKER_SELECTION_CAP = 1024;

/** Stable browser IDs for one retained Course Instance Assessment. */
export interface RetainedAssessmentId {
  readonly courseInstanceId: string;
  readonly assessmentId: string;
}

/**
 * The picker exposes each selection source through the same answer-free D1 rows.
 */
export type QuestionPickerSource =
  | { readonly kind: "library"; readonly label: string }
  | { readonly kind: "sharedLibrary"; readonly label: string }
  | { readonly kind: "mine"; readonly label: string }
  | {
      readonly kind: "retainedAssessment";
      readonly label: string;
      readonly retainedAssessment: RetainedAssessmentId;
    }
  | {
      readonly kind: "blueprintCourseAssessment";
      /** Exact immutable Blueprint Revision provenance for this reusable content. */
      readonly source: BlueprintAssessmentSource;
      readonly label: string;
    };

export type QuestionPickerSelectionMode = "none" | "one" | "many";

/** One ordered question selected from an answer-free D1 Question Search result. */
export interface QuestionPickerSelectedQuestion {
  readonly questionId: string;
  readonly row: QuestionLibraryBrowseRow;
}

/** Answer-free Question Details kept for inspection before an Assessment add. */
export interface QuestionPickerInspectionView {
  readonly questionTitle: string;
  readonly questionId: string;
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly backend: QuestionDetails["summary"]["backend"];
  readonly prompt: QuestionDetailsPromptView;
  readonly responsePreview: QuestionResponsePreview | null;
}

/**
 * Opens inspection of one result without selecting, removing, or reordering it.
 * The returned selection is the selection the caller already had.
 */
export function inspectQuestionPickerRow(
  selection: QuestionPickerSelection,
  row: QuestionLibraryBrowseRow,
): {
  readonly selection: QuestionPickerSelection;
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
} {
  return {
    selection,
    publishedQuestionRevisionTuple: row.publishedQuestionRevisionTuple,
  };
}

/** Keeps the answer-free Question Details fields an Instructor can inspect. */
export function questionPickerInspectionView(
  details: QuestionDetails,
): QuestionPickerInspectionView {
  return {
    questionTitle: details.summary.metadata.questionTitle,
    questionId: details.summary.questionId,
    publishedQuestionRevisionTuple: details.summary.publishedQuestionRevisionTuple,
    backend: details.summary.backend,
    prompt: details.prompt,
    responsePreview: details.responsePreview,
  };
}

/** The one public completion value consumed by Library and assessment parents. */
export interface QuestionPickerSelection {
  readonly questionIds: ReadonlyArray<string>;
  readonly questions: ReadonlyArray<QuestionPickerSelectedQuestion>;
}

/** Shared discovery page sizes. The picker forwards the same choice the Library uses. */
export type QuestionPickerPageSize = 50 | 100 | 250;

export interface QuestionPickerSearchRequest {
  readonly source: QuestionPickerSource;
  readonly query: QuestionLibraryBrowseQuery;
  readonly cursor: string | null;
  /** Omitted requests use the Library default of 50. */
  readonly pageSize?: QuestionPickerPageSize;
}

/**
 * Source adapters remain parent-owned because D2 server routes arrive after the
 * picker shell. Each adapter returns the same hostile transport shape as D1.
 */
export interface QuestionPickerSourceRepository {
  readonly search: (request: QuestionPickerSearchRequest) => Promise<unknown>;
}

/** One picker request keeps its content source beside the common Library query. */
export interface QuestionPickerQuery {
  readonly source: QuestionPickerSource;
  readonly libraryQuery: QuestionLibraryBrowseQuery;
}

/** Pool-member contexts constrain the shared Library request with authoritative Pool facts. */
export interface QuestionPickerEligibility {
  readonly disciplineUuid: string;
  readonly subjectUuid: string;
  readonly questionType: QuestionType;
  readonly backend: QuestionBackend;
  /** A source-bound Pool never offers its exact fixed starting Question again. */
  readonly excludedQuestionId?: string;
}

function selectionLimit(mode: QuestionPickerSelectionMode, maximumSelection: number): number {
  if (mode === "none") return 0;
  if (
    !Number.isSafeInteger(maximumSelection) ||
    maximumSelection < 1 ||
    maximumSelection > MAX_QUESTION_PICKER_SELECTION_CAP
  ) {
    throw new Error(
      `Choose a selection maximum from 1 through ${MAX_QUESTION_PICKER_SELECTION_CAP}.`,
    );
  }
  return mode === "one" ? 1 : maximumSelection;
}

function canonicalQuestionId(value: string): string {
  const questionId = validateCanonicalQuestionIdSyntax(value);
  if (questionId === null)
    throw new Error("A selected question must have a canonical Question ID.");
  return questionId;
}

function rowsWithCanonicalUniqueQuestionIds(
  rows: ReadonlyArray<QuestionLibraryBrowseRow>,
): ReadonlyArray<QuestionLibraryBrowseRow> {
  const known = new Set<string>();
  const unique: QuestionLibraryBrowseRow[] = [];
  for (const row of rows) {
    const questionId = canonicalQuestionId(row.displayId);
    if (known.has(questionId)) continue;
    known.add(questionId);
    unique.push(row);
  }
  return unique;
}

/** Picker choices create new reusable work, unlike historical Library reads. */
function isCurrentProductionPickerRow(row: QuestionLibraryBrowseRow): boolean {
  return row.questionFormat !== "imathas";
}

/** Builds the public, ordered selection while retaining only D1-safe row metadata. */
export function questionPickerSelection(
  mode: QuestionPickerSelectionMode,
  maximumSelection: number,
  rows: ReadonlyArray<QuestionLibraryBrowseRow>,
): QuestionPickerSelection {
  const uniqueRows = rowsWithCanonicalUniqueQuestionIds(rows);
  const maximum = selectionLimit(mode, maximumSelection);
  if (uniqueRows.length > maximum) {
    throw new Error(`Choose at most ${maximum} question${maximum === 1 ? "" : "s"} here.`);
  }
  const questions = uniqueRows.map((row) => ({
    questionId: canonicalQuestionId(row.displayId),
    row,
  }));
  const questionIds = questions.map((question) => question.questionId);
  return { questionIds, questions };
}

/** Adds or removes one D1-safe row while preserving ordered selection. */
export function toggleQuestionPickerSelection(
  mode: QuestionPickerSelectionMode,
  maximumSelection: number,
  selection: QuestionPickerSelection,
  row: QuestionLibraryBrowseRow,
  selected: boolean,
): QuestionPickerSelection {
  const questionId = canonicalQuestionId(row.displayId);
  const withoutCurrent = selection.questions.filter(
    (question) => question.questionId !== questionId,
  );
  if (!selected)
    return questionPickerSelection(
      mode,
      maximumSelection,
      withoutCurrent.map((question) => question.row),
    );
  if (mode === "one") return questionPickerSelection(mode, maximumSelection, [row]);
  return questionPickerSelection(mode, maximumSelection, [
    ...withoutCurrent.map((question) => question.row),
    row,
  ]);
}

/** Reorders the current tray without changing its public question membership. */
export function moveQuestionPickerSelection(
  mode: QuestionPickerSelectionMode,
  maximumSelection: number,
  selection: QuestionPickerSelection,
  index: number,
  direction: -1 | 1,
): QuestionPickerSelection {
  const nextIndex = index + direction;
  if (index < 0 || nextIndex < 0 || nextIndex >= selection.questions.length) return selection;
  const questions = [...selection.questions];
  const current = questions[index];
  const adjacent = questions[nextIndex];
  if (current === undefined || adjacent === undefined) return selection;
  questions[index] = adjacent;
  questions[nextIndex] = current;
  return questionPickerSelection(
    mode,
    maximumSelection,
    questions.map((question) => question.row),
  );
}

/** Adapts available Question Library searches for the picker source choices. */
export function questionLibraryPickerRepository(
  library: QuestionLibraryBrowseRepository,
  myQuestions: QuestionLibraryBrowseRepository,
): QuestionPickerSourceRepository {
  return {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      if (request.source.kind === "library" || request.source.kind === "sharedLibrary") {
        return await library.search(
          { ...request.query, kind: "questions", membership: "all" },
          request.cursor,
          request.pageSize,
        );
      }
      if (request.source.kind === "mine") {
        return await myQuestions.search(
          { ...request.query, kind: "questions", membership: "all" },
          request.cursor,
          request.pageSize,
        );
      }
      {
        throw new Error("This picker composition has not connected that source yet.");
      }
    },
  };
}

/** Current available Question Library choices for Instructor picker compositions. */
export function questionLibraryPickerSources(
  includeMyQuestions: boolean,
): ReadonlyArray<QuestionPickerSource> {
  return [
    { kind: "library", label: "Question Library" },
    ...(includeMyQuestions ? ([{ kind: "mine", label: "My Questions" }] as const) : []),
  ];
}

function reusableQuestionLibraryRow(item: {
  readonly summary: {
    readonly questionId: string;
    readonly publishedQuestionRevisionTuple: QuestionLibraryBrowseRow["publishedQuestionRevisionTuple"];
    readonly questionFormat: QuestionFormat;
    readonly metadata: {
      readonly questionTitle: string;
      readonly questionDescription: string;
      readonly questionLicense: "CC0-1.0" | "CC-BY-4.0" | "CC-BY-SA-4.0" | null;
    };
    readonly bloom: QuestionLibraryBrowseRow["bloom"];
    readonly authorship: { readonly authors: ReadonlyArray<{ readonly displayName: string }> };
    readonly capabilities: ReadonlyArray<string>;
  };
  readonly disciplineName: string;
  readonly disciplineIsRetired: boolean;
  readonly evidence: QuestionLibraryBrowseRow["evidence"];
}): QuestionLibraryBrowseRow {
  const summary = item.summary;
  return {
    kind: "question",
    displayId: summary.questionId,
    publishedQuestionRevisionTuple: summary.publishedQuestionRevisionTuple,
    questionTitle: summary.metadata.questionTitle,
    summary: summary.metadata.questionDescription,
    bloom: summary.bloom,
    disciplineName: item.disciplineName,
    disciplineIsRetired: item.disciplineIsRetired,
    questionFormat: summary.questionFormat,
    authors: summary.authorship.authors.map((author) => ({
      displayName: author.displayName,
      accountId: null,
    })),
    capabilities: summary.capabilities,
    questionLicense: summary.metadata.questionLicense,
    evidence: item.evidence,
  };
}

function selectedBlueprintAssessment(
  source: BlueprintAssessmentSource,
  blueprintRevision: Awaited<ReturnType<BlueprintCourseClient["getBlueprintRevision"]>>,
): Awaited<
  ReturnType<BlueprintCourseClient["getBlueprintRevision"]>
>["modules"][number]["assessments"][number] {
  if (
    blueprintRevision.blueprintRevisionTuple.blueprintCourseId !==
      source.blueprint_revision_tuple.blueprintCourseId ||
    blueprintRevision.blueprintRevisionTuple.revisionNumber !==
      source.blueprint_revision_tuple.revisionNumber
  ) {
    throw new Error(
      "The selected Blueprint Revision did not resolve. Choose an Assessment from the Course's Blueprint Revision.",
    );
  }
  for (const module of blueprintRevision.modules) {
    const assessment = module.assessments.find(
      (item) => item.blueprint_assessment_id === source.blueprint_assessment_id,
    );
    if (assessment !== undefined) return assessment;
  }
  throw new Error(
    "The selected Blueprint Assessment is not available in this exact Blueprint Revision.",
  );
}

function contentRows(
  content: BlueprintAssessmentContentView,
): ReadonlyArray<QuestionLibraryBrowseRow> {
  return content.entries.flatMap((entry) =>
    entry.kind === "fixed" ? [reusableQuestionLibraryRow(entry.question.question_library)] : [],
  );
}

function sourceRowsMatchQuery(
  rows: ReadonlyArray<QuestionLibraryBrowseRow>,
  query: QuestionLibraryBrowseQuery,
): QuestionLibraryBrowseRow[] {
  const needle = query.search.trim().toLocaleLowerCase();
  return needle === ""
    ? [...rows]
    : rows.filter((row) =>
        `${row.questionTitle}\n${row.summary}`.toLocaleLowerCase().includes(needle),
      );
}

/** Connects Blueprint Assessment fixed Questions to the shared picker definition. */
export function blueprintCourseQuestionPickerRepository(
  client: BlueprintCourseClient,
): QuestionPickerSourceRepository {
  return {
    async search(request): Promise<unknown> {
      if (request.source.kind !== "blueprintCourseAssessment") {
        throw new Error("Choose a Blueprint Course source for this picker composition.");
      }
      const cursor = request.cursor === null ? 0 : Number(request.cursor);
      if (!Number.isSafeInteger(cursor) || cursor < 0) {
        throw new Error("Use the picker continuation supplied by this source.");
      }
      const pageSize = request.pageSize ?? 50;
      const source = request.source.source;
      const revision = await client.getBlueprintRevision(
        source.blueprint_revision_tuple.blueprintCourseId,
        source.blueprint_revision_tuple.revisionNumber,
      );
      const matched = sourceRowsMatchQuery(
        contentRows(selectedBlueprintAssessment(source, revision).content),
        request.query,
      );
      const items = matched.slice(cursor, cursor + pageSize);
      const nextOffset = cursor + items.length;
      return {
        items,
        aggregates: [],
        nextCursor: nextOffset < matched.length ? String(nextOffset) : null,
        facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
      };
    },
  };
}

const SUPPORTED_POOL_MEMBER_LICENSES = new Set(["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"]);

function questionPickerContent(
  row: QuestionLibraryBrowseRow,
  onInspect?: (row: QuestionLibraryBrowseRow) => void,
): RecordContent {
  return {
    title: row.questionTitle,
    description: row.summary,
    details: [
      { kind: "questionId", questionTitle: row.questionTitle, displayId: row.displayId },
      {
        kind: "text",
        label: "Revision",
        value: String(row.publishedQuestionRevisionTuple.revisionNumber),
      },
    ],
    actions:
      onInspect === undefined
        ? []
        : [{ id: "inspect", kind: "command", label: "Inspect", onClick: () => onInspect(row) }],
  };
}

function pickerQuery(
  query: QuestionPickerQuery,
  eligibility: QuestionPickerEligibility | undefined,
): QuestionPickerQuery {
  const normalized = normalizeQuestionLibraryBrowseQuery({
    ...query.libraryQuery,
    kind: "questions",
    membership: "all",
    discipline_uuid: eligibility?.disciplineUuid ?? query.libraryQuery.discipline_uuid,
    subject_uuid: eligibility?.subjectUuid ?? query.libraryQuery.subject_uuid,
    questionType: eligibility?.questionType ?? query.libraryQuery.questionType,
    backend: eligibility?.backend ?? query.libraryQuery.backend,
  });
  return { source: query.source, libraryQuery: normalized };
}

/**
 * Picker-local definition over the shared session. Source adapters remain caller-owned,
 * while this boundary fixes discovery to Questions and filters Pool-member candidates.
 */
export function questionPickerSearchDefinition(
  repository: QuestionPickerSourceRepository,
  initialSource: QuestionPickerSource,
  eligibility?: QuestionPickerEligibility,
  onInspect?: (row: QuestionLibraryBrowseRow) => void,
): SearchDefinition<
  QuestionPickerQuery,
  QuestionLibraryBrowseRow,
  QuestionLibraryBrowsePage["aggregates"]
> {
  const initialQuery = pickerQuery(
    { source: initialSource, libraryQuery: EMPTY_QUESTION_LIBRARY_BROWSE_QUERY },
    eligibility,
  );
  return {
    initialQuery,
    cleanup: (query) => pickerQuery(query, eligibility),
    getText: (query) => query.libraryQuery.search,
    setText: (query, search) => ({
      ...query,
      libraryQuery: { ...query.libraryQuery, search },
    }),
    rowId: (row) => row.displayId,
    content: (row) => questionPickerContent(row, onInspect),
    fetchPage: async (
      query,
      cursor,
      pageSize,
    ): Promise<SearchPage<QuestionLibraryBrowseRow, QuestionLibraryBrowsePage["aggregates"]>> => {
      const raw = await repository.search({
        source: query.source,
        query: query.libraryQuery,
        cursor,
        pageSize,
      });
      const page = decodeQuestionLibraryBrowsePage(raw);
      const items = rowsWithCanonicalUniqueQuestionIds(page.items).filter(
        (row) =>
          isCurrentProductionPickerRow(row) &&
          (eligibility === undefined ||
            SUPPORTED_POOL_MEMBER_LICENSES.has(row.questionLicense ?? "")) &&
          row.displayId !== eligibility?.excludedQuestionId,
      );
      return { items, nextCursor: page.nextCursor, filterCounts: page.aggregates };
    },
    selection: { maximum: MAX_QUESTION_PICKER_SELECTION_CAP },
  };
}
