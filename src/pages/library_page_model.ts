// library_page_model.ts - bounded, transport-validated Question Library browse state.

import { normalizeQuestionIdSyntax } from "../question_id";
import { EMPTY_LIBRARY_CLASSIFICATION_FILTER, libraryClassificationFilter,
  type LibraryClassificationFilter } from "../api/library_classification_filter";
import type { QuestionFormat } from "../../generated/api/QuestionFormat";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import { MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES } from "../../generated/api/MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES";
import { MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS } from "../../generated/api/MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS";
import { MAX_QUESTION_SEARCH_TAG_FACETS } from "../../generated/api/MAX_QUESTION_SEARCH_TAG_FACETS";
import type { AuthenticatedSession } from "../api/contracts";
import { MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS } from "../api/decoders/question_type_facets";
import {
  MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
  MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS,
  QUESTION_BACKENDS,
  decodeQuestionRevisionReference,
} from "../api/decoders/shared";

/** A browser-safe current Question Library record. */
export interface QuestionLibraryBrowseRow {
  /** Copy/paste identity used by instructors and the browser deduplication key. */
  readonly displayId: string;
  /** Exact immutable revision selected by this browse result. */
  readonly questionRevision: QuestionRevisionReference;
  readonly questionTitle: string;
  readonly summary: string;
  /** Immutable source representation, without source location or content.
   * Retained Assessment picker candidates have no format projection, so they
   * explicitly retain unavailable metadata rather than guessing from backend. */
  readonly questionFormat: QuestionFormat | null;
  /** Reviewed Question Author display names; never Account or Question Owner identity. */
  readonly authorNames: ReadonlyArray<string>;
  readonly capabilities: ReadonlyArray<string>;
  readonly questionLicense: string | null;
  /** Server-disclosed learning evidence for this exact immutable publication. */
  readonly evidence: QuestionLibraryBrowseEvidence;
}

/**
 * A presentation-ready, answer-free view of the server-owned discovery evidence.
 *
 * The browser intentionally receives no quality contribution. Until the
 * Question Statistics release boundary exists, availability stays explicitly
 * unavailable and neutral in relevance-ranked search.
 */
export type QuestionLibraryBrowseEvidence = { readonly state: "unavailable" };

/** Server-computed count for the exact active query; never derived from loaded rows. */
export interface QuestionLibraryBrowseFacetAggregate {
  readonly facet:
    | "authorName"
    | "backend"
    | "tag"
    | "subject"
    | "topic"
    | "questionType"
    | "capability"
    | "questionLicense"
    | "usedInMyCourses";
  readonly value: string;
  readonly count: number;
}

export interface QuestionLibraryBrowseQuery extends LibraryClassificationFilter {
  readonly search: string;
  readonly authorName: string | null;
  readonly backend: string | null;
  readonly tag: string | null;
  readonly subjects: ReadonlyArray<string>;
  readonly topics: ReadonlyArray<string>;
  readonly questionType: string | null;
  readonly capability: string | null;
  readonly questionLicense: string | null;
  readonly usedInMyCourses: string | null;
  /** Closed server-resolved authorship scope; browser rows never carry Account identity. */
  readonly authorship: QuestionSearchAuthorship;
}

/** Honest notice that a free-form facet group is only the bounded leading set. */
export interface QuestionLibraryFacetTruncation {
  readonly authorNames: boolean;
  readonly tags: boolean;
  readonly subjects: boolean;
  readonly topics: boolean;
}

export const NO_QUESTION_LIBRARY_FACET_TRUNCATION: QuestionLibraryFacetTruncation = {
  authorNames: false,
  tags: false,
  subjects: false,
  topics: false,
};

export interface QuestionLibraryBrowsePage {
  readonly items: ReadonlyArray<QuestionLibraryBrowseRow>;
  readonly nextCursor: string | null;
  readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
  readonly facetTruncation: QuestionLibraryFacetTruncation;
}

/**
 * The production Question Library repository adapts to this narrow boundary. A hostile result is
 * intentional: this module owns browser-side validation before any row reaches JSX.
 */
