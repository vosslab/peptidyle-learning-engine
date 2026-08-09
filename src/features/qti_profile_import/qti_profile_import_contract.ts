// qti_profile_import_contract.ts - answer-free browser contract for recognized QTI imports.

import type { DraftQuestionDefinition } from "../../../generated/api/DraftQuestionDefinition";
import type { WorkspaceImportId } from "../../../generated/api/WorkspaceImportId";

export interface QtiProfileDiagnostic {
  readonly code: string;
  readonly location: string;
  readonly detail: string;
}

export type QtiProfileItemStatus = "accepted" | "rejected";

export interface QtiProfileItemReport {
  readonly sourceIdentifier: string;
  readonly title: string | null;
  readonly status: QtiProfileItemStatus;
  readonly diagnostics: ReadonlyArray<QtiProfileDiagnostic>;
  readonly defaults: ReadonlyArray<QtiProfileDiagnostic>;
  readonly warnings: ReadonlyArray<QtiProfileDiagnostic>;
}

export interface QtiProfileImportProgress {
  readonly importId: WorkspaceImportId;
  readonly state: "queued" | "processing";
}

export interface QtiProfileImportFailure {
  readonly importId: WorkspaceImportId;
  readonly state: "failed" | "unsupportedProfile";
  readonly error: string;
}

export interface QtiProfileImportReadyReport {
  readonly importId: WorkspaceImportId;
  readonly state: "ready";
  readonly profileId:
    "canvas-qti-1.2-static-single-choice/v1" | "blackboard-qti-2.1-static-single-choice-pool/v1";
  readonly profileLabel: string;
  readonly profileVersion: string;
  readonly reportRevision: string;
  readonly items: ReadonlyArray<QtiProfileItemReport>;
  readonly pleDefaults: ReadonlyArray<QtiProfileDiagnostic>;
  readonly reviewToken: string;
}

export type QtiProfileImportResponse =
  QtiProfileImportProgress | QtiProfileImportFailure | QtiProfileImportReadyReport;

export interface QtiProfileAcknowledgement {
  readonly reportRevision: string;
  readonly reviewToken: string;
}

export interface QtiProfileConversionResult {
  readonly draft: DraftQuestionDefinition;
  readonly revision: string;
}
