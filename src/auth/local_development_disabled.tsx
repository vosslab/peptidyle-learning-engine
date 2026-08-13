//! Production replacement for the local browser boundary.
//!
//! Keep this module intentionally capability-free: the build resolves it in
//! ordinary artifacts, so neither local credential text nor its endpoint can
//! enter the emitted browser files.

import type { JSX } from "solid-js";

import type { LocalCredentialLogin } from "./session_context";

export interface LocalDevelopmentSignInProps {
  readonly signIn: (credential: string) => Promise<boolean>;
}

/** Production has no local-file credential transport. */
export function localCredentialLogin(): LocalCredentialLogin | undefined {
  return undefined;
}

/** Production has no local-file credential form. */
export const LocalDevelopmentSignIn:
  ((props: LocalDevelopmentSignInProps) => JSX.Element) | undefined = undefined;
