// Display-only author interaction; response capture remains in the native PLE controls.

import { createEffect, useContext, type JSX } from "solid-js";

import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import { useApplicationApi } from "../api/application_api";
import { postEmbedAppearance } from "../appearance/embed_appearance";
import { AppearanceContext } from "../appearance/appearance_context";

import "./author_content_frame.css";

export function AuthorContentFrame(props: {
  readonly assessmentAttemptId: AssessmentAttemptId;
  readonly position: number;
}): JSX.Element {
  const runtime = useApplicationApi();
  const appearance = useContext(AppearanceContext);
  let frame: HTMLIFrameElement | undefined;
  // The author document intentionally has an opaque sandbox origin. This is
  // a closed cosmetic record, so its one delivery may use `*`.
  const sendAppearance = (): void => {
    if (appearance !== undefined) postEmbedAppearance(frame, appearance.appearance(), "*");
  };
  createEffect(sendAppearance);
  // ASVS 3.2.1, 3.4.5, 15.2.5: preserve an opaque origin with no identity,
  // answer, or control initialization. Only the closed cosmetic appearance
  // record crosses this boundary; the server document also enforces its CSP.
  return (
    <iframe
      ref={(element) => (frame = element)}
      class="author-content-frame"
      src={runtime.client.studentAuthorContentDocumentUrl(
        props.assessmentAttemptId,
        props.position,
      )}
      title={`Interactive content for Question ${props.position}`}
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
      allow=""
      onLoad={sendAppearance}
    />
  );
}
