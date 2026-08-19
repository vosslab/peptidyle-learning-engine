// PLE-owned passwordless sign-in, email completion, and account course selection.

import { A, useNavigate } from "@solidjs/router";
import { For, Show, createSignal, type JSX } from "solid-js";

import type { AccountCourse } from "../api/enrollment";
import {
  authenticatePasskeyWithBrowser,
  registerPasskeyWithBrowser,
} from "../api/http_client/enrollment";
import { useApiRuntime } from "../api/runtime";
import { courseRouteReference } from "../navigation/public_route";
import { useSessionBootstrap } from "../auth/session_context";
import { consumeTokenFragment } from "../auth/secret_fragment";

type AccountState =
  | { readonly kind: "idle" }
  | { readonly kind: "busy"; readonly message: string }
  | { readonly kind: "courses"; readonly courses: ReadonlyArray<AccountCourse> }
  | { readonly kind: "empty" }
  | { readonly kind: "error"; readonly message: string };

function accountError(message: string): AccountState {
  return { kind: "error", message };
}

function accountMessage(state: AccountState, kind: "busy" | "error"): string {
  return state.kind === kind ? state.message : "";
}

function accountCourses(state: AccountState): ReadonlyArray<AccountCourse> {
  return state.kind === "courses" ? state.courses : [];
}

interface CoursePickerProps {
  readonly courses: ReadonlyArray<AccountCourse>;
  readonly select: (course: AccountCourse) => Promise<void>;
  readonly busy: boolean;
}

function CoursePicker(props: CoursePickerProps): JSX.Element {
  return (
    <section class="auth-panel" aria-labelledby="account-courses-heading">
      <h2 id="account-courses-heading">Choose your course</h2>
      <p>Your PLE account can belong to courses from different instructors.</p>
      <div class="account-course-list">
        <For each={props.courses}>
          {(course) => (
            <button
              class="quiet-action account-course-action"
              type="button"
              disabled={props.busy}
              onClick={() => void props.select(course)}
            >
              <span>{course.title}</span>
              <small>{course.role}</small>
            </button>
          )}
        </For>
      </div>
    </section>
  );
}

export function SignInPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [email, setEmail] = createSignal("");
  const [state, setState] = createSignal<AccountState>({ kind: "idle" });

  async function loadCourses(): Promise<void> {
    setState({ kind: "busy", message: "Opening your account..." });
    try {
      const page = await runtime.client.listAccountCourses();
      setState(
        page.courses.length === 0 ? { kind: "empty" } : { kind: "courses", courses: page.courses },
      );
    } catch {
      setState(accountError("Your account is signed in, but its course list could not load."));
    }
  }

  async function signInWithPasskey(): Promise<void> {
    setState({ kind: "busy", message: "Waiting for your passkey..." });
    try {
      await authenticatePasskeyWithBrowser(runtime.client);
      await loadCourses();
    } catch {
      setState(accountError("Passkey sign-in did not finish. You can retry or use email."));
    }
  }

  async function sendEmail(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setState({ kind: "busy", message: "Sending a sign-in link..." });
    try {
      await runtime.client.startEmailAuthentication(email());
      setState({
        kind: "busy",
        message:
          "If that address can receive PLE mail, a one-time link is on its way. Open it in this browser.",
      });
    } catch {
      setState(accountError("Email sign-in is temporarily unavailable. Try again later."));
    }
  }

  async function selectCourse(course: AccountCourse): Promise<void> {
    setState({ kind: "busy", message: `Opening ${course.title}...` });
    try {
      await runtime.client.selectAccountCourse(course.courseId);
      await session.retry();
      navigate(`/courses/${courseRouteReference(course.courseReference)}`);
    } catch {
      setState(
        accountError("That course could not be opened. Refresh your account and try again."),
      );
    }
  }

  return (
    <section class="page auth-page" data-route-surface="signIn">
      <p class="eyebrow">Passwordless account</p>
      <h1>Sign in to PLE</h1>
      <p class="page-lede">
        Email is the normal way to create or enter your PLE account. A passkey is an optional
        shortcut.
      </p>

      <div class="auth-grid">
        <form class="auth-panel auth-form" onSubmit={(event) => void sendEmail(event)}>
          <h2>Sign in with email</h2>
          <label for="passwordless-email">Email address</label>
          <input
            id="passwordless-email"
            type="email"
            autocomplete="email"
            maxlength={320}
            required
            value={email()}
            onInput={(event) => setEmail(event.currentTarget.value)}
          />
          <p class="field-help">
            We give the same response whether or not an account already exists.
          </p>
          <button class="primary-action" type="submit" disabled={state().kind === "busy"}>
            Email me a sign-in link
          </button>
        </form>

        <section class="auth-panel" aria-labelledby="passkey-sign-in-heading">
          <h2 id="passkey-sign-in-heading">Use a passkey shortcut</h2>
          <p>Your device verifies you with its normal biometric, PIN, or security-key step.</p>
          <button
            class="quiet-action"
            type="button"
            disabled={state().kind === "busy"}
            onClick={() => void signInWithPasskey()}
          >
            Sign in with a passkey
          </button>
        </section>
      </div>

      <Show when={state().kind === "busy"}>
        <p class="calm-status" role="status" aria-live="polite">
          {accountMessage(state(), "busy")}
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <p class="inline-error" role="alert">
          {accountMessage(state(), "error")}
        </p>
      </Show>
      <Show when={state().kind === "courses"}>
        <CoursePicker courses={accountCourses(state())} select={selectCourse} busy={false} />
      </Show>
      <Show when={state().kind === "empty"}>
        <section class="auth-panel" role="status">
          <h2>Your account is ready</h2>
          <p>
            It does not belong to a course yet. If an instructor invited you, return to that
            invitation and claim it now.
          </p>
        </section>
      </Show>
    </section>
  );
}

