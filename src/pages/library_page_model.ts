// library_page_model.ts - bounded, transport-validated Question Library browse state.

import { normalizeQuestionIdSyntax } from "../question_id";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";

/** A browser-safe current Question Library record. */
export interface QuestionSearchResult {
  /** Copy/paste identity used by instructors and the browser deduplication key. */
  readonly displayId: string;
  readonly title: string;
  readonly summary: string;
  /** Reviewed publication-time attribution; never an account or ownership identity. */
  readonly byline: ReadonlyArray<string>;
  readonly classifications: ReadonlyArray<string>;
  readonly capabilities: ReadonlyArray<string>;
  readonly license: string;
  /** Server-disclosed learning evidence for this exact immutable publication. */
  readonly evidence: QuestionSearchEvidence;
}

/**
 * A presentation-ready, answer-free view of the server-owned discovery evidence.
 *
 * The browser intentionally receives no quality contribution. Available evidence
 * supplies only the disclosed observations instructors can interpret; an
 * insufficient state stays explicitly neutral in relevance-ranked search.
 */
export type QuestionSearchEvidence =
  | { readonly state: "insufficientEvidence" }
  | {
      readonly state: "available";
      readonly observedCourseCount: number;
      readonly independentLearnerObservationCount: number;
      readonly difficultyIndex: number;
      readonly discriminationIndex: number | undefined;
    };

/** Server-computed count for the exact active query; never derived from loaded rows. */
export interface QuestionSearchFacetAggregate {
  readonly facet:
    | "byline"
    | "backend"
    | "tag"
    | "questionType"
    | "classification"
    | "capability"
    | "license"
    | "evidence"
    | "usedInMyCourses";
  readonly value: string;
  readonly count: number;
}

export interface QuestionSearchQuery {
  readonly search: string;
  readonly byline: string | null;
  readonly backend: string | null;
  readonly tag: string | null;
  readonly questionType: string | null;
  readonly classification: string | null;
  readonly capability: string | null;
  readonly license: string | null;
  readonly evidence: string | null;
  readonly usedInMyCourses: string | null;
  /** Closed server-resolved authorship scope; browser rows never carry Account identity. */
  readonly authorship: QuestionSearchAuthorship;
}

export interface QuestionSearchPage {
  readonly items: ReadonlyArray<QuestionSearchResult>;
  readonly nextCursor: string | null;
  readonly aggregates: ReadonlyArray<QuestionSearchFacetAggregate>;
}

/**
 * The production Question Library repository adapts to this narrow boundary. A hostile result is
 * intentional: this module owns browser-side validation before any row reaches JSX.
 */
export interface QuestionLibraryRepository {
  readonly search: (query: QuestionSearchQuery, cursor: string | null) => Promise<unknown>;
}

export type QuestionSearchState =
  | {
      readonly kind: "loading";
      readonly rows: ReadonlyArray<QuestionSearchResult>;
      readonly aggregates: ReadonlyArray<QuestionSearchFacetAggregate>;
      readonly nextCursor: string | null;
    }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<QuestionSearchResult>;
      readonly nextCursor: string | null;
      readonly aggregates: ReadonlyArray<QuestionSearchFacetAggregate>;
    }
  | { readonly kind: "empty"; readonly aggregates: ReadonlyArray<QuestionSearchFacetAggregate> }
  | {
      readonly kind: "error";
      readonly rows: ReadonlyArray<QuestionSearchResult>;
      readonly aggregates: ReadonlyArray<QuestionSearchFacetAggregate>;
      readonly nextCursor: string | null;
    };

const MAX_CURSOR_LENGTH = 512;
const MAX_TEXT_LENGTH = 512;
const MAX_SUMMARY_LENGTH = 4_000;
export const MAX_QUESTION_SEARCH_PAGE_ITEMS = 100;
const MAX_QUESTION_SEARCH_AGGREGATES = 100;
const MAX_FACET_COUNT = 1_000_000_000;

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Readonly<Record<string, unknown>>,
  keys: ReadonlyArray<string>,
): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

function boundedText(value: unknown, path: string, maxLength = MAX_TEXT_LENGTH): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maxLength) {
    throw new Error(`${path} must be non-empty text within ${maxLength} characters`);
  }
  return value;
}

function stringList(value: unknown, path: string): ReadonlyArray<string> {
  if (!Array.isArray(value) || value.length > MAX_QUESTION_SEARCH_AGGREGATES) {
    throw new Error(`${path} must be an array`);
  }
  return value.map((item, index) => boundedText(item, `${path}[${index}]`));
}

