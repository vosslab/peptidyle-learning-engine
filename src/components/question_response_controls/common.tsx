// common.tsx - shared browser-safe response-controller contracts and controls.

import { createContext, createSignal, onCleanup, useContext, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { AssessmentAttemptId } from "../../../generated/api/AssessmentAttemptId";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { QuestionPresentationResponseFormat } from "../../../generated/api/QuestionPresentationResponseFormat";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ImathasQuestionBackendLaunch } from "../../api/contracts";
import type {
  StudentResponseFormatCheck,
  StudentResponseFormatIssue,
} from "../../api/decoders/student_response_format_check";
import type { ResponseFormatValidator } from "../../wasm/index";
import type { AssetUrlResolver } from "../question_renderer";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import type { DraftQuestionRouteId } from "../../navigation/public_route";

export type ResponseFormat = QuestionResponseFormat | QuestionPresentationResponseFormat;
/**
 * Format-only and save modes share native controls, but only save
 * mode persists editable Question responses. Keep that distinction at the
 * shared controller boundary so each response-format component stays native.
 */
export type ResponseControlMode = "save" | "formatOnly";
/** Current response persistence has one success and one failure outcome. */
export type ResponseSaveOutcome =
  { readonly kind: "accepted" } | { readonly kind: "rejected"; readonly message: string };
const ResponseControlModeContext = createContext<ResponseControlMode>("save");

export function ResponseControlModeProvider(props: {
  readonly mode: ResponseControlMode;
  readonly children: JSX.Element;
}): JSX.Element {
  return (
    <ResponseControlModeContext.Provider value={props.mode}>
      {props.children}
    </ResponseControlModeContext.Provider>
  );
}
export type MultipleChoiceResponseFormat =
  | Extract<QuestionResponseFormat, { kind: "multipleChoice" }>
  | Extract<QuestionPresentationResponseFormat, { kind: "singleChoice" | "multipleAnswer" }>;
export type NumericResponseFormat =
  | Extract<QuestionResponseFormat, { kind: "numeric" }>
  | Extract<QuestionPresentationResponseFormat, { kind: "numerical" }>;
export type ShortTextResponseFormat =
  | Extract<QuestionResponseFormat, { kind: "shortText" }>
  | Extract<QuestionPresentationResponseFormat, { kind: "fillIn" }>;
export type OrderingResponseFormat = Extract<ResponseFormat, { kind: "ordering" }>;
export type MultiBlankResponseFormat =
  | Extract<QuestionResponseFormat, { kind: "multiBlank" }>
  | Extract<QuestionPresentationResponseFormat, { kind: "multiFillIn" }>;
export type MatchingResponseFormat = Extract<ResponseFormat, { kind: "matching" }>;
export type HotspotResponseFormat = Extract<ResponseFormat, { kind: "hotspot" }>;

type QuestionResponseControlPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "validating" }
  | { readonly kind: "ready" }
  | { readonly kind: "restored" }
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "saving" }
  | { readonly kind: "saved" }
  | { readonly kind: "failed"; readonly message: string };

export interface StudentWorkRouteScope {
  readonly courseId: CourseInstanceId;
  readonly assessmentId: AssessmentId;
}

