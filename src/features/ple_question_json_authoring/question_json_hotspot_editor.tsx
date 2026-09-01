// question_json_hotspot_editor.tsx - keyboard-first authoring for normalized hotspot regions.

import { For, Show, type JSX } from "solid-js";

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type {
  PleQuestionJsonAssetClient,
  PleQuestionJsonAssetDescriptor,
} from "./question_json_asset_client";
import { PleQuestionJsonHotspotAssetPicker } from "./question_json_hotspot_asset_picker";
import type { PleQuestionJsonHotspotResponse } from "./question_json_source";

export interface PleQuestionJsonHotspotEditorProps {
  readonly response: () => PleQuestionJsonHotspotResponse | null;
  readonly assetClient: PleQuestionJsonAssetClient | undefined;
  readonly workspace: WorkspaceId;
  readonly pendingDescription: () => string;
  readonly fieldErrors: Readonly<Record<string, string | undefined>>;
  readonly disabled: boolean;
  readonly onSelectAsset: (asset: PleQuestionJsonAssetDescriptor) => void;
  readonly onPendingDescriptionChange: (description: string) => void;
  readonly onDescriptionChange: (description: string) => void;
  readonly onRegionLabelChange: (regionId: string, label: string) => void;
  readonly onRegionCoordinateChange: (
    regionId: string,
    coordinate: "x" | "y" | "width" | "height",
    value: number,
  ) => void;
  readonly onCorrectChange: (regionId: string, correct: boolean) => void;
  readonly onAddRegion: () => void;
  readonly onRemoveRegion: (regionId: string) => void;
  readonly onMoveRegion: (regionId: string, direction: "earlier" | "later") => void;
  readonly onStatus: (message: string) => void;
}

