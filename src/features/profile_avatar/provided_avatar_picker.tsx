// provided_avatar_picker.tsx - catalog-driven provided-avatar controls.

import { For, Show, type JSX } from "solid-js";

import { RecordList } from "../../components/record_list/record_list";
import {
  RECORD_LIST_GALLERY_PRESENTATION_CLASS,
  createRecordListPresentation,
  recordListPresentationSwitcher,
} from "../../components/record_list/record_list_presentation";
import type { RecordRegion } from "../../components/record_list/region_spec";
import {
  PROVIDED_AVATAR_CATALOG,
  type ProvidedAvatarCatalogEntry,
} from "./avatar_catalog_generated";
import "./provided_avatar_picker.css";

export interface AvatarVisualProps {
  /** Closed catalog identifier, supplied by the generated browser registry. */
  readonly avatarId: string;
  /** A square display size in CSS pixels. */
  readonly size?: 24 | 64 | 128;
  /** True when nearby text already supplies this avatar's accessible name. */
  readonly decorative?: boolean;
}

function catalogEntry(avatarId: string): ProvidedAvatarCatalogEntry | undefined {
  return PROVIDED_AVATAR_CATALOG.find((entry) => entry.id === avatarId);
}

/**
 * Renders one static PLE-provided avatar without making a network or API request.
 * Consumers that provide the name separately should set decorative to avoid repeating it.
 */
export function AvatarVisual(props: AvatarVisualProps): JSX.Element {
  const entry = (): ProvidedAvatarCatalogEntry | undefined => catalogEntry(props.avatarId);
  const size = (): 24 | 64 | 128 => props.size ?? 64;

  return (
    <span
      class="provided-avatar-visual"
      classList={{ "provided-avatar-visual-missing": entry() === undefined }}
      style={{ height: `${size()}px`, width: `${size()}px` }}
    >
      {entry() === undefined ? (
        <span aria-label="Unavailable provided avatar" role="img">
          ?
        </span>
      ) : (
        <img
          alt={props.decorative ? "" : `${entry()!.name}: ${entry()!.description}`}
          aria-hidden={props.decorative ? "true" : undefined}
          height={size()}
          src={entry()!.assetPath}
          width={size()}
        />
      )}
    </span>
  );
}

export interface ProvidedAvatarPickerProps {
  readonly currentAvatarId: string | undefined;
  /** Prevents a second selection while the authenticated-self request is pending. */
  readonly disabled?: boolean;
  readonly onSelect: (id: string) => void;
}

type AvatarPickerPresentation = "gallery" | "list";

function galleryRegions(
  props: ProvidedAvatarPickerProps,
): ReadonlyArray<RecordRegion<ProvidedAvatarCatalogEntry>> {
  const regions: ReadonlyArray<RecordRegion<ProvidedAvatarCatalogEntry>> = [
    {
      id: "avatar",
      role: "identity",
      priority: "required",
      width: "minmax(9rem, 1fr)",
      align: "stretch",
      content: (entry): JSX.Element => {
        const inputId = `provided-avatar-gallery-${entry.id}`;
        const selected = (): boolean => props.currentAvatarId === entry.id;
        return (
          <label
            class="provided-avatar-picker-tile"
            classList={{ "provided-avatar-picker-tile-selected": selected() }}
            for={inputId}
          >
            <input
              checked={selected()}
              disabled={props.disabled}
              id={inputId}
              name="provided-avatar"
              type="radio"
              value={entry.id}
              onChange={() => props.onSelect(entry.id)}
            />
            <AvatarVisual avatarId={entry.id} decorative size={64} />
            <span class="provided-avatar-picker-name">{entry.name}</span>
            <span class="provided-avatar-picker-description">{entry.description}</span>
          </label>
        );
      },
    },
  ];
  return regions;
}