export interface QuestionLibraryBrowseRepository {
  readonly search: (query: QuestionLibraryBrowseQuery, cursor: string | null) => Promise<unknown>;
}

export type QuestionLibraryBrowseState =
  | {
      readonly kind: "initial";
      readonly rows: readonly [];
      readonly aggregates: readonly [];
      readonly nextCursor: null;
      readonly facetTruncation: QuestionLibraryFacetTruncation;
    }
  | {
      readonly kind: "loading";
      readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
      readonly nextCursor: string | null;
      readonly facetTruncation: QuestionLibraryFacetTruncation;
    }
  | {
      readonly kind: "ready";
      readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      readonly nextCursor: string | null;
      readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
      readonly facetTruncation: QuestionLibraryFacetTruncation;
    }
  | {
      readonly kind: "empty";
      readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
      readonly facetTruncation: QuestionLibraryFacetTruncation;
    }
  | {
      readonly kind: "error";
      readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
      readonly aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
      readonly nextCursor: string | null;
      readonly facetTruncation: QuestionLibraryFacetTruncation;
    };

/**
 * One in-memory return snapshot for the Library -> Question -> Library path.
 *
 * This deliberately lasts only for the current browser document. It is not a
 * second persistence channel for search preferences, nor is it visible to a
 * Question detail route. The saved page is the last server-validated browse
 * result, so a return can restore an exact position even when it was reached
 * after loading more than the first cursor page.
 */
export interface QuestionLibraryReturnState {
  readonly token: string;
  readonly sessionScope: AuthenticatedSession;
  readonly origin: "search" | "browse";
  readonly query: QuestionLibraryBrowseQuery;
  readonly browseState: Extract<QuestionLibraryBrowseState, { readonly kind: "ready" }>;
  readonly scrollTop: number;
}

let pendingQuestionLibraryReturnState: QuestionLibraryReturnState | null = null;

export const QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER = "libraryReturn";

const QUESTION_LIBRARY_RETURN_TOKEN_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;

/** Generate an opaque route token; it never names an Account, Question, or query. */
export function createQuestionLibraryReturnToken(): string {
  return crypto.randomUUID();
}

/** Reject malformed route input before it can select an in-memory return view. */
export function parseQuestionLibraryReturnToken(value: unknown): string | null {
  return typeof value === "string" && QUESTION_LIBRARY_RETURN_TOKEN_PATTERN.test(value)
    ? value
    : null;
}

export function questionLibraryReturnPath(token: string): string {
  const origin =
    pendingQuestionLibraryReturnState?.token === token
      ? pendingQuestionLibraryReturnState.origin
      : "search";
  const pathname = origin === "browse" ? "/library/browse" : "/library";
  return `${pathname}?${new URLSearchParams({ [QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER]: token }).toString()}`;
}

/** Save the current Library view only immediately before opening a Question. */
export function saveQuestionLibraryReturnState(
  sessionScope: AuthenticatedSession,
  origin: "search" | "browse",
  token: string,
  query: QuestionLibraryBrowseQuery,
  browseState: QuestionLibraryBrowseState,
  scrollTop: number,
): void {
  if (parseQuestionLibraryReturnToken(token) === null) return;
  pendingQuestionLibraryReturnState = null;
  const retainedBrowseState = retainedQuestionLibraryReturnBrowseState(browseState);
  if (retainedBrowseState === null) return;
  pendingQuestionLibraryReturnState = {
    token,
    sessionScope,
    origin,
    query: normalizeQuestionLibraryBrowseQuery(query),
    browseState: retainedBrowseState,
    scrollTop: Number.isFinite(scrollTop) ? Math.max(0, scrollTop) : 0,
  };
}

/** Consume the single pending view on every Library mount, whether or not it matches. */
export function takeQuestionLibraryReturnState(
  sessionScope: AuthenticatedSession,
  token: string | null,
): QuestionLibraryReturnState | null {
  const saved =
    token !== null &&
    pendingQuestionLibraryReturnState?.token === token &&
    pendingQuestionLibraryReturnState.sessionScope === sessionScope
      ? pendingQuestionLibraryReturnState
      : null;
  pendingQuestionLibraryReturnState = null;
  return saved;
}