export interface QuestionResponseControlBaseProps {
  readonly attemptId: string;
  /** Exact publication identity and authorized resolver for image-backed controls. */
  readonly questionRevisionTuple?: QuestionRevisionTuple;
  readonly assetUrl?: AssetUrlResolver;
  /** Authorized private Draft route; local author preview only. */
  readonly hotspotDraftAsset?: {
    readonly draftQuestion: DraftQuestionRouteId;
    readonly assetUrl: AssetUrlResolver;
  };
  /** Format-only controls have no Student Response save capability. */
  readonly mode?: ResponseControlMode;
  /** Question Response Controls require only the key-free local format validation capability. */
  readonly validator: { readonly validateResponseFormat: ResponseFormatValidator };
  readonly onSave?: (response: StudentResponse) => Promise<ResponseSaveOutcome>;
  /** Delivery surfaces may name a durable save without changing response semantics. */
  readonly saveLabel?: string;
  readonly onEscape: () => void;
  /**
   * Editable delivery surfaces receive the raw response synchronously.  This
   * invalidates any older save before asynchronous format validation returns.
   */
  readonly onResponseEdit?: (response: StudentResponse) => number | undefined;
  readonly onResponseChange?: (
    response: StudentResponse,
    validation: StudentResponseFormatCheck,
    editRevision?: number,
  ) => void;
  /** Exact navigation scope required to activate an iMathAS Question Backend response. */
  readonly studentWorkRoute?: StudentWorkRouteScope;
  readonly beginImathasQuestionBackendLaunch?: () => Promise<ImathasQuestionBackendLaunch>;
  /** Current authorized lifecycle ID for a backend-owned document route. */
  readonly assessmentAttempt?: AssessmentAttemptId;
  /** Current 1-based position for a backend-owned document route. */
  readonly position?: number;
  /**
   * The active backend document registers its one generic form capture with
   * Assessment Attempt delivery so Finish can save that same opaque response.
   */
  readonly registerBackendOwnedCapture?: (capture: () => Promise<boolean>) => () => void;
}

export interface QuestionResponseControlProps extends QuestionResponseControlBaseProps {
  readonly responseFormat: ResponseFormat;
  readonly initialResponse?: StudentResponse;
}

export interface QuestionResponseControlBodyProps<
  D extends ResponseFormat,
> extends QuestionResponseControlBaseProps {
  readonly responseFormat: D;
  readonly initialResponse?: StudentResponse;
}

export type MultipleChoiceResponseProps =
  QuestionResponseControlBodyProps<MultipleChoiceResponseFormat>;

export interface ResponseController {
  readonly phase: () => QuestionResponseControlPhase;
  readonly invalid: () => boolean;
  readonly pending: () => boolean;
  readonly locked: () => boolean;
  readonly canSave: () => boolean;
  readonly canReset: () => boolean;
  /** Record an input edit before its asynchronous format check starts. */
  readonly edit: (response: StudentResponse) => Promise<void>;
  readonly validate: (response: StudentResponse) => Promise<void>;
  /** Restore the local response and invalidate any older format check. */
  readonly reset: (response: StudentResponse) => Promise<void>;
  readonly save: (response: StudentResponse) => Promise<void>;
}

export function textFromBlocks(blocks: ReadonlyArray<QuestionContentBlock>): string {
  return blocks
    .map((block) => {
      switch (block.kind) {
        case "text":
          return block.markdown;
        case "math":
        case "image":
        case "table":
          return block.description;
        case "code":
          return block.source;
      }
    })
    .join(" ");
}

function responseFormatMessageForIssue(issue: StudentResponseFormatIssue): string {
  switch (issue.kind) {
    case "selectionCount":
      return "Choose the requested number of responses.";
    case "duplicateChoice":
      return "Each response may be selected only once.";
    case "unknownChoice":
      return "That response is not available for this question.";
    case "numericNotFinite":
      return "Enter a finite number.";
    case "textTooLong":
      return `Keep the response within ${issue.maxLength} characters.`;
    case "orderingItemsMismatch":
      return "Place every Ordering Item in the requested order.";
    case "blankSlotsMismatch":
      return "Complete every blank once.";
    case "matchingPromptsMismatch":
      return "Match every prompt once.";
    case "duplicateMatchChoice":
      return "Use each matching choice only once.";
    case "unknownMatchChoice":
      return "That matching choice is not available.";
    case "duplicateHotspotRegion":
      return "Choose each labeled image region only once.";
    case "unknownHotspotRegion":
      return "Choose one of the available labeled image regions.";
    case "responseKindMismatch":
      return "This response does not match the question format.";
  }
}

function responseFormatMessage(check: StudentResponseFormatCheck): string {
  const first = check.issues[0];
  return first === undefined
    ? "Student Response Format is ready."
    : responseFormatMessageForIssue(first);
}

/** Browser-only format check: deliberately has no save or grading dependency. */
export async function validateResponseLocally(
  validator: { readonly validateResponseFormat: ResponseFormatValidator },
  responseFormat: ResponseFormat,
  response: StudentResponse,
): Promise<StudentResponseFormatCheck> {
  return validator.validateResponseFormat(responseFormat, response);
}

