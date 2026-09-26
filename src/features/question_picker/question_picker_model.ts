// question_picker_model.ts - reusable, answer-free Question Picker contracts.

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

export type QuestionPickerState =
  | {
      readonly kind: "loading";
      readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      readonly aggregates: QuestionLibraryBrowsePage["aggregates"];
      readonly nextCursor: string | null;
    }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      readonly aggregates: QuestionLibraryBrowsePage["aggregates"];
      readonly nextCursor: string | null;
    }
  | { readonly kind: "empty"; readonly aggregates: QuestionLibraryBrowsePage["aggregates"] }
  | {
      readonly kind: "error";
      readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      readonly aggregates: QuestionLibraryBrowsePage["aggregates"];
      readonly nextCursor: string | null;
    };

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
        return await library.search(request.query, request.cursor, request.pageSize);
      }
      if (request.source.kind === "mine") {
        return await myQuestions.search(request.query, request.cursor, request.pageSize);
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

const DEFAULT_QUESTION_PICKER_PAGE_SIZE: QuestionPickerPageSize = 50;

function pickerPageSize(pageSize: QuestionPickerPageSize | undefined): QuestionPickerPageSize {
  return pageSize ?? DEFAULT_QUESTION_PICKER_PAGE_SIZE;
}

function pickerPageOffset(cursor: string | null): number {
  if (cursor === null) return 0;
  if (!/^(?:0|[1-9][0-9]*)$/u.test(cursor)) {
    throw new Error("Use the picker continuation supplied by this source.");
  }
  const offset = Number(cursor);
  if (!Number.isSafeInteger(offset) || offset < 0) {
    throw new Error("Use the picker continuation supplied by this source.");
  }
  return offset;
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
  const evidence = item.evidence;
  return {
    displayId: summary.questionId,
    publishedQuestionRevisionTuple: summary.publishedQuestionRevisionTuple,
    questionTitle: summary.metadata.questionTitle,
    summary: summary.metadata.questionDescription,
    bloom: summary.bloom,
    disciplineName: item.disciplineName,
    disciplineIsRetired: item.disciplineIsRetired,
    questionFormat: summary.questionFormat,
    authorNames: summary.authorship.authors.map((author) => author.displayName),
    capabilities: summary.capabilities,
    questionLicense: summary.metadata.questionLicense,
    evidence,
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
    const content = module.assessments.find(
      (assessment) => assessment.blueprint_assessment_id === source.blueprint_assessment_id,
    );
    if (content !== undefined) return content;
  }
  throw new Error(
    "The selected Blueprint Assessment is not available in this exact Blueprint Revision.",
  );
}

function contentRows(
  content: BlueprintAssessmentContentView,
): ReadonlyArray<QuestionLibraryBrowseRow> {
  const rows: QuestionLibraryBrowseRow[] = [];
  for (const assessmentEntry of content.entries) {
    // Pool members are selected through the published-Pool picker. This fixed-Question
    // source exposes only Questions that the Blueprint Assessment stores as fixed entries.
    if (assessmentEntry.kind === "fixed") {
      rows.push(reusableQuestionLibraryRow(assessmentEntry.question.question_library));
    }
  }
  return rows;
}

function sourceRowsMatchQuery(
  rows: ReadonlyArray<QuestionLibraryBrowseRow>,
  query: QuestionLibraryBrowseQuery,
): QuestionLibraryBrowseRow[] {
  const needle = query.search.trim().toLocaleLowerCase();
  if (needle === "") return [...rows];
  return rows.filter((row) =>
    `${row.questionTitle}\n${row.summary}`.toLocaleLowerCase().includes(needle),
  );
}

/** Connects Blueprint Assessments to the established picker without creating a second row model. */
export function blueprintCourseQuestionPickerRepository(
  client: BlueprintCourseClient,
): QuestionPickerSourceRepository {
  return {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      const pageSize = pickerPageSize(request.pageSize);
      const offset = pickerPageOffset(request.cursor);
      let rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      if (request.source.kind === "blueprintCourseAssessment") {
        const source = request.source.source;
        const blueprintRevision = await client.getBlueprintRevision(
          source.blueprint_revision_tuple.blueprintCourseId,
          source.blueprint_revision_tuple.revisionNumber,
        );
        rows = contentRows(selectedBlueprintAssessment(source, blueprintRevision).content);
      } else {
        throw new Error("Choose a Blueprint Course source for this picker composition.");
      }
      const matched = sourceRowsMatchQuery(rows, request.query);
      const items = matched.slice(offset, offset + pageSize);
      const nextOffset = offset + items.length;
      const nextCursor = nextOffset < matched.length ? String(nextOffset) : null;
      return {
        items,
        aggregates: [],
        nextCursor,
        facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
      };
    },
  };
}

