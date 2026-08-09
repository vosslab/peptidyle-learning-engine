// qti_profile_import_model.ts - pure acknowledgement rules for QTI report review.

import type {
  QtiProfileImportResponse,
  QtiProfileImportReadyReport,
  QtiProfileItemReport,
} from "./qti_profile_import_contract";

export interface QtiConversionDraftState {
  /** Strong ETag for the draft currently displayed by the editor. */
  readonly revision: string;
  /** True when replacing the server draft would strand unsaved editor changes. */
  readonly dirty: boolean;
}

export type QtiConversionBlockReason = "reviewIncomplete" | "draftUnavailable" | "draftDirty";

export interface QtiProfileReviewState {
  readonly report: QtiProfileImportReadyReport | null;
  readonly acknowledged: boolean;
  readonly selectedItem: string | null;
}

export const EMPTY_QTI_PROFILE_REVIEW: QtiProfileReviewState = {
  report: null,
  acknowledged: false,
  selectedItem: null,
};

export function receiveQtiProfileReport(
  current: QtiProfileReviewState,
  report: QtiProfileImportReadyReport,
): QtiProfileReviewState {
  const sameReview =
    current.report?.reportRevision === report.reportRevision &&
    current.report.reviewToken === report.reviewToken;
  const selectionStillAccepted = report.items.some(
    (item) => item.status === "accepted" && item.sourceIdentifier === current.selectedItem,
  );
  return {
    report,
    acknowledged: sameReview && current.acknowledged,
    selectedItem: sameReview && selectionStillAccepted ? current.selectedItem : null,
  };
}

export function acknowledgeQtiProfileReport(
  current: QtiProfileReviewState,
  acknowledged: boolean,
): QtiProfileReviewState {
  return current.report === null ? current : { ...current, acknowledged };
}

export function selectQtiProfileItem(
  current: QtiProfileReviewState,
  item: QtiProfileItemReport,
): QtiProfileReviewState {
  if (
    current.report === null ||
    item.status !== "accepted" ||
    !current.report.items.some(
      (candidate) =>
        candidate.status === "accepted" && candidate.sourceIdentifier === item.sourceIdentifier,
    )
  ) {
    return current;
  }
  return { ...current, selectedItem: item.sourceIdentifier };
}

export function canConvertQtiProfileItem(current: QtiProfileReviewState): boolean {
  return current.report !== null && current.acknowledged && current.selectedItem !== null;
}

export function qtiConversionBlockReason(
  review: QtiProfileReviewState,
  draft: QtiConversionDraftState | null,
): QtiConversionBlockReason | null {
  if (!canConvertQtiProfileItem(review)) return "reviewIncomplete";
  if (draft === null) return "draftUnavailable";
  return draft.dirty ? "draftDirty" : null;
}

/** Ambiguous upload failures retry the exact import; terminal responses start a fresh import. */
export function shouldRetrySameQtiImport(
  activeImport: string | null,
  response: QtiProfileImportResponse | null,
): boolean {
  return activeImport !== null && response === null;
}

/** A committed replacement stays locked until the converted draft has been loaded successfully. */
export function shouldKeepQtiReplacementLocked(
  conversionCommitted: boolean,
  replacementLoaded: boolean,
): boolean {
  return conversionCommitted && !replacementLoaded;
}
