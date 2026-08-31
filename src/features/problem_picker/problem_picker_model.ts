// problem_picker_model.ts - reusable, answer-free question selection contracts.

import { normalizeQuestionIdSyntax } from "../../question_id";
import type { AssignmentDefinitionSourceView } from "../../../generated/api/AssignmentDefinitionSourceView";
import type { ReusableCurriculumClient } from "../../api/reusable_curriculum";
import {
  EMPTY_CATALOG_QUERY,
  decodeCatalogBrowsePage,
  normalizeCatalogBrowseQuery,
  type CatalogBrowsePage,
  type CatalogBrowseQuery,
  type CatalogBrowseRepository,
  type CatalogBrowseRow,
} from "../../pages/library_page_model";

/** The largest selection any current D2 consumer can request. */
export const MAX_PROBLEM_PICKER_SELECTION_CAP = 1024;

/** A stable browser locator for a curation aggregate owned by a later server route. */
export type QuestionCollectionReference = string;

/** A stable browser locator for one retained course definition. */
export interface RetainedAssignmentReference {
  readonly course: string;
  readonly assignment: string;
}

/**
 * The picker exposes each selection source through the same answer-free D1 rows.
 */
export type ProblemPickerSource =
  | { readonly kind: "catalog"; readonly label: string }
  | { readonly kind: "sharedCatalog"; readonly label: string }
  | { readonly kind: "mine"; readonly label: string }
  | {
      readonly kind: "collection";
      readonly label: string;
      readonly collection: QuestionCollectionReference;
    }
  | {
      readonly kind: "retainedAssignment";
      readonly label: string;
      readonly retainedAssignment: RetainedAssignmentReference;
    }
  | {
      readonly kind: "blueprintCourseAssignment";
      readonly source: AssignmentDefinitionSourceView;
      readonly label: string;
    };

export type ProblemPickerSelectionMode = "none" | "one" | "many";

/** One ordered question selected from an answer-free D1 catalog result. */
export interface ProblemPickerSelectedQuestion {
  readonly questionId: string;
  readonly row: CatalogBrowseRow;
}

/** The one public completion value consumed by Library and assignment parents. */
export interface ProblemPickerSelection {
  readonly questionIds: ReadonlyArray<string>;
  readonly questions: ReadonlyArray<ProblemPickerSelectedQuestion>;
}

export interface ProblemPickerSearchRequest {
  readonly source: ProblemPickerSource;
  readonly query: CatalogBrowseQuery;
  readonly cursor: string | null;
}

/**
 * Source adapters remain parent-owned because D2 server routes arrive after the
 * picker shell. Each adapter returns the same hostile transport shape as D1.
 */
export interface ProblemPickerSourceRepository {
  readonly search: (request: ProblemPickerSearchRequest) => Promise<unknown>;
}

export type ProblemPickerCurationIntent = {
  readonly kind: "addToCollection";
  readonly selection: ProblemPickerSelection;
};

/** Parent composition owns persistence, permissions, and destination selection. */
export interface ProblemPickerCurationActions {
  readonly request: (intent: ProblemPickerCurationIntent) => void;
}

export type ProblemPickerState =
  | {
      readonly kind: "loading";
      readonly rows: ReadonlyArray<CatalogBrowseRow>;
      readonly aggregates: CatalogBrowsePage["aggregates"];
      readonly nextCursor: string | null;
    }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<CatalogBrowseRow>;
      readonly aggregates: CatalogBrowsePage["aggregates"];
      readonly nextCursor: string | null;
    }
  | { readonly kind: "empty"; readonly aggregates: CatalogBrowsePage["aggregates"] }
  | {
      readonly kind: "error";
      readonly rows: ReadonlyArray<CatalogBrowseRow>;
      readonly aggregates: CatalogBrowsePage["aggregates"];
      readonly nextCursor: string | null;
    };