function decodeRow(value: unknown, path: string): QuestionSearchResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "byline",
      "capabilities",
      "displayId",
      "license",
      "summary",
      "classifications",
      "title",
      "evidence",
    ])
  ) {
    throw new Error(`${path} has an unexpected shape`);
  }
  const displayId = normalizeQuestionIdSyntax(boundedText(value["displayId"], `${path}.displayId`));
  if (displayId === null) {
    throw new Error(`${path}.displayId must be a canonical Question ID`);
  }
  const evidence = decodeBrowseEvidence(value["evidence"], `${path}.evidence`);
  return {
    displayId,
    title: boundedText(value["title"], `${path}.title`),
    summary: boundedText(value["summary"], `${path}.summary`, MAX_SUMMARY_LENGTH),
    byline: stringList(value["byline"], `${path}.byline`),
    classifications: stringList(value["classifications"], `${path}.classifications`),
    capabilities: stringList(value["capabilities"], `${path}.capabilities`),
    license: boundedText(value["license"], `${path}.license`),
    evidence,
  };
}

function decodeBrowseEvidence(value: unknown, path: string): QuestionSearchEvidence {
  if (!isRecord(value) || typeof value["state"] !== "string") {
    throw new Error(`${path} has an unexpected shape`);
  }
  if (value["state"] === "insufficientEvidence") {
    if (!hasExactKeys(value, ["state"])) throw new Error(`${path} has an unexpected shape`);
    return { state: "insufficientEvidence" };
  }
  if (value["state"] !== "available")
    throw new Error(`${path}.state is not a known evidence state`);
  if (
    !hasExactKeys(value, [
      "state",
      "observedCourseCount",
      "independentLearnerObservationCount",
      "difficultyIndex",
      "discriminationIndex",
    ])
  ) {
    throw new Error(`${path} has an unexpected shape`);
  }
  const observedCourseCount = boundedEvidenceCount(
    value["observedCourseCount"],
    `${path}.observedCourseCount`,
  );
  const independentLearnerObservationCount = boundedEvidenceCount(
    value["independentLearnerObservationCount"],
    `${path}.independentLearnerObservationCount`,
  );
  if (observedCourseCount < 2 || independentLearnerObservationCount < observedCourseCount) {
    throw new Error(`${path} must contain comparable evidence from two courses`);
  }
  const difficultyIndex = unitInterval(value["difficultyIndex"], `${path}.difficultyIndex`);
  const discrimination = value["discriminationIndex"];
  const discriminationIndex =
    discrimination === undefined
      ? undefined
      : correlation(discrimination, `${path}.discriminationIndex`);
  return {
    state: "available",
    observedCourseCount,
    independentLearnerObservationCount,
    difficultyIndex,
    discriminationIndex,
  };
}

function boundedEvidenceCount(value: unknown, path: string): number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < 1 ||
    value > MAX_FACET_COUNT
  ) {
    throw new Error(`${path} must be a positive safe integer`);
  }
  return value;
}

function unitInterval(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 1) {
    throw new Error(`${path} must be a finite number from 0 through 1`);
  }
  return value;
}

function correlation(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < -1 || value > 1) {
    throw new Error(`${path} must be a finite correlation from -1 through 1`);
  }
  return value;
}

function decodeAggregate(value: unknown, path: string): QuestionSearchFacetAggregate {
  if (!isRecord(value) || !hasExactKeys(value, ["count", "facet", "value"])) {
    throw new Error(`${path} has an unexpected shape`);
  }
  const facet = value["facet"];
  if (
    facet !== "byline" &&
    facet !== "backend" &&
    facet !== "tag" &&
    facet !== "questionType" &&
    facet !== "classification" &&
    facet !== "capability" &&
    facet !== "license" &&
    facet !== "evidence" &&
    facet !== "usedInMyCourses"
  ) {
    throw new Error(`${path}.facet is not a Question Library facet`);
  }
  const count = value["count"];
  if (
    typeof count !== "number" ||
    !Number.isSafeInteger(count) ||
    count < 0 ||
    count > MAX_FACET_COUNT
  ) {
    throw new Error(`${path}.count must be a non-negative safe integer`);
  }
  return { facet, value: boundedText(value["value"], `${path}.value`), count };
}

/** Strictly decode the live/generated-client result before browser presentation. */
export function decodeQuestionSearchPage(value: unknown): QuestionSearchPage {
  if (!isRecord(value) || !hasExactKeys(value, ["aggregates", "items", "nextCursor"])) {
    throw new Error("Question Library response has an unexpected shape");
  }
  if (
    !Array.isArray(value["items"]) ||
    value["items"].length > MAX_QUESTION_SEARCH_PAGE_ITEMS ||
    !Array.isArray(value["aggregates"]) ||
    value["aggregates"].length > MAX_QUESTION_SEARCH_AGGREGATES
  ) {
    throw new Error("Question Library response arrays are invalid");
  }
  const nextCursor = value["nextCursor"];
  if (
    nextCursor !== null &&
    (typeof nextCursor !== "string" ||
      nextCursor.length === 0 ||
      nextCursor.length > MAX_CURSOR_LENGTH)
  ) {
    throw new Error("Question Library response cursor is invalid");
  }
  return {
    items: value["items"].map((item, index) => decodeRow(item, `items[${index}]`)),
    nextCursor,
    aggregates: value["aggregates"].map((item, index) =>
      decodeAggregate(item, `aggregates[${index}]`),
    ),
  };
}

