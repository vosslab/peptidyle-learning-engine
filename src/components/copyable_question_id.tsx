// copyable_question_id.tsx - one operational instructor-facing Question ID.
import { createSignal, type JSX } from "solid-js";

import { validateCanonicalQuestionIdSyntax } from "../question_id";
import "./copyable_question_id.css";

export interface CopyableQuestionIdProps {
  readonly questionTitle: string;
  readonly displayId: string;
  /**
   * Library rows already display the Question title as their heading. Other
   * callers retain the detailed ID presentation by default.
   */
  readonly presentation?: "detailed" | "compact";
}
export function CopyableQuestionId(props: CopyableQuestionIdProps): JSX.Element {
  // ASVS V2.2.1: display and copy only the allowlisted public Question ID syntax.
  const questionId = validateCanonicalQuestionIdSyntax(props.displayId);
  const [status, setStatus] = createSignal("");
  async function copy(): Promise<void> {
    if (questionId === null) return;
    try {
      await navigator.clipboard.writeText(questionId);
      setStatus(`Copied ${questionId}.`);
    } catch {
      setStatus(`Copy failed. Select ${questionId} and copy it manually.`);
    }
  }
  if (questionId === null) {
    return <p class="copyable-question-id-status">Question ID is unavailable.</p>;
  }
  return (
    <div class="copyable-question-id">
      {props.presentation !== "compact" && <span>{props.questionTitle}</span>}
      <span>Question ID</span>
      <code aria-label={`Question ID ${questionId}`}>{questionId}</code>
      <button
        class="quiet-action"
        type="button"
        aria-label={`Copy Question ID ${questionId}`}
        onClick={() => void copy()}
      >
        Copy ID
      </button>
      <span class="copyable-question-id-status" role="status" aria-live="polite">
        {status()}
      </span>
    </div>
  );
}