function retainedQuestionLibraryReturnBrowseState(
  state: QuestionLibraryBrowseState,
): Extract<QuestionLibraryBrowseState, { readonly kind: "ready" }> | null {
  if (state.kind === "ready") return state;
  if ((state.kind !== "loading" && state.kind !== "error") || state.rows.length === 0) {
    return null;
  }
  return {
    kind: "ready",
    rows: state.rows,
    aggregates: state.aggregates,
    nextCursor: state.nextCursor,
    facetTruncation: state.facetTruncation,
  };
}

/** Keep a restored virtual-list position inside the current rendered scroll range. */
export function clampQuestionLibraryReturnScrollTop(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
): number {
  return Math.min(
    Math.max(0, Number.isFinite(scrollTop) ? scrollTop : 0),
    Math.max(0, scrollHeight - clientHeight),
  );
}

const MAX_TEXT_LENGTH = 512;
const MAX_SUMMARY_LENGTH = 4_000;
export const MAX_QUESTION_LIBRARY_BROWSE_PAGE_ITEMS = 100;
const MAX_QUESTION_LIBRARY_ROW_TEXT_ITEMS = 100;
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

function decodeNullableQuestionLicense(value: unknown, path: string): string | null {
  if (value === null) return null;
  if (value !== "CC0-1.0" && value !== "CC-BY-4.0" && value !== "CC-BY-SA-4.0") {
    throw new Error(`${path} must be an exact Question License or null`);
  }
  return value;
}

function decodeQuestionFormat(value: unknown, path: string): QuestionFormat {
  if (
    value !== "pleQuestionJson" &&
    value !== "webworkPg" &&
    value !== "webworkPgml" &&
    value !== "imathas"
  ) {
    throw new Error(`${path} must be an exact published Question Format`);
  }
  return value;
}

function stringList(value: unknown, path: string): ReadonlyArray<string> {
  if (!Array.isArray(value) || value.length > MAX_QUESTION_LIBRARY_ROW_TEXT_ITEMS) {
    throw new Error(`${path} must be an array`);
  }
  return value.map((item, index) => boundedText(item, `${path}[${index}]`));
}

function decodeRow(value: unknown, path: string): QuestionLibraryBrowseRow {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "authorNames",
      "capabilities",
      "displayId",
      "questionLicense",
      "questionFormat",
      "questionRevision",
      "summary",
      "questionTitle",
      "evidence",
    ])
  ) {
    throw new Error(`${path} has an unexpected shape`);
  }
  const rawDisplayId = boundedText(value["displayId"], `${path}.displayId`);
  const displayId = normalizeQuestionIdSyntax(rawDisplayId);
  if (displayId === null || displayId !== rawDisplayId) {
    throw new Error(`${path}.displayId must be a canonical Question ID`);
  }
  const evidence = decodeBrowseEvidence(value["evidence"], `${path}.evidence`);
  const questionRevision = decodeQuestionRevisionReference(
    value["questionRevision"],
    `${path}.questionRevision`,
    true,
  );
  if (questionRevision.questionId !== displayId) {
    throw new Error(`${path}.questionRevision.questionId must match displayId`);
  }
  return {
    displayId,
    questionRevision,
    questionTitle: boundedText(value["questionTitle"], `${path}.questionTitle`),
    summary: boundedText(value["summary"], `${path}.summary`, MAX_SUMMARY_LENGTH),
    questionFormat: decodeQuestionFormat(value["questionFormat"], `${path}.questionFormat`),
    authorNames: stringList(value["authorNames"], `${path}.authorNames`),
    capabilities: stringList(value["capabilities"], `${path}.capabilities`),
    questionLicense: decodeNullableQuestionLicense(
      value["questionLicense"],
      `${path}.questionLicense`,
    ),
    evidence,
  };
}

