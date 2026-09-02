// index.ts - the only browser boundary around generated wasm-bindgen glue.

import type { QuestionAssetRendition } from "../../generated/api/QuestionAssetRendition";
import type { Timestamp } from "../../generated/api/Timestamp";
import type { QuestionAttemptTiming } from "../../generated/api/QuestionAttemptTiming";
import type { QuestionBackendCapabilities } from "../../generated/api/QuestionBackendCapabilities";
import type { Capability } from "../../generated/api/Capability";
import type { QuestionResponseFormat } from "../../generated/api/QuestionResponseFormat";
import type { QuestionRevision } from "../../generated/api/QuestionRevision";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionPresentationToken } from "../../generated/api/QuestionPresentationToken";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { DraftQuestionBackendLocator } from "../../generated/api/DraftQuestionBackendLocator";
import type { QuestionVariationRule } from "../../generated/api/QuestionVariationRule";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { QuestionAttemptTimeLimit } from "../../generated/api/QuestionAttemptTimeLimit";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import { decodeQuestionRevisionReference } from "../api/decoders/shared";
import { decodeKeyFreeDraftPreview } from "../api/decoders/question_model";
import { decodeStudentResponseFormatCheck } from "../api/decoders/student_response_format_check";
export type {
  StudentResponseFormatCheck,
  StudentResponseFormatIssue,
} from "../api/decoders/student_response_format_check";
import type { StudentResponseFormatCheck } from "../api/decoders/student_response_format_check";

export type FormatValidator = (
  definition: QuestionResponseFormat,
  response: StudentResponse,
) => Promise<StudentResponseFormatCheck>;

export interface QuestionAttemptTimingEvaluation {
  readonly policy: QuestionAttemptTimeLimit;
  readonly timing: QuestionAttemptTiming;
  readonly evaluatedAt: Timestamp;
  readonly pauseExtensionMillis: number;
}

export type QuestionAttemptTimingDecision =
  "untimed" | "open" | "gracePeriod" | "submittedOnTime" | "submittedWithinGrace" | "timedOut";

export type TimerEvaluator = (
  evaluation: QuestionAttemptTimingEvaluation,
) => Promise<QuestionAttemptTimingDecision>;

export interface AssignmentQuestionConfig {
  readonly question: QuestionRevision;
  readonly backendCapabilities: QuestionBackendCapabilities;
}

export interface AssignmentConfig {
  readonly questions: ReadonlyArray<AssignmentQuestionConfig>;
  readonly requiredCapabilities: ReadonlyArray<Capability>;
}

export interface CapabilityViolation {
  readonly question: QuestionRevisionReference;
  readonly capability: Capability;
}

export type CapabilityValidator = (
  config: AssignmentConfig,
) => Promise<ReadonlyArray<CapabilityViolation>>;

/** Key-free browser inputs for deterministic workspace-draft preview. */
export interface PleDraftPreviewRequest {
  readonly workspace: string;
  readonly backendLocator: DraftQuestionBackendLocator;
  readonly title: string;
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
  readonly questionVariationRule: QuestionVariationRule;
}

/** Identity-free preview material returned only for local PLE Question Sources. */
export interface PleDraftPreview {
  readonly workspace: string;
  readonly seed: number;
  readonly title: string;
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
}

export type PleDraftPreviewResult =
  | { readonly kind: "ready"; readonly preview: PleDraftPreview }
  | {
      readonly kind: "unavailable";
      readonly backend: QuestionBackend;
      readonly capability: "offlinePreview";
    };

export type PleDraftPreviewer = (
  request: PleDraftPreviewRequest,
  seed: number,
) => Promise<PleDraftPreviewResult>;

export type PresentationVerification =
  { readonly kind: "match" } | { readonly kind: "mismatch" } | { readonly kind: "unavailable" };

export type PresentationVerifier = (
  envelope: QuestionPresentation,
  assets: ReadonlyArray<QuestionAssetRendition>,
  presentationToken: QuestionPresentationToken,
) => Promise<PresentationVerification>;

