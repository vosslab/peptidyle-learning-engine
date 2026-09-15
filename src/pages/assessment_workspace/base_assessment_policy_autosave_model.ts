import type { SaveBaseAssessmentPolicyInput } from "../../api/assessment_release";

export type BaseAssessmentPolicyPersistence =
  "saved" | "saving" | "invalid" | "rejected" | "failed" | "conflict";

export interface BaseAssessmentPolicyAutosaveState {
  readonly persistence: BaseAssessmentPolicyPersistence;
  readonly draft: SaveBaseAssessmentPolicyInput;
  readonly draftSeq: number;
  readonly inFlightSeq: number | undefined;
  readonly inFlightInput: SaveBaseAssessmentPolicyInput | undefined;
  readonly lastAccepted: SaveBaseAssessmentPolicyInput;
  readonly pending: SaveBaseAssessmentPolicyInput | undefined;
  readonly errors: ReadonlyArray<string>;
}

export function allBaseAssessmentPolicyEditsPersisted(
  state: BaseAssessmentPolicyAutosaveState,
): boolean {
  return state.persistence === "saved";
}

/** Starts the initial request or retains one newer valid draft behind the active request. */
export function baseAssessmentPolicyDraftChanged(
  state: BaseAssessmentPolicyAutosaveState,
  draft: SaveBaseAssessmentPolicyInput,
  valid: boolean,
  requestNow = true,
): BaseAssessmentPolicyAutosaveState {
  const draftSeq = state.draftSeq + 1;
  if (!valid) {
    return {
      ...state,
      draft,
      draftSeq,
      pending: undefined,
      persistence: "invalid",
      errors: [],
    };
  }
  if (state.inFlightSeq !== undefined) {
    return { ...state, draft, draftSeq, pending: draft, persistence: "saving", errors: [] };
  }
  if (!requestNow) {
    return { ...state, draft, draftSeq, pending: undefined, persistence: "saving", errors: [] };
  }
  return {
    ...state,
    draft,
    draftSeq,
    inFlightSeq: draftSeq,
    inFlightInput: draft,
    pending: undefined,
    persistence: "saving",
    errors: [],
  };
}

/** Returns the one request that the page may issue for the current state. */
export function baseAssessmentPolicyRequest(
  state: BaseAssessmentPolicyAutosaveState,
): { readonly input: SaveBaseAssessmentPolicyInput; readonly seq: number } | undefined {
  if (state.inFlightSeq === undefined || state.inFlightInput === undefined) return undefined;
  return { input: state.inFlightInput, seq: state.inFlightSeq };
}

/** Records an accepted save without allowing an older completion to replace a newer draft. */
export function baseAssessmentPolicySaveSucceeded(
  state: BaseAssessmentPolicyAutosaveState,
  seq: number,
  accepted: SaveBaseAssessmentPolicyInput,
  draftIsValid: boolean,
): BaseAssessmentPolicyAutosaveState {
  if (state.inFlightSeq !== seq) return state;
  const completed = {
    ...state,
    inFlightSeq: undefined,
    inFlightInput: undefined,
    lastAccepted: accepted,
  };
  if (!draftIsValid) return { ...completed, pending: undefined, persistence: "invalid" };
  if (completed.pending !== undefined) {
    return {
      ...completed,
      inFlightSeq: completed.draftSeq,
      inFlightInput: completed.pending,
      pending: undefined,
      persistence: "saving",
      errors: [],
    };
  }
  if (seq === completed.draftSeq) {
    return { ...completed, draft: accepted, persistence: "saved", errors: [] };
  }
  return { ...completed, persistence: "saving" };
}

export function baseAssessmentPolicySaveError(
  state: BaseAssessmentPolicyAutosaveState,
  seq: number,
  persistence: Exclude<BaseAssessmentPolicyPersistence, "saved" | "saving" | "invalid">,
): BaseAssessmentPolicyAutosaveState {
  if (state.inFlightSeq !== seq) return state;
  return {
    ...state,
    persistence,
    inFlightSeq: undefined,
    inFlightInput: undefined,
    pending: undefined,
  };
}

/** Retries the visible draft only after a failed request. */
export function baseAssessmentPolicyRetry(
  state: BaseAssessmentPolicyAutosaveState,
  valid: boolean,
): BaseAssessmentPolicyAutosaveState {
  if (!valid) return { ...state, persistence: "invalid" };
  if (state.inFlightSeq !== undefined) return state;
  return {
    ...state,
    inFlightSeq: state.draftSeq,
    inFlightInput: state.draft,
    pending: undefined,
    persistence: "saving",
    errors: [],
  };
}

/** Reload discards the local draft deliberately after an Edit Number conflict. */
export function baseAssessmentPolicyReloaded(
  state: BaseAssessmentPolicyAutosaveState,
  accepted: SaveBaseAssessmentPolicyInput,
): BaseAssessmentPolicyAutosaveState {
  return {
    ...state,
    draft: accepted,
    lastAccepted: accepted,
    inFlightSeq: undefined,
    inFlightInput: undefined,
    pending: undefined,
    persistence: "saved",
    errors: [],
  };
}

export function createBaseAssessmentPolicyAutosaveState(
  draft: SaveBaseAssessmentPolicyInput,
): BaseAssessmentPolicyAutosaveState {
  return {
    persistence: "saved",
    draft,
    draftSeq: 0,
    inFlightSeq: undefined,
    inFlightInput: undefined,
    lastAccepted: draft,
    pending: undefined,
    errors: [],
  };
}
