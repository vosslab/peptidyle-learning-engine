// Strict same-origin transport for B2 curriculum adoption operations.

import type { AlphaInstantiationPreviewRequest } from "../../../generated/api/AlphaInstantiationPreviewRequest";
import type { AlphaInstantiationPreviewView } from "../../../generated/api/AlphaInstantiationPreviewView";
import type { AssignmentFastForwardPreviewRequest } from "../../../generated/api/AssignmentFastForwardPreviewRequest";
import type { AssignmentFastForwardPreviewView } from "../../../generated/api/AssignmentFastForwardPreviewView";
import type { BlueprintInstantiationPreviewRequest } from "../../../generated/api/BlueprintInstantiationPreviewRequest";
import type { BlueprintInstantiationPreviewView } from "../../../generated/api/BlueprintInstantiationPreviewView";
import type { CourseRolloverPreviewRequest } from "../../../generated/api/CourseRolloverPreviewRequest";
import type { CourseRolloverPreviewView } from "../../../generated/api/CourseRolloverPreviewView";
import type { CourseTermShiftPreviewOutcome } from "../../../generated/api/CourseTermShiftPreviewOutcome";
import type { CourseTermShiftPreviewRequest } from "../../../generated/api/CourseTermShiftPreviewRequest";
import type { CurriculumCourseImportView as CurriculumCourseImportResponse } from "../../../generated/api/CurriculumCourseImportView";
import type { CurriculumAdoptionReconciliationResult } from "../../../generated/api/CurriculumAdoptionReconciliationResult";
import type { ForkAlphaPreviewRequest } from "../../../generated/api/ForkAlphaPreviewRequest";
import type { ForkAlphaPreviewView } from "../../../generated/api/ForkAlphaPreviewView";
import type { ReconcileCurriculumAdoptionCommand } from "../../../generated/api/ReconcileCurriculumAdoptionCommand";
import type { SourceDerivedAssignmentPreviewRequest } from "../../../generated/api/SourceDerivedAssignmentPreviewRequest";
import type { SourceDerivedAssignmentPreviewView } from "../../../generated/api/SourceDerivedAssignmentPreviewView";
import type { ApiClient } from "../client";
import type {
  CurriculumAdoptionClient,
  CurriculumAdoptionIdempotencyKey,
  EligibleAssignmentFastForwardPreview,
  EligibleCourseTermShiftPreview,
} from "../curriculum_adoption";
import {
  decodeAlphaInstantiationPreviewView,
  decodeAssignmentFastForwardPreviewRequest,
  decodeAssignmentFastForwardPreviewView,
  decodeAlphaInstantiationPreviewRequest as decodeAlphaRequest,
  decodeBlueprintInstantiationPreviewRequest,
  decodeBlueprintInstantiationPreviewView,
  decodeCourseRolloverPreviewRequest,
  decodeCourseRolloverPreviewView,
  decodeCourseTermShiftPreviewOutcome,
  decodeCourseTermShiftPreviewRequest,
  decodeCurriculumAdoptionIdempotencyKey,
  decodeCurriculumCourseImportView,
  decodeCurriculumCourseReference,
  decodeForkAlphaPreviewRequest,
  decodeForkAlphaPreviewView,
  decodeReconcileCurriculumAdoptionCommand,
  decodeSourceDerivedAssignmentPreviewRequest,
  decodeSourceDerivedAssignmentPreviewView,
  decodeForkAlphaCompleted,
  decodeBlueprintInstantiationCompleted,
  decodeAlphaInstantiationCompleted,
  decodeCourseRolloverCompleted,
  decodeCourseTermShiftCompleted,
  decodeAssignmentFastForwardCompleted,
  decodeSourceDerivedAssignmentCompleted,
  decodeCurriculumAdoptionReconciliationResult,
} from "../decoders/curriculum_adoption";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

type Decoder<T> = (value: unknown, path?: string) => T;

function requireNoScheduleCorrections(
  value: { readonly corrections: ReadonlyArray<unknown> },
  path: string,
): void {
  if (value.corrections.length !== 0) {
    throw new ApiProtocolError(`API ${path} cannot apply a preview that requires correction`);
  }
}

function requireCorrectionFreePreview(
  value: { readonly corrections: ReadonlyArray<unknown>; readonly pinCorrection: object | null },
  path: string,
): void {
  requireNoScheduleCorrections(value, path);
  if (value.pinCorrection !== null) {
    throw new ApiProtocolError(`API ${path} cannot apply a preview that requires correction`);
  }
}

function requireForkPreview(value: ForkAlphaPreviewView, path: string): void {
  if (value.pinCorrection !== null) {
    throw new ApiProtocolError(`API ${path} cannot apply a preview that requires correction`);
  }
}

