// Optional Hint, Question Feedback, and Worked Solution editor for one Question Pool.

import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import { ApiRequestError } from "../api/http_client/error";
import type { QuestionPoolSupportClient } from "../api/question_pool_support";

export function QuestionPoolSupportEditor(props: {
  readonly client: QuestionPoolSupportClient;
  readonly questionPoolId: QuestionPoolId;
}): JSX.Element {
  const [hint, setHint] = createSignal("");
  const [generalFeedback, setGeneralFeedback] = createSignal("");
  const [workedSolution, setWorkedSolution] = createSignal("");
  const [editNumber, setEditNumber] = createSignal<number | null>(null);
  const [status, setStatus] = createSignal("Loading Pool support...");
  const [busy, setBusy] = createSignal(false);
  const fieldId = (name: string): string => `pool-support-${name}-${props.questionPoolId}`;

  async function load(): Promise<void> {
    setBusy(true);
    setStatus("Loading Pool support...");
    try {
      const support = await props.client.readQuestionPoolSupport(props.questionPoolId);
      setHint(support.hint ?? "");
      setGeneralFeedback(support.generalFeedback ?? "");
      setWorkedSolution(support.workedSolution ?? "");
      setEditNumber(support.questionPoolMetadataEditNumber);
      setStatus("");
    } catch {
      setEditNumber(null);
      setStatus("Could not load Pool support.");
    } finally {
      setBusy(false);
    }
  }

  onMount(() => {
    void load();
  });

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const expected = editNumber();
    if (expected === null) return;
    setBusy(true);
    try {
      const saved = await props.client.saveQuestionPoolSupport({
        questionPoolId: props.questionPoolId,
        questionPoolMetadataEditNumber: expected,
        hint: hint(),
        generalFeedback: generalFeedback(),
        workedSolution: workedSolution(),
      });
      setHint(saved.hint ?? "");
      setGeneralFeedback(saved.generalFeedback ?? "");
      setWorkedSolution(saved.workedSolution ?? "");
      setEditNumber(saved.questionPoolMetadataEditNumber);
      setStatus("Pool support saved.");
    } catch (error) {
      setStatus(
        error instanceof ApiRequestError && error.status === 412
          ? "This Pool changed. Reload Pool support."
          : "Could not save Pool support.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <form aria-label="Pool support" onSubmit={(event) => void save(event)}>
      <fieldset disabled={busy()}>
        <legend>PLE-managed Pool support</legend>
        <p>
          <label for={fieldId("hint")}>Hint</label>
          <textarea
            id={fieldId("hint")}
            value={hint()}
            onInput={(event) => setHint(event.currentTarget.value)}
          />
        </p>
        <p>
          <label for={fieldId("feedback")}>Question Feedback</label>
          <textarea
            id={fieldId("feedback")}
            value={generalFeedback()}
            onInput={(event) => setGeneralFeedback(event.currentTarget.value)}
          />
        </p>
        <p>
          <label for={fieldId("solution")}>Worked Solution</label>
          <textarea
            id={fieldId("solution")}
            value={workedSolution()}
            onInput={(event) => setWorkedSolution(event.currentTarget.value)}
          />
        </p>
        <button type="submit">Save Pool support</button>
        <button type="button" onClick={() => void load()}>
          Reload Pool support
        </button>
      </fieldset>
      <Show when={status()}>
        <p role="status">{status()}</p>
      </Show>
    </form>
  );
}