function listRegions(
  props: ProvidedAvatarPickerProps,
): ReadonlyArray<RecordRegion<ProvidedAvatarCatalogEntry>> {
  const regions: ReadonlyArray<RecordRegion<ProvidedAvatarCatalogEntry>> = [
    {
      id: "avatar",
      role: "identity",
      priority: "required",
      width: "minmax(12rem, 1fr)",
      align: "start",
      content: (entry): JSX.Element => {
        const inputId = `provided-avatar-list-${entry.id}`;
        const descriptionId = `provided-avatar-list-description-${entry.id}`;
        const selected = (): boolean => props.currentAvatarId === entry.id;
        return (
          <label
            class="provided-avatar-picker-list-choice"
            classList={{ "provided-avatar-picker-list-choice-selected": selected() }}
            for={inputId}
          >
            <input
              aria-describedby={descriptionId}
              checked={selected()}
              disabled={props.disabled}
              id={inputId}
              name="provided-avatar"
              type="radio"
              value={entry.id}
              onChange={() => props.onSelect(entry.id)}
            />
            <AvatarVisual avatarId={entry.id} decorative size={24} />
            <span class="provided-avatar-picker-name">{entry.name}</span>
          </label>
        );
      },
    },
    {
      id: "description",
      role: "metadata",
      priority: "medium",
      width: "minmax(12rem, 2fr)",
      align: "start",
      content: (entry) => (
        <span id={`provided-avatar-list-description-${entry.id}`}>{entry.description}</span>
      ),
    },
  ];
  return regions;
}

/**
 * Presents the selectable generated catalog as native radio controls.
 * Native radios provide Tab entry and Arrow-key movement without a custom focus model.
 */
export function ProvidedAvatarPicker(props: ProvidedAvatarPickerProps): JSX.Element {
  const selectableEntries = (): readonly ProvidedAvatarCatalogEntry[] =>
    PROVIDED_AVATAR_CATALOG.filter((entry) => entry.isSelectable);
  const presentation = createRecordListPresentation<AvatarPickerPresentation>({
    variants: [
      { id: "gallery", label: "Gallery" },
      { id: "list", label: "List" },
    ],
  });
  const presentationSwitcher = recordListPresentationSwitcher(
    presentation,
    "Avatar picker presentation",
  );

  return (
    <fieldset class="provided-avatar-picker">
      <legend>Choose an avatar</legend>
      <p class="provided-avatar-picker-introduction">
        Pick a playful PLE avatar. Your choice is shown with your account where provided avatars are
        supported.
      </p>
      <Show when={presentationSwitcher}>
        {(switcher) => (
          <div
            class="provided-avatar-picker-presentations"
            aria-label={switcher().ariaLabel}
            role="group"
          >
            <For each={switcher().variants}>
              {(variant) => (
                <button
                  aria-pressed={switcher().selectedVariant() === variant.id}
                  class="quiet-action"
                  type="button"
                  onClick={() => switcher().selectVariant(variant.id)}
                >
                  {variant.label}
                </button>
              )}
            </For>
          </div>
        )}
      </Show>
      <Show
        when={presentation.variant() === "gallery"}
        fallback={
          <div aria-label="Choose an avatar" class="provided-avatar-picker-list" role="radiogroup">
            <RecordList
              ariaLabel="Available provided avatars"
              emptyState={{ title: "No provided avatars are available." }}
              recordId={(entry) => entry.id}
              regions={listRegions(props)}
              rows={selectableEntries()}
              state={{ kind: "ready" }}
            />
          </div>
        }
      >
        <div
          aria-label="Choose an avatar"
          class={`provided-avatar-picker-gallery ${RECORD_LIST_GALLERY_PRESENTATION_CLASS}`}
          role="radiogroup"
        >
          <RecordList
            ariaLabel="Available provided avatars"
            emptyState={{ title: "No provided avatars are available." }}
            recordId={(entry) => entry.id}
            regions={galleryRegions(props)}
            rows={selectableEntries()}
            state={{ kind: "ready" }}
          />
        </div>
      </Show>
    </fieldset>
  );
}