function decodeBrowseEvidence(value: unknown, path: string): QuestionLibraryBrowseEvidence {
  if (!isRecord(value) || value["state"] !== "unavailable") {
    throw new Error(`${path} has an unexpected shape`);
  }
  if (!hasExactKeys(value, ["state"])) throw new Error(`${path} has an unexpected shape`);
  return { state: "unavailable" };
}

function decodeAggregate(value: unknown, path: string): QuestionLibraryBrowseFacetAggregate {
  if (!isRecord(value) || !hasExactKeys(value, ["count", "facet", "value"])) {
    throw new Error(`${path} has an unexpected shape`);
  }
  const facet = value["facet"];
  if (
    facet !== "authorName" &&
    facet !== "backend" &&
    facet !== "tag" &&
    facet !== "subject" &&
    facet !== "topic" &&
    facet !== "questionType" &&
    facet !== "capability" &&
    facet !== "questionLicense" &&
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

function decodeFacetTruncation(value: unknown): QuestionLibraryFacetTruncation {
  if (!isRecord(value) || !hasExactKeys(value, ["authorNames", "tags", "subjects", "topics"])) {
    throw new Error("Question Library facet truncation has an unexpected shape");
  }
  const authorNames = value["authorNames"];
  const tags = value["tags"];
  const subjects = value["subjects"];
  const topics = value["topics"];
  if (
    typeof authorNames !== "boolean" ||
    typeof tags !== "boolean" ||
    typeof subjects !== "boolean" ||
    typeof topics !== "boolean"
  ) {
    throw new Error("Question Library facet truncation fields must be boolean");
  }
  return {
    authorNames,
    tags,
    subjects,
    topics,
  };
}

function validateAggregateGroupCaps(
  aggregates: ReadonlyArray<QuestionLibraryBrowseFacetAggregate>,
): void {
  const caps: Readonly<Record<QuestionLibraryBrowseFacetAggregate["facet"], number>> = {
    authorName: MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS,
    backend: QUESTION_BACKENDS.length,
    tag: MAX_QUESTION_SEARCH_TAG_FACETS,
    subject: MAX_QUESTION_SEARCH_TAG_FACETS,
    topic: MAX_QUESTION_SEARCH_TAG_FACETS,
    questionType: MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
    capability: MAX_QUESTION_SEARCH_CAPABILITY_FACETS,
    questionLicense: MAX_QUESTION_SEARCH_QUESTION_LICENSE_FACETS,
    usedInMyCourses: 1,
  };
  const counts = new Map<QuestionLibraryBrowseFacetAggregate["facet"], number>();
  for (const aggregate of aggregates) {
    const next = (counts.get(aggregate.facet) ?? 0) + 1;
    if (next > caps[aggregate.facet]) {
      throw new Error(`Question Library ${aggregate.facet} aggregate group is too large`);
    }
    counts.set(aggregate.facet, next);
  }
}

/** Strictly decode the live/generated-client result before browser presentation. */
export function decodeQuestionLibraryBrowsePage(value: unknown): QuestionLibraryBrowsePage {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["aggregates", "facetTruncation", "items", "nextCursor"])
  ) {
    throw new Error("Question Library response has an unexpected shape");
  }
  if (
    !Array.isArray(value["items"]) ||
    value["items"].length > MAX_QUESTION_LIBRARY_BROWSE_PAGE_ITEMS ||
    !Array.isArray(value["aggregates"])
  ) {
    throw new Error("Question Library response arrays are invalid");
  }
  const nextCursor = value["nextCursor"];
  if (
    nextCursor !== null &&
    (typeof nextCursor !== "string" ||
      nextCursor.length === 0 ||
      nextCursor.length > MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES)
  ) {
    throw new Error("Question Library response cursor is invalid");
  }
  const aggregates = value["aggregates"].map((item, index) =>
    decodeAggregate(item, `aggregates[${index}]`),
  );
  validateAggregateGroupCaps(aggregates);
  return {
    items: value["items"].map((item, index) => decodeRow(item, `items[${index}]`)),
    nextCursor,
    aggregates,
    facetTruncation: decodeFacetTruncation(value["facetTruncation"]),
  };
}

