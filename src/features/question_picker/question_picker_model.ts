// question_picker_model.ts - reusable, answer-free Question Picker contracts.

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionType } from "../../../generated/api/QuestionType";
import { questionNativePreview } from "../../components/question_native_preview";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type {
  SearchDefinition,
  SearchPage,
  SearchPageRequest,
} from "../../components/search_block/search_definition";
import type { QuestionDetailsPromptView } from "../../../generated/api/QuestionDetailsPromptView";
import type { QuestionResponsePreview } from "../../../generated/api/QuestionResponsePreview";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import { validateCanonicalQuestionIdSyntax } from "../../question_id";
import {
  type QuestionLibraryBrowseRepository,
  type QuestionLibraryBrowsePage,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryBrowseRow,
  type LibrarySearchPage,
} from "../../api/question_library_projection";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  normalizeQuestionLibraryBrowseQuery,
} from "../question_library/library_query";

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
    };

export type QuestionPickerSelectionMode = "none" | "one" | "many";

/** One ordered question selected from an answer-free D1 Question Search result. */
export interface QuestionPickerSelectedQuestion {
  readonly questionId: string;
  readonly row: QuestionLibraryBrowseRow;
}

/** Answer-free Question Details kept for inspection before selection. */
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

const SUPPORTED_POOL_MEMBER_LICENSES = ["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0"] as const;

export interface QuestionPickerSearchRequest {
  readonly source: QuestionPickerSource;
  readonly query: QuestionLibraryBrowseQuery;
  readonly page: SearchPageRequest;
  readonly eligibility?: QuestionPickerEligibility;
}

/**
 * Source adapters remain parent-owned because D2 server routes arrive after the
 * picker shell. Each adapter returns the typed browser projection from its route.
 */
export interface QuestionPickerSourceRepository {
  readonly search: (request: QuestionPickerSearchRequest) => Promise<LibrarySearchPage>;
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
  /** A source-bound Pool never offers its fixed starting Questions again. */
  readonly excludedQuestionIds?: ReadonlyArray<string>;
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
  if (!selected)
    return questionPickerSelection(
      mode,
      maximumSelection,
      selection.questions
        .filter((question) => question.questionId !== questionId)
        .map((question) => question.row),
    );
  // Selecting an already chosen Question does not replace its pinned revision or move it.
  if (selection.questionIds.includes(questionId)) return selection;
  if (mode === "one") return questionPickerSelection(mode, maximumSelection, [row]);
  return questionPickerSelection(mode, maximumSelection, [
    ...selection.questions.map((question) => question.row),
    row,
  ]);
}