function requireEligibleTermShift(value: CourseTermShiftPreviewOutcome, path: string): void {
  if (value.kind !== "eligible") {
    throw new ApiProtocolError(`API ${path} requires an eligible term-shift preview`);
  }
  requireNoScheduleCorrections(value.preview, path);
}

function requireEligibleFastForward(value: AssignmentFastForwardPreviewView, path: string): void {
  if (value.decision.kind !== "eligible") {
    throw new ApiProtocolError(`API ${path} requires an eligible fast-forward preview`);
  }
}

function sourcePath(reference: string, prefix: "alpha-courses" | "course-blueprints"): string {
  return `/api/${prefix}/${encodeURIComponent(reference)}`;
}

function coursePath(course: string): string {
  return `/api/courses/${encodeURIComponent(course)}`;
}

function assignmentPath(course: string, assignment: string): string {
  return `${coursePath(course)}/assignments/${encodeURIComponent(assignment)}`;
}

function importsPath(course: string): string {
  return `/api/courses/${encodeURIComponent(course)}/curriculum-imports`;
}

function applyBody<T>(preview: T, idempotencyKey: CurriculumAdoptionIdempotencyKey): unknown {
  return {
    preview,
    idempotencyKey: decodeCurriculumAdoptionIdempotencyKey(idempotencyKey),
  };
}

async function adoptionJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: Decoder<T>,
  body?: unknown,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: body === undefined ? "GET" : "POST",
    body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 200) {
    throw new ApiProtocolError(`API response ${path} must use status 200`);
  }
  const value = await boundedResponseJson(response, path);
  return decoder(value, "response");
}

async function preview<TRequest, TView>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  request: TRequest,
  requestDecoder: Decoder<TRequest>,
  responseDecoder: Decoder<TView>,
): Promise<TView> {
  const checkedRequest = requestDecoder(request, "request");
  return adoptionJson(fetchImplementation, basePath, path, responseDecoder, checkedRequest);
}

async function apply<TView, TResult>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string | ((preview: TView) => string),
  previewValue: TView,
  idempotencyKey: CurriculumAdoptionIdempotencyKey,
  previewDecoder: Decoder<TView>,
  responseDecoder: Decoder<TResult>,
  eligibility?: (value: TView, path: string) => void,
): Promise<TResult> {
  const checkedPreview = previewDecoder(previewValue, "preview");
  const eligibilityPath = typeof path === "string" ? path : "/api/curriculum-adoption/apply";
  eligibility?.(checkedPreview, eligibilityPath);
  const resolvedPath = typeof path === "string" ? path : path(checkedPreview);
  return adoptionJson(
    fetchImplementation,
    basePath,
    resolvedPath,
    responseDecoder,
    applyBody(checkedPreview, idempotencyKey),
  );
}