/**
 * Source-aware, cursor-only session. A changed source or query invalidates an
 * earlier response; errors retain already loaded safe rows and the selection
 * belongs to the component, outside this transport session.
 */
export class QuestionPickerSession {
  #generation = 0;
  #source: QuestionPickerSource | undefined;
  #query = EMPTY_QUESTION_LIBRARY_BROWSE_QUERY;
  #state: QuestionPickerState = { kind: "loading", rows: [], aggregates: [], nextCursor: null };
  #loading = false;
  #queuedReset = false;
  #pageSize: QuestionPickerPageSize = DEFAULT_QUESTION_PICKER_PAGE_SIZE;
  #pageCursor: string | null = null;
  #previousCursors: ReadonlyArray<string | null> = [];

  public constructor(
    private readonly repository: QuestionPickerSourceRepository,
    private readonly publish: (state: QuestionPickerState) => void,
  ) {}

  public get state(): QuestionPickerState {
    return this.#state;
  }

  public get pageSize(): QuestionPickerPageSize {
    return this.#pageSize;
  }

  public get hasPrevious(): boolean {
    return this.#previousCursors.length > 0;
  }

  public get loading(): boolean {
    return this.#loading;
  }

  public async reset(
    source: QuestionPickerSource,
    query: QuestionLibraryBrowseQuery,
  ): Promise<void> {
    this.#generation += 1;
    this.#source = source;
    this.#query = normalizeQuestionLibraryBrowseQuery(query);
    this.#pageCursor = null;
    this.#previousCursors = [];
    if (this.#loading) {
      this.#queuedReset = true;
      return;
    }
    await this.loadPage(null, [], this.#generation);
  }

  public async changePageSize(pageSize: QuestionPickerPageSize): Promise<void> {
    if (pageSize === this.#pageSize) return;
    this.#pageSize = pageSize;
    const source = this.#source;
    if (source === undefined) return;
    await this.reset(source, this.#query);
  }

  public async retry(): Promise<void> {
    this.#generation += 1;
    if (this.#loading) {
      this.#queuedReset = true;
      return;
    }
    await this.loadPage(this.#pageCursor, this.#previousCursors, this.#generation);
  }

  public async loadNext(): Promise<void> {
    if (this.#loading || this.#state.kind !== "ready" || this.#state.nextCursor === null) return;
    await this.loadPage(
      this.#state.nextCursor,
      [...this.#previousCursors, this.#pageCursor],
      this.#generation,
    );
  }

  public async loadPrevious(): Promise<void> {
    if (this.#loading || this.#previousCursors.length === 0) return;
    const cursors = this.#previousCursors;
    const cursor = cursors[cursors.length - 1] ?? null;
    await this.loadPage(cursor, cursors.slice(0, -1), this.#generation);
  }

  private setState(state: QuestionPickerState): void {
    this.#state = state;
    this.publish(state);
  }

  private async loadPage(
    cursor: string | null,
    previousCursors: ReadonlyArray<string | null>,
    generation: number,
  ): Promise<void> {
    const source = this.#source;
    if (this.#loading || source === undefined) return;
    this.#loading = true;
    const previous = this.#state;
    const retainedRows = previous.kind === "empty" ? [] : previous.rows;
    const retainedAggregates = previous.aggregates;
    const retainedCursor = previous.kind === "empty" ? null : previous.nextCursor;
    this.setState({
      kind: "loading",
      rows: retainedRows,
      aggregates: retainedAggregates,
      nextCursor: retainedCursor,
    });
    try {
      const raw = await this.repository.search({
        source,
        query: this.#query,
        cursor,
        pageSize: this.#pageSize,
      });
      const page = decodeQuestionLibraryBrowsePage(raw);
      if (generation !== this.#generation) return;
      this.#pageCursor = cursor;
      this.#previousCursors = previousCursors;
      const rows = rowsWithCanonicalUniqueQuestionIds(
        page.items.filter(isCurrentProductionPickerRow),
      );
      this.setState(
        rows.length === 0
          ? { kind: "empty", aggregates: page.aggregates }
          : { kind: "ready", rows, aggregates: page.aggregates, nextCursor: page.nextCursor },
      );
    } catch {
      if (generation === this.#generation) {
        this.setState({
          kind: "error",
          rows: retainedRows,
          aggregates: retainedAggregates,
          nextCursor: retainedCursor,
        });
      }
    } finally {
      this.#loading = false;
      if (this.#queuedReset) {
        this.#queuedReset = false;
        void this.loadPage(null, [], this.#generation);
      }
    }
  }
}
