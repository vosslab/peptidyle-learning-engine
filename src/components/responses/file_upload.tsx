// file_upload.tsx - secure-upload unavailable-state controller.

import type { JSX } from "solid-js";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
import { Actions, type FileUploadDefinition, type WidgetBodyProps } from "./common";

function acceptedExtensions(definition: FileUploadDefinition): string {
  return definition.acceptedExtensions.length === 0
    ? "Any allowed file type"
    : definition.acceptedExtensions.join(", ");
}

/** Refuse uploads until a tenant-scoped upload-slot contract exists. */
export function FileUploadResponse(props: WidgetBodyProps<FileUploadDefinition>): JSX.Element {
  return (
    <section
      class="response-widget"
      data-phase="unavailable"
      onKeyDown={(event) =>
        handleWidgetKeyDown(
          event,
          props.onEscape,
          () => undefined,
          () => false,
        )
      }
    >
      <h3>File upload is not available for this question yet</h3>
      <p class="field-help">
        This question accepts {acceptedExtensions(props.definition)} up to{" "}
        {props.definition.maxBytes} bytes. Your instructor can use a supported response type while
        secure upload is being enabled.
      </p>
      <p
        id={`${props.attemptId}-format-status`}
        class="format-status error"
        role="status"
        aria-live="polite"
      >
        A secure, tenant-scoped upload slot is required before a file can be submitted.
      </p>
      <Actions disabled onSubmit={() => undefined} onEscape={props.onEscape} />
    </section>
  );
}