export const EMPTY_QUESTION_SEARCH_QUERY: QuestionSearchQuery = {
  search: "",
  byline: null,
  backend: null,
  tag: null,
  questionType: null,
  classification: null,
  capability: null,
  license: null,
  evidence: null,
  usedInMyCourses: null,
  authorship: "any",
};

export function normalizeQuestionSearchQuery(query: QuestionSearchQuery): QuestionSearchQuery {
  return {
    search: query.search.trim().replace(/\s+/g, " "),
    byline: query.byline,
    backend: query.backend,
    tag: query.tag,
    questionType: query.questionType,
    classification: query.classification,
    capability: query.capability,
    license: query.license,
    evidence: query.evidence,
    usedInMyCourses: query.usedInMyCourses,
    authorship: query.authorship,
  };
}

/** Fixed-row virtual window keeps DOM work bounded independently of Question Library size. */
export function questionSearchVirtualWindow<T>(
  rows: ReadonlyArray<T>,
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number,
  overscanRows: number,
): Readonly<{ readonly offset: number; readonly rows: ReadonlyArray<T> }> {
  if (rowHeight <= 0 || overscanRows < 0) {
    throw new Error("virtual window dimensions must be positive");
  }
  const first = Math.max(0, Math.floor(Math.max(0, scrollTop) / rowHeight) - overscanRows);
  const count = Math.ceil(Math.max(0, viewportHeight) / rowHeight) + overscanRows * 2;
  return { offset: first * rowHeight, rows: rows.slice(first, first + count) };
}

function rowKey(row: QuestionSearchResult): string {
  return row.displayId;
}

function appendUnique(
  previous: ReadonlyArray<QuestionSearchResult>,
  incoming: ReadonlyArray<QuestionSearchResult>,
): ReadonlyArray<QuestionSearchResult> {
  const keys = new Set(previous.map(rowKey));
  const appended = incoming.filter((row) => {
    const key = rowKey(row);
    if (keys.has(key)) {
      return false;
    }
    keys.add(key);
    return true;
  });
  return [...previous, ...appended];
}

/**
 * Cursor-only session: one request at a time and stale responses cannot append.
 *
 * A replacement query clears its old result rows while loading, but retains the
 * last server-computed aggregates until the replacement response arrives. This
 * keeps an already-selected native facet option present (and therefore selected)
 * through the asynchronous transition. Those retained counts describe the last
 * completed query only; the next completed response replaces them wholesale.
 */
export class QuestionSearchSession {
  #generation = 0;
  #query = EMPTY_QUESTION_SEARCH_QUERY;
  #state: QuestionSearchState = { kind: "loading", rows: [], aggregates: [], nextCursor: null };
  #loading = false;
  #queuedReset = false;

  public constructor(
    private readonly repository: QuestionLibraryRepository,
    private readonly publish: (state: QuestionSearchState) => void,
  ) {}

  public get state(): QuestionSearchState {
    return this.#state;
  }

  public async reset(query: QuestionSearchQuery): Promise<void> {
    this.#generation += 1;
    this.#query = normalizeQuestionSearchQuery(query);
    if (this.#loading) {
      this.#queuedReset = true;
      return;
    }
    await this.loadPage(null, true, this.#generation);
  }

  public async retry(): Promise<void> {
    if (
      this.#state.kind === "error" &&
      this.#state.rows.length > 0 &&
      this.#state.nextCursor !== null
    ) {
      await this.loadPage(this.#state.nextCursor, false, this.#generation);
      return;
    }
    this.#generation += 1;
    if (this.#loading) {
      this.#queuedReset = true;
      return;
    }
    await this.loadPage(null, true, this.#generation);
  }

  public async loadNext(): Promise<void> {
    if (this.#loading || this.#state.kind !== "ready" || this.#state.nextCursor === null) {
      return;
    }
    await this.loadPage(this.#state.nextCursor, false, this.#generation);
  }

  private setState(state: QuestionSearchState): void {
    this.#state = state;
    this.publish(state);
  }

  private async loadPage(
    cursor: string | null,
    replace: boolean,
    generation: number,
  ): Promise<void> {
    if (this.#loading) {
      return;
    }
    this.#loading = true;
    const retainedRows = replace || this.#state.kind === "empty" ? [] : this.#state.rows;
    // Replacement results must not transiently remove native select options.
    // The aggregate values remain server-owned and are replaced, never merged,
    // when the exact replacement query completes.
    const retainedAggregates = this.#state.aggregates;
    const retainedCursor = replace || this.#state.kind === "empty" ? null : this.#state.nextCursor;
    this.setState({
      kind: "loading",
      rows: retainedRows,
      aggregates: retainedAggregates,
      nextCursor: retainedCursor,
    });
    try {
      const page = decodeQuestionSearchPage(await this.repository.search(this.#query, cursor));
      if (generation !== this.#generation) {
        return;
      }
      const rows = replace ? appendUnique([], page.items) : appendUnique(retainedRows, page.items);
      this.setState(
        rows.length === 0
          ? { kind: "empty", aggregates: page.aggregates }
          : { kind: "ready", rows, nextCursor: page.nextCursor, aggregates: page.aggregates },
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
