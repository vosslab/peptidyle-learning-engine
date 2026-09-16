// One document-local return view; never a result or authorization cache.
import type { AuthenticatedSession } from "../api/contracts";
import type { BlueprintCourseClassificationSearch } from "../api/blueprint_course";

export interface BlueprintSearchSnapshot {
  readonly query: string;
  readonly promotedOnly: boolean;
  readonly classification: BlueprintCourseClassificationSearch;
  readonly classificationDescription: string;
}

export interface BlueprintSearchReturnState {
  readonly draft: BlueprintSearchSnapshot;
  readonly submitted: BlueprintSearchSnapshot;
  readonly pages: number;
  readonly scrollY: number;
  readonly linkKey: string;
}

export const BLUEPRINT_SEARCH_RETURN_PARAMETER = "blueprintReturn";
let pending: {
  readonly session: AuthenticatedSession;
  readonly token: string;
  readonly state: BlueprintSearchReturnState;
} | null = null;

/** Save only immediately before same-tab result navigation, as in Question Library. */
export function saveBlueprintSearchReturnState(
  session: AuthenticatedSession,
  state: BlueprintSearchReturnState,
): string {
  const token = crypto.randomUUID();
  pending = { session, token, state };
  return token;
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
