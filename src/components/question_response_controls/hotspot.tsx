// hotspot.tsx - accessible labeled-region control for image hotspots.

import { createSignal, For, type JSX } from "solid-js";

import type { ResponseItemReference } from "../../../generated/api/ResponseItemReference";
import type { StudentHotspotSelection } from "../../../generated/api/StudentHotspotSelection";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type HotspotResponseFormat,
  type QuestionResponseControlBodyProps,
} from "./common";

function selectionCount(responseFormat: HotspotResponseFormat): number | undefined {
  if ("minimum" in responseFormat) {
    return responseFormat.minimum === responseFormat.maximum ? responseFormat.minimum : undefined;
  }
  if (responseFormat.selection.kind === "exactlyOne") return 1;
  if (responseFormat.selection.kind === "exactly") return responseFormat.selection.count;
  return undefined;
}

/** The public response contract may specify a selection rule, never correctness. */
function selectionProgress(responseFormat: HotspotResponseFormat, count: number): string | null {
  if ("minimum" in responseFormat) {
    return `${count} selected. Select from ${responseFormat.minimum} through ${responseFormat.maximum}.`;
  }
  switch (responseFormat.selection.kind) {
    case "exactlyOne":
      return null;
    case "exactly":
      return `${count} selected. Select exactly ${responseFormat.selection.count}.`;
    case "atLeastOne":
      return `${count} selected. Select at least 1.`;
    case "anyNumber":
      return `${count} selected.`;
  }
}

export function HotspotResponse(
  props: QuestionResponseControlBodyProps<HotspotResponseFormat>,
): JSX.Element {
  const restored =
    props.initialResponse?.kind === "hotspot" ? props.initialResponse.selections : [];
  const restoredIds = restored.map((selection) => selection.region);
  const [selected, setSelected] = createSignal<ReadonlyArray<ResponseItemReference>>(restoredIds);
  let firstRegion!: HTMLInputElement;
  const selections = (): Array<StudentHotspotSelection> => selected().map((region) => ({ region }));
  const response = (): StudentResponse => ({ kind: "hotspot", selections: selections() });
  const controller = createSubmissionController(props, response());
  const required = selectionCount(props.responseFormat);
  const progress = (): string | null => selectionProgress(props.responseFormat, selected().length);
  function choose(id: ResponseItemReference): void {
    if (controller.locked()) return;
    const next =
      required === 1
        ? [id]
        : selected().includes(id)
          ? selected().filter((selectedId) => selectedId !== id)
          : [...selected(), id];
    setSelected(next);
    void controller.validate({
      kind: "hotspot",
      selections: next.map((region) => ({ region })),
    });
  }
  function submit(): void {
    void controller.submit(response());
  }
  function reset(): void {
    const next = [...restoredIds];
    setSelected(next);
    void controller.reset({
      kind: "hotspot",
      selections: next.map((region) => ({ region })),
    });
    queueMicrotask(() => firstRegion.focus());
  }
  return (
    <section
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-hotspot-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Choose the labeled image region{required === 1 ? "" : "s"}</legend>
        <p class="keyboard-instructions" id={`${props.attemptId}-hotspot-help`}>
          {"description" in props.responseFormat
            ? props.responseFormat.description
            : props.responseFormat.surface.description}
          . Tab to a region and press Space to select it. This list is the primary no-mouse
          alternative to selecting the image.
        </p>
        {progress() === null ? null : (
          <p
            class="completion-progress"
            role="status"
            aria-label="Selection count"
            aria-live="polite"
          >
            {progress()}
          </p>
        )}
        <div class="choice-list">
          <For
            each={
              "regions" in props.responseFormat
                ? props.responseFormat.regions
                : props.responseFormat.surface.regions
            }
          >
            {(region, index) => (
              <label class="choice-card" classList={{ selected: selected().includes(region.id) }}>
                <input
                  id={`${props.attemptId}-hotspot-${index()}`}
                  type={required === 1 ? "radio" : "checkbox"}
                  name={`hotspot-${props.attemptId}`}
                  checked={selected().includes(region.id)}
                  ref={
                    index() === 0
                      ? (element): void => {
                          firstRegion = element;
                        }
                      : undefined
                  }
                  onChange={() => choose(region.id)}
                />
                <span>{textFromBlocks(region.label)}</span>
              </label>
            )}
          </For>
        </div>
      </fieldset>
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.locked()}
        resetDisabled={controller.locked()}
        onSubmit={submit}
        onReset={reset}
        onEscape={props.onEscape}
      />
    </section>
  );
}