/** Preserve an empty numeric control as invalid rather than coercing it to zero. */
export function numericResponseFromInput(input: string): StudentResponse {
  return { kind: "numeric", value: input.trim() === "" ? Number.NaN : Number(input) };
}

function phaseMessage(phase: QuestionResponseControlPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Complete the response, then save it.";
    case "validating":
      return "Checking response format...";
    case "ready":
      return "Response format is ready to save.";
    case "restored":
      return "Review the response before saving changes.";
    case "invalid":
    case "failed":
      return phase.message;
    case "saving":
      return "Saving your response. Please wait.";
    case "saved":
      return "Response saved.";
  }
}

function formatOnlyPhaseMessage(phase: QuestionResponseControlPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Complete the response. Its format is checked locally.";
    case "validating":
      return "Checking response format...";
    case "ready":
    case "restored":
      return "Response format is ready.";
    case "invalid":
    case "failed":
      return phase.message;
    // Format-only controls never enter save states, but preserve a safe
    // status if a future caller supplies one.
    case "saving":
    case "saved":
      return "Response format is ready.";
  }
}

/** Key-free validation state machine. Validation never invokes server grading. */
export function createResponseController(
  props: QuestionResponseControlProps,
  initialResponse?: StudentResponse,
): ResponseController {
  const [phase, setPhase] = createSignal<QuestionResponseControlPhase>({ kind: "idle" });
  let validationRequest = 0;
  let saveRequest = 0;
  let disposed = false;
  let latestEdit:
    { readonly response: StudentResponse; readonly revision: number | undefined } | undefined;
  let validatedResponse: StudentResponse | undefined;

  onCleanup(() => {
    disposed = true;
    validationRequest += 1;
    saveRequest += 1;
  });

  function latestEditRevision(response: StudentResponse): number | undefined {
    if (
      latestEdit === undefined ||
      JSON.stringify(latestEdit.response) !== JSON.stringify(response)
    ) {
      return undefined;
    }
    return latestEdit.revision;
  }

  async function validate(response: StudentResponse, editRevision?: number): Promise<void> {
    if (disposed) return;
    if (phase().kind === "saving") {
      return;
    }
    validationRequest += 1;
    const request = validationRequest;
    const effectiveEditRevision = editRevision ?? latestEditRevision(response);
    validatedResponse = undefined;
    setPhase({ kind: "validating" });
    try {
      const check = await validateResponseLocally(props.validator, props.responseFormat, response);
      if (disposed || request !== validationRequest || phase().kind === "saving") return;
      if (check.issues.length === 0) validatedResponse = response;
      props.onResponseChange?.(response, check, effectiveEditRevision);
      setPhase(
        check.issues.length === 0
          ? { kind: "ready" }
          : { kind: "invalid", message: responseFormatMessage(check) },
      );
    } catch (error: unknown) {
      if (disposed || request !== validationRequest || phase().kind === "saving") return;
      validatedResponse = undefined;
      const message = error instanceof Error ? error.message : "format validation was unavailable";
      setPhase({ kind: "failed", message: `Cannot check this response yet: ${message}.` });
    }
  }

  async function edit(response: StudentResponse): Promise<void> {
    const editRevision = props.onResponseEdit?.(response);
    latestEdit = { response, revision: editRevision };
    await validate(response, editRevision);
  }

  async function save(response: StudentResponse): Promise<void> {
    if (disposed || props.mode === "formatOnly") return;
    if (phase().kind === "saving") {
      return;
    }
    const retryingSavedResponse =
      phase().kind === "failed" &&
      validatedResponse !== undefined &&
      JSON.stringify(validatedResponse) === JSON.stringify(response);
    if (phase().kind !== "ready" && !retryingSavedResponse) {
      await validate(response);
      if (phase().kind !== "ready") return;
    }
    saveRequest += 1;
    const request = saveRequest;
    setPhase({ kind: "saving" });
    try {
      if (props.onSave === undefined) {
        setPhase({ kind: "failed", message: "Response save is unavailable." });
        return;
      }
      const outcome = await props.onSave(response);
      if (disposed || request !== saveRequest) return;
      switch (outcome.kind) {
        case "accepted":
          setPhase({ kind: "saved" });
          return;
        case "rejected":
          setPhase({ kind: "failed", message: outcome.message });
          return;
      }
    } catch (error: unknown) {
      if (disposed || request !== saveRequest) return;
      const message =
        error instanceof Error
          ? `Your response is still available. Save failed: ${error.message}. Try again.`
          : "Your response is still available. Save failed. Try again.";
      setPhase({ kind: "failed", message });
    }
  }

  async function reset(response: StudentResponse): Promise<void> {
    if (disposed) return;
    if (phase().kind === "saving") {
      return;
    }
    // A restored response supersedes every earlier asynchronous format check.
    const editRevision = props.onResponseEdit?.(response);
    latestEdit = { response, revision: editRevision };
    validatedResponse = undefined;
    validationRequest += 1;
    const request = validationRequest;
    setPhase({ kind: "validating" });
    try {
      const check = await validateResponseLocally(props.validator, props.responseFormat, response);
      if (disposed || request !== validationRequest || phase().kind === "saving") return;
      if (check.issues.length === 0) validatedResponse = response;
      props.onResponseChange?.(response, check, editRevision);
      setPhase(
        check.issues.length === 0
          ? { kind: "restored" }
          : { kind: "invalid", message: responseFormatMessage(check) },
      );
    } catch (error: unknown) {
      if (disposed || request !== validationRequest || phase().kind === "saving") return;
      validatedResponse = undefined;
      const message = error instanceof Error ? error.message : "format validation was unavailable";
      setPhase({ kind: "failed", message: `Cannot check this response yet: ${message}.` });
    }
  }

  // A fresh issued control starts neutral. Only a genuinely restored student
  // response should surface format readiness or an error before interaction.
  if (props.initialResponse !== undefined && initialResponse !== undefined) {
    void validate(initialResponse);
  }
  return {
    phase,
    invalid: () => phase().kind === "invalid" || phase().kind === "failed",
    pending: () => phase().kind === "saving",
    locked: () => phase().kind === "saving",
    canSave: () =>
      props.mode !== "formatOnly" &&
      (phase().kind === "ready" ||
        phase().kind === "restored" ||
        phase().kind === "saved" ||
        phase().kind === "failed"),
    canReset: () => phase().kind !== "saving",
    edit,
    validate,
    reset,
    save,
  };
}