function integerValue(value: string): number | null {
  if (!/^[0-9]+$/u.test(value)) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

/** The visual image overlay is deliberately deferred; the labeled list is the complete author path. */
export function PleQuestionJsonHotspotEditor(
  props: PleQuestionJsonHotspotEditorProps,
): JSX.Element {
  const response = (): PleQuestionJsonHotspotResponse | null => props.response();
  const selectedAssetId = (): string | undefined => response()?.surface.asset;
  const description = (): string => response()?.surface.description ?? props.pendingDescription();
  const clientUnavailable = (): boolean => props.assetClient === undefined;

  function coordinateError(regionId: string, coordinate: string): string | undefined {
    return props.fieldErrors[`response.regions.${regionId}.${coordinate}`];
  }

  return (
    <fieldset class="ple-question-json-authoring__hotspot">
      <legend>Image hotspot</legend>
      <p class="ple-question-json-authoring__help">
        Select a verified image, describe it for students, then define labeled regions with the
        keyboard. Coordinates use a scale-independent 0 through 10,000 surface.
      </p>
      <Show when={clientUnavailable()}>
        <p class="ple-question-json-authoring__error" role="alert">
          Image selection is unavailable in this editor session. Return after the private media
          service is available; this incomplete hotspot is not saved or reviewed.
        </p>
      </Show>
      <Show when={props.assetClient}>
        {(client) => (
          <PleQuestionJsonHotspotAssetPicker
            client={client()}
            workspace={props.workspace}
            purpose="Instructor-selected hotspot surface"
            selectedAssetId={selectedAssetId()}
            disabled={props.disabled}
            onSelect={props.onSelectAsset}
          />
        )}
      </Show>
      <label class="ple-question-json-authoring__field">
        <span>Image description for Students</span>
        <textarea
          value={description()}
          disabled={props.disabled || clientUnavailable()}
          aria-invalid={props.fieldErrors["response.surface.description"] !== undefined}
          onInput={(event) => {
            const next = event.currentTarget.value;
            if (response() === null) props.onPendingDescriptionChange(next);
            else props.onDescriptionChange(next);
          }}
        />
        <span class="ple-question-json-authoring__help">
          Explain what students should inspect. The answer remains limited to the labeled regions.
        </span>
      </label>
      <Show when={response() === null}>
        <section class="ple-question-json-authoring__error" role="status">
          <p>
            Choose a verified image and add its student-facing description to start this hotspot.
          </p>
        </section>
      </Show>
      <Show when={response()}>
        {(hotspot) => (
          <>
            <Show when={props.fieldErrors["response.regions"] !== undefined}>
              <p class="ple-question-json-authoring__error" role="alert">
                {props.fieldErrors["response.regions"]}
              </p>
            </Show>
            <ol class="ple-question-json-authoring__choice-list" aria-label="Labeled image regions">
              <For each={hotspot().regions}>
                {(region, index) => (
                  <li class="ple-question-json-authoring__choice">
                    <div class="ple-question-json-authoring__choice-header">
                      <h3 class="ple-question-json-authoring__choice-title">
                        Region {index() + 1}
                      </h3>
                      <div class="ple-question-json-authoring__row-actions">
                        <button
                          type="button"
                          class="quiet-action"
                          disabled={props.disabled || index() === 0}
                          onClick={() => props.onMoveRegion(region.id, "earlier")}
                        >
                          Earlier
                        </button>
                        <button
                          type="button"
                          class="quiet-action"
                          disabled={props.disabled || index() === hotspot().regions.length - 1}
                          onClick={() => props.onMoveRegion(region.id, "later")}
                        >
                          Later
                        </button>
                        <button
                          type="button"
                          class="quiet-action"
                          disabled={props.disabled || hotspot().regions.length <= 1}
                          onClick={() => props.onRemoveRegion(region.id)}
                        >
                          Remove region
                        </button>
                      </div>
                    </div>
                    <label class="ple-question-json-authoring__field">
                      <span>Region label</span>
                      <input
                        value={region.label}
                        disabled={props.disabled}
                        aria-invalid={coordinateError(region.id, "label") !== undefined}
                        onInput={(event) =>
                          props.onRegionLabelChange(region.id, event.currentTarget.value)
                        }
                      />
                    </label>
                    <label class="ple-question-json-authoring__check">
                      <input
                        type="checkbox"
                        checked={hotspot().correctRegions.includes(region.id)}
                        disabled={props.disabled}
                        onChange={(event) =>
                          props.onCorrectChange(region.id, event.currentTarget.checked)
                        }
                      />
                      <span>Correct region</span>
                    </label>
                    <div
                      class="ple-question-json-authoring__grid"
                      aria-label={`Region ${index() + 1} coordinates`}
                    >
                      <For each={["x", "y", "width", "height"] as const}>
                        {(coordinate) => {
                          const error = (): string | undefined =>
                            coordinateError(region.id, coordinate);
                          return (
                            <label class="ple-question-json-authoring__field">
                              <span>
                                {coordinate === "x"
                                  ? "X"
                                  : coordinate === "y"
                                    ? "Y"
                                    : coordinate === "width"
                                      ? "Width"
                                      : "Height"}
                              </span>
                              <input
                                type="number"
                                min="0"
                                max="10000"
                                step="1"
                                inputmode="numeric"
                                value={region[coordinate]}
                                disabled={props.disabled}
                                aria-invalid={error() !== undefined}
                                onInput={(event) => {
                                  const next = integerValue(event.currentTarget.value);
                                  if (next === null) {
                                    props.onStatus(
                                      "Use a whole-number coordinate from 0 through 10,000.",
                                    );
                                    return;
                                  }
                                  props.onRegionCoordinateChange(region.id, coordinate, next);
                                }}
                              />
                              <Show when={error() !== undefined}>
                                <span class="ple-question-json-authoring__error" role="alert">
                                  {error()}
                                </span>
                              </Show>
                            </label>
                          );
                        }}
                      </For>
                    </div>
                  </li>
                )}
              </For>
            </ol>
            <div class="ple-question-json-authoring__actions">
              <button
                type="button"
                class="quiet-action"
                disabled={props.disabled || hotspot().regions.length >= 100}
                onClick={props.onAddRegion}
              >
                Add labeled region
              </button>
              <p class="ple-question-json-authoring__help">
                Pointer overlay authoring is optional future work. This labeled list is the complete
                keyboard authoring path.
              </p>
            </div>
          </>
        )}
      </Show>
    </fieldset>
  );
}
