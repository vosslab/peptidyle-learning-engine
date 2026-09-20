// Display-only author interaction; response capture remains in the native PLE controls.

import type { JSX } from "solid-js";

import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import { useApplicationApi } from "../api/application_api";

import "./author_content_frame.css";

export function AuthorContentFrame(props: {
  readonly assessmentAttemptId: AssessmentAttemptId;
  readonly position: number;
}): JSX.Element {
  const runtime = useApplicationApi();
  // ASVS 3.2.1, 3.4.5, 15.2.5: preserve an opaque origin with no parent
  // initialization or message listener. The server document also enforces its CSP.
  return (
    <iframe
      class="author-content-frame"
      src={runtime.client.studentAuthorContentDocumentUrl(
        props.assessmentAttemptId,
        props.position,
      )}
      title={`Interactive content for Question ${props.position}`}
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
      allow=""
    />
  );
}
