import { For, Index, Show, createSignal, onCleanup, type JSX } from "solid-js";
import type {
  PleQuestionJsonDocument,
  PleQuestionJsonHotspotRegion,
  PleQuestionJsonHotspotResponse,
} from "./question_json_source";
import { nextHotspotRegionId } from "./question_json_hotspot_model";

export function PleQuestionJsonHotspotEditor(props: {
  readonly source: () => PleQuestionJsonDocument;
  readonly disabled: boolean;
  readonly fieldErrors: Readonly<Record<string, string>>;
  readonly previewPath: (asset: string) => string;
  readonly onUpload: (file: File, signal: AbortSignal) => Promise<void>;
  readonly onEdit: (source: PleQuestionJsonDocument) => void;
  readonly onStatus: (message: string) => void;
  readonly onLiteralValidityChange: (valid: boolean) => void;
}): JSX.Element {
  const [uploading, setUploading] = createSignal(false);
  const [imageUnavailable, setImageUnavailable] = createSignal(false);
  let upload: AbortController | null = null;
  const invalidCoordinates = new Set<string>();
  onCleanup(() => {
    upload?.abort();
    props.onLiteralValidityChange(true);
  });
  const response = (): PleQuestionJsonHotspotResponse | null => {
    const current = props.source().response;
    return current.kind === "hotspot" ? current : null;
  };
  function edit(next: PleQuestionJsonHotspotResponse): void {
    props.onEdit({ ...props.source(), response: next });
  }
  function editRegion(id: string, patch: Partial<PleQuestionJsonHotspotRegion>): void {
    const current = response();
    if (current === null) return;
    edit({
      ...current,
      regions: current.regions.map((region) =>
        region.id === id ? { ...region, ...patch } : region,
      ),
    });
  }
  async function chooseImage(input: HTMLInputElement): Promise<void> {
    const file = input.files?.[0];
    if (file === undefined || uploading() || props.disabled) return;
    upload = new AbortController();
    setUploading(true);
    try {
      await props.onUpload(file, upload.signal);
      setImageUnavailable(false);
    } finally {
      upload = null;
      setUploading(false);
      input.value = "";
    }
  }
  function addRegion(): void {
    const current = response();
    if (current === null || current.regions.length >= 100) return;
    const id = nextHotspotRegionId(current);
    edit({
      ...current,
      regions: [
        ...current.regions,
        { id, label: "New region", x: 0, y: 0, width: 1000, height: 1000 },
      ],
    });
    props.onStatus(
      "New region added. Adjust its coordinates so regions do not overlap before saving.",
    );
  }
  return (
    <fieldset class="ple-question-json-hotspot">
      <legend>Image regions (HOTSPOT)</legend>
      <label class="ple-question-json-authoring__field">
        <span>{response() === null ? "Upload image" : "Replace image"}</span>
        <input
          type="file"
          accept="image/png,image/jpeg,image/webp"
          disabled={props.disabled || uploading()}
          onChange={(event) => void chooseImage(event.currentTarget)}
        />
        <span class="ple-question-json-authoring__help">
          PNG, JPEG, or WebP; maximum 8 MiB. SVG is not yet supported. Save the draft after
          uploading.
        </span>
      </label>
      <Show when={uploading()}>
        <p role="status">Uploading image...</p>
        <button type="button" class="quiet-action" onClick={() => upload?.abort()}>
          Cancel upload
        </button>
      </Show>
      <Show when={response()}>
        <label class="ple-question-json-authoring__field">
          <span>Image description</span>
          <textarea
            aria-invalid={props.fieldErrors["response.surface.description"] !== undefined}
            value={response()?.surface.description ?? ""}
            disabled={props.disabled}
            onInput={(event) => {
              const current = response();
              if (current !== null)
                edit({
                  ...current,
                  surface: { ...current.surface, description: event.currentTarget.value },
                });
            }}
          />
        </label>
        <p id="hotspot-coordinate-help" class="ple-question-json-authoring__help">
          Coordinates run from 0 to 10000 (100% of the image), starting at the top left. Regions
          must stay inside the image and must not overlap. Select all regions that count as correct.
        </p>
        <div class="ple-question-json-hotspot__image">
          {/* ASVS 1.2.1: labels and descriptions are text, never injected HTML. */}
          <img
            src={props.previewPath(response()?.surface.questionAsset ?? "")}
            alt={response()?.surface.description ?? ""}
            onLoad={() => setImageUnavailable(false)}
            onError={() => setImageUnavailable(true)}
          />
          <For each={response()?.regions ?? []}>
            {(region, index) => (
              <span
                aria-hidden="true"
                class="ple-question-json-hotspot__rectangle"
                style={{
                  left: `${region.x / 100}%`,
                  top: `${region.y / 100}%`,
                  width: `${region.width / 100}%`,
                  height: `${region.height / 100}%`,
                }}
              >
                {index() + 1}
              </span>
            )}
          </For>
        </div>
        <Show when={imageUnavailable()}>
          <p role="alert">
            The private image is unavailable. Your draft and region edits are retained.
          </p>
        </Show>
        <Index each={response()?.regions ?? []}>
          {(region, index) => (
            <fieldset>
              <legend>Region {index + 1}</legend>
              <label class="ple-question-json-authoring__field">
                <span>Region {index + 1} label</span>
                <input
                  value={region().label}
                  disabled={props.disabled}
                  onInput={(event) => editRegion(region().id, { label: event.currentTarget.value })}
                />
              </label>
              <div class="ple-question-json-hotspot__coordinates">
                <For each={["x", "y", "width", "height"] as const}>
                  {(coordinate) => (
                    <label class="ple-question-json-authoring__field">
                      <span>
                        Region {index + 1} {coordinate}
                      </span>
                      <input
                        type="number"
                        required
                        min={0}
                        max={10000}
                        step={1}
                        value={region()[coordinate]}
                        disabled={props.disabled}
                        aria-describedby="hotspot-coordinate-help"
                        onInput={(event) => {
                          const value = event.currentTarget.valueAsNumber;
                          const key = `${region().id}.${coordinate}`;
                          if (Number.isFinite(value)) {
                            invalidCoordinates.delete(key);
                            editRegion(region().id, { [coordinate]: value });
                          } else {
                            invalidCoordinates.add(key);
                            props.onStatus("Enter a whole-number coordinate from 0 to 10000.");
                          }
                          props.onLiteralValidityChange(invalidCoordinates.size === 0);
                        }}
                      />
                    </label>
                  )}
                </For>
              </div>
              <label class="ple-question-json-authoring__field">
                <span>
                  <input
                    type="checkbox"
                    checked={response()?.correctRegions.includes(region().id) ?? false}
                    disabled={props.disabled}
                    onChange={(event) => {
                      const current = response();
                      if (current === null) return;
                      const correctRegions = event.currentTarget.checked
                        ? [...current.correctRegions, region().id]
                        : current.correctRegions.filter((id) => id !== region().id);
                      edit({ ...current, correctRegions });
                    }}
                  />
                  Region {index + 1} is correct
                </span>
              </label>
              <button
                type="button"
                class="quiet-action"
                disabled={props.disabled || (response()?.regions.length ?? 0) <= 1}
                onClick={() => {
                  const current = response();
                  if (current === null) return;
                  for (const key of invalidCoordinates)
                    if (key.startsWith(`${region().id}.`)) invalidCoordinates.delete(key);
                  props.onLiteralValidityChange(invalidCoordinates.size === 0);
                  edit({
                    ...current,
                    regions: current.regions.filter((item) => item.id !== region().id),
                    correctRegions: current.correctRegions.filter((id) => id !== region().id),
                  });
                }}
              >
                Remove region {index + 1}
              </button>
            </fieldset>
          )}
        </Index>
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || (response()?.regions.length ?? 0) >= 100}
          onClick={addRegion}
        >
          Add region
        </button>
      </Show>
      <For
        each={Object.entries(props.fieldErrors).filter(([field]) => field.startsWith("response."))}
      >
        {([, message]) => <p role="alert">{message}</p>}
      </For>
    </fieldset>
  );
}