export interface WasmFacade {
  readonly mode: "wasm" | "serverFallback";
  readonly degradedReason?: string;
  readonly validateResponseFormat: FormatValidator;
  readonly questionAttemptTimingDecision: TimerEvaluator;
  readonly validateAssignmentConfig: CapabilityValidator;
  readonly previewPleDraft: PleDraftPreviewer;
  readonly verifyPresentationDescriptor: PresentationVerifier;
}

interface WasmBindgenModule {
  readonly default: (moduleOrPath: URL) => Promise<unknown>;
  readonly question_attempt_timing_decision: (evaluationJson: string) => string;
  readonly validate_assignment_config: (configJson: string) => string;
  readonly validate_response_format: (definitionJson: string, responseJson: string) => string;
  readonly preview_ple_draft: (draftJson: string, seedJson: string) => string;
  readonly verify_presentation_descriptor: (
    envelopeJson: string,
    questionAssetRenditionsJson: string,
    presentationToken: string,
  ) => boolean;
}

let sharedFacade: Promise<WasmFacade> | undefined;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isWasmBindgenModule(value: unknown): value is WasmBindgenModule {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value["default"] === "function" &&
    typeof value["question_attempt_timing_decision"] === "function" &&
    typeof value["validate_assignment_config"] === "function" &&
    typeof value["validate_response_format"] === "function" &&
    typeof value["preview_ple_draft"] === "function" &&
    typeof value["verify_presentation_descriptor"] === "function"
  );
}

function rejectUnknownFields(
  record: Record<string, unknown>,
  allowed: ReadonlySet<string>,
  shape: string,
): void {
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) throw new Error(`${shape} has unknown field ${key}`);
  }
}

function parseQuestionBackend(value: unknown): QuestionBackend {
  switch (value) {
    case "ple":
    case "webwork":
    case "qti":
    case "h5p":
    case "imathas":
      return value;
    default:
      throw new Error("WASM draft preview has an unknown backend");
  }
}

/** Strictly decodes the reviewed, key-free WebAssembly draft-preview result. */
export function decodePleDraftPreviewResult(json: string): PleDraftPreviewResult {
  const value: unknown = JSON.parse(json);
  if (!isRecord(value)) throw new Error("WASM draft preview result must be an object");
  const kind = requiredString(value, "kind");
  if (kind === "unavailable") {
    rejectUnknownFields(
      value,
      new Set(["kind", "backend", "capability"]),
      "WASM unavailable preview",
    );
    if (value["capability"] !== "offlinePreview")
      throw new Error("WASM unavailable preview must name offlinePreview");
    return { kind, backend: parseQuestionBackend(value["backend"]), capability: "offlinePreview" };
  }
  if (kind !== "ready") throw new Error(`Unknown WASM draft preview kind ${kind}`);
  rejectUnknownFields(value, new Set(["kind", "preview"]), "WASM ready preview");
  if (!isRecord(value["preview"]))
    throw new Error("WASM ready preview must contain a preview object");
  const preview = value["preview"];
  rejectUnknownFields(
    preview,
    new Set(["workspace", "seed", "title", "prompt", "response"]),
    "WASM preview",
  );
  for (const forbidden of ["problem", "version", "answer", "key", "grading", "correct", "score"]) {
    if (forbidden in preview) throw new Error(`WASM preview must not contain ${forbidden}`);
  }
  const decoded = decodeKeyFreeDraftPreview(preview, "wasmPreview");
  return {
    kind,
    preview: {
      ...decoded,
    },
  };
}

function requiredString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`WASM draft-preview result field ${key} must be a string`);
  }
  return value;
}

function parseStudentResponseFormatCheck(json: string): StudentResponseFormatCheck {
  const value: unknown = JSON.parse(json);
  return decodeStudentResponseFormatCheck(value, "WASM Student Response Format Check");
}

function parseQuestionAttemptTimingDecision(json: string): QuestionAttemptTimingDecision {
  const value: unknown = JSON.parse(json);
  switch (value) {
    case "untimed":
    case "open":
    case "gracePeriod":
    case "submittedOnTime":
    case "submittedWithinGrace":
    case "timedOut":
      return value;
    default:
      throw new Error("WASM timer verdict has an unknown value");
  }
}

