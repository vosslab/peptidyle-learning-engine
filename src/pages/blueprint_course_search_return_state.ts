// One document-local return view; never a result or authorization cache.
import type { AuthenticatedSession } from "../api/contracts";
import type {
  BlueprintCourseClassificationSearch,
  BlueprintCourseListSort,
} from "../api/blueprint_course";
import type { RecordPageSize } from "../components/record_list/record_page_controls";

export interface BlueprintSearchSnapshot {
  readonly query: string;
  readonly promotedOnly: boolean;
  readonly classification: BlueprintCourseClassificationSearch;
  readonly classificationDescription: string;
  readonly sort: BlueprintCourseListSort;
  readonly tag: string;
}

export interface BlueprintSearchReturnState {
  readonly draft: BlueprintSearchSnapshot;
  readonly submitted: BlueprintSearchSnapshot;
  /** The current opaque cursor is refetched directly; earlier pages are never replayed. */
  readonly currentCursor: string | undefined;
  /** Retained cursors preserve native Previous navigation after a direct return. */
  readonly previousCursors: ReadonlyArray<string | undefined>;
  readonly pageSize: RecordPageSize;
  readonly scrollY: number;
  readonly linkKey: string;
}

export interface BlueprintSearchHistoryState {
  readonly accountId: string;
  readonly token: string;
  readonly state: BlueprintSearchReturnState;
}

const BLUEPRINT_SEARCH_RETURN_TOKEN_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;
let pending: {
  readonly session: AuthenticatedSession;
  readonly token: string;
  readonly state: BlueprintSearchReturnState;
} | null = null;

/** Generate an opaque route token; it never identifies an Account or Blueprint Course. */
export function createBlueprintSearchReturnToken(): string {
  return crypto.randomUUID();
}

/** Reject malformed route input before it can select an in-memory return view. */
export function parseBlueprintSearchReturnToken(value: unknown): string | null {
  return typeof value === "string" && BLUEPRINT_SEARCH_RETURN_TOKEN_PATTERN.test(value)
    ? value
    : null;
}

/** Save only immediately before same-tab result navigation, as in Question Library. */
export function saveBlueprintSearchReturnState(
  session: AuthenticatedSession,
  token: string,
  state: BlueprintSearchReturnState,
): void {
  if (parseBlueprintSearchReturnToken(token) === null) return;
  pending = { session, token, state };
}

/** Store the temporary return snapshot on its source history entry for reload and Back. */
export function blueprintSearchHistoryState(
  session: AuthenticatedSession,
  token: string,
  state: BlueprintSearchReturnState,
): BlueprintSearchHistoryState | null {
  if (parseBlueprintSearchReturnToken(token) === null) return null;
  return { accountId: session.account.id, token, state };
}

/**
 * A search return token on the detail URL is the way back to that public result page.
 * Without a token, ownership selects My Blueprint Courses and other Instructor access
 * selects public search.
 */
export function blueprintDetailCollectionLink(
  readAccess: string | undefined,
  returnToken: string | null,
): { readonly href: string; readonly label: string } {
  const token = parseBlueprintSearchReturnToken(returnToken);
  if (token !== null) {
    return {
      href: "/blueprint-courses/search/public",
      label: "Return to Public Blueprint Courses",
    };
  }
  if (readAccess === "active_instructor") {
    return {
      href: "/blueprint-courses/search/public",
      label: "Return to Public Blueprint Courses",
    };
  }
  return { href: "/blueprint-courses", label: "Return to My Blueprint Courses" };
}

/** Exact session identity and single consumption prevent cross-Account restoration. */
export function takeBlueprintSearchReturnState(
  session: AuthenticatedSession,
  token: unknown,
  historyState?: unknown,
): BlueprintSearchReturnState | null {
  const parsedToken = parseBlueprintSearchReturnToken(token);
  const pendingState =
    pending?.session === session && (parsedToken === null || pending.token === parsedToken)
      ? pending.state
      : null;
  pending = null;
  if (pendingState !== null) return pendingState;
  if (!isRecord(historyState)) return null;
  if (historyState.accountId !== session.account.id) return null;
  if (parseBlueprintSearchReturnToken(historyState.token) === null) return null;
  if (parsedToken !== null && historyState.token !== parsedToken) return null;
  return isReturnState(historyState.state) ? historyState.state : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isSnapshot(value: unknown): value is BlueprintSearchSnapshot {
  if (!isRecord(value) || !isRecord(value.classification)) return false;
  const classification = value.classification;
  const nullableString = (item: unknown): boolean => item === null || typeof item === "string";
  return (
    typeof value.query === "string" &&
    typeof value.promotedOnly === "boolean" &&
    nullableString(classification.disciplineUuid) &&
    nullableString(classification.subjectUuid) &&
    nullableString(classification.topicUuid) &&
    nullableString(classification.subtopicUuid) &&
    typeof classification.crossDiscipline === "boolean" &&
    typeof value.classificationDescription === "string" &&
    (value.sort === "name" || value.sort === "adoptions" || value.sort === "students") &&
    typeof value.tag === "string"
  );
}

function isReturnState(value: unknown): value is BlueprintSearchReturnState {
  if (!isRecord(value) || !isSnapshot(value.draft) || !isSnapshot(value.submitted)) return false;
  const cursors = value.previousCursors;
  const validCursor = (item: unknown): boolean => item === undefined || typeof item === "string";
  return (
    validCursor(value.currentCursor) &&
    Array.isArray(cursors) &&
    cursors.every(validCursor) &&
    (value.pageSize === 50 || value.pageSize === 100 || value.pageSize === 250) &&
    typeof value.scrollY === "number" &&
    Number.isFinite(value.scrollY) &&
    typeof value.linkKey === "string"
  );
}