/** Adapts available Question Library searches for the picker source choices. */
export function questionLibraryPickerRepository(
  library: QuestionLibraryBrowseRepository,
  myQuestions: QuestionLibraryBrowseRepository,
): QuestionPickerSourceRepository {
  return {
    async search(request: QuestionPickerSearchRequest): Promise<LibrarySearchPage> {
      if (request.source.kind === "library" || request.source.kind === "sharedLibrary") {
        return await library.search(
          { ...request.query, kind: "questions", questions: "all" },
          request.page,
          { excludedQuestionIds: request.eligibility?.excludedQuestionIds },
        );
      }
      if (request.source.kind === "mine") {
        return await myQuestions.search(
          { ...request.query, kind: "questions", questions: "all" },
          request.page,
          { excludedQuestionIds: request.eligibility?.excludedQuestionIds },
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

function questionPickerContent(
  row: QuestionLibraryBrowseRow,
  onInspect: (row: QuestionLibraryBrowseRow) => void,
): RecordContent {
  return {
    title: row.questionTitle,
    preview: questionNativePreview(row),
    description: row.summary,
    details: [
      { kind: "questionId", questionTitle: row.questionTitle, displayId: row.displayId },
      {
        kind: "text",
        label: "Revision",
        value: String(row.publishedQuestionRevisionTuple.revisionNumber),
      },
    ],
    actions: [{ id: "inspect", kind: "command", label: "Inspect", onClick: () => onInspect(row) }],
  };
}

function pickerQuery(query: QuestionPickerQuery): QuestionPickerQuery {
  const normalized = normalizeQuestionLibraryBrowseQuery({
    ...query.libraryQuery,
    kind: "questions",
    questions: "all",
  });
  return { source: query.source, libraryQuery: normalized };
}

function constrainedPickerLibraryQuery(
  query: QuestionLibraryBrowseQuery,
  eligibility: QuestionPickerEligibility | undefined,
): QuestionLibraryBrowseQuery {
  const intersect = (selected: ReadonlyArray<string>, allowed: ReadonlyArray<string>): string[] =>
    selected.length === 0 ? [...allowed] : selected.filter((value) => allowed.includes(value));
  const backends = eligibility ? [eligibility.backend] : ["ple", "webwork"];
  return {
    ...query,
    discipline_uuids: eligibility
      ? intersect(query.discipline_uuids, [eligibility.disciplineUuid])
      : query.discipline_uuids,
    subject_uuids: eligibility
      ? intersect(query.subject_uuids, [eligibility.subjectUuid])
      : query.subject_uuids,
    questionTypes: eligibility
      ? intersect(query.questionTypes, [eligibility.questionType])
      : query.questionTypes,
    backends: intersect(query.backends, backends),
    questionLicenses: eligibility
      ? intersect(query.questionLicenses, SUPPORTED_POOL_MEMBER_LICENSES)
      : query.questionLicenses,
  };
}

function pickerQueryMatchesEligibility(
  query: QuestionPickerQuery,
  eligibility: QuestionPickerEligibility | undefined,
): boolean {
  const hasMatch = (selected: ReadonlyArray<string>, allowed: ReadonlyArray<string>): boolean =>
    selected.length === 0 || selected.some((value) => allowed.includes(value));
  return (
    hasMatch(
      query.libraryQuery.backends,
      eligibility ? [eligibility.backend] : ["ple", "webwork"],
    ) &&
    (eligibility === undefined ||
      (hasMatch(query.libraryQuery.discipline_uuids, [eligibility.disciplineUuid]) &&
        hasMatch(query.libraryQuery.subject_uuids, [eligibility.subjectUuid]) &&
        hasMatch(query.libraryQuery.questionTypes, [eligibility.questionType]) &&
        (query.libraryQuery.questionLicenses.length === 0 ||
          query.libraryQuery.questionLicenses.some((license) =>
            SUPPORTED_POOL_MEMBER_LICENSES.includes(
              license as (typeof SUPPORTED_POOL_MEMBER_LICENSES)[number],
            ),
          ))))
  );
}

/**
 * Picker-local definition over the shared session. Source adapters remain caller-owned,
 * while this boundary fixes discovery to Questions and filters Pool-member candidates.
 */
export function questionPickerSearchDefinition(
  repository: QuestionPickerSourceRepository,
  initialSource: QuestionPickerSource,
  onInspect: (row: QuestionLibraryBrowseRow) => void,
  eligibility?: QuestionPickerEligibility,
): SearchDefinition<
  QuestionPickerQuery,
  QuestionLibraryBrowseRow,
  QuestionLibraryBrowsePage["aggregates"]
> {
  const initialQuery = pickerQuery({
    source: initialSource,
    libraryQuery: EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  });
  return {
    initialQuery,
    cleanup: (query) => pickerQuery(query),
    isEligible: (query) => pickerQueryMatchesEligibility(query, eligibility),
    getText: (query) => query.libraryQuery.search,
    setText: (query, search) => ({
      ...query,
      libraryQuery: { ...query.libraryQuery, search },
    }),
    rowId: (row) => row.displayId,
    content: (row) => questionPickerContent(row, onInspect),
    fetchPage: async (
      query,
      page,
    ): Promise<SearchPage<QuestionLibraryBrowseRow, QuestionLibraryBrowsePage["aggregates"]>> => {
      const result = await repository.search({
        source: query.source,
        query: constrainedPickerLibraryQuery(query.libraryQuery, eligibility),
        page,
        eligibility,
      });
      const items = result.items.map((row) => {
        if (row.kind !== "question") throw new Error("Question picker result contained a Pool");
        return row;
      });
      return {
        items,
        pageNumber: result.pageNumber,
        pageSize: result.pageSize,
        totalCount: result.totalCount,
        filterCounts: result.aggregates,
      };
    },
    selection: { maximum: MAX_QUESTION_PICKER_SELECTION_CAP },
  };
}