function parseCapability(value: unknown): Capability {
  switch (value) {
    case "algorithmicGeneration":
    case "clientRendering":
    case "serverGrading":
    case "partialCredit":
    case "hints":
    case "questionAttemptTimeLimit":
    case "printExport":
    case "offlinePreview":
      return value;
    default:
      throw new Error("WASM capability violation has an unknown capability");
  }
}

function parseCapabilityViolations(json: string): ReadonlyArray<CapabilityViolation> {
  const value: unknown = JSON.parse(json);
  if (!Array.isArray(value)) {
    throw new Error("WASM capability result must be an array");
  }
  return value.map((entry) => {
    if (!isRecord(entry)) {
      throw new Error("WASM capability violation must be an object");
    }
    return {
      question: decodeQuestionRevisionReference(
        entry["question"],
        "capabilityViolation.question",
        true,
      ),
      capability: parseCapability(entry["capability"]),
    };
  });
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "WebAssembly initialization failed";
}

function wasmAssetUrl(fileName: "ple_bridge.js" | "ple_bridge_bg.wasm"): URL {
  return new URL(`wasm/${fileName}`, `${window.location.origin}/`);
}

async function initializeWasmFacade(
  formatFallback: FormatValidator,
  timerFallback: TimerEvaluator,
  capabilityFallback: CapabilityValidator,
): Promise<WasmFacade> {
  try {
    const bridgeUrl = wasmAssetUrl("ple_bridge.js").href;
    const loaded: unknown = await import(bridgeUrl);
    if (!isWasmBindgenModule(loaded)) {
      throw new Error("Generated WebAssembly bridge has an unexpected export shape");
    }
    await loaded.default(wasmAssetUrl("ple_bridge_bg.wasm"));

    const validateResponseFormat: FormatValidator = (definition, response) => {
      const json = loaded.validate_response_format(
        JSON.stringify(definition),
        JSON.stringify(response),
      );
      return Promise.resolve(parseStudentResponseFormatCheck(json));
    };
    const questionAttemptTimingDecision: TimerEvaluator = (evaluation) =>
      Promise.resolve(
        parseQuestionAttemptTimingDecision(
          loaded.question_attempt_timing_decision(JSON.stringify(evaluation)),
        ),
      );
    const validateAssignmentConfig: CapabilityValidator = (config) =>
      Promise.resolve(
        parseCapabilityViolations(loaded.validate_assignment_config(JSON.stringify(config))),
      );
    const previewPleDraft: PleDraftPreviewer = (request, seed) =>
      Promise.resolve(
        decodePleDraftPreviewResult(
          loaded.preview_ple_draft(JSON.stringify(request), JSON.stringify(seed)),
        ),
      );
    const verifyPresentationDescriptor: PresentationVerifier = (
      envelope,
      assets,
      presentationToken,
    ) =>
      Promise.resolve({
        kind: loaded.verify_presentation_descriptor(
          JSON.stringify(envelope),
          JSON.stringify(assets),
          presentationToken,
        )
          ? "match"
          : "mismatch",
      });
    return {
      mode: "wasm",
      validateResponseFormat,
      questionAttemptTimingDecision,
      validateAssignmentConfig,
      previewPleDraft,
      verifyPresentationDescriptor,
    };
  } catch (error: unknown) {
    return {
      mode: "serverFallback",
      degradedReason: errorMessage(error),
      validateResponseFormat: formatFallback,
      questionAttemptTimingDecision: timerFallback,
      validateAssignmentConfig: capabilityFallback,
      previewPleDraft: (request) =>
        Promise.resolve({
          kind: "unavailable",
          backend: request.backendLocator.backend,
          capability: "offlinePreview",
        }),
      verifyPresentationDescriptor: () => Promise.resolve({ kind: "unavailable" }),
    };
  }
}

/** Loads and initializes one shared facade for every browser consumer. */
export function loadWasmFacade(
  formatFallback: FormatValidator,
  timerFallback: TimerEvaluator,
  capabilityFallback: CapabilityValidator,
): Promise<WasmFacade> {
  sharedFacade ??= initializeWasmFacade(formatFallback, timerFallback, capabilityFallback);
  return sharedFacade;
}
