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
  seededDemoRole,
  seededDemoRoleLabel,
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
  | {
      readonly kind: "pendingSysadminTotp";
      readonly response: SeededDemoAccounts;
      readonly displayName: string;
      readonly attestationId: string;
      readonly submitting: boolean;
      readonly failed: boolean;
    }
  | { readonly kind: "unavailable" }
  | { readonly kind: "error" };

function seededAccounts(state: SeededDemoState): ReadonlyArray<SeededDemoAccount> {
  return state.kind === "ready" || state.kind === "opening" ? state.response.accounts : [];
}

function unavailableAccountCount(state: SeededDemoState): number {
  return state.kind === "ready" || state.kind === "opening" || state.kind === "pendingSysadminTotp"
    ? state.response.unavailableAccountCount
    : 0;
}

function seededDemoOpeningName(state: SeededDemoState): string {
  return state.kind === "opening" ? state.displayName : "";
}

function pendingSysadminTotpSubmitting(state: SeededDemoState): boolean {
  return state.kind === "pendingSysadminTotp" && state.submitting;
}

function pendingSysadminTotpFailed(state: SeededDemoState): boolean {
  return state.kind === "pendingSysadminTotp" && state.failed;
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
      const selection = await runtime.client.selectSeededDemoAccount(account.persona);
      if ("pendingMfa" in selection) {
        setSeededDemo({
          kind: "pendingSysadminTotp",
          response,
          displayName: account.displayName,
          attestationId: selection.attestationId,
          submitting: false,
          failed: false,
        });
        return;
      }
      await session.retry();
      const currentSession = session.state();
      navigate(
        currentSession.kind === "authenticated" &&
          currentSession.session.account.productRole === "instructor"
          ? "/library"
          : currentSession.kind === "authenticated" &&
              currentSession.session.account.productRole === "student"
            ? "/"
            : "/",
      );
    } catch {
      setSeededDemo({ kind: "ready", response });
    }
  }

  async function completeSeededDemoSysadminTotp(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const current = seededDemo();
    if (current.kind !== "pendingSysadminTotp" || current.submitting) return;
    const form = event.currentTarget;
    if (!(form instanceof HTMLFormElement)) return;
    const code = new FormData(form).get("code");
    if (typeof code !== "string") return;
    setSeededDemo({ ...current, submitting: true, failed: false });
    try {
      await runtime.client.completeSeededDemoSysadminTotp(current.attestationId, code);
      await session.retry();
      if (session.state().kind === "authenticated") {
        navigate("/");
        return;
      }
    } catch {
      // The server owns code validation, replay handling, and attempt limits.
    }
    const latest = seededDemo();
    if (latest.kind === "pendingSysadminTotp") {
      setSeededDemo({ ...latest, submitting: false, failed: true });
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
          <p>Choose a role to explore its tools and course views.</p>
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
                    data-product-role={seededDemoRole(account.persona)}
                    type="button"
                    disabled={seededDemo().kind === "opening"}
                    onClick={() => void selectSeededDemoAccount(account)}
                  >
                    <span>
                      Assume the role of {seededDemoRoleLabel(account.persona)}{" "}
                      {account.displayName}
                    </span>
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
          <Show when={seededDemo().kind === "pendingSysadminTotp"}>
            <section class="live-demo-totp" aria-labelledby="live-demo-totp-heading">
              <h3 id="live-demo-totp-heading">Verify Morgan Delgado&apos;s administrator access</h3>
              <p>
                Enter the current code from the local demonstration authenticator. This step does
                not create a session until the code is accepted.
              </p>
              <form onSubmit={(event) => void completeSeededDemoSysadminTotp(event)}>
                <label>
                  Authentication code
                  <input
                    name="code"
                    inputmode="numeric"
                    autocomplete="one-time-code"
                    maxlength="6"
                    pattern="[0-9]{6}"
                    required
                    disabled={pendingSysadminTotpSubmitting(seededDemo())}
                  />
                </label>
                <button
                  class="quiet-action"
                  type="submit"
                  disabled={pendingSysadminTotpSubmitting(seededDemo())}
                >
                  Verify and open administrator tools
                </button>
              </form>
              <Show when={pendingSysadminTotpFailed(seededDemo())}>
                <p class="inline-error" role="alert">
                  That code could not be verified. Try the current code from the authenticator.
                </p>
              </Show>
            </section>
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
