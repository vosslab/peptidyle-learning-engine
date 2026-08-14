// Signed-in account passkey management.

import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { PasskeySummary } from "../api/enrollment";
import { registerPasskeyWithBrowser } from "../api/http_client/enrollment";
import { useApiRuntime } from "../api/runtime";
import { usePresentationContrast } from "../presentation/contrast_context";

type PasskeyState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly passkeys: ReadonlyArray<PasskeySummary> }
  | { readonly kind: "error"; readonly message: string };

function passkeyError(state: PasskeyState): string {
  return state.kind === "error" ? state.message : "";
}

function readyPasskeys(state: PasskeyState): ReadonlyArray<PasskeySummary> {
  return state.kind === "ready" ? state.passkeys : [];
}

function passkeyActivity(passkey: PasskeySummary): string {
  const timestamp = passkey.lastUsedAtMillis ?? passkey.createdAtMillis;
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(new Date(timestamp));
}

export function AccountSecurityPage(): JSX.Element {
  const runtime = useApiRuntime();
  const presentation = usePresentationContrast();
  const [state, setState] = createSignal<PasskeyState>({ kind: "loading" });
  const [label, setLabel] = createSignal("");
  const [newEmail, setNewEmail] = createSignal("");
  const [announcement, setAnnouncement] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  async function load(): Promise<void> {
    setState({ kind: "loading" });
    try {
      setState({ kind: "ready", passkeys: await runtime.client.listPasskeys() });
    } catch {
      setState({ kind: "error", message: "Your passkeys could not load." });
    }
  }

  async function add(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setBusy(true);
    try {
      await registerPasskeyWithBrowser(runtime.client, label());
      setLabel("");
      setAnnouncement("Passkey added.");
      await load();
    } catch {
      setAnnouncement("The passkey was not added. Your existing sign-in methods are unchanged.");
    } finally {
      setBusy(false);
    }
  }

  async function remove(passkey: PasskeySummary): Promise<void> {
    setBusy(true);
    try {
      await runtime.client.revokePasskey(passkey.id);
      setAnnouncement(`${passkey.label} was removed.`);
      await load();
    } catch {
      setAnnouncement(`${passkey.label} could not be removed.`);
    } finally {
      setBusy(false);
    }
  }

  async function changeEmail(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setBusy(true);
    try {
      await runtime.client.startAccountEmailChange(newEmail());
      setNewEmail("");
      setAnnouncement(
        "Check the new address and open its one-time link in this browser within ten minutes.",
      );
    } catch {
      setAnnouncement("The email change could not start. Your current sign-in remains unchanged.");
    } finally {
      setBusy(false);
    }
  }

  onMount(() => void load());

  return (
    <section class="page auth-page" data-route-surface="accountSecurity">
      <p class="eyebrow">Account security</p>
      <h1>Your passkeys</h1>
      <p class="page-lede">
        Passkeys are optional sign-in shortcuts. You can always use your verified email to sign in.
      </p>
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>

      <section class="auth-panel presentation-preference" aria-labelledby="presentation-heading">
        <div>
          <h2 id="presentation-heading">Visual contrast</h2>
          <p>
            Standard keeps each course palette expressive. Increased contrast strengthens text,
            controls, and separators without changing the course theme.
          </p>
        </div>
        <label for="account-contrast">Contrast level</label>
        <select
          id="account-contrast"
          value={presentation.contrast()}
          disabled={presentation.saving()}
          onChange={(event) =>
            void presentation.setContrast(
              event.currentTarget.value === "increased" ? "increased" : "standard",
            )
          }
        >
          <option value="standard">Standard theme</option>
          <option value="increased">Increased contrast</option>
        </select>
        <Show when={presentation.error()}>
          {(message) => (
            <p class="inline-error" role="alert">
              {message()}
            </p>
          )}
        </Show>
      </section>

      <Show when={state().kind === "loading"}>
        <p class="loading-state" role="status">
          Loading passkeys...
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="inline-error" role="alert">
          <p>{passkeyError(state())}</p>
          <button class="quiet-action" type="button" onClick={() => void load()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={state().kind === "ready"}>
        <div class="passkey-list">
          <For each={readyPasskeys(state())}>
            {(passkey) => (
              <article class="auth-panel passkey-card">
                <div>
                  <h2>{passkey.label}</h2>
                  <p>Last activity: {passkeyActivity(passkey)}</p>
                </div>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={busy()}
                  onClick={() => void remove(passkey)}
                >
                  Remove passkey
                </button>
              </article>
            )}
          </For>
        </div>
      </Show>

      <form class="auth-panel auth-form" onSubmit={(event) => void add(event)}>
        <h2>Add another passkey</h2>
        <label for="new-passkey-label">Passkey name</label>
        <input
          id="new-passkey-label"
          maxlength={80}
          required
          value={label()}
          onInput={(event) => setLabel(event.currentTarget.value)}
        />
        <p class="field-help">For example: Biology laptop, phone, or USB security key.</p>
        <button class="primary-action" type="submit" disabled={busy()}>
          Add passkey
        </button>
      </form>

      <form class="auth-panel auth-form" onSubmit={(event) => void changeEmail(event)}>
        <h2>Change your sign-in email</h2>
        <label for="new-account-email">New email address</label>
        <input
          id="new-account-email"
          type="email"
          autocomplete="email"
          maxlength={320}
          required
          value={newEmail()}
          onInput={(event) => setNewEmail(event.currentTarget.value)}
        />
        <p class="field-help">
          Your account stays unchanged until you verify the new address in this browser.
        </p>
        <button class="quiet-action" type="submit" disabled={busy()}>
          Verify new email
        </button>
      </form>
    </section>
  );
}
