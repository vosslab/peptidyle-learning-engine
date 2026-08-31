// question_picker_model.ts - reusable, answer-free Question Picker contracts.

import { normalizeQuestionIdSyntax } from "../../question_id";
import type { BlueprintAssignmentRevisionReference } from "../../../generated/api/BlueprintAssignmentRevisionReference";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import {
  EMPTY_QUESTION_SEARCH_QUERY,
  decodeQuestionSearchPage,
  normalizeQuestionSearchQuery,
  type QuestionLibraryRepository,
  type QuestionSearchPage,
  type QuestionSearchQuery,
  type QuestionSearchResult,
} from "../../pages/library_page_model";

/** The largest selection any current D2 consumer can request. */
export const MAX_QUESTION_PICKER_SELECTION_CAP = 1024;

/** A stable browser locator for a curation aggregate owned by a later server route. */
export type QuestionFolderReference = string;

/** A stable browser locator for one retained course definition. */
export interface RetainedAssignmentReference {
  readonly course: string;
  readonly assignment: string;
}

/**
 * The picker exposes each selection source through the same answer-free D1 rows.
 */
export type QuestionPickerSource =
  | { readonly kind: "library"; readonly label: string }
  | { readonly kind: "sharedLibrary"; readonly label: string }
  | { readonly kind: "mine"; readonly label: string }
  | {
      readonly kind: "folder";
      readonly label: string;
      readonly folder: QuestionFolderReference;
    }
  | {
      readonly kind: "retainedAssignment";
      readonly label: string;
      readonly retainedAssignment: RetainedAssignmentReference;
    }
  | {
      readonly kind: "blueprintCourseAssignment";
      readonly source: BlueprintAssignmentRevisionReference;
      readonly label: string;
    };

export type QuestionPickerSelectionMode = "none" | "one" | "many";

/** One ordered question selected from an answer-free D1 Question Search result. */
export interface QuestionPickerSelectedQuestion {
  readonly questionId: string;
  readonly row: QuestionSearchResult;
}

/** The one public completion value consumed by Library and assignment parents. */
export interface QuestionPickerSelection {
  readonly questionIds: ReadonlyArray<string>;
  readonly questions: ReadonlyArray<QuestionPickerSelectedQuestion>;
}

export interface QuestionPickerSearchRequest {
  readonly source: QuestionPickerSource;
  readonly query: QuestionSearchQuery;
  readonly cursor: string | null;
}

/**
 * Source adapters remain parent-owned because D2 server routes arrive after the
 * picker shell. Each adapter returns the same hostile transport shape as D1.
 */
export interface QuestionPickerSourceRepository {
  readonly search: (request: QuestionPickerSearchRequest) => Promise<unknown>;
}

export type QuestionPickerCurationIntent = {
  readonly kind: "addToCollection";
  readonly selection: QuestionPickerSelection;
};

/** Parent composition owns persistence, permissions, and destination selection. */
export interface QuestionPickerCurationActions {
  readonly request: (intent: QuestionPickerCurationIntent) => void;
}

export type QuestionPickerState =
  | {
      readonly kind: "loading";
      readonly rows: ReadonlyArray<QuestionSearchResult>;
      readonly aggregates: QuestionSearchPage["aggregates"];
      readonly nextCursor: string | null;
    }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<QuestionSearchResult>;
      readonly aggregates: QuestionSearchPage["aggregates"];
      readonly nextCursor: string | null;
    }
  | { readonly kind: "empty"; readonly aggregates: QuestionSearchPage["aggregates"] }
  | {
      readonly kind: "error";
      readonly rows: ReadonlyArray<QuestionSearchResult>;
      readonly aggregates: QuestionSearchPage["aggregates"];
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
  const questionId = normalizeQuestionIdSyntax(value);
  if (questionId === null)
    throw new Error("A selected question must have a canonical Question ID.");
  return questionId;
}

