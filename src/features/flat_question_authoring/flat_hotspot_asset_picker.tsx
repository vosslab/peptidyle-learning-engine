// flat_hotspot_asset_picker.tsx - private, keyboard-first selection of immutable hotspot images.

import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import {
  FlatQuestionAssetProtocolError,
  FlatQuestionAssetRequestError,
  type FlatQuestionAssetClient,
  type FlatQuestionAssetDescriptor,
} from "./flat_question_asset_client";

type AssetPickerPhase = "ready" | "loading" | "uploading";

export interface FlatHotspotAssetPickerProps {
  readonly client: FlatQuestionAssetClient;
  readonly workspace: WorkspaceId;
  /** Stable instructional provenance transmitted with each author-selected image. */
  readonly provenance: string;
  /** Retains an existing source selection when this surface reloads. */
  readonly selectedAssetId?: string;
  readonly disabled?: boolean;
  readonly onSelect: (asset: FlatQuestionAssetDescriptor) => void;
}

function userMessage(error: unknown): string {
  if (error instanceof FlatQuestionAssetProtocolError) return error.message;
  if (error instanceof FlatQuestionAssetRequestError) {
    switch (error.status) {
      case 401:
        return "Your session ended. Sign in again, then choose the preserved image.";
      case 403:
      case 404:
        return "This question is no longer available to this account. Your current image choice remains visible.";
      case 413:
        return "That image is too large. Choose a smaller PNG, JPEG, or WebP image.";
      case 415:
      case 422:
        return "Choose a complete still PNG, JPEG, or WebP image.";
      default:
        return "The image library is unavailable. Your current image choice remains visible; try again.";
    }
  }
  return "The image library is unavailable. Your current image choice remains visible; try again.";
}

/** Never shows browser paths or storage metadata; returns only a safe immutable descriptor upstream. */
export function FlatHotspotAssetPicker(props: FlatHotspotAssetPickerProps): JSX.Element {
  const [assets, setAssets] = createSignal<ReadonlyArray<FlatQuestionAssetDescriptor>>([]);
  const [selectedAssetId, setSelectedAssetId] = createSignal(props.selectedAssetId ?? "");
  const [selectedFile, setSelectedFile] = createSignal<File | null>(null);
  const [phase, setPhase] = createSignal<AssetPickerPhase>("loading");
  const [errorMessage, setErrorMessage] = createSignal<string | null>(null);
  const [statusMessage, setStatusMessage] = createSignal<string | null>(null);
  let fileInput: HTMLInputElement | undefined;

  const busy = (): boolean => phase() !== "ready";
  const unavailable = (): boolean => props.disabled === true || busy();

  function selectedAsset(): FlatQuestionAssetDescriptor | undefined {
    const fromParent = props.selectedAssetId;
    const target = fromParent === undefined ? selectedAssetId() : fromParent;
    return assets().find((asset) => asset.assetId === target);
  }

  async function loadAssets(): Promise<void> {
    setPhase("loading");
    setStatusMessage(null);
    try {
      const loaded = await props.client.list(props.workspace);
      setAssets(loaded);
      setErrorMessage(null);
      setPhase("ready");
    } catch (error: unknown) {
      setErrorMessage(userMessage(error));
      setPhase("ready");
    }
  }

  function selectAsset(assetId: string): void {
    setSelectedAssetId(assetId);
    const asset = assets().find((candidate) => candidate.assetId === assetId);
    if (asset === undefined) return;
    props.onSelect(asset);
    setErrorMessage(null);
    setStatusMessage(`Selected ${asset.displayLabel}.`);
  }

  function selectFile(selected: File | null): void {
    setSelectedFile(selected);
    setStatusMessage(selected === null ? null : "Image ready to upload.");
  }

  async function uploadSelectedFile(): Promise<void> {
    const file = selectedFile();
    if (file === null) {
      setErrorMessage("Choose an image file before uploading.");
      return;
    }
    setPhase("uploading");
    setStatusMessage(null);
    try {
      const asset = await props.client.upload(props.workspace, {
        image: file,
        displayLabel: file.name,
        provenance: props.provenance,
      });
      setAssets((current) => [...current, asset]);
      setSelectedAssetId(asset.assetId);
      setSelectedFile(null);
      if (fileInput !== undefined) fileInput.value = "";
      props.onSelect(asset);
      setErrorMessage(null);
      setStatusMessage(`Uploaded and selected ${asset.displayLabel}.`);
      setPhase("ready");
    } catch (error: unknown) {
      setErrorMessage(userMessage(error));
      setPhase("ready");
    }
  }

  onMount(() => {
    void loadAssets();
  });

  return (
    <fieldset class="flat-question-authoring__hotspot-assets" aria-describedby="hotspot-image-help">
      <legend>Image</legend>
      <p class="flat-question-authoring__help" id="hotspot-image-help">
        Choose one immutable image for the hotspot surface. The server verifies the image and keeps
        its storage details private.
      </p>
      <label class="flat-question-authoring__field">
        <span>Image</span>
        <select
          value={selectedAsset()?.assetId ?? selectedAssetId()}
          disabled={unavailable()}
          onChange={(event) => selectAsset(event.currentTarget.value)}
        >
          <option value="">Choose an image</option>
          <For each={assets()}>
            {(asset) => (
              <option value={asset.assetId}>
                {asset.displayLabel} ({asset.intrinsicWidth} by {asset.intrinsicHeight})
              </option>
            )}
          </For>
        </select>
      </label>
      <Show when={phase() === "loading"}>
        <p class="flat-question-authoring__help" role="status">
          Loading your image library...
        </p>
      </Show>
      <Show when={phase() === "ready" && assets().length === 0 && errorMessage() === null}>
        <p class="flat-question-authoring__help">No images yet. Add the first image below.</p>
      </Show>
      <Show when={errorMessage() !== null}>
        <div class="flat-question-authoring__error" role="alert">
          <p>{errorMessage()}</p>
          <button
            type="button"
            class="quiet-action"
            disabled={unavailable()}
            onClick={() => void loadAssets()}
          >
            Retry image library
          </button>
        </div>
      </Show>
      <Show when={statusMessage() !== null}>
        <p class="flat-question-authoring__help" role="status">
          {statusMessage()}
        </p>
      </Show>
      <div class="flat-question-authoring__asset-upload">
        <label class="flat-question-authoring__field">
          <span>Add image file</span>
          <input
            ref={(element: HTMLInputElement) => {
              fileInput = element;
            }}
            type="file"
            accept="image/png,image/jpeg,image/webp"
            disabled={unavailable()}
            onInput={(event) => selectFile(event.currentTarget.files?.item(0) ?? null)}
          />
        </label>
        <button
          type="button"
          class="quiet-action"
          disabled={unavailable() || selectedFile() === null}
          onClick={() => void uploadSelectedFile()}
        >
          Upload image
        </button>
      </div>
    </fieldset>
  );
}