export function Status(props: {
  readonly attemptId: string;
  readonly controller: ResponseController;
}): JSX.Element {
  const mode = useContext(ResponseControlModeContext);
  return (
    <p
      id={`${props.attemptId}-format-status`}
      class="format-status"
      classList={{
        error: props.controller.invalid(),
        ready:
          props.controller.phase().kind === "ready" ||
          props.controller.phase().kind === "restored" ||
          props.controller.phase().kind === "saved",
      }}
      role="status"
      aria-label="Response format"
      aria-live="polite"
    >
      {mode === "formatOnly"
        ? formatOnlyPhaseMessage(props.controller.phase())
        : phaseMessage(props.controller.phase())}
    </p>
  );
}

export function Actions(props: {
  readonly disabled: boolean;
  readonly resetDisabled?: boolean;
  readonly onSave: () => void;
  readonly saveLabel?: string;
  readonly onReset?: () => void;
  readonly resetLabel?: "Restore initial response" | "Reset order";
  readonly onEscape: () => void;
}): JSX.Element {
  const mode = useContext(ResponseControlModeContext);
  return (
    <div class="response-actions">
      {mode !== "formatOnly" ? (
        <button
          class="primary-action"
          type="button"
          disabled={props.disabled}
          onClick={props.onSave}
        >
          {props.saveLabel ?? "Save response"}
        </button>
      ) : null}
      {props.onReset === undefined ? null : (
        <button
          class="quiet-action"
          type="button"
          disabled={props.resetDisabled ?? props.disabled}
          onClick={props.onReset}
        >
          {props.resetLabel ?? "Restore initial response"}
        </button>
      )}
      <button class="quiet-action" type="button" onClick={props.onEscape}>
        Return to assessment <span aria-hidden="true">(Esc)</span>
      </button>
    </div>
  );
}