export const EMPTY_QUESTION_LIBRARY_BROWSE_QUERY: QuestionLibraryBrowseQuery = {
  ...EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  search: "",
  authorName: null,
  backend: null,
  tag: null,
  subjects: [],
  topics: [],
  questionType: null,
  capability: null,
  questionLicense: null,
  usedInMyCourses: null,
  authorship: "any",
};

export function normalizeQuestionLibraryBrowseQuery(
  query: QuestionLibraryBrowseQuery,
): QuestionLibraryBrowseQuery {
  return {
    ...libraryClassificationFilter(query),
    search: query.search.trim().replace(/\s+/g, " "),
    authorName: query.authorName,
    backend: query.backend,
    tag: query.tag,
    subjects: query.subjects.map((subject) => subject.trim().replace(/\s+/g, " ")),
    topics: query.topics.map((topic) => topic.trim().replace(/\s+/g, " ")),
    questionType: query.questionType,
    capability: query.capability,
    questionLicense: query.questionLicense,
    usedInMyCourses: query.usedInMyCourses,
    authorship: query.authorship,
  };
}

/** Fixed-row virtual window keeps DOM work bounded independently of Question Library size. */
export function questionLibraryBrowseVirtualWindow<T>(
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

function rowKey(row: QuestionLibraryBrowseRow): string {
  return row.displayId;
}

function appendUnique(
  previous: ReadonlyArray<QuestionLibraryBrowseRow>,
  incoming: ReadonlyArray<QuestionLibraryBrowseRow>,
): ReadonlyArray<QuestionLibraryBrowseRow> {
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
export class QuestionLibraryBrowseSession {
  #generation = 0;
  #query = EMPTY_QUESTION_LIBRARY_BROWSE_QUERY;
  #state: QuestionLibraryBrowseState = {
    kind: "initial",
    rows: [],
    aggregates: [],
    nextCursor: null,
    facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  };
  #loading = false;
  #queuedReset = false;

  public constructor(
    private readonly repository: QuestionLibraryBrowseRepository,
    private readonly publish: (state: QuestionLibraryBrowseState) => void,
  ) {}

  public get state(): QuestionLibraryBrowseState {
    return this.#state;
  }

  /** Rehydrate the server-validated page retained for an immediate detail return. */
  public restore(
    query: QuestionLibraryBrowseQuery,
    state: Extract<QuestionLibraryBrowseState, { readonly kind: "ready" }>,
  ): void {
    this.#generation += 1;
    this.#queuedReset = false;
    this.#loading = false;
    this.#query = normalizeQuestionLibraryBrowseQuery(query);
    this.setState(state);
  }

  public async reset(query: QuestionLibraryBrowseQuery): Promise<void> {
    this.#generation += 1;
    this.#query = normalizeQuestionLibraryBrowseQuery(query);
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

  private setState(state: QuestionLibraryBrowseState): void {
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
    const retainedFacetTruncation = this.#state.facetTruncation;
    const retainedCursor = replace || this.#state.kind === "empty" ? null : this.#state.nextCursor;
    this.setState({
      kind: "loading",
      rows: retainedRows,
      aggregates: retainedAggregates,
      nextCursor: retainedCursor,
      facetTruncation: retainedFacetTruncation,
    });
    try {
      const page = decodeQuestionLibraryBrowsePage(
        await this.repository.search(this.#query, cursor),
      );
      if (generation !== this.#generation) {
        return;
      }
      const rows = replace ? appendUnique([], page.items) : appendUnique(retainedRows, page.items);
      this.setState(
        rows.length === 0
          ? {
              kind: "empty",
              aggregates: page.aggregates,
              facetTruncation: page.facetTruncation,
            }
          : {
              kind: "ready",
              rows,
              nextCursor: page.nextCursor,
              aggregates: page.aggregates,
              facetTruncation: page.facetTruncation,
            },
      );
    } catch {
      if (generation === this.#generation) {
        this.setState({
          kind: "error",
          rows: retainedRows,
          aggregates: retainedAggregates,
          nextCursor: retainedCursor,
          facetTruncation: retainedFacetTruncation,
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
