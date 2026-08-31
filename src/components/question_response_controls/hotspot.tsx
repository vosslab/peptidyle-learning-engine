// hotspot.tsx - accessible candidate-region alternative for image hotspots.

import { createSignal, For, type JSX } from "solid-js";

import type { ResponseItemReference } from "../../../generated/api/ResponseItemReference";
import type { HotspotRegion } from "../../../generated/api/HotspotRegion";
import type { StudentHotspotPoint } from "../../../generated/api/StudentHotspotPoint";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type HotspotDefinition,
  type QuestionResponseControlBodyProps,
} from "./common";

function center(region: HotspotRegion): { x: number; y: number } {
  return {
    x: Math.round(region.x + region.width / 2),
    y: Math.round(region.y + region.height / 2),
  };
}

function selectionCount(definition: HotspotDefinition): number | undefined {
  if (definition.selection.kind === "exactlyOne") return 1;
  if (definition.selection.kind === "exactly") return definition.selection.count;
  return undefined;
}

/** The public response contract may specify a selection rule, never correctness. */
function selectionProgress(definition: HotspotDefinition, count: number): string | null {
  switch (definition.selection.kind) {
    case "exactlyOne":
      return null;
    case "exactly":
      return `${count} selected. Select exactly ${definition.selection.count}.`;
    case "atLeastOne":
      return `${count} selected. Select at least 1.`;
    case "anyNumber":
      return `${count} selected.`;
  }
}

export function HotspotResponse(
  props: QuestionResponseControlBodyProps<HotspotDefinition>,
): JSX.Element {
  const restored = props.initialResponse?.points ?? [];
  const restoredIds = props.definition.regions
    .filter((region) =>
      restored.some((point) => point.x === center(region).x && point.y === center(region).y),
    )
    .map((region) => region.id);
  const [selected, setSelected] = createSignal<ReadonlyArray<ResponseItemReference>>(restoredIds);
  let firstRegion!: HTMLInputElement;
  const points = (): Array<StudentHotspotPoint> =>
    selected().map((id) => center(props.definition.regions.find((region) => region.id === id)!));
  const response = (): StudentResponse => ({ kind: "hotspot", points: points() });
  const controller = createSubmissionController(props, response());
  const required = selectionCount(props.definition);
  const progress = (): string | null => selectionProgress(props.definition, selected().length);
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
      points: next.map((selectedId) =>
        center(props.definition.regions.find((region) => region.id === selectedId)!),
      ),
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
      points: next.map((id) =>
        center(props.definition.regions.find((region) => region.id === id)!),
      ),
    });
    queueMicrotask(() => firstRegion.focus());
  }
  return (
    <section
      class="response-widget"
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
        <p class="keyboard-hint" id={`${props.attemptId}-hotspot-help`}>
          {props.definition.description}. Tab to a region and press Space to select it. This list is
          the primary no-mouse alternative to selecting the image.
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
          <For each={props.definition.regions}>
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
