// profile_appearance.tsx - Account appearance controls shown on Profile.

import { Match, Show, Switch, createEffect, createMemo, createSignal, type JSX } from "solid-js";

import type { Theme } from "../../generated/api/Theme";
import { useSessionBootstrap } from "../auth/session_context";
import { useAppearance } from "./appearance_context";
import { ThemeChooser } from "./theme_chooser";

export function ProfileAppearance(): JSX.Element {
  const session = useSessionBootstrap();
  const appearance = useAppearance();
  const isInstructor = createMemo(() => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "instructor";
  });
  const savedTheme = createMemo<Theme>(() => appearance.settings()?.personalTheme ?? "grass");
  const [selectedTheme, setSelectedTheme] = createSignal<Theme>("grass");
  const [themeDirty, setThemeDirty] = createSignal(false);
  const [themeMessage, setThemeMessage] = createSignal<string>();
  createEffect(() => {
    const theme = savedTheme();
    if (!themeDirty()) setSelectedTheme(theme);
  });
  const canSaveTheme = createMemo(() => selectedTheme() !== savedTheme());
  const hasDisplayModePreference = createMemo(() => {
    const preference = appearance.settings()?.displayModePreference;
    return preference === "light" || preference === "dark";
  });

  async function savePersonalTheme(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (appearance.busy() || !canSaveTheme()) return;
    setThemeMessage(undefined);
    try {
      await appearance.updatePersonalTheme(selectedTheme());
      setThemeDirty(false);
      setThemeMessage("Personal theme saved.");
    } catch {
      setThemeMessage("Personal theme could not save. Try again.");
    }
  }

  async function followBrowser(): Promise<void> {
    if (appearance.busy()) return;
    try {
      await appearance.updateDisplayModePreference(null);
    } catch {
      // The document owner presents the shared error in the appearance status.
    }
  }

  return (
    <section aria-labelledby="profile-appearance-heading">
      <h2 id="profile-appearance-heading">Appearance</h2>
      <Switch>
        <Match when={appearance.settingsState() === "loading"}>
          <p class="calm-status" role="status">
            Loading appearance settings...
          </p>
        </Match>
        <Match when={appearance.settingsState() === "error"}>
          <p role="alert">Appearance settings are unavailable. Refresh to try again.</p>
        </Match>
        <Match when={appearance.settingsState() === "ready"}>
          <p>Displaying {appearance.appearance().mode === "light" ? "Light" : "Dark"} mode.</p>
          <Show when={hasDisplayModePreference()}>
            <button type="button" disabled={appearance.busy()} onClick={() => void followBrowser()}>
              Follow browser setting
            </button>
          </Show>
          <Show when={isInstructor()}>
            <form
              class="profile-appearance-form"
              aria-busy={appearance.busy()}
              onSubmit={(event) => void savePersonalTheme(event)}
            >
              <fieldset disabled={appearance.busy()}>
                <legend>Personal theme</legend>
                <p class="calm-status">This theme appears on your global Instructor pages.</p>
                <ThemeChooser
                  name="personal-theme"
                  selectedTheme={selectedTheme}
                  mode={() => appearance.appearance().mode}
                  disabled={appearance.busy}
                  onSelect={(theme) => {
                    setSelectedTheme(theme);
                    setThemeDirty(true);
                    setThemeMessage(undefined);
                  }}
                />
              </fieldset>
              <button type="submit" disabled={appearance.busy() || !canSaveTheme()}>
                {appearance.busy() ? "Saving personal theme..." : "Save personal theme"}
              </button>
              <Show when={themeMessage()}>
                <p class="calm-status" role="status" aria-live="polite">
                  {themeMessage()}
                </p>
              </Show>
            </form>
          </Show>
          <Show when={appearance.error()}>
            <p role="alert">{appearance.error()}</p>
          </Show>
          <Show when={appearance.busy()}>
            <p class="calm-status" role="status" aria-live="polite">
              Saving appearance settings...
            </p>
          </Show>
        </Match>
      </Switch>
    </section>
  );
}
