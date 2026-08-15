// copyable_question_id.tsx - one operational instructor-facing Question ID.
import { createSignal, type JSX } from "solid-js";
import "./copyable_question_id.css";

export interface CopyableQuestionIdProps {
  readonly displayId: string;
}
export function CopyableQuestionId(props: CopyableQuestionIdProps): JSX.Element {
  const [status, setStatus] = createSignal("");
  async function copy(): Promise<void> {
    try {
      await navigator.clipboard.writeText(props.displayId);
      setStatus(`Copied ${props.displayId}.`);
    } catch {
      setStatus(`Copy failed. Select ${props.displayId} and copy it manually.`);
    }
  }
  return (
    <div class="copyable-question-id">
      <code>{props.displayId}</code>
      <button
        class="quiet-action"
        type="button"
        aria-label={`Copy question ID ${props.displayId}`}
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
