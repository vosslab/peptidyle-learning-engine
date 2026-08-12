// copyable_problem_id.tsx - one operational instructor-facing immutable question identity.

import { createSignal, type JSX } from "solid-js";

import "./copyable_problem_id.css";

export interface CopyableProblemIdProps {
  readonly displayId: string;
}

/** Renders a selectable ID and a keyboard-operable clipboard action with visible recovery. */
export function CopyableProblemId(props: CopyableProblemIdProps): JSX.Element {
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
    <div class="copyable-problem-id">
      <code>{props.displayId}</code>
      <button
        class="quiet-action"
        type="button"
        aria-label={`Copy question ID ${props.displayId}`}
        onClick={() => void copy()}
      >
        Copy ID
      </button>
      <span class="copyable-problem-id-status" role="status" aria-live="polite">
        {status()}
      </span>
    </div>
  );
}