function selectionLimit(mode: ProblemPickerSelectionMode, maximumSelection: number): number {
  if (mode === "none") return 0;
  if (
    !Number.isSafeInteger(maximumSelection) ||
    maximumSelection < 1 ||
    maximumSelection > MAX_PROBLEM_PICKER_SELECTION_CAP
  ) {
    throw new Error(
      `Choose a selection maximum from 1 through ${MAX_PROBLEM_PICKER_SELECTION_CAP}.`,
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
  rows: ReadonlyArray<CatalogBrowseRow>,
): ReadonlyArray<CatalogBrowseRow> {
  const known = new Set<string>();
  const unique: CatalogBrowseRow[] = [];
  for (const row of rows) {
    const questionId = canonicalQuestionId(row.displayId);
    if (known.has(questionId)) continue;
    known.add(questionId);
    unique.push(row);
  }
  return unique;
}

/** Builds the public, ordered selection while retaining only D1-safe row metadata. */
export function problemPickerSelection(
  mode: ProblemPickerSelectionMode,
  maximumSelection: number,
  rows: ReadonlyArray<CatalogBrowseRow>,
): ProblemPickerSelection {
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
export function toggleProblemPickerSelection(
  mode: ProblemPickerSelectionMode,
  maximumSelection: number,
  selection: ProblemPickerSelection,
  row: CatalogBrowseRow,
  selected: boolean,
): ProblemPickerSelection {
  const questionId = canonicalQuestionId(row.displayId);
  const withoutCurrent = selection.questions.filter(
    (question) => question.questionId !== questionId,
  );
  if (!selected)
    return problemPickerSelection(
      mode,
      maximumSelection,
      withoutCurrent.map((question) => question.row),
    );
  if (mode === "one") return problemPickerSelection(mode, maximumSelection, [row]);
  return problemPickerSelection(mode, maximumSelection, [
    ...withoutCurrent.map((question) => question.row),
    row,
  ]);
}

/** Reorders the current tray without changing its public question membership. */
export function moveProblemPickerSelection(
  mode: ProblemPickerSelectionMode,
  maximumSelection: number,
  selection: ProblemPickerSelection,
  index: number,
  direction: -1 | 1,
): ProblemPickerSelection {
  const nextIndex = index + direction;
  if (index < 0 || nextIndex < 0 || nextIndex >= selection.questions.length) return selection;
  const questions = [...selection.questions];
  const current = questions[index];
  const adjacent = questions[nextIndex];
  if (current === undefined || adjacent === undefined) return selection;
  questions[index] = adjacent;
  questions[nextIndex] = current;
  return problemPickerSelection(
    mode,
    maximumSelection,
    questions.map((question) => question.row),
  );
}

/** Small adapter that lets catalog-only parents compose the shared picker immediately. */
export function catalogProblemPickerRepository(
  catalog: CatalogBrowseRepository,
): ProblemPickerSourceRepository {
  return {
    async search(request: ProblemPickerSearchRequest): Promise<unknown> {
      if (request.source.kind !== "catalog") {
        throw new Error("This picker composition has not connected that source yet.");
      }
      return await catalog.search(request.query, request.cursor);
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

function reusableCatalogRow(item: {
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
}): CatalogBrowseRow {
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
  source: AssignmentDefinitionSourceView,
  course: Awaited<ReturnType<ReusableCurriculumClient["getBlueprintCourse"]>>["blueprintCourse"],
): Awaited<
  ReturnType<ReusableCurriculumClient["getBlueprintCourse"]>
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
        readonly question: { readonly catalog: Parameters<typeof reusableCatalogRow>[0] };
      }
    | {
        readonly kind: "pool";
        readonly candidates: ReadonlyArray<{
          readonly catalog: Parameters<typeof reusableCatalogRow>[0];
        }>;
      }
  >;
}): ReadonlyArray<CatalogBrowseRow> {
  const rows: CatalogBrowseRow[] = [];
  for (const entry of definition.entries) {
    if (entry.kind === "fixed") rows.push(reusableCatalogRow(entry.question.catalog));
    else for (const candidate of entry.candidates) rows.push(reusableCatalogRow(candidate.catalog));
  }
  return rows;
}

function sourceRowsMatchQuery(
  rows: ReadonlyArray<CatalogBrowseRow>,
  query: CatalogBrowseQuery,
): CatalogBrowseRow[] {
  const needle = query.search.trim().toLocaleLowerCase();
  if (needle === "") return [...rows];
  return rows.filter((row) => `${row.title}\n${row.summary}`.toLocaleLowerCase().includes(needle));
}

/** Connects reusable definitions to the established picker without creating a second row model. */
export function reusableCurriculumProblemPickerRepository(
  client: ReusableCurriculumClient,
): ProblemPickerSourceRepository {
  return {
    async search(request: ProblemPickerSearchRequest): Promise<unknown> {
      const offset = pickerPageOffset(request.cursor);
      let rows: ReadonlyArray<CatalogBrowseRow>;
      if (request.source.kind === "blueprintCourseAssignment") {
        const observed = await client.getBlueprintCourse(request.source.source.reference);
        rows = definitionRows(
          selectedBlueprintAssignment(request.source.source, observed.blueprintCourse).definition,
        );
      } else {
        throw new Error("Choose a reusable curriculum source for this picker composition.");
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
export class ProblemPickerSession {
  #generation = 0;
  #source: ProblemPickerSource | undefined;
  #query = EMPTY_CATALOG_QUERY;
  #state: ProblemPickerState = { kind: "loading", rows: [], aggregates: [], nextCursor: null };
  #loading = false;
  #queuedReset = false;

  public constructor(
    private readonly repository: ProblemPickerSourceRepository,
    private readonly publish: (state: ProblemPickerState) => void,
  ) {}

  public get state(): ProblemPickerState {
    return this.#state;
  }

  public async reset(source: ProblemPickerSource, query: CatalogBrowseQuery): Promise<void> {
    this.#generation += 1;
    this.#source = source;
    this.#query = normalizeCatalogBrowseQuery(query);
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

  private setState(state: ProblemPickerState): void {
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
      const page = decodeCatalogBrowsePage(raw);
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
