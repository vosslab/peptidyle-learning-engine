// index.ts - the only browser boundary around generated wasm-bindgen glue.

import type { ChoiceId } from "../../generated/api/ChoiceId";
import type { ActivityTimestamp } from "../../generated/api/ActivityTimestamp";
import type { AttemptTimerRecord } from "../../generated/api/AttemptTimerRecord";
import type { BackendCapabilities } from "../../generated/api/BackendCapabilities";
import type { Capability } from "../../generated/api/Capability";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";
import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { SelectionCardinality } from "../../generated/api/SelectionCardinality";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TimingPolicy } from "../../generated/api/TimingPolicy";
import type { VersionId } from "../../generated/api/VersionId";

export type ResponseFormatViolation =
  | { readonly kind: "responseKindMismatch" }
  | { readonly kind: "numericNotFinite" }
  | {
      readonly kind: "selectionCount";
      readonly expected: SelectionCardinality;
      readonly actual: number;
    }
  | { readonly kind: "duplicateChoice"; readonly choice: ChoiceId }
  | { readonly kind: "unknownChoice"; readonly choice: ChoiceId }
  | {
      readonly kind: "textTooLong";
      readonly maxLength: number;
      readonly actualLength: number;
    }
  | { readonly kind: "orderingItemsMismatch" }
  | { readonly kind: "missingUploadReference" };

export interface ResponseFormatReport {
  readonly violations: ReadonlyArray<ResponseFormatViolation>;
}

export type FormatValidator = (
  definition: ResponseDefinition,
  response: StudentResponse,
) => Promise<ResponseFormatReport>;

export interface TimerEvaluation {
  readonly policy: TimingPolicy;
  readonly timer: AttemptTimerRecord;
  readonly evaluatedAt: ActivityTimestamp;
  readonly pauseExtensionMillis: number;
}

export type TimerVerdict =
  "untimed" | "open" | "gracePeriod" | "submittedOnTime" | "submittedWithinGrace" | "timedOut";

export type TimerEvaluator = (evaluation: TimerEvaluation) => Promise<TimerVerdict>;

export interface AssignmentQuestionConfig {
  readonly question: QuestionDefinition;
  readonly backendCapabilities: BackendCapabilities;
}

export interface AssignmentConfig {
  readonly questions: ReadonlyArray<AssignmentQuestionConfig>;
  readonly requiredCapabilities: ReadonlyArray<Capability>;
}

export interface CapabilityViolation {
  readonly question: VersionId;
  readonly capability: Capability;
}

export type CapabilityValidator = (
  config: AssignmentConfig,
) => Promise<ReadonlyArray<CapabilityViolation>>;

export interface WasmFacade {
  readonly mode: "wasm" | "serverFallback";
  readonly degradedReason?: string;
  readonly validateResponseFormat: FormatValidator;
  readonly timerVerdict: TimerEvaluator;
  readonly validateAssignmentConfig: CapabilityValidator;
}

interface WasmBindgenModule {
  readonly default: (moduleOrPath: URL) => Promise<unknown>;
  readonly timer_verdict: (evaluationJson: string) => string;
  readonly validate_assignment_config: (configJson: string) => string;
  readonly validate_response_format: (definitionJson: string, responseJson: string) => string;
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
    typeof value["timer_verdict"] === "function" &&
    typeof value["validate_assignment_config"] === "function" &&
    typeof value["validate_response_format"] === "function"
  );
}

function requiredString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`WASM validation report field ${key} must be a string`);
  }
  return value;
}

function requiredNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`WASM validation report field ${key} must be a finite number`);
  }
  return value;
}

function parseSelectionCardinality(value: unknown): SelectionCardinality {
  if (!isRecord(value)) {
    throw new Error("WASM selection cardinality must be an object");
  }
  const kind = requiredString(value, "kind");
  switch (kind) {
    case "exactlyOne":
    case "anyNumber":
    case "atLeastOne":
      return { kind };
    case "exactly":
      return { kind, count: requiredNumber(value, "count") };
    default:
      throw new Error(`Unknown WASM selection cardinality ${kind}`);
  }
}

function parseViolation(value: unknown): ResponseFormatViolation {
  if (!isRecord(value)) {
    throw new Error("WASM format violation must be an object");
  }
  const kind = requiredString(value, "kind");
  switch (kind) {
    case "responseKindMismatch":
    case "numericNotFinite":
    case "orderingItemsMismatch":
    case "missingUploadReference":
      return { kind };
    case "selectionCount":
      return {
        kind,
        expected: parseSelectionCardinality(value["expected"]),
        actual: requiredNumber(value, "actual"),
      };
    case "duplicateChoice":
    case "unknownChoice":
      return { kind, choice: requiredString(value, "choice") };
    case "textTooLong":
      return {
        kind,
        maxLength: requiredNumber(value, "maxLength"),
        actualLength: requiredNumber(value, "actualLength"),
      };
    default:
      throw new Error(`Unknown WASM format violation ${kind}`);
  }
}

function parseFormatReport(json: string): ResponseFormatReport {
  const value: unknown = JSON.parse(json);
  if (!isRecord(value) || !Array.isArray(value["violations"])) {
    throw new Error("WASM format report must contain a violations array");
  }
  return { violations: value["violations"].map(parseViolation) };
}

function parseTimerVerdict(json: string): TimerVerdict {
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
    case "perQuestionTiming":
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
      question: requiredString(entry, "question"),
      capability: parseCapability(entry["capability"]),
    };
  });
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "WebAssembly initialization failed";
}

async function initializeWasmFacade(
  formatFallback: FormatValidator,
  timerFallback: TimerEvaluator,
  capabilityFallback: CapabilityValidator,
): Promise<WasmFacade> {
  try {
    const bridgeUrl = new URL("wasm/ple_bridge.js", document.baseURI).href;
    const loaded: unknown = await import(bridgeUrl);
    if (!isWasmBindgenModule(loaded)) {
      throw new Error("Generated WebAssembly bridge has an unexpected export shape");
    }
    await loaded.default(new URL("wasm/ple_bridge_bg.wasm", document.baseURI));

    const validateResponseFormat: FormatValidator = (definition, response) => {
      const json = loaded.validate_response_format(
        JSON.stringify(definition),
        JSON.stringify(response),
      );
      return Promise.resolve(parseFormatReport(json));
    };
    const timerVerdict: TimerEvaluator = (evaluation) =>
      Promise.resolve(parseTimerVerdict(loaded.timer_verdict(JSON.stringify(evaluation))));
    const validateAssignmentConfig: CapabilityValidator = (config) =>
      Promise.resolve(
        parseCapabilityViolations(loaded.validate_assignment_config(JSON.stringify(config))),
      );
    return { mode: "wasm", validateResponseFormat, timerVerdict, validateAssignmentConfig };
  } catch (error: unknown) {
    return {
      mode: "serverFallback",
      degradedReason: errorMessage(error),
      validateResponseFormat: formatFallback,
      timerVerdict: timerFallback,
      validateAssignmentConfig: capabilityFallback,
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
