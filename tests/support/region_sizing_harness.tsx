import { render } from "solid-js/web";

import { OpaqueWebworkPreviewFrame } from "../../src/components/opaque_webwork_preview_frame";

export function mountQuestionPreview(target: HTMLElement): void {
  render(
    () => (
      <OpaqueWebworkPreviewFrame
        class="question-preview"
        src={`${window.location.origin}/preview-poster.html`}
        title="Question preview"
      />
    ),
    target,
  );
}
