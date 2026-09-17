// copyable_question_id.tsx - one operational instructor-facing Question reference.
import { createSignal, type JSX } from "solid-js";

import { validateCanonicalQuestionIdSyntax } from "../question_id";
import "./copyable_question_id.css";

export interface CopyableQuestionIdProps {
  readonly questionTitle: string;
  readonly displayId: string;
  /**
   * Library rows already display the Question title as their heading. Other
   * callers retain the detailed reference presentation by default.
   */
  readonly presentation?: "detailed" | "compact";
}
export function CopyableQuestionId(props: CopyableQuestionIdProps): JSX.Element {
  // ASVS V2.2.1: display and copy only the allowlisted public Question-reference syntax.
  const questionReference = validateCanonicalQuestionIdSyntax(props.displayId);
  const [status, setStatus] = createSignal("");
  async function copy(): Promise<void> {
    if (questionReference === null) return;
    try {
      await navigator.clipboard.writeText(questionReference);
      setStatus(`Copied ${questionReference}.`);
    } catch {
      setStatus(`Copy failed. Select ${questionReference} and copy it manually.`);
    }
  }
  if (questionReference === null) {
    return <p class="copyable-question-id-status">Question reference is unavailable.</p>;
  }
  return (
    <div class="copyable-question-id">
      {props.presentation !== "compact" && <span>{props.questionTitle}</span>}
      <span>Question reference</span>
      <code aria-label={`Question reference ${questionReference}`}>{questionReference}</code>
      <button
        class="quiet-action"
        type="button"
        aria-label={`Copy question reference ${questionReference}`}
        onClick={() => void copy()}
      >
        Copy reference
      </button>
      <span class="copyable-question-id-status" role="status" aria-live="polite">
        {status()}
      </span>
    </div>
  );
}
