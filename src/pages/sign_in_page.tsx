// Deployment-gated seeded-demo entry for the current available session surface.

import { useNavigate } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { SeededDemoAccount, SeededDemoAccounts } from "../api/live_demo";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame";
import {
  isLiveDemoUnavailable,
  seededDemoAvailabilityStatus,
  seededDemoDescription,
  seededDemoRole,
  seededDemoRoleLabel,
} from "./live_demo_auth_model";
import {
  EMAIL_CODE_LENGTH,
  emailCodeCompletionFailure,
  emailCodeStartFailure,
} from "./email_code_sign_in_model";
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

type EmailCodeState =
  | { readonly kind: "ready" }
  | { readonly kind: "requesting" }
  | { readonly kind: "codeEntry"; readonly challengeId: string }
  | { readonly kind: "completing"; readonly challengeId: string }
  | { readonly kind: "invalidEmail" }
  | { readonly kind: "invalidCode"; readonly challengeId: string }
  | { readonly kind: "completionError"; readonly challengeId: string }
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

function emailCodeChallengeId(state: EmailCodeState): string | undefined {
  return state.kind === "codeEntry" ||
    state.kind === "completing" ||
    state.kind === "invalidCode" ||
    state.kind === "completionError"
    ? state.challengeId
    : undefined;
}

function emailCodeSubmitting(state: EmailCodeState): boolean {
  return state.kind === "requesting" || state.kind === "completing";
}

/** Renders deployment-gated seeded-demo entry for the ordinary session boundary. */
export function SignInPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [seededDemo, setSeededDemo] = createSignal<SeededDemoState>({ kind: "loading" });
  const [emailCode, setEmailCode] = createSignal<EmailCodeState>({ kind: "ready" });
  let retry: HTMLButtonElement | undefined;
  let emailCodeInput: HTMLInputElement | undefined;

  async function navigateAfterAuthentication(): Promise<void> {
    await session.retry();
    const currentSession = session.state();
    navigate(
      currentSession.kind === "authenticated" &&
        currentSession.session.account.userRole === "instructor"
        ? "/library"
        : "/",
    );
  }

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
      await navigateAfterAuthentication();
    } catch {
      setSeededDemo({ kind: "ready", response });
    }
  }

  async function startEmailCodeSignIn(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (emailCodeSubmitting(emailCode())) return;
    const form = event.currentTarget;
    if (!(form instanceof HTMLFormElement)) return;
    const email = new FormData(form).get("email");
    if (typeof email !== "string") return;
    setEmailCode({ kind: "requesting" });
    try {
      const started = await runtime.client.startEmailCodeSignIn(email.trim());
      setEmailCode({ kind: "codeEntry", challengeId: started.challengeId });
      queueMicrotask(() => emailCodeInput?.focus());
    } catch (error: unknown) {
      const failure = emailCodeStartFailure(error);
      if (failure === "unavailable") {
        setEmailCode({ kind: "unavailable" });
        return;
      }
      setEmailCode(failure === "invalidEmail" ? { kind: "invalidEmail" } : { kind: "error" });
    }
  }

  async function completeEmailCodeSignIn(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const challengeId = emailCodeChallengeId(emailCode());
    if (challengeId === undefined || emailCodeSubmitting(emailCode())) return;
    const form = event.currentTarget;
    if (!(form instanceof HTMLFormElement)) return;
    const code = new FormData(form).get("code");
    if (typeof code !== "string") return;
    setEmailCode({ kind: "completing", challengeId });
    try {
      await runtime.client.completeEmailCodeSignIn(challengeId, code.trim());
      await navigateAfterAuthentication();
    } catch (error: unknown) {
      const failure = emailCodeCompletionFailure(error);
      if (failure === "unavailable") {
        setEmailCode({ kind: "unavailable" });
        return;
      }
      setEmailCode(
        failure === "invalidCode"
          ? { kind: "invalidCode", challengeId }
          : { kind: "completionError", challengeId },
      );
      queueMicrotask(() => emailCodeInput?.focus());
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
    <PageFrame
      routeSurface="signIn"
      eyebrow="Sign in"
      title="Sign in to Peptidyle Learning Engine"
      lede="Students and Instructors can use a code sent to their email address."
    >
      <section class="auth-panel email-code-panel" aria-labelledby="email-code-heading">
        <h2 id="email-code-heading">Sign in with email</h2>
        <Show
          when={emailCodeChallengeId(emailCode()) === undefined}
          fallback={
            <form class="auth-form" onSubmit={(event) => void completeEmailCodeSignIn(event)}>
              <p class="field-help">Enter the code from your email to finish signing in.</p>
              <label>
                Email code
                <input
                  ref={(element) => {
                    emailCodeInput = element;
                  }}
                  name="code"
                  inputmode="text"
                  autocomplete="one-time-code"
                  maxlength={EMAIL_CODE_LENGTH}
                  pattern={`[A-Za-z0-9_-]{${EMAIL_CODE_LENGTH}}`}
                  required
                  disabled={emailCodeSubmitting(emailCode())}
                />
              </label>
              <button
                class="quiet-action"
                type="submit"
                disabled={emailCodeSubmitting(emailCode())}
              >
                Verify code and sign in
              </button>
              <Show when={emailCode().kind === "invalidCode"}>
                <p class="inline-error" role="alert">
                  That code could not be verified. Request a new code and try again.
                </p>
              </Show>
              <Show when={emailCode().kind === "completionError"}>
                <p class="inline-error" role="alert">
                  Sign-in could not be completed. Try again in a moment.
                </p>
              </Show>
              <button
                class="quiet-action"
                type="button"
                disabled={emailCodeSubmitting(emailCode())}
                onClick={() => setEmailCode({ kind: "ready" })}
              >
                Use another email address
              </button>
            </form>
          }
        >
          <form class="auth-form" onSubmit={(event) => void startEmailCodeSignIn(event)}>
            <p class="field-help">We will send a code if this email can sign in.</p>
            <label>
              Email address
              <input
                name="email"
                type="email"
                autocomplete="email"
                maxlength="320"
                required
                disabled={emailCodeSubmitting(emailCode())}
              />
            </label>
            <button class="quiet-action" type="submit" disabled={emailCodeSubmitting(emailCode())}>
              Send email code
            </button>
            <Show when={emailCode().kind === "invalidEmail"}>
              <p class="inline-error" role="alert">
                Enter a valid email address.
              </p>
            </Show>
            <Show when={emailCode().kind === "unavailable"}>
              <p class="inline-error" role="alert">
                Email sign-in is unavailable for this installation.
              </p>
            </Show>
            <Show when={emailCode().kind === "error"}>
              <p class="inline-error" role="alert">
                Email sign-in could not be started. Try again in a moment.
              </p>
            </Show>
          </form>
        </Show>
      </section>
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
                    data-user-role={seededDemoRole(account.persona)}
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
    </PageFrame>
  );
}
