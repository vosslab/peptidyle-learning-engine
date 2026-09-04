// Deployment-gated seeded-demo entry for the current available session surface.

import { useNavigate } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { SeededDemoAccount, SeededDemoAccounts } from "../api/live_demo";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import {
  isLiveDemoUnavailable,
  seededDemoAvailabilityStatus,
  seededDemoDescription,
} from "./live_demo_auth_model";
import "./live_demo_auth.css";

type SeededDemoState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly response: SeededDemoAccounts }
  | {
      readonly kind: "opening";
      readonly response: SeededDemoAccounts;
      readonly displayName: string;
    }
  | { readonly kind: "unavailable" }
  | { readonly kind: "error" };

function seededAccounts(state: SeededDemoState): ReadonlyArray<SeededDemoAccount> {
  return state.kind === "ready" || state.kind === "opening" ? state.response.accounts : [];
}

function unavailableAccountCount(state: SeededDemoState): number {
  return state.kind === "ready" || state.kind === "opening"
    ? state.response.unavailableAccountCount
    : 0;
}

function seededDemoOpeningName(state: SeededDemoState): string {
  return state.kind === "opening" ? state.displayName : "";
}

/** Renders deployment-gated seeded-demo entry for the ordinary session boundary. */
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
      setSeededDemo({ kind: "ready", response });
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
    const current = seededDemo();
    if (current.kind !== "ready") return;
    const response = current.response;
    setSeededDemo({ kind: "opening", response, displayName: account.displayName });
    try {
      await runtime.client.selectSeededDemoAccount(account.persona);
      await session.retry();
      navigate("/");
    } catch {
      setSeededDemo({ kind: "ready", response });
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
            <p>No available seeded-demo mapping remains for this installation.</p>
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
            <Show when={unavailableAccountCount(seededDemo()) > 0}>
              <p class="calm-status live-demo-status" role="status" aria-live="polite">
                {seededDemoAvailabilityStatus(unavailableAccountCount(seededDemo()))}
              </p>
            </Show>
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