/** Creates the B2 capability without coupling it to a screen or store model. */
export function createCurriculumAdoptionClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof CurriculumAdoptionClient> {
  return {
    previewForkAlpha: (request: ForkAlphaPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${sourcePath(request.source.reference, "alpha-courses")}/fork/preview`,
        request,
        decodeForkAlphaPreviewRequest,
        decodeForkAlphaPreviewView,
      ),
    applyForkAlpha: (previewValue: ForkAlphaPreviewView, key: CurriculumAdoptionIdempotencyKey) =>
      apply(
        fetchImplementation,
        basePath,
        `${sourcePath(previewValue.source.reference, "alpha-courses")}/fork/apply`,
        previewValue,
        key,
        decodeForkAlphaPreviewView,
        decodeForkAlphaCompleted,
        requireForkPreview,
      ),
    previewBlueprintInstantiation: (request: BlueprintInstantiationPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${sourcePath(request.source.reference, "course-blueprints")}/instantiate/preview`,
        request,
        decodeBlueprintInstantiationPreviewRequest,
        decodeBlueprintInstantiationPreviewView,
      ),
    applyBlueprintInstantiation: (
      previewValue: BlueprintInstantiationPreviewView,
      key: CurriculumAdoptionIdempotencyKey,
    ) =>
      apply(
        fetchImplementation,
        basePath,
        `${sourcePath(previewValue.source.reference, "course-blueprints")}/instantiate/apply`,
        previewValue,
        key,
        decodeBlueprintInstantiationPreviewView,
        decodeBlueprintInstantiationCompleted,
        requireCorrectionFreePreview,
      ),
    previewAlphaInstantiation: (request: AlphaInstantiationPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${sourcePath(request.source.reference, "alpha-courses")}/instantiate/preview`,
        request,
        decodeAlphaRequest,
        decodeAlphaInstantiationPreviewView,
      ),
    applyAlphaInstantiation: (
      previewValue: AlphaInstantiationPreviewView,
      key: CurriculumAdoptionIdempotencyKey,
    ) =>
      apply(
        fetchImplementation,
        basePath,
        `${sourcePath(previewValue.source.reference, "alpha-courses")}/instantiate/apply`,
        previewValue,
        key,
        decodeAlphaInstantiationPreviewView,
        decodeAlphaInstantiationCompleted,
        requireCorrectionFreePreview,
      ),
    previewCourseRollover: (request: CourseRolloverPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${coursePath(request.witness.course)}/curriculum-rollover/preview`,
        request,
        decodeCourseRolloverPreviewRequest,
        decodeCourseRolloverPreviewView,
      ),
    applyCourseRollover: (
      previewValue: CourseRolloverPreviewView,
      key: CurriculumAdoptionIdempotencyKey,
    ) =>
      apply(
        fetchImplementation,
        basePath,
        `${coursePath(previewValue.witness.course)}/curriculum-rollover/apply`,
        previewValue,
        key,
        decodeCourseRolloverPreviewView,
        decodeCourseRolloverCompleted,
        requireCorrectionFreePreview,
      ),
    previewCourseTermShift: (request: CourseTermShiftPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${coursePath(request.witness.course)}/curriculum-term-shift/preview`,
        request,
        decodeCourseTermShiftPreviewRequest,
        decodeCourseTermShiftPreviewOutcome,
      ),
    applyCourseTermShift: (
      previewValue: EligibleCourseTermShiftPreview,
      key: CurriculumAdoptionIdempotencyKey,
    ) =>
      apply(
        fetchImplementation,
        basePath,
        (preview) =>
          `${coursePath(preview.kind === "eligible" ? preview.preview.witness.course : preview.course)}/curriculum-term-shift/apply`,
        previewValue,
        key,
        decodeCourseTermShiftPreviewOutcome,
        decodeCourseTermShiftCompleted,
        requireEligibleTermShift,
      ),
    inspectCurriculumImports: (course): Promise<CurriculumCourseImportResponse> => {
      const checkedCourse = decodeCurriculumCourseReference(course);
      return adoptionJson<CurriculumCourseImportResponse>(
        fetchImplementation,
        basePath,
        importsPath(checkedCourse),
        decodeCurriculumCourseImportView,
      ).then((inspection) => {
        if (inspection.witness.course !== checkedCourse) {
          throw new ApiProtocolError(
            `API ${importsPath(checkedCourse)} response witness must name the requested course`,
          );
        }
        return inspection;
      });
    },
    previewAssignmentFastForward: (request: AssignmentFastForwardPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${assignmentPath(request.course, request.assignment.assignment)}/curriculum-fast-forward/preview`,
        request,
        decodeAssignmentFastForwardPreviewRequest,
        decodeAssignmentFastForwardPreviewView,
      ),
    applyAssignmentFastForward: (
      previewValue: EligibleAssignmentFastForwardPreview,
      key: CurriculumAdoptionIdempotencyKey,
    ) =>
      apply(
        fetchImplementation,
        basePath,
        `${assignmentPath(previewValue.course, previewValue.assignment.assignment)}/curriculum-fast-forward/apply`,
        previewValue,
        key,
        decodeAssignmentFastForwardPreviewView,
        decodeAssignmentFastForwardCompleted,
        requireEligibleFastForward,
      ),
    previewSourceDerivedAssignment: (request: SourceDerivedAssignmentPreviewRequest) =>
      preview(
        fetchImplementation,
        basePath,
        `${coursePath(request.course)}/curriculum-source-derived-assignment/preview`,
        request,
        decodeSourceDerivedAssignmentPreviewRequest,
        decodeSourceDerivedAssignmentPreviewView,
      ),
    applySourceDerivedAssignment: (
      previewValue: SourceDerivedAssignmentPreviewView,
      key: CurriculumAdoptionIdempotencyKey,
    ) =>
      apply(
        fetchImplementation,
        basePath,
        `${coursePath(previewValue.course)}/curriculum-source-derived-assignment/apply`,
        previewValue,
        key,
        decodeSourceDerivedAssignmentPreviewView,
        decodeSourceDerivedAssignmentCompleted,
        requireCorrectionFreePreview,
      ),
    reconcileCurriculumAdoption: (
      command: ReconcileCurriculumAdoptionCommand,
    ): Promise<CurriculumAdoptionReconciliationResult> => {
      const path = "/api/curriculum-adoption/reconcile";
      const checkedCommand = decodeReconcileCurriculumAdoptionCommand(command);
      return adoptionJson(
        fetchImplementation,
        basePath,
        path,
        decodeCurriculumAdoptionReconciliationResult,
        checkedCommand,
      );
    },
  };
}
