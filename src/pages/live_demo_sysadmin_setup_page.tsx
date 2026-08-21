// Operator-discovered first-claim page for the seeded live-demo Sysadmin account.

import { A, useNavigate } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { AccountCourse } from "../api/enrollment";
import { registerLiveDemoSysadminWithBrowser } from "../api/http_client/live_demo";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { courseRouteReference } from "../navigation/public_route";
import { AccountCoursePicker } from "./account_course_picker";
import {
  clearSysadminOwnershipProof,
  isLiveDemoUnavailable,
  sysadminCourseFailure,
  sysadminOwnershipFailure,
  sysadminOwnershipAvailability,
  sysadminSetupBusyMessage,
  sysadminSetupCourseRows,
  sysadminSetupErrorMessage,
  sysadminSetupFormVisible,
  sysadminSetupRetry,
  type SysadminSetupState,
} from "./live_demo_auth_model";
import "./live_demo_auth.css";

export function LiveDemoSysadminSetupPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [state, setState] = createSignal<SysadminSetupState>({ kind: "loading" });
  const [ownershipProof, setOwnershipProof] = createSignal("");
  const [passkeyLabel, setPasskeyLabel] = createSignal("This device");
  let proofInput: HTMLInputElement | undefined;
  let labelInput: HTMLInputElement | undefined;
  let courseHeading: HTMLHeadingElement | undefined;
  let availabilityRetry: HTMLButtonElement | undefined;
  let courseRetry: HTMLButtonElement | undefined;

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
              kind: "availability-error",
              message: "Administrator setup is unavailable for this deployment.",
            },
      );
      if (!isLiveDemoUnavailable(error)) queueMicrotask(() => availabilityRetry?.focus());
    }
  }

  async function loadCourses(): Promise<void> {
    setState({ kind: "courses-busy", message: "Opening your account..." });
    try {
      const page = await runtime.client.listAccountCourses();
      if (page.courses.length === 0) {
        setState({ kind: "empty" });
        return;
      }
      setState({ kind: "courses", courses: page.courses });
      queueMicrotask(() => courseHeading?.focus());
    } catch {
      setState(sysadminCourseFailure("list"));
      queueMicrotask(() => courseRetry?.focus());
    }
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const proof = ownershipProof();
    setState({ kind: "ownership-busy" });
    try {
      await registerLiveDemoSysadminWithBrowser(runtime.client, proof, passkeyLabel());
      await loadCourses();
    } catch (error: unknown) {
      const failure = sysadminOwnershipFailure(error);
      setState(failure);
      if (failure.kind === "ownership-error") focusAfterError(failure.focus);
    } finally {
      setOwnershipProof(clearSysadminOwnershipProof());
    }
  }

  async function selectCourse(course: AccountCourse): Promise<void> {
    setState({ kind: "courses-busy", message: `Opening ${course.title}...` });
    try {
      await runtime.client.selectAccountCourse(course.courseId);
      await session.retry();
      navigate(`/courses/${courseRouteReference(course.courseReference)}`);
    } catch {
      setState(sysadminCourseFailure("select"));
      queueMicrotask(() => courseRetry?.focus());
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
      <Show when={sysadminSetupFormVisible(state())}>
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
            disabled={state().kind === "ownership-busy"}
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
            disabled={state().kind === "ownership-busy"}
            onInput={(event) => setPasskeyLabel(event.currentTarget.value)}
          />
          <button class="primary-action" type="submit" disabled={state().kind === "ownership-busy"}>
            Set up administrator passkey
          </button>
        </form>
      </Show>
      <Show when={state().kind === "ownership-busy" || state().kind === "courses-busy"}>
        <p class="calm-status" role="status" aria-live="polite">
          {sysadminSetupBusyMessage(state())}
        </p>
      </Show>
      <Show when={sysadminSetupErrorMessage(state())}>
        <section class="inline-error" role="alert">
          <p>{sysadminSetupErrorMessage(state())}</p>
          <Show when={sysadminSetupRetry(state()) === "availability"}>
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
          <Show when={sysadminSetupRetry(state()) === "courses"}>
            <button
              class="quiet-action"
              type="button"
              ref={(element) => {
                courseRetry = element;
              }}
              onClick={() => void loadCourses()}
            >
              Retry course list
            </button>
            <A class="quiet-link" href="/sign-in">
              Return to sign in
            </A>
          </Show>
        </section>
      </Show>
      <Show when={state().kind === "courses"}>
        <AccountCoursePicker
          courses={sysadminSetupCourseRows(state())}
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
