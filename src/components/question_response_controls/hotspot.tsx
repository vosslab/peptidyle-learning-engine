// hotspot.tsx - accessible labeled-region control for image hotspots.

import { createSignal, For, Show, type JSX } from "solid-js";

import type { ResponseItemId } from "../../../generated/api/ResponseItemId";
import type { StudentHotspotSelection } from "../../../generated/api/StudentHotspotSelection";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { HotspotRegion } from "../../../generated/api/HotspotRegion";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import { QuestionContentError, resolveSameOriginQuestionImageUrl } from "../question_renderer";
import {
  Actions,
  createResponseController,
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

/** Public Hotspot coordinates use the inclusive 0..10000 normalized image scale. */
export function hotspotRegionStyle(
  region: Pick<HotspotRegion, "x" | "y" | "width" | "height">,
): JSX.CSSProperties {
  return {
    left: `${region.x / 100}%`,
    top: `${region.y / 100}%`,
    width: `${region.width / 100}%`,
    height: `${region.height / 100}%`,
  };
}

/** Resolve the one answer-free surface through its publication or private preview identity. */
export function resolveHotspotImageUrl(
  props: Pick<
    QuestionResponseControlBodyProps<HotspotResponseFormat>,
    | "responseFormat"
    | "questionRevisionTuple"
    | "questionImageUrl"
    | "hotspotDraftQuestionImage"
    | "mode"
  >,
): string | undefined {
  const asset =
    "surface" in props.responseFormat
      ? props.responseFormat.surface.questionImageAssetTuple
      : props.responseFormat.questionImageAssetTuple;
  // ASVS 1.2.2: reuse the exact same-origin asset boundary, never author-provided URLs.
  if (props.questionRevisionTuple !== undefined && props.questionImageUrl !== undefined) {
    return resolveSameOriginQuestionImageUrl(
      asset,
      props.questionRevisionTuple,
      props.questionImageUrl,
    );
  }
  const draft = props.hotspotDraftQuestionImage;
  if (props.mode !== "formatOnly" || draft === undefined) return undefined;
  const url = draft.questionImageUrl(asset);
  const path = `/api/authoring/drafts/${encodeURIComponent(draft.draftQuestion)}/images/${encodeURIComponent(asset.questionImageAssetId)}`;
  if (
    url.origin !== globalThis.location.origin ||
    url.pathname !== path ||
    url.search !== "" ||
    url.hash !== "" ||
    url.username !== "" ||
    url.password !== ""
  ) {
    throw new QuestionContentError(
      "Draft images must use their authorized exact Draft asset route.",
    );
  }
  return url.href;
}

export function HotspotResponse(
  props: QuestionResponseControlBodyProps<HotspotResponseFormat>,
): JSX.Element {
  const restored =
    props.initialResponse?.kind === "hotspot" ? props.initialResponse.selections : [];
  const restoredIds = restored.map((selection) => selection.region);
  const [selected, setSelected] = createSignal<ReadonlyArray<ResponseItemId>>(restoredIds);
  const regions = (): ReadonlyArray<HotspotRegion> =>
    "regions" in props.responseFormat
      ? props.responseFormat.regions
      : props.responseFormat.surface.regions;
  const description = (): string =>
    "description" in props.responseFormat
      ? props.responseFormat.description
      : props.responseFormat.surface.description;
  const imageUrl = (): string | undefined => resolveHotspotImageUrl(props);
  let firstRegion!: HTMLInputElement;
  const selections = (): Array<StudentHotspotSelection> => selected().map((region) => ({ region }));
  const response = (): StudentResponse => ({ kind: "hotspot", selections: selections() });
  const controller = createResponseController(props, response());
  const required = selectionCount(props.responseFormat);
  const progress = (): string | null => selectionProgress(props.responseFormat, selected().length);
  function choose(id: ResponseItemId): void {
    if (controller.locked()) return;
    const next =
      required === 1
        ? [id]
        : selected().includes(id)
          ? selected().filter((selectedId) => selectedId !== id)
          : [...selected(), id];
    setSelected(next);
    void controller.edit({
      kind: "hotspot",
      selections: next.map((region) => ({ region })),
    });
  }
  function save(): void {
    void controller.save(response());
  }
  function reset(): void {
    const next = [...restoredIds];
    setSelected(next);
    void controller.reset({
      kind: "hotspot",
      selections: next.map((region) => ({ region })),
    });
    queueMicrotask(() => firstRegion?.focus());
  }
  return (
    <section
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, save, controller.canSave)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-hotspot-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Choose the labeled image region{required === 1 ? "" : "s"}</legend>
        <p class="keyboard-instructions" id={`${props.attemptId}-hotspot-help`}>
          {description()}. Click or tap a region on the image, or Tab to a labeled region below and
          press Space to select it.
        </p>
        <Show
          when={imageUrl()}
          keyed
          fallback={
            <p class="field-help">The image is unavailable. Use the labeled regions below.</p>
          }
        >
          {(url) => {
            // Each exact image owns its readiness: replacement cannot reuse old load/error state.
            const [imageFailed, setImageFailed] = createSignal(false);
            const [imageLoaded, setImageLoaded] = createSignal(false);
            return (
              <>
                <div class="hotspot-image-surface">
                  <img
                    src={url}
                    alt={description()}
                    draggable={false}
                    onLoad={() => {
                      if (imageUrl() !== url) return;
                      setImageLoaded(true);
                      setImageFailed(false);
                    }}
                    onError={() => {
                      if (imageUrl() !== url) return;
                      setImageLoaded(false);
                      setImageFailed(true);
                    }}
                  />
                  <Show when={imageLoaded() && !imageFailed()}>
                    <For each={regions()}>
                      {(region) => (
                        <button
                          type="button"
                          class="hotspot-image-region"
                          classList={{ selected: selected().includes(region.id) }}
                          style={hotspotRegionStyle(region)}
                          aria-label={textFromBlocks(region.label)}
                          aria-pressed={selected().includes(region.id)}
                          disabled={controller.locked()}
                          tabIndex={-1}
                          onClick={() => choose(region.id)}
                        />
                      )}
                    </For>
                  </Show>
                </div>
                <Show when={imageFailed()}>
                  <p class="inline-error" role="alert">
                    The image could not load. Use the labeled regions below.
                  </p>
                </Show>
              </>
            );
          }}
        </Show>
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
          <For each={regions()}>
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
        disabled={!controller.canSave() || controller.locked()}
        resetDisabled={controller.locked()}
        onSave={save}
        saveLabel={props.saveLabel}
        onReset={reset}
        onEscape={props.onEscape}
      />
    </section>
  );
}
