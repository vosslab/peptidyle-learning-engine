// common.tsx - shared browser-safe response-controller contracts and controls.

import { createSignal, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { CourseId } from "../../../generated/api/CourseId";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ExternalToolLaunch } from "../../api/contracts";
import type { SubmissionOutcome } from "../../features/question_attempt/question_attempt_state";
import type {
  StudentResponseFormatCheck,
  StudentResponseFormatIssue,
  WasmFacade,
} from "../../wasm/index";

export type MultipleChoiceDefinition = Extract<QuestionResponseFormat, { kind: "multipleChoice" }>;
export type NumericDefinition = Extract<QuestionResponseFormat, { kind: "numeric" }>;
export type ShortTextDefinition = Extract<QuestionResponseFormat, { kind: "shortText" }>;
export type OrderingDefinition = Extract<QuestionResponseFormat, { kind: "ordering" }>;
export type MultiBlankDefinition = Extract<QuestionResponseFormat, { kind: "multiBlank" }>;
export type MatchingDefinition = Extract<QuestionResponseFormat, { kind: "matching" }>;
export type HotspotDefinition = Extract<QuestionResponseFormat, { kind: "hotspot" }>;

type WidgetPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "validating" }
  | { readonly kind: "ready" }
  | { readonly kind: "restored" }
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "submitting" }
  | { readonly kind: "recoveryPending"; readonly message: string }
  | { readonly kind: "submitted" }
  | { readonly kind: "failed"; readonly message: string };

export interface StudentWorkRouteScope {
  readonly courseId: CourseId;
  readonly assignmentId: AssignmentId;
}

export interface QuestionResponseControlBaseProps {
  readonly attemptId: string;
  /** Response controls require only the key-free local format validation capability. */
  readonly validator: Pick<WasmFacade, "validateResponseFormat">;
  readonly onSubmit: (response: StudentResponse) => Promise<SubmissionOutcome>;
  readonly onEscape: () => void;
  readonly onResponseChange?: (
    response: StudentResponse,
    validation: StudentResponseFormatCheck,
  ) => void;
  /** Exact navigation scope required to activate an external-tool response. */
  readonly studentWorkRoute?: StudentWorkRouteScope;
  readonly beginExternalToolLaunch?: () => Promise<ExternalToolLaunch>;
}

export interface QuestionResponseControlProps extends QuestionResponseControlBaseProps {
  readonly definition: QuestionResponseFormat;
  readonly initialResponse?: StudentResponse;
}

export interface QuestionResponseControlBodyProps<
  D extends QuestionResponseFormat,
> extends QuestionResponseControlBaseProps {
  readonly definition: D;
  readonly initialResponse?: Extract<StudentResponse, { readonly kind: D["kind"] }>;
}

export type MultipleChoiceResponseProps =
  QuestionResponseControlBodyProps<MultipleChoiceDefinition>;

export interface SubmissionController {
  readonly phase: () => WidgetPhase;
  readonly invalid: () => boolean;
  readonly pending: () => boolean;
  readonly locked: () => boolean;
  readonly canSubmit: () => boolean;
  readonly canReset: () => boolean;
  readonly validate: (response: StudentResponse) => Promise<void>;
  /** Restore an unsubmitted response and invalidate any older format check. */
  readonly reset: (response: StudentResponse) => Promise<void>;
  readonly submit: (response: StudentResponse) => Promise<void>;
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

function issueMessage(issue: StudentResponseFormatIssue): string {
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
      return "Place every item in the requested order.";
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
    case "missingUploadReference":
      return "Choose an uploaded file before submitting.";
    case "responseKindMismatch":
      return "This response does not match the question format.";
  }
}

function checkMessage(check: StudentResponseFormatCheck): string {
  const first = check.violations[0];
  return first === undefined ? "Student Response Format is ready to submit." : issueMessage(first);
}

/** Browser-only format check: deliberately has no submit or grading dependency. */
export async function validateResponseLocally(
  validator: Pick<WasmFacade, "validateResponseFormat">,
  definition: QuestionResponseFormat,
  response: StudentResponse,
): Promise<StudentResponseFormatCheck> {
  return validator.validateResponseFormat(definition, response);
}

/** Preserve an empty numeric control as invalid rather than coercing it to zero. */
export function numericResponseFromInput(input: string): StudentResponse {
  return { kind: "numeric", value: input.trim() === "" ? Number.NaN : Number(input) };
}

function phaseMessage(phase: WidgetPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Complete the response, then submit it.";
    case "validating":
      return "Checking response format...";
    case "ready":
      return "Response format is ready to submit.";
    case "restored":
      return "Response restored. Check it, then submit when you are ready.";
    case "invalid":
    case "failed":
      return phase.message;
    case "submitting":
      return "Submitting your response. Please wait.";
    case "recoveryPending":
      return phase.message;
    case "submitted":
      return "Answer submitted. Server feedback will appear when it is released.";
  }
}