function rowsWithUniqueQuestionIds(
  rows: ReadonlyArray<QuestionSearchResult>,
): ReadonlyArray<QuestionSearchResult> {
  const known = new Set<string>();
  const unique: QuestionSearchResult[] = [];
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
  rows: ReadonlyArray<QuestionSearchResult>,
): QuestionPickerSelection {
  const uniqueRows = rowsWithUniqueQuestionIds(rows);
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
  row: QuestionSearchResult,
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

/** Small adapter that lets Question-Library-only parents compose the shared picker immediately. */
export function questionLibraryPickerRepository(
  library: QuestionLibraryRepository,
): QuestionPickerSourceRepository {
  return {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      if (request.source.kind !== "library") {
        throw new Error("This picker composition has not connected that source yet.");
      }
      return await library.search(request.query, request.cursor);
    },
  };
}

const PICKER_SOURCE_PAGE_SIZE = 100;

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
    readonly metadata: {
      readonly title: string;
      readonly taxonomy: ReadonlyArray<{
        readonly scheme: string;
        readonly code: string;
        readonly label: string;
      }>;
      readonly license:
        | { readonly kind: "allRightsReserved" | "ccBy" | "ccBySa" | "ccByNc" | "cc0" }
        | { readonly kind: "other"; readonly spdx: string };
    };
    readonly byline: { readonly names: ReadonlyArray<string> };
    readonly capabilities: ReadonlyArray<string>;
  };
  readonly evidence:
    | { readonly state: "insufficientEvidence" }
    | {
        readonly state: "available";
        readonly observedCourseCount: number;
        readonly independentLearnerObservationCount: number;
        readonly difficultyIndex: number;
        readonly discriminationIndex?: number;
      };
}): QuestionSearchResult {
  const summary = item.summary;
  const evidence =
    item.evidence.state === "insufficientEvidence"
      ? { state: "insufficientEvidence" as const }
      : {
          state: "available" as const,
          observedCourseCount: item.evidence.observedCourseCount,
          independentLearnerObservationCount: item.evidence.independentLearnerObservationCount,
          difficultyIndex: item.evidence.difficultyIndex,
          discriminationIndex: item.evidence.discriminationIndex,
        };
  return {
    displayId: summary.questionId,
    title: summary.metadata.title,
    summary: summary.metadata.title,
    byline: summary.byline.names,
    taxonomy: summary.metadata.taxonomy.map((term) => `${term.scheme}:${term.code}`),
    capabilities: summary.capabilities,
    license:
      summary.metadata.license.kind === "other"
        ? summary.metadata.license.spdx
        : summary.metadata.license.kind,
    evidence,
  };
}

function selectedBlueprintAssignment(
  source: BlueprintAssignmentRevisionReference,
  course: Awaited<ReturnType<BlueprintCourseClient["getBlueprintCourse"]>>["blueprintCourse"],
): Awaited<
  ReturnType<BlueprintCourseClient["getBlueprintCourse"]>
>["blueprintCourse"]["modules"][number]["definitions"][number] {
  if (course.reference !== source.reference || course.revision !== source.revision) {
    throw new Error("The selected Blueprint Course changed. Choose a current reusable assignment.");
  }
  for (const module of course.modules) {
    const definition = module.definitions.find(
      (assignment) => assignment.assignment_id === source.assignment_id,
    );
    if (definition !== undefined) return definition;
  }
  throw new Error(
    "The selected reusable assignment is no longer available in this Blueprint Course.",
  );
}

function definitionRows(definition: {
  readonly entries: ReadonlyArray<
    | {
        readonly kind: "fixed";
        readonly question: {
          readonly question_library: Parameters<typeof reusableQuestionLibraryRow>[0];
        };
      }
    | {
        readonly kind: "pool";
        readonly candidates: ReadonlyArray<{
          readonly question_library: Parameters<typeof reusableQuestionLibraryRow>[0];
        }>;
      }
  >;
}): ReadonlyArray<QuestionSearchResult> {
  const rows: QuestionSearchResult[] = [];
  for (const entry of definition.entries) {
    if (entry.kind === "fixed")
      rows.push(reusableQuestionLibraryRow(entry.question.question_library));
    else
      for (const candidate of entry.candidates)
        rows.push(reusableQuestionLibraryRow(candidate.question_library));
  }
  return rows;
}

