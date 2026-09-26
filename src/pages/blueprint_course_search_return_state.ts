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

export const BLUEPRINT_SEARCH_RETURN_PARAMETER = "blueprintReturn";
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

/** The same token belongs in the source history entry and the detail route href. */
export function blueprintSearchReturnPath(token: string): string {
  return `/blueprint-courses/search/public?${new URLSearchParams({
    [BLUEPRINT_SEARCH_RETURN_PARAMETER]: token,
  }).toString()}`;
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
      href: blueprintSearchReturnPath(token),
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
  token: string | null,
): BlueprintSearchReturnState | null {
  const state = pending?.session === session && pending.token === token ? pending.state : null;
  pending = null;
  return state;
}
