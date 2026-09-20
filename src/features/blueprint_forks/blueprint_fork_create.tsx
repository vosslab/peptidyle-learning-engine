// One explicit fork command for the displayed immutable Blueprint Revision.

import { useNavigate } from "@solidjs/router";
import { Show, createSignal, onCleanup, type JSX } from "solid-js";

import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintCourseClient } from "../../api/blueprint_course";

export interface BlueprintForkCreateProps {
  readonly client: BlueprintCourseClient;
  readonly source: BlueprintCourseView;
}

/** Creates a separately owned Private fork, without changing the source. */
export function BlueprintForkCreate(props: BlueprintForkCreateProps): JSX.Element {
  const navigate = useNavigate();
  const [pending, setPending] = createSignal(false);
  const [error, setError] = createSignal("");
  let action:
    | {
        readonly blueprintCourseId: string;
        readonly revisionNumber: string;
        readonly key: string;
      }
    | undefined;
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  function canFork(): boolean {
    // ASVS 8.3.1: visibility is only a UI affordance; the server authorizes the command.
    return props.source.availability === "public" || props.source.availability === "archived";
  }

  async function createFork(): Promise<void> {
    if (pending() || !canFork()) return;
    const blueprintCourseId = props.source.id;
    const revisionNumber = props.source.current_revision_tuple.revisionNumber;
    setPending(true);
    setError("");
    try {
      // ASVS 2.3.1: uncertain retries retain this operation's key and exact source Revision.
      if (
        action?.blueprintCourseId !== blueprintCourseId ||
        action.revisionNumber !== revisionNumber
      ) {
        action = { blueprintCourseId, revisionNumber, key: crypto.randomUUID() };
      }
      const result = await props.client.forkBlueprintCourse(
        blueprintCourseId,
        revisionNumber,
        action.key,
      );
      if (disposed) return;
      // ASVS 1.2.2: navigate only to a fixed local route with an encoded Blueprint Course ID.
      navigate(`/blueprint-courses/${encodeURIComponent(result.blueprintCourse.id)}`);
    } catch {
      // ASVS 16.5.1: do not display transport internals or response bodies.
      if (!disposed)
        setError(
          "The fork could not be confirmed. Retry to confirm the same fork request; the source Blueprint Course is unchanged.",
        );
    } finally {
      if (!disposed) setPending(false);
    }
  }

  return (
    <Show when={canFork()}>
      <div class="blueprint-course-save-actions" aria-busy={pending()}>
        <p>
          Fork Revision {props.source.current_revision_tuple.revisionNumber} into your own
          independent Private Blueprint Course. Later source changes are not applied automatically.
        </p>
        <button type="button" disabled={pending()} onClick={() => void createFork()}>
          {pending()
            ? "Creating fork..."
            : error()
              ? "Retry fork creation"
              : "Fork Blueprint Course"}
        </button>
        <Show when={error()}>
          <p role="alert">{error()}</p>
        </Show>
      </div>
    </Show>
  );
}
