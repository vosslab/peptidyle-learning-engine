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
import type { QuestionResponseControlProps, QuestionResponseControlBaseProps } from "./common";
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

/** Exhaustive dispatch point for every browser-safe Question Response Format variant. */
export function QuestionResponseControl(props: QuestionResponseControlProps): JSX.Element {
  let body: JSX.Element;
  switch (props.responseFormat.kind) {
    case "numeric":
    case "numerical":
      body = (
        <NumericResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "numeric" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "multipleChoice":
    case "singleChoice":
    case "multipleAnswer":
      body = (
        <MultipleChoiceController
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "multipleChoice" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "shortText":
    case "fillIn":
      body = (
        <ShortTextResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "shortText" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "multiBlank":
    case "multiFillIn":
      body = (
        <MultiBlankResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "multiBlank" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "matching":
      body = (
        <MatchingResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "matching" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "ordering":
      body = (
        <OrderingResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "ordering" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "hotspot":
      body = (
        <HotspotResponse
          {...props}
          responseFormat={props.responseFormat}
          initialResponse={
            props.initialResponse?.kind === "hotspot" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "imathasQuestionBackend":
      body = (
        <ImathasQuestionBackendResponse
          attemptId={props.attemptId}
          onSubmit={props.onSubmit}
          onEscape={props.onEscape}
          onResponseChange={props.onResponseChange}
          studentWorkRoute={props.studentWorkRoute}
          beginImathasQuestionBackendLaunch={props.beginImathasQuestionBackendLaunch}
        />
      );
      break;
    default:
      body = assertNever(props.responseFormat);
  }
  return (
    <>
      <style>{QUESTION_RESPONSE_CONTROL_STYLES}</style>
      {body}
    </>
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
