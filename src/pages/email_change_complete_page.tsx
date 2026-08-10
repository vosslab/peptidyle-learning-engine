// Browser-bound completion for a signed-in account email change.

import { A } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { consumeTokenFragment } from "../auth/secret_fragment";

type ChangeState = "working" | "changed" | "error";

export function EmailChangeCompletePage(): JSX.Element {
  const runtime = useApiRuntime();
  const [token] = createSignal(consumeTokenFragment(window.location, window.history));
  const [state, setState] = createSignal<ChangeState>(token() === null ? "error" : "working");

  onMount(() => {
    const oneTimeToken = token();
    if (oneTimeToken === null) return;
    void runtime.client
      .completeAccountEmailChange(oneTimeToken)
      .then(() => setState("changed"))
      .catch(() => setState("error"));
  });

  return (
    <section class="page auth-page" data-route-surface="emailChangeComplete">
      <p class="eyebrow">Account security</p>
      <h1>Confirm your new email</h1>
      <Show when={state() === "working"}>
        <p class="calm-status" role="status" aria-live="polite">
          Verifying this one-time link...
        </p>
      </Show>
      <Show when={state() === "changed"}>
        <section class="auth-panel" role="status">
          <h2>Email changed</h2>
          <p>Your PLE account now uses the verified address for email sign-in.</p>
          <A href="/account/security">Return to account security</A>
        </section>
      </Show>
      <Show when={state() === "error"}>
        <section class="inline-error" role="alert">
          <h2>Email not changed</h2>
          <p>This link is expired, used, or no longer belongs to this signed-in browser.</p>
          <A href="/account/security">Request another verification link</A>
        </section>
      </Show>
    </section>
  );
}
