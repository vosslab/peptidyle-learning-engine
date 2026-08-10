// hotspot.tsx - accessible candidate-region alternative for image hotspots.

import { createSignal, For, type JSX } from "solid-js";

import type { ChoiceId } from "../../../generated/api/ChoiceId";
import type { HotspotRegion } from "../../../generated/api/HotspotRegion";
import type { HotspotPoint } from "../../../generated/api/HotspotPoint";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type HotspotDefinition,
  type WidgetBodyProps,
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

export function HotspotResponse(props: WidgetBodyProps<HotspotDefinition>): JSX.Element {
  const restored = props.initialResponse?.points ?? [];
  const restoredIds = props.definition.regions
    .filter((region) =>
      restored.some((point) => point.x === center(region).x && point.y === center(region).y),
    )
    .map((region) => region.id);
  const [selected, setSelected] = createSignal<ReadonlyArray<ChoiceId>>(restoredIds);
  const points = (): Array<HotspotPoint> =>
    selected().map((id) => center(props.definition.regions.find((region) => region.id === id)!));
  const response = (): StudentResponse => ({ kind: "hotspot", points: points() });
  const controller = createSubmissionController(props, response());
  const required = selectionCount(props.definition);
  function choose(id: ChoiceId): void {
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
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-hotspot-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
      >
        <legend>Choose the labeled image region{required === 1 ? "" : "s"}</legend>
        <p class="keyboard-hint" id={`${props.attemptId}-hotspot-help`}>
          {props.definition.description}. Tab to a region and press Space to select it. This list is
          the primary no-mouse alternative to selecting the image.
        </p>
        <div class="choice-list">
          <For each={props.definition.regions}>
            {(region, index) => (
              <label class="choice-card" classList={{ selected: selected().includes(region.id) }}>
                <input
                  id={`${props.attemptId}-hotspot-${index()}`}
                  type={required === 1 ? "radio" : "checkbox"}
                  name={`hotspot-${props.attemptId}`}
                  checked={selected().includes(region.id)}
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
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}