/** Key-free validation state machine. Validation never invokes server grading. */
export function createSubmissionController(
  props: QuestionResponseControlProps,
  initialResponse?: StudentResponse,
): SubmissionController {
  const [phase, setPhase] = createSignal<WidgetPhase>({ kind: "idle" });
  let validationRequest = 0;
  let submissionRequest = 0;

  async function validate(response: StudentResponse): Promise<void> {
    if (
      phase().kind === "submitting" ||
      phase().kind === "recoveryPending" ||
      phase().kind === "submitted"
    ) {
      return;
    }
    validationRequest += 1;
    const request = validationRequest;
    setPhase({ kind: "validating" });
    try {
      const report = await validateResponseLocally(props.validator, props.definition, response);
      if (request !== validationRequest || phase().kind === "submitting") return;
      props.onResponseChange?.(response, report);
      setPhase(
        report.violations.length === 0
          ? { kind: "ready" }
          : { kind: "invalid", message: checkMessage(report) },
      );
    } catch (error: unknown) {
      if (request !== validationRequest || phase().kind === "submitting") return;
      const message = error instanceof Error ? error.message : "format validation was unavailable";
      setPhase({ kind: "failed", message: `Cannot check this response yet: ${message}.` });
    }
  }

  async function submit(response: StudentResponse): Promise<void> {
    if (
      phase().kind === "submitting" ||
      phase().kind === "recoveryPending" ||
      phase().kind === "submitted"
    ) {
      return;
    }
    if (phase().kind !== "ready") {
      await validate(response);
      if (phase().kind !== "ready") return;
    }
    submissionRequest += 1;
    const request = submissionRequest;
    setPhase({ kind: "submitting" });
    try {
      const outcome = await props.onSubmit(response);
      if (request !== submissionRequest) return;
      switch (outcome.kind) {
        case "accepted":
          setPhase({ kind: "submitted" });
          return;
        case "recoveryPending":
          setPhase({ kind: "recoveryPending", message: outcome.message });
          return;
        case "rejected":
          setPhase({ kind: "failed", message: outcome.message });
          return;
      }
    } catch (error: unknown) {
      if (request !== submissionRequest) return;
      const message =
        error instanceof Error
          ? `Your response is still available. Submission failed: ${error.message}. Try again.`
          : "Your response is still available. Submission failed. Try again.";
      setPhase({ kind: "failed", message });
    }
  }

  async function reset(response: StudentResponse): Promise<void> {
    if (
      phase().kind === "submitting" ||
      phase().kind === "recoveryPending" ||
      phase().kind === "submitted"
    ) {
      return;
    }
    // A restored response supersedes every earlier asynchronous format report.
    validationRequest += 1;
    const request = validationRequest;
    setPhase({ kind: "validating" });
    try {
      const report = await validateResponseLocally(props.validator, props.definition, response);
      if (request !== validationRequest || phase().kind === "submitting") return;
      props.onResponseChange?.(response, report);
      setPhase(
        report.violations.length === 0
          ? { kind: "restored" }
          : { kind: "invalid", message: checkMessage(report) },
      );
    } catch (error: unknown) {
      if (request !== validationRequest || phase().kind === "submitting") return;
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
    pending: () => phase().kind === "submitting" || phase().kind === "recoveryPending",
    locked: () =>
      phase().kind === "submitting" ||
      phase().kind === "recoveryPending" ||
      phase().kind === "submitted",
    canSubmit: () => phase().kind === "ready" || phase().kind === "restored",
    canReset: () =>
      phase().kind !== "submitting" &&
      phase().kind !== "recoveryPending" &&
      phase().kind !== "submitted",
    validate,
    reset,
    submit,
  };
}

export function Status(props: {
  readonly attemptId: string;
  readonly controller: SubmissionController;
}): JSX.Element {
  return (
    <p
      id={`${props.attemptId}-format-status`}
      class="format-status"
      classList={{
        error: props.controller.invalid(),
        ready:
          props.controller.phase().kind === "ready" ||
          props.controller.phase().kind === "restored" ||
          props.controller.phase().kind === "submitted",
      }}
      role="status"
      aria-label="Response format"
      aria-live="polite"
    >
      {phaseMessage(props.controller.phase())}
    </p>
  );
}

export function Actions(props: {
  readonly disabled: boolean;
  readonly resetDisabled?: boolean;
  readonly onSubmit: () => void;
  readonly onReset?: () => void;
  readonly resetLabel?: "Clear response" | "Reset order";
  readonly onEscape: () => void;
}): JSX.Element {
  return (
    <div class="response-actions">
      <button
        class="primary-action"
        type="button"
        disabled={props.disabled}
        onClick={props.onSubmit}
      >
        Submit answer
      </button>
      {props.onReset === undefined ? null : (
        <button
          class="quiet-action"
          type="button"
          disabled={props.resetDisabled ?? props.disabled}
          onClick={props.onReset}
        >
          {props.resetLabel ?? "Clear response"}
        </button>
      )}
      <button class="quiet-action" type="button" onClick={props.onEscape}>
        Return to assignment <span aria-hidden="true">(Esc)</span>
      </button>
    </div>
  );
}
