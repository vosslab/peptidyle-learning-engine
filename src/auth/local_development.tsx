//! The only browser boundary that may carry local-file credential UI or transport.
//!
//! `pipeline/build.mjs` resolves this module only for the explicit local stack
//! build. Its production replacement exports the same harmless shape without
//! importing either a credential endpoint or credential-facing markup.

import { createSignal, Show, type JSX } from "solid-js";

import { createHttpLocalCredentialLogin } from "../api/http_client/local_development_auth";
import { createMockLocalCredentialLogin } from "../api/mock/local_development_auth";
import type { LocalCredentialLogin } from "./session_context";

declare global {
  interface Window {
    __PLE_USE_MOCK_API__?: boolean;
  }
}

/** Chooses a local-only transport without adding it to the production client API. */
export function localCredentialLogin(): LocalCredentialLogin {
  return window.__PLE_USE_MOCK_API__ === true
    ? createMockLocalCredentialLogin()
    : createHttpLocalCredentialLogin();
}

export interface LocalDevelopmentSignInProps {
  readonly signIn: (credential: string) => Promise<boolean>;
}

/** Renders only in an explicit local browser build. */
export function LocalDevelopmentSignIn(props: LocalDevelopmentSignInProps): JSX.Element {
  const [credential, setCredential] = createSignal("");
  const [signInFailed, setSignInFailed] = createSignal(false);

  async function signIn(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setSignInFailed(false);
    if (await props.signIn(credential())) {
      setCredential("");
    } else {
      setSignInFailed(true);
    }
  }

  return (
    <form class="local-sign-in" onSubmit={(event) => void signIn(event)}>
      <label for="local-development-credential">Local development credential</label>
      <input
        id="local-development-credential"
        type="password"
        autocomplete="off"
        spellcheck={false}
        value={credential()}
        onInput={(event) => setCredential(event.currentTarget.value)}
        required
      />
      <p class="field-help">
        Paste the instructor or student value from containers/local-login.txt.
      </p>
      <Show when={signInFailed()}>
        <p class="inline-error" role="alert">
          That local credential was not accepted. Copy it again and retry.
        </p>
      </Show>
      <button class="primary-action" type="submit">
        Sign in locally
      </button>
    </form>
  );
}