function sourceRowsMatchQuery(
  rows: ReadonlyArray<QuestionSearchResult>,
  query: QuestionSearchQuery,
): QuestionSearchResult[] {
  const needle = query.search.trim().toLocaleLowerCase();
  if (needle === "") return [...rows];
  return rows.filter((row) => `${row.title}\n${row.summary}`.toLocaleLowerCase().includes(needle));
}

/** Connects reusable definitions to the established picker without creating a second row model. */
export function blueprintCourseQuestionPickerRepository(
  client: BlueprintCourseClient,
): QuestionPickerSourceRepository {
  return {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      const offset = pickerPageOffset(request.cursor);
      let rows: ReadonlyArray<QuestionSearchResult>;
      if (request.source.kind === "blueprintCourseAssignment") {
        const observed = await client.getBlueprintCourse(request.source.source.reference);
        rows = definitionRows(
          selectedBlueprintAssignment(request.source.source, observed.blueprintCourse).definition,
        );
      } else {
        throw new Error("Choose a Blueprint Course source for this picker composition.");
      }
      const matched = sourceRowsMatchQuery(rows, request.query);
      const items = matched.slice(offset, offset + PICKER_SOURCE_PAGE_SIZE);
      const nextOffset = offset + items.length;
      const nextCursor = nextOffset < matched.length ? String(nextOffset) : null;
      return { items, aggregates: [], nextCursor };
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
  #query = EMPTY_QUESTION_SEARCH_QUERY;
  #state: QuestionPickerState = { kind: "loading", rows: [], aggregates: [], nextCursor: null };
  #loading = false;
  #queuedReset = false;

  public constructor(
    private readonly repository: QuestionPickerSourceRepository,
    private readonly publish: (state: QuestionPickerState) => void,
  ) {}

  public get state(): QuestionPickerState {
    return this.#state;
  }

  public async reset(source: QuestionPickerSource, query: QuestionSearchQuery): Promise<void> {
    this.#generation += 1;
    this.#source = source;
    this.#query = normalizeQuestionSearchQuery(query);
    if (this.#loading) {
      this.#queuedReset = true;
      return;
    }
    await this.loadPage(null, true, this.#generation);
  }

  public async retry(): Promise<void> {
    this.#generation += 1;
    if (this.#loading) {
      this.#queuedReset = true;
      return;
    }
    await this.loadPage(null, true, this.#generation);
  }

  public async loadNext(): Promise<void> {
    if (this.#loading || this.#state.kind !== "ready" || this.#state.nextCursor === null) return;
    await this.loadPage(this.#state.nextCursor, false, this.#generation);
  }

  private setState(state: QuestionPickerState): void {
    this.#state = state;
    this.publish(state);
  }

  private async loadPage(
    cursor: string | null,
    replace: boolean,
    generation: number,
  ): Promise<void> {
    const source = this.#source;
    if (this.#loading || source === undefined) return;
    this.#loading = true;
    const previous = this.#state;
    const retainedRows = replace || previous.kind === "empty" ? [] : previous.rows;
    const retainedAggregates = previous.aggregates;
    const retainedCursor = replace || previous.kind === "empty" ? null : previous.nextCursor;
    this.setState({
      kind: "loading",
      rows: retainedRows,
      aggregates: retainedAggregates,
      nextCursor: retainedCursor,
    });
    try {
      const raw = await this.repository.search({ source, query: this.#query, cursor });
      const page = decodeQuestionSearchPage(raw);
      if (generation !== this.#generation) return;
      const rows = rowsWithUniqueQuestionIds(
        replace ? page.items : [...retainedRows, ...page.items],
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
        void this.loadPage(null, true, this.#generation);
      }
    }
  }
}
