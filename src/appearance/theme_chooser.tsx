// theme_chooser.tsx - shared five-color Theme selection cards.

import { For, type Accessor, type JSX } from "solid-js";

import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { Theme } from "../../generated/api/Theme";
import { THEME_OPTIONS, themeStyle, themeTokens } from "./theme_registry";

import "./theme_chooser.css";

export interface ThemeChooserProps {
  /** Radio-group name, unique within the page form. */
  readonly name: string;
  readonly selectedTheme: Accessor<Theme>;
  readonly mode: Accessor<DisplayMode>;
  readonly disabled?: Accessor<boolean>;
  readonly onSelect: (theme: Theme) => void;
  /** Retains the Course Appearance selector used by existing evidence. */
  readonly courseOption?: boolean;
}

/** A shared chooser that previews every Theme in the current Light or Dark form. */
export function ThemeChooser(props: ThemeChooserProps): JSX.Element {
  const disabled = (): boolean => props.disabled?.() ?? false;
  return (
    <div class="theme-chooser">
      <For each={THEME_OPTIONS}>
        {(option) => (
          <label
            class="theme-chooser__card"
            data-theme-option={option.id}
            data-course-theme-option={props.courseOption ? option.id : undefined}
            style={themeStyle(themeTokens(option.id, props.mode()))}
          >
            <input
              type="radio"
              name={props.name}
              value={option.id}
              checked={props.selectedTheme() === option.id}
              disabled={disabled()}
              onInput={() => props.onSelect(option.id)}
            />
            <span class="theme-chooser__label">{option.definition.name}</span>
            <span
              class="theme-chooser__palette"
              aria-label={`${option.definition.name} five-color palette`}
            >
              <span class="theme-chooser__canvas">Canvas</span>
              <span class="theme-chooser__surface">Surface</span>
              <span class="theme-chooser__secondary">Secondary</span>
              <span class="theme-chooser__accent">Accent</span>
              <span class="theme-chooser__highlight">Highlight</span>
            </span>
          </label>
        )}
      </For>
    </div>
  );
}
