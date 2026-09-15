// provided_avatar_picker.tsx - catalog-driven provided-avatar controls.

import { For, type JSX } from "solid-js";

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

/**
 * Presents the selectable generated catalog as native radio controls.
 * Native radios provide Tab entry and Arrow-key movement without a custom focus model.
 */
export function ProvidedAvatarPicker(props: ProvidedAvatarPickerProps): JSX.Element {
  const selectableEntries = (): readonly ProvidedAvatarCatalogEntry[] =>
    PROVIDED_AVATAR_CATALOG.filter((entry) => entry.isSelectable);

  return (
    <fieldset class="provided-avatar-picker">
      <legend>Choose a provided avatar</legend>
      <p class="provided-avatar-picker-introduction">
        Pick a playful PLE avatar. Your choice is shown with your account where provided avatars are
        supported.
      </p>
      <div class="provided-avatar-picker-grid" role="radiogroup">
        <For each={selectableEntries()}>
          {(entry) => {
            const inputId = `provided-avatar-${entry.id}`;
            const selected = (): boolean => props.currentAvatarId === entry.id;
            return (
              <label
                class="provided-avatar-picker-tile"
                classList={{ "provided-avatar-picker-tile-selected": selected() }}
                for={inputId}
              >
                <input
                  checked={selected()}
                  id={inputId}
                  name="provided-avatar"
                  type="radio"
                  value={entry.id}
                  disabled={props.disabled}
                  onChange={() => props.onSelect(entry.id)}
                />
                <AvatarVisual avatarId={entry.id} decorative size={64} />
                <span class="provided-avatar-picker-name">{entry.name}</span>
                <span class="provided-avatar-picker-description">{entry.description}</span>
              </label>
            );
          }}
        </For>
      </div>
    </fieldset>
  );
}
