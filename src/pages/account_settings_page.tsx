// account_settings_page.tsx - one role-neutral, authenticated-self preference surface.

import { A } from "@solidjs/router";
import { Match, Switch, createResource, createSignal, For, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";

function availableTimeZones(current: string): readonly string[] {
  const intl = Intl as typeof Intl & {
    readonly supportedValuesOf?: (key: "timeZone") => readonly string[];
  };
  const supported = intl.supportedValuesOf?.("timeZone") ?? [];
  return [...new Set([...supported, "UTC", current])].sort((left, right) =>
    left.localeCompare(right),
  );
}

/** The complete current Account Settings scope: one self-only display preference. */
export function AccountSettingsPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [settings, { mutate }] = createResource(() => applicationApi.client.getAccountSettings());
  const [draft, setDraft] = createSignal<string>();
  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const selectedTimeZone = (): string => draft() ?? settings()?.timeZone ?? "UTC";

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving()) return;
    setSaving(true);
    setMessage("");
    try {
      const saved = await applicationApi.client.updateAccountSettings({
        timeZone: selectedTimeZone(),
      });
      mutate(saved);
      setDraft(saved.timeZone);
      setMessage("Your time zone was saved.");
    } catch (_error: unknown) {
      setMessage("Your time zone could not be saved. Try again.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <main class="page" data-route-surface="accountSettings">
      <p class="eyebrow">Account settings</p>
      <h1>Account settings</h1>
      <p class="page-lede">Choose how dates and times are shown. This does not change deadlines.</p>
      <section aria-labelledby="account-settings-time-zone-heading">
        <h2 id="account-settings-time-zone-heading">Time zone</h2>
        <Switch>
          <Match when={settings.loading}>
            <p class="calm-status" role="status">
              Loading your time zone...
            </p>
          </Match>
          <Match when={settings.error !== undefined}>
            <p role="alert">Your time zone is unavailable. Refresh to try again.</p>
          </Match>
          <Match when={settings()}>
            <form aria-busy={saving()} onSubmit={(event) => void save(event)}>
              <label for="account-settings-time-zone">
                Time zone
                <select
                  id="account-settings-time-zone"
                  value={selectedTimeZone()}
                  disabled={saving()}
                  onInput={(event) => setDraft(event.currentTarget.value)}
                >
                  <For each={availableTimeZones(selectedTimeZone())}>
                    {(timeZone) => <option value={timeZone}>{timeZone}</option>}
                  </For>
                </select>
              </label>
              <button class="primary-action" type="submit" disabled={saving()}>
                {saving() ? "Saving..." : "Save time zone"}
              </button>
            </form>
          </Match>
        </Switch>
        <Match when={message()}>
          {(value) => (
            <p role="status" aria-live="polite">
              {value()}
            </p>
          )}
        </Match>
      </section>
      <A class="quiet-link" href="/">
        Return to your dashboard
      </A>
    </main>
  );
}
