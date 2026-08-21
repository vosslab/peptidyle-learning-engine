//! Browser-test replacement for the explicit local credential boundary.

import { createMockLocalCredentialLogin } from "../api/mock/local_development_auth";
import { createHttpLocalCredentialLogin } from "../api/http_client/local_development_auth";
import type { LocalCredentialLogin } from "./session_context";

export { LocalDevelopmentSignIn } from "./local_development";
export type { LocalDevelopmentSignInProps } from "./local_development";

declare global {
  interface Window {
    __PLE_USE_MOCK_API__?: boolean;
  }
}

/** Lets a browser test select HTTP interception before application boot. */
export function localCredentialLogin(): LocalCredentialLogin {
  return window.__PLE_USE_MOCK_API__ === false
    ? createHttpLocalCredentialLogin()
    : createMockLocalCredentialLogin();
}
