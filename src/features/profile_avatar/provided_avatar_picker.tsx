// provided_avatar_picker.tsx - catalog-driven provided-avatar controls.

import { type JSX } from "solid-js";

import type { RecordContent } from "../../components/data_sheet/record_content";
import type { DataSheetSelection } from "../../components/data_sheet/data_sheet_state";
import type { RecordField } from "../../components/data_sheet/data_sheet_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
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

function avatarContent(entry: ProvidedAvatarCatalogEntry): RecordContent {
  return {
    title: entry.name,
    description: entry.description,
    details: [],
    media: { src: entry.assetPath, alt: `${entry.name}: ${entry.description}` },
    actions: [],
  };
}

const avatarFields: ReadonlyArray<RecordField<ProvidedAvatarCatalogEntry, "name", never>> = [
  {
    id: "name",
    label: "Avatar",
    role: "identity",
    value: (entry) => entry.name,
    sortKey: "name",
    defaultColumn: { priority: 1 },
  },
  {
    id: "description",
    label: "Description",
    value: (entry) => entry.description,
    defaultColumn: { priority: 2 },
  },
];

/**
 * Presents the selectable generated catalog as native radio controls.
 * Native radios provide Tab entry and Arrow-key movement without a custom focus model.
 */
export function ProvidedAvatarPicker(props: ProvidedAvatarPickerProps): JSX.Element {
  const selectableEntries = (): readonly ProvidedAvatarCatalogEntry[] =>
    PROVIDED_AVATAR_CATALOG.filter((entry) => entry.isSelectable);
  const selection: DataSheetSelection<ProvidedAvatarCatalogEntry> = {
    kind: "radio",
    selectedIds: () =>
      props.currentAvatarId === undefined ? new Set() : new Set([props.currentAvatarId]),
    disabled: () => props.disabled === true,
    onChange: (entry, selected) => {
      if (selected) props.onSelect(entry.id);
    },
  };
  const avatarSearch = createLocalCompleteSearch(
    { rows: selectableEntries },
    {
      fields: avatarFields,
      rowId: (entry) => entry.id,
      keyword: (entry) => `${entry.name} ${entry.description}`,
      content: avatarContent,
    },
  );

  return (
    <fieldset class="provided-avatar-picker">
      <legend>Choose an avatar</legend>
      <p class="provided-avatar-picker-introduction">
        Pick a playful PLE avatar. Your choice is shown with your account where provided avatars are
        supported.
      </p>
      <SearchBlock
        layout="above"
        controls={<SearchControls state={avatarSearch.state} textLabel="Search avatars" />}
        state={avatarSearch.state}
        collection={{
          ariaLabel: "Available provided avatars",
          emptyState: { title: "No avatars match your search." },
          ...avatarSearch.collection,
          selection: () => selection,
        }}
      />
    </fieldset>
  );
}
