// Browser-owned contract for B2 curriculum adoption and import maintenance.

import type { AlphaInstantiationCompleted } from "../../generated/api/AlphaInstantiationCompleted";
import type { AlphaInstantiationPreviewRequest } from "../../generated/api/AlphaInstantiationPreviewRequest";
import type { AlphaInstantiationPreviewView } from "../../generated/api/AlphaInstantiationPreviewView";
import type { AssignmentFastForwardCompleted } from "../../generated/api/AssignmentFastForwardCompleted";
import type { AssignmentFastForwardPreviewRequest } from "../../generated/api/AssignmentFastForwardPreviewRequest";
import type { AssignmentFastForwardPreviewView } from "../../generated/api/AssignmentFastForwardPreviewView";
import type { BlueprintInstantiationCompleted } from "../../generated/api/BlueprintInstantiationCompleted";
import type { BlueprintInstantiationPreviewRequest } from "../../generated/api/BlueprintInstantiationPreviewRequest";
import type { BlueprintInstantiationPreviewView } from "../../generated/api/BlueprintInstantiationPreviewView";
import type { CourseRolloverCompleted } from "../../generated/api/CourseRolloverCompleted";
import type { CourseRolloverPreviewRequest } from "../../generated/api/CourseRolloverPreviewRequest";
import type { CourseRolloverPreviewView } from "../../generated/api/CourseRolloverPreviewView";
import type { CourseReference } from "../../generated/api/CourseReference";
import type { CourseTermShiftCompleted } from "../../generated/api/CourseTermShiftCompleted";
import type { CourseTermShiftPreviewOutcome } from "../../generated/api/CourseTermShiftPreviewOutcome";
import type { CourseTermShiftPreviewRequest } from "../../generated/api/CourseTermShiftPreviewRequest";
import type { CurriculumAdoptionReconciliationResult } from "../../generated/api/CurriculumAdoptionReconciliationResult";
import type { CurriculumAdoptionReceiptBinding } from "../../generated/api/CurriculumAdoptionReceiptBinding";
import type { CurriculumCourseImportView } from "../../generated/api/CurriculumCourseImportView";
import type { ForkAlphaCompleted } from "../../generated/api/ForkAlphaCompleted";
import type { ForkAlphaPreviewRequest } from "../../generated/api/ForkAlphaPreviewRequest";
import type { ForkAlphaPreviewView } from "../../generated/api/ForkAlphaPreviewView";
import type { ReconcileCurriculumAdoptionCommand } from "../../generated/api/ReconcileCurriculumAdoptionCommand";
import type { SourceDerivedAssignmentCompleted } from "../../generated/api/SourceDerivedAssignmentCompleted";
import type { SourceDerivedAssignmentPreviewRequest } from "../../generated/api/SourceDerivedAssignmentPreviewRequest";
import type { SourceDerivedAssignmentPreviewView } from "../../generated/api/SourceDerivedAssignmentPreviewView";

/** Opaque key retained for an idempotent completed-operation retry. */
export type CurriculumAdoptionIdempotencyKey = string;

/** Only the eligible branch contains the witness accepted by term-shift apply. */
export type EligibleCourseTermShiftPreview = Extract<
  CourseTermShiftPreviewOutcome,
  { kind: "eligible" }
>;

/** Fast-forward apply accepts only the decision that the server can commit. */
export type EligibleAssignmentFastForwardPreview = Omit<
  AssignmentFastForwardPreviewView,
  "decision"
> & {
  readonly decision: Extract<AssignmentFastForwardPreviewView["decision"], { kind: "eligible" }>;
};

/** Browser command surface for the dedicated B2 adoption aggregate. */
export interface CurriculumAdoptionClient {
  readonly previewForkAlpha: (request: ForkAlphaPreviewRequest) => Promise<ForkAlphaPreviewView>;
  readonly applyForkAlpha: (
    preview: ForkAlphaPreviewView,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<ForkAlphaCompleted>;
  readonly previewBlueprintInstantiation: (
    request: BlueprintInstantiationPreviewRequest,
  ) => Promise<BlueprintInstantiationPreviewView>;
  readonly applyBlueprintInstantiation: (
    preview: BlueprintInstantiationPreviewView,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<BlueprintInstantiationCompleted>;
  readonly previewAlphaInstantiation: (
    request: AlphaInstantiationPreviewRequest,
  ) => Promise<AlphaInstantiationPreviewView>;
  readonly applyAlphaInstantiation: (
    preview: AlphaInstantiationPreviewView,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<AlphaInstantiationCompleted>;
  readonly previewCourseRollover: (
    request: CourseRolloverPreviewRequest,
  ) => Promise<CourseRolloverPreviewView>;
  readonly applyCourseRollover: (
    preview: CourseRolloverPreviewView,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<CourseRolloverCompleted>;
  readonly previewCourseTermShift: (
    request: CourseTermShiftPreviewRequest,
  ) => Promise<CourseTermShiftPreviewOutcome>;
  readonly applyCourseTermShift: (
    preview: EligibleCourseTermShiftPreview,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<CourseTermShiftCompleted>;
  readonly inspectCurriculumImports: (
    course: CourseReference,
  ) => Promise<CurriculumCourseImportView>;
  readonly previewAssignmentFastForward: (
    request: AssignmentFastForwardPreviewRequest,
  ) => Promise<AssignmentFastForwardPreviewView>;
  readonly applyAssignmentFastForward: (
    preview: EligibleAssignmentFastForwardPreview,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<AssignmentFastForwardCompleted>;
  readonly previewSourceDerivedAssignment: (
    request: SourceDerivedAssignmentPreviewRequest,
  ) => Promise<SourceDerivedAssignmentPreviewView>;
  readonly applySourceDerivedAssignment: (
    preview: SourceDerivedAssignmentPreviewView,
    idempotencyKey: CurriculumAdoptionIdempotencyKey,
  ) => Promise<SourceDerivedAssignmentCompleted>;
  readonly reconcileCurriculumAdoption: (
    command: ReconcileCurriculumAdoptionCommand,
  ) => Promise<CurriculumAdoptionReconciliationResult>;
}

/** A completed result always carries the exact immutable receipt binding. */
export type CurriculumAdoptionCompleted =
  | ForkAlphaCompleted
  | BlueprintInstantiationCompleted
  | AlphaInstantiationCompleted
  | CourseRolloverCompleted
  | CourseTermShiftCompleted
  | AssignmentFastForwardCompleted
  | SourceDerivedAssignmentCompleted;

/** The only field accepted by the generic receipt-led recovery operation. */
export type CurriculumAdoptionReceipt = CurriculumAdoptionReceiptBinding;
