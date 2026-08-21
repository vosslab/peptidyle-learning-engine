// Operator-discovered first-claim page for the seeded live-demo Sysadmin account.

import { A, useNavigate } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { AccountCourse } from "../api/enrollment";
import { DecodeError } from "../api/decoder";
import { ApiRequestError } from "../api/http_client/error";
import { registerLiveDemoSysadminWithBrowser } from "../api/http_client/live_demo";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { courseRouteReference } from "../navigation/public_route";
import { AccountCoursePicker } from "./account_course_picker";
import {
  isLiveDemoUnavailable,
  sysadminOwnershipAvailability,
  type SysadminOwnershipAvailability,
} from "./live_demo_auth_model";
import "./live_demo_auth.css";

type SetupState =
  | { readonly kind: "loading" }
  | { readonly kind: SysadminOwnershipAvailability }
  | { readonly kind: "busy"; readonly message: string }
  | { readonly kind: "courses"; readonly courses: ReadonlyArray<AccountCourse> }
  | { readonly kind: "empty" }
  | { readonly kind: "unavailable" }
  | {
      readonly kind: "error";
      readonly message: string;
      readonly focus: "proof" | "label";
      readonly retry: "availability" | "setup";
    };

function courseRows(state: SetupState): ReadonlyArray<AccountCourse> {
  return state.kind === "courses" ? state.courses : [];
}

function setupBusyMessage(state: SetupState): string {
  return state.kind === "busy" ? state.message : "";
}

function setupErrorMessage(state: SetupState): string {
  return state.kind === "error" ? state.message : "";
}

function setupErrorNeedsAvailabilityRetry(state: SetupState): boolean {
  return state.kind === "error" && state.retry === "availability";
}

function setupFormVisible(state: SetupState): boolean {
  if (state.kind === "error") return state.retry === "setup";
  return state.kind === "ready" || state.kind === "busy";
}

function ownershipAttemptFailure(error: unknown): SetupState {
  if (error instanceof ApiRequestError && error.status === 409) {
    return { kind: "complete" };
  }
  if (
    error instanceof DecodeError ||
    (error instanceof ApiRequestError && [400, 401, 403].includes(error.status))
  ) {
    return {
      kind: "error",
      message:
        "The setup code could not be verified. Check the code supplied for this demo and try again.",
      focus: "proof",
      retry: "setup",
    };
  }
  return {
    kind: "error",
    message:
      "The passkey was not set up. You can try again with this device or another passkey-capable device.",
    focus: "label",
    retry: "setup",
  };
}

export function LiveDemoSysadminSetupPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [state, setState] = createSignal<SetupState>({ kind: "loading" });
  const [ownershipProof, setOwnershipProof] = createSignal("");
  const [passkeyLabel, setPasskeyLabel] = createSignal("This device");
  let proofInput: HTMLInputElement | undefined;
  let labelInput: HTMLInputElement | undefined;
  let courseHeading: HTMLHeadingElement | undefined;
  let availabilityRetry: HTMLButtonElement | undefined;

  function focusAfterError(target: "proof" | "label"): void {
    queueMicrotask(() => (target === "proof" ? proofInput : labelInput)?.focus());
  }

  async function loadAvailability(): Promise<void> {
    setState({ kind: "loading" });
    try {
      const status = await runtime.client.getLiveDemoSysadminOwnershipStatus();
      setState({ kind: sysadminOwnershipAvailability(status.available) });
    } catch (error: unknown) {
      setState(
        isLiveDemoUnavailable(error)
          ? { kind: "unavailable" }
          : {
              kind: "error",
              message: "Administrator setup is unavailable for this deployment.",
              focus: "proof",
              retry: "availability",
            },
      );
      if (!isLiveDemoUnavailable(error)) queueMicrotask(() => availabilityRetry?.focus());
    }
  }

  async function loadCourses(): Promise<void> {
    setState({ kind: "busy", message: "Opening your account..." });
    try {
      const page = await runtime.client.listAccountCourses();
      if (page.courses.length === 0) {
        setState({ kind: "empty" });
        return;
      }
      setState({ kind: "courses", courses: page.courses });
      queueMicrotask(() => courseHeading?.focus());
    } catch {
      setState({
        kind: "error",
        message: "Your administrator passkey is ready, but your course list could not load.",
        focus: "label",
        retry: "setup",
      });
      focusAfterError("label");
    }
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const proof = ownershipProof();
    setState({ kind: "busy", message: "Setting up your administrator passkey..." });
    try {
      await registerLiveDemoSysadminWithBrowser(runtime.client, proof, passkeyLabel());
      await loadCourses();
    } catch (error: unknown) {
      const failure = ownershipAttemptFailure(error);
      setState(failure);
      if (failure.kind === "error") focusAfterError(failure.focus);
    } finally {
      // The operator proof lives only in this signal for the duration of this attempt.
      setOwnershipProof("");
    }
  }

  async function selectCourse(course: AccountCourse): Promise<void> {
    setState({ kind: "busy", message: `Opening ${course.title}...` });
    try {
      await runtime.client.selectAccountCourse(course.courseId);
      await session.retry();
      navigate(`/courses/${courseRouteReference(course.courseReference)}`);
    } catch {
      setState({
        kind: "error",
        message: "That course could not be opened. Return to sign in and try again.",
        focus: "label",
        retry: "setup",
      });
      focusAfterError("label");
    }
  }

  onMount(() => void loadAvailability());

  return (
    <section class="page auth-page sysadmin-setup-page" data-route-surface="liveDemoSysadminSetup">
      <p class="eyebrow">Administrator setup</p>
      <h1>Set up administrator access</h1>
      <Show when={state().kind === "loading"}>
        <p class="calm-status" role="status" aria-live="polite">
          Checking administrator setup availability...
        </p>
      </Show>
      <Show when={state().kind === "unavailable"}>
        <section class="auth-panel" role="status">
          <p>Administrator setup is unavailable for this deployment.</p>
          <A class="quiet-link" href="/sign-in">
            Return to sign in
          </A>
        </section>
      </Show>
      <Show when={state().kind === "complete"}>
        <section class="auth-panel" role="status">
          <p>Administrator setup is already complete. Sign in with the administrator passkey.</p>
          <A class="quiet-link" href="/sign-in">
            Return to sign in
          </A>
        </section>
      </Show>
      <Show when={setupFormVisible(state())}>
        <form class="auth-panel sysadmin-setup-form" onSubmit={(event) => void submit(event)}>
          <p>
            Enter the setup code supplied for this deployment, then create the administrator passkey
            on this device.
          </p>
          <label for="live-demo-ownership-proof">Administrator setup code</label>
          <input
            id="live-demo-ownership-proof"
            ref={(element) => {
              proofInput = element;
            }}
            type="password"
            autocomplete="off"
            required
            value={ownershipProof()}
            disabled={state().kind === "busy"}
            onInput={(event) => setOwnershipProof(event.currentTarget.value)}
          />
          <label for="live-demo-passkey-label">Passkey name</label>
          <input
            id="live-demo-passkey-label"
            ref={(element) => {
              labelInput = element;
            }}
            maxlength={80}
            required
            value={passkeyLabel()}
            disabled={state().kind === "busy"}
            onInput={(event) => setPasskeyLabel(event.currentTarget.value)}
          />
          <button class="primary-action" type="submit" disabled={state().kind === "busy"}>
            Set up administrator passkey
          </button>
        </form>
      </Show>
      <Show when={state().kind === "busy"}>
        <p class="calm-status" role="status" aria-live="polite">
          {setupBusyMessage(state())}
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="inline-error" role="alert">
          <p>{setupErrorMessage(state())}</p>
          <Show when={setupErrorNeedsAvailabilityRetry(state())}>
            <button
              class="quiet-action"
              type="button"
              ref={(element) => {
                availabilityRetry = element;
              }}
              onClick={() => void loadAvailability()}
            >
              Retry
            </button>
          </Show>
        </section>
      </Show>
      <Show when={state().kind === "courses"}>
        <AccountCoursePicker
          courses={courseRows(state())}
          select={selectCourse}
          busy={false}
          headingRef={(element) => {
            courseHeading = element;
          }}
        />
      </Show>
      <Show when={state().kind === "empty"}>
        <section class="auth-panel" role="status">
          <h2>Your administrator account is ready</h2>
          <p>It does not belong to a course yet.</p>
        </section>
      </Show>
    </section>
  );
}
