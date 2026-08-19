// Learner-owned invitation claim without persistent browser secret storage.

import { A, useNavigate } from "@solidjs/router";
import { Show, createSignal, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { consumeTokenFragment } from "../auth/secret_fragment";
import { courseRouteReference } from "../navigation/public_route";

type InvitationState =
  | { readonly kind: "ready" }
  | { readonly kind: "busy"; readonly message: string }
  | { readonly kind: "emailSent" }
  | { readonly kind: "error"; readonly message: string };

function invitationMessage(state: InvitationState, kind: "busy" | "error"): string {
  return state.kind === kind ? state.message : "";
}

export function CourseInvitationPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [token] = createSignal(consumeTokenFragment(window.location, window.history));
  const [email, setEmail] = createSignal("");
  const [state, setState] = createSignal<InvitationState>(
    token() === null
      ? { kind: "error", message: "This course invitation is missing or malformed." }
      : { kind: "ready" },
  );

  async function sendSignInLink(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setState({ kind: "busy", message: "Sending your sign-in link..." });
    try {
      await runtime.client.startEmailAuthentication(email());
      setState({ kind: "emailSent" });
    } catch {
      setState({ kind: "error", message: "Email sign-in is temporarily unavailable." });
    }
  }

  async function claimInvitation(): Promise<void> {
    const invitationToken = token();
    if (invitationToken === null) return;
    setState({ kind: "busy", message: "Claiming your course..." });
    try {
      const claimed = await runtime.client.redeemCourseInvitation(invitationToken);
      await session.retry();
      navigate(`/courses/${courseRouteReference(claimed.courseReference)}`);
    } catch {
      setState({
        kind: "error",
        message:
          "The invitation could not be claimed. Sign in with the invited email, then try this button again.",
      });
    }
  }

  return (
    <section class="page auth-page" data-route-surface="courseInvitation">
      <p class="eyebrow">Course invitation</p>
      <h1>Join your PLE course</h1>
      <p class="page-lede">
        The invitation is tied to one email and one course. It does not expose course records before
        you authenticate.
      </p>

      <Show when={token() !== null}>
        <section class="auth-panel">
          <h2>Already signed in?</h2>
          <p>Claim the course now. PLE verifies the invitation email against your account.</p>
          <button
            class="primary-action"
            type="button"
            disabled={state().kind === "busy"}
            onClick={() => void claimInvitation()}
          >
            Claim this course
          </button>
        </section>

        <form class="auth-panel auth-form" onSubmit={(event) => void sendSignInLink(event)}>
          <h2>Need to sign in first?</h2>
          <label for="invitation-email">Invited email address</label>
          <input
            id="invitation-email"
            type="email"
            autocomplete="email"
            maxlength={320}
            required
            value={email()}
            onInput={(event) => setEmail(event.currentTarget.value)}
          />
          <button class="quiet-action" type="submit" disabled={state().kind === "busy"}>
            Email me a sign-in link
          </button>
          <Show when={state().kind === "emailSent"}>
            <p role="status">
              Open the one-time sign-in link in this browser. Then return to this tab and choose
              Claim this course.
            </p>
          </Show>
        </form>
      </Show>

      <Show when={state().kind === "busy"}>
        <p class="calm-status" role="status" aria-live="polite">
          {invitationMessage(state(), "busy")}
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="inline-error" role="alert">
          <p>{invitationMessage(state(), "error")}</p>
          <A href="/sign-in" target="_blank" rel="noopener">
            Open sign-in in another tab
          </A>
        </section>
      </Show>
    </section>
  );
}
