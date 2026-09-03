// Deployment-gated seeded-demo entry for the current mounted session surface.

import { useNavigate } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { SeededDemoAccount } from "../api/live_demo";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { isLiveDemoUnavailable, seededDemoDescription } from "./live_demo_auth_model";
import "./live_demo_auth.css";

type SeededDemoState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly accounts: ReadonlyArray<SeededDemoAccount> }
  | { readonly kind: "opening"; readonly displayName: string }
  | { readonly kind: "unavailable" }
  | { readonly kind: "error" };

function seededAccounts(state: SeededDemoState): ReadonlyArray<SeededDemoAccount> {
  return state.kind === "ready" ? state.accounts : [];
}

function seededDemoOpeningName(state: SeededDemoState): string {
  return state.kind === "opening" ? state.displayName : "";
}

/**
 * Provides the only currently mounted sign-in entry. Email-code and passkey
 * authentication will return here once their Account session adapters exist.
 */
export function SignInPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [seededDemo, setSeededDemo] = createSignal<SeededDemoState>({ kind: "loading" });
  let retry: HTMLButtonElement | undefined;

  async function loadSeededDemoAccounts(): Promise<void> {
    setSeededDemo({ kind: "loading" });
    try {
      const response = await runtime.client.listSeededDemoAccounts();
      setSeededDemo({ kind: "ready", accounts: response.accounts });
    } catch (error: unknown) {
      if (isLiveDemoUnavailable(error)) {
        setSeededDemo({ kind: "unavailable" });
        return;
      }
      setSeededDemo({ kind: "error" });
      queueMicrotask(() => retry?.focus());
    }
  }

  async function selectSeededDemoAccount(account: SeededDemoAccount): Promise<void> {
    const accounts = seededAccounts(seededDemo());
    setSeededDemo({ kind: "opening", displayName: account.displayName });
    try {
      await runtime.client.selectSeededDemoAccount(account.persona);
      await session.retry();
      navigate("/");
    } catch {
      setSeededDemo({ kind: "ready", accounts });
    }
  }

  onMount(() => void loadSeededDemoAccounts());

  return (
    <section class="page auth-page" data-route-surface="signIn">
      <p class="eyebrow">Live demo</p>
      <h1>Explore Peptidyle Learning Engine</h1>
      <p class="page-lede">
        Select a seeded Account to explore the current disposable demonstration.
      </p>

      <Show
        when={seededDemo().kind !== "unavailable"}
        fallback={
          <section class="auth-panel" role="status">
            <h2>Demo entry is unavailable</h2>
            <p>This installation has no complete seeded-demo configuration.</p>
          </section>
        }
      >
        <section class="auth-panel live-demo-panel" aria-labelledby="live-demo-heading">
          <h2 id="live-demo-heading">Choose a demo Account</h2>
          <p>Each choice creates the ordinary host-only Authenticated Session.</p>
          <Show when={seededDemo().kind === "loading"}>
            <p class="calm-status live-demo-status" role="status" aria-live="polite">
              Loading available demo Accounts...
            </p>
          </Show>
          <Show when={seededDemo().kind === "ready" || seededDemo().kind === "opening"}>
            <div class="live-demo-persona-list">
              <For each={seededAccounts(seededDemo())}>
                {(account) => (
                  <button
                    class="quiet-action live-demo-persona-action"
                    type="button"
                    disabled={seededDemo().kind === "opening"}
                    onClick={() => void selectSeededDemoAccount(account)}
                  >
                    <span>Continue as {account.displayName}</span>
                    <small>{seededDemoDescription(account.persona)}</small>
                  </button>
                )}
              </For>
            </div>
          </Show>
          <Show when={seededDemo().kind === "opening"}>
            <p class="calm-status live-demo-status" role="status" aria-live="polite">
              Opening {seededDemoOpeningName(seededDemo())}'s Account...
            </p>
          </Show>
          <Show when={seededDemo().kind === "error"}>
            <section class="inline-error" role="alert">
              <p>That demo Account could not be opened. Try again in a moment.</p>
              <button
                class="quiet-action"
                type="button"
                ref={(element) => {
                  retry = element;
                }}
                onClick={() => void loadSeededDemoAccounts()}
              >
                Retry
              </button>
            </section>
          </Show>
        </section>
      </Show>
    </section>
  );
}