export function EmailAuthenticationCompletePage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const navigate = useNavigate();
  const [token] = createSignal(consumeTokenFragment(window.location, window.history));
  const [displayName, setDisplayName] = createSignal("");
  const [passkeyLabel, setPasskeyLabel] = createSignal("This device");
  const [offerPasskey, setOfferPasskey] = createSignal(false);
  const [state, setState] = createSignal<AccountState>(
    token() === null
      ? accountError("This sign-in link is missing or malformed.")
      : { kind: "idle" },
  );

  async function completeEmail(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const oneTimeToken = token();
    if (oneTimeToken === null) return;
    setState({ kind: "busy", message: "Confirming your one-time link..." });
    try {
      const completed = await runtime.client.completeEmailAuthentication(
        oneTimeToken,
        displayName(),
      );
      setOfferPasskey(completed.passkeyEnrollmentSuggested);
      if (completed.passkeyEnrollmentSuggested) {
        setState({ kind: "idle" });
      } else {
        await loadCourses();
      }
    } catch {
      setState(accountError("This one-time link is expired, used, or belongs to another browser."));
    }
  }

  async function addPasskey(): Promise<void> {
    setState({ kind: "busy", message: "Waiting for your new passkey..." });
    try {
      await registerPasskeyWithBrowser(runtime.client, passkeyLabel());
      setOfferPasskey(false);
      await loadCourses();
    } catch {
      setState(accountError("The passkey was not added. Your email sign-in is still available."));
    }
  }

  async function loadCourses(): Promise<void> {
    setState({ kind: "busy", message: "Opening your account..." });
    try {
      const page = await runtime.client.listAccountCourses();
      setState(
        page.courses.length === 0 ? { kind: "empty" } : { kind: "courses", courses: page.courses },
      );
    } catch {
      setState(accountError("Your account is ready, but its courses could not load."));
    }
  }

  async function selectCourse(course: AccountCourse): Promise<void> {
    setState({ kind: "busy", message: `Opening ${course.title}...` });
    try {
      await runtime.client.selectAccountCourse(course.courseId);
      await session.retry();
      navigate(`/courses/${courseRouteReference(course.courseReference)}`);
    } catch {
      setState(accountError("That course could not be opened. Try again from the sign-in page."));
    }
  }

  return (
    <section class="page auth-page" data-route-surface="emailAuthenticationComplete">
      <p class="eyebrow">One-time email link</p>
      <h1>Finish signing in</h1>
      <Show when={token() !== null && state().kind === "idle" && !offerPasskey()}>
        <form class="auth-panel auth-form" onSubmit={(event) => void completeEmail(event)}>
          <label for="account-display-name">Name shown in PLE</label>
          <input
            id="account-display-name"
            autocomplete="name"
            maxlength={200}
            required
            value={displayName()}
            onInput={(event) => setDisplayName(event.currentTarget.value)}
          />
          <p class="field-help">Use the name instructors and classmates should see.</p>
          <button class="primary-action" type="submit">
            Confirm this email sign-in
          </button>
        </form>
      </Show>
      <Show when={offerPasskey()}>
        <section class="auth-panel">
          <h2>Add an optional passkey</h2>
          <p>
            Email remains your normal PLE sign-in. A passkey can make sign-in faster on this device.
          </p>
          <label for="initial-passkey-label">Passkey name</label>
          <input
            id="initial-passkey-label"
            maxlength={80}
            required
            value={passkeyLabel()}
            onInput={(event) => setPasskeyLabel(event.currentTarget.value)}
          />
          <div class="action-row">
            <button class="primary-action" type="button" onClick={() => void addPasskey()}>
              Add this passkey
            </button>
            <button class="quiet-action" type="button" onClick={() => void loadCourses()}>
              Continue without a passkey
            </button>
          </div>
        </section>
      </Show>
      <Show when={state().kind === "busy"}>
        <p class="calm-status" role="status" aria-live="polite">
          {accountMessage(state(), "busy")}
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="inline-error" role="alert">
          <p>{accountMessage(state(), "error")}</p>
          <A href="/sign-in">Request another sign-in link</A>
        </section>
      </Show>
      <Show when={state().kind === "courses"}>
        <CoursePicker courses={accountCourses(state())} select={selectCourse} busy={false} />
      </Show>
      <Show when={state().kind === "empty"}>
        <section class="auth-panel" role="status">
          <h2>Your account is ready</h2>
          <p>Return to your course invitation and choose Claim course.</p>
        </section>
      </Show>
    </section>
  );
}
