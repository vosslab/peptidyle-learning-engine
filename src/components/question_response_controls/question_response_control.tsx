// question_response_control.tsx - stable question-agnostic dispatcher for browser-safe response entry.

import type { JSX } from "solid-js";

import { QUESTION_RESPONSE_CONTROL_STYLES } from "../question_response_control_styles";
import { ImathasQuestionBackendResponse } from "./imathas_question_backend";
import { HotspotResponse } from "./hotspot";
import { MatchingResponse } from "./matching";
import { MultipleChoiceResponse as MultipleChoiceController } from "./multiple_choice";
import { MultiBlankResponse } from "./multi_blank";
import { NumericResponse } from "./numeric";
import { OrderingResponse } from "./ordering";
import { ShortTextResponse } from "./short_text";
import {
  ResponseControlModeProvider,
  type QuestionResponseControlProps,
  type QuestionResponseControlBaseProps,
} from "./common";
import type { QuestionPresentationResponseFormat } from "../../../generated/api/QuestionPresentationResponseFormat";

export {
  createSubmissionController,
  numericResponseFromInput,
  validateResponseLocally,
  type MultipleChoiceResponseProps,
  type QuestionResponseControlProps,
} from "./common";
export {
  isImathasQuestionBackendReadyMessage,
  isSafeImathasQuestionBackendLaunchPath,
} from "./imathas_question_backend";
export { handleQuestionResponseControlKeyDown } from "./keyboard";

/** Standalone multiple-choice entry point retained for the reference Assignment Attempt screen. */
export function MultipleChoiceResponse(
  props: import("./common").MultipleChoiceResponseProps,
): JSX.Element {
  return (
    <>
      <style>{QUESTION_RESPONSE_CONTROL_STYLES}</style>
      <MultipleChoiceController {...props} />
    </>
  );
}

function assertNever(value: never): never {
  throw new Error(`Unhandled Question Response Format: ${JSON.stringify(value)}`);
}

/**
 * Render the native control inside its mode provider.  Do not construct this
 * body in the dispatcher: native controls read their mode from context while
 * rendering.
 */
function QuestionResponseControlBody(props: QuestionResponseControlProps): JSX.Element {
  switch (props.responseFormat.kind) {
    case "numeric":
    case "numerical":
      return (
        <NumericResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "numeric" ? props.initialResponse : undefined
          }
        />
      );
    case "multipleChoice":
    case "singleChoice":
    case "multipleAnswer":
      return (
        <MultipleChoiceController
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "multipleChoice" ? props.initialResponse : undefined
          }
        />
      );
    case "shortText":
    case "fillIn":
      return (
        <ShortTextResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "shortText" ? props.initialResponse : undefined
          }
        />
      );
    case "multiBlank":
    case "multiFillIn":
      return (
        <MultiBlankResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "multiBlank" ? props.initialResponse : undefined
          }
        />
      );
    case "matching":
      return (
        <MatchingResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "matching" ? props.initialResponse : undefined
          }
        />
      );
    case "ordering":
      return (
        <OrderingResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "ordering" ? props.initialResponse : undefined
          }
        />
      );
    case "hotspot":
      return (
        <HotspotResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "hotspot" ? props.initialResponse : undefined
          }
        />
      );
    case "imathasQuestionBackend":
      // A dedicated iMathAS launch is a submission-capable integration, not a
      // native format-only control. M12 deliberately does not activate it.
      return (
        props.mode === "formatOnly" || props.onSubmit === undefined ? (
          <p class="calm-status" role="status">
            This response format is not available for local checking.
          </p>
        ) : (
          <ImathasQuestionBackendResponse
            attemptId={props.attemptId}
            onSubmit={props.onSubmit}
            onEscape={props.onEscape}
            onResponseChange={props.onResponseChange}
            studentWorkRoute={props.studentWorkRoute}
            beginImathasQuestionBackendLaunch={props.beginImathasQuestionBackendLaunch}
          />
        )
      );
    default:
      return assertNever(props.responseFormat);
  }
}

/** Exhaustive dispatch point for every browser-safe Question Response Format variant. */
export function QuestionResponseControl(props: QuestionResponseControlProps): JSX.Element {
  return (
    <ResponseControlModeProvider mode={props.mode ?? "submission"}>
      <style>{QUESTION_RESPONSE_CONTROL_STYLES}</style>
      <QuestionResponseControlBody {...props} />
    </ResponseControlModeProvider>
  );
}

/** Student delivery accepts only the response format frozen with its Question Presentation. */
export function QuestionPresentationResponseControl(
  props: QuestionResponseControlBaseProps & {
    readonly responseFormat: QuestionPresentationResponseFormat;
    readonly initialResponse?: import("../../../generated/api/StudentResponse").StudentResponse;
  },
): JSX.Element {
  return <QuestionResponseControl {...props} />;
}
