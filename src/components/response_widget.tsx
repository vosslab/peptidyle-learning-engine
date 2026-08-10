// response_widget.tsx - stable question-agnostic dispatcher for browser-safe response entry.

import type { JSX } from "solid-js";

import { RESPONSE_WIDGET_STYLES } from "./response_widget_styles";
import { ExternalToolResponse } from "./response_widget/external_tool";
import { FileUploadResponse } from "./responses/file_upload";
import { HotspotResponse } from "./responses/hotspot";
import { MatchingResponse } from "./responses/matching";
import { MultipleChoiceResponse as MultipleChoiceController } from "./responses/multiple_choice";
import { MultiBlankResponse } from "./responses/multi_blank";
import { NumericResponse } from "./responses/numeric";
import { OrderingResponse } from "./responses/ordering";
import { ShortTextResponse } from "./responses/short_text";
import type { ResponseWidgetProps } from "./responses/common";

export {
  createSubmissionController,
  numericResponseFromInput,
  validateResponseLocally,
  type MultipleChoiceResponseProps,
  type ResponseWidgetProps,
} from "./responses/common";
export {
  isExternalToolReadyMessage,
  isSafeExternalToolLaunchPath,
} from "./response_widget/external_tool";
export { handleWidgetKeyDown } from "./response_widget/keyboard";

/** Standalone multiple-choice entry point retained for the reference run screen. */
export function MultipleChoiceResponse(
  props: import("./responses/common").MultipleChoiceResponseProps,
): JSX.Element {
  return (
    <>
      <style>{RESPONSE_WIDGET_STYLES}</style>
      <MultipleChoiceController {...props} />
    </>
  );
}

function assertNever(value: never): never {
  throw new Error(`Unhandled response definition: ${JSON.stringify(value)}`);
}

/** Exhaustive dispatch point for every browser-safe ResponseDefinition variant. */
export function ResponseWidget(props: ResponseWidgetProps): JSX.Element {
  // File uploads remain unavailable until their secure, tenant-scoped upload slot contract exists.
  let body: JSX.Element;
  switch (props.definition.kind) {
    case "numeric":
      body = (
        <NumericResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "numeric" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "multipleChoice":
      body = (
        <MultipleChoiceController
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "multipleChoice" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "shortText":
      body = (
        <ShortTextResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "shortText" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "multiBlank":
      body = (
        <MultiBlankResponse
          {...props}
          definition={props.definition}
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
          definition={props.definition}
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
          definition={props.definition}
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
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "hotspot" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "fileUpload":
      body = (
        <FileUploadResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "fileUpload" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "externalTool":
      body = (
        <ExternalToolResponse
          attemptId={props.attemptId}
          onSubmit={props.onSubmit}
          onEscape={props.onEscape}
          onResponseChange={props.onResponseChange}
          getExternalToolLaunch={props.getExternalToolLaunch}
        />
      );
      break;
    default:
      body = assertNever(props.definition);
  }
  return (
    <>
      <style>{RESPONSE_WIDGET_STYLES}</style>
      {body}
    </>
  );
}
